//! Arbitrary-precision decimal: sign, big coefficient, power-of-ten
//! exponent. Every rounding takes an explicit digit count; the 34-digit
//! IEEE 754-2008 decimal128 behaviour is applied by `num.rs`.

use super::bigint::Big;
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rnd {
    HalfEven,
    HalfUp,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseErr {
    Syntax,
    Range,
}

#[derive(Clone, Debug)]
pub struct Dec {
    pub neg: bool,
    pub coef: Big,
    pub exp: i32,
}

impl PartialEq for Dec {
    fn eq(&self, o: &Dec) -> bool {
        self.cmp_val(o) == Ordering::Equal
    }
}

impl Dec {
    pub fn zero() -> Dec {
        Dec {
            neg: false,
            coef: Big::zero(),
            exp: 0,
        }
    }
    pub fn one() -> Dec {
        Dec::from_parts(false, 1, 0)
    }
    pub fn from_parts(neg: bool, coef: u64, exp: i32) -> Dec {
        Dec {
            neg,
            coef: Big::from_u64(coef),
            exp,
        }
        .norm()
    }
    pub fn from_i64(n: i64) -> Dec {
        Dec::from_parts(n < 0, n.unsigned_abs(), 0)
    }
    pub fn pow10(e: i32) -> Dec {
        Dec::from_parts(false, 1, e)
    }

    fn norm(mut self) -> Dec {
        if self.coef.is_zero() {
            self.neg = false;
            self.exp = 0;
        }
        self
    }

    pub fn is_zero(&self) -> bool {
        self.coef.is_zero()
    }

    /// Number of decimal digits in the coefficient (zero counts as 1).
    pub fn nd(&self) -> usize {
        self.coef.to_dec_string().len()
    }

    /// Exponent of the leading digit (value = d.ddd × 10^adj). Zero gives a very low value.
    pub fn adj_exp(&self) -> i32 {
        if self.is_zero() {
            i32::MIN / 4
        } else {
            self.exp + self.nd() as i32 - 1
        }
    }

    pub fn neg(&self) -> Dec {
        Dec {
            neg: !self.neg,
            coef: self.coef.clone(),
            exp: self.exp,
        }
        .norm()
    }
    pub fn abs(&self) -> Dec {
        Dec {
            neg: false,
            coef: self.coef.clone(),
            exp: self.exp,
        }
    }

    pub fn round(&self, prec: usize, rnd: Rnd) -> Dec {
        let nd = self.nd();
        if self.is_zero() || nd <= prec {
            return self.clone();
        }
        let k = nd - prec;
        let p = Big::pow10(k);
        let (mut q, r) = self.coef.divrem_big(&p);
        let half = Big::pow10(k - 1).mul_small(5);
        let up = match r.cmp(&half) {
            Ordering::Greater => true,
            Ordering::Equal => rnd == Rnd::HalfUp || q.is_odd(),
            Ordering::Less => false,
        };
        if up {
            q = q.add_small(1);
        }
        let mut d = Dec {
            neg: self.neg,
            coef: q,
            exp: self.exp + k as i32,
        };
        if d.nd() > prec {
            d.coef = d.coef.divrem_small(10).0;
            d.exp += 1;
        }
        d.norm()
    }

    /// Round so that no digit remains below 10^-n (half up, for display).
    pub fn round_decimals_half_up(&self, n: i32) -> Dec {
        if self.is_zero() || self.exp >= -n {
            return self.clone();
        }
        let k = (-n - self.exp) as usize;
        let p = Big::pow10(k);
        let (mut q, r) = self.coef.divrem_big(&p);
        if r >= Big::pow10(k - 1).mul_small(5) {
            q = q.add_small(1);
        }
        Dec {
            neg: self.neg,
            coef: q,
            exp: -n,
        }
        .norm()
    }

    pub fn add_exact(&self, o: &Dec) -> Dec {
        if self.is_zero() {
            return o.clone();
        }
        if o.is_zero() {
            return self.clone();
        }
        let e = self.exp.min(o.exp);
        let a = self.coef.mul_pow10((self.exp - e) as usize);
        let b = o.coef.mul_pow10((o.exp - e) as usize);
        if self.neg == o.neg {
            return Dec {
                neg: self.neg,
                coef: a.add_big(&b),
                exp: e,
            };
        }
        match a.cmp(&b) {
            Ordering::Equal => Dec::zero(),
            Ordering::Greater => Dec {
                neg: self.neg,
                coef: a.sub_big(&b),
                exp: e,
            },
            Ordering::Less => Dec {
                neg: o.neg,
                coef: b.sub_big(&a),
                exp: e,
            },
        }
    }

    pub fn add(&self, o: &Dec, prec: usize) -> Dec {
        if self.is_zero() {
            return o.round(prec, Rnd::HalfEven);
        }
        if o.is_zero() {
            return self.round(prec, Rnd::HalfEven);
        }
        let (big, small) = if self.adj_exp() >= o.adj_exp() {
            (self, o)
        } else {
            (o, self)
        };
        let gap = big.adj_exp() - small.adj_exp();
        let tiny;
        let s2 = if gap > prec as i32 + 5 {
            // Far below the rounding position: only its sign and non-zeroness matter.
            tiny = Dec {
                neg: small.neg,
                coef: Big::from_u64(1),
                exp: big.adj_exp() - prec as i32 - 4,
            };
            &tiny
        } else {
            small
        };
        big.add_exact(s2).round(prec, Rnd::HalfEven)
    }

    pub fn sub(&self, o: &Dec, prec: usize) -> Dec {
        self.add(&o.neg(), prec)
    }

    pub fn mul(&self, o: &Dec, prec: usize) -> Dec {
        Dec {
            neg: self.neg != o.neg,
            coef: self.coef.mul_big(&o.coef),
            exp: self.exp + o.exp,
        }
        .norm()
        .round(prec, Rnd::HalfEven)
    }

    /// None when dividing by zero.
    pub fn div(&self, o: &Dec, prec: usize) -> Option<Dec> {
        if o.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Dec::zero());
        }
        let na = self.nd() as i64;
        let nb = o.nd() as i64;
        let shift = (prec as i64 + 3 + nb - na).max(0) as usize;
        let num = self.coef.mul_pow10(shift);
        let (mut q, r) = num.divrem_big(&o.coef);
        let mut exp = self.exp - o.exp - shift as i32;
        if !r.is_zero() {
            q = q.mul_small(10).add_small(1);
            exp -= 1;
        }
        Some(
            Dec {
                neg: self.neg != o.neg,
                coef: q,
                exp,
            }
            .round(prec, Rnd::HalfEven),
        )
    }

    /// Correctly rounded square root; None for negatives.
    pub fn sqrt(&self, prec: usize) -> Option<Dec> {
        if self.is_zero() {
            return Some(Dec::zero());
        }
        if self.neg {
            return None;
        }
        let mut c = self.coef.clone();
        let mut e = self.exp;
        if e.rem_euclid(2) != 0 {
            c = c.mul_small(10);
            e -= 1;
        }
        let want = 2 * (prec + 3);
        let nd = c.to_dec_string().len();
        let s = if nd < want {
            (want - nd).div_ceil(2)
        } else {
            0
        };
        c = c.mul_pow10(2 * s);
        e -= 2 * s as i32;
        let mut r = c.isqrt();
        let exact = r.mul_big(&r) == c;
        let mut re = e / 2;
        if !exact {
            r = r.mul_small(10).add_small(1);
            re -= 1;
        }
        Some(
            Dec {
                neg: false,
                coef: r,
                exp: re,
            }
            .round(prec, Rnd::HalfEven),
        )
    }

    pub fn cmp_abs(&self, o: &Dec) -> Ordering {
        match (self.is_zero(), o.is_zero()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let (a, b) = (self.adj_exp(), o.adj_exp());
        if a != b {
            return a.cmp(&b);
        }
        let e = self.exp.min(o.exp);
        self.coef
            .mul_pow10((self.exp - e) as usize)
            .cmp(&o.coef.mul_pow10((o.exp - e) as usize))
    }

    pub fn cmp_val(&self, o: &Dec) -> Ordering {
        match (self.neg, o.neg) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.cmp_abs(o),
            (true, true) => o.cmp_abs(self),
        }
    }

    pub fn is_int(&self) -> bool {
        if self.is_zero() || self.exp >= 0 {
            return true;
        }
        let (_, r) = self.coef.divrem_big(&Big::pow10((-self.exp) as usize));
        r.is_zero()
    }

    /// (integer part toward zero, whether anything was dropped).
    fn split_int(&self) -> (Dec, bool) {
        if self.exp >= 0 || self.is_zero() {
            return (self.clone(), false);
        }
        let (q, r) = self.coef.divrem_big(&Big::pow10((-self.exp) as usize));
        (
            Dec {
                neg: self.neg,
                coef: q,
                exp: 0,
            }
            .norm(),
            !r.is_zero(),
        )
    }

    pub fn trunc(&self) -> Dec {
        self.split_int().0
    }

    pub fn floor(&self) -> Dec {
        let (t, dropped) = self.split_int();
        if self.neg && dropped {
            t.add_exact(&Dec::from_i64(-1))
        } else {
            t
        }
    }

    pub fn fract(&self) -> Dec {
        self.add_exact(&self.trunc().neg())
    }

    pub fn to_i64(&self) -> Option<i64> {
        let t = self.trunc();
        if t.exp < 0 || t.adj_exp() > 18 {
            return None;
        }
        let v = t.coef.mul_pow10(t.exp as usize).to_u64()?;
        let v = i64::try_from(v).ok()?;
        Some(if t.neg { -v } else { v })
    }

    pub fn parse(s: &str) -> Result<Dec, ParseErr> {
        let (neg, rest) = match s.as_bytes().first() {
            Some(b'-') => (true, &s[1..]),
            Some(b'+') => (false, &s[1..]),
            _ => (false, s),
        };
        let (mant, exp_s) = match rest.find(['e', 'E']) {
            Some(i) => (&rest[..i], Some(&rest[i + 1..])),
            None => (rest, None),
        };
        let (int_s, frac_s) = match mant.split_once('.') {
            Some((a, b)) => (a, b),
            None => (mant, ""),
        };
        if int_s.is_empty() && frac_s.is_empty() {
            return Err(ParseErr::Syntax);
        }
        let digits = format!("{int_s}{frac_s}");
        let coef = Big::from_dec_str(&digits).ok_or(ParseErr::Syntax)?;
        let mut exp10: i64 = 0;
        if let Some(es) = exp_s {
            let (eneg, ed) = match es.as_bytes().first() {
                Some(b'-') => (true, &es[1..]),
                Some(b'+') => (false, &es[1..]),
                _ => (false, es),
            };
            if ed.is_empty() || !ed.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseErr::Syntax);
            }
            if ed.len() > 6 {
                return Err(ParseErr::Range);
            }
            let v: i64 = ed.parse().map_err(|_| ParseErr::Syntax)?;
            exp10 = if eneg { -v } else { v };
        }
        let e = exp10 - frac_s.len() as i64;
        if e.abs() > 100_000 {
            return Err(ParseErr::Range);
        }
        Ok(Dec {
            neg,
            coef,
            exp: e as i32,
        }
        .norm())
    }

    /// Exact text form `[-]<coef>e<exp>` (used by the state file).
    pub fn to_sci_string(&self) -> String {
        format!(
            "{}{}e{}",
            if self.neg { "-" } else { "" },
            self.coef.to_dec_string(),
            self.exp
        )
    }

    pub fn to_f64(&self) -> f64 {
        self.to_sci_string().parse::<f64>().unwrap_or(f64::NAN)
    }

    pub fn from_f64(x: f64) -> Dec {
        if !x.is_finite() {
            return Dec::zero();
        }
        Dec::parse(&format!("{x:e}")).unwrap_or_else(|_| Dec::zero())
    }
}
