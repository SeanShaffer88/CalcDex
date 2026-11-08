//! Minimal unsigned big integer (u32 limbs) for the decimal engine.
//! Only what the decimal code needs; no panics on any input.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Big(Vec<u32>);

const CHUNK: u32 = 1_000_000_000;

impl Big {
    pub fn zero() -> Big {
        Big(Vec::new())
    }

    pub fn from_u64(mut n: u64) -> Big {
        let mut v = Vec::new();
        while n > 0 {
            v.push(n as u32);
            n >>= 32;
        }
        Big(v)
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn trim(&mut self) {
        while let Some(&0) = self.0.last() {
            self.0.pop();
        }
    }

    pub fn is_odd(&self) -> bool {
        self.0.first().is_some_and(|l| l & 1 == 1)
    }

    /// Value as u64 when it fits.
    pub fn to_u64(&self) -> Option<u64> {
        match self.0.len() {
            0 => Some(0),
            1 => Some(self.0[0] as u64),
            2 => Some(self.0[0] as u64 | (self.0[1] as u64) << 32),
            _ => None,
        }
    }

    pub fn add_big(&self, o: &Big) -> Big {
        let n = self.0.len().max(o.0.len());
        let mut r = Vec::with_capacity(n + 1);
        let mut c = 0u64;
        for i in 0..n {
            let s = *self.0.get(i).unwrap_or(&0) as u64 + *o.0.get(i).unwrap_or(&0) as u64 + c;
            r.push(s as u32);
            c = s >> 32;
        }
        if c > 0 {
            r.push(c as u32);
        }
        Big(r)
    }

    /// `self - o`; if `o > self` the result is zero (callers compare first).
    pub fn sub_big(&self, o: &Big) -> Big {
        if *self <= *o {
            return Big::zero();
        }
        let mut r = Vec::with_capacity(self.0.len());
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let mut d = self.0[i] as i64 - borrow - *o.0.get(i).unwrap_or(&0) as i64;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            r.push(d as u32);
        }
        let mut b = Big(r);
        b.trim();
        b
    }

    pub fn mul_big(&self, o: &Big) -> Big {
        if self.is_zero() || o.is_zero() {
            return Big::zero();
        }
        let mut r = vec![0u32; self.0.len() + o.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut c = 0u64;
            for (j, &b) in o.0.iter().enumerate() {
                let t = a as u64 * b as u64 + r[i + j] as u64 + c;
                r[i + j] = t as u32;
                c = t >> 32;
            }
            let mut k = i + o.0.len();
            while c > 0 {
                let t = r[k] as u64 + c;
                r[k] = t as u32;
                c = t >> 32;
                k += 1;
            }
        }
        let mut b = Big(r);
        b.trim();
        b
    }

    pub fn mul_small(&self, m: u32) -> Big {
        if m == 0 || self.is_zero() {
            return Big::zero();
        }
        let mut r = Vec::with_capacity(self.0.len() + 1);
        let mut c = 0u64;
        for &l in &self.0 {
            let t = l as u64 * m as u64 + c;
            r.push(t as u32);
            c = t >> 32;
        }
        if c > 0 {
            r.push(c as u32);
        }
        Big(r)
    }

    pub fn add_small(&self, a: u32) -> Big {
        self.add_big(&Big::from_u64(a as u64))
    }

    /// Quotient and remainder by a non-zero u32 (zero divisor gives (0, 0)).
    pub fn divrem_small(&self, d: u32) -> (Big, u32) {
        if d == 0 {
            return (Big::zero(), 0);
        }
        let mut q = vec![0u32; self.0.len()];
        let mut rem = 0u64;
        for i in (0..self.0.len()).rev() {
            let cur = rem << 32 | self.0[i] as u64;
            q[i] = (cur / d as u64) as u32;
            rem = cur % d as u64;
        }
        let mut b = Big(q);
        b.trim();
        (b, rem as u32)
    }

    pub fn bit_len(&self) -> usize {
        match self.0.last() {
            None => 0,
            Some(&top) => (self.0.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
        }
    }

    fn bit(&self, i: usize) -> bool {
        self.0.get(i / 32).is_some_and(|l| l >> (i % 32) & 1 == 1)
    }

    fn shl1_or(&mut self, bit: bool) {
        let mut c = bit as u32;
        for l in self.0.iter_mut() {
            let nc = *l >> 31;
            *l = *l << 1 | c;
            c = nc;
        }
        if c > 0 {
            self.0.push(c);
        }
    }

    fn sub_assign_ge(&mut self, o: &Big) {
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let mut d = self.0[i] as i64 - borrow - *o.0.get(i).unwrap_or(&0) as i64;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            self.0[i] = d as u32;
        }
        self.trim();
    }

    /// Quotient and remainder. A zero divisor gives (0, 0).
    pub fn divrem_big(&self, d: &Big) -> (Big, Big) {
        if d.is_zero() {
            return (Big::zero(), Big::zero());
        }
        if self < d {
            return (Big::zero(), self.clone());
        }
        if d.0.len() == 1 {
            let (q, r) = self.divrem_small(d.0[0]);
            return (q, Big::from_u64(r as u64));
        }
        let mut q = Big(vec![0u32; self.0.len()]);
        let mut r = Big::zero();
        for i in (0..self.bit_len()).rev() {
            r.shl1_or(self.bit(i));
            if r >= *d {
                r.sub_assign_ge(d);
                q.0[i / 32] |= 1 << (i % 32);
            }
        }
        q.trim();
        (q, r)
    }

    /// 10^n.
    pub fn pow10(n: usize) -> Big {
        Big::from_u64(1).mul_pow10(n)
    }

    /// self * 10^n.
    pub fn mul_pow10(&self, n: usize) -> Big {
        let mut r = self.clone();
        let mut left = n;
        while left >= 9 {
            r = r.mul_small(CHUNK);
            left -= 9;
        }
        if left > 0 {
            r = r.mul_small(10u32.pow(left as u32));
        }
        r
    }

    /// Integer square root (floor).
    pub fn isqrt(&self) -> Big {
        if self.is_zero() {
            return Big::zero();
        }
        let mut x = Big::from_u64(1);
        for _ in 0..self.bit_len().div_ceil(2) {
            x = x.mul_small(2);
        }
        loop {
            let (q, _) = self.divrem_big(&x);
            let y = x.add_big(&q).divrem_small(2).0;
            if y >= x {
                return x;
            }
            x = y;
        }
    }

    pub fn to_dec_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut chunks = Vec::new();
        let mut cur = self.clone();
        while !cur.is_zero() {
            let (q, r) = cur.divrem_small(CHUNK);
            chunks.push(r);
            cur = q;
        }
        let mut s = chunks.pop().map(|c| c.to_string()).unwrap_or_default();
        while let Some(c) = chunks.pop() {
            s.push_str(&format!("{c:09}"));
        }
        s
    }

    pub fn from_dec_str(s: &str) -> Option<Big> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut r = Big::zero();
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let end = (i + 9).min(bytes.len());
            let part: u32 = s[i..end].parse().ok()?;
            r = r.mul_small(10u32.pow((end - i) as u32)).add_small(part);
            i = end;
        }
        Some(r)
    }
}

impl PartialOrd for Big {
    fn partial_cmp(&self, o: &Big) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Big {
    fn cmp(&self, o: &Big) -> Ordering {
        if self.0.len() != o.0.len() {
            return self.0.len().cmp(&o.0.len());
        }
        for i in (0..self.0.len()).rev() {
            if self.0[i] != o.0[i] {
                return self.0[i].cmp(&o.0[i]);
            }
        }
        Ordering::Equal
    }
}
