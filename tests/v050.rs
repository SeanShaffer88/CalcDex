//! v0.5.0 students rest (50-enhancement program items 12, 13,
//! 14, 15, 17, 18, 19). Engines live in the library
//! (src/students.rs + meta.rs FORMULAS/GLOSSARY); the GUI/CLI are
//! thin callers, and the CLI tests drive the real binary.

use calcdex::meta::{formula_for, glossary_for, FORMULAS, GLOSSARY};
use calcdex::rigour::{DisplayFormat, DisplayMode, HELP_TEXT};
use calcdex::settings::{parse_settings, serialize_settings, Settings};
use calcdex::students::{
    check_answer, deeper_steps, detect_slip, exact_expression, exact_line,
    exact_line_for_expression, first_result_value, parse_answer, plot_csv, practice_eligible,
    practice_problem, practice_problem_seeded, practice_random, sample_function, solution_text,
    solution_text_for_text, substituted_formula, Rational, Rng,
};
use calcdex::{all_calcs, find_calc, run_calc};
use std::collections::BTreeSet;

fn calc(id: &str) -> calcdex::Calc {
    find_calc(id).unwrap_or_else(|| panic!("no calc {id}"))
}

// ---------- #19 registry-wide formula + glossary ----------

#[test]
fn every_calculator_has_formula_and_glossary() {
    let calcs = all_calcs();
    assert_eq!(calcs.len(), 316);
    for c in &calcs {
        let f = formula_for(c);
        assert!(!f.trim().is_empty(), "{} has no formula", c.id);
        let g = glossary_for(c);
        assert!(!g.is_empty(), "{} has no glossary entries", c.id);
        for (sym, meaning) in g {
            assert!(!sym.trim().is_empty(), "{} has an empty symbol", c.id);
            assert!(
                meaning.trim().len() >= 8,
                "{} symbol {sym} has a too-short meaning: {meaning}",
                c.id
            );
        }
    }
    // No stale rows for calculators that do not exist.
    assert_eq!(FORMULAS.len(), calcs.len());
    assert_eq!(GLOSSARY.len(), calcs.len());
    for (id, _) in FORMULAS {
        assert!(find_calc(id).is_some(), "FORMULAS row for unknown {id}");
    }
    for (id, _) in GLOSSARY {
        assert!(find_calc(id).is_some(), "GLOSSARY row for unknown {id}");
    }
}

#[test]
fn glossary_pairs_are_in_input_order_for_substitution() {
    // The deeper-steps substitution pairs glossary[i] with
    // input i; circle's first symbol must be the radius.
    let c = calc("circle");
    assert_eq!(glossary_for(&c)[0].0, "r");
    let sub = substituted_formula(&c, &[5.0]);
    assert!(sub.contains("5^2"), "{sub}");
    assert!(sub.contains("2 x pi x 5"), "{sub}");
}

// ---------- #12 deeper steps ----------

#[test]
fn deeper_steps_have_the_four_labelled_stages() {
    for (id, args) in [
        ("circle", vec![5.0]),                          // Math
        ("compound-interest", vec![1000.0, 5.0, 10.0]), // Finance
        ("bmi", vec![70.0, 175.0]),                     // Health
        ("force", vec![10.0, 9.8]),                     // Physics
        ("triangle-area", vec![10.0, 4.0]),             // Math
    ] {
        let c = calc(id);
        let out = run_calc(id, &args).unwrap();
        let deep = deeper_steps(&c, &args, &out);
        assert!(deep.contains("Step 1 — the formula:"), "{id}: {deep}");
        assert!(deep.contains(formula_for(&c)), "{id}: {deep}");
        assert!(
            deep.contains("Step 2 — the formula with your values in:"),
            "{id}"
        );
        assert!(deep.contains("Step 3 — the working:"), "{id}");
        // Stage 3 is the calculator's own output, verbatim.
        assert!(deep.contains(&out), "{id}: stage 3 must be verbatim");
        assert!(deep.contains("Step 4 — the result:"), "{id}");
        assert!(
            deep.contains(out.lines().next().unwrap()),
            "{id}: stage 4 is the headline"
        );
    }
    // The substituted stage really carries the user's values.
    let c = calc("circle");
    let out = run_calc("circle", &[5.0]).unwrap();
    let deep = deeper_steps(&c, &[5.0], &out);
    assert!(deep.contains("pi x 5^2"), "{deep}");
}

// ---------- #14 rational type + exact expression evaluation ----------

#[test]
fn rational_arithmetic_is_exact() {
    let third = Rational::new(1, 3).unwrap();
    let sixth = Rational::new(1, 6).unwrap();
    assert_eq!(third.add(sixth).unwrap().show(), "1/2");
    assert_eq!(Rational::new(2, 4).unwrap().show(), "1/2"); // normalised
    assert_eq!(Rational::new(6, -8).unwrap().show(), "-3/4"); // sign normalised
    assert_eq!(Rational::int(2).pow(10).unwrap().show(), "1024");
    assert_eq!(Rational::new(2, 3).unwrap().pow(2).unwrap().show(), "4/9");
    assert_eq!(
        Rational::new(3, 4)
            .unwrap()
            .div(Rational::new(9, 8).unwrap())
            .unwrap()
            .show(),
        "2/3"
    );
    assert!(Rational::new(1, 0).is_none());
    assert_eq!(Rational::from_decimal_str("1.25").unwrap().show(), "5/4");
    assert_eq!(Rational::from_decimal_str("-0.5").unwrap().show(), "-1/2");
}

#[test]
fn exact_expression_evaluation_and_fallback() {
    use calcdex::rigour::AngleMode;
    let o = exact_expression("1/3+1/6", AngleMode::Degrees).unwrap();
    assert_eq!(o.exact.as_ref().unwrap().show(), "1/2");
    let o = exact_expression("2^10", AngleMode::Degrees).unwrap();
    assert_eq!(o.exact.as_ref().unwrap().show(), "1024");
    let o = exact_expression("(2/3)^2", AngleMode::Degrees).unwrap();
    assert_eq!(o.exact.as_ref().unwrap().show(), "4/9");
    // sqrt is outside the exact scope: decimal result, no exact form.
    let o = exact_expression("sqrt(2)", AngleMode::Degrees).unwrap();
    assert!(o.exact.is_none());
    assert!((o.decimal - std::f64::consts::SQRT_2).abs() < 1e-12);
    let line = exact_line_for_expression("sqrt(2)", AngleMode::Degrees);
    assert!(line.contains("no exact rational form"), "{line}");
    let line = exact_line_for_expression("1/3+1/6", AngleMode::Degrees);
    assert!(line.contains("1/2"), "{line}");
    // x is the plot variable, never an exact-evaluable constant.
    assert!(exact_expression("x+1", AngleMode::Degrees).is_err());
}

#[test]
fn exact_lines_for_geometry_and_fractions() {
    // Pi family shows the exact form alongside the decimal.
    let line = exact_line(&calc("circle"), &[5.0]);
    assert!(line.contains("25*pi"), "{line}");
    assert!(line.contains("10*pi"), "{line}");
    let line = exact_line(&calc("circumference"), &[10.0]);
    assert!(line.contains("10*pi"), "{line}");
    let line = exact_line(&calc("sphere"), &[3.0]);
    assert!(line.contains("36*pi"), "{line}"); // 4/3*pi*27 = 36*pi
                                               // Fraction calculators are labelled exact.
    let line = exact_line(&calc("fraction-add"), &[1.0, 3.0, 1.0, 6.0]);
    assert!(line.to_lowercase().contains("exact"), "{line}");
    // Everything else falls back honestly.
    let line = exact_line(&calc("bmi"), &[70.0, 175.0]);
    assert!(line.contains("decimal"), "{line}");
    // Exact mode joins the display-format setting cleanly.
    let f = DisplayFormat::parse("exact").unwrap();
    assert_eq!(f.mode, DisplayMode::Exact);
    assert_eq!(f.as_setting(), "exact");
}

// ---------- #13 check my answer ----------

#[test]
fn parse_answer_forms() {
    assert_eq!(parse_answer("3/4").unwrap().to_f64(), 0.75);
    assert_eq!(parse_answer("1 1/2").unwrap().to_f64(), 1.5);
    assert_eq!(parse_answer("-1 1/2").unwrap().to_f64(), -1.5);
    assert_eq!(parse_answer("42").unwrap().to_f64(), 42.0);
    assert_eq!(parse_answer(" 0.5 ").unwrap().to_f64(), 0.5);
    assert!(parse_answer("abc").is_err());
    assert!(parse_answer("1/0").is_err());
    assert!(parse_answer("").is_err());
}

#[test]
fn slip_detectors() {
    let e = 50.0;
    assert!(detect_slip(5000.0, e).unwrap().contains("100x"));
    assert!(detect_slip(0.5, e).unwrap().contains("divided by 100"));
    assert!(detect_slip(200.0, e).unwrap().contains("4x"));
    assert!(detect_slip(12.5, e).unwrap().contains("quarter"));
    assert!(detect_slip(100.0, e).unwrap().contains("2x"));
    assert!(detect_slip(25.0, e).unwrap().contains("half"));
    assert!(detect_slip(2500.0, e).unwrap().contains("squared"));
    assert!(detect_slip(7.0710678, e).unwrap().contains("square root"));
    assert!(detect_slip(-50.0, e).unwrap().contains("sign flipped"));
    // A rounded student value still matches its slip (0.5% band).
    assert!(detect_slip(314.16, 78.539816).unwrap().contains("4x"));
    // Unrelated wrong answers name no slip; right answers never reach here.
    assert!(detect_slip(63.0, e).is_none());
    assert!(detect_slip(50.0, e).is_none());
}

#[test]
fn check_answer_numeric_tolerance() {
    let c = calc("circle");
    let out = run_calc("circle", &[5.0]).unwrap();
    // 0.4% off: correct.
    let o = check_answer(&c, &out, "78.85").unwrap();
    assert!(o.correct, "{}", o.message);
    assert!(o.message.contains("0.5%"), "{}", o.message);
    // 5% off: not yet, with the slip named (x4 via diameter is
    // 314.16; plain 5% off names no slip).
    let o = check_answer(&c, &out, "82.5").unwrap();
    assert!(!o.correct);
    assert!(o.message.contains("Not yet"), "{}", o.message);
    assert!(o.message.contains("Compare your working"), "{}", o.message);
    let o = check_answer(&c, &out, "314.16").unwrap();
    assert!(!o.correct);
    assert!(o.message.contains("Likely slip"), "{}", o.message);
    assert!(o.message.contains("4x"), "{}", o.message);
}

#[test]
fn check_answer_fractions_are_exact() {
    let c = calc("fraction-add");
    let out = run_calc("fraction-add", &[1.0, 3.0, 1.0, 6.0]).unwrap();
    let o = check_answer(&c, &out, "1/2").unwrap();
    assert!(o.correct && o.exact, "{}", o.message);
    assert!(o.message.contains("exactly right"), "{}", o.message);
    // An unsimplified equivalent fraction is still exactly right.
    let o = check_answer(&c, &out, "2/4").unwrap();
    assert!(o.correct, "{}", o.message);
    // Close-but-not-equal is NOT correct for fractions.
    let o = check_answer(&c, &out, "0.501").unwrap();
    assert!(!o.correct, "{}", o.message);
    // And a decimal answer equal to the fraction is exactly right too.
    let o = check_answer(&c, &out, "0.5").unwrap();
    assert!(o.correct, "{}", o.message);
}

#[test]
fn first_result_value_picks_the_first_assignment() {
    let out = "Area = 78.539816. Circumference = 31.415927. Diameter = 10.";
    assert_eq!(first_result_value(out), Some(78.539816));
    let out = run_calc("percent-of", &[25.0, 200.0]).unwrap();
    assert_eq!(first_result_value(&out), Some(50.0));
}

// ---------- #15 practice mode ----------

#[test]
fn rng_is_deterministic() {
    let mut a = Rng::new(42);
    let mut b = Rng::new(42);
    let mut c = Rng::new(43);
    let sa: Vec<u64> = (0..10).map(|_| a.next_u64()).collect();
    let sb: Vec<u64> = (0..10).map(|_| b.next_u64()).collect();
    let sc: Vec<u64> = (0..10).map(|_| c.next_u64()).collect();
    assert_eq!(sa, sb, "same seed must give the same sequence");
    assert_ne!(sa, sc, "different seeds must differ");
    let mut r = Rng::new(1);
    for _ in 0..100 {
        let v = r.range(2.0, 3.0);
        assert!((2.0..3.0).contains(&v), "{v}");
    }
}

#[test]
fn practice_problems_are_reproducible_and_runnable() {
    let c = calc("circle");
    let p1 = practice_problem_seeded(&c, 7).unwrap();
    let p2 = practice_problem_seeded(&c, 7).unwrap();
    assert_eq!(p1, p2, "same seed + calculator = same problem");
    assert!(p1.question.contains("Circle"), "{}", p1.question);
    // The generated inputs actually run.
    run_calc("circle", &p1.args).unwrap();
    // A stream from one RNG also stays runnable.
    let mut rng = Rng::new(99);
    for _ in 0..10 {
        let p = practice_problem(&c, &mut rng).unwrap();
        run_calc("circle", &p.args).unwrap();
    }
    // Random problems come from the eligible pool only.
    let calcs = all_calcs();
    let mut rng = Rng::new(5);
    for _ in 0..10 {
        let (_, p) = practice_random(&calcs, &mut rng).unwrap();
        let cc = calc(&p.calc_id);
        assert!(practice_eligible(&cc), "{} must be eligible", p.calc_id);
        run_calc(&p.calc_id, &p.args).unwrap();
    }
}

#[test]
fn practice_exclusions_are_the_stated_ones() {
    for id in [
        "mean",
        "convert-length",
        "age",
        "weighted-average",
        "angle-convert",
    ] {
        assert!(!practice_eligible(&calc(id)), "{id} must be excluded");
    }
    for id in ["circle", "bmi", "compound-interest", "fraction-add"] {
        assert!(practice_eligible(&calc(id)), "{id} must be eligible");
    }
}

// ---------- #17 plot sampler ----------

#[test]
fn sampler_points_and_gaps() {
    use calcdex::rigour::AngleMode;
    let pts = sample_function("x^2", -2.0, 2.0, 4, AngleMode::Degrees).unwrap();
    assert_eq!(pts.len(), 5);
    assert_eq!(pts[0], (-2.0, Some(4.0)));
    assert_eq!(pts[2], (0.0, Some(0.0)));
    assert_eq!(pts[4], (2.0, Some(4.0)));
    // 1/x is undefined at 0: the middle sample is a gap.
    let pts = sample_function("1/x", -1.0, 1.0, 2, AngleMode::Degrees).unwrap();
    assert_eq!(pts[1], (0.0, None));
    // log(x) is undefined for x <= 0.
    let pts = sample_function("log10(x)", -1.0, 1.0, 2, AngleMode::Degrees).unwrap();
    assert_eq!(pts[0].1, None);
    assert_eq!(pts[2], (1.0, Some(0.0)));
    // The CSV form: header + a blank y at the gap.
    let csv = plot_csv("1/x", -1.0, 1.0, 2, AngleMode::Degrees).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "x,y");
    assert_eq!(lines[2], "0,");
    // Trig respects the angle mode: sin peaks at 90 degrees.
    let pts = sample_function("sin(x)", 0.0, 180.0, 2, AngleMode::Degrees).unwrap();
    assert_eq!(pts[1].0, 90.0);
    assert!((pts[1].1.unwrap() - 1.0).abs() < 1e-9);
}

// ---------- #18 export worked solution ----------

#[test]
fn solution_text_contains_name_formula_inputs_result() {
    let c = calc("circle");
    let out = run_calc("circle", &[5.0]).unwrap();
    let text = solution_text(&c, &[5.0], &out, false);
    assert!(
        text.contains("Circle (Area, Circumference, Diameter)"),
        "{text}"
    );
    assert!(text.contains("Formula: A = pi x r^2"), "{text}");
    assert!(text.contains("Radius r: 5"), "{text}");
    assert!(text.contains("78.539816"), "{text}");
    // Deeper export stages the working.
    let text = solution_text(&c, &[5.0], &out, true);
    assert!(text.contains("Step 1 — the formula:"), "{text}");
    // Text-calculator solutions work too.
    let t = solution_text_for_text("Expression Calculator", "2*(3+4)^2", "Result: 98.");
    assert!(t.contains("Expression Calculator"), "{t}");
    assert!(t.contains("2*(3+4)^2"), "{t}");
    assert!(t.contains("Result: 98."), "{t}");
}

// ---------- settings: steps key round-trips, old files load ----------

#[test]
fn settings_steps_key_round_trip_and_legacy_default() {
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let s = Settings {
        steps_deeper: true,
        ..Default::default()
    };
    let text = serialize_settings(&s);
    assert!(text.contains("steps=deeper"), "{text}");
    assert_eq!(parse_settings(&text, &valid), s);
    // A v0.4.0 settings file (no steps line) loads as Standard.
    let legacy = parse_settings("display_format=off\n", &valid);
    assert!(!legacy.steps_deeper);
    // And the exact format survives the settings round trip.
    let s2 = Settings {
        display: DisplayFormat::parse("exact").unwrap(),
        ..Default::default()
    };
    let text = serialize_settings(&s2);
    assert!(text.contains("display_format=exact"), "{text}");
    assert_eq!(
        parse_settings(&text, &valid).display.mode,
        DisplayMode::Exact
    );
}

#[test]
fn help_text_covers_the_students_tools() {
    assert!(HELP_TEXT.contains("CHECK MY ANSWER"));
    assert!(HELP_TEXT.contains("PRACTICE"));
    assert!(HELP_TEXT.contains("Deeper"));
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
fn cli_v050_end_to_end() {
    let dir = std::env::temp_dir().join(format!("calcdex-v050-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stdout = |o: &std::process::Output| String::from_utf8_lossy(&o.stdout).to_string();

    // #19: --describe prints formula + glossary.
    let out = cli(&["--describe", "circle"], &dir);
    assert!(out.status.success());
    let t = stdout(&out);
    assert!(t.contains("Formula: A = pi x r^2"), "{t}");
    assert!(t.contains("r = the radius"), "{t}");

    // #13: --check with a slip.
    let out = cli(&["circle", "5", "--check", "314.16"], &dir);
    assert!(out.status.success());
    let t = stdout(&out);
    assert!(t.contains("Not yet"), "{t}");
    assert!(t.contains("Likely slip"), "{t}");
    let out = cli(&["circle", "5", "--check", "78.54"], &dir);
    assert!(stdout(&out).contains("Correct"), "{}", stdout(&out));

    // #12: --steps deeper stages.
    let out = cli(&["circle", "5", "--steps", "deeper"], &dir);
    let t = stdout(&out);
    assert!(t.contains("Step 1 — the formula:"), "{t}");
    assert!(
        t.contains("Step 2 — the formula with your values in:"),
        "{t}"
    );
    assert!(t.contains("pi x 5^2"), "{t}");

    // #14: --format exact on geometry + expression.
    let out = cli(&["circle", "5", "--format", "exact"], &dir);
    assert!(stdout(&out).contains("25*pi"), "{}", stdout(&out));
    let out = cli(&["expr", "1/3+1/6", "--format", "exact"], &dir);
    assert!(stdout(&out).contains("= 1/2"), "{}", stdout(&out));
    let out = cli(&["expr", "sqrt(2)", "--format", "exact"], &dir);
    assert!(
        stdout(&out).contains("no exact rational form"),
        "{}",
        stdout(&out)
    );

    // #15: practice from a seed, then checked with the same seed.
    let p = practice_problem_seeded(&calc("circle"), 7).unwrap();
    let out = cli(&["circle", "--practice", "--seed", "7"], &dir);
    let t = stdout(&out);
    assert!(t.contains("Practice"), "{t}");
    assert!(t.contains("Work out the first answer"), "{t}");
    assert!(
        !t.contains("Correct"),
        "the problem alone must not leak a verdict: {t}"
    );
    let expected = first_result_value(&run_calc("circle", &p.args).unwrap()).unwrap();
    let out = cli(
        &[
            "circle",
            "--practice",
            "--seed",
            "7",
            "--answer",
            &calcdex::n(expected),
        ],
        &dir,
    );
    assert!(stdout(&out).contains("Correct"), "{}", stdout(&out));
    // An excluded calculator refuses practice with the reason.
    let out = cli(&["mean", "--practice", "--seed", "7"], &dir);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not used for practice"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // #17: --plot prints CSV with gaps.
    let out = cli(&["expr", "x^2", "--plot", "-2", "2"], &dir);
    let t = stdout(&out);
    assert!(t.starts_with("x,y"), "{t}");
    assert!(t.contains("0,0"), "{t}");
    let out = cli(&["expr", "1/x", "--plot", "-1", "1"], &dir);
    assert!(stdout(&out).contains("\n0,\n"), "{}", stdout(&out));

    // #18: --export writes the worked solution.
    let path = dir.join("sol.txt");
    let out = cli(&["circle", "5", "--export", path.to_str().unwrap()], &dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Formula: A = pi x r^2"), "{text}");
    assert!(text.contains("78.539816"), "{text}");
}
