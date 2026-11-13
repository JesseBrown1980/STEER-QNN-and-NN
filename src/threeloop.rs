//! Three loops (panel V50): sensing, the guide's memory, and an external referee whose correction
//! reaches the stored values. After each predicted bit the true bit arrives and the value that
//! answered moves toward it; corrected values live in an overlay beside the trained tables.
//! Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network, Surgery};
use std::collections::HashMap;

const WINDOW: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// sensing + guide memory + correction; rates for [black-hole set, neutron-star set, other trained, created]
    ThreeLoops([u32; 4]),
    NoCorrection,
    /// the box table answers and is corrected with this rate
    NoMemory(u32),
    /// one adaptive value per bit position within the byte, this rate
    NoSensor(u32),
    Nothing,
}

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    pub kind: Kind,
}

pub struct Result {
    pub total_bytes: usize,
    pub bytes: Vec<i64>,
    pub own_inner: Vec<u64>,
    pub overlay: Vec<usize>,
}

struct State {
    sum: Vec<i64>,
    hist: Vec<i32>,
    age: usize,
    overlay: HashMap<u64, u16>,
    order0: Vec<u16>,
    total: i64,
    on_mid: u64,
}

fn toward(p: u16, bit: u8, r: u32) -> u16 {
    let target: i32 = if bit == 1 { 4095 } else { 1 };
    let p = i32::from(p);
    (p + ((target - p) >> r)).clamp(1, 4095) as u16
}

/// Every arm on a text: `text.0[i]` is the block of the i-th tail of `text.1`.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(net: &Network, sur: &Surgery, text: (&[usize], &[u8]), arms: &[Arm]) -> Result {
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
    let mut st: Vec<State> = arms
        .iter()
        .map(|_| State {
            sum: vec![0; fan],
            hist: vec![0; fan * WINDOW],
            age: 0,
            overlay: HashMap::new(),
            order0: vec![2048; 256],
            total: 0,
            on_mid: 0,
        })
        .collect();
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let (mut idx, mut bits, mut nodes) = ([0u32; 8], [0u8; 8], [0usize; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut mpos: Vec<Option<usize>> = vec![None; fan * 8];
    let mut cst = vec![0i32; fan];
    let mut pv_all = vec![0u16; fan * 8];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    for (i, &b) in mix.iter().enumerate() {
        let own_m = ids[i / tail] / fan;
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            idx[q] = (base + node) as u32;
            nodes[q] = node;
            bits[q] = bit;
            node = (node << 1) | bit as usize;
        }
        let a = argmin(&a_sum);
        if a != cur_t {
            cur_t = a;
            for s in &mut st {
                s.sum.fill(0);
                s.hist.fill(0);
                s.age = 0;
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
                mpos[k * 8 + q] = find(t, idx[q]);
            }
        }
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            match arm.kind {
                Kind::Nothing => {
                    s.total += i64::from(acost[cur_t]);
                }
                Kind::NoSensor(r) => {
                    for q in 0..8 {
                        let p = s.order0[nodes[q]];
                        s.total += i64::from(bc(p, bits[q]));
                        s.order0[nodes[q]] = toward(p, bits[q], r);
                    }
                }
                Kind::NoMemory(r) => {
                    for q in 0..8 {
                        let key = ((cur_t as u64 + 100_000) << 32) | u64::from(idx[q]);
                        let p = s.overlay.get(&key).copied().unwrap_or(ptop[q]);
                        s.total += i64::from(bc(p, bits[q]));
                        s.overlay.insert(key, toward(p, bits[q], r));
                    }
                }
                Kind::NoCorrection | Kind::ThreeLoops(_) => {
                    let learning = matches!(arm.kind, Kind::ThreeLoops(_));
                    let pick = argmin(&s.sum);
                    for k in 0..fan {
                        let id = cur_t * fan + k;
                        let t = &net.mids[id];
                        let mut ck = 0;
                        for q in 0..8 {
                            let e = k * 8 + q;
                            let stored = mpos[e].map(|p| t.vals[p]);
                            let pv = if learning {
                                let key = ((id as u64) << 32) | u64::from(idx[q]);
                                s.overlay.get(&key).copied().or(stored).unwrap_or(ptop[q])
                            } else {
                                stored.unwrap_or(ptop[q])
                            };
                            pv_all[e] = pv;
                            ck += bc(pv, bits[q]);
                        }
                        cst[k] = ck;
                    }
                    s.total += i64::from(cst[pick]);
                    s.on_mid += u64::from(cur_t * fan + pick == own_m);
                    if let Kind::ThreeLoops(rates) = arm.kind {
                        // the referee corrects the values that answered, in the chosen table only
                        let id = cur_t * fan + pick;
                        let t = &net.mids[id];
                        for q in 0..8 {
                            let e = pick * 8 + q;
                            let r = match mpos[e] {
                                None => rates[3],
                                Some(p) => {
                                    let rank = u64::from(t.rank[p]);
                                    if rank >= sur.thr[5][1][id] {
                                        rates[0]
                                    } else if rank >= sur.thr[3][1][id] {
                                        rates[1]
                                    } else {
                                        rates[2]
                                    }
                                }
                            };
                            let key = ((id as u64) << 32) | u64::from(idx[q]);
                            s.overlay.insert(key, toward(pv_all[e], bits[q], r));
                        }
                    }
                    let w = s.age % WINDOW;
                    for k in 0..fan {
                        let h = k * WINDOW + w;
                        s.sum[k] += i64::from(cst[k] - s.hist[h]);
                        s.hist[h] = cst[k];
                    }
                    s.age += 1;
                }
            }
        }
        let w = i % WINDOW;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * WINDOW + w]);
            a_hist[j * WINDOW + w] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    Result {
        total_bytes: mix.len(),
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
        overlay: st.iter().map(|s| s.overlay.len()).collect(),
    }
}

/// The eight arms of the spec, NO_CORRECTION first.
pub fn arms() -> Vec<Arm> {
    let mk = |name: &str, kind: Kind| Arm {
        name: name.to_string(),
        kind,
    };
    vec![
        mk("NO_CORRECTION", Kind::NoCorrection),
        mk("THREE_LOOPS_DENSITY_RATES", Kind::ThreeLoops([6, 5, 4, 3])),
        mk("THREE_LOOPS_REVERSED_RATES", Kind::ThreeLoops([3, 4, 5, 6])),
        mk("THREE_LOOPS_UNIFORM_4", Kind::ThreeLoops([4, 4, 4, 4])),
        mk("THREE_LOOPS_UNIFORM_5", Kind::ThreeLoops([5, 5, 5, 5])),
        mk("NO_MEMORY", Kind::NoMemory(4)),
        mk("NO_SENSOR", Kind::NoSensor(4)),
        mk("NOTHING", Kind::Nothing),
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
    fn three_loops_keep_their_invariants() {
        assert_eq!(toward(2048, 1, 4), 2048 + ((4095 - 2048) >> 4));
        assert_eq!(toward(2048, 0, 4), (2048 + ((1 - 2048) >> 4)) as u16);
        assert!(toward(4095, 1, 3) == 4095 && toward(1, 0, 3) == 1);
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
        let r = run(&net, &sur, (&ids, &bytes), &a);
        // NO_CORRECTION is the full network with box fallback; it and NOTHING keep no overlay
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
        assert_eq!(r.bytes[0], base);
        assert_eq!((r.overlay[0], r.overlay[7]), (0, 0));
        assert!(r.overlay[1] > 0 && r.overlay[5] > 0);
        // on this very repetitive text the corrected network does at least as well as the fixed one
        assert!(r.bytes[3] <= r.bytes[0]);
        let again = run(&net, &sur, (&ids, &bytes), &a);
        assert_eq!(r.bytes, again.bytes);
    }
}
