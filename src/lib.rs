//! CalcDex core library.
//!
//! Every calculator is a pure operation `(numbers) -> Result<text>` held in
//! ONE registry. The GUI, the CLI and the tests all dispatch through this
//! same registry — there are no GUI-only calculators.
//!
//! Signature notes:
//! - Fixed-input calculators declare labelled inputs; the slice holds
//!   exactly one number per label, in order.
//! - List calculators (`list = true`) accept any number of values. In the
//!   GUI they are typed into one field separated by commas/spaces; on the
//!   CLI each value is its own argument.
//! - Pair-list calculators (`pairs = true`) take values in repeating pairs
//!   (documented per calculator, e.g. value,probability,value,probability).

// Guard-clause validation style (consecutive `if { return Err }`) is used
// throughout the calculator modules on purpose.
#![allow(clippy::possible_missing_else)]

pub mod calcs;
pub mod engineers;
pub mod features;
pub mod meta;
pub mod paths;
pub mod rigour;
pub mod settings;
pub mod students;
pub mod teachers;

/// Below this window width the history rail collapses: the header
/// LOG toggle then switches the centre to the log view. The width is
/// measured on the physical screen (see `history_rail_visible`).
pub const HISTORY_RAIL_MIN_WIDTH: f32 = 1100.0;

/// Whether the history rail fits beside the mode rail and the centre.
/// `content_width` is the window's layout width in points and
/// `zoom_factor` the UI zoom (1.5 in classroom projection mode). The
/// product is the physical width, so turning projection on — which
/// enlarges everything — never makes the rail vanish from a window
/// that showed it at normal size: the panes give way instead.
pub fn history_rail_visible(content_width: f32, zoom_factor: f32) -> bool {
    content_width * zoom_factor >= HISTORY_RAIL_MIN_WIDTH
}

pub type CalcFn = Box<dyn Fn(&[f64]) -> Result<String, String> + Sync + Send>;
/// A text-input calculator (v0.4.0 expression calculator): the second
/// argument is the angle mode, true = degrees, false = radians.
pub type CalcTextFn = Box<dyn Fn(&str, bool) -> Result<String, String> + Sync + Send>;

pub struct Calc {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    /// Plain-words description including the standard formula used.
    pub desc: &'static str,
    /// Labelled inputs, in order (for list calculators: one label
    /// describing the list).
    pub inputs: &'static [&'static str],
    pub list: bool,
    pub pairs: bool,
    /// Date groups: input triples (year, month, day starting at `start`)
    /// that the GUI/CLI treat as ONE date field labelled `label`, entered
    /// as MM/DD/YYYY. Empty for non-date calculators.
    pub date_groups: &'static [DateGroup],
    /// Teaching example inputs (item #11), filled from
    /// `meta::EXAMPLES` when the registry is built.
    pub example: &'static [f64],
    /// Optional hand-written per-field hints (item #20), filled from
    /// `meta::HINTS`; `meta::hint_for` falls back to a generated hint
    /// for any field without one, so every field has a hint.
    pub hints: &'static [&'static str],
    /// Pair calculators whose list may end with ONE extra single
    /// value (linear regression's calibration measurement).
    pub tail: bool,
    /// Unit family per input (v0.4.0 #6/#7), "" = none known.
    pub unit_families: &'static [&'static str],
    /// Expected unit code per input for typed quantities ("kg", "cm"...).
    pub quantity_units: &'static [&'static str],
    /// Expected dimension name per input for the dimensional check.
    pub dimensions: &'static [&'static str],
    /// Unit names for parametric converters (value-field typed input).
    pub converter_units: &'static [&'static str],
    /// Text-input calculator support (#5): the run function takes the
    /// raw text instead of numbers.
    pub text_run: Option<CalcTextFn>,
    pub example_text: &'static str,
    pub run: CalcFn,
}

/// One merged date field: the inputs at `start..start+3` are
/// (year, month, day) for a date the user enters as a single MM/DD/YYYY
/// value under the display `label` (e.g. "Birth Date").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DateGroup {
    pub start: usize,
    pub label: &'static str,
}

pub const fn dg(start: usize, label: &'static str) -> DateGroup {
    DateGroup { start, label }
}

impl Calc {
    #[allow(clippy::too_many_arguments)]
    pub fn new<F>(
        id: &'static str,
        name: &'static str,
        category: &'static str,
        desc: &'static str,
        inputs: &'static [&'static str],
        run: F,
    ) -> Calc
    where
        F: Fn(&[f64]) -> Result<String, String> + Sync + Send + 'static,
    {
        Calc {
            id,
            name,
            category,
            desc,
            inputs,
            list: false,
            pairs: false,
            date_groups: &[],
            example: &[],
            hints: &[],
            tail: false,
            unit_families: &[],
            quantity_units: &[],
            dimensions: &[],
            converter_units: &[],
            text_run: None,
            example_text: "",
            run: Box::new(run),
        }
    }
    pub fn with_dates(mut self, groups: &'static [DateGroup]) -> Calc {
        self.date_groups = groups;
        self
    }
    pub fn list_mode(mut self) -> Calc {
        self.list = true;
        self
    }
    pub fn pair_mode(mut self) -> Calc {
        self.list = true;
        self.pairs = true;
        self
    }
    /// Pairs plus one optional trailing single value.
    pub fn pair_tail_mode(mut self) -> Calc {
        self.list = true;
        self.pairs = true;
        self.tail = true;
        self
    }
    pub fn with_example(mut self, example: &'static [f64]) -> Calc {
        self.example = example;
        self
    }
    pub fn with_hints(mut self, hints: &'static [&'static str]) -> Calc {
        self.hints = hints;
        self
    }
    /// Attach typed-quantity metadata (#6/#7): one family, expected
    /// unit and dimension per input ("" where none is known).
    pub fn with_unit_info(
        mut self,
        families: &'static [&'static str],
        units: &'static [&'static str],
        dimensions: &'static [&'static str],
    ) -> Calc {
        self.unit_families = families;
        self.quantity_units = units;
        self.dimensions = dimensions;
        self
    }
    pub fn with_converter_units(mut self, units: &'static [&'static str]) -> Calc {
        self.converter_units = units;
        self
    }
    /// Make this a text-input calculator (#5).
    pub fn text_mode<F>(mut self, example_text: &'static str, run: F) -> Calc
    where
        F: Fn(&str, bool) -> Result<String, String> + Sync + Send + 'static,
    {
        self.example_text = example_text;
        self.text_run = Some(Box::new(run));
        self
    }
}

/// Build the full registry (all category modules register here).
pub fn all_calcs() -> Vec<Calc> {
    let mut v: Vec<Calc> = Vec::new();
    calcs::math::register(&mut v);
    calcs::finance::register(&mut v);
    calcs::health::register(&mut v);
    calcs::physics::register(&mut v);
    calcs::chemistry::register(&mut v);
    calcs::construction::register(&mut v);
    calcs::conversion::register(&mut v);
    calcs::stats::register(&mut v);
    calcs::everyday::register(&mut v);
    calcs::sports_food_eco::register(&mut v);
    calcs::engineering::register(&mut v);
    // v0.3.0: attach the central metadata tables (examples + hints)
    // to the registry entries, so they are part of each Calc spec.
    for c in &mut v {
        for (id, ex) in meta::EXAMPLES {
            if *id == c.id {
                c.example = ex;
            }
        }
        for (id, hints) in meta::HINTS {
            if *id == c.id {
                c.hints = hints;
            }
        }
        for (id, families, units, dims) in meta::UNIT_INFO {
            if *id == c.id {
                c.unit_families = families;
                c.quantity_units = units;
                c.dimensions = dims;
            }
        }
    }
    v
}

pub fn find_calc(id: &str) -> Option<Calc> {
    all_calcs().into_iter().find(|c| c.id == id)
}

/// Run a calculator by id against already-parsed numbers.
pub fn run_calc(id: &str, args: &[f64]) -> Result<String, String> {
    match find_calc(id) {
        Some(c) => {
            if c.text_run.is_some() {
                return Err(
                    "This calculator takes text (an expression), not numbers — type the expression in its field, or use: CalcDex expr \"<expression>\"."
                        .into(),
                );
            }
            if !c.list && args.len() != c.inputs.len() {
                return Err(format!(
                    "'{}' needs {} number(s) ({}), got {}.",
                    c.name,
                    c.inputs.len(),
                    c.inputs.join(", "),
                    args.len()
                ));
            }
            if c.list && args.is_empty() {
                return Err("Enter at least one number.".into());
            }
            if c.pairs && !c.tail && args.len() % 2 != 0 {
                return Err(
                    "This calculator takes values in pairs — you gave an odd count of numbers."
                        .into(),
                );
            }
            (c.run)(args)
        }
        None => Err(format!("No calculator called '{id}'. Try: CalcDex --list")),
    }
}

/// Parse a pasted or typed list of numbers (v0.7.0, item #32):
/// the ONE parser behind every list field — the window's list box
/// and the command line both call it, so the same text parses
/// identically on both. Values may be separated by commas,
/// semicolons, tabs, spaces or new lines (a pasted spreadsheet
/// column is one value per line), in any mixture; empty stretches
/// between separators are skipped. A token that is not a number
/// is named in the error, with the accepted shapes restated.
pub fn parse_value_list(text: &str) -> Result<Vec<f64>, String> {
    let mut out = Vec::new();
    for tok in text.split(|ch: char| {
        ch == ',' || ch == ';' || ch == '\t' || ch == '\n' || ch == '\r' || ch == ' '
    }) {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        match tok.parse::<f64>() {
            Ok(v) => out.push(v),
            Err(_) => {
                return Err(format!(
                    "'{tok}' is not a number — a value list is numbers separated by commas, spaces, tabs, semicolons or new lines (a pasted column works as-is)."
                ))
            }
        }
    }
    if out.is_empty() {
        return Err(
            "No numbers found — type or paste at least one number (values may be separated by commas, spaces, tabs, semicolons or new lines)."
                .into(),
        );
    }
    Ok(out)
}

/// Run a text-input calculator by id (#5). `degrees` selects the
/// angle mode for trigonometric functions.
pub fn run_text_calc(id: &str, text: &str, degrees: bool) -> Result<String, String> {
    match find_calc(id) {
        Some(c) => match &c.text_run {
            Some(f) => f(text, degrees),
            None => Err(format!("'{}' is not a text calculator.", c.name)),
        },
        None => Err(format!("No calculator called '{id}'. Try: CalcDex --list")),
    }
}

// ---------- shared helpers for calculator modules ----------

/// Get argument i (registry guarantees count for fixed calculators).
pub fn a(x: &[f64], i: usize) -> f64 {
    x.get(i).copied().unwrap_or(0.0)
}

pub fn chk(cond: bool, msg: &str) -> Result<(), String> {
    if cond {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

pub fn nz(v: f64, what: &str) -> Result<f64, String> {
    if v == 0.0 {
        Err(format!(
            "{what} cannot be zero (that would divide by zero)."
        ))
    } else {
        Ok(v)
    }
}

pub fn finite(v: f64) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(
            "That calculation produced a number too big (or not a number). Check the inputs."
                .into(),
        )
    }
}

/// Money format, 2 decimal places.
pub fn m(x: f64) -> String {
    format!("{x:.2}")
}

/// General number format: integers without decimals, otherwise up to
/// 10 decimal places, trailing zeros trimmed. Full precision is kept —
/// nothing is rounded away silently for the academic/professional
/// audience; calculators that present money or estimates say so.
pub fn n(x: f64) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    if x == x.trunc() && x.abs() < 1e15 {
        return format!("{}", x as i64);
    }
    let s = format!("{x:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// Scientific notation, 6 decimal places (e.g. 6.022141e23).
pub fn sci(x: f64) -> String {
    format!("{x:.6e}")
}

/// `n` with a scientific-notation fallback for the magnitudes
/// where 10 decimal places would print a nonzero value as 0
/// (or lose it entirely): |x| < 1e-6 or >= 1e15 switches to a
/// trimmed scientific form (3.972891714e-19). Used where a result
/// can genuinely be that small or large — the Expression
/// Calculator above all (photon energies, particle masses).
pub fn n_full(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return n(x);
    }
    let ax = x.abs();
    if ax < 1e-6 || ax >= 1e15 {
        let s = format!("{x:.12e}");
        let (mant, exp) = s.split_once('e').unwrap_or((&s, ""));
        let mant = mant.trim_end_matches('0').trim_end_matches('.');
        return format!("{mant}e{exp}");
    }
    n(x)
}

/// n with a fixed number of decimals, trimmed.
pub fn nd(x: f64, dp: usize) -> String {
    let s = format!("{x:.dp$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

// ---------- dates (Howard Hinnant's days-from-civil algorithm) ----------

/// Days since 1970-01-01 for a Gregorian (year, month, day).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of days_from_civil: (year, month, day).
pub fn civil_from_days(z0: i64) -> (i64, i64, i64) {
    let z = z0 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn valid_date(y: f64, m: f64, d: f64) -> Result<(i64, i64, i64), String> {
    let (y, m, d) = (y as i64, m as i64, d as i64);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1 {
        return Err("Enter a real date: year, month 1-12, day 1-31.".into());
    }
    // Round-trip check catches e.g. Feb 30.
    let back = civil_from_days(days_from_civil(y, m, d));
    if back != (y, m, d) {
        return Err("That is not a real calendar date (check the day of the month).".into());
    }
    Ok((y, m, d))
}

// ---------- date strings (MM/DD/YYYY), system-clock today, CLI expansion ----------

/// The one format hint shown under every date field, in plain words.
pub const DATE_HINT: &str =
    "Format: MM/DD/YYYY \u{2014} for example 04/23/1988 means April 23, 1988.";

/// Format a (year, month, day) as MM/DD/YYYY.
pub fn format_date(y: i64, m: i64, d: i64) -> String {
    format!("{m:02}/{d:02}/{y:04}")
}

/// Parse a user-typed date in MM/DD/YYYY form (M/D/YYYY without leading
/// zeros is also accepted). Errors teach what is wrong, per the audience
/// (students/teachers checking their work).
pub fn parse_date_str(raw: &str) -> Result<(i64, i64, i64), String> {
    let t = raw.trim();
    let parts: Vec<&str> = t.split('/').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.trim().is_empty()) {
        return Err(format!(
            "'{t}' is not in the date format this field needs. {DATE_HINT}"
        ));
    }
    let nums: Result<Vec<i64>, _> = parts.iter().map(|p| p.trim().parse::<i64>()).collect();
    let nums = match nums {
        Ok(v) => v,
        Err(_) => {
            return Err(format!(
                "'{t}' is not a real date \u{2014} a date is three numbers separated by slashes. {DATE_HINT}"
            ))
        }
    };
    let (m, d, y) = (nums[0], nums[1], nums[2]);
    if !(1..=12).contains(&m) {
        return Err(format!(
            "'{t}' is not a real date \u{2014} the month must be 1 to 12, but the first number is {m}. {DATE_HINT}"
        ));
    }
    if !(1..=31).contains(&d) {
        return Err(format!(
            "'{t}' is not a real date \u{2014} the day must be 1 to 31, but the second number is {d}. {DATE_HINT}"
        ));
    }
    if y < 1 {
        return Err(format!(
            "'{t}' is not a real date \u{2014} the year must be 1 or later. {DATE_HINT}"
        ));
    }
    // Round-trip check catches e.g. Feb 30 and Feb 29 in a non-leap year.
    if civil_from_days(days_from_civil(y, m, d)) != (y, m, d) {
        return Err(format!(
            "'{t}' is not a real date \u{2014} that month does not have a day {d} in year {y} (watch February: it has 29 days only in a leap year)."
        ));
    }
    Ok((y, m, d))
}

/// Days since the Unix epoch from the system clock (UTC day boundary).
pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86_400)
}

/// Today's date as (year, month, day), from the system clock.
pub fn today_ymd() -> (i64, i64, i64) {
    civil_from_days(today_days())
}

/// Today's date in MM/DD/YYYY form.
pub fn today_str() -> String {
    let (y, m, d) = today_ymd();
    format_date(y, m, d)
}

/// Current timestamp "YYYY-MM-DD HH:MM" from the system clock (UTC;
/// no timezone database is bundled, so log times are UTC).
pub fn now_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        sod / 3600,
        (sod % 3600) / 60
    )
}

/// If input index `i` starts a date group in this calculator, return it.
pub fn date_group_at(c: &Calc, i: usize) -> Option<&DateGroup> {
    c.date_groups.iter().find(|g| g.start == i)
}

/// How the inputs of a run are shown to the user (and stored in the log):
/// date triples collapse to their MM/DD/YYYY form, other numbers are
/// shown one per token. Returns one display token per field.
pub fn display_tokens(c: &Calc, args: &[f64]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if c.list {
        return args.iter().map(|v| n(*v)).collect();
    }
    let mut i = 0usize;
    while i < args.len() {
        if let Some(_g) = date_group_at(c, i) {
            if i + 2 < args.len() {
                out.push(format_date(
                    args[i] as i64,
                    args[i + 1] as i64,
                    args[i + 2] as i64,
                ));
                i += 3;
                continue;
            }
        }
        out.push(n(args[i]));
        i += 1;
    }
    out
}

/// Expand CLI-style string arguments into the registry's numeric input
/// vector for one calculator. A date group accepts EITHER one argument
/// in MM/DD/YYYY form OR the old three separate numbers, so
/// `CalcDex age 04/23/1988 10/25/2026` and
/// `CalcDex age 1988 4 23 2026 10 25` are equivalent. Non-date tokens
/// may also be comma-separated groups (previous behaviour).
pub fn expand_args(c: &Calc, toks: &[String]) -> Result<Vec<f64>, String> {
    // List calculators (v0.7.0 #32): every argument goes through
    // the shared value-list parser, so a pasted column (one value
    // per line, in quotes) parses exactly like separate arguments,
    // and the window's list box accepts the identical text.
    if c.list {
        let mut nums = Vec::new();
        for t in toks {
            nums.extend(parse_value_list(t)?);
        }
        if nums.is_empty() {
            return Err("Enter at least one number.".into());
        }
        return Ok(nums);
    }
    // Flatten: a token containing '/' is kept whole (it is a date);
    // other tokens are split on commas like before.
    let mut flat: Vec<String> = Vec::new();
    for tok in toks {
        if tok.contains('/') {
            flat.push(tok.clone());
        } else {
            for part in tok.split(',') {
                if !part.is_empty() {
                    flat.push(part.to_string());
                }
            }
        }
    }
    if c.date_groups.is_empty() {
        let mut nums = Vec::new();
        for (idx, t) in flat.iter().enumerate() {
            // v0.4.0 #6/#7: typed quantities ("5 ft 3 in",
            // "1.2 kg") are honoured where a unit family is
            // known for the field; plain numbers are untouched.
            nums.push(rigour::parse_field_value(c, idx, t, &flat)?);
        }
        return Ok(nums);
    }
    let mut nums: Vec<f64> = Vec::new();
    let mut pos = 0usize; // position in flat
    let mut i = 0usize; // input index
    while i < c.inputs.len() {
        if date_group_at(c, i).is_some() {
            let Some(tok) = flat.get(pos) else {
                return Err(format!(
                    "'{}' needs a {} date ({}).",
                    c.name,
                    date_group_at(c, i).map(|g| g.label).unwrap_or("date"),
                    DATE_HINT
                ));
            };
            if tok.contains('/') {
                let (y, m, d) = parse_date_str(tok)?;
                nums.push(y as f64);
                nums.push(m as f64);
                nums.push(d as f64);
                pos += 1;
            } else {
                // Old three-number form.
                for k in 0..3 {
                    let Some(t) = flat.get(pos + k) else {
                        return Err(format!("'{}' needs more numbers.", c.name));
                    };
                    match t.trim().parse::<f64>() {
                        Ok(v) => nums.push(v),
                        Err(_) => return Err(format!("'{t}' is not a number.")),
                    }
                }
                pos += 3;
            }
            i += 3;
        } else {
            let Some(tok) = flat.get(pos) else {
                return Err(format!("'{}' needs more numbers.", c.name));
            };
            nums.push(rigour::parse_field_value(c, i, tok, &flat)?);
            pos += 1;
            i += 1;
        }
    }
    if pos < flat.len() {
        return Err(format!(
            "'{}' got extra input starting at '{}'.",
            c.name, flat[pos]
        ));
    }
    Ok(nums)
}

// ---------- calculation log (persistent history, GUI + CLI shared) ----------

/// One saved calculation. `inputs` is the human-readable form (dates as
/// MM/DD/YYYY), `restore` holds one entered token per field separated by
/// ";;" so the GUI can refill the calculator exactly as it was run.
/// `tag` and `note` are the lab-notebook annotation (v0.7.0, item
/// #39); both are empty for ordinary entries. `pinned` is the
/// log-polish flag (v0.8.0, item #49): a pinned entry is never
/// dropped by the LOG_CAP trim (the cap counts unpinned entries).
#[derive(Clone, PartialEq, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub id: String,
    pub name: String,
    pub inputs: String,
    pub result: String,
    pub restore: String,
    pub tag: String,
    pub note: String,
    pub pinned: bool,
}

impl LogEntry {
    /// An entry with no lab-notebook annotation, not pinned.
    pub fn plain(
        timestamp: String,
        id: String,
        name: String,
        inputs: String,
        result: String,
        restore: String,
    ) -> LogEntry {
        LogEntry {
            timestamp,
            id,
            name,
            inputs,
            result,
            restore,
            tag: String::new(),
            note: String::new(),
            pinned: false,
        }
    }
}

pub const LOG_CAP: usize = 500;

fn log_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\p")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn log_unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('p') => out.push('|'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// One record per line: timestamp | id | name | inputs | restore |
/// result, then — since v0.7.0 (#39) — | tag | note (every field
/// escaped, result may contain anything once escaped). Older files
/// hold only the first six fields; `parse_entry` accepts both, so
/// a v0.1.1-era history.txt reads unchanged. Since v0.8.0 (#49) a
/// PINNED entry carries one more field, "|1"; unpinned entries
/// serialize byte-identically to the v0.7.0 format, so only lines
/// for pinned entries use the 9-field shape.
pub fn serialize_entry(e: &LogEntry) -> String {
    let mut line = [
        e.timestamp.as_str(),
        e.id.as_str(),
        e.name.as_str(),
        e.inputs.as_str(),
        e.restore.as_str(),
        e.result.as_str(),
        e.tag.as_str(),
        e.note.as_str(),
    ]
    .iter()
    .map(|f| log_escape(f))
    .collect::<Vec<_>>()
    .join("|");
    if e.pinned {
        line.push_str("|1");
    }
    line
}

pub fn parse_entry(line: &str) -> Option<LogEntry> {
    // Split on unescaped '|'.
    let mut fields: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            cur.push(ch);
            if let Some(n) = chars.next() {
                cur.push(n);
            }
        } else if ch == '|' {
            fields.push(cur.clone());
            cur.clear();
        } else {
            cur.push(ch);
        }
        let _ = chars.peek();
    }
    fields.push(cur);
    // 6 fields = the pre-v0.7.0 format (tag/note default empty);
    // 8 fields = with the lab-notebook annotation; 9 fields adds
    // the v0.8.0 pin flag ("1" = pinned).
    if fields.len() != 6 && fields.len() != 8 && fields.len() != 9 {
        return None;
    }
    let mut it = fields.into_iter().map(|f| log_unescape(&f));
    Some(LogEntry {
        timestamp: it.next()?,
        id: it.next()?,
        name: it.next()?,
        inputs: it.next()?,
        restore: it.next()?,
        result: it.next()?,
        tag: it.next().unwrap_or_default(),
        note: it.next().unwrap_or_default(),
        pinned: it.next().map(|f| f == "1").unwrap_or(false),
    })
}

/// Where the log lives: `history.txt` in the app's data folder
/// (v0.9.0: `data\` beside the executable, with the legacy
/// `%APPDATA%\CalcDex` copy-migration and fallbacks — see
/// `crate::paths`).
pub fn history_path() -> std::path::PathBuf {
    crate::paths::data_file("history.txt")
}

pub fn load_history_from(path: &std::path::Path) -> Vec<LogEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines().filter_map(parse_entry).collect()
}

pub fn load_history() -> Vec<LogEntry> {
    load_history_from(&history_path())
}

pub fn save_history_to(path: &std::path::Path, entries: &[LogEntry]) -> std::io::Result<()> {
    let mut text = String::new();
    for e in entries {
        text.push_str(&serialize_entry(e));
        text.push('\n');
    }
    // Atomic (v0.9.0): temp file + flush + rename, like every save
    // of the app's own files.
    crate::paths::atomic_write(path, text.as_bytes())
}

/// Append one entry to the log at `path`. The cap (item #49):
/// at most LOG_CAP UNPINNED entries are kept — when a new entry
/// would exceed that, the oldest unpinned entries are dropped.
/// Pinned entries are never dropped by the trim, so the file can
/// hold LOG_CAP + (number of pins) lines. Returns the full
/// updated list (oldest first).
pub fn append_entry_to(path: &std::path::Path, entry: LogEntry) -> std::io::Result<Vec<LogEntry>> {
    let mut entries = load_history_from(path);
    entries.push(entry);
    let mut unpinned = entries.iter().filter(|e| !e.pinned).count();
    let mut i = 0;
    while unpinned > LOG_CAP && i < entries.len() {
        if !entries[i].pinned {
            entries.remove(i);
            unpinned -= 1;
        } else {
            i += 1;
        }
    }
    save_history_to(path, &entries)?;
    Ok(entries)
}

/// Append one entry to the default log file.
pub fn append_history(entry: LogEntry) -> std::io::Result<Vec<LogEntry>> {
    append_entry_to(&history_path(), entry)
}

pub fn clear_history() -> std::io::Result<()> {
    save_history_to(&history_path(), &[])
}

/// The backup written before every clear (undo for CLEAR LOG):
/// history-backup.txt beside history.txt in the same folder.
pub fn history_backup_path() -> std::path::PathBuf {
    let p = history_path();
    match p.parent() {
        Some(dir) => dir.join("history-backup.txt"),
        None => std::path::PathBuf::from("history-backup.txt"),
    }
}

pub fn backup_history_to(path: &std::path::Path) -> std::io::Result<Vec<LogEntry>> {
    let entries = load_history_from(path);
    let backup = match path.parent() {
        Some(dir) => dir.join("history-backup.txt"),
        None => std::path::PathBuf::from("history-backup.txt"),
    };
    save_history_to(&backup, &entries)?;
    Ok(entries)
}

/// Clear the log at `path`, first writing its current contents to
/// the backup file in the same folder (the undo copy).
pub fn clear_history_with_backup_at(path: &std::path::Path) -> std::io::Result<()> {
    backup_history_to(path)?;
    save_history_to(path, &[])
}

/// Clear the default log, backing it up first (undoable).
pub fn clear_history_with_backup() -> std::io::Result<()> {
    clear_history_with_backup_at(&history_path())
}

/// Undo the last clear: restore the backup into the log at `path`.
/// Returns the restored entries.
pub fn restore_history_backup_at(path: &std::path::Path) -> std::io::Result<Vec<LogEntry>> {
    let backup = match path.parent() {
        Some(dir) => dir.join("history-backup.txt"),
        None => std::path::PathBuf::from("history-backup.txt"),
    };
    let entries = load_history_from(&backup);
    save_history_to(path, &entries)?;
    Ok(entries)
}

/// Undo the last clear of the default log.
pub fn restore_history_backup() -> std::io::Result<Vec<LogEntry>> {
    restore_history_backup_at(&history_path())
}

/// Whether a non-empty cleared-log backup exists (for the GUI's
/// RESTORE CLEARED LOG button).
pub fn history_backup_exists() -> bool {
    !load_history_from(&history_backup_path()).is_empty()
}

/// Build a log entry for a successful run.
pub fn make_entry(c: &Calc, args: &[f64], result: &str) -> LogEntry {
    let toks = display_tokens(c, args);
    LogEntry {
        timestamp: now_stamp(),
        id: c.id.to_string(),
        name: c.name.to_string(),
        inputs: toks.join(", "),
        result: result.to_string(),
        restore: toks.join(";;"),
        tag: String::new(),
        note: String::new(),
        pinned: false,
    }
}

// ---------- lab notebook (v0.7.0, item #39) ----------
//
// The calculation log doubles as a lab notebook: any entry can
// carry a tag and a free-text note (edited in the window's log
// view, or with the CLI's --log-tag / --log-note). Editing an
// entry is a change to the user's record, so — the house rule —
// the current history is first copied to
// history-edit-backup.txt in the same folder (the same
// backup-first pattern as CLEAR LOG, a separate file so the two
// undos never clobber each other).

/// The backup written before every tag/note edit:
/// history-edit-backup.txt beside the history file.
pub fn history_edit_backup_path() -> std::path::PathBuf {
    let p = history_path();
    match p.parent() {
        Some(dir) => dir.join("history-edit-backup.txt"),
        None => std::path::PathBuf::from("history-edit-backup.txt"),
    }
}

fn edit_backup_for(path: &std::path::Path) -> std::path::PathBuf {
    match path.parent() {
        Some(dir) => dir.join("history-edit-backup.txt"),
        None => std::path::PathBuf::from("history-edit-backup.txt"),
    }
}

/// Set (or, with an empty value, clear) the tag and/or note of one
/// log entry in the file at `path`. `newest_index` counts from the
/// newest entry: 0 is the entry `--log` shows first. The file's
/// current contents are backed up BEFORE the edit (see above).
/// Returns the updated entry.
pub fn annotate_entry_at(
    path: &std::path::Path,
    newest_index: usize,
    tag: Option<&str>,
    note: Option<&str>,
) -> Result<LogEntry, String> {
    let mut entries = load_history_from(path);
    if entries.is_empty() {
        return Err(
            "The calculation log is empty — there is no entry to annotate. Run any calculator first."
                .into(),
        );
    }
    if newest_index >= entries.len() {
        return Err(format!(
            "There is no entry #{n} — the log holds {total} entr{ies}. Entry numbers count from the newest, as --log shows them.",
            n = newest_index + 1,
            total = entries.len(),
            ies = if entries.len() == 1 { "y" } else { "ies" }
        ));
    }
    // Backup first (house rule for edits to the record).
    save_history_to(&edit_backup_for(path), &entries)
        .map_err(|e| format!("Could not write the pre-edit backup: {e}"))?;
    let idx = entries.len() - 1 - newest_index;
    if let Some(t) = tag {
        entries[idx].tag = t.trim().to_string();
    }
    if let Some(n) = note {
        entries[idx].note = n.trim().to_string();
    }
    let updated = entries[idx].clone();
    save_history_to(path, &entries).map_err(|e| format!("Could not save the log: {e}"))?;
    Ok(updated)
}

/// The lab record (item #39): the whole log as one plain-text
/// document, entries in chronological order (oldest first, the
/// order they were recorded), each with its tag and note when it
/// has them. Header states the ordering and that timestamps are
/// UTC (no timezone database is bundled, as the log itself says).
pub fn lab_record_text(entries: &[LogEntry]) -> String {
    let mut out = String::new();
    out.push_str("CalcDex lab record\n");
    out.push_str(&format!("Exported: {} (UTC)\n", now_stamp()));
    out.push_str(&format!(
        "{} entries, oldest first. Timestamps are UTC.\n",
        entries.len()
    ));
    let tags: Vec<&str> = {
        let mut t: Vec<&str> = entries
            .iter()
            .map(|e| e.tag.as_str())
            .filter(|t| !t.is_empty())
            .collect();
        t.sort();
        t.dedup();
        t
    };
    if !tags.is_empty() {
        out.push_str(&format!("Tags used: {}\n", tags.join(", ")));
    }
    out.push('\n');
    for (i, e) in entries.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} — {} ({})\n",
            i + 1,
            e.timestamp,
            e.name,
            e.id
        ));
        out.push_str(&format!("   Inputs: {}\n", e.inputs));
        out.push_str(&format!("   Result: {}\n", e.result.replace('\n', "\n   ")));
        if !e.tag.is_empty() {
            out.push_str(&format!("   Tag: {}\n", e.tag));
        }
        if !e.note.is_empty() {
            out.push_str(&format!("   Note: {}\n", e.note.replace('\n', "\n   ")));
        }
        out.push('\n');
    }
    out
}

// ---------- statistics helpers ----------

/// Error function (Abramowitz & Stegun 7.1.26, |error| <= 1.5e-7).
pub fn erf(x: f64) -> f64 {
    let s = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    s * y
}

// ---------- log polish: search, filter, pin, export (v0.8.0, #49) ----------

/// The log search/filter criteria (item #49). Every field is
/// optional; an entry must satisfy ALL set fields to match.
/// `text` is a case-insensitive substring match over the entry's
/// name, calculator id, inputs, result, tag and note. `date`
/// matches the entry's recorded day: the timestamp begins
/// "YYYY-MM-DD", so a "2026-10-25" filter matches that whole day
/// (a shorter prefix, e.g. "2026-10", matches the month).
#[derive(Clone, Default, Debug)]
pub struct LogFilter {
    pub text: Option<String>,
    pub calc_id: Option<String>,
    pub tag: Option<String>,
    pub date: Option<String>,
    pub pinned_only: bool,
}

impl LogFilter {
    pub fn is_empty(&self) -> bool {
        self.text.is_none()
            && self.calc_id.is_none()
            && self.tag.is_none()
            && self.date.is_none()
            && !self.pinned_only
    }

    pub fn matches(&self, e: &LogEntry) -> bool {
        if self.pinned_only && !e.pinned {
            return false;
        }
        if let Some(id) = &self.calc_id {
            if e.id != *id {
                return false;
            }
        }
        if let Some(tag) = &self.tag {
            if e.tag.to_lowercase() != tag.trim().to_lowercase() {
                return false;
            }
        }
        if let Some(date) = &self.date {
            if !e.timestamp.starts_with(date.trim()) {
                return false;
            }
        }
        if let Some(text) = &self.text {
            let q = text.to_lowercase();
            let hay = format!(
                "{} {} {} {} {} {}",
                e.name, e.id, e.inputs, e.result, e.tag, e.note
            )
            .to_lowercase();
            if !hay.contains(&q) {
                return false;
            }
        }
        true
    }

    /// A one-line factual description of the criteria, used in
    /// export headers and result counts.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(t) = &self.text {
            parts.push(format!("text \"{t}\""));
        }
        if let Some(id) = &self.calc_id {
            parts.push(format!("calculator {id}"));
        }
        if let Some(tag) = &self.tag {
            parts.push(format!("tag \"{tag}\""));
        }
        if let Some(d) = &self.date {
            parts.push(format!("date {d}"));
        }
        if self.pinned_only {
            parts.push("pinned only".to_string());
        }
        if parts.is_empty() {
            "no filter (the whole log)".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// The entries matching `filter`, each paired with its GLOBAL
/// entry number — the number `--log` shows for it (1 = newest) —
/// in the log's stored order (oldest first). Callers reverse for
/// newest-first display; the global numbers keep --log-pin /
/// --log-tag / --log-note unambiguous from a filtered view.
pub fn filter_entries<'a>(
    entries: &'a [LogEntry],
    filter: &LogFilter,
) -> Vec<(usize, &'a LogEntry)> {
    let total = entries.len();
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| filter.matches(e))
        .map(|(i, e)| (total - i, e))
        .collect()
}

/// The filtered log export (item #49): the matching entries as
/// one plain-text document, oldest first (the order they were
/// recorded), with the filter stated in the header, the pin mark
/// shown, and the timestamps-are-UTC note the log always carries.
pub fn log_export_text(entries: &[LogEntry], filter: &LogFilter) -> String {
    let matched = filter_entries(entries, filter);
    let mut out = String::new();
    out.push_str("CalcDex calculation log export\n");
    out.push_str(&format!("Exported: {} (UTC)\n", now_stamp()));
    out.push_str(&format!(
        "Filter: {}. {} of {} entries match, oldest first. Timestamps are UTC.\n\n",
        filter.describe(),
        matched.len(),
        entries.len()
    ));
    for (num, e) in &matched {
        out.push_str(&format!(
            "#{} {} — {} ({}){}\n",
            num,
            e.timestamp,
            e.name,
            e.id,
            if e.pinned { " [pinned]" } else { "" }
        ));
        out.push_str(&format!("   Inputs: {}\n", e.inputs));
        out.push_str(&format!("   Result: {}\n", e.result.replace('\n', "\n   ")));
        if !e.tag.is_empty() {
            out.push_str(&format!("   Tag: {}\n", e.tag));
        }
        if !e.note.is_empty() {
            out.push_str(&format!("   Note: {}\n", e.note.replace('\n', "\n   ")));
        }
        out.push('\n');
    }
    out
}

/// Pin or unpin one log entry in the file at `path` (item #49).
/// `newest_index` counts from the newest entry, exactly like
/// `annotate_entry_at`. The file's current contents are backed up
/// BEFORE the edit, to the same history-edit-backup.txt the
/// tag/note edits use (the house backup-first rule). Returns the
/// updated entry.
pub fn set_entry_pinned_at(
    path: &std::path::Path,
    newest_index: usize,
    pinned: bool,
) -> Result<LogEntry, String> {
    let mut entries = load_history_from(path);
    if entries.is_empty() {
        return Err(
            "The calculation log is empty — there is no entry to pin. Run any calculator first."
                .into(),
        );
    }
    if newest_index >= entries.len() {
        return Err(format!(
            "There is no entry #{n} — the log holds {total} entr{ies}. Entry numbers count from the newest, as --log shows them.",
            n = newest_index + 1,
            total = entries.len(),
            ies = if entries.len() == 1 { "y" } else { "ies" }
        ));
    }
    save_history_to(&edit_backup_for(path), &entries)
        .map_err(|e| format!("Could not write the pre-edit backup: {e}"))?;
    let idx = entries.len() - 1 - newest_index;
    entries[idx].pinned = pinned;
    let updated = entries[idx].clone();
    save_history_to(path, &entries).map_err(|e| format!("Could not save the log: {e}"))?;
    Ok(updated)
}

/// Standard normal CDF Phi(z).
pub fn norm_cdf(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// Two-sided p-value for a z statistic.
pub fn p_from_z(z: f64) -> f64 {
    2.0 * (1.0 - norm_cdf(z.abs()))
}

/// Two-sided 95% critical value of Student's t distribution for
/// `df` degrees of freedom (v0.7.0 #33): the standard published
/// table for df 1–30; beyond 30 the normal value 1.96 is used
/// (the t distribution has effectively become the normal there —
/// at df 30 the exact value is 2.042, at df 120 it is 1.980, so
/// the 1.96 step at df 31 is a documented approximation, also
/// stated in DECISIONS.md).
pub fn t_crit_95(df: usize) -> f64 {
    const TABLE: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160,
        2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056,
        2.052, 2.048, 2.045, 2.042,
    ];
    if df == 0 {
        return f64::NAN;
    }
    if df <= 30 {
        TABLE[df - 1]
    } else {
        1.96
    }
}

/// Factorial for non-negative integers up to 170.
pub fn factorial(x: f64) -> Result<f64, String> {
    if x < 0.0 || x != x.trunc() {
        return Err("Factorial needs a whole number, 0 or bigger.".into());
    }
    if x > 170.0 {
        return Err("Factorial is only supported up to 170 (bigger answers overflow).".into());
    }
    let mut r = 1.0f64;
    let mut i = 2.0;
    while i <= x {
        r *= i;
        i += 1.0;
    }
    Ok(r)
}

pub fn gcd_i(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
