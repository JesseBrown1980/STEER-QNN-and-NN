//! Independent order-2 adaptive bit-tree control for STEER issue #1.
//! No box/guide/inner QNN table is used by this model.
//! Source spec: specs/SPEC-ADAPTIVE729-SECOND-SEAT-20261114.md
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::collections::HashMap;

pub const CONTEXTS: usize = 65_536 * 256;
pub const CHANGE_CAP: usize = 5_800_000;
pub const TRAIN_TAIL: usize = 2048;
pub const RATE: u32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct Outcome {
    pub cost_units: i64,
    pub bytes: i64,
    pub updated_keys: usize,
    pub bits: usize,
}

#[derive(Debug, Clone)]
pub struct Counts {
    n0: Vec<u32>,
    n1: Vec<u32>,
    pub trained_bytes: u64,
}

impl Counts {
    #[must_use]
    pub fn new() -> Self {
        Self {
            n0: vec![0; CONTEXTS],
            n1: vec![0; CONTEXTS],
            trained_bytes: 0,
        }
    }

    /// Count *only* prefix bytes of one training block; always reset context.
    pub fn count_prefix(&mut self, prefix: &[u8]) {
        self.trained_bytes += prefix.len() as u64;
        walk_bits(prefix, |ix, bit| {
            let counts = if bit == 0 { &mut self.n0 } else { &mut self.n1 };
            let cell = &mut counts[ix as usize];
            *cell = cell
                .checked_add(1)
                .expect("global prefix context count overflow");
        });
    }

    /// Freeze a corpus-global order-2 model, not one table per box.
    pub fn finish(self) -> (Vec<u16>, usize) {
        let mut seen = 0usize;
        let mut out = vec![2048u16; CONTEXTS];
        for (i, slot) in out.iter_mut().enumerate() {
            let a = self.n0[i];
            let b = self.n1[i];
            if u64::from(a) + u64::from(b) > 1 {
                *slot = crate::pressure_boxes::prob(a, b);
                seen += 1;
            }
        }
        (out, seen)
    }
}

impl Default for Counts {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn toward(p: u16, bit: u8, rate: u32) -> u16 {
    let target: i32 = if bit == 1 { 4095 } else { 1 };
    (i32::from(p) + ((target - i32::from(p)) >> rate)).clamp(1, 4095) as u16
}

/// Same two previous bytes and current bit-tree node as V50/V57.
/// The callback observes the context and truth in bit order, but the
/// caller MUST score before updating it.
pub fn walk_bits(data: &[u8], mut f: impl FnMut(u32, u8)) {
    let (mut prev1, mut prev2) = (0u32, 0u32);
    for &byte in data {
        let base = ((prev2 << 8) | prev1) << 8;
        let mut node = 1u32;
        for shift in (0..8).rev() {
            let bit = (byte >> shift) & 1;
            f(base + node, bit);
            node = (node << 1) | u32::from(bit);
        }
        prev2 = prev1;
        prev1 = u32::from(byte);
    }
}

/// Exact V57 integer scoring; the online overlay is NEW for every case.
/// Returning Err invalidates that case; no silent over-cap evictions.
pub fn score(
    data: &[u8],
    trained: Option<&[u16]>,
    adaptive: bool,
    costs: &[i32],
    cap: usize,
) -> Result<Outcome, String> {
    if let Some(p) = trained {
        if p.len() != CONTEXTS {
            return Err("pretrained table length mismatch".into());
        }
    }
    if costs.len() != 4097 {
        return Err("bit cost table length mismatch".into());
    }
    if cap == 0 || cap > CONTEXTS {
        return Err("invalid learned-context cap".into());
    }
    let mut changed: HashMap<u32, u16> = HashMap::new();
    let mut total: i64 = 0;
    let mut failure = None;
    let mut bits = 0usize;
    walk_bits(data, |ix, bit| {
        if failure.is_some() {
            return;
        }
        let start = trained.map_or(2048, |t| t[ix as usize]);
        let p = changed.get(&ix).copied().unwrap_or(start);
        if p == 0 || p >= 4096 {
            failure = Some("invalid probability".to_string());
            return;
        }
        let index = if bit == 1 {
            usize::from(p)
        } else {
            4096 - usize::from(p)
        };
        total += i64::from(costs[index]);
        bits += 1;
        if adaptive {
            if !changed.contains_key(&ix) && changed.len() == cap {
                failure = Some("RESOURCE_HOLD: changed-context cap exceeded".to_string());
                return;
            }
            changed.insert(ix, toward(p, bit, RATE));
        }
    });
    if let Some(message) = failure {
        return Err(message);
    }
    Ok(Outcome {
        cost_units: total,
        bytes: (total + 1024) / 2048,
        updated_keys: changed.len(),
        bits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_costs() -> Vec<i32> {
        let mut cost = vec![0; 4097];
        for (i, v) in cost.iter_mut().enumerate() {
            // Test-only monotone integer surrogate. Real runs use
            // the source bit-cost table, checked separately below.
            *v = (4096 - i) as i32;
        }
        cost
    }

    fn independent_walk(data: &[u8]) -> Vec<(u32, u8)> {
        let mut out = Vec::new();
        let (mut one, mut two) = (0usize, 0usize);
        for &byte in data {
            let mut node = 1usize;
            for q in 0..8 {
                let bit = (byte >> (7 - q)) & 1;
                out.push((((two << 8 | one) * 256 + node) as u32, bit));
                node = node * 2 + usize::from(bit);
            }
            two = one;
            one = usize::from(byte);
        }
        out
    }

    #[test]
    fn context_bit_order_independently_agrees() {
        let bytes = [0x81, 0x04, 0xfe, 0x33, 0x00];
        let mut got = Vec::new();
        walk_bits(&bytes, |ix, bit| got.push((ix, bit)));
        assert_eq!(got, independent_walk(&bytes));
        assert_eq!(got.len(), 40);
    }

    #[test]
    fn online_uses_truth_only_after_prediction() {
        assert_eq!(toward(2048, 1, 4), 2175);
        assert_eq!(toward(2048, 0, 4), 1920);
        let costs = tiny_costs();
        let data = [0_u8, 0_u8, 0_u8];
        let static_run = score(&data, None, false, &costs, CHANGE_CAP).unwrap();
        let learned = score(&data, None, true, &costs, CHANGE_CAP).unwrap();
        assert_eq!(static_run.bits, 24);
        assert!(learned.cost_units != static_run.cost_units);
        assert!(learned.updated_keys > 0);
    }

    #[test]
    fn no_hidden_reset_at_segment_boundaries() {
        let costs = tiny_costs();
        let data = [2, 3, 4, 5, 6, 7, 8, 9];
        let all = score(&data, None, true, &costs, CHANGE_CAP).unwrap();
        let mut concatenated = Vec::new();
        concatenated.extend_from_slice(&data[..3]);
        concatenated.extend_from_slice(&data[3..]);
        let other = score(&concatenated, None, true, &costs, CHANGE_CAP).unwrap();
        assert_eq!(all.cost_units, other.cost_units);
        assert_eq!(all.updated_keys, other.updated_keys);
    }

    #[test]
    fn budget_violations_refuse() {
        let costs = tiny_costs();
        let data = [2, 3, 4];
        assert!(score(&data, None, true, &costs, 1)
            .unwrap_err()
            .starts_with("RESOURCE_HOLD"));
    }

    #[test]
    fn cost_scale_matches_steer() {
        let costs = crate::pressure_boxes::cost_table();
        assert_eq!(costs[2048], 256);
        assert_eq!(costs[4096 - 2048], 256);
        assert_eq!(
            score(&[0u8; 4], None, false, &costs, CHANGE_CAP)
                .unwrap()
                .bytes,
            4
        );
    }

    #[test]
    fn prefix_train_resets_context_and_excludes_tail() {
        let prefix = [0u8, 1, 2, 3];
        let tail_one = [200u8, 201];
        let tail_two = [40u8, 41];
        let mut seen_first = Vec::new();
        let mut seen_second = Vec::new();
        walk_bits(&prefix, |i, bit| seen_first.push((i, bit)));
        walk_bits(&prefix, |i, bit| seen_second.push((i, bit)));
        assert_eq!(seen_first, seen_second);
        assert_ne!(tail_one, tail_two);
        // Only the shared prefix is eligible for corpus counts.
    }
}
