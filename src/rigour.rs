//! Rigour-batch engines (v0.4.0, 50-enhancement program items
//! #2, #3, #4, #5, #6, #7, #9, #10, #37) plus the standing-rule
//! features (built-in help, offer-only update check). Everything
//! here is a pure library operation unless noted: the GUI and the
//! CLI both call these functions, so no feature is window-only.

use crate::features::csv_row;
use crate::Calc;

// ---------- #9 rounding modes + #1/#2/#3/#37 display formats ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RoundingMode {
    /// School rounding: halves go away from zero (2.5 -> 3, -2.5 -> -3).
    #[default]
    HalfUp,
    /// Banker's rounding: halves go to the even neighbour (2.5 -> 2, 3.5 -> 4).
    HalfEven,
    /// Truncate toward zero (2.9 -> 2, -2.9 -> -2).
    Truncate,
}

impl RoundingMode {
    pub fn parse(s: &str) -> Option<RoundingMode> {
        match s
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], " ")
            .as_str()
        {
            "half up" | "halfup" | "up" => Some(RoundingMode::HalfUp),
            "bankers" | "banker's" | "half even" | "even" => Some(RoundingMode::HalfEven),
            "truncate" | "trunc" | "down" => Some(RoundingMode::Truncate),
            _ => None,
        }
    }
    pub fn as_setting(&self) -> &'static str {
        match self {
            RoundingMode::HalfUp => "half-up",
            RoundingMode::HalfEven => "bankers",
            RoundingMode::Truncate => "truncate",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            RoundingMode::HalfUp => "Half up",
            RoundingMode::HalfEven => "Banker's (half to even)",
            RoundingMode::Truncate => "Truncate",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DisplayMode {
    #[default]
    Off,
    SigFigs,
    Decimals,
    Engineering,
    /// #37: sig figs inferred from the inputs as typed.
    Auto,
    /// Exact mode (v0.5.0, program #14): fraction results are
    /// labelled exact, pi-bearing geometry also shows its exact
    /// form ("25*pi"), and the Expression Calculator evaluates
    /// rational expressions exactly. Everything else falls back
    /// to the plain decimal output — the exact-form line itself
    /// is produced by crate::students (it needs the calculator
    /// and the args) and appended by the CLI/GUI callers, so
    /// annotate_display/headline_display add nothing here.
    Exact,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct DisplayFormat {
    pub mode: DisplayMode,
    /// Figure count for SigFigs (1-10) / Decimals (0-10).
    pub n: Option<u8>,
    pub rounding: RoundingMode,
}

impl DisplayFormat {
    /// Parse the CLI `--format` value: `off`, `sig-figs:3`,
    /// `decimals:2`, `engineering`, `auto` (also `sigfigs:3`,
    /// `dec:2`, `eng`).
    pub fn parse(s: &str) -> Result<DisplayFormat, String> {
        let s = s.trim().to_ascii_lowercase();
        let (name, num) = match s.split_once(':') {
            Some((a, b)) => (a, Some(b)),
            None => (s.as_str(), None),
        };
        let n = match num {
            Some(v) => Some(
                v.parse::<u8>()
                    .map_err(|_| format!("'{s}' — the number after ':' must be a whole number."))?,
            ),
            None => None,
        };
        let mode = match name {
            "off" | "none" => DisplayMode::Off,
            "sig-figs" | "sigfigs" | "sig" | "sf" => {
                match n {
                    Some(v) if (1..=10).contains(&v) => {}
                    _ => return Err("--format sig-figs needs :N with N from 1 to 10.".into()),
                }
                DisplayMode::SigFigs
            }
            "decimals" | "dec" | "dp" => {
                match n {
                    Some(v) if v <= 10 => {}
                    _ => return Err("--format decimals needs :N with N from 0 to 10.".into()),
                }
                DisplayMode::Decimals
            }
            "engineering" | "eng" => DisplayMode::Engineering,
            "auto" => DisplayMode::Auto,
            "exact" => DisplayMode::Exact,
            _ => {
                return Err(format!(
                    "'{s}' is not a display format — use off, sig-figs:N, decimals:N, engineering or auto."
                ))
            }
        };
        Ok(DisplayFormat {
            mode,
            n,
            rounding: RoundingMode::HalfUp,
        })
    }
    pub fn as_setting(&self) -> String {
        match self.mode {
            DisplayMode::Off => "off".to_string(),
            DisplayMode::SigFigs => format!("sig-figs:{}", self.n.unwrap_or(3)),
            DisplayMode::Decimals => format!("decimals:{}", self.n.unwrap_or(2)),
            DisplayMode::Engineering => "engineering".to_string(),
            DisplayMode::Auto => "auto".to_string(),
            DisplayMode::Exact => "exact".to_string(),
        }
    }
    pub fn is_off(&self) -> bool {
        self.mode == DisplayMode::Off
    }
}

/// Round a value that has already been scaled to an integer
/// boundary (e.g. value x 10^decimals) using the rounding mode.
fn round_scaled(x: f64, mode: RoundingMode) -> f64 {
    if !x.is_finite() {
        return x;
    }
    match mode {
        RoundingMode::Truncate => x.trunc(),
        RoundingMode::HalfUp => {
            if x >= 0.0 {
                (x + 0.5).floor()
            } else {
                (x - 0.5).ceil()
            }
        }
        RoundingMode::HalfEven => {
            let f = x.floor();
            let d = x - f;
            if d < 0.5 {
                f
            } else if d > 0.5 {
                f + 1.0
            } else if (f as i64) % 2 == 0 {
                f
            } else {
                f + 1.0
            }
        }
    }
}

/// Round `value` to `dp` decimal places with the mode (#9 applied
/// inside the display formatters, #2).
pub fn round_dp(value: f64, dp: u8, mode: RoundingMode) -> f64 {
    let scale = 10f64.powi(dp as i32);
    round_scaled(value * scale, mode) / scale
}

/// #2: format with exactly `dp` decimal places (zeros kept, so the
/// chosen precision is visible).
pub fn format_decimals(value: f64, dp: u8, mode: RoundingMode) -> String {
    if !value.is_finite() {
        return crate::n(value);
    }
    let r = round_dp(value, dp, mode);
    let r = if r == 0.0 { 0.0 } else { r }; // avoid "-0.00"
    format!("{r:.prec$}", prec = dp as usize)
}

/// #1 with the rounding mode threaded through (#9). Same display
/// conventions as `features::format_sig_figs`.
pub fn format_sig_figs_r(value: f64, n: u8, mode: RoundingMode) -> String {
    let n = n.max(1) as i32;
    if !value.is_finite() {
        return crate::n(value);
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let a = value.abs();
    let mut e = a.log10().floor() as i32;
    let scale_exp = e - n + 1;
    let scaled = a / 10f64.powi(scale_exp);
    let mut r = round_scaled(scaled, mode);
    if r >= 10f64.powi(n) {
        r /= 10.0;
        e += 1;
    }
    if r < 10f64.powi(n - 1) && r != 0.0 {
        r *= 10.0;
        e -= 1;
    }
    let digits = format!("{}", r as i64);
    let digits = if digits.len() < n as usize {
        format!("{}{}", "0".repeat(n as usize - digits.len()), digits)
    } else {
        digits
    };
    if e >= n || e < -4 {
        let mantissa = if n == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        return format!("{sign}{mantissa}e{e}");
    }
    if e >= 0 {
        let int_len = (e + 1) as usize;
        if int_len >= digits.len() {
            format!("{sign}{digits}")
        } else {
            format!("{sign}{}.{}", &digits[..int_len], &digits[int_len..])
        }
    } else {
        format!("{sign}0.{}{}", "0".repeat((-e - 1) as usize), digits)
    }
}

/// #3: engineering notation — exponent a multiple of 3, with the SI
/// prefix letter where one exists (p, n, µ, m, k, M, G, T).
pub fn format_engineering(value: f64, mode: RoundingMode) -> String {
    if !value.is_finite() {
        return crate::n(value);
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let a = value.abs();
    let mut exp = (a.log10().floor() as i32).div_euclid(3) * 3;
    exp = exp.clamp(-12, 12);
    let mut mant = value / 10f64.powi(exp);
    mant = round_dp(mant, 4, mode);
    if mant.abs() >= 1000.0 && exp < 12 {
        mant /= 1000.0;
        exp += 3;
    }
    let prefix = match exp {
        -12 => " p",
        -9 => " n",
        -6 => " \u{b5}",
        -3 => " m",
        0 => "",
        3 => " k",
        6 => " M",
        9 => " G",
        12 => " T",
        _ => "",
    };
    let m = crate::nd(mant, 4);
    if exp == 0 {
        m
    } else if prefix.is_empty() {
        format!("{m}e{exp}")
    } else {
        format!("{m}{prefix} ({}e{})", mant_prefix_word(exp), exp)
    }
}

fn mant_prefix_word(exp: i32) -> &'static str {
    match exp {
        -12 => "pico",
        -9 => "nano",
        -6 => "micro",
        -3 => "milli",
        3 => "kilo",
        6 => "mega",
        9 => "giga",
        12 => "tera",
        _ => "",
    }
}

// ---------- number scanning (shared by annotation + copy + spec) ----------

/// Byte ranges of the numbers in `text`. Numbers inside dates
/// (touching `/`, or a digit-flanked `-`) are skipped, exactly as in
/// the sig-figs scanner.
pub fn number_spans(text: &str) -> Vec<(usize, usize)> {
    let b = text.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i] as char;
        let starts = c.is_ascii_digit()
            || (c == '.' && i + 1 < b.len() && (b[i + 1] as char).is_ascii_digit())
            || (c == '-'
                && i + 1 < b.len()
                && (b[i + 1] as char).is_ascii_digit()
                && (i == 0 || !(b[i - 1] as char).is_ascii_digit()));
        if !starts {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i;
        if b[j] == b'-' {
            j += 1;
        }
        let mut seen_dot = false;
        let mut seen_exp = false;
        while j < b.len() {
            let ch = b[j] as char;
            if ch.is_ascii_digit() {
                j += 1;
            } else if ch == '.'
                && !seen_dot
                && !seen_exp
                && j + 1 < b.len()
                && (b[j + 1] as char).is_ascii_digit()
            {
                seen_dot = true;
                j += 1;
            } else if (ch == 'e' || ch == 'E') && !seen_exp && j > start {
                let mut k = j + 1;
                if k < b.len() && (b[k] == b'-' || b[k] == b'+') {
                    k += 1;
                }
                if k < b.len() && (b[k] as char).is_ascii_digit() {
                    seen_exp = true;
                    j = k;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let before = if start == 0 {
            None
        } else {
            Some(b[start - 1] as char)
        };
        let after = if j >= b.len() {
            None
        } else {
            Some(b[j] as char)
        };
        let after_dash_digit =
            after == Some('-') && j + 1 < b.len() && (b[j + 1] as char).is_ascii_digit();
        let before_dash_digit =
            before == Some('-') && start >= 2 && (b[start - 2] as char).is_ascii_digit();
        // A digit run glued to a letter or underscore is part of
        // an identifier (x_1, CO2), not a standalone number.
        let in_identifier = matches!(before, Some(c) if c.is_ascii_alphabetic() || c == '_');
        if before != Some('/')
            && after != Some('/')
            && !after_dash_digit
            && !before_dash_digit
            && !in_identifier
        {
            spans.push((start, j));
        }
        i = j.max(start + 1);
    }
    spans
}

fn map_numbers(text: &str, f: impl Fn(f64) -> String) -> String {
    let spans = number_spans(text);
    if spans.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut pos = 0usize;
    for (s, e) in spans {
        out.push_str(&text[pos..s]);
        let tok = &text[s..e];
        match tok.parse::<f64>() {
            Ok(v) => out.push_str(&f(v)),
            Err(_) => out.push_str(tok),
        }
        pos = e;
    }
    out.push_str(&text[pos..]);
    out
}

// ---------- #37 sig-fig inference ----------

/// Count the significant figures in one number AS TYPED, by the
/// standard convention: leading zeros never count; trailing zeros
/// count only when a decimal point is present ("100" -> 1,
/// "100." -> 3, "1.00" -> 3, "0.004560" -> 4, "0.5" -> 1). A
/// scientific-notation exponent does not change the count.
pub fn count_sig_figs(raw: &str) -> Option<u32> {
    let t = raw.trim().trim_start_matches(['+', '-']);
    if t.is_empty() || t.parse::<f64>().is_err() {
        return None;
    }
    let mantissa = t.split(['e', 'E']).next().unwrap_or(t);
    let has_dot = mantissa.contains('.');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    if digits.chars().all(|c| c == '0') {
        return Some(1); // zero itself: one figure by convention here
    }
    let start = digits.find(|c| c != '0').unwrap_or(0);
    let trimmed = if has_dot {
        &digits[start..]
    } else {
        digits[start..].trim_end_matches('0')
    };
    Some(trimmed.len() as u32)
}

/// The minimum sig-fig count over every numeric token in the given
/// raw input strings (the Auto mode's precision), with the count.
pub fn infer_sig_figs(inputs: &[String]) -> Option<u8> {
    let mut best: Option<u32> = None;
    for raw in inputs {
        for tok in raw.split([',', ';', ' ', '\t']) {
            if let Some(c) = count_sig_figs(tok) {
                best = Some(best.map_or(c, |b| b.min(c)));
            }
        }
    }
    best.map(|c| c.min(10) as u8)
}

/// The display annotation for a result under a DisplayFormat. Only
/// ONE annotation line is ever added (DECISIONS: display-format
/// precedence). `auto_inputs` are the raw input strings, used only
/// by Auto mode. The full-precision text is never altered.
pub fn annotate_display(output: &str, fmt: &DisplayFormat, auto_inputs: &[String]) -> String {
    let head = output.lines().next().unwrap_or("");
    match fmt.mode {
        DisplayMode::Off => output.to_string(),
        DisplayMode::SigFigs => {
            let n = fmt.n.unwrap_or(3);
            let shown = map_numbers(head, |v| format_sig_figs_r(v, n, fmt.rounding));
            format!("{output}\n[To {n} significant figures: {shown}]")
        }
        DisplayMode::Decimals => {
            let n = fmt.n.unwrap_or(2);
            let shown = map_numbers(head, |v| format_decimals(v, n, fmt.rounding));
            format!("{output}\n[To {n} decimal places: {shown}]")
        }
        DisplayMode::Engineering => {
            let shown = map_numbers(head, |v| format_engineering(v, fmt.rounding));
            format!("{output}\n[Engineering notation: {shown}]")
        }
        DisplayMode::Auto => match infer_sig_figs(auto_inputs) {
            Some(n) => {
                let shown = map_numbers(head, |v| format_sig_figs_r(v, n, fmt.rounding));
                format!(
                    "{output}\n[Inputs carry {n} significant figures — headline to {n} significant figures: {shown}]"
                )
            }
            None => output.to_string(),
        },
        // Exact mode: the caller appends the crate::students
        // exact-form line.
        DisplayMode::Exact => output.to_string(),
    }
}

/// Just the re-rendered headline under a format (GUI display).
pub fn headline_display(
    output: &str,
    fmt: &DisplayFormat,
    auto_inputs: &[String],
) -> Option<String> {
    let head = output.lines().next().unwrap_or("");
    match fmt.mode {
        DisplayMode::Off => None,
        DisplayMode::SigFigs => {
            let n = fmt.n.unwrap_or(3);
            Some(format!(
                "\u{2248} to {n} significant figures: {}",
                map_numbers(head, |v| format_sig_figs_r(v, n, fmt.rounding))
            ))
        }
        DisplayMode::Decimals => {
            let n = fmt.n.unwrap_or(2);
            Some(format!(
                "\u{2248} to {n} decimal places: {}",
                map_numbers(head, |v| format_decimals(v, n, fmt.rounding))
            ))
        }
        DisplayMode::Engineering => Some(format!(
            "\u{2248} engineering: {}",
            map_numbers(head, |v| format_engineering(v, fmt.rounding))
        )),
        DisplayMode::Auto => infer_sig_figs(auto_inputs).map(|n| {
            format!(
                "\u{2248} inputs carry {n} sig figs: {}",
                map_numbers(head, |v| format_sig_figs_r(v, n, fmt.rounding))
            )
        }),
        // Exact mode: the exact-form line is shown by the caller.
        DisplayMode::Exact => None,
    }
}

// ---------- #4 copy formats ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyFormat {
    Plain,
    Latex,
    Scientific,
    Csv,
}

impl CopyFormat {
    pub fn parse(s: &str) -> Option<CopyFormat> {
        match s.trim().to_ascii_lowercase().as_str() {
            "plain" | "text" => Some(CopyFormat::Plain),
            "latex" | "tex" => Some(CopyFormat::Latex),
            "scientific" | "sci" => Some(CopyFormat::Scientific),
            "csv" => Some(CopyFormat::Csv),
            _ => None,
        }
    }
}

fn sci_trim(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let s = format!("{v:.6e}");
    let (mant, exp) = s.split_once('e').unwrap();
    let mant = mant.trim_end_matches('0').trim_end_matches('.');
    format!("{mant}e{exp}")
}

fn latex_escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\textbackslash{}"),
            '&' => out.push_str("\\&"),
            '%' => out.push_str("\\%"),
            '$' => out.push_str("\\$"),
            '#' => out.push_str("\\#"),
            '_' => out.push_str("\\_"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\^{}"),
            _ => out.push(ch),
        }
    }
    out
}

/// Format a full result for copying / `--as` (#4). LaTeX: the text
/// is escaped and every number is wrapped in $...$ math mode — an
/// honest export of the values and lines CalcDex printed, NOT a
/// re-derivation of the formula in LaTeX (documented limit).
/// CSV: one `label,value` row per result line that reads as
/// `label: value` or `label = value`; any other line becomes a
/// single text field row.
pub fn format_copy(output: &str, fmt: CopyFormat) -> String {
    match fmt {
        CopyFormat::Plain => output.to_string(),
        CopyFormat::Scientific => output
            .lines()
            .map(|line| {
                // Numbers right after '^' are exponents/indices in
                // the printed working (1.75^2) — leave them plain
                // instead of turning them into 2e0 noise.
                let spans = number_spans(line);
                if spans.is_empty() {
                    return line.to_string();
                }
                let mut out = String::new();
                let mut pos = 0usize;
                for (s, e) in spans {
                    out.push_str(&line[pos..s]);
                    if s > 0 && line.as_bytes()[s - 1] == b'^' {
                        out.push_str(&line[s..e]);
                    } else if let Ok(v) = line[s..e].parse::<f64>() {
                        out.push_str(&sci_trim(v));
                    } else {
                        out.push_str(&line[s..e]);
                    }
                    pos = e;
                }
                out.push_str(&line[pos..]);
                out
            })
            .collect::<Vec<_>>()
            .join("\n"),
        CopyFormat::Csv => {
            let mut rows = Vec::new();
            for line in output.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Some((label, value)) = line.split_once(": ") {
                    rows.push(csv_row(&[
                        label.trim().to_string(),
                        value.trim().to_string(),
                    ]));
                } else if let Some(pos) = line.rfind(" = ") {
                    rows.push(csv_row(&[
                        line[..pos].trim().to_string(),
                        line[pos + 3..].trim().to_string(),
                    ]));
                } else {
                    rows.push(csv_row(&[line.trim().to_string()]));
                }
            }
            rows.join("\n")
        }
        CopyFormat::Latex => output
            .lines()
            .map(|line| {
                let spans = number_spans(line);
                if spans.is_empty() {
                    return latex_escape(line);
                }
                let mut out = String::new();
                let mut pos = 0usize;
                for (s, e) in spans {
                    out.push_str(&latex_escape(&line[pos..s]));
                    out.push('$');
                    out.push_str(&line[s..e]);
                    out.push('$');
                    pos = e;
                }
                out.push_str(&latex_escape(&line[pos..]));
                out
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

// ---------- #10 spec check ----------

#[derive(Clone, Debug, PartialEq)]
pub struct SpecVerdict {
    pub label: String,
    pub value: f64,
    pub pass: bool,
    pub note: String,
}

/// Extract a line's result value: the first number after the first
/// ':' when the line has one (labelled results), else the number
/// after the last '=' (equation-style results), else the first
/// number. Steps lines (after a "Steps:" line) are not results.
fn line_result_value(line: &str) -> Option<(String, f64)> {
    let spans = number_spans(line);
    if spans.is_empty() {
        return None;
    }
    let pick = |idx: usize| line[spans[idx].0..spans[idx].1].parse::<f64>().ok();
    if let Some(colon) = line.find(':') {
        if let Some(idx) = spans.iter().position(|(s, _)| *s > colon) {
            let label = line[..colon].trim().to_string();
            return pick(idx).map(|v| (label, v));
        }
    }
    if line.contains('=') {
        let idx = spans.len() - 1;
        let label = line[..line.rfind('=').unwrap()].trim().to_string();
        return pick(idx).map(|v| (label, v));
    }
    let label = line.trim().chars().take(48).collect::<String>();
    pick(0).map(|v| (label, v))
}

/// Compare every numeric result line against [min, max] (#10).
/// Limits are INCLUSIVE: a value exactly on a limit PASSes.
pub fn spec_check(output: &str, min: f64, max: f64) -> Vec<SpecVerdict> {
    let mut out = Vec::new();
    let mut in_steps = false;
    for line in output.lines() {
        if line.trim_start().starts_with("Steps:") {
            in_steps = true;
        }
        if in_steps {
            continue;
        }
        let Some((label, value)) = line_result_value(line) else {
            continue;
        };
        let (pass, note) = if value < min {
            (
                false,
                format!(
                    "{} is below the minimum {} by {}",
                    crate::n(value),
                    crate::n(min),
                    crate::n(min - value)
                ),
            )
        } else if value > max {
            (
                false,
                format!(
                    "{} is above the maximum {} by {}",
                    crate::n(value),
                    crate::n(max),
                    crate::n(value - max)
                ),
            )
        } else {
            (
                true,
                format!(
                    "{} is inside [{}, {}]",
                    crate::n(value),
                    crate::n(min),
                    crate::n(max)
                ),
            )
        };
        out.push(SpecVerdict {
            label,
            value,
            pass,
            note,
        });
    }
    out
}

pub fn spec_verdict_lines(output: &str, min: f64, max: f64) -> Vec<String> {
    spec_check(output, min, max)
        .iter()
        .map(|v| {
            format!(
                "{}: {} — {}",
                if v.pass { "PASS" } else { "FAIL" },
                v.label,
                v.note
            )
        })
        .collect()
}

// ---------- #38 constants library (CODATA 2018) ----------
//
// The named physical constants of the Expression Calculator:
// the published CODATA 2018 values (the exact SI-defining values
// are marked exact in the listing text). Each constant is usable
// in an expression by its symbol or any of its aliases — the
// tokenizer lowercases identifiers, so `N_A`, `n_a` and `NA`-style
// spellings all land on the same alias list. One deliberate
// exception: the elementary charge's symbol is `e`, but in an
// expression a bare `e` stays Euler's number (the meaning it has
// always had here), so the charge is written `e_charge`.

pub struct Constant {
    /// Full name, for listings.
    pub name: &'static str,
    /// The identifier the GUI's INSERT button puts in the field.
    pub symbol: &'static str,
    pub value: f64,
    pub unit: &'static str,
    /// Lowercase identifiers accepted in expressions.
    pub aliases: &'static [&'static str],
}

pub const CONSTANTS: &[Constant] = &[
    Constant {
        name: "Speed of light in vacuum",
        symbol: "c",
        value: 299_792_458.0,
        unit: "m/s (exact)",
        aliases: &["c"],
    },
    Constant {
        name: "Planck constant",
        symbol: "h",
        value: 6.626_070_15e-34,
        unit: "J s (exact)",
        aliases: &["h", "planck"],
    },
    Constant {
        name: "Reduced Planck constant",
        symbol: "hbar",
        value: 1.054_571_817e-34,
        unit: "J s",
        aliases: &["hbar", "\u{127}"],
    },
    Constant {
        name: "Elementary charge (bare e stays Euler's number — write e_charge)",
        symbol: "e_charge",
        value: 1.602_176_634e-19,
        unit: "C (exact)",
        aliases: &["e_charge", "elementary_charge", "qe"],
    },
    Constant {
        name: "Boltzmann constant",
        symbol: "k_b",
        value: 1.380_649e-23,
        unit: "J/K (exact)",
        aliases: &["k_b", "kb", "boltzmann"],
    },
    Constant {
        name: "Avogadro constant",
        symbol: "n_a",
        value: 6.022_140_76e23,
        unit: "1/mol (exact)",
        aliases: &["n_a", "na", "avogadro"],
    },
    Constant {
        name: "Molar gas constant",
        symbol: "r",
        value: 8.314_462_618,
        unit: "J/(mol K)",
        aliases: &["r", "r_gas", "gas_constant"],
    },
    Constant {
        name: "Newtonian gravitational constant",
        symbol: "g",
        value: 6.674_30e-11,
        unit: "m^3/(kg s^2)",
        aliases: &["g", "g_newton", "gravitational_constant"],
    },
    Constant {
        name: "Vacuum electric permittivity",
        symbol: "eps0",
        value: 8.8541878128e-12,
        unit: "F/m",
        aliases: &["eps0", "epsilon0", "\u{3b5}0"],
    },
    Constant {
        name: "Vacuum magnetic permeability",
        symbol: "mu0",
        value: 1.256_637_062_12e-6,
        unit: "N/A^2",
        aliases: &["mu0", "\u{3bc}0"],
    },
    Constant {
        name: "Electron mass",
        symbol: "m_e",
        value: 9.1093837015e-31,
        unit: "kg",
        aliases: &["m_e", "me", "electron_mass"],
    },
    Constant {
        name: "Proton mass",
        symbol: "m_p",
        value: 1.672_621_923_69e-27,
        unit: "kg",
        aliases: &["m_p", "mp", "proton_mass"],
    },
    Constant {
        name: "Atomic mass constant (one dalton)",
        symbol: "u",
        value: 1.660_539_066_60e-27,
        unit: "kg",
        aliases: &["u", "amu", "dalton"],
    },
    Constant {
        name: "Faraday constant",
        symbol: "faraday",
        value: 96_485.332_12,
        unit: "C/mol",
        aliases: &["faraday", "f_const"],
    },
    Constant {
        name: "Stefan-Boltzmann constant",
        symbol: "sigma",
        value: 5.670_374_419e-8,
        unit: "W/(m^2 K^4)",
        aliases: &["sigma", "sigma_sb", "stefan_boltzmann", "\u{3c3}"],
    },
];

/// A constant's value for listings: plain for everyday sizes,
/// scientific notation (full stored precision, zeros trimmed)
/// for the very small or very large.
pub fn constant_value_str(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if v.abs() < 1e-3 || v.abs() >= 1e16 {
        let s = format!("{v:.12e}");
        let (mant, exp) = s.split_once('e').unwrap_or((&s, ""));
        let mant = mant.trim_end_matches('0').trim_end_matches('.');
        return format!("{mant}e{exp}");
    }
    // Everyday magnitudes print with the program's standard
    // formatter (10 decimals, trimmed) — full CODATA precision
    // at these scales, without binary-noise tail digits.
    crate::n(v)
}

/// Look a built-in constant up by any accepted identifier
/// (case-insensitive; the expression tokenizer lowercases).
pub fn builtin_constant(ident: &str) -> Option<&'static Constant> {
    let id = ident.to_ascii_lowercase();
    CONSTANTS.iter().find(|c| c.aliases.contains(&id.as_str()))
}

/// Names an expression can never use for a user constant: the
/// function names, the mathematical constants and the plot
/// variable.
const RESERVED_NAMES: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "sqrt", "ln", "log10", "exp", "abs", "pi", "e",
    "tau", "x",
];

/// Validate a user-defined constant's name (#38). Returns the
/// normalized (trimmed, lowercased) name, or a teaching error
/// naming the problem: bad shape, a reserved name, or a collision
/// with a built-in constant (built-ins win — a user value must
/// never silently shadow a CODATA value).
pub fn validate_constant_name(raw: &str) -> Result<String, String> {
    let name = raw.trim().to_ascii_lowercase();
    if name.is_empty() {
        return Err("A constant needs a name — letters, digits and underscores, starting with a letter (e.g. my_g or tube_volume).".into());
    }
    if name.len() > 24 {
        return Err(format!(
            "'{name}' is too long for a constant name — 24 characters at most."
        ));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() {
        return Err(format!(
            "'{name}' cannot be a constant name — it must start with a letter."
        ));
    }
    if !name.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
        return Err(format!(
            "'{name}' cannot be a constant name — use only letters, digits and underscores."
        ));
    }
    if RESERVED_NAMES.contains(&name.as_str()) {
        return Err(format!(
            "'{name}' is already taken by the expression language itself (a function, pi/e/tau, or the plot variable x) — pick another name."
        ));
    }
    if let Some(c) = builtin_constant(&name) {
        return Err(format!(
            "'{name}' is the built-in constant {} ({}) — built-in constants cannot be replaced. Pick another name.",
            c.name, c.symbol
        ));
    }
    Ok(name)
}

// User-defined constants live in the shared settings file, but
// the expression parser is a pure library function with no
// settings handle — so the shell (window and CLI alike) installs
// the file's constants into this process-wide table at startup
// and after every change, and the parser resolves through it.
// Built-ins are consulted first and can never be shadowed
// (validate_constant_name refuses those names up front anyway).
static CUSTOM_CONSTANTS: std::sync::OnceLock<std::sync::RwLock<Vec<(String, f64)>>> =
    std::sync::OnceLock::new();

fn custom_constants_lock() -> &'static std::sync::RwLock<Vec<(String, f64)>> {
    CUSTOM_CONSTANTS.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

/// Install the user-defined constants (called by the shell from
/// the shared settings file).
pub fn set_custom_constants(list: Vec<(String, f64)>) {
    if let Ok(mut guard) = custom_constants_lock().write() {
        *guard = list;
    }
}

/// The installed user-defined constants, in definition order.
pub fn custom_constants() -> Vec<(String, f64)> {
    custom_constants_lock()
        .read()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Look an installed user constant up by name (case-insensitive).
pub fn custom_constant(ident: &str) -> Option<f64> {
    let id = ident.to_ascii_lowercase();
    custom_constants_lock()
        .read()
        .ok()?
        .iter()
        .find(|(n, _)| *n == id)
        .map(|(_, v)| *v)
}

// ---------- #5 expression parser ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleMode {
    Degrees,
    Radians,
}

#[derive(Clone, Debug)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    /// The ± sign (v0.7.0 #40): binds an uncertainty to the atom
    /// before it. Only the uncertainty evaluator consumes it; the
    /// plain parser treats it as unexpected input.
    PlusMinus,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    /// 1-based character position in the source, for error messages.
    pos: usize,
}

fn tokenize(src: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        let pos = i + 1;
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit()
            || (ch == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            let mut seen_dot = false;
            let mut seen_exp = false;
            while i < chars.len() {
                let c = chars[i];
                if c.is_ascii_digit() {
                    i += 1;
                } else if c == '.' && !seen_dot && !seen_exp {
                    seen_dot = true;
                    i += 1;
                } else if (c == 'e' || c == 'E') && !seen_exp {
                    let mut k = i + 1;
                    if k < chars.len() && (chars[k] == '-' || chars[k] == '+') {
                        k += 1;
                    }
                    if k < chars.len() && chars[k].is_ascii_digit() {
                        seen_exp = true;
                        i = k;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            let text: String = chars[start..i].iter().collect();
            match text.parse::<f64>() {
                Ok(v) => toks.push(Token {
                    tok: Tok::Num(v),
                    pos,
                }),
                Err(_) => {
                    return Err(format!(
                        "Parse error at position {pos}: '{text}' is not a number."
                    ))
                }
            }
            continue;
        }
        if ch.is_alphabetic() {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            toks.push(Token {
                tok: Tok::Ident(name.to_ascii_lowercase()),
                pos,
            });
            continue;
        }
        match ch {
            '+' | '-' | '*' | '/' | '^' => toks.push(Token {
                tok: Tok::Op(ch),
                pos,
            }),
            '(' => toks.push(Token {
                tok: Tok::LParen,
                pos,
            }),
            ')' => toks.push(Token {
                tok: Tok::RParen,
                pos,
            }),
            '\u{b1}' => toks.push(Token {
                tok: Tok::PlusMinus,
                pos,
            }),
            _ => {
                return Err(format!(
                    "Parse error at position {pos}: '{ch}' is not part of an expression — use numbers, + - * / ^, brackets, and function names like sqrt."
                ))
            }
        }
        i += 1;
    }
    Ok(toks)
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
    mode: AngleMode,
    src_len: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.i)
    }
    fn err_expected(&self, what: &str) -> String {
        match self.peek() {
            Some(t) => format!("Parse error at position {}: expected {what} here.", t.pos),
            None => format!(
                "Parse error at position {}: expected {what}, but the expression ended.",
                self.src_len + 1
            ),
        }
    }
    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut v = self.parse_term()?;
        while let Some(t) = self.peek() {
            let Tok::Op(op) = t.tok else { break };
            if op != '+' && op != '-' {
                break;
            }
            self.i += 1;
            let rhs = self.parse_term()?;
            v = if op == '+' { v + rhs } else { v - rhs };
        }
        Ok(v)
    }
    fn parse_term(&mut self) -> Result<f64, String> {
        let mut v = self.parse_unary()?;
        while let Some(t) = self.peek() {
            let Tok::Op(op) = t.tok else { break };
            if op != '*' && op != '/' {
                break;
            }
            self.i += 1;
            let rhs = self.parse_unary()?;
            if op == '/' && rhs == 0.0 {
                return Err("Division by zero has no answer.".into());
            }
            v = if op == '*' { v * rhs } else { v / rhs };
        }
        Ok(v)
    }
    fn parse_unary(&mut self) -> Result<f64, String> {
        if let Some(t) = self.peek() {
            if matches!(t.tok, Tok::Op('-')) {
                self.i += 1;
                return Ok(-self.parse_unary()?);
            }
            if matches!(t.tok, Tok::Op('+')) {
                self.i += 1;
                return self.parse_unary();
            }
        }
        self.parse_power()
    }
    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_atom()?;
        if let Some(t) = self.peek() {
            if matches!(t.tok, Tok::Op('^')) {
                self.i += 1;
                // Right-associative: the exponent is a full unary
                // (2^3^2 = 2^(3^2) = 512), and may be negative.
                let exp = self.parse_unary()?;
                return Ok(base.powf(exp));
            }
        }
        Ok(base)
    }
    fn parse_atom(&mut self) -> Result<f64, String> {
        let Some(t) = self.peek().cloned() else {
            return Err(self.err_expected("a number, a constant, or '('"));
        };
        match t.tok {
            Tok::Num(v) => {
                self.i += 1;
                Ok(v)
            }
            Tok::LParen => {
                self.i += 1;
                let v = self.parse_expr()?;
                match self.peek() {
                    Some(x) if matches!(x.tok, Tok::RParen) => {
                        self.i += 1;
                        Ok(v)
                    }
                    _ => Err(self.err_expected("')'")),
                }
            }
            Tok::Ident(name) => {
                self.i += 1;
                match name.as_str() {
                    "pi" => Ok(std::f64::consts::PI),
                    "e" => Ok(std::f64::consts::E),
                    "tau" => Ok(std::f64::consts::TAU),
                    "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sqrt" | "ln" | "log10"
                    | "exp" | "abs" => {
                        match self.peek() {
                            Some(x) if matches!(x.tok, Tok::LParen) => {
                                self.i += 1;
                            }
                            _ => {
                                return Err(self.err_expected(&format!(
                                    "'(' after the function name '{name}'"
                                )))
                            }
                        }
                        let arg = self.parse_expr()?;
                        match self.peek() {
                            Some(x) if matches!(x.tok, Tok::RParen) => {
                                self.i += 1;
                            }
                            _ => return Err(self.err_expected("')'")),
                        }
                        self.apply_fn(&name, arg)
                    }
                    _ => {
                        // v0.7.0 #38: the CODATA constants library,
                        // then the user's own defined constants.
                        if let Some(c) = builtin_constant(&name) {
                            return Ok(c.value);
                        }
                        if let Some(v) = custom_constant(&name) {
                            return Ok(v);
                        }
                        Err(format!(
                            "Parse error at position {}: '{name}' is not a known constant or function. Constants: pi, e, tau, and the CODATA library (c, h, k_b, n_a, ...) plus any you define — CalcDex --constants lists them all. Functions: sin, cos, tan, asin, acos, atan, sqrt, ln, log10, exp, abs.",
                            t.pos
                        ))
                    }
                }
            }
            _ => Err(self.err_expected("a number, a constant, or '('")),
        }
    }
    fn apply_fn(&self, name: &str, arg: f64) -> Result<f64, String> {
        apply_fn_value(name, arg, self.mode)
    }
}

/// The function table of the expression language, free-standing
/// so the uncertainty evaluator (#40) shares it exactly.
fn apply_fn_value(name: &str, arg: f64, mode: AngleMode) -> Result<f64, String> {
    {
        let deg = mode == AngleMode::Degrees;
        let to_rad = |x: f64| {
            if deg {
                x * std::f64::consts::PI / 180.0
            } else {
                x
            }
        };
        let from_rad = |x: f64| {
            if deg {
                x * 180.0 / std::f64::consts::PI
            } else {
                x
            }
        };
        let v = match name {
            "sin" => to_rad(arg).sin(),
            "cos" => to_rad(arg).cos(),
            "tan" => to_rad(arg).tan(),
            "asin" => {
                if !(-1.0..=1.0).contains(&arg) {
                    return Err(
                        "asin needs a value between -1 and 1 (it undoes sin, whose results are in that range)."
                            .into(),
                    );
                }
                from_rad(arg.asin())
            }
            "acos" => {
                if !(-1.0..=1.0).contains(&arg) {
                    return Err(
                        "acos needs a value between -1 and 1 (it undoes cos, whose results are in that range)."
                            .into(),
                    );
                }
                from_rad(arg.acos())
            }
            "atan" => from_rad(arg.atan()),
            "sqrt" => {
                if arg < 0.0 {
                    return Err("sqrt of a negative number has no real answer.".into());
                }
                arg.sqrt()
            }
            "ln" => {
                if arg <= 0.0 {
                    return Err("ln needs a value over 0.".into());
                }
                arg.ln()
            }
            "log10" => {
                if arg <= 0.0 {
                    return Err("log10 needs a value over 0.".into());
                }
                arg.log10()
            }
            "exp" => arg.exp(),
            "abs" => arg.abs(),
            _ => return Err(format!("Unknown function '{name}'.")),
        };
        if !v.is_finite() {
            return Err("That expression produced a number too big (or not a number).".into());
        }
        Ok(v)
    }
}

/// Evaluate an expression (#5). Errors teach: they name the
/// position and what was expected there.
pub fn eval_expression(src: &str, mode: AngleMode) -> Result<f64, String> {
    let toks = tokenize(src)?;
    if toks.is_empty() {
        return Err("The expression is empty — type something like 2*(3+4)^2.".into());
    }
    let mut p = Parser {
        toks,
        i: 0,
        mode,
        src_len: src.chars().count(),
    };
    let v = p.parse_expr()?;
    if let Some(t) = p.peek() {
        return Err(format!(
            "Parse error at position {}: the expression was complete, but more input follows — check for a missing operator.",
            t.pos
        ));
    }
    if !v.is_finite() {
        return Err("That expression produced a number too big (or not a number).".into());
    }
    Ok(v)
}

// ---------- #40 uncertainty propagation ----------
//
// A value written `10 ± 0.2` (or `10 +/- 0.2`) is a measurement:
// a value with an uncertainty. The ± binds to the atom directly
// before it — a number, a constant, or a bracketed expression —
// tighter than any operator, so `2 * 10 ± 0.2` is 2 x (10 ± 0.2).
// Propagation is the standard first-order (maximum-error) lab
// rules, and every rule that fires is named in `steps`:
//   + and - : absolute uncertainties add.
//   x and / : relative uncertainties add.
//   ^ n     : the relative uncertainty scales by |n|; with an
//             uncertain exponent too, the combined relative rule
//             sqrt((n.u/a)^2 + (ln a . v)^2) is used.
//   f(x)    : the uncertainty scales by |f'(x)| (the slope rule).
// Plain values are measurements with zero uncertainty, so mixed
// expressions work throughout. DECISIONS.md records the grammar.

/// The result of an uncertainty-propagating evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct UncertainOutcome {
    pub value: f64,
    pub uncertainty: f64,
    /// One line per propagation rule that fired, in order.
    pub steps: Vec<String>,
}

/// True when the text carries an uncertainty marker (± or +/-).
pub fn has_uncertainty(text: &str) -> bool {
    text.contains('\u{b1}') || text.contains("+/-")
}

/// "50 ± 2 (4%)" — the percentage is the uncertainty's share of
/// the value, omitted when the value is zero. Magnitudes use
/// n_full so a photon-scale result never prints as 0.
pub fn format_uncertain(o: &UncertainOutcome) -> String {
    if o.uncertainty <= 0.0 {
        return crate::n_full(o.value);
    }
    let pct = if o.value != 0.0 {
        format!(
            " ({}%)",
            crate::nd(o.uncertainty / o.value.abs() * 100.0, 2)
        )
    } else {
        String::new()
    };
    format!(
        "{} \u{b1} {}{}",
        crate::n_full(o.value),
        crate::n_full(o.uncertainty),
        pct
    )
}

#[derive(Clone, Copy, Debug)]
struct UVal {
    v: f64,
    u: f64,
}

fn pct(u: f64, v: f64) -> String {
    if v == 0.0 {
        "\u{2014}".to_string()
    } else {
        format!("{}%", crate::nd(u / v.abs() * 100.0, 2))
    }
}

struct UParser {
    toks: Vec<Token>,
    i: usize,
    mode: AngleMode,
    src_len: usize,
    steps: Vec<String>,
}

impl UParser {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.i)
    }
    fn err_expected(&self, what: &str) -> String {
        match self.peek() {
            Some(t) => format!("Parse error at position {}: expected {what} here.", t.pos),
            None => format!(
                "Parse error at position {}: expected {what}, but the expression ended.",
                self.src_len + 1
            ),
        }
    }
    fn parse_expr(&mut self) -> Result<UVal, String> {
        let mut v = self.parse_term()?;
        while let Some(t) = self.peek() {
            let Tok::Op(op) = t.tok else { break };
            if op != '+' && op != '-' {
                break;
            }
            self.i += 1;
            let rhs = self.parse_term()?;
            v = self.add_sub(v, rhs, op);
        }
        Ok(v)
    }
    fn parse_term(&mut self) -> Result<UVal, String> {
        let mut v = self.parse_unary()?;
        while let Some(t) = self.peek() {
            let Tok::Op(op) = t.tok else { break };
            if op != '*' && op != '/' {
                break;
            }
            self.i += 1;
            let rhs = self.parse_unary()?;
            v = if op == '*' {
                self.mul(v, rhs)
            } else {
                self.div(v, rhs)?
            };
        }
        Ok(v)
    }
    fn parse_unary(&mut self) -> Result<UVal, String> {
        if let Some(t) = self.peek() {
            if matches!(t.tok, Tok::Op('-')) {
                self.i += 1;
                let x = self.parse_unary()?;
                return Ok(UVal { v: -x.v, u: x.u });
            }
            if matches!(t.tok, Tok::Op('+')) {
                self.i += 1;
                return self.parse_unary();
            }
        }
        self.parse_power()
    }
    fn parse_power(&mut self) -> Result<UVal, String> {
        let base = self.parse_atom()?;
        if let Some(t) = self.peek() {
            if matches!(t.tok, Tok::Op('^')) {
                self.i += 1;
                // Right-associative, exactly like the plain parser.
                let exp = self.parse_unary()?;
                return self.pow(base, exp);
            }
        }
        Ok(base)
    }
    /// After an atom: an optional ± uncertainty. For numbers and
    /// constants it sets the uncertainty; after brackets it may
    /// only appear when the inside carried none.
    fn pm_suffix(&mut self, mut x: UVal, what: &str) -> Result<UVal, String> {
        if let Some(t) = self.peek() {
            if matches!(t.tok, Tok::PlusMinus) {
                self.i += 1;
                match self.peek().map(|t| t.tok.clone()) {
                    Some(Tok::Num(u)) => {
                        self.i += 1;
                        if u < 0.0 {
                            return Err(
                                "An uncertainty cannot be negative — write the size of the error, e.g. 10 \u{b1} 0.2."
                                    .into(),
                            );
                        }
                        if x.u > 0.0 {
                            return Err(format!(
                                "This {what} already has an uncertainty from inside — put the \u{b1} in one place only."
                            ));
                        }
                        x.u = u;
                        return Ok(x);
                    }
                    _ => return Err(self.err_expected("an uncertainty (a number) after \u{b1}")),
                }
            }
        }
        Ok(x)
    }
    fn parse_atom(&mut self) -> Result<UVal, String> {
        let Some(t) = self.peek().cloned() else {
            return Err(self.err_expected("a number, a constant, or '('"));
        };
        match t.tok {
            Tok::Num(v) => {
                self.i += 1;
                self.pm_suffix(UVal { v, u: 0.0 }, "number")
            }
            Tok::LParen => {
                self.i += 1;
                let v = self.parse_expr()?;
                match self.peek() {
                    Some(x) if matches!(x.tok, Tok::RParen) => {
                        self.i += 1;
                    }
                    _ => return Err(self.err_expected("')'")),
                }
                self.pm_suffix(v, "bracketed expression")
            }
            Tok::Ident(name) => {
                self.i += 1;
                match name.as_str() {
                    "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sqrt" | "ln" | "log10"
                    | "exp" | "abs" => {
                        match self.peek() {
                            Some(x) if matches!(x.tok, Tok::LParen) => {
                                self.i += 1;
                            }
                            _ => {
                                return Err(self.err_expected(&format!(
                                    "'(' after the function name '{name}'"
                                )))
                            }
                        }
                        let arg = self.parse_expr()?;
                        match self.peek() {
                            Some(x) if matches!(x.tok, Tok::RParen) => {
                                self.i += 1;
                            }
                            _ => return Err(self.err_expected("')'")),
                        }
                        self.apply_fn_unc(&name, arg)
                    }
                    _ => {
                        let v = match name.as_str() {
                            "pi" => std::f64::consts::PI,
                            "e" => std::f64::consts::E,
                            "tau" => std::f64::consts::TAU,
                            _ => {
                                if let Some(c) = builtin_constant(&name) {
                                    c.value
                                } else if let Some(v) = custom_constant(&name) {
                                    v
                                } else {
                                    return Err(format!(
                                        "Parse error at position {}: '{name}' is not a known constant or function. Constants: pi, e, tau, and the CODATA library (c, h, k_b, n_a, ...) plus any you define — CalcDex --constants lists them all. Functions: sin, cos, tan, asin, acos, atan, sqrt, ln, log10, exp, abs.",
                                        t.pos
                                    ));
                                }
                            }
                        };
                        self.pm_suffix(UVal { v, u: 0.0 }, "constant")
                    }
                }
            }
            _ => Err(self.err_expected("a number, a constant, or '('")),
        }
    }
    fn add_sub(&mut self, x: UVal, y: UVal, op: char) -> UVal {
        let u = x.u + y.u;
        if x.u > 0.0 || y.u > 0.0 {
            self.steps.push(format!(
                "{}: absolute uncertainties add — {} + {} = {}.",
                if op == '+' { "Addition" } else { "Subtraction" },
                crate::n(x.u),
                crate::n(y.u),
                crate::n(u)
            ));
        }
        UVal {
            v: if op == '+' { x.v + y.v } else { x.v - y.v },
            u,
        }
    }
    fn mul(&mut self, x: UVal, y: UVal) -> UVal {
        let v = x.v * y.v;
        let u = if x.v == 0.0 || y.v == 0.0 {
            // A zero factor kills the relative form; the absolute
            // form still holds: u = |x| uy + |y| ux.
            x.v.abs() * y.u + y.v.abs() * x.u
        } else {
            v.abs() * (x.u / x.v.abs() + y.u / y.v.abs())
        };
        if x.u > 0.0 || y.u > 0.0 {
            self.steps.push(format!(
                "Multiplication: relative uncertainties add — {} + {} = {} of the result.",
                pct(x.u, x.v),
                pct(y.u, y.v),
                pct(u, v)
            ));
        }
        UVal { v, u }
    }
    fn div(&mut self, x: UVal, y: UVal) -> Result<UVal, String> {
        if y.v == 0.0 {
            return Err("Division by zero has no answer.".into());
        }
        let v = x.v / y.v;
        let u = if x.v == 0.0 {
            x.u / y.v.abs()
        } else {
            v.abs() * (x.u / x.v.abs() + y.u / y.v.abs())
        };
        if x.u > 0.0 || y.u > 0.0 {
            self.steps.push(format!(
                "Division: relative uncertainties add — {} + {} = {} of the result.",
                pct(x.u, x.v),
                pct(y.u, y.v),
                pct(u, v)
            ));
        }
        Ok(UVal { v, u })
    }
    fn pow(&mut self, x: UVal, y: UVal) -> Result<UVal, String> {
        let v = x.v.powf(y.v);
        if !v.is_finite() {
            return Err("That expression produced a number too big (or not a number).".into());
        }
        if y.u == 0.0 {
            if x.u == 0.0 {
                return Ok(UVal { v, u: 0.0 });
            }
            if x.v == 0.0 {
                return Err(
                    "0 with an uncertainty cannot be raised to a power this way — the slope at 0 is not defined."
                        .into(),
                );
            }
            let u = v.abs() * y.v.abs() * (x.u / x.v.abs());
            self.steps.push(format!(
                "Power: the relative uncertainty scales by the power — {} x {} = {} of the result.",
                crate::n(y.v.abs()),
                pct(x.u, x.v),
                pct(u, v)
            ));
            return Ok(UVal { v, u });
        }
        // Both parts uncertain: the combined relative rule.
        if x.v <= 0.0 {
            return Err(
                "A power with an uncertain exponent needs a positive base (the rule uses ln of the base)."
                    .into(),
            );
        }
        let rel = ((y.v * x.u / x.v).powi(2) + (x.v.ln() * y.u).powi(2)).sqrt();
        let u = v.abs() * rel;
        self.steps.push(format!(
            "Power with both parts uncertain: the combined relative rule sqrt((n x u_base/base)^2 + (ln(base) x u_exp)^2) gives {} of the result.",
            pct(u, v)
        ));
        Ok(UVal { v, u })
    }
    fn apply_fn_unc(&mut self, name: &str, arg: UVal) -> Result<UVal, String> {
        let v = apply_fn_value(name, arg.v, self.mode)?;
        if arg.u == 0.0 {
            return Ok(UVal { v, u: 0.0 });
        }
        // First-order (slope) rule: u_out = |f'(x)| x u_in.
        let deg = self.mode == AngleMode::Degrees;
        let rad = if deg {
            arg.v * std::f64::consts::PI / 180.0
        } else {
            arg.v
        };
        let conv = std::f64::consts::PI / 180.0;
        let deriv = match name {
            "sqrt" => {
                if arg.v == 0.0 {
                    return Err(
                        "sqrt at 0 with an uncertainty cannot be propagated — the slope there is not defined."
                            .into(),
                    );
                }
                1.0 / (2.0 * arg.v.sqrt())
            }
            "ln" => 1.0 / arg.v.abs(),
            "log10" => 1.0 / (arg.v.abs() * std::f64::consts::LN_10),
            "exp" => arg.v.exp(),
            "sin" => rad.cos().abs() * if deg { conv } else { 1.0 },
            "cos" => rad.sin().abs() * if deg { conv } else { 1.0 },
            "tan" => (1.0 / rad.cos().powi(2)) * if deg { conv } else { 1.0 },
            "asin" | "acos" => (if deg { 1.0 / conv } else { 1.0 }) / (1.0 - arg.v.powi(2)).sqrt(),
            "atan" => (if deg { 1.0 / conv } else { 1.0 }) / (1.0 + arg.v.powi(2)),
            "abs" => 1.0,
            _ => 0.0,
        };
        let u = deriv * arg.u;
        self.steps.push(format!(
            "{name}: the uncertainty scales by the slope |{name}'(x)| = {} — {} x {} = {}.",
            crate::nd(deriv, 6),
            crate::n(arg.u),
            crate::nd(deriv, 6),
            crate::n(u)
        ));
        Ok(UVal { v, u })
    }
}

/// Evaluate an expression carrying ± uncertainties (#40). The
/// "+/-" spelling is normalized to ± first (so positions in error
/// messages count a +/- as one character). Plain sub-results are
/// exact (zero uncertainty) and propagate as such.
pub fn eval_expression_uncertain(src: &str, mode: AngleMode) -> Result<UncertainOutcome, String> {
    let normalized = src.replace("+/-", "\u{b1}");
    let toks = tokenize(&normalized)?;
    if toks.is_empty() {
        return Err("The expression is empty — type something like 10 \u{b1} 0.2 + 5.".into());
    }
    let mut p = UParser {
        toks,
        i: 0,
        mode,
        src_len: normalized.chars().count(),
        steps: Vec::new(),
    };
    let v = p.parse_expr()?;
    if let Some(t) = p.peek() {
        return Err(format!(
            "Parse error at position {}: the expression was complete, but more input follows — check for a missing operator.",
            t.pos
        ));
    }
    if !v.v.is_finite() || !v.u.is_finite() {
        return Err("That expression produced a number too big (or not a number).".into());
    }
    Ok(UncertainOutcome {
        value: v.v,
        uncertainty: v.u,
        steps: p.steps,
    })
}

// ---------- #6/#7 typed quantities + dimensional check ----------

struct Unit {
    aliases: &'static [&'static str],
    dim: &'static str,
    /// How many canonical units in one of this unit (linear dims).
    factor: f64,
}

/// Linear unit tables, by dimension; the canonical unit of each
/// dimension has factor 1. Temperature is affine and handled
/// separately.
static UNITS: &[Unit] = &[
    // length — canonical metre
    Unit {
        aliases: &[
            "mm",
            "millimetre",
            "millimetres",
            "millimeter",
            "millimeters",
        ],
        dim: "length",
        factor: 0.001,
    },
    Unit {
        aliases: &[
            "cm",
            "centimetre",
            "centimetres",
            "centimeter",
            "centimeters",
        ],
        dim: "length",
        factor: 0.01,
    },
    Unit {
        aliases: &["m", "metre", "metres", "meter", "meters"],
        dim: "length",
        factor: 1.0,
    },
    Unit {
        aliases: &["km", "kilometre", "kilometres", "kilometer", "kilometers"],
        dim: "length",
        factor: 1000.0,
    },
    Unit {
        aliases: &["in", "inch", "inches"],
        dim: "length",
        factor: 0.0254,
    },
    Unit {
        aliases: &["ft", "foot", "feet"],
        dim: "length",
        factor: 0.3048,
    },
    Unit {
        aliases: &["yd", "yard", "yards"],
        dim: "length",
        factor: 0.9144,
    },
    Unit {
        aliases: &["mi", "mile", "miles"],
        dim: "length",
        factor: 1609.344,
    },
    // mass — canonical gram
    Unit {
        aliases: &["mg", "milligram", "milligrams"],
        dim: "mass",
        factor: 0.001,
    },
    Unit {
        aliases: &["g", "gram", "grams", "gramme", "grammes"],
        dim: "mass",
        factor: 1.0,
    },
    Unit {
        aliases: &["kg", "kilogram", "kilograms"],
        dim: "mass",
        factor: 1000.0,
    },
    Unit {
        aliases: &["t", "tonne", "tonnes", "metricton", "metrictons"],
        dim: "mass",
        factor: 1.0e6,
    },
    Unit {
        aliases: &["oz", "ounce", "ounces"],
        dim: "mass",
        factor: 28.349523125,
    },
    Unit {
        aliases: &["lb", "lbs", "pound", "pounds"],
        dim: "mass",
        factor: 453.59237,
    },
    Unit {
        aliases: &["stone", "stones"],
        dim: "mass",
        factor: 6350.29318,
    },
    // time — canonical second
    Unit {
        aliases: &["ms", "millisecond", "milliseconds"],
        dim: "time",
        factor: 0.001,
    },
    Unit {
        aliases: &["s", "sec", "secs", "second", "seconds"],
        dim: "time",
        factor: 1.0,
    },
    Unit {
        aliases: &["min", "mins", "minute", "minutes"],
        dim: "time",
        factor: 60.0,
    },
    Unit {
        aliases: &["h", "hr", "hrs", "hour", "hours"],
        dim: "time",
        factor: 3600.0,
    },
    Unit {
        aliases: &["d", "day", "days"],
        dim: "time",
        factor: 86400.0,
    },
    Unit {
        aliases: &["week", "weeks"],
        dim: "time",
        factor: 604800.0,
    },
    // volume — canonical litre
    Unit {
        aliases: &[
            "ml",
            "millilitre",
            "millilitres",
            "milliliter",
            "milliliters",
        ],
        dim: "volume",
        factor: 0.001,
    },
    Unit {
        aliases: &["l", "litre", "litres", "liter", "liters"],
        dim: "volume",
        factor: 1.0,
    },
    Unit {
        aliases: &[
            "m3",
            "cubicmetre",
            "cubicmetres",
            "cubicmeter",
            "cubicmeters",
        ],
        dim: "volume",
        factor: 1000.0,
    },
    Unit {
        aliases: &["cm3", "cc", "cubiccentimetre", "cubiccentimetres"],
        dim: "volume",
        factor: 0.001,
    },
    Unit {
        aliases: &["gal", "gallon", "gallons"],
        dim: "volume",
        factor: 3.785411784,
    },
    Unit {
        aliases: &["qt", "quart", "quarts"],
        dim: "volume",
        factor: 0.946352946,
    },
    Unit {
        aliases: &["pt", "pint", "pints"],
        dim: "volume",
        factor: 0.473176473,
    },
    Unit {
        aliases: &["cup", "cups"],
        dim: "volume",
        factor: 0.2365882365,
    },
    // speed — canonical m/s
    Unit {
        aliases: &["m/s", "mps"],
        dim: "speed",
        factor: 1.0,
    },
    Unit {
        aliases: &["km/h", "kmh", "kph"],
        dim: "speed",
        factor: 1.0 / 3.6,
    },
    Unit {
        aliases: &["mph"],
        dim: "speed",
        factor: 0.44704,
    },
    Unit {
        aliases: &["knot", "knots", "kn"],
        dim: "speed",
        factor: 0.514444,
    },
    Unit {
        aliases: &["ft/s", "fps"],
        dim: "speed",
        factor: 0.3048,
    },
    // force — canonical newton
    Unit {
        aliases: &["n", "newton", "newtons"],
        dim: "force",
        factor: 1.0,
    },
    Unit {
        aliases: &["kn", "kilonewton", "kilonewtons"],
        dim: "force",
        factor: 1000.0,
    },
    Unit {
        aliases: &["lbf", "poundforce"],
        dim: "force",
        factor: 4.4482216152605,
    },
    Unit {
        aliases: &["dyn", "dyne", "dynes"],
        dim: "force",
        factor: 1.0e-5,
    },
    // area — canonical m^2
    Unit {
        aliases: &[
            "m2",
            "sqm",
            "squaremetre",
            "squaremetres",
            "squaremeter",
            "squaremeters",
        ],
        dim: "area",
        factor: 1.0,
    },
    Unit {
        aliases: &["cm2", "squarecentimetre", "squarecentimetres"],
        dim: "area",
        factor: 0.0001,
    },
    Unit {
        aliases: &["km2", "squarekilometre", "squarekilometres"],
        dim: "area",
        factor: 1.0e6,
    },
    Unit {
        aliases: &["ft2", "sqft", "squarefoot", "squarefeet"],
        dim: "area",
        factor: 0.09290304,
    },
    Unit {
        aliases: &["in2", "sqin", "squareinch", "squareinches"],
        dim: "area",
        factor: 0.00064516,
    },
    Unit {
        aliases: &["acre", "acres"],
        dim: "area",
        factor: 4046.8564224,
    },
    Unit {
        aliases: &["ha", "hectare", "hectares"],
        dim: "area",
        factor: 10000.0,
    },
    // energy — canonical joule
    Unit {
        aliases: &["j", "joule", "joules"],
        dim: "energy",
        factor: 1.0,
    },
    Unit {
        aliases: &["kj", "kilojoule", "kilojoules"],
        dim: "energy",
        factor: 1000.0,
    },
    Unit {
        aliases: &["mj", "megajoule", "megajoules"],
        dim: "energy",
        factor: 1.0e6,
    },
    Unit {
        aliases: &["kwh"],
        dim: "energy",
        factor: 3.6e6,
    },
    Unit {
        aliases: &["wh"],
        dim: "energy",
        factor: 3600.0,
    },
    Unit {
        aliases: &["cal", "calorie", "calories"],
        dim: "energy",
        factor: 4.184,
    },
    Unit {
        aliases: &["kcal", "kilocalorie", "kilocalories"],
        dim: "energy",
        factor: 4184.0,
    },
    Unit {
        aliases: &["btu"],
        dim: "energy",
        factor: 1055.05585,
    },
    // power — canonical watt
    Unit {
        aliases: &["w", "watt", "watts"],
        dim: "power",
        factor: 1.0,
    },
    Unit {
        aliases: &["kw", "kilowatt", "kilowatts"],
        dim: "power",
        factor: 1000.0,
    },
    Unit {
        aliases: &["hp", "horsepower"],
        dim: "power",
        factor: 745.699872,
    },
    // voltage — canonical volt
    Unit {
        aliases: &["mv", "millivolt", "millivolts"],
        dim: "voltage",
        factor: 0.001,
    },
    Unit {
        aliases: &["v", "volt", "volts"],
        dim: "voltage",
        factor: 1.0,
    },
    Unit {
        aliases: &["kv", "kilovolt", "kilovolts"],
        dim: "voltage",
        factor: 1000.0,
    },
    // current — canonical amp
    Unit {
        aliases: &["ma", "milliamp", "milliamps", "milliampere", "milliamperes"],
        dim: "current",
        factor: 0.001,
    },
    Unit {
        aliases: &["a", "amp", "amps", "ampere", "amperes"],
        dim: "current",
        factor: 1.0,
    },
    Unit {
        aliases: &["ka", "kiloamp", "kiloamps"],
        dim: "current",
        factor: 1000.0,
    },
    // resistance — canonical ohm
    Unit {
        aliases: &["ohm", "ohms", "\u{3a9}"],
        dim: "resistance",
        factor: 1.0,
    },
    Unit {
        aliases: &["kohm", "kilohm", "kilohms", "k\u{3a9}"],
        dim: "resistance",
        factor: 1000.0,
    },
    Unit {
        aliases: &["mohm", "megohm", "megohms"],
        dim: "resistance",
        factor: 1.0e6,
    },
    // frequency — canonical hertz
    Unit {
        aliases: &["hz", "hertz"],
        dim: "frequency",
        factor: 1.0,
    },
    Unit {
        aliases: &["khz", "kilohertz"],
        dim: "frequency",
        factor: 1000.0,
    },
    Unit {
        aliases: &["mhz", "megahertz"],
        dim: "frequency",
        factor: 1.0e6,
    },
    Unit {
        aliases: &["ghz", "gigahertz"],
        dim: "frequency",
        factor: 1.0e9,
    },
    Unit {
        aliases: &["rpm"],
        dim: "frequency",
        factor: 1.0 / 60.0,
    },
    // pressure — canonical pascal
    Unit {
        aliases: &["pa", "pascal", "pascals"],
        dim: "pressure",
        factor: 1.0,
    },
    Unit {
        aliases: &["kpa", "kilopascal", "kilopascals"],
        dim: "pressure",
        factor: 1000.0,
    },
    Unit {
        aliases: &["bar", "bars"],
        dim: "pressure",
        factor: 100000.0,
    },
    Unit {
        aliases: &["atm", "atmosphere", "atmospheres"],
        dim: "pressure",
        factor: 101325.0,
    },
    Unit {
        aliases: &["psi"],
        dim: "pressure",
        factor: 6894.757,
    },
    Unit {
        aliases: &["mmhg", "torr"],
        dim: "pressure",
        factor: 133.322,
    },
    // angle — canonical degree
    Unit {
        aliases: &["deg", "degree", "degrees", "\u{b0}"],
        dim: "angle",
        factor: 1.0,
    },
    Unit {
        aliases: &["rad", "radian", "radians"],
        dim: "angle",
        factor: 57.29577951308232,
    },
    Unit {
        aliases: &["grad", "gradian", "gradians"],
        dim: "angle",
        factor: 0.9,
    },
    Unit {
        aliases: &["turn", "turns"],
        dim: "angle",
        factor: 360.0,
    },
    // acceleration — canonical m/s^2
    Unit {
        aliases: &["m/s2"],
        dim: "acceleration",
        factor: 1.0,
    },
    // density — canonical kg/m^3
    Unit {
        aliases: &["kg/m3"],
        dim: "density",
        factor: 1.0,
    },
    Unit {
        aliases: &["g/cm3"],
        dim: "density",
        factor: 1000.0,
    },
    Unit {
        aliases: &["g/l"],
        dim: "density",
        factor: 1.0,
    },
    Unit {
        aliases: &["kg/l"],
        dim: "density",
        factor: 1000.0,
    },
    Unit {
        aliases: &["lb/ft3"],
        dim: "density",
        factor: 16.018463,
    },
];

const TEMP_UNITS: &[&str] = &["c", "f", "k", "celsius", "fahrenheit", "kelvin"];

fn find_unit(name: &str) -> Option<&'static Unit> {
    let key = name.replace(' ', "").to_ascii_lowercase();
    UNITS.iter().find(|u| {
        u.aliases
            .iter()
            .any(|a| a.replace(' ', "").eq_ignore_ascii_case(&key))
    })
}

fn is_temp_unit(name: &str) -> bool {
    TEMP_UNITS.contains(&name.replace(' ', "").to_ascii_lowercase().as_str())
}

/// Split a typed quantity into (number, unit) segments:
/// "5 ft 3 in" -> [(5, "ft"), (3, "in")].
fn quantity_segments(text: &str) -> Result<Vec<(f64, String)>, String> {
    let cleaned: String = text
        .chars()
        .map(|c| match c {
            '\u{b2}' => '2',
            '^' => '\u{0}',
            '\u{3a9}' => '\u{3a9}',
            _ => c,
        })
        .filter(|c| *c != '\u{0}')
        .collect();
    let chars: Vec<char> = cleaned.chars().collect();
    let mut segs = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        // Number (sign, digits, dot, optional exponent).
        let start = i;
        if chars[i] == '-' || chars[i] == '+' {
            i += 1;
        }
        let mut seen_digit = false;
        let mut seen_dot = false;
        while i < chars.len() {
            let c = chars[i];
            if c.is_ascii_digit() {
                seen_digit = true;
                i += 1;
            } else if c == '.' && !seen_dot {
                seen_dot = true;
                i += 1;
            } else {
                break;
            }
        }
        if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') && seen_digit {
            let mut k = i + 1;
            if k < chars.len() && (chars[k] == '-' || chars[k] == '+') {
                k += 1;
            }
            if k < chars.len() && chars[k].is_ascii_digit() {
                i = k;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
        }
        if !seen_digit {
            return Err(format!(
                "'{text}' is not a quantity — expected a number followed by a unit, like '5 ft 3 in'."
            ));
        }
        let num_text: String = chars[start..i].iter().collect();
        let value: f64 = num_text
            .parse()
            .map_err(|_| format!("'{num_text}' is not a number."))?;
        // Unit: letters, '/', digits (m/s2), '%', degree/ohm symbols.
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let ustart = i;
        while i < chars.len()
            && (chars[i].is_alphabetic()
                || chars[i] == '/'
                || chars[i].is_ascii_digit()
                || chars[i] == '%'
                || chars[i] == '\u{b0}'
                || chars[i] == '\u{3a9}')
        {
            i += 1;
        }
        let unit: String = chars[ustart..i].iter().collect();
        if unit.is_empty() {
            return Err(format!(
                "'{text}' has a number with no unit — add one, like 'kg', 'ft' or 'h'."
            ));
        }
        segs.push((value, unit));
    }
    if segs.is_empty() {
        return Err("Enter a quantity like '5 ft 3 in' or '1.2 kg'.".into());
    }
    Ok(segs)
}

/// Which dimension a typed quantity belongs to, from its first
/// unit — or None for a plain number / unrecognised text (#7).
pub fn detect_quantity_dimension(text: &str) -> Option<&'static str> {
    if text.trim().parse::<f64>().is_ok() {
        return None;
    }
    let segs = quantity_segments(text).ok()?;
    let unit = &segs.first()?.1;
    if is_temp_unit(unit) {
        return Some("temperature");
    }
    find_unit(unit).map(|u| u.dim)
}

/// Dimensions that may be written as a sum of segments
/// ("5 ft 3 in", "2 h 30 min", "3 lb 4 oz").
fn compoundable(dim: &str) -> bool {
    matches!(dim, "length" | "mass" | "time" | "volume" | "area")
}

/// Parse a typed quantity in dimension `family` to its canonical
/// unit (length m, mass g, temperature C, time s, volume L, speed
/// m/s, and the factor-1 unit of the other tables) (#6).
pub fn parse_quantity(text: &str, family: &str) -> Result<f64, String> {
    let segs = quantity_segments(text)?;
    if family == "temperature" {
        if segs.len() != 1 {
            return Err(
                "A temperature is a single value — compound temperatures like '70 F 2 C' do not add up."
                    .into(),
            );
        }
        let (v, unit) = &segs[0];
        let u = unit.replace(' ', "").to_ascii_lowercase();
        return match u.as_str() {
            "c" | "celsius" => Ok(*v),
            "f" | "fahrenheit" => Ok((v - 32.0) * 5.0 / 9.0),
            "k" | "kelvin" => Ok(v - 273.15),
            _ => Err(format!(
                "'{unit}' is not a temperature unit — use C, F or K."
            )),
        };
    }
    let mut total = 0.0;
    for (v, unit) in &segs {
        let Some(u) = find_unit(unit) else {
            return Err(format!(
                "'{unit}' is not a unit I know — check the spelling (e.g. kg, ft, in, h, min, L)."
            ));
        };
        if u.dim != family {
            return Err(format!(
                "'{unit}' is a {} unit, but this field expects {} — the value looks like a {}.",
                u.dim, family, u.dim
            ));
        }
        total += v * u.factor;
    }
    if segs.len() > 1 && !compoundable(family) {
        return Err(format!(
            "A {family} value is a single quantity — compound entries only add up for length, mass, time, volume and area."
        ));
    }
    Ok(total)
}

/// Parse a typed quantity and convert it INTO `expected_unit`
/// (which must belong to the same dimension) (#6).
pub fn parse_quantity_to(text: &str, family: &str, expected_unit: &str) -> Result<f64, String> {
    let canon = parse_quantity(text, family)?;
    if family == "temperature" {
        let u = expected_unit.replace(' ', "").to_ascii_lowercase();
        return match u.as_str() {
            "c" | "celsius" => Ok(canon),
            "f" | "fahrenheit" => Ok(canon * 9.0 / 5.0 + 32.0),
            "k" | "kelvin" => Ok(canon + 273.15),
            _ => Err(format!("'{expected_unit}' is not a temperature unit.")),
        };
    }
    let Some(u) = find_unit(expected_unit) else {
        return Err(format!("'{expected_unit}' is not a known {family} unit."));
    };
    if u.dim != family {
        return Err(format!(
            "'{expected_unit}' is a {} unit, not {}.",
            u.dim, family
        ));
    }
    Ok(canon / u.factor)
}

/// The plain-words warning for a dimension mismatch (#7), or None
/// when the typed value's dimension matches (or is unknowable —
/// plain numbers never warn).
pub fn dimension_warning(calc: &Calc, field: usize, raw: &str) -> Option<String> {
    let expected = calc.dimensions.get(field).copied().unwrap_or("");
    if expected.is_empty() {
        return None;
    }
    let got = detect_quantity_dimension(raw)?;
    if got == expected {
        return None;
    }
    Some(format!(
        "Warning: this field expects a {expected}, but '{raw}' looks like a {got} value. Check the unit — the calculation was not run.",
        raw = raw.trim()
    ))
}

/// Parse one form field's raw text into its number, honouring
/// typed quantities (#6) and the dimensional check (#7).
/// `siblings` are the raw texts of all the calculator's fields in
/// order (a converter's value field reads its FROM-unit sibling to
/// know which unit the typed quantity must be expressed in).
pub fn parse_field_value(
    calc: &Calc,
    field: usize,
    raw: &str,
    siblings: &[String],
) -> Result<f64, String> {
    let t = raw.trim();
    if let Ok(v) = t.parse::<f64>() {
        return Ok(v);
    }
    if let Some(w) = dimension_warning(calc, field, raw) {
        return Err(w);
    }
    let family = calc.unit_families.get(field).copied().unwrap_or("");
    if family.is_empty() {
        return Err(format!("'{t}' is not a number."));
    }
    let expected: String = if !calc.converter_units.is_empty() && field == 0 {
        // Converter value field: convert into the chosen FROM unit.
        let code = siblings
            .get(1)
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);
        calc.converter_units
            .get(code)
            .copied()
            .unwrap_or("")
            .to_string()
    } else {
        calc.quantity_units
            .get(field)
            .copied()
            .unwrap_or("")
            .to_string()
    };
    if expected.is_empty() {
        // No named expected unit: use the dimension's canonical unit.
        return parse_quantity(t, family);
    }
    parse_quantity_to(t, family, &expected)
}

// ---------- built-in help (standing rule) ----------

/// The built-in guide, compiled into the program (works offline).
/// The window's HELP panel and the CLI's `--help-guide` print this
/// same text.
pub const HELP_TEXT: &str = "CalcDex — built-in help\n\nWHAT IT IS\nCalcDex is one offline program holding 300+ calculators — money,\nhealth, school maths, physics, chemistry, building, cooking,\nsports and more. Every result shows its formula and the working,\nso you can check it, not just trust it.\n\nTHE FIVE THINGS PEOPLE DO MOST\n1. Pick a calculator: click a mode in the left rail (MATH,\n   FINANCE, ...), then click the calculator's name.\n2. Type the values in the labelled boxes. Hints under each box\n   say the format and unit. Some boxes accept typed quantities\n   like 5 ft 3 in or 1.2 kg — they are converted for you.\n3. Press CALCULATE (or Enter). The answer lands in the display;\n   the full working sits in the steps block underneath.\n4. Re-do an old calculation: every run is saved in the history\n   rail on the right. RE-OPEN restores its calculator + inputs.\n5. Search instead of browsing: Ctrl+K, then type what you mean —\n   everyday words work ('home loan' finds the mortgage one).\n\nKEYBOARD SHORTCUTS\nCtrl+K — jump to Search.  / — also jumps to Search.\nAlt+1..9, Alt+0, Alt+Q/W/E/R/T — switch mode (printed on the rail).\nAlt+F — Favourites mode.  Enter — Calculate.  F1 — this help.\n\nDISPLAY FORMAT + ROUNDING\nThe FORMAT selector in the header re-shows the headline rounded:\nsignificant figures, decimal places, engineering notation\n(1.63 k), or Auto (the precision of the numbers you typed).\nRounding (header row in the display area) chooses how halves\nround. The stored result always keeps full precision — rounding\nis display only. COPY buttons under the display copy the result\nas plain text, LaTeX, scientific notation, or CSV. SPEC CHECK\ncompares a result against your own min/max limits (PASS/FAIL).\n\nLEARNING TOOLS\nFORMULA & SYMBOLS under each form shows the calculator's\nformula and what every symbol means. The STEPS header selector's\nDeeper mode shows four stages: the formula, the formula with\nyour values in it, the working, and the result. CHECK MY ANSWER\nlets you type your own answer and be checked (fraction answers\nexactly, others within 0.5%), with a likely slip named when a\nwrong answer matches a common one. The Exact display format\nlabels fraction results exact, adds exact forms like 25*pi for\npi geometry, and works rational expressions out exactly.\nPRACTICE (header) generates problems with hidden answers from a\nseed — the same seed always makes the same problems. The\nExpression Calculator has a PLOT of its expression in x, and\nEXPORT SOLUTION (under the steps) saves a worked solution as a\ntext file.\n\nWHERE YOUR FILES LIVE\nSettings: data\\settings.txt in the data folder beside\nCalcDex.exe (favourites, recent, remembered inputs, display\nchoices); a settings file in the old %APPDATA%\\CalcDex folder\nis copied across on first run, the original left untouched.\nCalculation log: data\\history.txt beside the program (newest 500).\nCLEAR LOG first saves the log to history-backup.txt in the same\nfolder — RESTORE CLEARED LOG puts it back.\n\nTHE COMMAND LINE IN 3 LINES\nCalcDex --list lists every calculator; CalcDex <id> <numbers>\nruns one (e.g. CalcDex bmi 70 175); CalcDex expr \"2*(3+4)^2\"\nevaluates an expression. Everything in the window works there.\n\nUPDATES\nThe HELP panel's 'Check for updates' button asks github.com for\nthe newest CalcDex release and tells you if one exists. It never\ndownloads or installs anything — you choose. CalcDex makes no\nnetwork connection at all unless you press that button or switch\non 'Check for updates at startup' in the HELP panel.\n\nFOR TEACHERS\nPROJECTION in the header switches the window to larger text\n(about 1.5x) and higher contrast for classroom projection; it\nis saved in the settings file (CalcDex --projection on|off).\nDEMO hides a run's result values — inputs and working structure\nstay visible — until REVEAL is pressed for that result; the\ncalculation log still stores the real results. On the command\nline: CalcDex <id> <numbers> --demo (add --reveal to show).\nThe SETS view builds problem sets: add calculators with their\nvalues, save the set as a plain-text file (problems only — the\nsame file is the student assignment), and export a student\nworksheet (working space, no answers) and a separate teacher\nanswer key. Sets can be generated from a seed — the whole class\ngets identical problems — and a class answers CSV can be marked\nagainst a set. Formula sheets (formula + symbol meanings per\ncategory) and a saved class examples library live there too.\nCommand line: CalcDex set new|add|key|worksheet|generate|open,\nCalcDex --mark <set> <answers.csv>, CalcDex --formula-sheet\n<Category|All> <file>, CalcDex --save-example \"<title>\" with a\nrun, CalcDex --examples, CalcDex --open-example \"<title>\".\nCourse packs in the rail (GCSE, A-Level, AP, Undergrad) filter\nthe calculator strip to a course level; CalcDex --pack GCSE\n--list prints a pack on the command line.\\n\\nFOR SCIENTISTS\\nList fields accept a pasted column: values separated by new\nlines, commas, semicolons, tabs or spaces all parse the same\n(so does the command line). Replicates Summary gives a set of\nrepeated measurements its mean, sample SD, SEM and 95%\nconfidence interval (Student's t) in one step; Outlier Check\nflags values outside the 1.5 x IQR fences; Serial Dilution\nplans a dilution series in Chemistry.\\nThe CONSTANTS view (header) lists the CODATA 2018 constants\n— c, h, k_b, n_a and the rest — and INSERT puts one into the\nExpression Calculator. The same names work typed: h*c/500e-9\nis the energy of a 500 nm photon. Define your own with\nCalcDex --define name=value (or in the CONSTANTS view); they\nare saved in the settings file and can never replace a\nbuilt-in. A measurement written 10 \\u{b1} 0.2 (or 10 +/- 0.2) in\nan expression carries its uncertainty through + - * / ^ and\nthe functions, with the propagation rule shown for each step.\\nThe log is also a lab notebook: tag or annotate any entry\n(the TAG / NOTE button in the log view, or CalcDex --log-tag\nand --log-note with the entry's number from --log), and\nexport the whole log as a plain-text lab record with\nCalcDex --lab-export <file>. Editing an entry first saves a\nbackup copy (history-edit-backup.txt) beside the log.\nThe log also takes polish: --log-search <text> finds entries,\n--log-filter calc=<id> / tag=<tag> / date=<YYYY-MM-DD> /\npinned=yes narrows the list (the window's log view has the\nsame boxes), --log-pin <n> pins an entry so the 500-entry\ntrim never drops it, and --log-export <file> writes the\nfiltered set out as plain text.\n\nFOR ENGINEERS\nPhysics holds an electronics pack: the Resistor Colour Code\n(type the band colours, or a value like 4.7k to get its\nbands), Resistors in Series & Parallel, and the Voltage\nDivider. Construction holds the mechanical pack: Tolerance\nStack-Up (worst case and RSS), Safety Factor & Margin of\nSafety (an FoS below 1 is called a predicted failure, in\nplain words), Bolt Torque (T = K x D x F) and Beam — Stress\n& Deflection for a simply supported beam with a central\nload. The MATERIALS view (header) is the built-in table of\ntypical density, resistivity and Young's modulus values;\nCalcDex --materials prints it too, and the beam calculator\ntakes a material's code when its E field is 0 — or the\nview's USE E IN BEAM button fills E in directly.";

/// Compare two version strings ("v0.4.0" vs "0.3.9"): numeric,
/// component by component; missing components count as 0.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.trim()
            .trim_start_matches(['v', 'V'])
            .split('.')
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (va, vb) = (parse(a), parse(b));
    for i in 0..va.len().max(vb.len()) {
        let (x, y) = (
            va.get(i).copied().unwrap_or(0),
            vb.get(i).copied().unwrap_or(0),
        );
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Pull the `tag_name` out of a GitHub release JSON reply.
pub fn latest_tag_from_json(json: &str) -> Option<String> {
    let key = "\"tag_name\"";
    let at = json.find(key)? + key.len();
    let rest = &json[at..];
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_string())
}

/// Offer-only update check: ask the GitHub releases endpoint for
/// the newest CalcDex release and report the verdict in plain
/// words. Never downloads or installs. Uses the system `curl`
/// (shipped with Windows 10/11) with a short timeout, so a network
/// failure is a message, never a hang or a crash.
pub fn check_for_updates() -> String {
    let out = std::process::Command::new("curl")
        .args([
            "-fsS",
            "--max-time",
            "6",
            "-A",
            "CalcDex",
            "https://api.github.com/repos/SeanShaffer88/CalcDex/releases/latest",
        ])
        .output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            return format!(
                "Could not check for updates just now (the update server replied with an error{}). Nothing was downloaded. You can check yourself at https://github.com/SeanShaffer88/CalcDex/releases",
                if o.stderr.is_empty() {
                    String::new()
                } else {
                    format!(": {}", String::from_utf8_lossy(&o.stderr).trim())
                }
            )
        }
        Err(e) => {
            return format!(
                "Could not check for updates ({e}). Nothing was downloaded. You can check yourself at https://github.com/SeanShaffer88/CalcDex/releases"
            )
        }
    };
    let json = String::from_utf8_lossy(&out.stdout);
    let Some(tag) = latest_tag_from_json(&json) else {
        return "Could not read the update server's reply. Nothing was downloaded.".to_string();
    };
    let current = env!("CARGO_PKG_VERSION");
    match compare_versions(&tag, current) {
        std::cmp::Ordering::Greater => format!(
            "A newer CalcDex is available: {tag} (you have v{current}). Release notes + downloads: https://github.com/SeanShaffer88/CalcDex/releases/latest — nothing has been downloaded or installed; the choice is yours."
        ),
        _ => format!(
            "You have the newest CalcDex (v{current}; the latest release is {tag})."
        ),
    }
}
