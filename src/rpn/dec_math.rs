//! Transcendental functions on `Dec`, computed with guard digits.
//! `p` is the working precision in digits; callers round the result to
//! their target (34) afterwards. Error is far below 1 ulp of 34 digits
//! for the documented argument ranges, but results are not formally
//! proven correctly rounded.

use super::bigint::Big;
use super::consts::{LN10_STR, PI_STR};
use super::dec::{Dec, Rnd};
use std::cmp::Ordering;

fn dv(a: &Dec, b: &Dec, p: usize) -> Dec {
    a.div(b, p).unwrap_or_else(Dec::zero)
}

fn small(n: i64) -> Dec {
    Dec::from_i64(n)
}

fn tiny(w: usize, d: &Dec) -> bool {
    d.is_zero() || d.adj_exp() < -(w as i32) - 2
}

pub fn pi(p: usize) -> Dec {
    Dec::parse(PI_STR)
        .unwrap_or_else(|_| Dec::zero())
        .round(p.min(250), Rnd::HalfEven)
}

pub fn ln10(p: usize) -> Dec {
    Dec::parse(LN10_STR)
        .unwrap_or_else(|_| Dec::zero())
        .round(p.min(250), Rnd::HalfEven)
}

/// e^x. Callers keep |x| below about 14500 (decimal128 range).
pub fn exp(x: &Dec, p: usize) -> Dec {
    if x.is_zero() {
        return Dec::one();
    }
    let w = p + 8;
    let two = small(2);
    let lim = Dec::from_parts(false, 1, -2);
    let mut y = x.clone();
    let mut m = 0u32;
    while y.cmp_abs(&lim) == Ordering::Greater && m < 64 {
        y = dv(&y, &two, w);
        m += 1;
    }
    let mut sum = Dec::one();
    let mut term = Dec::one();
    for n in 1..2000i64 {
        term = dv(&term.mul(&y, w), &small(n), w);
        sum = sum.add(&term, w);
        if tiny(w, &term) {
            break;
        }
    }
    for _ in 0..m {
        sum = sum.mul(&sum, w);
    }
    sum.round(p, Rnd::HalfEven)
}

/// e^x - 1, accurate for small x.
pub fn expm1(x: &Dec, p: usize) -> Dec {
    if x.is_zero() {
        return Dec::zero();
    }
    let w = p + 8;
    if x.cmp_abs(&Dec::from_parts(false, 5, -1)) == Ordering::Less {
        let mut sum = x.clone();
        let mut term = x.clone();
        for n in 2..400i64 {
            term = dv(&term.mul(x, w), &small(n), w);
            sum = sum.add(&term, w);
            if tiny(w, &term) {
                break;
            }
        }
        return sum.round(p, Rnd::HalfEven);
    }
    exp(x, w).sub(&Dec::one(), w).round(p, Rnd::HalfEven)
}

/// ln(1+z) core for |z| < 0.5 (exact z in, relative accuracy kept).
fn ln1p_core(z: &Dec, w: usize) -> Dec {
    let one = Dec::one();
    let two = small(2);
    let lim = Dec::from_parts(false, 1, -2);
    let mut z = z.clone();
    let mut k = 0u32;
    if z.cmp_abs(&lim) == Ordering::Greater {
        let mut m = one.add(&z, w);
        while m.sub(&one, w).cmp_abs(&lim) == Ordering::Greater && k < 64 {
            m = m.sqrt(w).unwrap_or_else(Dec::one);
            k += 1;
        }
        z = m.sub(&one, w);
    }
    let s = dv(&z, &two.add(&z, w), w);
    let s2 = s.mul(&s, w);
    let mut sum = s.clone();
    let mut term = s;
    let mut n = 3i64;
    loop {
        term = term.mul(&s2, w);
        let t = dv(&term, &small(n), w);
        sum = sum.add(&t, w);
        if tiny(w, &t) || n > 4000 {
            break;
        }
        n += 2;
    }
    let mut r = sum.mul(&two, w);
    for _ in 0..k {
        r = r.mul(&two, w);
    }
    r
}

/// Natural log, x > 0.
pub fn ln(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let z = x.add_exact(&small(-1));
    if z.cmp_abs(&Dec::from_parts(false, 5, -1)) == Ordering::Less {
        return ln1p_core(&z, w).round(p, Rnd::HalfEven);
    }
    let e10 = x.adj_exp();
    let m = Dec {
        neg: false,
        coef: x.coef.clone(),
        exp: -(x.nd() as i32 - 1),
    };
    let lm = ln1p_core(&m.add_exact(&small(-1)), w);
    lm.add(&small(e10 as i64).mul(&ln10(w), w), w)
        .round(p, Rnd::HalfEven)
}

/// ln(1+x), x > -1.
pub fn ln1p(x: &Dec, p: usize) -> Dec {
    if x.cmp_abs(&Dec::from_parts(false, 5, -1)) == Ordering::Less {
        return ln1p_core(x, p + 8).round(p, Rnd::HalfEven);
    }
    ln(&x.add_exact(&Dec::one()), p)
}

fn sin_cos_small(r: &Dec, w: usize) -> (Dec, Dec) {
    let r2 = r.mul(r, w);
    let mut s = r.clone();
    let mut t = r.clone();
    let mut k = 1i64;
    loop {
        t = dv(&t.mul(&r2, w).neg(), &small((2 * k) * (2 * k + 1)), w);
        s = s.add(&t, w);
        if tiny(w, &t) || k > 400 {
            break;
        }
        k += 1;
    }
    let mut c = Dec::one();
    let mut u = Dec::one();
    k = 1;
    loop {
        u = dv(&u.mul(&r2, w).neg(), &small((2 * k - 1) * (2 * k)), w);
        c = c.add(&u, w);
        if tiny(w, &u) || k > 400 {
            break;
        }
        k += 1;
    }
    (s, c)
}

/// (sin x, cos x) for x in radians; None when |x| is too large to reduce.
pub fn sin_cos(x: &Dec, p: usize) -> Option<(Dec, Dec)> {
    if x.is_zero() {
        return Some((Dec::zero(), Dec::one()));
    }
    let big = x.adj_exp().max(0);
    if big > 100 {
        return None;
    }
    // Extra guard digits: inputs near a multiple of pi/2 cancel heavily.
    let w = p + 50 + big as usize;
    let half_pi = dv(&pi(w), &small(2), w);
    let q = dv(x, &half_pi, w)
        .add(&Dec::from_parts(false, 5, -1), w)
        .floor();
    let k4 = {
        let ip = q.trunc();
        let ip_abs_mod = if ip.exp >= 2 || ip.is_zero() {
            0
        } else {
            let m = ip.coef.divrem_small(4).1 as u64 * 10u64.pow(ip.exp.max(0) as u32);
            (m % 4) as usize
        };
        if ip.neg {
            (4 - ip_abs_mod) % 4
        } else {
            ip_abs_mod
        }
    };
    let r = x.sub(&q.mul(&half_pi, w), w);
    let (s, c) = sin_cos_small(&r, w);
    let (s, c) = match k4 {
        0 => (s, c),
        1 => (c, s.neg()),
        2 => (s.neg(), c.neg()),
        _ => (c.neg(), s),
    };
    Some((s.round(p, Rnd::HalfEven), c.round(p, Rnd::HalfEven)))
}

pub fn atan(x: &Dec, p: usize) -> Dec {
    if x.is_zero() {
        return Dec::zero();
    }
    let w = p + 8;
    let one = Dec::one();
    let neg = x.neg;
    let a = x.abs();
    let (mut t, inv) = if a.cmp_val(&one) == Ordering::Greater {
        (dv(&one, &a, w), true)
    } else {
        (a, false)
    };
    for _ in 0..3 {
        let s = t.mul(&t, w).add(&one, w).sqrt(w).unwrap_or_else(Dec::one);
        t = dv(&t, &s.add(&one, w), w);
    }
    let t2 = t.mul(&t, w);
    let mut sum = t.clone();
    let mut term = t;
    let mut n = 3i64;
    loop {
        term = term.mul(&t2, w).neg();
        let u = dv(&term, &small(n), w);
        sum = sum.add(&u, w);
        if tiny(w, &u) || n > 2000 {
            break;
        }
        n += 2;
    }
    let mut r = sum.mul(&small(8), w);
    if inv {
        r = dv(&pi(w), &small(2), w).sub(&r, w);
    }
    if neg {
        r = r.neg();
    }
    r.round(p, Rnd::HalfEven)
}

/// asin for |x| <= 1.
pub fn asin(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let one = Dec::one();
    if x.cmp_abs(&one) == Ordering::Equal {
        let h = dv(&pi(w), &small(2), w);
        return if x.neg { h.neg() } else { h }.round(p, Rnd::HalfEven);
    }
    let prod = one.add_exact(&x.neg()).mul(&one.add_exact(x), w + 10);
    let d = prod.sqrt(w).unwrap_or_else(Dec::one);
    atan(&dv(x, &d, w), p)
}

/// acos for |x| <= 1: 2*atan(sqrt((1-x)/(1+x))), exact at x = 1.
pub fn acos(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let one = Dec::one();
    let a = one.add_exact(&x.neg());
    let b = one.add_exact(x);
    if a.is_zero() {
        return Dec::zero();
    }
    if b.is_zero() {
        return pi(p);
    }
    let t = dv(&a, &b, w + 10).sqrt(w).unwrap_or_else(Dec::zero);
    atan(&t, w).mul(&small(2), w).round(p, Rnd::HalfEven)
}

pub fn sinh(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    if x.cmp_abs(&Dec::from_parts(false, 1, -2)) == Ordering::Less {
        let x2 = x.mul(x, w);
        let mut sum = x.clone();
        let mut t = x.clone();
        let mut k = 1i64;
        loop {
            t = dv(&t.mul(&x2, w), &small((2 * k) * (2 * k + 1)), w);
            sum = sum.add(&t, w);
            if tiny(w, &t) || k > 200 {
                break;
            }
            k += 1;
        }
        return sum.round(p, Rnd::HalfEven);
    }
    let a = exp(x, w);
    let b = dv(&Dec::one(), &a, w);
    dv(&a.sub(&b, w), &small(2), w).round(p, Rnd::HalfEven)
}

pub fn cosh(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let a = exp(x, w);
    let b = dv(&Dec::one(), &a, w);
    dv(&a.add(&b, w), &small(2), w).round(p, Rnd::HalfEven)
}

pub fn tanh(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let one = Dec::one();
    if x.cmp_abs(&small(40)) == Ordering::Greater {
        return if x.neg { one.neg() } else { one };
    }
    if x.cmp_abs(&Dec::from_parts(false, 1, -2)) == Ordering::Less {
        return dv(&sinh(x, w), &cosh(x, w), w).round(p, Rnd::HalfEven);
    }
    let e = exp(&x.mul(&small(2), w), w);
    dv(&e.sub(&one, w), &e.add(&one, w), w).round(p, Rnd::HalfEven)
}

pub fn asinh(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let one = Dec::one();
    let a = x.abs();
    let s = a.mul(&a, w).add(&one, w).sqrt(w).unwrap_or_else(Dec::one);
    let t = a.add(&dv(&a.mul(&a, w), &s.add(&one, w), w), w);
    let r = ln1p(&t, w);
    if x.neg {
        r.neg().round(p, Rnd::HalfEven)
    } else {
        r.round(p, Rnd::HalfEven)
    }
}

/// acosh for x >= 1.
pub fn acosh(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let xm1 = x.add_exact(&small(-1));
    let xp1 = x.add_exact(&small(1));
    let s = xm1.mul(&xp1, w + 10).sqrt(w).unwrap_or_else(Dec::zero);
    ln1p(&xm1.add(&s, w), p)
}

/// atanh for |x| < 1.
pub fn atanh(x: &Dec, p: usize) -> Dec {
    let w = p + 8;
    let a = x.abs();
    let one = Dec::one();
    let t = dv(&a.mul(&small(2), w), &one.sub(&a, w + 10), w);
    let r = dv(&ln1p(&t, w), &small(2), w);
    if x.neg {
        r.neg().round(p, Rnd::HalfEven)
    } else {
        r.round(p, Rnd::HalfEven)
    }
}

/// y^n for integer n. None on overflow; underflow gives zero.
pub fn powi(y: &Dec, n: i64, p: usize) -> Option<Dec> {
    let w = p + 10;
    let mut base = y.clone();
    let mut e = n.unsigned_abs();
    let mut r = Dec::one();
    while e > 0 {
        if e & 1 == 1 {
            r = r.mul(&base, w);
        }
        e >>= 1;
        if e > 0 {
            base = base.mul(&base, w);
        }
        if r.adj_exp() > 10_000 || (e > 0 && base.adj_exp() > 10_000) {
            return None;
        }
        if r.adj_exp() < -10_000 && !r.is_zero() {
            return if n < 0 { None } else { Some(Dec::zero()) };
        }
    }
    if n < 0 {
        r = Dec::one().div(&r, w)?;
    }
    Some(r.round(p, Rnd::HalfEven))
}

/// n! as a decimal (None if it would not fit).
pub fn fact(n: u64) -> Option<Dec> {
    let mut r = Big::from_u64(1);
    for i in 2..=n {
        r = r.mul_small(u32::try_from(i).ok()?);
        if r.bit_len() > 21_000 {
            return None;
        }
    }
    Some(
        Dec {
            neg: false,
            coef: r,
            exp: 0,
        }
        .round(34, Rnd::HalfEven),
    )
}

/// C(n,k) or P(n,k) exactly, then rounded. None if too large.
pub fn comb_perm(n: u64, k: u64, perm: bool) -> Option<Dec> {
    let steps = if perm { k } else { k.min(n - k) };
    if steps > 6000 {
        return None;
    }
    let mut num = Big::from_u64(1);
    for i in 0..steps {
        num = num.mul_big(&Big::from_u64(n - i));
        if num.bit_len() > 42_000 {
            return None;
        }
    }
    if !perm {
        let mut den = Big::from_u64(1);
        for i in 2..=steps {
            den = den.mul_big(&Big::from_u64(i));
        }
        num = num.divrem_big(&den).0;
    }
    Some(
        Dec {
            neg: false,
            coef: num,
            exp: 0,
        }
        .round(34, Rnd::HalfEven),
    )
}

/// y - x*floor(y/x), exact. None for x = 0 or absurd exponent gaps.
pub fn modulo(y: &Dec, x: &Dec) -> Option<Dec> {
    if x.is_zero() {
        return None;
    }
    if y.is_zero() {
        return Some(Dec::zero());
    }
    let e = y.exp.min(x.exp);
    if y.exp.max(x.exp) - e > 12_000 {
        return None;
    }
    let yc = y.coef.mul_pow10((y.exp - e) as usize);
    let xc = x.coef.mul_pow10((x.exp - e) as usize);
    let (_, mut r) = yc.divrem_big(&xc);
    if !r.is_zero() && y.neg != x.neg {
        r = xc.sub_big(&r);
    }
    Some(
        Dec {
            neg: x.neg,
            coef: r,
            exp: e,
        }
        .round(34, Rnd::HalfEven),
    )
}
