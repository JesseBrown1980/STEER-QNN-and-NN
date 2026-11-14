//! The coherence length of the best box and the stickiness it predicts (panel V60).
//! L_G = bytes / (number of changes of the segment-best box at grain G);
//! STRENGTH* = floor(256 * log2((K - 1) * L)). Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network};

pub const GRAINS: [usize; 3] = [64, 256, 2048];

/// The sweep of the spec: a guard, thirteen quarter-octave values from 1024 to 8192, a guard.
pub const GRID: [i64; 15] = [
    512, 1024, 1218, 1448, 1722, 2048, 2435, 2896, 3444, 4096, 4871, 5793, 6889, 8192, 16384,
];

pub struct Measure {
    pub bytes: usize,
    /// per grain: changes of the segment-best box
    pub changes: [u64; 3],
    /// changes of the true box between consecutive tails
    pub label_changes: u64,
    /// tails whose best box is their true box
    pub tails_best_is_true: u64,
    pub tails: u64,
}

impl Measure {
    pub fn length(&self, g: usize) -> u64 {
        match self.changes[g] {
            0 => self.bytes as u64,
            c => self.bytes as u64 / c,
        }
    }
    pub fn label_length(&self) -> u64 {
        match self.label_changes {
            0 => self.bytes as u64,
            c => self.bytes as u64 / c,
        }
    }
}

/// floor(256 * log2(x)) for x >= 1, by squaring a 1.63 fixed-point mantissa eight times.
pub fn log2_256(x: u64) -> i64 {
    assert!(x >= 1, "log2 of zero");
    let n = 63 - u64::from(x.leading_zeros());
    let mut m = u128::from(x) << (63 - n);
    let mut frac = 0i64;
    for _ in 0..8 {
        m = (m * m) >> 63;
        frac <<= 1;
        if m >= 1u128 << 64 {
            frac |= 1;
            m >>= 1;
        }
    }
    (n as i64) * 256 + frac
}

/// The stickiness the formula gives for `options` choices changing once in `length` bytes.
pub fn strength_star(options: u64, length: u64) -> i64 {
    log2_256((options - 1) * length.max(1))
}

/// Segment-best boxes of a text at the three grains, from the box tables alone.
#[allow(clippy::needless_range_loop)]
pub fn measure(net: &Network, text: (&[usize], &[u8])) -> Measure {
    let fan = net.cfg.fan;
    let tail = net.cfg.tail;
    let (ids, mix) = text;
    let cost = cost_table();
    let mut sums = vec![vec![0i64; fan]; GRAINS.len()];
    let mut last = [usize::MAX; 3];
    let mut changes = [0u64; 3];
    let (mut tails, mut hit) = (0u64, 0u64);
    let (mut c1, mut c2) = (0usize, 0usize);
    for (i, &b) in mix.iter().enumerate() {
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            idx[q] = (base + node) as u32;
            bits[q] = bit;
            node = (node << 1) | bit as usize;
        }
        for j in 0..fan {
            let t = &net.tops[j];
            let mut cj = 0i64;
            for q in 0..8 {
                let pv = find(t, idx[q]).map_or(2048, |p| t.vals[p]) as usize;
                cj += i64::from(if bits[q] == 1 {
                    cost[pv]
                } else {
                    cost[4096 - pv]
                });
            }
            for s in &mut sums {
                s[j] += cj;
            }
        }
        for (g, &grain) in GRAINS.iter().enumerate() {
            if (i + 1) % grain == 0 || i + 1 == mix.len() {
                let best = argmin(&sums[g]);
                if last[g] != usize::MAX && best != last[g] {
                    changes[g] += 1;
                }
                last[g] = best;
                sums[g].fill(0);
                if grain == tail {
                    tails += 1;
                    hit += u64::from(best == ids[i / tail] / (fan * fan));
                }
            }
        }
        c2 = c1;
        c1 = b as usize;
    }
    let label_changes = ids
        .windows(2)
        .filter(|w| w[0] / (fan * fan) != w[1] / (fan * fan))
        .count() as u64;
    Measure {
        bytes: mix.len(),
        changes,
        label_changes,
        tails_best_is_true: hit,
        tails,
    }
}

/// Index in GRID of the value nearest to `s` on the logarithmic scale (ties to the lower index).
pub fn nearest_grid(s: i64) -> usize {
    let target = log2_256(s.max(1) as u64);
    let mut best = 0;
    for (k, &g) in GRID.iter().enumerate() {
        if (log2_256(g as u64) - target).abs() < (log2_256(GRID[best] as u64) - target).abs() {
            best = k;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::neuron::{run, Arm, BoxRule};
    use crate::pressure_boxes::{build, surgery, Config, Source};
    use crate::surgery::{split, split_shuffled};

    struct Synthetic {
        block: usize,
    }

    impl Source for Synthetic {
        fn read_block(&mut self, j: usize, buf: &mut [u8]) -> std::result::Result<(), String> {
            let words: [&[u8]; 9] = [
                b"alpha ",
                b"beta ",
                b"gamma ",
                b"fn main ",
                b"let x ",
                b"return ",
                b"|json=0\n",
                b"TAG|k=v",
                b"|sha=",
            ];
            let mut s = (j as u64 / 3) * 7919 + 17 + j as u64;
            let dialect = (j / 9) % 3;
            let mut pos = 0;
            while pos < self.block {
                s = s
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let w = words[dialect * 3 + ((s >> 33) % 3) as usize];
                for &c in w {
                    if pos < self.block {
                        buf[pos] = c;
                        pos += 1;
                    }
                }
            }
            Ok(())
        }
    }

    #[test]
    fn coherence_keeps_its_invariants() {
        // exact on powers of two, and the figure the spec quotes
        assert_eq!(log2_256(1), 0);
        assert_eq!(log2_256(2), 256);
        assert_eq!(log2_256(1 << 40), 40 * 256);
        assert_eq!(strength_star(27, 2048), 4019);
        assert!(log2_256(3) == 405 && log2_256(1_000_000) == 5102);
        assert_eq!(GRID[nearest_grid(4019)], 4096);
        assert_eq!(GRID[nearest_grid(1340)], 1448);
        assert_eq!(GRID[nearest_grid(1)], 512);
        let cfg = Config {
            fan: 3,
            block: 6000,
            tail: 500,
            tail_mod: 3,
            tail_rem: 1,
            skip_rem: None,
        };
        let net = build(cfg, &mut Synthetic { block: cfg.block }).expect("build");
        let sur = surgery(&net);
        let (ids, bytes) = split(&net, 1);
        let m = measure(&net, (&ids, &bytes));
        // a coarser grain cannot change more often than there are segment boundaries
        assert_eq!(m.bytes, bytes.len());
        assert!(m.changes[0] <= (bytes.len() / 64) as u64);
        assert!(m.length(2) >= 1 && m.tails_best_is_true <= m.tails);
        // in block order the three dialects come in runs of boxes, so the label changes fan - 1 times
        assert_eq!(m.label_changes, 2);
        let (sid, sbytes) = split_shuffled(&net, 1);
        let ms = measure(&net, (&sid, &sbytes));
        assert!(ms.label_changes >= m.label_changes);
        // a stickiness of zero is the plain box cell, with either device
        let reference = Arm {
            name: "REFERENCE".to_string(),
            loops: vec![(None, 64)],
            vote: false,
            serial: false,
            autapse: None,
        };
        let a = [reference];
        let plain = run(&net, &sur, (&ids, &bytes), BoxRule::Plain64, &a);
        let hold0 = run(&net, &sur, (&ids, &bytes), BoxRule::Hold(0), &a);
        let hold = run(&net, &sur, (&ids, &bytes), BoxRule::Hold(1 << 40), &a);
        assert_eq!(plain.bytes, hold0.bytes);
        // an unbounded stickiness never leaves the box it first took
        let first = run(
            &net,
            &sur,
            (&ids[..1], &bytes[..500]),
            BoxRule::Hold(1 << 40),
            &a,
        );
        assert!(hold.own_box <= bytes.len() as u64 && first.total_bytes == 500);
        assert_ne!(hold.bytes, plain.bytes);
    }
}
