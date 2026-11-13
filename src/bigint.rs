//! A minimal signed big integer: sign and little-endian u32 limbs. Integer only.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Big {
    neg: bool,
    mag: Vec<u32>,
}

fn trim(v: &mut Vec<u32>) {
    while v.last() == Some(&0) {
        v.pop();
    }
}

fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    Ordering::Equal
}

fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let mut carry = 0u64;
    for i in 0..a.len().max(b.len()) {
        let s = u64::from(*a.get(i).unwrap_or(&0)) + u64::from(*b.get(i).unwrap_or(&0)) + carry;
        out.push(s as u32);
        carry = s >> 32;
    }
    if carry > 0 {
        out.push(carry as u32);
    }
    out
}

/// a - b with |a| >= |b|
fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0i64;
    for (i, &ai) in a.iter().enumerate() {
        let mut d = i64::from(ai) - i64::from(*b.get(i).unwrap_or(&0)) - borrow;
        if d < 0 {
            d += 1 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(d as u32);
    }
    trim(&mut out);
    out
}

impl Big {
    pub fn zero() -> Self {
        Big {
            neg: false,
            mag: Vec::new(),
        }
    }
    pub fn from_i128(x: i128) -> Self {
        let neg = x < 0;
        let mut m = x.unsigned_abs();
        let mut mag = Vec::new();
        while m > 0 {
            mag.push(m as u32);
            m >>= 32;
        }
        Big { neg, mag }
    }
    pub fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }
    /// -1, 0 or +1
    pub fn sign(&self) -> i8 {
        if self.mag.is_empty() {
            0
        } else if self.neg {
            -1
        } else {
            1
        }
    }
    pub fn neg(&self) -> Self {
        if self.is_zero() {
            self.clone()
        } else {
            Big {
                neg: !self.neg,
                mag: self.mag.clone(),
            }
        }
    }
    pub fn add(&self, o: &Big) -> Big {
        if self.neg == o.neg {
            return Big {
                neg: self.neg,
                mag: add_mag(&self.mag, &o.mag),
            };
        }
        match cmp_mag(&self.mag, &o.mag) {
            Ordering::Equal => Big::zero(),
            Ordering::Greater => Big {
                neg: self.neg,
                mag: sub_mag(&self.mag, &o.mag),
            },
            Ordering::Less => Big {
                neg: o.neg,
                mag: sub_mag(&o.mag, &self.mag),
            },
        }
    }
    pub fn sub(&self, o: &Big) -> Big {
        self.add(&o.neg())
    }
    pub fn mul(&self, o: &Big) -> Big {
        if self.is_zero() || o.is_zero() {
            return Big::zero();
        }
        let mut out = vec![0u64; self.mag.len() + o.mag.len() + 1];
        for (i, &a) in self.mag.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in o.mag.iter().enumerate() {
                let t = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = t & 0xFFFF_FFFF;
                carry = t >> 32;
            }
            let mut k = i + o.mag.len();
            while carry > 0 {
                let t = out[k] + carry;
                out[k] = t & 0xFFFF_FFFF;
                carry = t >> 32;
                k += 1;
            }
        }
        let mut mag: Vec<u32> = out.into_iter().map(|x| x as u32).collect();
        trim(&mut mag);
        Big {
            neg: self.neg != o.neg,
            mag,
        }
    }
    pub fn mul_i128(&self, x: i128) -> Big {
        self.mul(&Big::from_i128(x))
    }
    /// floor(x / 2) for x >= 0
    pub fn half(&self) -> Big {
        assert!(!self.neg, "half of a negative");
        let mut mag = self.mag.clone();
        let mut carry = 0u32;
        for limb in mag.iter_mut().rev() {
            let next = *limb & 1;
            *limb = (*limb >> 1) | (carry << 31);
            carry = next;
        }
        trim(&mut mag);
        Big { neg: false, mag }
    }
    /// 2^e
    pub fn pow2(e: u32) -> Big {
        let mut mag = vec![0u32; (e / 32) as usize];
        mag.push(1u32 << (e % 32));
        Big { neg: false, mag }
    }
    /// decimal digits, with a leading '-' when negative
    pub fn dec(&self) -> String {
        if self.is_zero() {
            return "0".into();
        }
        let mut mag = self.mag.clone();
        let mut chunks = Vec::new();
        while !mag.is_empty() {
            let mut rem = 0u64;
            for limb in mag.iter_mut().rev() {
                let cur = (rem << 32) | u64::from(*limb);
                *limb = (cur / 1_000_000_000) as u32;
                rem = cur % 1_000_000_000;
            }
            trim(&mut mag);
            chunks.push(rem);
        }
        let mut out = String::new();
        if self.neg {
            out.push('-');
        }
        out.push_str(&chunks.pop().unwrap_or(0).to_string());
        while let Some(c) = chunks.pop() {
            out.push_str(&format!("{c:09}"));
        }
        out
    }
    /// number of significant bits of |x|; 0 for zero
    pub fn bits(&self) -> usize {
        match self.mag.last() {
            None => 0,
            Some(&top) => 32 * (self.mag.len() - 1) + (32 - top.leading_zeros() as usize),
        }
    }
}
