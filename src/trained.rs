//! A cell with trained loops (panel V52): 16 loops (4 densities x 4 memories) and 3 autapses feed
//! one cell that chooses the inner table by a weighted total; the 19 integer weights are trained
//! by greedy integer steps on a training text. No gradient, no floating point.

use crate::pressure_boxes::{argmin, cost_table, find, Network, Surgery};

pub const DENSITIES: [Option<usize>; 4] = [None, Some(1), Some(3), Some(5)];
pub const DENSITY_NAMES: [&str; 4] = ["NORMAL", "1/4", "1/16", "1/256"];
pub const MEMORIES: [usize; 4] = [8, 64, 512, 4096];
pub const DELAYS: [usize; 3] = [8, 64, 512];
pub const NW: usize = 19;
const RING: usize = 1024;
/// one autapse unit, in the same scale as a loop's cost per byte times 4096
const UNIT: i64 = 4096 * 64;

pub type Weights = [i32; NW];

/// weight 1 on the loop (NORMAL, memory 64)
pub fn reference() -> Weights {
    let mut w = [0; NW];
    w[1] = 1;
    w
}

pub fn weight_name(c: usize) -> String {
    if c < 16 {
        format!("loop_{}_m{}", DENSITY_NAMES[c / 4], MEMORIES[c % 4])
    } else {
        format!("autapse_d{}", DELAYS[c - 16])
    }
}

pub struct Outcome {
    pub total_bytes: usize,
    pub bytes: Vec<i64>,
    pub own_inner: Vec<u64>,
}

struct Loop {
    sum: Vec<i64>,
    hist: Vec<i32>,
    memory: usize,
}

struct Cell {
    ring: Vec<usize>,
    total: i64,
    on_mid: u64,
}

/// Every weight vector on a text: `text.0[i]` is the block of the i-th tail of `text.1`.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(net: &Network, sur: &Surgery, text: (&[usize], &[u8]), cells: &[Weights]) -> Outcome {
    let fan = net.cfg.fan;
    let tail = net.cfg.tail;
    let (ids, mix) = text;
    let cost = cost_table();
    let bc = |p: u16, bit: u8| -> i32 {
        if bit == 1 {
            cost[p as usize]
        } else {
            cost[4096 - p as usize]
        }
    };
    let mut loops: Vec<Loop> = Vec::new();
    for _ in DENSITIES {
        for m in MEMORIES {
            loops.push(Loop {
                sum: vec![0; fan],
                hist: vec![0; fan * m],
                memory: m,
            });
        }
    }
    let mut st: Vec<Cell> = cells
        .iter()
        .map(|_| Cell {
            ring: vec![0; RING],
            total: 0,
            on_mid: 0,
        })
        .collect();
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * 64];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut mhas = vec![false; fan * 8];
    let mut mrank = vec![0u64; fan * 8];
    let mut mval = vec![0u16; fan * 8];
    let mut level = vec![0i32; 4 * fan];
    let mut scaled = vec![0i64; 16 * fan];
    let mut score = vec![0i64; fan];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    let mut age = 0usize;
    for (i, &b) in mix.iter().enumerate() {
        let own_m = ids[i / tail] / fan;
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            idx[q] = (base + node) as u32;
            bits[q] = bit;
            node = (node << 1) | bit as usize;
        }
        let a = argmin(&a_sum);
        if a != cur_t {
            cur_t = a;
            age = 0;
            for l in &mut loops {
                l.sum.fill(0);
                l.hist.fill(0);
            }
        }
        for j in 0..fan {
            let t = &net.tops[j];
            let mut cj = 0;
            for q in 0..8 {
                let pv = find(t, idx[q]).map_or(2048, |p| t.vals[p]);
                if j == cur_t {
                    ptop[q] = pv;
                }
                cj += bc(pv, bits[q]);
            }
            acost[j] = cj;
        }
        for k in 0..fan {
            let t = &net.mids[cur_t * fan + k];
            for q in 0..8 {
                let e = k * 8 + q;
                match find(t, idx[q]) {
                    Some(p) => {
                        mhas[e] = true;
                        mrank[e] = u64::from(t.rank[p]);
                        mval[e] = t.vals[p];
                    }
                    None => mhas[e] = false,
                }
            }
        }
        // each loop's cost per byte, scaled to a common length, from the past
        for (li, l) in loops.iter().enumerate() {
            let scale = (4096 / l.memory) as i64;
            for k in 0..fan {
                scaled[li * fan + k] = l.sum[k] * scale;
            }
        }
        // this byte's cost of every table at each density (the NORMAL one is the answer)
        for (di, dens) in DENSITIES.iter().enumerate() {
            for k in 0..fan {
                let th = dens.map_or(0, |d| sur.thr[d][1][cur_t * fan + k]);
                let mut ck = 0;
                for q in 0..8 {
                    let e = k * 8 + q;
                    let pv = if mhas[e] && mrank[e] >= th {
                        mval[e]
                    } else {
                        ptop[q]
                    };
                    ck += bc(pv, bits[q]);
                }
                level[di * fan + k] = ck;
            }
        }
        for (x, w) in cells.iter().enumerate() {
            let s = &mut st[x];
            score.fill(0);
            for li in 0..16 {
                let wl = i64::from(w[li]);
                if wl != 0 {
                    for k in 0..fan {
                        score[k] += wl * scaled[li * fan + k];
                    }
                }
            }
            for (ai, &d) in DELAYS.iter().enumerate() {
                let wa = i64::from(w[16 + ai]);
                if wa != 0 && d <= age {
                    score[s.ring[(age - d) % RING]] -= wa * UNIT;
                }
            }
            let pick = argmin(&score);
            s.ring[age % RING] = pick;
            s.total += i64::from(level[pick]);
            s.on_mid += u64::from(cur_t * fan + pick == own_m);
        }
        for (li, l) in loops.iter_mut().enumerate() {
            let slot = age % l.memory;
            let di = li / 4;
            for k in 0..fan {
                let c = level[di * fan + k];
                let h = k * l.memory + slot;
                l.sum[k] += i64::from(c - l.hist[h]);
                l.hist[h] = c;
            }
        }
        age += 1;
        let w = i % 64;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * 64 + w]);
            a_hist[j * 64 + w] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    Outcome {
        total_bytes: mix.len(),
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
    }
}

/// Greedy integer training from REFERENCE. Returns the weights and the path rows.
pub fn train(
    net: &Network,
    sur: &Surgery,
    text: (&[usize], &[u8]),
    rounds: usize,
    label: &str,
) -> (Weights, Vec<String>) {
    let mut w = reference();
    let mut best = run(net, sur, text, &[w]).bytes[0];
    let mut rows = vec![format!(
        "TRAIN|cell={label}|round=0|change=REFERENCE|bytes_on_training_text={best}|json=0"
    )];
    for round in 1..=rounds {
        let mut cands: Vec<Weights> = Vec::with_capacity(2 * NW);
        for c in 0..NW {
            for step in [1, -1] {
                let mut v = w;
                v[c] += step;
                cands.push(v);
            }
        }
        let out = run(net, sur, text, &cands);
        let mut pick = 0;
        for (x, &b) in out.bytes.iter().enumerate() {
            if b < out.bytes[pick] {
                pick = x;
            }
        }
        if out.bytes[pick] >= best {
            rows.push(format!(
                "TRAIN|cell={label}|round={round}|change=NONE_IMPROVES|stopped=1|json=0"
            ));
            break;
        }
        best = out.bytes[pick];
        w = cands[pick];
        rows.push(format!(
            "TRAIN|cell={label}|round={round}|change={}{}|bytes_on_training_text={best}|json=0",
            weight_name(pick / 2),
            if pick % 2 == 0 { "+1" } else { "-1" }
        ));
    }
    (w, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pressure_boxes::{build, evaluate, surgery, Config, Source};
    use crate::surgery::split;

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
    fn trained_cell_keeps_its_invariants() {
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
        // REFERENCE is the full network with box fallback
        let rows = evaluate(&net, &sur, &[3]);
        let run_row = rows
            .iter()
            .find(|r| r.starts_with("RUN|"))
            .expect("run row");
        let base: i64 = run_row
            .split('|')
            .find_map(|f| f.strip_prefix("baseline_PARENT_cost_bytes="))
            .and_then(|v| v.parse().ok())
            .expect("baseline");
        let mut doubled = reference();
        doubled[1] = 2;
        let r = run(&net, &sur, (&ids, &bytes), &[reference(), doubled]);
        assert_eq!(r.bytes[0], base);
        assert_eq!(r.bytes[1], base); // scaling one loop's weight changes nothing
                                      // training never ends above where it started, on the text it trained on
        let (w, path) = train(&net, &sur, (&ids, &bytes), 3, "TEST");
        let after = run(&net, &sur, (&ids, &bytes), &[w]).bytes[0];
        assert!(after <= base);
        assert!(path.len() >= 2);
        assert_eq!(weight_name(1), "loop_NORMAL_m64");
        assert_eq!(weight_name(18), "autapse_d512");
    }
}
