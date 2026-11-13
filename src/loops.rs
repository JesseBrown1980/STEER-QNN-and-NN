//! Side loops between stages (panel V48, the operator's idea): the sender's signal filters through
//! a loop of three layers at chosen densities, comes back to the sender as feedback, and only then
//! is sent on. Integers only. Feedback here is a second decision by the sender.

use crate::pressure_boxes::{argmin, cost_table, find, Network, Surgery};

const WINDOW: usize = 64;

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    /// densities of the three loop layers: None = NORMAL, Some(d) = index into DENS (sealed in the box)
    pub loop_dens: [Option<usize>; 3],
    /// true = GATE (take the loop's table only when its three layers agree), false = SUM
    pub gate: bool,
    pub stages: usize,
}

pub struct Result {
    pub total_bytes: usize,
    pub reference_bytes: i64,
    pub reference_own: u64,
    pub bytes: Vec<i64>,
    pub own_inner: Vec<u64>,
    pub changed: Vec<u64>,
}

#[derive(Clone)]
struct Layer {
    sum: Vec<i64>,
    hist: Vec<i32>,
}

impl Layer {
    fn new(fan: usize) -> Layer {
        Layer {
            sum: vec![0; fan],
            hist: vec![0; fan * WINDOW],
        }
    }
    fn clear(&mut self) {
        self.sum.fill(0);
        self.hist.fill(0);
    }
    fn push(&mut self, cst: &[i32], slot: usize) {
        for (k, &c) in cst.iter().enumerate() {
            let h = k * WINDOW + slot;
            self.sum[k] += i64::from(c - self.hist[h]);
            self.hist[h] = c;
        }
    }
}

struct Stage {
    sender: Layer,
    lp: [Layer; 3],
}

struct State {
    stages: Vec<Stage>,
    age: usize,
    total: i64,
    on_mid: u64,
    changed: u64,
}

/// The reference (the sender alone) and every arm on a text: `text.0[i]` is the block of the
/// i-th tail of `text.1`.
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
        .map(|a| State {
            stages: (0..a.stages)
                .map(|_| Stage {
                    sender: Layer::new(fan),
                    lp: [Layer::new(fan), Layer::new(fan), Layer::new(fan)],
                })
                .collect(),
            age: 0,
            total: 0,
            on_mid: 0,
            changed: 0,
        })
        .collect();
    let mut reference = Layer::new(fan);
    let (mut r_age, mut r_total, mut r_on) = (0usize, 0i64, 0u64);
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut mhas = vec![false; fan * 8];
    let mut mrank = vec![0u64; fan * 8];
    let mut mval = vec![0u16; fan * 8];
    let mut cst = vec![0i32; fan];
    let mut cand = vec![0u16; fan * 8];
    let mut total_sum = vec![0i64; fan];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
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
            reference.clear();
            r_age = 0;
            for s in &mut st {
                s.age = 0;
                for g in &mut s.stages {
                    g.sender.clear();
                    for l in &mut g.lp {
                        l.clear();
                    }
                }
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
        // candidate outputs of a layer at one density for an incoming light; fills cand and cst
        let layer_out =
            |dens: Option<usize>, light: &[u16; 8], cand: &mut [u16], cst: &mut [i32]| {
                for k in 0..fan {
                    let th = dens.map_or(0, |d| sur.thr[d][1][cur_t * fan + k]);
                    let mut ck = 0;
                    for q in 0..8 {
                        let e = k * 8 + q;
                        let pv = if mhas[e] && mrank[e] >= th {
                            mval[e]
                        } else {
                            light[q]
                        };
                        cand[e] = pv;
                        ck += bc(pv, bits[q]);
                    }
                    cst[k] = ck;
                }
            };
        // ---- reference: the sender alone
        {
            let pick = argmin(&reference.sum);
            layer_out(None, &ptop, &mut cand, &mut cst);
            r_total += i64::from(cst[pick]);
            r_on += u64::from(cur_t * fan + pick == own_m);
            reference.push(&cst, r_age % WINDOW);
            r_age += 1;
        }
        // ---- the arms
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            let slot = s.age % WINDOW;
            let mut light = ptop;
            let mut last_cost = 0i32;
            let mut last_pick = 0usize;
            let mut any_change = false;
            for g in &mut s.stages {
                // the sender's provisional table and the loop layers' tables, from the past
                let p0 = argmin(&g.sender.sum);
                let picks = [
                    argmin(&g.lp[0].sum),
                    argmin(&g.lp[1].sum),
                    argmin(&g.lp[2].sum),
                ];
                let fin = if arm.gate {
                    if picks[0] == picks[1] && picks[1] == picks[2] && picks[0] != p0 {
                        picks[0]
                    } else {
                        p0
                    }
                } else {
                    for k in 0..fan {
                        total_sum[k] =
                            g.sender.sum[k] + g.lp[0].sum[k] + g.lp[1].sum[k] + g.lp[2].sum[k];
                    }
                    argmin(&total_sum)
                };
                any_change |= fin != p0;
                // the sender's candidates on the incoming light
                layer_out(None, &light, &mut cand, &mut cst);
                g.sender.push(&cst, slot);
                let stage_cost = cst[fin];
                let mut out = [0u16; 8];
                out.copy_from_slice(&cand[fin * 8..fin * 8 + 8]);
                // the side loop: the provisional light goes through the three layers
                let mut loop_light = [0u16; 8];
                loop_light.copy_from_slice(&cand[p0 * 8..p0 * 8 + 8]);
                for l in 0..3 {
                    layer_out(arm.loop_dens[l], &loop_light, &mut cand, &mut cst);
                    g.lp[l].push(&cst, slot);
                    loop_light.copy_from_slice(&cand[picks[l] * 8..picks[l] * 8 + 8]);
                }
                light = out;
                last_cost = stage_cost;
                last_pick = fin;
            }
            s.total += i64::from(last_cost);
            s.on_mid += u64::from(cur_t * fan + last_pick == own_m);
            s.changed += u64::from(any_change);
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
    Result {
        total_bytes: mix.len(),
        reference_bytes: (r_total + 1024) / 2048,
        reference_own: r_on,
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
        changed: st.iter().map(|s| s.changed).collect(),
    }
}

/// The ten arms of the spec.
pub fn arms() -> Vec<Arm> {
    let kinds: [(&str, [Option<usize>; 3]); 4] = [
        ("NNN", [None, None, None]),
        ("PPP", [Some(1), Some(1), Some(1)]),
        ("HOURGLASS", [Some(3), Some(5), Some(3)]),
        ("DEEP", [Some(5), Some(5), Some(5)]),
    ];
    let mut v = Vec::new();
    for (name, d) in kinds {
        for gate in [false, true] {
            v.push(Arm {
                name: format!("{name}_{}_1_stage", if gate { "GATE" } else { "SUM" }),
                loop_dens: d,
                gate,
                stages: 1,
            });
        }
    }
    for gate in [false, true] {
        v.push(Arm {
            name: format!("HOURGLASS_{}_3_stages", if gate { "GATE" } else { "SUM" }),
            loop_dens: [Some(3), Some(5), Some(3)],
            gate,
            stages: 3,
        });
    }
    v
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
    fn loops_keep_their_invariants() {
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
        assert_eq!(a.len(), 10);
        let r = run(&net, &sur, (&ids, &bytes), &a);
        // the reference is the full network with box fallback of the pressure module (same tie rule)
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
        assert_eq!(r.reference_bytes, base);
        for x in 0..a.len() {
            assert!(r.bytes[x] > 0);
            assert!(r.changed[x] as usize <= r.total_bytes);
        }
        let again = run(&net, &sur, (&ids, &bytes), &a);
        assert_eq!(r.bytes, again.bytes);
    }
}
