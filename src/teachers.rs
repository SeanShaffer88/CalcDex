//! Teachers-batch engines (v0.6.0, 50-enhancement program items
//! #21–#30). Everything here is a pure library operation: the GUI
//! and the CLI both call these functions, so no feature is
//! window-only.
//!
//! The shared foundation is the PROBLEM SET: an ordered list of
//! entries (calculator id + the concrete inputs, held as display
//! tokens — dates as MM/DD/YYYY, exactly like the settings file's
//! remembered inputs). One plain-text file format serves the set
//! builder (#22), seeded generation (#23), bulk marking (#26),
//! printable worksheets (#27) and assignment export/import (#30):
//!
//! ```text
//! # CalcDex problem set
//! # Name: Fractions and percents — week 4
//! # Seed: 7
//! percent-of | 25 | 80
//! circle | 5
//! age | 04/23/1988 | 10/25/2026
//! ```
//!
//! `#` lines are comments; `# Name:` and `# Seed:` carry the set's
//! name and (for generated sets) the seed it was made from. Every
//! other line is one problem: the calculator id, then one input
//! per `|`-separated field, in the calculator's field order (the
//! order `--describe` numbers them). A set file holds PROBLEMS
//! ONLY — never answers — which is what makes it safe to hand to
//! students as an assignment (#30). It differs from the batch CSV
//! format (#31) in shape, not spirit: batch CSV holds one
//! calculator over many rows; a set holds many calculators, one
//! problem per line, in a file a teacher can read and edit by hand.
//!
//! Entries are validated when they are added or a file is loaded:
//! the calculator must exist, take numbers (not text), and the
//! inputs must actually run — so a set can never contain a problem
//! CalcDex itself cannot answer.

use crate::features::{csv_row, headline_of, parse_csv};
use crate::meta::{display_fields, formula_for, glossary_for};
use crate::students::{check_answer, practice_problem, substituted_formula, Rng};
use crate::{display_tokens, expand_args, find_calc, run_calc, Calc};

// ---------- problem sets (#22 foundation) ----------

#[derive(Clone, Debug, PartialEq)]
pub struct SetEntry {
    pub calc_id: String,
    /// Display tokens, one per user-visible field (dates merged).
    pub inputs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProblemSet {
    pub name: String,
    /// The seed the set was generated from, when it was (#23).
    pub seed_note: Option<u64>,
    pub entries: Vec<SetEntry>,
}

impl ProblemSet {
    pub fn new(name: &str) -> ProblemSet {
        ProblemSet {
            name: if name.trim().is_empty() {
                "Untitled set".to_string()
            } else {
                name.trim().to_string()
            },
            seed_note: None,
            entries: Vec::new(),
        }
    }
}

/// Run one entry: parse its tokens for its calculator and run it.
/// The one dispatch every rendering (key, worksheet answers,
/// marking) goes through.
pub fn run_entry(entry: &SetEntry) -> Result<(Calc, Vec<f64>, String), String> {
    let calc = find_calc(&entry.calc_id)
        .ok_or_else(|| format!("No calculator called '{}'.", entry.calc_id))?;
    if calc.text_run.is_some() {
        return Err(format!(
            "'{}' takes text, not numbers — problem sets hold numeric calculators.",
            calc.name
        ));
    }
    let toks: Vec<String> = entry.inputs.clone();
    let args = expand_args(&calc, &toks)?;
    let out = run_calc(calc.id, &args)?;
    Ok((calc, args, out))
}

/// Validate a would-be entry and build it: the calculator exists,
/// takes numbers, and the given inputs run cleanly.
pub fn make_entry(calc_id: &str, inputs: &[String]) -> Result<SetEntry, String> {
    let entry = SetEntry {
        calc_id: calc_id.to_string(),
        inputs: inputs.to_vec(),
    };
    run_entry(&entry)?;
    Ok(entry)
}

/// Add an entry to a set (the `set add` verb's engine).
pub fn set_add(set: &mut ProblemSet, calc_id: &str, inputs: &[String]) -> Result<(), String> {
    let entry = make_entry(calc_id, inputs)?;
    set.entries.push(entry);
    Ok(())
}

/// One problem stated in words: "Percent Of with X = 25, Y = 80".
/// Used by the worksheet, the key, `set open` and the SETS view —
/// the same wording everywhere.
pub fn problem_line(calc: &Calc, inputs: &[String]) -> String {
    let fields = display_fields(calc);
    if calc.list {
        return format!(
            "{} with {} = {}",
            calc.name,
            fields.first().cloned().unwrap_or_default(),
            inputs.join(", ")
        );
    }
    let pairs: Vec<String> = fields
        .iter()
        .zip(inputs.iter())
        .map(|(f, t)| format!("{f} = {t}"))
        .collect();
    format!("{} with {}", calc.name, pairs.join(", "))
}

pub fn entry_problem_line(entry: &SetEntry) -> Result<String, String> {
    let calc = find_calc(&entry.calc_id)
        .ok_or_else(|| format!("No calculator called '{}'.", entry.calc_id))?;
    Ok(problem_line(&calc, &entry.inputs))
}

/// The set file format (see the module docs).
pub fn set_to_text(set: &ProblemSet) -> String {
    let mut out = String::new();
    out.push_str("# CalcDex problem set\n");
    out.push_str(&format!("# Name: {}\n", set.name.replace('\n', " ")));
    if let Some(seed) = set.seed_note {
        out.push_str(&format!("# Seed: {seed}\n"));
    }
    out.push_str("# One problem per line: calculator id | input | input | ...\n");
    for e in &set.entries {
        let mut parts = vec![e.calc_id.clone()];
        parts.extend(e.inputs.iter().cloned());
        out.push_str(&parts.join(" | "));
        out.push('\n');
    }
    out
}

/// Parse a set file. Every entry is validated (see module docs);
/// the first bad line stops the load with a message naming it.
pub fn parse_set(text: &str) -> Result<ProblemSet, String> {
    let mut set = ProblemSet::new("");
    let mut saw_header = false;
    for (ln, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim();
            if let Some(name) = comment.strip_prefix("Name:") {
                set.name = name.trim().to_string();
            } else if let Some(seed) = comment.strip_prefix("Seed:") {
                set.seed_note = seed.trim().parse::<u64>().ok();
            } else if comment.starts_with("CalcDex problem set") {
                saw_header = true;
            }
            continue;
        }
        let parts: Vec<String> = line.split('|').map(|p| p.trim().to_string()).collect();
        if parts.len() < 2 || parts[0].is_empty() {
            return Err(format!(
                "Line {} is not a problem — it should look like: circle | 5",
                ln + 1
            ));
        }
        let entry = make_entry(&parts[0], &parts[1..])
            .map_err(|e| format!("Line {} ({}): {e}", ln + 1, parts[0]))?;
        set.entries.push(entry);
    }
    if !saw_header {
        return Err(
            "That file is not a CalcDex problem set — it should start with the line '# CalcDex problem set' and list one problem per line."
                .into(),
        );
    }
    // A headed set with no problems yet is valid (it is what
    // `set new` creates, ready for `set add`); renderings that
    // need problems check for emptiness themselves.
    Ok(set)
}

pub fn write_set(path: &std::path::Path, set: &ProblemSet) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, set_to_text(set))
}

pub fn read_set(path: &std::path::Path) -> Result<ProblemSet, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    parse_set(&text)
}

/// The numbered problem list `set open` prints: problems only,
/// never answers (this is also the import view, #30).
pub fn set_listing(set: &ProblemSet) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", set.name));
    if let Some(seed) = set.seed_note {
        out.push_str(&format!("Made from seed {seed}.\n"));
    }
    out.push_str(&format!("{} problem(s):\n", set.entries.len()));
    for (i, e) in set.entries.iter().enumerate() {
        match entry_problem_line(e) {
            Ok(line) => out.push_str(&format!("  {}. {line}\n", i + 1)),
            Err(err) => out.push_str(&format!("  {}. {err}\n", i + 1)),
        }
    }
    out
}

// ---------- seeded generation (#23) ----------

/// Generate a set of `count` problems from the given calculators,
/// using the practice generator (#15) driven by ONE seeded RNG.
/// Deterministic by construction: the same seed, calculator list
/// and count always produce the identical set, byte for byte — a
/// whole class can be given the same problems from the seed alone.
pub fn generate_set(
    ids: &[&str],
    seed: u64,
    count: usize,
    name: &str,
) -> Result<ProblemSet, String> {
    if count == 0 {
        return Err("A set needs at least 1 problem — give a count of 1 or more.".into());
    }
    if ids.is_empty() {
        return Err("Name at least one calculator to make problems from.".into());
    }
    let mut calcs: Vec<Calc> = Vec::new();
    for id in ids {
        let calc = find_calc(id)
            .ok_or_else(|| format!("No calculator called '{id}'. Try: CalcDex --list"))?;
        if !crate::students::practice_eligible(&calc) {
            return Err(format!(
                "'{}' cannot make practice problems. {}",
                calc.name,
                crate::students::PRACTICE_EXCLUSION_NOTE
            ));
        }
        calcs.push(calc);
    }
    let mut rng = Rng::new(seed);
    let mut set = ProblemSet::new(name);
    set.seed_note = Some(seed);
    let mut turn = 0usize;
    while set.entries.len() < count {
        let calc = &calcs[turn % calcs.len()];
        turn += 1;
        match practice_problem(calc, &mut rng) {
            Some(p) => {
                let toks = display_tokens(calc, &p.args);
                set.entries.push(SetEntry {
                    calc_id: calc.id.to_string(),
                    inputs: toks,
                });
            }
            None => {
                return Err(format!(
                    "Could not make a problem for '{}' — try another calculator.",
                    calc.name
                ))
            }
        }
    }
    Ok(set)
}

// ---------- worksheet + answer key renderings (#27 / #22) ----------

const WORK_LINES: usize = 3;

fn work_space() -> String {
    let mut s = String::new();
    for _ in 0..WORK_LINES {
        s.push_str("   ________________________________________________________________\n");
    }
    s.push_str("   Answer: ______________________\n");
    s
}

/// The student worksheet (#27): the set's problems, numbered,
/// with working space and an answer line under each, plus
/// name/date/class header lines. A plain-text file the teacher
/// prints from any editor — CalcDex has no print dialog.
/// Contains NO answers (checked by test, #30).
pub fn worksheet_text(set: &ProblemSet) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", set.name));
    out.push_str("Name: ______________________  Date: ____________  Class: ____________\n\n");
    out.push_str("Show your working in the space under each problem.\n\n");
    for (i, e) in set.entries.iter().enumerate() {
        match entry_problem_line(e) {
            Ok(line) => out.push_str(&format!("{}. {line}\n", i + 1)),
            Err(err) => out.push_str(&format!("{}. {err}\n", i + 1)),
        }
        out.push_str(&work_space());
        out.push('\n');
    }
    out
}

/// The teacher's answer key (#22): the same numbering as the
/// worksheet, each problem with its answer (the run's headline)
/// and a one-line working (the formula with the problem's own
/// values substituted, from the deeper-steps engine). Headed as
/// the teacher copy so the two files are never confused.
pub fn answer_key_text(set: &ProblemSet) -> String {
    let mut out = String::new();
    out.push_str("ANSWER KEY — TEACHER COPY\n");
    out.push_str(&format!("{}\n\n", set.name));
    for (i, e) in set.entries.iter().enumerate() {
        match run_entry(e) {
            Ok((calc, args, output)) => {
                out.push_str(&format!("{}. {}\n", i + 1, problem_line(&calc, &e.inputs)));
                out.push_str(&format!("   Answer: {}\n", headline_of(&output)));
                out.push_str(&format!(
                    "   Working: {}\n",
                    substituted_formula(&calc, &args)
                ));
            }
            Err(err) => {
                out.push_str(&format!("{}. Could not run: {err}\n", i + 1));
            }
        }
        out.push('\n');
    }
    out
}

// ---------- formula sheets (#29) ----------

/// A formula sheet for one category — or every category when
/// `category` is None: each calculator's name, its formula (the
/// #19 metadata) and what every symbol means. The printable
/// reference a teacher hands out with a worksheet.
pub fn formula_sheet_text(calcs: &[Calc], category: Option<&str>) -> Result<String, String> {
    let mut out = String::new();
    match category {
        Some(cat) => {
            let matched: Vec<&Calc> = calcs.iter().filter(|c| c.category == cat).collect();
            if matched.is_empty() {
                let names: Vec<&str> = {
                    let mut v: Vec<&str> = Vec::new();
                    for c in calcs {
                        if !v.contains(&c.category) {
                            v.push(c.category);
                        }
                    }
                    v
                };
                return Err(format!(
                    "No category called '{cat}'. Categories: {}.",
                    names.join(", ")
                ));
            }
            out.push_str(&format!("CalcDex formula sheet — {cat}\n"));
            out.push_str("One formula per calculator, with what every symbol means.\n\n");
            for c in matched {
                push_formula_block(&mut out, c);
            }
        }
        None => {
            out.push_str("CalcDex formula sheet — all categories\n");
            out.push_str("One formula per calculator, with what every symbol means.\n\n");
            let mut cats: Vec<&str> = Vec::new();
            for c in calcs {
                if !cats.contains(&c.category) {
                    cats.push(c.category);
                }
            }
            for cat in cats {
                out.push_str(&format!("== {cat} ==\n\n"));
                for c in calcs.iter().filter(|c| c.category == cat) {
                    push_formula_block(&mut out, c);
                }
            }
        }
    }
    Ok(out)
}

fn push_formula_block(out: &mut String, c: &Calc) {
    out.push_str(&format!("{} ({})\n", c.name, c.id));
    out.push_str(&format!("  Formula: {}\n", formula_for(c)));
    for (sym, meaning) in glossary_for(c) {
        out.push_str(&format!("  {sym} = {meaning}\n"));
    }
    out.push('\n');
}

// ---------- bulk marking (#26) ----------

#[derive(Clone, Debug, PartialEq)]
pub struct MarkRow {
    pub name: String,
    /// One per set entry: true = right.
    pub marks: Vec<bool>,
    pub score: usize,
    pub total: usize,
}

#[derive(Clone, Debug)]
pub struct MarkOutcome {
    /// The marked CSV: name, per-problem right/wrong, score.
    pub output_csv: String,
    pub rows: Vec<MarkRow>,
    /// One "{name}: {score}/{total}" line per student.
    pub summary: Vec<String>,
}

/// Mark a class set of answers against a problem set (#26). The
/// answers CSV holds one row per student: their name, then their
/// answers in set order (a first row headed "Name,..." is treated
/// as a header and skipped). Checking is the check-my-answer
/// engine (#13): exact for the fraction calculators, within 0.5%
/// otherwise; a blank, missing or unparseable answer — or one the
/// checker cannot compare — counts as wrong.
pub fn mark_set(set: &ProblemSet, answers_csv: &str) -> Result<MarkOutcome, String> {
    if set.entries.is_empty() {
        return Err("The problem set is empty — nothing to mark against.".into());
    }
    // Pre-run every entry once: the expected outputs.
    let mut expected: Vec<(Calc, String)> = Vec::new();
    for e in &set.entries {
        let (calc, _args, out) = run_entry(e)?;
        expected.push((calc, out));
    }
    let rows_in = parse_csv(answers_csv);
    let mut out_rows: Vec<Vec<String>> = Vec::new();
    let mut header: Vec<String> = vec!["Student".to_string()];
    for i in 0..set.entries.len() {
        header.push(format!("Q{}", i + 1));
    }
    header.push("Score".to_string());
    out_rows.push(header);
    let mut rows: Vec<MarkRow> = Vec::new();
    let mut summary: Vec<String> = Vec::new();
    for row in &rows_in {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let name = row.first().cloned().unwrap_or_default().trim().to_string();
        if rows.is_empty()
            && (name.eq_ignore_ascii_case("name") || name.eq_ignore_ascii_case("student"))
        {
            continue; // a header row, not a student
        }
        if name.is_empty() {
            continue;
        }
        let mut marks: Vec<bool> = Vec::new();
        for (i, (calc, out)) in expected.iter().enumerate() {
            let ans = row.get(i + 1).map(|s| s.trim()).unwrap_or("");
            let right = !ans.is_empty()
                && check_answer(calc, out, ans)
                    .map(|o| o.correct)
                    .unwrap_or(false);
            marks.push(right);
        }
        let score = marks.iter().filter(|m| **m).count();
        let total = set.entries.len();
        let mut cells: Vec<String> = vec![name.clone()];
        for m in &marks {
            cells.push(if *m { "right" } else { "wrong" }.to_string());
        }
        cells.push(format!("{score}/{total}"));
        out_rows.push(cells);
        summary.push(format!("{name}: {score}/{total}"));
        rows.push(MarkRow {
            name,
            marks,
            score,
            total,
        });
    }
    if rows.is_empty() {
        return Err(
            "No student rows found — the answers CSV needs one row per student: name, then their answers in problem order."
                .into(),
        );
    }
    Ok(MarkOutcome {
        output_csv: out_rows
            .iter()
            .map(|r| csv_row(r))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
        rows,
        summary,
    })
}

/// The default marked-CSV path for `--mark`: the answers file's
/// name with ".marked.csv" appended.
pub fn marked_path_for(answers_path: &std::path::Path) -> std::path::PathBuf {
    let mut s = answers_path.as_os_str().to_owned();
    s.push(".marked.csv");
    std::path::PathBuf::from(s)
}

// ---------- saved class examples library (#28) ----------

#[derive(Clone, Debug, PartialEq)]
pub struct SavedExample {
    pub title: String,
    pub calc_id: String,
    pub note: String,
    /// Display tokens, one per user-visible field (dates merged).
    pub inputs: Vec<String>,
}

/// Where the examples library lives: `examples.txt` in the app's
/// data folder (v0.9.0: `data\` beside the executable, with the
/// legacy `%APPDATA%\CalcDex` copy-migration and fallbacks — see
/// `crate::paths`; same rule as the settings and history files).
pub fn examples_path() -> std::path::PathBuf {
    crate::paths::data_file("examples.txt")
}

fn ex_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\p")
        .replace(';', "\\;")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn ex_unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
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

/// Split on unescaped '|' (the escape pairs stay in the pieces
/// for `ex_unescape` to resolve).
fn split_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            cur.push(ch);
            if let Some(n) = chars.next() {
                cur.push(n);
            }
        } else if ch == '|' {
            fields.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    fields.push(cur);
    fields
}

/// Split the inputs field on unescaped ";;".
fn split_input_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == '\\' && i + 1 < b.len() {
            cur.push(b[i]);
            cur.push(b[i + 1]);
            i += 2;
        } else if b[i] == ';' && i + 1 < b.len() && b[i + 1] == ';' {
            out.push(ex_unescape(&cur));
            cur.clear();
            i += 2;
        } else {
            cur.push(b[i]);
            i += 1;
        }
    }
    out.push(ex_unescape(&cur));
    out
}

/// The examples file format: one record per line —
/// title | calculator id | note | input;;tokens — with backslash
/// escapes (`\\`, `\p` for |, `\;`, `\n`) inside fields, the same
/// discipline as the settings file.
pub fn serialize_examples(list: &[SavedExample]) -> String {
    let mut out = String::new();
    out.push_str(
        "# CalcDex saved examples — title | calculator id | note | inputs (see teachers.rs).\n",
    );
    for e in list {
        let toks: Vec<String> = e.inputs.iter().map(|t| ex_escape(t)).collect();
        out.push_str(&format!(
            "{} | {} | {} | {}\n",
            ex_escape(&e.title),
            e.calc_id,
            ex_escape(&e.note),
            toks.join(";;")
        ));
    }
    out
}

/// Parse the examples file. Unknown calculator ids and malformed
/// lines are skipped (a library from another version can never
/// break startup); a repeated title keeps the LAST copy.
pub fn parse_examples(
    text: &str,
    valid_ids: &std::collections::BTreeSet<String>,
) -> Vec<SavedExample> {
    let mut list: Vec<SavedExample> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = split_fields(line);
        if fields.len() != 4 {
            continue;
        }
        let title = ex_unescape(fields[0].trim());
        let calc_id = fields[1].trim().to_string();
        if title.is_empty() || !valid_ids.contains(&calc_id) {
            continue;
        }
        let inputs = if fields[3].trim().is_empty() {
            Vec::new()
        } else {
            split_input_tokens(fields[3].trim())
        };
        let ex = SavedExample {
            title: title.clone(),
            calc_id,
            note: ex_unescape(fields[2].trim()),
            inputs,
        };
        if let Some(pos) = list.iter().position(|e| e.title == title) {
            list[pos] = ex;
        } else {
            list.push(ex);
        }
    }
    list
}

pub fn load_examples_from(
    path: &std::path::Path,
    valid_ids: &std::collections::BTreeSet<String>,
) -> Vec<SavedExample> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse_examples(&text, valid_ids),
        Err(_) => Vec::new(),
    }
}

pub fn load_examples(valid_ids: &std::collections::BTreeSet<String>) -> Vec<SavedExample> {
    load_examples_from(&examples_path(), valid_ids)
}

pub fn save_examples_to(path: &std::path::Path, list: &[SavedExample]) -> std::io::Result<()> {
    // Atomic save (v0.9.0): temp file + flush + rename.
    crate::paths::atomic_write(path, serialize_examples(list).as_bytes())
}

pub fn save_examples(list: &[SavedExample]) -> std::io::Result<()> {
    save_examples_to(&examples_path(), list)
}

/// Add (or replace, by title) one example in the library.
pub fn upsert_example(list: &mut Vec<SavedExample>, ex: SavedExample) {
    if let Some(pos) = list.iter().position(|e| e.title == ex.title) {
        list[pos] = ex;
    } else {
        list.push(ex);
    }
}

// ---------- demonstration mode masking (#25) ----------

/// The line shown where a hidden answer would be.
pub const DEMO_HIDDEN: &str = "Answer hidden — press REVEAL";

/// Mask a run's result values for demonstration mode (#25): every
/// number in the output is replaced with ••• EXCEPT numbers equal
/// to one of the run's own input values — the inputs and the
/// structure of the working stay readable, the answers do not.
/// (A result that happens to equal an input value stays visible;
/// stated in DECISIONS.) Display-only: the calculation log always
/// stores the real, unmasked run — the log is the teacher's own
/// record, and RE-OPEN is a teacher action.
pub fn mask_result_text(output: &str, input_values: &[f64]) -> String {
    let keep = |v: f64| input_values.iter().any(|iv| (v - *iv).abs() < 1e-12);
    let mut out_lines: Vec<String> = Vec::new();
    for line in output.lines() {
        let spans = crate::rigour::number_spans(line);
        if spans.is_empty() {
            out_lines.push(line.to_string());
            continue;
        }
        let mut rebuilt = String::new();
        let mut pos = 0usize;
        for (s, e) in spans {
            rebuilt.push_str(&line[pos..s]);
            let tok = &line[s..e];
            match tok.parse::<f64>() {
                Ok(v) if keep(v) => rebuilt.push_str(tok),
                _ => rebuilt.push_str("•••"),
            }
            pos = e;
        }
        rebuilt.push_str(&line[pos..]);
        out_lines.push(rebuilt);
    }
    out_lines.join("\n")
}
