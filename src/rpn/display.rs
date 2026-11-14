//! Display formats: FIX / SCI / ENG / ALL (HP-42S style).
//! Pure functions; never panic on any f64.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disp {
    Fix(u8),
    Sci(u8),
    Eng(u8),
    All,
}

impl Disp {
    pub fn label(self) -> String {
        match self {
            Disp::Fix(n) => format!("FIX {n}"),
            Disp::Sci(n) => format!("SCI {n}"),
            Disp::Eng(n) => format!("ENG {n}"),
            Disp::All => "ALL".to_string(),
        }
    }
    pub fn to_file(self) -> String {
        match self {
            Disp::Fix(n) => format!("FIX:{n}"),
            Disp::Sci(n) => format!("SCI:{n}"),
            Disp::Eng(n) => format!("ENG:{n}"),
            Disp::All => "ALL".to_string(),
        }
    }
    pub fn from_file(s: &str) -> Option<Disp> {
        if s == "ALL" {
            return Some(Disp::All);
        }
        let (k, n) = s.split_once(':')?;
        let n: u8 = n.parse().ok().filter(|n| *n <= 11)?;
        match k {
            "FIX" => Some(Disp::Fix(n)),
            "SCI" => Some(Disp::Sci(n)),
            "ENG" => Some(Disp::Eng(n)),
            _ => None,
        }
    }
}

/// Split `{:.*e}` output into (negative, digit string, exponent).
fn sci_parts(x: f64, decimals: usize) -> (bool, String, i32) {
    let s = format!("{:.*e}", decimals, x.abs());
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let digits: String = m.chars().filter(|c| c.is_ascii_digit()).collect();
    (
        x.is_sign_negative() && x != 0.0,
        digits,
        e.parse().unwrap_or(0),
    )
}

/// Place the decimal point: digits d0 d1 ... with value d0.d1... * 10^e.
/// `int_digits` is how many digits sit left of the point.
fn place_point(digits: &str, int_digits: i32) -> String {
    if int_digits <= 0 {
        let zeros = "0".repeat((-int_digits) as usize);
        return format!("0.{zeros}{digits}");
    }
    let k = int_digits as usize;
    if digits.len() <= k {
        let pad = "0".repeat(k - digits.len());
        format!("{digits}{pad}")
    } else {
        format!("{}.{}", &digits[..k], &digits[k..])
    }
}

fn fix(x: f64, n: u8) -> String {
    if x != 0.0 && x.abs() >= 1e12 {
        return sci(x, n.min(11));
    }
    let s = format!("{:.*}", n as usize, x);
    if s.starts_with('-') && s.chars().all(|c| !c.is_ascii_digit() || c == '0') {
        return s[1..].to_string();
    }
    s
}

fn sci(x: f64, n: u8) -> String {
    let (neg, digits, e) = sci_parts(x, n as usize);
    let m = place_point(&digits, 1);
    format!("{}{}E{}", if neg { "-" } else { "" }, m, e)
}

fn eng(x: f64, n: u8) -> String {
    let (neg, digits, e) = sci_parts(x, n as usize);
    let e3 = e - e.rem_euclid(3);
    let m = place_point(&digits, 1 + e - e3);
    format!("{}{}E{}", if neg { "-" } else { "" }, m, e3)
}

fn all(x: f64) -> String {
    let (neg, digits, e) = sci_parts(x, 11);
    let trimmed = digits.trim_end_matches('0');
    let digits = if trimmed.is_empty() { "0" } else { trimmed };
    let sign = if neg { "-" } else { "" };
    if x == 0.0 {
        return "0".to_string();
    }
    if (-5..=11).contains(&e) {
        format!("{sign}{}", place_point(digits, e + 1))
    } else {
        format!("{sign}{}E{}", place_point(digits, 1), e)
    }
}

pub fn format_num(x: f64, d: Disp) -> String {
    if !x.is_finite() {
        return "Out of Range".to_string();
    }
    match d {
        Disp::Fix(n) => fix(x, n),
        Disp::Sci(n) => sci(x, n),
        Disp::Eng(n) => eng(x, n),
        Disp::All => all(x),
    }
}
