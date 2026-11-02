//! v0.7.0 scientists & researchers rest (50-enhancement program
//! items 32, 33, 35, 36, 38, 39, 40). Engines live in the
//! library (lib.rs shared list parser + lab notebook, rigour.rs
//! constants + uncertainty evaluator, stats.rs / chemistry.rs
//! calculators); the GUI/CLI are thin callers, and the CLI tests
//! drive the real binary.

use calcdex::rigour::{
    builtin_constant, constant_value_str, custom_constant, eval_expression,
    eval_expression_uncertain, format_uncertain, has_uncertainty, set_custom_constants,
    validate_constant_name, AngleMode, CONSTANTS, HELP_TEXT,
};
use calcdex::settings::{parse_settings, serialize_settings, Settings};
use calcdex::{
    all_calcs, annotate_entry_at, append_entry_to, lab_record_text, load_history_from, parse_entry,
    parse_value_list, run_calc, run_text_calc, serialize_entry, t_crit_95, LogEntry,
};
use std::collections::BTreeSet;

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ---------- registry ----------

#[test]
fn registry_grew_with_the_three_new_calculators() {
    let calcs = all_calcs();
    // 309 at v0.7.0; v0.8.0 adds the engineering seven (316).
    assert_eq!(calcs.len(), 316);
    for id in ["replicates-summary", "outlier-check", "serial-dilution"] {
        assert!(calcs.iter().any(|c| c.id == id), "registry is missing {id}");
    }
}

// ---------- #32 paste-a-column ----------

#[test]
fn value_list_parser_accepts_every_separator() {
    // A pasted spreadsheet column: one value per line.
    assert_eq!(
        parse_value_list("10\n12\n23\n23").unwrap(),
        vec![10.0, 12.0, 23.0, 23.0]
    );
    // Mixed separators in one paste, with stray empties.
    assert_eq!(
        parse_value_list("1, 2;3\t4  5\n6,,7").unwrap(),
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
    );
    // Scientific notation survives.
    assert_eq!(parse_value_list("1.5e3, 2").unwrap(), vec![1500.0, 2.0]);
    // A bad token is named, with the accepted shapes restated.
    let err = parse_value_list("1, abc, 3").unwrap_err();
    assert!(err.contains("'abc'"), "{err}");
    assert!(err.contains("new lines"), "{err}");
    // Nothing at all is its own teaching error.
    let err = parse_value_list("  \n ").unwrap_err();
    assert!(err.contains("No numbers found"), "{err}");
    // The registry path for list calculators uses the same parser.
    let out = run_calc("mean", &parse_value_list("1\n2\n3").unwrap()).unwrap();
    assert!(out.contains("Mean: 2"), "{out}");
}

// ---------- #33 replicates summary ----------

#[test]
fn replicates_summary_known_answers() {
    // The classic teaching dataset 10, 12, 23, 23, 16, 23, 21, 16:
    // mean 18, sample SD 5.237229, SEM 1.85164, margin (t(7) =
    // 2.365) 4.379129 — all computed independently of the program.
    let out = run_calc(
        "replicates-summary",
        &[10.0, 12.0, 23.0, 23.0, 16.0, 23.0, 21.0, 16.0],
    )
    .unwrap();
    assert!(out.contains("n = 8"), "{out}");
    assert!(out.contains("Mean = 18"), "{out}");
    assert!(out.contains("Sample SD = 5.237229"), "{out}");
    assert!(out.contains("SEM = 1.85164"), "{out}");
    assert!(out.contains("13.620871 to 22.379129"), "{out}");
    assert!(out.contains("t(7) x SEM = 2.365 x 1.85164"), "{out}");
    // One value is not a replicate set.
    assert!(run_calc("replicates-summary", &[5.0])
        .unwrap_err()
        .contains("at least 2"));
}

#[test]
fn t_table_spot_values() {
    assert_eq!(t_crit_95(1), 12.706);
    assert_eq!(t_crit_95(7), 2.365);
    assert_eq!(t_crit_95(30), 2.042);
    // Beyond the table: the documented normal approximation.
    assert_eq!(t_crit_95(31), 1.96);
    assert_eq!(t_crit_95(500), 1.96);
}

// ---------- #36 outlier check ----------

#[test]
fn outlier_check_known_answers() {
    // Sorted 10,10,11,11,12,12,12,13,13,100: Tukey halves give
    // Q1 = 11, Q3 = 13, IQR = 2, fences 8 and 16; 100 is flagged.
    let out = run_calc(
        "outlier-check",
        &[10.0, 12.0, 11.0, 13.0, 12.0, 11.0, 10.0, 12.0, 13.0, 100.0],
    )
    .unwrap();
    assert!(out.contains("Q1 = 11"), "{out}");
    assert!(out.contains("Median = 12"), "{out}");
    assert!(out.contains("Q3 = 13"), "{out}");
    assert!(out.contains("IQR = 2"), "{out}");
    assert!(out.contains("below 8 or above 16"), "{out}");
    assert!(out.contains("Outliers (1): 100"), "{out}");
    // A clean dataset says so plainly.
    let out = run_calc("outlier-check", &[5.0, 6.0, 7.0, 6.0, 5.0, 7.0, 6.0]).unwrap();
    assert!(out.contains("Outliers: none"), "{out}");
    // Too few values is a teaching error.
    assert!(run_calc("outlier-check", &[1.0, 2.0, 3.0])
        .unwrap_err()
        .contains("at least 4"));
}

// ---------- #35 serial dilution ----------

#[test]
fn serial_dilution_both_modes_and_validation() {
    // Mode 1: 1 + 9 volumes = a 1:10 series from 100.
    let out = run_calc("serial-dilution", &[100.0, 1.0, 1.0, 9.0, 10.0, 4.0]).unwrap();
    assert!(out.contains("divides the concentration by 10"), "{out}");
    assert!(out.contains("Step 1: 10."), "{out}");
    assert!(out.contains("Step 2: 1."), "{out}");
    assert!(out.contains("Step 3: 0.1."), "{out}");
    assert!(out.contains("Step 4: 0.01."), "{out}");
    // Mode 2: a plain factor of 5 from 100.
    let out = run_calc("serial-dilution", &[100.0, 2.0, 0.0, 0.0, 5.0, 3.0]).unwrap();
    assert!(out.contains("Step 1: 20."), "{out}");
    assert!(out.contains("Step 2: 4."), "{out}");
    assert!(out.contains("Step 3: 0.8."), "{out}");
    // Validation teaches.
    assert!(
        run_calc("serial-dilution", &[100.0, 3.0, 1.0, 9.0, 10.0, 4.0])
            .unwrap_err()
            .contains("Mode must be 1")
    );
    assert!(
        run_calc("serial-dilution", &[100.0, 2.0, 0.0, 0.0, 1.0, 4.0])
            .unwrap_err()
            .contains("over 1")
    );
    assert!(
        run_calc("serial-dilution", &[100.0, 1.0, 1.0, 0.0, 10.0, 4.0])
            .unwrap_err()
            .contains("diluent")
    );
    assert!(
        run_calc("serial-dilution", &[100.0, 1.0, 1.0, 9.0, 10.0, 0.0])
            .unwrap_err()
            .contains("whole number")
    );
    assert!(
        run_calc("serial-dilution", &[0.0, 1.0, 1.0, 9.0, 10.0, 4.0])
            .unwrap_err()
            .contains("over 0")
    );
}

// ---------- #38 constants library ----------

#[test]
fn constants_library_values_are_codata() {
    assert_eq!(CONSTANTS.len(), 15);
    assert_eq!(builtin_constant("c").unwrap().value, 299_792_458.0);
    assert_eq!(builtin_constant("h").unwrap().value, 6.626_070_15e-34);
    assert_eq!(builtin_constant("k_b").unwrap().value, 1.380_649e-23);
    assert_eq!(builtin_constant("kb").unwrap().value, 1.380_649e-23);
    assert_eq!(builtin_constant("n_a").unwrap().value, 6.022_140_76e23);
    assert_eq!(builtin_constant("avogadro").unwrap().value, 6.022_140_76e23);
    assert_eq!(builtin_constant("eps0").unwrap().value, 8.8541878128e-12);
    assert_eq!(builtin_constant("mu0").unwrap().value, 1.256_637_062_12e-6);
    assert_eq!(builtin_constant("m_e").unwrap().value, 9.1093837015e-31);
    assert_eq!(builtin_constant("m_p").unwrap().value, 1.672_621_923_69e-27);
    assert_eq!(builtin_constant("u").unwrap().value, 1.660_539_066_60e-27);
    assert_eq!(builtin_constant("faraday").unwrap().value, 96_485.332_12);
    assert_eq!(builtin_constant("sigma").unwrap().value, 5.670_374_419e-8);
    assert_eq!(builtin_constant("g").unwrap().value, 6.674_30e-11);
    assert_eq!(builtin_constant("r").unwrap().value, 8.314_462_618);
    // Every alias resolves inside a real expression.
    assert_eq!(
        eval_expression("c", AngleMode::Degrees).unwrap(),
        299_792_458.0
    );
    // The photon-energy identity: E = h c / lambda at 500 nm.
    let e = eval_expression("h*c/500e-9", AngleMode::Degrees).unwrap();
    assert!(
        approx(e, 3.972_891_714_297_857e-19, 1e-30),
        "photon energy {e}"
    );
    // ... and the calculator prints it in full, never as 0.
    let out = run_text_calc("expression-calculator", "h*c/500e-9", true).unwrap();
    assert!(out.contains("3.972891714298e-19"), "{out}");
    // Bare e is still Euler's number; the charge is e_charge.
    assert_eq!(
        eval_expression("e", AngleMode::Degrees).unwrap(),
        std::f64::consts::E
    );
    assert_eq!(
        eval_expression("e_charge", AngleMode::Degrees).unwrap(),
        1.602_176_634e-19
    );
    // Listing format keeps full precision at extreme scales.
    assert_eq!(constant_value_str(6.626_070_15e-34), "6.62607015e-34");
    assert_eq!(constant_value_str(299_792_458.0), "299792458");
}

#[test]
fn custom_constants_validate_install_and_persist() {
    // Validation refuses built-ins, reserved names and bad shapes.
    assert!(validate_constant_name("c")
        .unwrap_err()
        .contains("built-in"));
    assert!(validate_constant_name("n_a")
        .unwrap_err()
        .contains("built-in"));
    assert!(validate_constant_name("sin")
        .unwrap_err()
        .contains("already taken"));
    assert!(validate_constant_name("pi")
        .unwrap_err()
        .contains("already taken"));
    assert!(validate_constant_name("x")
        .unwrap_err()
        .contains("already taken"));
    assert!(validate_constant_name("2bad")
        .unwrap_err()
        .contains("start with a letter"));
    assert!(validate_constant_name("has space")
        .unwrap_err()
        .contains("letters, digits"));
    assert_eq!(validate_constant_name(" My_G ").unwrap(), "my_g");
    // Install + use.
    set_custom_constants(vec![("tube_factor".to_string(), 2.5)]);
    assert_eq!(custom_constant("tube_factor"), Some(2.5));
    assert_eq!(
        eval_expression("tube_factor * 4", AngleMode::Degrees).unwrap(),
        10.0
    );
    set_custom_constants(Vec::new());
    assert!(eval_expression("tube_factor * 4", AngleMode::Degrees).is_err());
    // Settings round trip: the constant survives, and a stored
    // line colliding with a built-in is dropped on load.
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let mut s = Settings::default();
    s.constants.push(("tube_factor".to_string(), 2.5));
    let text = serialize_settings(&s);
    assert!(text.contains("constant.tube_factor=2.5"), "{text}");
    assert_eq!(parse_settings(&text, &valid), s);
    let loaded = parse_settings("constant.c=5\nconstant.ok_one=3\n", &valid);
    assert_eq!(loaded.constants, vec![("ok_one".to_string(), 3.0)]);
}

// ---------- #40 uncertainty propagation ----------

#[test]
fn uncertainty_rules_known_answers() {
    let ev = |s: &str| eval_expression_uncertain(s, AngleMode::Degrees).unwrap();
    // Addition: absolute uncertainties add (0.2 + 0.1 = 0.3).
    let o = ev("(10 \u{b1} 0.2) + (5 \u{b1} 0.1)");
    assert!(
        approx(o.value, 15.0, 1e-12) && approx(o.uncertainty, 0.3, 1e-12),
        "{o:?}"
    );
    assert!(o.steps[0].contains("absolute uncertainties add"), "{o:?}");
    // Subtraction follows the same rule.
    let o = ev("(10 \u{b1} 0.2) - (5 \u{b1} 0.1)");
    assert!(
        approx(o.value, 5.0, 1e-12) && approx(o.uncertainty, 0.3, 1e-12),
        "{o:?}"
    );
    // Multiplication: relative uncertainties add (2% + 2% = 4%).
    let o = ev("(10 \u{b1} 0.2) * (5 \u{b1} 0.1)");
    assert!(
        approx(o.value, 50.0, 1e-12) && approx(o.uncertainty, 2.0, 1e-12),
        "{o:?}"
    );
    assert!(o.steps[0].contains("relative uncertainties add"), "{o:?}");
    assert_eq!(format_uncertain(&o), "50 \u{b1} 2 (4%)");
    // Division: same relative rule (2 / 1 -> 2 +- 0.08).
    let o = ev("(10 \u{b1} 0.2) / (5 \u{b1} 0.1)");
    assert!(
        approx(o.value, 2.0, 1e-12) && approx(o.uncertainty, 0.08, 1e-12),
        "{o:?}"
    );
    // Power with a plain exponent: relative uncertainty x |n|.
    let o = ev("(10 \u{b1} 0.2)^2");
    assert!(
        approx(o.value, 100.0, 1e-9) && approx(o.uncertainty, 4.0, 1e-9),
        "{o:?}"
    );
    assert!(o.steps[0].contains("scales by the power"), "{o:?}");
    // Functions: the slope rule — sqrt(100 +- 2) = 10 +- 0.1.
    let o = ev("sqrt(100 \u{b1} 2)");
    assert!(
        approx(o.value, 10.0, 1e-12) && approx(o.uncertainty, 0.1, 1e-12),
        "{o:?}"
    );
    assert!(o.steps[0].contains("slope"), "{o:?}");
    // The +/- spelling, and ± binding to the atom before it:
    // 2 * (10 ± 0.2) = 20 ± 0.4.
    let o = ev("2 * 10 +/- 0.2");
    assert!(
        approx(o.value, 20.0, 1e-12) && approx(o.uncertainty, 0.4, 1e-12),
        "{o:?}"
    );
    // Constants can carry uncertainties too.
    let o = ev("c \u{b1} 3");
    assert!(approx(o.uncertainty, 3.0, 1e-12), "{o:?}");
    // The registered calculator shows the result + the rules.
    let out = run_text_calc(
        "expression-calculator",
        "(10 \u{b1} 0.2) * (5 \u{b1} 0.1)",
        true,
    )
    .unwrap();
    assert!(out.contains("Result: 50 \u{b1} 2 (4%)."), "{out}");
    assert!(out.contains("relative uncertainties add"), "{out}");
    // Detection + teaching errors.
    assert!(has_uncertainty("10 \u{b1} 0.2"));
    assert!(has_uncertainty("10 +/- 0.2"));
    assert!(!has_uncertainty("10 + 0.2"));
    assert!(eval_expression_uncertain("10 \u{b1}", AngleMode::Degrees).is_err());
    assert!(
        eval_expression_uncertain("(10 \u{b1} 0.2) \u{b1} 0.1", AngleMode::Degrees)
            .unwrap_err()
            .contains("one place only")
    );
}

// ---------- #39 lab notebook ----------

fn entry(ts: &str, name: &str) -> LogEntry {
    LogEntry::plain(
        ts.into(),
        "circle".into(),
        name.into(),
        "5".into(),
        "Area = 78.539816.".into(),
        "5".into(),
    )
}

#[test]
fn lab_notebook_serialization_round_trip_and_legacy() {
    // Tag + note ride along, escapes and all.
    let mut e = entry("2026-10-26 09:00", "Circle (Area, Circumference, Diameter)");
    e.tag = "titration | 3".into();
    e.note = "first line\nsecond \\ line".into();
    let line = serialize_entry(&e);
    assert_eq!(parse_entry(&line), Some(e));
    // A v0.1.1-format line (six fields) parses with an empty
    // tag/note — old history files read unchanged.
    let legacy = "2025-10-25 21:14|circle|Circle|5|5|Area = 78.539816.";
    let parsed = parse_entry(legacy).expect("legacy line must parse");
    assert_eq!(parsed.tag, "");
    assert_eq!(parsed.note, "");
    assert_eq!(parsed.result, "Area = 78.539816.");
    // Any other field count is not an entry.
    assert!(parse_entry("a|b|c").is_none());
}

#[test]
fn lab_notebook_annotate_backs_up_first() {
    let dir = std::env::temp_dir().join(format!("calcdex-v070-lab-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("history.txt");
    append_entry_to(&path, entry("2026-10-26 09:00", "Oldest")).unwrap();
    append_entry_to(&path, entry("2026-10-26 09:05", "Middle")).unwrap();
    append_entry_to(&path, entry("2026-10-26 09:10", "Newest")).unwrap();
    // #1 = the newest entry (the --log numbering).
    let updated = annotate_entry_at(&path, 0, Some("titration-3"), Some("burette rinsed")).unwrap();
    assert_eq!(updated.name, "Newest");
    let entries = load_history_from(&path);
    assert_eq!(entries[2].tag, "titration-3");
    assert_eq!(entries[2].note, "burette rinsed");
    assert_eq!(entries[0].tag, "");
    // The pre-edit backup holds the log as it was BEFORE the edit.
    let backup = load_history_from(&dir.join("history-edit-backup.txt"));
    assert_eq!(backup.len(), 3);
    assert!(backup.iter().all(|e| e.tag.is_empty()));
    // Numbering reaches older entries; out-of-range teaches.
    annotate_entry_at(&path, 2, Some("old"), None).unwrap();
    assert_eq!(load_history_from(&path)[0].tag, "old");
    let err = annotate_entry_at(&path, 3, Some("x"), None).unwrap_err();
    assert!(err.contains("no entry #4"), "{err}");
    assert!(
        annotate_entry_at(&dir.join("missing.txt"), 0, Some("x"), None)
            .unwrap_err()
            .contains("empty")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn lab_record_text_is_chronological_and_complete() {
    let mut entries = vec![
        entry("2026-10-26 09:00", "First"),
        entry("2026-10-26 09:05", "Second"),
    ];
    entries[1].tag = "titration-3".into();
    entries[1].note = "burette rinsed".into();
    let text = lab_record_text(&entries);
    assert!(text.contains("CalcDex lab record"), "{text}");
    assert!(text.contains("UTC"), "{text}");
    assert!(text.contains("oldest first"), "{text}");
    assert!(text.contains("Tag: titration-3"), "{text}");
    assert!(text.contains("Note: burette rinsed"), "{text}");
    assert!(text.contains("Tags used: titration-3"), "{text}");
    let first = text.find("First").unwrap();
    let second = text.find("Second").unwrap();
    assert!(first < second, "record must be chronological: {text}");
}

#[test]
fn help_text_covers_the_scientists_tools() {
    assert!(HELP_TEXT.contains("FOR SCIENTISTS"));
    assert!(HELP_TEXT.contains("CONSTANTS"));
    assert!(HELP_TEXT.contains("lab notebook"));
    assert!(HELP_TEXT.contains("Replicates Summary"));
}

// ---------- CLI end-to-end (binary) ----------

fn cli(args: &[&str], appdata: &std::path::Path) -> std::process::Output {
    // v0.9.0: the app's files live in `data\` beside the exe;
    // CALCDEX_DATA_DIR points each run at the temp dir instead,
    // keeping the test fully isolated.
    std::process::Command::new(env!("CARGO_BIN_EXE_CalcDex"))
        .args(args)
        .env("CALCDEX_DATA_DIR", appdata)
        .output()
        .unwrap()
}

#[test]
fn cli_v070_end_to_end() {
    let dir = std::env::temp_dir().join(format!("calcdex-v070-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stdout = |o: &std::process::Output| String::from_utf8_lossy(&o.stdout).to_string();
    let stderr = |o: &std::process::Output| String::from_utf8_lossy(&o.stderr).to_string();

    // #38: the constants listing, a user constant, and the
    // built-in collision refusal.
    let out = cli(&["--constants"], &dir);
    assert!(out.status.success());
    let t = stdout(&out);
    assert!(t.contains("Planck constant"), "{t}");
    assert!(t.contains("6.62607015e-34"), "{t}");
    let out = cli(&["--define", "my_g=9.81"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let out = cli(&["expr", "my_g * 2"], &dir);
    assert!(stdout(&out).contains("Result: 19.62."), "{}", stdout(&out));
    let out = cli(&["--constants"], &dir);
    assert!(stdout(&out).contains("my_g"), "{}", stdout(&out));
    assert!(stdout(&out).contains("(your constant)"), "{}", stdout(&out));
    let out = cli(&["--define", "c=5"], &dir);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("built-in"), "{}", stderr(&out));
    // The photon energy through the real binary.
    let out = cli(&["expr", "h*c/500e-9"], &dir);
    assert!(
        stdout(&out).contains("3.972891714298e-19"),
        "{}",
        stdout(&out)
    );

    // #40 through the binary.
    let out = cli(&["expr", "(10 \u{b1} 0.2) * (5 \u{b1} 0.1)"], &dir);
    assert!(
        stdout(&out).contains("50 \u{b1} 2 (4%)"),
        "{}",
        stdout(&out)
    );

    // #32/#33: a pasted column as ONE argument.
    let out = cli(
        &["replicates-summary", "10\n12\n23\n23\n16\n23\n21\n16"],
        &dir,
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("Mean = 18"), "{}", stdout(&out));
    let out = cli(&["mean", "1, abc, 3"], &dir);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("'abc'"), "{}", stderr(&out));

    // #35 through the binary.
    let out = cli(&["serial-dilution", "100", "2", "0", "0", "10", "4"], &dir);
    assert!(stdout(&out).contains("Step 4: 0.01."), "{}", stdout(&out));

    // #39: run, tag, note, read back, export.
    let out = cli(&["circle", "5"], &dir);
    assert!(out.status.success());
    let out = cli(&["--log"], &dir);
    let t = stdout(&out);
    assert!(t.contains("#1"), "{t}");
    let out = cli(&["--log-tag", "1", "titration-3"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let out = cli(&["--log-note", "1", "burette", "rinsed"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let out = cli(&["--log"], &dir);
    let t = stdout(&out);
    assert!(t.contains("Tag: titration-3"), "{t}");
    assert!(t.contains("Note: burette rinsed"), "{t}");
    let lab = dir.join("lab.txt");
    let out = cli(&["--lab-export", lab.to_str().unwrap()], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = std::fs::read_to_string(&lab).unwrap();
    assert!(text.contains("CalcDex lab record"), "{text}");
    assert!(text.contains("titration-3"), "{text}");
    assert!(text.contains("burette rinsed"), "{text}");
    // Out-of-range entry numbers are refused.
    let out = cli(&["--log-tag", "9", "x"], &dir);
    assert!(!out.status.success());
    // The pre-edit backup exists beside the log.
    assert!(dir.join("history-edit-backup.txt").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
