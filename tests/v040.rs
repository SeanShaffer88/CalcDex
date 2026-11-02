//! v0.4.0 rigour rest (50-enhancement program items 2, 3, 4, 5,
//! 6, 7, 8, 9, 10, 37) + standing-rule features (built-in help,
//! clear-log backup/restore, batch overwrite protection, update
//! check version logic). Engines live in the library (src/rigour.rs
//! + lib log functions); the GUI/CLI are thin callers.

use calcdex::rigour::{
    annotate_display, compare_versions, count_sig_figs, detect_quantity_dimension,
    dimension_warning, eval_expression, format_copy, format_decimals, format_engineering,
    format_sig_figs_r, infer_sig_figs, latest_tag_from_json, parse_field_value, parse_quantity,
    parse_quantity_to, spec_check, spec_verdict_lines, AngleMode, CopyFormat, DisplayFormat,
    DisplayMode, RoundingMode, HELP_TEXT,
};
use calcdex::settings::{parse_settings, serialize_settings, Settings};
use calcdex::{
    all_calcs, clear_history_with_backup_at, expand_args, find_calc, load_history_from,
    restore_history_backup_at, run_calc, run_text_calc, save_history_to, LogEntry,
};
use std::collections::BTreeSet;

fn calc(id: &str) -> calcdex::Calc {
    find_calc(id).unwrap_or_else(|| panic!("no calc {id}"))
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// ---------- #9 rounding modes ----------

#[test]
fn rounding_modes_at_zero_decimals() {
    // 2.5 / 3.5 / -2.5 in all three modes.
    assert_eq!(format_decimals(2.5, 0, RoundingMode::HalfUp), "3");
    assert_eq!(format_decimals(2.5, 0, RoundingMode::HalfEven), "2");
    assert_eq!(format_decimals(2.5, 0, RoundingMode::Truncate), "2");
    assert_eq!(format_decimals(3.5, 0, RoundingMode::HalfUp), "4");
    assert_eq!(format_decimals(3.5, 0, RoundingMode::HalfEven), "4");
    assert_eq!(format_decimals(3.5, 0, RoundingMode::Truncate), "3");
    assert_eq!(format_decimals(-2.5, 0, RoundingMode::HalfUp), "-3");
    assert_eq!(format_decimals(-2.5, 0, RoundingMode::HalfEven), "-2");
    assert_eq!(format_decimals(-2.5, 0, RoundingMode::Truncate), "-2");
    // Threaded through sig figs too.
    assert_eq!(format_sig_figs_r(2.5, 1, RoundingMode::HalfUp), "3");
    assert_eq!(format_sig_figs_r(2.5, 1, RoundingMode::HalfEven), "2");
    assert_eq!(format_sig_figs_r(3.5, 1, RoundingMode::HalfEven), "4");
    assert_eq!(format_sig_figs_r(2.99, 1, RoundingMode::Truncate), "2");
    // And through the decimals formatter at 2 places.
    assert_eq!(format_decimals(1.005, 2, RoundingMode::Truncate), "1.00");
    assert_eq!(format_decimals(1628.89, 2, RoundingMode::HalfUp), "1628.89");
    assert_eq!(format_decimals(1628.8, 2, RoundingMode::HalfUp), "1628.80");
}

// ---------- #2/#3/#37 display formats ----------

#[test]
fn engineering_notation_boundaries() {
    let f = |v: f64| format_engineering(v, RoundingMode::HalfUp);
    assert!(f(999.0).starts_with("999"), "{}", f(999.0));
    assert!(f(1000.0).starts_with("1 k"), "{}", f(1000.0));
    assert!(f(0.001).starts_with("1 m"), "{}", f(0.001));
    assert!(f(1.0e6).starts_with("1 M"), "{}", f(1.0e6));
    assert!(f(1630.0).starts_with("1.63 k"), "{}", f(1630.0));
    assert!(f(4_200_000.0).starts_with("4.2 M"), "{}", f(4_200_000.0));
    assert!(f(0.35).starts_with("350 m"), "{}", f(0.35));
    assert!(f(-4200.0).starts_with("-4.2 k"), "{}", f(-4200.0));
    assert!(f(2.0e-12).starts_with("2 p"), "{}", f(2.0e-12));
    assert_eq!(f(0.0), "0");
    assert!(f(0.0000123).starts_with("12.3 \u{b5}"), "{}", f(0.0000123));
}

#[test]
fn display_format_parse_and_single_annotation() {
    let out = run_calc("compound-interest", &[1000.0, 5.0, 10.0]).unwrap();
    let dec = DisplayFormat::parse("decimals:2").unwrap();
    let ann = annotate_display(&out, &dec, &[]);
    assert!(ann.contains("[To 2 decimal places:"), "{ann}");
    assert_eq!(
        ann.matches('[').count(),
        1,
        "exactly one annotation line: {ann}"
    );
    assert!(ann.contains("1628.89"), "full precision kept: {ann}");
    let eng = DisplayFormat::parse("engineering").unwrap();
    let ann = annotate_display(&out, &eng, &[]);
    assert!(ann.contains("[Engineering notation:"), "{ann}");
    assert!(ann.contains("1.6289 k"), "{ann}");
    let sf = DisplayFormat::parse("sig-figs:3").unwrap();
    assert!(annotate_display(&out, &sf, &[]).contains("1.63e3"));
    assert!(DisplayFormat::parse("banana").is_err());
    assert!(DisplayFormat::parse("sig-figs:99").is_err());
    assert!(DisplayFormat::parse("decimals:2").unwrap().mode == DisplayMode::Decimals);
}

#[test]
fn sig_fig_inference_counts_as_typed() {
    assert_eq!(count_sig_figs("0.004560"), Some(4));
    assert_eq!(count_sig_figs("100"), Some(1));
    assert_eq!(count_sig_figs("100."), Some(3));
    assert_eq!(count_sig_figs("1.00"), Some(3));
    assert_eq!(count_sig_figs("0.5"), Some(1));
    assert_eq!(count_sig_figs("1.50e3"), Some(3));
    assert_eq!(count_sig_figs("-12.0"), Some(3));
    assert_eq!(count_sig_figs("abc"), None);
    // Auto takes the minimum across inputs. "1000" (no decimal
    // point) carries 1 significant figure by the standard
    // convention; "1000." carries 4.
    let inputs = vec!["1000".to_string(), "5".to_string(), "10".to_string()];
    assert_eq!(infer_sig_figs(&inputs), Some(1));
    let inputs = vec!["1000".to_string(), "5.00".to_string()];
    assert_eq!(infer_sig_figs(&inputs), Some(1));
    let inputs = vec!["1000.".to_string(), "5.00".to_string(), "10.0".to_string()];
    assert_eq!(infer_sig_figs(&inputs), Some(3));
    // Auto annotation names its reason.
    let out = run_calc("compound-interest", &[1000.0, 5.0, 10.0]).unwrap();
    let auto = DisplayFormat::parse("auto").unwrap();
    let ann = annotate_display(&out, &auto, &inputs);
    assert!(ann.contains("Inputs carry 3 significant figures"), "{ann}");
}

// ---------- #4 copy formats ----------

#[test]
fn copy_formatters_on_sample_output() {
    let out = "Future value: 1628.89.\nSteps: 1000 x 1.05^10 = 1628.89.";
    assert_eq!(format_copy(out, CopyFormat::Plain), out);
    let sci = format_copy(out, CopyFormat::Scientific);
    assert!(sci.contains("1.62889e3"), "{sci}");
    // Exponents in the printed working stay plain (no 2e0 noise).
    let sci2 = format_copy("Working: 1.75^2 = 3.0625.", CopyFormat::Scientific);
    assert!(sci2.contains("^2"), "{sci2}");
    assert!(!sci2.contains("2e0"), "{sci2}");
    let csv = format_copy(out, CopyFormat::Csv);
    assert!(
        csv.lines().next().unwrap().starts_with("Future value,"),
        "{csv}"
    );
    assert!(csv.contains("1628.89"), "{csv}");
    // A line with no label becomes a single text field row.
    let csv2 = format_copy("Just a sentence.", CopyFormat::Csv);
    assert_eq!(csv2, "Just a sentence.");
    // LaTeX: numbers wrapped, specials escaped.
    let tex = format_copy("Result: 50% of x_1 = 100.", CopyFormat::Latex);
    assert!(tex.contains("$50$"), "{tex}");
    assert!(tex.contains("$50$\\%"), "{tex}");
    assert!(tex.contains("x\\_1"), "{tex}");
}

// ---------- #5 expression parser ----------

#[test]
fn expression_parser_arithmetic() {
    let d = AngleMode::Degrees;
    assert_eq!(eval_expression("2+3*4", d).unwrap(), 14.0);
    assert_eq!(eval_expression("(2+3)*4", d).unwrap(), 20.0);
    assert_eq!(eval_expression("2^3^2", d).unwrap(), 512.0); // right-assoc
    assert_eq!(eval_expression("-2^2", d).unwrap(), -4.0);
    assert_eq!(eval_expression("2^-2", d).unwrap(), 0.25);
    assert_eq!(eval_expression("sqrt(16)+abs(-3)", d).unwrap(), 7.0);
    assert!(approx(eval_expression("sin(30)", d).unwrap(), 0.5));
    assert!(approx(
        eval_expression("sin(pi/2)", AngleMode::Radians).unwrap(),
        1.0
    ));
    assert!(approx(eval_expression("ln(e)", d).unwrap(), 1.0));
    assert_eq!(eval_expression("log10(1000)", d).unwrap(), 3.0);
    assert_eq!(eval_expression("sqrt(sqrt(81))", d).unwrap(), 3.0);
    assert!(approx(
        eval_expression("tau/2", d).unwrap(),
        std::f64::consts::PI
    ));
    assert!(approx(eval_expression("asin(1)", d).unwrap(), 90.0));
    assert!(approx(eval_expression("2*(3+4)^2", d).unwrap(), 98.0));
    // The registered calculator + paper-tape output shape.
    let out = run_text_calc("expression-calculator", "2*(3+4)^2", true).unwrap();
    assert!(out.contains("Result: 98."), "{out}");
    // Numeric dispatch refuses it with a pointer to the text path.
    assert!(run_calc("expression-calculator", &[1.0]).is_err());
}

#[test]
fn expression_parser_errors_teach() {
    let d = AngleMode::Degrees;
    let e = eval_expression("2 + * 3", d).unwrap_err();
    assert!(e.contains("position 5"), "{e}");
    let e = eval_expression("2*(3", d).unwrap_err();
    assert!(e.contains("expected ')'"), "{e}");
    let e = eval_expression("foo(1)", d).unwrap_err();
    assert!(e.contains("not a known constant or function"), "{e}");
    let e = eval_expression("sqrt(-1)", d).unwrap_err();
    assert!(e.contains("no real answer"), "{e}");
    let e = eval_expression("1/0", d).unwrap_err();
    assert!(e.contains("Division by zero"), "{e}");
    let e = eval_expression("   ", d).unwrap_err();
    assert!(e.contains("empty"), "{e}");
    let e = eval_expression("2+", d).unwrap_err();
    assert!(e.contains("position"), "{e}");
    let e = eval_expression("asin(2)", d).unwrap_err();
    assert!(e.contains("between -1 and 1"), "{e}");
}

// ---------- #6/#7 typed quantities + dimensional check ----------

#[test]
fn typed_quantities_parse_and_convert() {
    assert!(approx(
        parse_quantity("5 ft 3 in", "length").unwrap(),
        1.6002
    ));
    assert!(approx(parse_quantity("1.2 kg", "mass").unwrap(), 1200.0));
    assert!(approx(
        parse_quantity("70 F", "temperature").unwrap(),
        21.111111
    ));
    assert!(approx(
        parse_quantity("2 h 30 min", "time").unwrap(),
        9000.0
    ));
    assert!(approx(
        parse_quantity("3 lb 4 oz", "mass").unwrap(),
        3.0 * 453.59237 + 4.0 * 28.349523125
    ));
    assert!(approx(
        parse_quantity_to("5 ft 3 in", "length", "cm").unwrap(),
        160.02
    ));
    assert!(approx(
        parse_quantity_to("154 lb", "mass", "kg").unwrap(),
        69.85322498
    ));
    // Mismatched family is a teaching error naming both dimensions.
    let e = parse_quantity("1.2 kg", "length").unwrap_err();
    assert!(e.contains("mass"), "{e}");
    assert!(e.contains("length"), "{e}");
    // Detection for the dimensional check.
    assert_eq!(detect_quantity_dimension("70 kg"), Some("mass"));
    assert_eq!(detect_quantity_dimension("5 ft"), Some("length"));
    assert_eq!(detect_quantity_dimension("42"), None);
}

#[test]
fn field_level_typed_input_and_warnings() {
    let bmi = calc("bmi");
    let siblings = vec!["154 lb".to_string(), "5 ft 9 in".to_string()];
    let w = parse_field_value(&bmi, 0, "154 lb", &siblings).unwrap();
    assert!(approx(w, 69.85322498), "{w}");
    let h = parse_field_value(&bmi, 1, "5 ft 9 in", &siblings).unwrap();
    assert!(approx(h, 175.26), "{h}");
    // End-to-end through expand_args (the CLI path).
    let nums = expand_args(&bmi, &siblings).unwrap();
    let out = run_calc("bmi", &nums).unwrap();
    assert!(out.contains("22.74"), "{out}");
    // Dimensional check: mismatched typed input warns; matching
    // and plain numbers never do.
    let warn = dimension_warning(&bmi, 0, "175 cm").expect("expected a warning");
    assert!(warn.contains("expects a mass"), "{warn}");
    assert!(warn.contains("looks like a length"), "{warn}");
    assert!(dimension_warning(&bmi, 0, "70 kg").is_none());
    assert!(dimension_warning(&bmi, 0, "70").is_none());
    // A converter's value field converts into its FROM unit:
    // 5 ft 3 in expressed in yards (unit 6) = 1.75.
    let conv = calc("convert-length");
    let sib = vec!["5 ft 3 in".to_string(), "6".to_string(), "3".to_string()];
    let v = parse_field_value(&conv, 0, "5 ft 3 in", &sib).unwrap();
    assert!(approx(v, 1.75), "{v}");
    // Unit metadata lines up with the registry's input counts.
    for c in all_calcs() {
        if !c.unit_families.is_empty() {
            assert_eq!(c.unit_families.len(), c.inputs.len(), "{} families", c.id);
            assert_eq!(c.dimensions.len(), c.inputs.len(), "{} dimensions", c.id);
            assert_eq!(c.quantity_units.len(), c.inputs.len(), "{} units", c.id);
        }
        if !c.converter_units.is_empty() {
            assert_eq!(
                c.unit_families.first().copied(),
                c.dimensions.first().copied()
            );
        }
    }
}

// ---------- #8 compound converters ----------

#[test]
fn compound_converters_known_answers() {
    let out = run_calc("compound-density", &[1000.0, 0.0, 1.0]).unwrap();
    assert!(out.contains("= 1 g/cm^3"), "{out}");
    let out = run_calc("compound-fuel-consumption", &[8.0, 0.0, 1.0]).unwrap();
    assert!(out.contains("29.401823 mpg (US)"), "{out}");
    let out = run_calc("compound-fuel-consumption", &[10.0, 3.0, 0.0]).unwrap();
    assert!(out.contains("= 10 L/100km"), "{out}");
    let out = run_calc("compound-data-rate", &[100.0, 2.0, 4.0]).unwrap();
    assert!(out.contains("12500 KB/s"), "{out}");
    let out = run_calc("compound-concentration", &[1.0, 3.0, 0.0]).unwrap();
    assert!(out.contains("= 10 mg/mL"), "{out}");
    let out = run_calc("compound-energy-density", &[3_600_000.0, 0.0, 3.0]).unwrap();
    assert!(out.contains("= 1 kWh/m^3"), "{out}");
    // Registry: 300 + expression + 5 compound = 306 (v0.7.0: 309;
    // v0.8.0: 316).
    assert_eq!(all_calcs().len(), 316);
}

// ---------- #10 spec check ----------

#[test]
fn spec_check_pass_fail_and_inclusive_limits() {
    let v = spec_check("BMI: 22.7.\nSteps: 70 / 1.75^2 = 22.7.", 18.5, 25.0);
    assert_eq!(v.len(), 1, "steps lines are not results: {v:?}");
    assert!(v[0].pass);
    // Exactly on a limit = PASS (inclusive, labelled).
    let v = spec_check("Value: 10.", 10.0, 10.0);
    assert!(v[0].pass, "{v:?}");
    assert!(v[0].note.contains("inside"), "{v:?}");
    // Outside: FAIL says which limit and by how much.
    let lines = spec_verdict_lines("Value: 5.", 10.0, 20.0);
    assert!(lines[0].starts_with("FAIL"), "{lines:?}");
    assert!(lines[0].contains("below the minimum 10 by 5"), "{lines:?}");
    let lines = spec_verdict_lines("Value: 25.", 10.0, 20.0);
    assert!(lines[0].contains("above the maximum 20 by 5"), "{lines:?}");
    // A real run: wide limits pass, narrow limits fail.
    let out = run_calc("compound-interest", &[1000.0, 5.0, 10.0]).unwrap();
    let vs = spec_check(&out, 1000.0, 2000.0);
    assert!(!vs.is_empty());
    assert!(vs.iter().all(|v| v.pass), "{vs:?}");
    let vs = spec_check(&out, 0.0, 100.0);
    assert!(vs.iter().any(|v| !v.pass), "{vs:?}");
}

// ---------- standing rules: help, update check, log undo ----------

#[test]
fn help_text_and_version_compare() {
    assert!(HELP_TEXT.contains("KEYBOARD SHORTCUTS"));
    assert!(HELP_TEXT.contains("THE COMMAND LINE IN 3 LINES"));
    assert!(HELP_TEXT.contains("history-backup"));
    assert_eq!(
        compare_versions("v0.4.0", "0.3.9"),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        compare_versions("0.4.0", "v0.4.0"),
        std::cmp::Ordering::Equal
    );
    assert_eq!(compare_versions("0.3.9", "0.4.0"), std::cmp::Ordering::Less);
    assert_eq!(
        compare_versions("1.0", "0.9.9"),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        latest_tag_from_json("{\"tag_name\": \"v1.2.3\", \"name\": \"x\"}").as_deref(),
        Some("v1.2.3")
    );
    assert_eq!(latest_tag_from_json("{}"), None);
    // The update fetch itself needs a network and is labelled
    // Unverified in the docs; only its pure parts are tested here.
}

#[test]
fn clear_log_backup_and_restore_round_trip() {
    let dir = std::env::temp_dir().join(format!("calcdex-v040-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("history.txt");
    let entries = vec![LogEntry {
        timestamp: "2026-10-25 12:00".into(),
        id: "bmi".into(),
        name: "Body Mass Index (BMI)".into(),
        inputs: "70, 175".into(),
        result: "BMI: 22.86.".into(),
        restore: "70;;175".into(),
        tag: String::new(),
        note: String::new(),
        pinned: false,
    }];
    save_history_to(&path, &entries).unwrap();
    clear_history_with_backup_at(&path).unwrap();
    assert!(load_history_from(&path).is_empty(), "log cleared");
    let backup = dir.join("history-backup.txt");
    assert_eq!(
        load_history_from(&backup).len(),
        1,
        "backup holds the cleared entries"
    );
    let restored = restore_history_backup_at(&path).unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(load_history_from(&path)[0].id, "bmi");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn settings_display_round_trip_and_legacy() {
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let s = Settings {
        display: DisplayFormat {
            mode: DisplayMode::Decimals,
            n: Some(2),
            rounding: RoundingMode::HalfEven,
        },
        update_check_startup: true,
        ..Default::default()
    };
    let text = serialize_settings(&s);
    assert!(text.contains("display_format=decimals:2"), "{text}");
    assert!(text.contains("rounding=bankers"), "{text}");
    assert!(text.contains("update_check_startup=on"), "{text}");
    assert_eq!(parse_settings(&text, &valid), s);
    // Legacy v0.3.0 sig_figs line still loads as the sig-figs mode.
    let legacy = parse_settings("sig_figs=4\n", &valid);
    assert_eq!(legacy.display.mode, DisplayMode::SigFigs);
    assert_eq!(legacy.display.n, Some(4));
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
fn cli_v040_end_to_end() {
    let dir = std::env::temp_dir().join(format!("calcdex-v040-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // expr alias + paper tape (the run lands in the log).
    let out = cli(&["expr", "2*(3+4)^2"], &dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("Result: 98"));
    let hist = std::fs::read_to_string(dir.join("history.txt")).unwrap();
    assert!(hist.contains("Expression Calculator"), "{hist}");
    assert!(hist.contains("2*(3+4)^2"), "{hist}");

    // Typed quantities on the CLI.
    let out = cli(&["bmi", "154 lb", "5 ft 9 in"], &dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("22.74"));
    // Dimension mismatch refuses with the warning.
    let out = cli(&["bmi", "175 cm", "175"], &dir);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("expects a mass"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Display format + spec + copy on runs.
    let out = cli(
        &[
            "compound-interest",
            "1000",
            "5",
            "10",
            "--format",
            "engineering",
        ],
        &dir,
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(text.contains("1.6289 k"), "{text}");
    let out = cli(&["bmi", "70", "175", "--spec", "10", "30"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("PASS"));
    let out = cli(&["bmi", "70", "175", "--spec", "30", "40"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("FAIL"));
    let out = cli(
        &["--as", "csv", "compound-interest", "1000", "5", "10"],
        &dir,
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("1628.89"));

    // Help guide.
    let out = cli(&["--help-guide"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("built-in help"));

    // Clear-log backup + restore undo.
    let out = cli(&["--clear-log"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("backed up"));
    let out = cli(&["--log"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("log is empty"));
    let out = cli(&["--restore-log"], &dir);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Restored"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = cli(&["--log"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("Body Mass Index"));

    // Batch overwrite protection.
    let in_csv = dir.join("in.csv");
    let out_csv = dir.join("out.csv");
    std::fs::write(&in_csv, "Principal P,Annual rate %,Years\n1000,5,10\n").unwrap();
    let out = cli(
        &[
            "--batch",
            "compound-interest",
            in_csv.to_str().unwrap(),
            out_csv.to_str().unwrap(),
        ],
        &dir,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = cli(
        &[
            "--batch",
            "compound-interest",
            in_csv.to_str().unwrap(),
            out_csv.to_str().unwrap(),
        ],
        &dir,
    );
    assert!(!out.status.success(), "second write must be refused");
    assert!(String::from_utf8_lossy(&out.stderr).contains("--force"));
    let out = cli(
        &[
            "--batch",
            "compound-interest",
            in_csv.to_str().unwrap(),
            out_csv.to_str().unwrap(),
            "--force",
        ],
        &dir,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.join("out.csv.bak").exists(), "old output backed up");

    let _ = std::fs::remove_dir_all(&dir);
}
