//! v0.3.0 Batch 1 (50-enhancement program items 1, 11, 16, 20, 31,
//! 34, 46, 47, 48, 50) — engine tests. Every feature lives in the
//! library; the GUI/CLI are thin callers.

use calcdex::features::{
    annotate_sig_figs, batch_run, format_sig_figs, linear_regression, parse_csv,
    regression_predict_x, search_ranked, search_score, sig_figs_headline, vary_run, SYNONYMS,
};
use calcdex::meta::{defaults_for, display_fields, hint_for};
use calcdex::settings::{parse_settings, serialize_settings, Settings};
use calcdex::{all_calcs, find_calc, run_calc};
use std::collections::BTreeSet;

fn calc(id: &str) -> calcdex::Calc {
    find_calc(id).unwrap_or_else(|| panic!("no calc {id}"))
}

// ---------- #1 sig figs ----------

#[test]
fn sig_figs_classic_traps() {
    assert_eq!(format_sig_figs(0.0, 3), "0");
    assert_eq!(format_sig_figs(0.00456, 3), "0.00456");
    assert_eq!(format_sig_figs(0.00456, 2), "0.0046");
    assert_eq!(format_sig_figs(1628.89, 3), "1.63e3");
    assert_eq!(format_sig_figs(1628.89, 5), "1628.9");
    assert_eq!(format_sig_figs(999.9, 4), "999.9");
    assert_eq!(format_sig_figs(999.9, 3), "1.00e3");
    assert_eq!(format_sig_figs(0.0000999, 2), "0.00010");
    assert_eq!(format_sig_figs(-123.456, 4), "-123.5");
    assert_eq!(format_sig_figs(1000.0, 3), "1.00e3");
    assert_eq!(format_sig_figs(5.0, 3), "5.00");
    assert_eq!(format_sig_figs(0.5, 3), "0.500");
    assert_eq!(format_sig_figs(123456.0, 4), "1.235e5");
    assert_eq!(format_sig_figs(2.5, 1), "3");
    assert_eq!(format_sig_figs(9.99, 2), "10");
}

#[test]
fn sig_figs_annotation_keeps_full_precision_and_dates() {
    let out = run_calc("compound-interest", &[1000.0, 5.0, 10.0]).unwrap();
    let ann = annotate_sig_figs(&out, 3);
    assert!(ann.contains("1628.89"), "full precision kept: {ann}");
    assert!(ann.contains("1.63e3"), "rounded showing added: {ann}");
    // A date in the headline is never mangled.
    let due = run_calc("due-date", &[2025.0, 1.0, 1.0]).unwrap();
    let head = sig_figs_headline(&due, 3);
    assert!(head.contains("2025"), "date year mangled: {head}");
}

// ---------- #11 examples ----------

#[test]
fn every_calculator_has_a_working_example() {
    let calcs = all_calcs();
    assert_eq!(
        calcs.len(),
        316,
        "300 + expression + 5 compound converters (v0.4.0) + replicates/outlier/serial-dilution (v0.7.0) + the engineering seven (v0.8.0)"
    );
    for c in &calcs {
        if c.text_run.is_some() {
            // Text calculator (v0.4.0 #5): its example is text.
            assert!(!c.example_text.is_empty(), "{} has no text example", c.id);
            assert!(calcdex::run_text_calc(c.id, c.example_text, true).is_ok());
            continue;
        }
        assert!(
            !c.example.is_empty(),
            "{} has no example inputs (item #11)",
            c.id
        );
        let out = run_calc(c.id, c.example);
        assert!(
            out.is_ok(),
            "{} example {:?} failed: {:?}",
            c.id,
            c.example,
            out.err()
        );
    }
}

// ---------- #20 hints ----------

#[test]
fn every_input_has_a_hint() {
    for c in all_calcs() {
        for i in 0..c.inputs.len() {
            // Inner fields of a date group are never rendered — the
            // group's own hint (at its start index) covers them.
            if c.date_groups.iter().any(|g| i > g.start && i < g.start + 3) {
                continue;
            }
            let h = hint_for(&c, i);
            assert!(!h.is_empty(), "{} input {i} has no hint", c.id);
        }
        // --describe data: display fields line up with the inputs.
        if !c.list {
            assert_eq!(display_fields(&c).len(), {
                let mut n = 0;
                let mut i = 0;
                while i < c.inputs.len() {
                    if calcdex::date_group_at(&c, i).is_some() {
                        n += 1;
                        i += 3;
                    } else if c.date_groups.iter().any(|g| i > g.start && i < g.start + 3) {
                        i += 1;
                    } else {
                        n += 1;
                        i += 1;
                    }
                }
                n
            });
        }
    }
    // The code-field hints actually explain the codes.
    let h = hint_for(&calc("ideal-weight-devine"), 0);
    assert!(h.contains("1 = male"), "code hint: {h}");
    let h = hint_for(&calc("convert-temperature"), 1);
    assert!(h.contains("0 = Celsius"), "code hint: {h}");
    let h = hint_for(&calc("bmi"), 0);
    assert!(h.contains("kg"), "unit hint from label: {h}");
}

// ---------- #50 smart defaults ----------

#[test]
fn defaults_exist_and_run() {
    for c in all_calcs() {
        if c.text_run.is_some() {
            continue; // text calculator: its default is its example text
        }
        let d = defaults_for(&c);
        assert!(!d.is_empty(), "{} has no defaults", c.id);
        assert!(run_calc(c.id, d).is_ok(), "{} defaults fail", c.id);
    }
    // Gas-law defaults are the discipline standard (STP).
    assert_eq!(defaults_for(&calc("ideal-gas-volume")), &[1.0, 273.15, 1.0]);
}

// ---------- #16 vary ----------

#[test]
fn vary_sweep_matches_direct_runs() {
    let c = calc("compound-interest");
    let rows = vary_run(&c, &[1000.0, 5.0, 10.0], 1, 0.0, 10.0, 2.0).unwrap();
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[0].value, 0.0);
    assert_eq!(rows[5].value, 10.0);
    let direct = run_calc("compound-interest", &[1000.0, 10.0, 10.0]).unwrap();
    assert_eq!(rows[5].result.as_ref().unwrap(), &direct);
    // Cap + bad sweeps are teaching errors.
    assert!(vary_run(&c, &[1000.0, 5.0, 10.0], 1, 0.0, 1000.0, 1.0)
        .unwrap_err()
        .contains("200"));
    assert!(vary_run(&c, &[1000.0, 5.0, 10.0], 1, 0.0, 10.0, 0.0).is_err());
    assert!(vary_run(&c, &[1000.0, 5.0, 10.0], 9, 0.0, 10.0, 1.0).is_err());
    // List calculators refuse (no single numeric input to sweep).
    let m = calc("mean");
    assert!(vary_run(&m, &[1.0], 0, 0.0, 1.0, 1.0).is_err());
}

// ---------- #31 batch CSV ----------

#[test]
fn csv_parser_handles_quotes() {
    let rows = parse_csv("a,b\n\"x,y\",\"q\"\"z\"\n1,2\n");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1], vec!["x,y".to_string(), "q\"z".to_string()]);
}

#[test]
fn batch_runs_rows_and_isolates_errors() {
    let c = calc("compound-interest");
    let input = "Principal P,Annual rate %,Years\n1000,5,10\n2000,10,5\nbad,5,10\n";
    let out = batch_run(&c, input).unwrap();
    assert_eq!(out.ok, 2);
    assert_eq!(out.errors, 1);
    assert!(out.output_csv.contains("1628.89"), "{}", out.output_csv);
    assert!(out.output_csv.contains("ERROR:"), "{}", out.output_csv);
    // Header mismatch is refused with the expected header named.
    let err = batch_run(&c, "A,B,C\n1,2,3\n").unwrap_err();
    assert!(err.contains("Principal P"), "{err}");
    // List calculator batch: header is the list label, rows are values.
    let m = calc("mean");
    let out = batch_run(&m, "Numbers (list)\n1,2,3\n10,20\n").unwrap();
    assert_eq!(out.ok, 2);
    assert!(out.output_csv.contains("Mean: 2"), "{}", out.output_csv);
}

// ---------- #34 regression ----------

#[test]
fn regression_textbook_dataset() {
    // Textbook set: x 1..5, y 2,4,5,4,5 -> m=0.6, b=2.2, r=0.7746, R2=0.6.
    let reg = linear_regression(&[1.0, 2.0, 3.0, 4.0, 5.0], &[2.0, 4.0, 5.0, 4.0, 5.0]).unwrap();
    assert!((reg.slope - 0.6).abs() < 1e-9);
    assert!((reg.intercept - 2.2).abs() < 1e-9);
    assert!((reg.r - 0.7745966692).abs() < 1e-6);
    assert!((reg.r2 - 0.6).abs() < 1e-9);
    assert_eq!(reg.sum_x, 15.0);
    assert_eq!(reg.sum_y, 20.0);
    assert_eq!(reg.sum_xy, 66.0);
    assert_eq!(reg.sum_xx, 55.0);
    let px = regression_predict_x(&reg, 5.0).unwrap();
    assert!((px - 4.6666667).abs() < 1e-6);
    // The registered calculator shows the sums + calibration.
    let out = run_calc(
        "linear-regression",
        &[1.0, 2.0, 2.0, 4.0, 3.0, 5.0, 4.0, 4.0, 5.0, 5.0, 5.0],
    )
    .unwrap();
    assert!(out.contains("Slope m = 0.6"), "{out}");
    assert!(out.contains("R\u{b2} = 0.6"), "{out}");
    assert!(out.contains("predicted x"), "{out}");
    assert!(out.contains("4.666667"), "{out}");
    assert!(linear_regression(&[1.0, 1.0], &[2.0, 3.0]).is_err());
}

// ---------- #47 synonyms ----------

#[test]
fn synonym_search_finds_everyday_terms() {
    assert!(SYNONYMS.len() >= 40, "only {} pairs", SYNONYMS.len());
    let calcs = all_calcs();
    let ids = |q: &str| -> Vec<String> {
        search_ranked(&calcs, q)
            .into_iter()
            .map(|i| calcs[i].id.to_string())
            .collect()
    };
    assert!(
        ids("home loan").contains(&"mortgage-payment".to_string()),
        "{:?}",
        ids("home loan")
    );
    assert!(ids("bmi").contains(&"bmi".to_string()));
    assert!(ids("body mass").contains(&"bmi".to_string()));
    // Ranking: exact name beats a description mention.
    let c = calc("mortgage-payment");
    assert!(search_score(&c, "Mortgage Payment") >= 60);
    assert_eq!(search_score(&c, "zzzz-not-a-term"), 0);
}

// ---------- #46/#48 settings ----------

#[test]
fn settings_round_trip_and_safety() {
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let mut s = Settings {
        display: calcdex::rigour::DisplayFormat {
            mode: calcdex::rigour::DisplayMode::SigFigs,
            n: Some(3),
            rounding: calcdex::rigour::RoundingMode::HalfUp,
        },
        ..Default::default()
    };
    s.toggle_favourite("bmi");
    s.toggle_favourite("age");
    s.note_run("bmi", vec!["70".into(), "175".into()]);
    s.note_run("age", vec!["04/23/1988".into(), "10/25/2026".into()]);
    let text = serialize_settings(&s);
    let back = parse_settings(&text, &valid);
    assert_eq!(back, s);
    // Recent is most-recent-first and capped.
    assert_eq!(back.recent[0], "age");
    // Unknown ids are dropped, junk lines ignored, nothing panics.
    let junk = parse_settings(
        "sig_figs=banana\ndisplay_format=banana\nrounding=banana\nfavourite=no-such-calc\ninputs.ghost=1;;2\n???\ninputs.bmi=70;;175\n",
        &valid,
    );
    assert_eq!(junk.display.mode, calcdex::rigour::DisplayMode::Off);
    assert!(junk.favourites.is_empty());
    assert_eq!(
        junk.remembered.get("bmi").unwrap(),
        &vec!["70".to_string(), "175".to_string()]
    );
    // Empty/corrupt text -> defaults.
    assert_eq!(parse_settings("", &valid), Settings::default());
    assert_eq!(
        parse_settings("\u{0}\u{1}garbage", &valid),
        Settings::default()
    );
}

// ---------- CLI state features (standing rule: full CLI support) ----------

#[test]
fn cli_favourites_and_remembered_inputs_share_the_settings_file() {
    let dir = std::env::temp_dir().join(format!("calcdex-cli-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // v0.9.0: the app's files live in `data\` beside the exe;
    // CALCDEX_DATA_DIR points this run at the temp dir instead,
    // keeping the test fully isolated.
    let exe = env!("CARGO_BIN_EXE_CalcDex");
    let run = |args: &[&str]| {
        std::process::Command::new(exe)
            .args(args)
            .env("CALCDEX_DATA_DIR", &dir)
            .output()
            .unwrap()
    };
    let out = run(&["--favourite", "bmi"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("added to"));
    // A normal CLI run remembers its inputs in the same file.
    let out = run(&["bmi", "70", "175"]);
    assert!(out.status.success());
    let settings_file = dir.join("settings.txt");
    let text = std::fs::read_to_string(&settings_file).unwrap();
    assert!(text.contains("favourite=bmi"), "{text}");
    assert!(text.contains("inputs.bmi=70;;175"), "{text}");
    assert!(text.contains("recent=bmi"), "{text}");
    let out = run(&["--favourites"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("bmi"));
    let out = run(&["--unfavourite", "bmi"]);
    assert!(out.status.success());
    let out = run(&["--favourites"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("No favourites yet"));
    // Unknown ids are refused.
    assert!(!run(&["--favourite", "no-such-calc"]).status.success());
    let _ = std::fs::remove_dir_all(&dir);
}
