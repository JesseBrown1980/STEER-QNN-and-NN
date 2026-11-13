//! Read the weights, then edit them (panel V54). Two readings from the tables alone:
//! INFO = weight x relative entropy of an entry against its box value (what the entry adds once
//! the box is projected out), and the distance from every table to its nearest sibling.
//! Two edits: density cuts by INFO instead of by weight, and aliasing a redundant table to its
//! nearest sibling. Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, h16, Network, Surgery, DENS};

/// density indices into DENS used here: 1/4, 1/16, 1/64, 1/256
pub const CUTS: [usize; 4] = [1, 3, 4, 5];
const WINDOW: usize = 64;

pub struct Reading {
    /// per table, per entry: INFO rank (information << 8 | hash)
    pub info: Vec<Vec<u32>>,
    /// per cut: per-table thresholds, SEALED then OPEN
    pub info_thr: Vec<[Vec<u64>; 2]>,
    /// per table: (nearest sibling, distance x1000 of 4096, total weight)
    pub nearest: Vec<(usize, u64)>,
    pub rows: Vec<String>,
}

fn kth_from_top(sorted: &[u32], keep: usize) -> u64 {
    if keep > 0 {
        u64::from(sorted[sorted.len() - keep])
    } else {
        1 << 32
    }
}

/// Both readings, from the tables alone.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn read(net: &Network, sur: &Surgery) -> Reading {
    let fan = net.cfg.fan;
    let cost = cost_table();
    let mut rows = Vec::new();
    // ---- R1: the information of every entry against its box value
    let mut info: Vec<Vec<u32>> = Vec::with_capacity(net.mids.len());
    let mut raw_total = 0u128;
    for (id, t) in net.mids.iter().enumerate() {
        let top = &net.tops[id / fan];
        let mut v = Vec::with_capacity(t.keys.len());
        for (i, &key) in t.keys.iter().enumerate() {
            let p = i64::from(t.vals[i]);
            let q = i64::from(find(top, key).map_or(2048, |x| top.vals[x]));
            let c = i64::from(t.rank[i] >> 16);
            // relative entropy in 1/256 bit: p*(cost(q)-cost(p)) + (1-p)*(cost(1-q)-cost(1-p)), probabilities over 4096
            let kl = (p * i64::from(cost[q as usize] - cost[p as usize])
                + (4096 - p) * i64::from(cost[(4096 - q) as usize] - cost[(4096 - p) as usize]))
                / 4096;
            let bits = (c * kl.max(0)).min((1 << 24) - 1) as u32;
            raw_total += u128::from(bits);
            v.push((bits << 8) | (h16(key) & 255));
        }
        info.push(v);
    }
    let total: usize = info.iter().map(Vec::len).sum();
    let mut all: Vec<u32> = Vec::with_capacity(total);
    for v in &info {
        all.extend_from_slice(v);
    }
    all.sort_unstable();
    let box_size: Vec<usize> = (0..fan)
        .map(|b| (0..fan).map(|k| info[b * fan + k].len()).sum())
        .collect();
    let mut info_thr = Vec::new();
    for &d in &CUTS {
        let du = DENS[d] as usize;
        let g = kth_from_top(&all, total / du);
        let mut sealed = vec![0u64; fan * fan];
        for b in 0..fan {
            let mut bx: Vec<u32> = Vec::with_capacity(box_size[b]);
            for k in 0..fan {
                bx.extend_from_slice(&info[b * fan + k]);
            }
            bx.sort_unstable();
            let th = kth_from_top(&bx, box_size[b] / du);
            for k in 0..fan {
                sealed[b * fan + k] = th;
            }
        }
        // how much of the total information each ranking keeps at this density (OPEN)
        let wth = &sur.thr[d][2];
        let (mut kept_by_info, mut kept_by_weight) = (0u128, 0u128);
        for (id, t) in net.mids.iter().enumerate() {
            for (i, &r) in info[id].iter().enumerate() {
                let bits = u128::from(r >> 8);
                if u64::from(r) >= g {
                    kept_by_info += bits;
                }
                if u64::from(t.rank[i]) >= wth[id] {
                    kept_by_weight += bits;
                }
            }
        }
        rows.push(format!(
            "READ_INFO|density=1/{}|share_of_total_information_kept_permille_by_INFO_ranking={}|by_WEIGHT_ranking={}|json=0",
            DENS[d],
            (1000 * kept_by_info) / raw_total.max(1),
            (1000 * kept_by_weight) / raw_total.max(1)
        ));
        info_thr.push([sealed, vec![g; fan * fan]]);
    }
    // ---- R2: the nearest sibling of every table
    let mut nearest = vec![(0usize, u64::MAX); fan * fan];
    for b in 0..fan {
        let top = &net.tops[b];
        for a in 0..fan {
            let ta = &net.mids[b * fan + a];
            let mut num = vec![0u128; fan];
            let mut den = 0u128;
            for (i, &key) in ta.keys.iter().enumerate() {
                let w = u128::from(ta.rank[i] >> 16);
                let q = find(top, key).map_or(2048, |x| top.vals[x]);
                den += w;
                for o in 0..fan {
                    if o != a {
                        let tb = &net.mids[b * fan + o];
                        let v = find(tb, key).map_or(q, |x| tb.vals[x]);
                        num[o] += w * u128::from(ta.vals[i].abs_diff(v));
                    }
                }
            }
            let mut best = (0usize, u64::MAX);
            for o in 0..fan {
                if o != a {
                    let dist = ((1000 * num[o]) / den.max(1)) as u64;
                    if dist < best.1 {
                        best = (b * fan + o, dist);
                    }
                }
            }
            nearest[b * fan + a] = best;
        }
    }
    let mut d: Vec<u64> = nearest.iter().map(|n| n.1).collect();
    d.sort_unstable();
    rows.push(format!(
        "READ_SIBLINGS|unit=weight-averaged_|p_a-v_b|_x1000_of_4096|smallest_distance_to_a_sibling={}|81st_smallest={}|243rd_smallest={}|median={}|largest={}|json=0",
        d[0],
        d[80.min(d.len() - 1)],
        d[242.min(d.len() - 1)],
        d[d.len() / 2],
        d[d.len() - 1]
    ));
    Reading {
        info,
        info_thr,
        nearest,
        rows,
    }
}

#[derive(Clone)]
pub enum Edit {
    Base,
    /// (cut index into CUTS, rule 0 = SEALED / 1 = OPEN, by information instead of weight)
    Cut(usize, usize, bool),
    /// per table: which table of the same box answers in its place (itself = unchanged)
    Alias(Vec<usize>),
    /// per table: answered by the box table alone
    FuseBox(Vec<bool>),
}

pub struct Arm {
    pub name: String,
    pub edit: Edit,
}

pub struct Outcome {
    pub total_bytes: usize,
    pub bytes: Vec<i64>,
    pub own_inner: Vec<u64>,
}

struct State {
    sum: Vec<i64>,
    hist: Vec<i32>,
    age: usize,
    total: i64,
    on_mid: u64,
}

/// Every arm on a text: `text.0[i]` is the block of the i-th tail of `text.1`.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(
    net: &Network,
    sur: &Surgery,
    rd: &Reading,
    text: (&[usize], &[u8]),
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
            total: 0,
            on_mid: 0,
        })
        .collect();
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut mpos: Vec<Option<usize>> = vec![None; fan * 8];
    let mut full = vec![0i32; fan];
    let mut cst = vec![0i32; fan];
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
            let mut ck = 0;
            for q in 0..8 {
                let p = find(t, idx[q]);
                mpos[k * 8 + q] = p;
                ck += bc(p.map_or(ptop[q], |x| t.vals[x]), bits[q]);
            }
            full[k] = ck;
        }
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            let pick = argmin(&s.sum);
            match &arm.edit {
                Edit::Base => cst.copy_from_slice(&full),
                Edit::Cut(ci, rule, by_info) => {
                    for k in 0..fan {
                        let id = cur_t * fan + k;
                        let t = &net.mids[id];
                        let th = if *by_info {
                            rd.info_thr[*ci][*rule][id]
                        } else {
                            sur.thr[CUTS[*ci]][*rule + 1][id]
                        };
                        let mut ck = 0;
                        for q in 0..8 {
                            let pv = match mpos[k * 8 + q] {
                                Some(p) => {
                                    let r = if *by_info { rd.info[id][p] } else { t.rank[p] };
                                    if u64::from(r) >= th {
                                        t.vals[p]
                                    } else {
                                        ptop[q]
                                    }
                                }
                                None => ptop[q],
                            };
                            ck += bc(pv, bits[q]);
                        }
                        cst[k] = ck;
                    }
                }
                Edit::Alias(map) => {
                    for k in 0..fan {
                        cst[k] = full[map[cur_t * fan + k] % fan];
                    }
                }
                Edit::FuseBox(gone) => {
                    for k in 0..fan {
                        cst[k] = if gone[cur_t * fan + k] {
                            acost[cur_t]
                        } else {
                            full[k]
                        };
                    }
                }
            }
            s.total += i64::from(cst[pick]);
            s.on_mid += u64::from(cur_t * fan + pick == own_m);
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
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
    }
}

/// BASE, the sixteen cuts (density x SEALED/OPEN x WEIGHT/INFO) and the alias and fusion arms.
pub fn arms(net: &Network, rd: &Reading) -> Vec<Arm> {
    let fan = net.cfg.fan;
    let nm = fan * fan;
    let mut v = vec![Arm {
        name: "BASE".to_string(),
        edit: Edit::Base,
    }];
    for (ci, &d) in CUTS.iter().enumerate() {
        for (rule, rn) in ["SEALED", "OPEN"].iter().enumerate() {
            for (by_info, bn) in [(false, "WEIGHT"), (true, "INFO")] {
                v.push(Arm {
                    name: format!("CUT_1/{}_{bn}_{rn}", DENS[d]),
                    edit: Edit::Cut(ci, rule, by_info),
                });
            }
        }
    }
    let mut order: Vec<usize> = (0..nm).collect();
    order.sort_by_key(|&i| (rd.nearest[i].1, i));
    for f in [81usize, 243] {
        let chosen = &order[..f.min(nm)];
        let mut nearest: Vec<usize> = (0..nm).collect();
        let mut random: Vec<usize> = (0..nm).collect();
        let mut gone = vec![false; nm];
        for &t in chosen {
            nearest[t] = rd.nearest[t].0;
            let b = t / fan;
            let mut o = (h16(t as u32 ^ 0x1b87_3593) as usize) % fan;
            if b * fan + o == t {
                o = (o + 1) % fan;
            }
            random[t] = b * fan + o;
            gone[t] = true;
        }
        v.push(Arm {
            name: format!("ALIAS_NEAREST_{f}"),
            edit: Edit::Alias(nearest),
        });
        v.push(Arm {
            name: format!("ALIAS_RANDOM_{f}"),
            edit: Edit::Alias(random),
        });
        v.push(Arm {
            name: format!("FUSE_BOX_{f}"),
            edit: Edit::FuseBox(gone),
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
    fn reading_and_edits_keep_their_invariants() {
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
        assert_eq!(rd.info.len(), 9);
        assert_eq!(rd.rows.len(), CUTS.len() + 1);
        // a table's nearest sibling is another table of the same box
        for (t, n) in rd.nearest.iter().enumerate() {
            assert!(n.0 != t && n.0 / 3 == t / 3);
        }
        let (ids, bytes) = split(&net, 1);
        let a = arms(&net, &rd);
        assert_eq!(a.len(), 1 + 16 + 6);
        let r = run(&net, &sur, &rd, (&ids, &bytes), &a);
        // BASE is the full network with box fallback; a weight cut matches the pressure module's arm
        let rows = evaluate(&net, &sur, &[3]);
        let get = |prefix: &str, field: &str| -> i64 {
            rows.iter()
                .find(|r| r.starts_with(prefix))
                .and_then(|r| r.split('|').find_map(|f| f.strip_prefix(field)))
                .and_then(|v| v.parse().ok())
                .expect("row")
        };
        assert_eq!(r.bytes[0], get("RUN|", "baseline_PARENT_cost_bytes="));
        assert_eq!(
            r.bytes[1],
            get("ARM|d=1/4|rule=SEALED|fallback=PARENT|", "bytes=")
        );
        assert_eq!(
            r.bytes[3],
            get("ARM|d=1/4|rule=OPEN|fallback=PARENT|", "bytes=")
        );
        assert!(r.bytes.iter().all(|&b| b > 0));
    }
}
