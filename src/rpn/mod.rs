//! RPN / programmable mode — P1: the HP-42S-style stack machine.
//!
//! Clean-room (see docs/RPN_PLAN.md): behaviour comes from the
//! published HP-42S manual and public Free42 docs, not upstream
//! source. Numbers are f64 for now (Free42 "Binary" equivalent);
//! P2 swaps in 34-digit decimal.
//!
//! Errors never panic: every failure is an `RpnError` with a stable
//! code and plain-words message, and a failing operation leaves the
//! stack untouched.

pub mod display;
pub mod state;

use display::{format_num, Disp};
use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};
use std::fmt;

/// Upper bounds (memory / runtime caps, AGENTS.md).
pub const MAX_VARS: usize = 1000;
pub const MAX_TOKENS: usize = 100_000;
pub const MAX_NAME_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Angle {
    Deg,
    Rad,
    Grad,
}

impl Angle {
    pub fn label(self) -> &'static str {
        match self {
            Angle::Deg => "DEG",
            Angle::Rad => "RAD",
            Angle::Grad => "GRAD",
        }
    }
    pub fn from_label(s: &str) -> Option<Angle> {
        match s {
            "DEG" => Some(Angle::Deg),
            "RAD" => Some(Angle::Rad),
            "GRAD" => Some(Angle::Grad),
            _ => None,
        }
    }
    /// Units in a full circle.
    fn full(self) -> f64 {
        match self {
            Angle::Deg => 360.0,
            Angle::Rad => TAU,
            Angle::Grad => 400.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpnError {
    pub code: &'static str,
    pub msg: String,
}

impl RpnError {
    pub fn new(code: &'static str, msg: impl Into<String>) -> Self {
        RpnError {
            code,
            msg: msg.into(),
        }
    }
}

impl fmt::Display for RpnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.msg, self.code)
    }
}

fn invalid() -> RpnError {
    RpnError::new(
        "RPN-102",
        "Invalid Data: the function is not defined for this value.",
    )
}
fn div0() -> RpnError {
    RpnError::new("RPN-103", "Divide by 0.")
}
fn range() -> RpnError {
    RpnError::new(
        "RPN-104",
        "Out of Range: the result is too large to represent.",
    )
}

pub fn valid_var_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_LEN
        && !name.chars().any(|c| c == '|' || c.is_control())
}

fn finite(x: f64) -> Result<f64, RpnError> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(range())
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    /// stack[0] = X, 1 = Y, 2 = Z, 3 = T.
    pub stack: [f64; 4],
    pub lastx: f64,
    /// Stack lift enabled (a number entry lifts the stack).
    pub lift: bool,
    pub angle: Angle,
    pub disp: Disp,
    pub vars: BTreeMap<String, f64>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

struct Tok {
    text: String,
    quoted: bool,
}

fn tokenize(line: &str) -> Result<Vec<Tok>, RpnError> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut s = String::new();
            let mut closed = false;
            for ch in chars.by_ref() {
                if ch == '"' {
                    closed = true;
                    break;
                }
                s.push(ch);
            }
            if !closed {
                return Err(RpnError::new(
                    "RPN-108",
                    "A quoted name is missing its closing quote.",
                ));
            }
            out.push(Tok {
                text: s,
                quoted: true,
            });
        } else {
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                s.push(ch);
                chars.next();
            }
            out.push(Tok {
                text: s,
                quoted: false,
            });
        }
        if out.len() > MAX_TOKENS {
            return Err(RpnError::new("RPN-109", "Too many tokens in one line."));
        }
    }
    Ok(out)
}

/// A number token: digits with optional sign, point and exponent.
/// Rejects words like "inf" / "nan" that Rust's parser would accept.
fn parse_number(t: &str) -> Option<f64> {
    let b = t.as_bytes();
    let first = *b.first()?;
    let starts_ok = first.is_ascii_digit()
        || first == b'.'
        || ((first == b'-' || first == b'+')
            && b.get(1).is_some_and(|c| c.is_ascii_digit() || *c == b'.'));
    if !starts_ok
        || !t
            .bytes()
            .all(|c| c.is_ascii_digit() || b".+-eE".contains(&c))
    {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn to_angle_units(rad: f64, a: Angle) -> f64 {
    rad * a.full() / TAU
}

/// Exact results at quarter-turn multiples (sin 180° = 0, not 1.2e-16).
fn quarter(x: f64, a: Angle) -> Option<i64> {
    let q = x / a.full() * 4.0;
    if q.fract() == 0.0 && q.abs() < 1e15 {
        Some((q as i64).rem_euclid(4))
    } else {
        None
    }
}

fn sin_a(x: f64, a: Angle) -> f64 {
    match quarter(x, a) {
        Some(k) => [0.0, 1.0, 0.0, -1.0][k as usize],
        None => (x * TAU / a.full()).sin(),
    }
}
fn cos_a(x: f64, a: Angle) -> f64 {
    match quarter(x, a) {
        Some(k) => [1.0, 0.0, -1.0, 0.0][k as usize],
        None => (x * TAU / a.full()).cos(),
    }
}
fn tan_a(x: f64, a: Angle) -> Result<f64, RpnError> {
    match quarter(x, a) {
        Some(k) if k % 2 == 0 => Ok(0.0),
        Some(_) => Err(invalid()),
        None => Ok((x * TAU / a.full()).tan()),
    }
}

fn is_int(x: f64) -> bool {
    x.fract() == 0.0
}

fn fact(x: f64) -> Result<f64, RpnError> {
    if !is_int(x) || x < 0.0 {
        return Err(invalid());
    }
    if x > 170.0 {
        return Err(range());
    }
    let mut r = 1.0f64;
    let mut i = 2.0;
    while i <= x {
        r *= i;
        i += 1.0;
    }
    Ok(r)
}

fn comb_perm(n: f64, k: f64, perm: bool) -> Result<f64, RpnError> {
    if !is_int(n) || !is_int(k) || n < 0.0 || k < 0.0 || k > n {
        return Err(invalid());
    }
    let steps = if perm { k } else { k.min(n - k) };
    if steps > 1.0e6 {
        return Err(range());
    }
    let mut r = 1.0f64;
    let mut i = 0.0;
    while i < steps {
        r = if perm {
            r * (n - i)
        } else {
            r * (n - i) / (i + 1.0)
        };
        i += 1.0;
    }
    finite(if perm { r } else { r.round() })
}

impl Session {
    pub fn new() -> Self {
        Session {
            stack: [0.0; 4],
            lastx: 0.0,
            lift: true,
            angle: Angle::Deg,
            disp: Disp::Fix(4),
            vars: BTreeMap::new(),
        }
    }

    pub fn x(&self) -> f64 {
        self.stack[0]
    }

    pub fn display_x(&self) -> String {
        format_num(self.stack[0], self.disp)
    }

    /// Four lines, T first, X last.
    pub fn stack_text(&self) -> String {
        ["T", "Z", "Y", "X"]
            .iter()
            .zip([3usize, 2, 1, 0])
            .map(|(n, i)| format!("{n}: {}", format_num(self.stack[i], self.disp)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn to_json(&self) -> String {
        let f = |i: usize| format!("\"{}\"", format_num(self.stack[i], self.disp));
        format!(
            "{{\"ok\":true,\"x\":{},\"y\":{},\"z\":{},\"t\":{},\"lastx\":\"{}\",\"angle\":\"{}\",\"disp\":\"{}\"}}",
            f(0),
            f(1),
            f(2),
            f(3),
            format_num(self.lastx, self.disp),
            self.angle.label(),
            self.disp.label()
        )
    }

    fn push_value(&mut self, x: f64) {
        if self.lift {
            self.stack[3] = self.stack[2];
            self.stack[2] = self.stack[1];
            self.stack[1] = self.stack[0];
        }
        self.stack[0] = x;
        self.lift = true;
    }

    fn unary(
        &mut self,
        f: impl FnOnce(f64, Angle) -> Result<f64, RpnError>,
    ) -> Result<(), RpnError> {
        let x = self.stack[0];
        let r = finite(f(x, self.angle)?)?;
        self.lastx = x;
        self.stack[0] = r;
        self.lift = true;
        Ok(())
    }

    fn binary(
        &mut self,
        f: impl FnOnce(f64, f64) -> Result<f64, RpnError>,
    ) -> Result<(), RpnError> {
        let (y, x) = (self.stack[1], self.stack[0]);
        let r = finite(f(y, x)?)?;
        self.lastx = x;
        self.stack[0] = r;
        self.stack[1] = self.stack[2];
        self.stack[2] = self.stack[3];
        self.lift = true;
        Ok(())
    }

    fn name_arg(toks: &[Tok], i: &mut usize, cmd: &str) -> Result<String, RpnError> {
        let t = toks.get(*i).ok_or_else(|| {
            RpnError::new(
                "RPN-101",
                format!("{cmd} needs a name or register number after it."),
            )
        })?;
        *i += 1;
        if !valid_var_name(&t.text) {
            return Err(RpnError::new(
                "RPN-101",
                format!(
                    "'{}' is not a valid name (1-{MAX_NAME_LEN} characters, no '|').",
                    t.text
                ),
            ));
        }
        Ok(t.text.clone())
    }

    fn digits_arg(toks: &[Tok], i: &mut usize, cmd: &str) -> Result<u8, RpnError> {
        let t = toks.get(*i).ok_or_else(|| {
            RpnError::new(
                "RPN-101",
                format!("{cmd} needs a digit count 0-11 after it."),
            )
        })?;
        *i += 1;
        match t.text.parse::<u8>() {
            Ok(n) if n <= 11 => Ok(n),
            _ => Err(RpnError::new(
                "RPN-102",
                format!("{cmd} takes a whole number from 0 to 11, not '{}'.", t.text),
            )),
        }
    }

    fn store(&mut self, name: String, v: f64) -> Result<(), RpnError> {
        if !self.vars.contains_key(&name) && self.vars.len() >= MAX_VARS {
            return Err(RpnError::new("RPN-110", "Too many variables."));
        }
        self.vars.insert(name, finite(v)?);
        Ok(())
    }

    fn var(&self, name: &str) -> Result<f64, RpnError> {
        self.vars.get(name).copied().ok_or_else(|| {
            RpnError::new(
                "RPN-105",
                format!("Nonexistent: no variable named '{name}'."),
            )
        })
    }

    fn arith(op: char, a: f64, b: f64) -> Result<f64, RpnError> {
        match op {
            '+' => Ok(a + b),
            '-' => Ok(a - b),
            '*' => Ok(a * b),
            _ => {
                if b == 0.0 {
                    Err(div0())
                } else {
                    Ok(a / b)
                }
            }
        }
    }

    /// Run one line of tokens. On error the failing operation has
    /// changed nothing; earlier tokens in the line stay applied.
    pub fn exec_line(&mut self, line: &str) -> Result<(), RpnError> {
        let toks = tokenize(line)?;
        let mut i = 0;
        while i < toks.len() {
            let t = &toks[i];
            i += 1;
            if !t.quoted {
                if let Some(n) = parse_number(&t.text) {
                    self.push_value(n);
                    continue;
                }
            } else {
                return Err(RpnError::new(
                    "RPN-100",
                    format!(
                        "Unexpected name \"{}\": names follow STO, RCL, CLV and friends.",
                        t.text
                    ),
                ));
            }
            let w = t.text.to_uppercase();
            match w.as_str() {
                "+" => self.binary(|y, x| finite(y + x))?,
                "-" => self.binary(|y, x| Ok(y - x))?,
                "*" | "×" => self.binary(|y, x| Ok(y * x))?,
                "/" | "÷" => self.binary(|y, x| if x == 0.0 { Err(div0()) } else { Ok(y / x) })?,
                "Y^X" => self.binary(|y, x| {
                    if y == 0.0 && x < 0.0 {
                        Err(div0())
                    } else if y < 0.0 && !is_int(x) {
                        Err(invalid())
                    } else {
                        Ok(y.powf(x))
                    }
                })?,
                "MOD" => self.binary(|y, x| if x == 0.0 { Err(div0()) } else { Ok(y - x * (y / x).floor()) })?,
                "COMB" => self.binary(|y, x| comb_perm(y, x, false))?,
                "PERM" => self.binary(|y, x| comb_perm(y, x, true))?,
                "%" => {
                    let (y, x) = (self.stack[1], self.stack[0]);
                    let r = finite(y * x / 100.0)?;
                    self.lastx = x;
                    self.stack[0] = r;
                    self.lift = true;
                }
                "%CH" => {
                    let (y, x) = (self.stack[1], self.stack[0]);
                    if y == 0.0 {
                        return Err(div0());
                    }
                    let r = finite((x - y) / y * 100.0)?;
                    self.lastx = x;
                    self.stack[0] = r;
                    self.lift = true;
                }
                "1/X" => self.unary(|x, _| if x == 0.0 { Err(div0()) } else { Ok(1.0 / x) })?,
                "X^2" => self.unary(|x, _| Ok(x * x))?,
                "SQRT" | "√X" => self.unary(|x, _| if x < 0.0 { Err(invalid()) } else { Ok(x.sqrt()) })?,
                "E^X" => self.unary(|x, _| Ok(x.exp()))?,
                "10^X" => self.unary(|x, _| Ok(10f64.powf(x)))?,
                "E^X-1" => self.unary(|x, _| Ok(x.exp_m1()))?,
                "LN" => self.unary(|x, _| if x <= 0.0 { Err(invalid()) } else { Ok(x.ln()) })?,
                "LOG" => self.unary(|x, _| if x <= 0.0 { Err(invalid()) } else { Ok(x.log10()) })?,
                "LN1+X" => self.unary(|x, _| if x <= -1.0 { Err(invalid()) } else { Ok(x.ln_1p()) })?,
                "ABS" => self.unary(|x, _| Ok(x.abs()))?,
                "CHS" | "+/-" => self.unary(|x, _| Ok(-x))?,
                "IP" => self.unary(|x, _| Ok(x.trunc()))?,
                "FP" => self.unary(|x, _| Ok(x.fract()))?,
                "FACT" => self.unary(|x, _| fact(x))?,
                "SIN" => self.unary(|x, a| Ok(sin_a(x, a)))?,
                "COS" => self.unary(|x, a| Ok(cos_a(x, a)))?,
                "TAN" => self.unary(tan_a)?,
                "ASIN" => self.unary(|x, a| {
                    if x.abs() > 1.0 {
                        Err(invalid())
                    } else {
                        Ok(to_angle_units(x.asin(), a))
                    }
                })?,
                "ACOS" => self.unary(|x, a| {
                    if x.abs() > 1.0 {
                        Err(invalid())
                    } else {
                        Ok(to_angle_units(x.acos(), a))
                    }
                })?,
                "ATAN" => self.unary(|x, a| Ok(to_angle_units(x.atan(), a)))?,
                "SINH" => self.unary(|x, _| Ok(x.sinh()))?,
                "COSH" => self.unary(|x, _| Ok(x.cosh()))?,
                "TANH" => self.unary(|x, _| Ok(x.tanh()))?,
                "ASINH" => self.unary(|x, _| Ok(x.asinh()))?,
                "ACOSH" => self.unary(|x, _| if x < 1.0 { Err(invalid()) } else { Ok(x.acosh()) })?,
                "ATANH" => self.unary(|x, _| if x.abs() >= 1.0 { Err(invalid()) } else { Ok(x.atanh()) })?,
                "D-R" | "→RAD" => self.unary(|x, _| Ok(x.to_radians()))?,
                "R-D" | "→DEG" => self.unary(|x, _| Ok(x.to_degrees()))?,
                "PI" | "π" => self.push_value(PI),
                "LASTX" => {
                    let l = self.lastx;
                    self.push_value(l);
                }
                "ENTER" | "ENTER^" | "↑" => {
                    self.stack[3] = self.stack[2];
                    self.stack[2] = self.stack[1];
                    self.stack[1] = self.stack[0];
                    self.lift = false;
                }
                "CLX" => {
                    self.stack[0] = 0.0;
                    self.lift = false;
                }
                "CLST" => {
                    self.stack = [0.0; 4];
                    self.lift = false;
                }
                "X<>Y" | "SWAP" => {
                    self.stack.swap(0, 1);
                    self.lift = true;
                }
                "RDN" | "R↓" => {
                    self.stack.rotate_left(1);
                    self.lift = true;
                }
                "RUP" | "R↑" => {
                    self.stack.rotate_right(1);
                    self.lift = true;
                }
                "DEG" => self.angle = Angle::Deg,
                "RAD" => self.angle = Angle::Rad,
                "GRAD" => self.angle = Angle::Grad,
                "ALL" => self.disp = Disp::All,
                "FIX" => self.disp = Disp::Fix(Self::digits_arg(&toks, &mut i, "FIX")?),
                "SCI" => self.disp = Disp::Sci(Self::digits_arg(&toks, &mut i, "SCI")?),
                "ENG" => self.disp = Disp::Eng(Self::digits_arg(&toks, &mut i, "ENG")?),
                "STO" => {
                    let n = Self::name_arg(&toks, &mut i, "STO")?;
                    self.store(n, self.stack[0])?;
                }
                "RCL" => {
                    let n = Self::name_arg(&toks, &mut i, "RCL")?;
                    let v = self.var(&n)?;
                    self.push_value(v);
                }
                "CLV" => {
                    let n = Self::name_arg(&toks, &mut i, "CLV")?;
                    self.var(&n)?;
                    self.vars.remove(&n);
                }
                "STO+" | "STO-" | "STO*" | "STO/" => {
                    let op = w.chars().nth(3).unwrap_or('+');
                    let n = Self::name_arg(&toks, &mut i, &w)?;
                    let v = Self::arith(op, self.var(&n)?, self.stack[0])?;
                    self.store(n, v)?;
                }
                "RCL+" | "RCL-" | "RCL*" | "RCL/" => {
                    let op = w.chars().nth(3).unwrap_or('+');
                    let n = Self::name_arg(&toks, &mut i, &w)?;
                    let v = self.var(&n)?;
                    let x = self.stack[0];
                    let r = finite(Self::arith(op, x, v)?)?;
                    self.lastx = x;
                    self.stack[0] = r;
                    self.lift = true;
                }
                _ => {
                    return Err(RpnError::new(
                        "RPN-100",
                        format!("Unknown token '{}'. Numbers, operators and HP-42S names such as ENTER, SIN, STO are accepted.", t.text),
                    ))
                }
            }
        }
        Ok(())
    }
}
