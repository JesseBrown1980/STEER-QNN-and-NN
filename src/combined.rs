//! The three gains that held in both orders, in one network (panel V57): an autapse guide (box
//! cell and inner cell), a referee that corrects the stored values, and cores chosen by
//! information. Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network, Surgery};
use crate::reading::{Reading, CUTS};
use std::collections::HashMap;

const WINDOW: usize = 64;
const RING: usize = 1024;
pub const BOX_DELAYS: [usize; 3] = [8, 64, 512];
pub const BOX_STRENGTH: i64 = 4096;
pub const INNER_DELAY: usize = 64;
pub const INNER_STRENGTH: i64 = 3072;
pub const RATE: u32 = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Core {
    Normal,
    /// index into reading::CUTS, entries ranked by information (one threshold for all tables)
    Info(usize),
    /// index into reading::CUTS, entries ranked by weight (one threshold for all tables)
    Weight(usize),
    Nothing,
}

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    pub core: Core,
    pub learn: bool,
    pub inner_autapse: bool,
    /// the correction step while learning
    pub step: Step,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// always 1/2^r
    Fixed(u32),
    /// 1/4 for a cell's first two corrections, then halving, never below 1/2^f
    Floor(u32),
}

fn step_of(rule: Step, corrections: u16) -> u32 {
    match rule {
        Step::Fixed(r) => r,
        Step::Floor(f) => {
            let young = match corrections {
                0..=1 => 2,
                2..=3 => 3,
                4..=7 => 4,
                8..=15 => 5,
                16..=31 => 6,
                _ => 7,
            };
            young.min(f)
        }
    }
}

pub struct Outcome {
    pub total_bytes: usize,
    pub own_box: u64,
    pub bytes: Vec<i64>,
    pub own_inner: Vec<u64>,
    pub learned: Vec<usize>,
}

struct State {
    sum: Vec<i64>,
    hist: Vec<i32>,
    age: usize,
    ring: Vec<usize>,
    cells: HashMap<u64, (u16, u16)>,
    total: i64,
    on_mid: u64,
}

fn toward(p: u16, bit: u8, r: u32) -> u16 {
    let target: i32 = if bit == 1 { 4095 } else { 1 };
    let p = i32::from(p);
    (p + ((target - p) >> r)).clamp(1, 4095) as u16
}

/// Every arm under one box cell on a text: `text.0[i]` is the block of the i-th tail of `text.1`.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(
    net: &Network,
    sur: &Surgery,
    rd: &Reading,
    text: (&[usize], &[u8]),
    box_autapse: bool,
    arms: &[Arm],
) -> Outcome {
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
            ring: vec![0; RING],
            cells: HashMap::new(),
            total: 0,
            on_mid: 0,
        })
        .collect();
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let mut a_ring = vec![0usize; RING];
    let mut scratch = vec![0i64; fan];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut mpos: Vec<Option<usize>> = vec![None; fan * 8];
    let mut cst = vec![0i32; fan];
    let mut pv_pick = [0u16; 8];
    let mut pv_all = vec![0u16; fan * 8];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    let mut own_box = 0u64;
    for (i, &b) in mix.iter().enumerate() {
        let own = ids[i / tail];
        let (own_m, own_t) = (own / fan, own / (fan * fan));
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            idx[q] = (base + node) as u32;
            bits[q] = bit;
            node = (node << 1) | bit as usize;
        }
        let a = if box_autapse {
            scratch.copy_from_slice(&a_sum);
            for d in BOX_DELAYS {
                if d <= i {
                    scratch[a_ring[(i - d) % RING]] -= BOX_STRENGTH;
                }
            }
            argmin(&scratch)
        } else {
            argmin(&a_sum)
        };
        a_ring[i % RING] = a;
        if a != cur_t {
            cur_t = a;
            for s in &mut st {
                s.sum.fill(0);
                s.hist.fill(0);
                s.age = 0;
            }
        }
        own_box += u64::from(cur_t == own_t);
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
            let pick = if arm.inner_autapse {
                scratch.copy_from_slice(&s.sum);
                if INNER_DELAY <= s.age {
                    scratch[s.ring[(s.age - INNER_DELAY) % RING]] -= INNER_STRENGTH;
                }
                argmin(&scratch)
            } else {
                argmin(&s.sum)
            };
            s.ring[s.age % RING] = pick;
            for k in 0..fan {
                let id = cur_t * fan + k;
                let t = &net.mids[id];
                let mut ck = 0;
                for q in 0..8 {
                    let e = k * 8 + q;
                    let stored = mpos[e]
                        .filter(|&p| match arm.core {
                            Core::Normal => true,
                            Core::Info(ci) => u64::from(rd.info[id][p]) >= rd.info_thr[ci][1][id],
                            Core::Weight(ci) => u64::from(t.rank[p]) >= sur.thr[CUTS[ci]][2][id],
                            Core::Nothing => false,
                        })
                        .map(|p| t.vals[p]);
                    let pv = if arm.learn {
                        let key = ((id as u64) << 32) | u64::from(idx[q]);
                        s.cells.get(&key).map(|c| c.0).or(stored).unwrap_or(ptop[q])
                    } else {
                        stored.unwrap_or(ptop[q])
                    };
                    pv_all[e] = pv;
                    ck += bc(pv, bits[q]);
                }
                cst[k] = ck;
            }
            pv_pick.copy_from_slice(&pv_all[pick * 8..pick * 8 + 8]);
            s.total += i64::from(cst[pick]);
            s.on_mid += u64::from(cur_t * fan + pick == own_m);
            if arm.learn {
                let id = cur_t * fan + pick;
                for q in 0..8 {
                    let key = ((id as u64) << 32) | u64::from(idx[q]);
                    let n = s.cells.get(&key).map_or(0, |c| c.1);
                    let r = step_of(arm.step, n);
                    s.cells
                        .insert(key, (toward(pv_pick[q], bits[q], r), n.saturating_add(1)));
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
        let w = i % WINDOW;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * WINDOW + w]);
            a_hist[j * WINDOW + w] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    Outcome {
        total_bytes: mix.len(),
        own_box,
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
        learned: st.iter().map(|s| s.cells.len()).collect(),
    }
}

/// The eleven arms of the spec, N_STATIC_PLAIN first. CUTS index 1 = 1/16, 3 = 1/256.
pub fn arms() -> Vec<Arm> {
    let mk = |name: &str, core: Core, learn: bool, inner_autapse: bool| Arm {
        name: name.to_string(),
        core,
        learn,
        inner_autapse,
        step: Step::Fixed(RATE),
    };
    vec![
        mk("N_STATIC_PLAIN", Core::Normal, false, false),
        mk("N_STATIC_AUTAPSE", Core::Normal, false, true),
        mk("N_LEARN_PLAIN", Core::Normal, true, false),
        mk("N_LEARN_AUTAPSE", Core::Normal, true, true),
        mk("INFO16_LEARN_AUTAPSE", Core::Info(1), true, true),
        mk("INFO256_LEARN_AUTAPSE", Core::Info(3), true, true),
        mk("WEIGHT16_LEARN_AUTAPSE", Core::Weight(1), true, true),
        mk("WEIGHT256_LEARN_AUTAPSE", Core::Weight(3), true, true),
        mk("NOTHING_LEARN_AUTAPSE", Core::Nothing, true, true),
        mk("INFO16_STATIC_AUTAPSE", Core::Info(1), false, true),
        mk("INFO256_STATIC_AUTAPSE", Core::Info(3), false, true),
    ]
}

/// The eight arms of the floor panel (V58), N_FIXED4 first; all with the inner autapse.
pub fn floor_arms() -> Vec<Arm> {
    let mk = |name: &str, core: Core, step: Step| Arm {
        name: name.to_string(),
        core,
        learn: true,
        inner_autapse: true,
        step,
    };
    vec![
        mk("N_FIXED4", Core::Normal, Step::Fixed(4)),
        mk("N_FIXED3", Core::Normal, Step::Fixed(3)),
        mk("N_FLOOR4", Core::Normal, Step::Floor(4)),
        mk("N_FLOOR3", Core::Normal, Step::Floor(3)),
        mk("INFO256_FIXED4", Core::Info(3), Step::Fixed(4)),
        mk("INFO256_FLOOR4", Core::Info(3), Step::Floor(4)),
        mk("INFO256_FLOOR3", Core::Info(3), Step::Floor(3)),
        mk("INFO16_FLOOR4", Core::Info(1), Step::Floor(4)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::neuron::{autapse_arms, run as neuron_run, BoxRule};
    use crate::pressure_boxes::{build, evaluate, surgery, Config, Source};
    use crate::reading::read;
    use crate::surgery::split;
    use crate::threeloop::{arms as three_arms, run as three_run};

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
    fn combined_network_matches_its_parts() {
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
        let rd = read(&net, &sur);
        let (ids, bytes) = split(&net, 1);
        let a = arms();
        assert_eq!(a.len(), 11);
        let plain = run(&net, &sur, &rd, (&ids, &bytes), false, &a);
        // the fixed full network with plain guides
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
        assert_eq!(plain.bytes[0], base);
        // the referee alone is THREE_LOOPS_UNIFORM_4 of the three-loop panel
        let three = three_run(&net, &sur, (&ids, &bytes), &three_arms());
        assert_eq!(plain.bytes[2], three.bytes[3]);
        // the guide alone is the autapse panel's box cell B4096 with the inner EXC_MEDIUM_ONLY cell
        let au = neuron_run(
            &net,
            &sur,
            (&ids, &bytes),
            BoxRule::Autapse(BOX_STRENGTH),
            &autapse_arms(),
        );
        let with_box = run(&net, &sur, &rd, (&ids, &bytes), true, &a);
        assert_eq!(with_box.bytes[0], au.bytes[0]);
        assert_eq!(with_box.bytes[1], au.bytes[5]);
        assert_eq!(with_box.own_box, au.own_box);
        assert!(with_box.learned[3] > 0 && with_box.learned[0] == 0);
        // the floor panel's N_FIXED4 is this panel's N_LEARN_AUTAPSE; a floor of 2 is the fixed step 1/4
        let fl = floor_arms();
        assert_eq!(fl.len(), 8);
        let f = run(&net, &sur, &rd, (&ids, &bytes), true, &fl);
        assert_eq!(f.bytes[0], with_box.bytes[3]);
        let mut a2 = fl[0].clone();
        a2.step = Step::Fixed(2);
        let mut b2 = fl[0].clone();
        b2.step = Step::Floor(2);
        let g = run(&net, &sur, &rd, (&ids, &bytes), false, &[a2, b2]);
        assert_eq!(g.bytes[0], g.bytes[1]);
        assert_eq!(
            (
                step_of(Step::Floor(4), 0),
                step_of(Step::Floor(4), 3),
                step_of(Step::Floor(4), 100)
            ),
            (2, 3, 4)
        );
    }
}
