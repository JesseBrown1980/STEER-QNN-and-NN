//! A lazy guide (panel V47): keep one inner table, score only it and the box table at each byte,
//! and rescan all tables of the box only when unsure. The idea comes from decoders of quantum
//! error correction (a fast decoder with a slower exact one when it fails). Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network};
use crate::surgery::split;

pub const HIST: usize = 256;

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    /// Some(T): rescan when the table is not cheaper than the box table by more than T over the window
    pub unsure: Option<i64>,
    /// do not rescan again for this many bytes (lazy arms)
    pub cooldown: usize,
    /// Some(P): rescan every P bytes regardless (the sham)
    pub period: Option<usize>,
}

pub struct Result {
    pub total_bytes: usize,
    pub full_bytes: i64,
    pub full_own_inner: u64,
    pub bytes: Vec<i64>,
    pub effort: Vec<u64>,
    pub rescans: Vec<u64>,
    pub own_inner: Vec<u64>,
}

struct State {
    table: usize,
    cur_hist: Vec<i32>,
    box_hist: Vec<i32>,
    cur_sum: i64,
    box_sum: i64,
    since_scan: usize,
    total: i64,
    effort: u64,
    rescans: u64,
    on_mid: u64,
}

/// FULL (box window `box_window`, inner window `inner_window`, all tables scored at every byte)
/// and every arm, on the tails of blocks j mod 9 = rem.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(
    net: &Network,
    rem: usize,
    arms: &[Arm],
    box_window: usize,
    inner_window: usize,
) -> Result {
    let fan = net.cfg.fan;
    let tail = net.cfg.tail;
    let cost = cost_table();
    let bc = |p: u16, bit: u8| -> i32 {
        if bit == 1 {
            cost[p as usize]
        } else {
            cost[4096 - p as usize]
        }
    };
    let (ids, mix) = split(net, rem);
    // ring of the last HIST bytes: their eight contexts and bits
    let mut ring_idx = vec![[0u32; 8]; HIST];
    let mut ring_bits = vec![[0u8; 8]; HIST];
    let mut filled = 0usize;
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * box_window];
    let mut f_sum = vec![0i64; fan];
    let mut f_hist = vec![0i32; fan * inner_window];
    let mut f_age = 0usize;
    let (mut f_total, mut f_on) = (0i64, 0u64);
    let mut st: Vec<State> = arms
        .iter()
        .map(|_| State {
            table: 0,
            cur_hist: vec![0; HIST],
            box_hist: vec![0; HIST],
            cur_sum: 0,
            box_sum: 0,
            since_scan: 0,
            total: 0,
            effort: 0,
            rescans: 0,
            on_mid: 0,
        })
        .collect();
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut scan = vec![0i64; fan];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    // cost of one inner table (box fallback) and of the box table alone, for a stored byte
    let table_cost = |id: usize, top: usize, idx: &[u32; 8], bits: &[u8; 8]| -> (i32, i32) {
        let (t, tp) = (&net.mids[id], &net.tops[top]);
        let (mut ck, mut cb) = (0, 0);
        for q in 0..8 {
            let qv = find(tp, idx[q]).map_or(2048, |p| tp.vals[p]);
            let pv = find(t, idx[q]).map_or(qv, |p| t.vals[p]);
            ck += bc(pv, bits[q]);
            cb += bc(qv, bits[q]);
        }
        (ck, cb)
    };
    for (i, &b) in mix.iter().enumerate() {
        let own = ids[i / tail];
        let own_m = own / fan;
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        let slot = i % HIST;
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            ring_idx[slot][q] = (base + node) as u32;
            ring_bits[slot][q] = bit;
            node = (node << 1) | bit as usize;
        }
        let (idx, bits) = (ring_idx[slot], ring_bits[slot]);
        let a = argmin(&a_sum);
        let box_changed = a != cur_t;
        if box_changed {
            cur_t = a;
            f_sum.fill(0);
            f_hist.fill(0);
            f_age = 0;
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
        // ---- FULL: every table of the box at every byte
        let pick = argmin(&f_sum);
        let w = f_age % inner_window;
        for k in 0..fan {
            let t = &net.mids[cur_t * fan + k];
            let mut ck = 0;
            for q in 0..8 {
                let pv = find(t, idx[q]).map_or(ptop[q], |p| t.vals[p]);
                ck += bc(pv, bits[q]);
            }
            if k == pick {
                f_total += i64::from(ck);
            }
            let h = k * inner_window + w;
            f_sum[k] += i64::from(ck - f_hist[h]);
            f_hist[h] = ck;
        }
        f_age += 1;
        f_on += u64::from(cur_t * fan + pick == own_m);
        // ---- the arms
        let have = filled.min(HIST); // stored bytes before this one
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            let unsure = arm
                .unsure
                .is_some_and(|t| s.since_scan >= arm.cooldown && s.box_sum - s.cur_sum <= t);
            let due = arm.period.is_some_and(|p| s.since_scan >= p);
            if (box_changed || unsure || due) && have > 0 {
                // rescan: all tables of the box over the stored bytes
                scan.fill(0);
                s.cur_hist.fill(0);
                s.box_hist.fill(0);
                let mut box_total = 0i64;
                let mut per_slot = vec![0i32; have];
                for back in 1..=have {
                    let sl = (i + HIST - back) % HIST;
                    let mut box_c = 0;
                    for k in 0..fan {
                        let (ck, cb) =
                            table_cost(cur_t * fan + k, cur_t, &ring_idx[sl], &ring_bits[sl]);
                        scan[k] += i64::from(ck);
                        box_c = cb;
                    }
                    s.box_hist[sl] = box_c;
                    box_total += i64::from(box_c);
                    per_slot[back - 1] = box_c;
                }
                s.table = argmin(&scan);
                // the chosen table's stored costs, for the rolling comparison
                let mut cur_total = 0i64;
                for back in 1..=have {
                    let sl = (i + HIST - back) % HIST;
                    let (ck, _) =
                        table_cost(cur_t * fan + s.table, cur_t, &ring_idx[sl], &ring_bits[sl]);
                    s.cur_hist[sl] = ck;
                    cur_total += i64::from(ck);
                }
                s.cur_sum = cur_total;
                s.box_sum = box_total;
                s.effort += (have * (fan + 2)) as u64;
                s.rescans += 1;
                s.since_scan = 0;
            } else if box_changed {
                s.table = 0;
                s.cur_hist.fill(0);
                s.box_hist.fill(0);
                s.cur_sum = 0;
                s.box_sum = 0;
                s.since_scan = 0;
            }
            // this byte: only the kept table and the box table
            let (ck, cb) = table_cost(cur_t * fan + s.table, cur_t, &idx, &bits);
            s.total += i64::from(ck);
            s.effort += 2;
            s.on_mid += u64::from(cur_t * fan + s.table == own_m);
            s.cur_sum += i64::from(ck - s.cur_hist[slot]);
            s.cur_hist[slot] = ck;
            s.box_sum += i64::from(cb - s.box_hist[slot]);
            s.box_hist[slot] = cb;
            s.since_scan += 1;
        }
        filled += 1;
        let wb = i % box_window;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * box_window + wb]);
            a_hist[j * box_window + wb] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    Result {
        total_bytes: mix.len(),
        full_bytes: (f_total + 1024) / 2048,
        full_own_inner: f_on,
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        effort: st.iter().map(|s| s.effort).collect(),
        rescans: st.iter().map(|s| s.rescans).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
    }
}

/// The 8 lazy arms and the 3 periodic shams of the spec.
pub fn arms() -> Vec<Arm> {
    let mut v = Vec::new();
    for t in [0i64, 256, 1024, 4096] {
        for c in [32usize, 128] {
            v.push(Arm {
                name: format!("LAZY_T{t}_C{c}"),
                unsure: Some(t),
                cooldown: c,
                period: None,
            });
        }
    }
    for p in [64usize, 256, 1024] {
        v.push(Arm {
            name: format!("PERIODIC_P{p}"),
            unsure: None,
            cooldown: 0,
            period: Some(p),
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pressure_boxes::{build, Config, Source};

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
    fn lazy_guide_counts_its_effort() {
        let cfg = Config {
            fan: 3,
            block: 6000,
            tail: 500,
            tail_mod: 3,
            tail_rem: 1,
            skip_rem: None,
        };
        let net = build(cfg, &mut Synthetic { block: cfg.block }).expect("build");
        let a = arms();
        assert_eq!(a.len(), 11);
        let r = run(&net, 1, &a, 64, 64);
        assert_eq!(r.total_bytes, 9 * 500);
        assert!(r.full_bytes > 0);
        for x in 0..a.len() {
            // at least the kept table and the box table at every byte
            assert!(r.effort[x] >= 2 * r.total_bytes as u64);
            assert!(r.bytes[x] > 0);
        }
        // a sham that rescans more often spends more effort
        assert!(r.effort[8] >= r.effort[9] && r.effort[9] >= r.effort[10]);
        assert!(r.rescans[8] >= r.rescans[10]);
        let again = run(&net, 1, &a, 64, 64);
        assert_eq!(r.bytes, again.bytes);
    }
}
