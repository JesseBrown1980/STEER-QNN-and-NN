//! A gated linear network over the tables' predictions (panel V59): the logistic mixer of
//! PAQ-style coders, trained in integers. Inputs are the log-odds of the 27 inner tables of the
//! chosen box and of the box table; the weight set is selected by a guide's choice (the gate).
//! Tests whether the guide/answer asymmetry holds when the combination is learned. Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network, Surgery};

const WINDOW: usize = 64;
const RING: usize = 1024;
const SHIFT: u32 = 14;
const ONE: i32 = 1 << 16;
const LIMIT: i32 = 3072;
const INNER_DELAY: usize = 64;
const INNER_STRENGTH: i64 = 3072;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// no mixer: the hard switch by the full guide
    Switch,
    Single,
    Full,
    BlackHole,
    Autapse,
    /// no mixer: the box table of the chosen box alone (panel V62 check)
    BoxOnly,
}

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    pub gate: Gate,
    /// the 27 inner inputs come from the 1/256 layer instead of the full one
    pub crushed_inputs: bool,
}

pub struct Outcome {
    pub total_bytes: usize,
    pub bytes: Vec<i64>,
    /// the weight sets at the end, per arm (empty for the switch)
    pub weights: Vec<Vec<i32>>,
    /// per arm that asked for it: each weight set averaged over the bits on which it was used
    /// (integer sums, rounded down; a set never used keeps the start weights); else empty
    pub averaged: Vec<Vec<i32>>,
    /// per arm that asked for the average: the number of weight sets used at least once
    pub used_sets: Vec<usize>,
}

/// One arm of a run, with its own training step (panel V62).
#[derive(Clone, Copy)]
pub struct Spec<'a> {
    pub gate: Gate,
    pub crushed_inputs: bool,
    /// a weight grows by (input x (truth - p)) >> shift
    pub shift: u32,
    /// starting weight sets (None = weight 1 on the box table)
    pub init: Option<&'a [i32]>,
    pub frozen: bool,
    pub average: bool,
}

/// log-odds of p/4096 in 1/256 bit, and its inverse table over [-LIMIT, LIMIT]
pub fn tables() -> (Vec<i32>, Vec<u16>) {
    let cost = cost_table();
    let mut stretch = vec![0i32; 4097];
    for p in 1..4096usize {
        stretch[p] = (cost[4096 - p] - cost[p]).clamp(-LIMIT, LIMIT);
    }
    let mut squash = vec![1u16; (2 * LIMIT + 1) as usize];
    let mut p = 1usize;
    for l in -LIMIT..=LIMIT {
        while p < 4095 && stretch[p + 1] <= l {
            p += 1;
        }
        squash[(l + LIMIT) as usize] = p as u16;
    }
    (stretch, squash)
}

struct State {
    w: Vec<i32>,
    total: i64,
    sum: Vec<i64>,
    count: Vec<i64>,
}

/// Every arm on a text. `init`: starting weight sets per arm (None = weight 1 on the box table);
/// `frozen`: the weights are not trained.
pub fn run(
    net: &Network,
    sur: &Surgery,
    text: (&[usize], &[u8]),
    arms: &[Arm],
    init: Option<&[Vec<i32>]>,
    frozen: bool,
) -> Outcome {
    let specs: Vec<Spec> = arms
        .iter()
        .enumerate()
        .map(|(x, a)| Spec {
            gate: a.gate,
            crushed_inputs: a.crushed_inputs,
            shift: SHIFT,
            init: init.map(|all| all[x].as_slice()),
            frozen,
            average: false,
        })
        .collect();
    run_specs(net, sur, text, &specs)
}

/// Every arm on a text, each with its own step, start weights and averaging.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run_specs(net: &Network, sur: &Surgery, text: (&[usize], &[u8]), arms: &[Spec]) -> Outcome {
    let fan = net.cfg.fan;
    let nin = fan + 1;
    let (_, mix) = text;
    let cost = cost_table();
    let (stretch, squash) = tables();
    let bc = |p: u16, bit: u8| -> i32 {
        if bit == 1 {
            cost[p as usize]
        } else {
            cost[4096 - p as usize]
        }
    };
    let sets = |g: Gate| -> usize {
        match g {
            Gate::Switch | Gate::BoxOnly => 0,
            Gate::Single => 1,
            _ => fan * fan,
        }
    };
    let mut st: Vec<State> = arms
        .iter()
        .map(|a| {
            let w = match a.init {
                Some(given) => given.to_vec(),
                None => {
                    let mut w = vec![0i32; sets(a.gate) * nin];
                    for s in 0..sets(a.gate) {
                        w[s * nin + fan] = ONE;
                    }
                    w
                }
            };
            let n = if a.average { sets(a.gate) } else { 0 };
            State {
                w,
                total: 0,
                sum: vec![0i64; n * nin],
                count: vec![0i64; n],
            }
        })
        .collect();
    // the three guides, shared by the arms
    let mut g_full = (vec![0i64; fan], vec![0i32; fan * WINDOW]);
    let mut g_bh = (vec![0i64; fan], vec![0i32; fan * WINDOW]);
    let mut ring = vec![0usize; RING];
    let mut scratch = vec![0i64; fan];
    let mut age = 0usize;
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut pfull = vec![0u16; fan * 8];
    let mut pbh = vec![0u16; fan * 8];
    let mut cfull = vec![0i32; fan];
    let mut cbh = vec![0i32; fan];
    let mut input = vec![0i32; nin];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    for (i, &b) in mix.iter().enumerate() {
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
            g_full.0.fill(0);
            g_full.1.fill(0);
            g_bh.0.fill(0);
            g_bh.1.fill(0);
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
            let id = cur_t * fan + k;
            let t = &net.mids[id];
            let th = sur.thr[5][1][id];
            let (mut cf, mut cb) = (0, 0);
            for q in 0..8 {
                let e = k * 8 + q;
                let (pf, pb) = match find(t, idx[q]) {
                    Some(p) => (
                        t.vals[p],
                        if u64::from(t.rank[p]) >= th {
                            t.vals[p]
                        } else {
                            ptop[q]
                        },
                    ),
                    None => (ptop[q], ptop[q]),
                };
                pfull[e] = pf;
                pbh[e] = pb;
                cf += bc(pf, bits[q]);
                cb += bc(pb, bits[q]);
            }
            cfull[k] = cf;
            cbh[k] = cb;
        }
        let pick_full = argmin(&g_full.0);
        let pick_bh = argmin(&g_bh.0);
        let pick_aut = {
            scratch.copy_from_slice(&g_full.0);
            if INNER_DELAY <= age {
                scratch[ring[(age - INNER_DELAY) % RING]] -= INNER_STRENGTH;
            }
            argmin(&scratch)
        };
        ring[age % RING] = pick_aut;
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            if arm.gate == Gate::Switch {
                s.total += i64::from(cfull[pick_full]);
                continue;
            }
            if arm.gate == Gate::BoxOnly {
                s.total += i64::from(acost[cur_t]);
                continue;
            }
            let set = match arm.gate {
                Gate::Single | Gate::Switch | Gate::BoxOnly => 0,
                Gate::Full => cur_t * fan + pick_full,
                Gate::BlackHole => cur_t * fan + pick_bh,
                Gate::Autapse => cur_t * fan + pick_aut,
            };
            let w = &mut s.w[set * nin..(set + 1) * nin];
            let inner = if arm.crushed_inputs { &pbh } else { &pfull };
            for q in 0..8 {
                let mut dot = 0i64;
                for k in 0..fan {
                    input[k] = stretch[inner[k * 8 + q] as usize];
                    dot += i64::from(w[k]) * i64::from(input[k]);
                }
                input[fan] = stretch[ptop[q] as usize];
                dot += i64::from(w[fan]) * i64::from(input[fan]);
                let logit = (dot >> 16).clamp(i64::from(-LIMIT), i64::from(LIMIT)) as i32;
                let p = squash[(logit + LIMIT) as usize];
                s.total += i64::from(bc(p, bits[q]));
                if !arm.frozen {
                    let err = (i32::from(bits[q]) << 12) - i32::from(p);
                    for k in 0..nin {
                        w[k] = (w[k] + ((input[k] * err) >> arm.shift)).clamp(-(1 << 24), 1 << 24);
                    }
                }
                if arm.average {
                    let acc = &mut s.sum[set * nin..(set + 1) * nin];
                    for k in 0..nin {
                        acc[k] += i64::from(w[k]);
                    }
                    s.count[set] += 1;
                }
            }
        }
        let slot = age % WINDOW;
        for k in 0..fan {
            let h = k * WINDOW + slot;
            g_full.0[k] += i64::from(cfull[k] - g_full.1[h]);
            g_full.1[h] = cfull[k];
            g_bh.0[k] += i64::from(cbh[k] - g_bh.1[h]);
            g_bh.1[h] = cbh[k];
        }
        age += 1;
        let w = i % WINDOW;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * WINDOW + w]);
            a_hist[j * WINDOW + w] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    let averaged = st
        .iter()
        .map(|s| {
            let mut out = vec![0i32; s.sum.len()];
            for (set, &n) in s.count.iter().enumerate() {
                for k in 0..nin {
                    out[set * nin + k] = if n > 0 {
                        s.sum[set * nin + k].div_euclid(n) as i32
                    } else if k == fan {
                        ONE
                    } else {
                        0
                    };
                }
            }
            out
        })
        .collect();
    Outcome {
        total_bytes: mix.len(),
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        used_sets: st
            .iter()
            .map(|s| s.count.iter().filter(|&&n| n > 0).count())
            .collect(),
        averaged,
        weights: st.into_iter().map(|s| s.w).collect(),
    }
}

/// The seven online arms of the spec, SWITCH first.
pub fn arms() -> Vec<Arm> {
    let mk = |name: &str, gate: Gate, crushed_inputs: bool| Arm {
        name: name.to_string(),
        gate,
        crushed_inputs,
    };
    vec![
        mk("SWITCH", Gate::Switch, false),
        mk("MIX_SINGLE", Gate::Single, false),
        mk("MIX_GATE_FULL", Gate::Full, false),
        mk("MIX_GATE_BH", Gate::BlackHole, false),
        mk("MIX_GATE_AUTAPSE", Gate::Autapse, false),
        mk("MIXBH_GATE_FULL", Gate::Full, true),
        mk("MIXBH_GATE_BH", Gate::BlackHole, true),
    ]
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
    fn mixer_keeps_its_invariants() {
        let (stretch, squash) = tables();
        // stretch is odd and monotone; squash inverts it to within one step of the table
        assert_eq!(stretch[2048], 0);
        assert!(stretch[1..4096].windows(2).all(|w| w[0] <= w[1]));
        for p in [1usize, 100, 1000, 2048, 3000, 4000, 4095] {
            let back = usize::from(squash[(stretch[p] + LIMIT) as usize]);
            assert_eq!(stretch[back], stretch[p]);
        }
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
        let a = arms();
        let r = run(&net, &sur, (&ids, &bytes), &a, None, false);
        // the switch is the full network with box fallback
        let rows = evaluate(&net, &sur, &[3]);
        let base: i64 = rows
            .iter()
            .find(|r| r.starts_with("RUN|"))
            .and_then(|r| {
                r.split('|')
                    .find_map(|f| f.strip_prefix("baseline_PARENT_cost_bytes="))
            })
            .and_then(|v| v.parse().ok())
            .expect("baseline");
        assert_eq!(r.bytes[0], base);
        // a frozen mixer at its start weights answers like the box table alone
        let box_only: i64 = rows
            .iter()
            .find(|r| r.starts_with("ARM|d=1/256|rule=INVERSE|fallback=PARENT|"))
            .map_or(0, |_| 0);
        let f = run(&net, &sur, (&ids, &bytes), &a[1..2], None, true);
        let g = run(&net, &sur, (&ids, &bytes), &a[2..3], None, true);
        assert_eq!(f.bytes[0], g.bytes[0]);
        assert!(f.bytes[0] > box_only);
        // frozen with trained weights is deterministic and training changes the weights
        let again = run(&net, &sur, (&ids, &bytes), &a, Some(&r.weights), true);
        let twice = run(&net, &sur, (&ids, &bytes), &a, Some(&r.weights), true);
        assert_eq!(again.bytes, twice.bytes);
        assert_ne!(r.weights[1], f.weights[0]);
        // panel V62: the step of V59 reproduces V59; a slower step gives other weights; the
        // average differs from the last weights; a frozen arm's average is its own weights on
        // the sets it used; the start mixer costs what the box table alone costs up to the
        // rounding of the log-odds table
        let mk = |gate, shift, init, frozen, average| Spec {
            gate,
            crushed_inputs: false,
            shift,
            init,
            frozen,
            average,
        };
        let v = run_specs(
            &net,
            &sur,
            (&ids, &bytes),
            &[
                mk(Gate::Single, 14, None, false, true),
                mk(Gate::Single, 18, None, false, true),
                mk(Gate::Single, 14, Some(&r.weights[1]), true, true),
                mk(Gate::BoxOnly, 14, None, true, false),
                mk(Gate::Single, 14, None, true, false),
                mk(Gate::Full, 14, Some(&r.weights[2]), true, true),
            ],
        );
        assert_eq!(v.bytes[0], r.bytes[1]);
        assert_eq!(v.weights[0], r.weights[1]);
        assert_ne!(v.weights[1], v.weights[0]);
        assert_ne!(v.averaged[0], v.weights[0]);
        assert_eq!(v.averaged[2], r.weights[1]);
        assert_eq!(v.used_sets[2], 1);
        assert_eq!(v.bytes[4], f.bytes[0]);
        let gap = (v.bytes[4] - v.bytes[3]).abs();
        assert!(200 * gap <= v.bytes[3], "{} {}", v.bytes[4], v.bytes[3]);
        for set in 0..9 {
            let a = &v.averaged[5][set * 4..(set + 1) * 4];
            assert!(a == &r.weights[2][set * 4..(set + 1) * 4] || a == [0, 0, 0, ONE]);
        }
    }
}
