//! v0.6.0 teachers batch (items #21–#30): problem sets (the
//! shared foundation), seeded generation, worksheets, answer
//! keys, bulk marking, formula sheets, the saved examples
//! library, course packs, demonstration-mode masking,
//! projection settings, and the CLI end-to-end.

use calcdex::meta::{pack_members, packs_for, COURSE_PACKS, COURSE_PACK_NAMES};
use calcdex::settings::{parse_settings, serialize_settings, Settings};
use calcdex::teachers::{
    answer_key_text, formula_sheet_text, generate_set, mark_set, mask_result_text, parse_examples,
    parse_set, problem_line, serialize_examples, set_add, set_listing, set_to_text, upsert_example,
    worksheet_text, ProblemSet, SavedExample, SetEntry,
};
use calcdex::{all_calcs, find_calc, run_calc};
use std::collections::BTreeSet;

fn sample_set() -> ProblemSet {
    let mut set = ProblemSet::new("Fractions and circles — week 4");
    set_add(
        &mut set,
        "fraction-add",
        &["1".into(), "2".into(), "1".into(), "4".into()],
    )
    .unwrap();
    set_add(&mut set, "circle", &["5".into()]).unwrap();
    set_add(&mut set, "percent-of", &["25".into(), "80".into()]).unwrap();
    set
}

// ---------- set format: round trip + validation ----------

#[test]
fn set_format_round_trip() {
    let set = sample_set();
    let text = set_to_text(&set);
    assert!(text.starts_with("# CalcDex problem set\n"));
    assert!(text.contains("# Name: Fractions and circles — week 4"));
    assert!(text.contains("circle | 5"));
    let back = parse_set(&text).unwrap();
    assert_eq!(back.entries, set.entries);
    assert_eq!(back.name, set.name);
    // And writing the parsed set reproduces the same file text.
    assert_eq!(set_to_text(&back), text);
}

#[test]
fn set_load_validates_entries() {
    // Unknown calculator.
    assert!(parse_set("# CalcDex problem set\nnope | 1\n").is_err());
    // Inputs that do not run (circle with a negative radius runs?
    // no — division by zero in percent-of: 25% of nothing is fine,
    // but fraction with denominator 0 must be refused).
    assert!(parse_set("# CalcDex problem set\nfraction-add | 1 | 0 | 1 | 2\n").is_err());
    // The text calculator cannot be a set problem.
    assert!(parse_set("# CalcDex problem set\nexpression-calculator | 2+2\n").is_err());
    // Not a set file at all.
    assert!(parse_set("just some words\n").is_err());
    assert!(parse_set("").is_err());
    // But a headed set with no problems yet is valid — it is what
    // `set new` creates, ready for `set add`.
    let empty = parse_set("# CalcDex problem set\n").unwrap();
    assert!(empty.entries.is_empty());
}

#[test]
fn set_file_contains_no_answers() {
    // Item #30: the exported set is the assignment — it must not
    // contain any result values.
    let set = sample_set();
    let text = set_to_text(&set);
    // Circle r=5 -> area 78.54; 25% of 80 -> 20 (as a result line).
    let (_, _, circle_out) = calcdex::teachers::run_entry(&set.entries[1]).unwrap();
    for line in circle_out.lines() {
        assert!(
            !text.contains(line),
            "set file leaked a result line: {line}\n{text}"
        );
    }
    assert!(!text.contains("78.5398"), "{text}");
    assert!(!text.contains("Answer"), "{text}");
}

// ---------- worksheet + answer key (#27 / #22) ----------

#[test]
fn worksheet_has_problems_space_and_no_answers() {
    let set = sample_set();
    let ws = worksheet_text(&set);
    assert!(ws.contains("Name: ______________________"), "{ws}");
    assert!(ws.contains("Date: ____________"), "{ws}");
    assert!(ws.contains("1. Add Two Fractions with"), "{ws}");
    assert!(
        ws.contains("2. Circle (Area, Circumference, Diameter) with"),
        "{ws}"
    );
    assert!(ws.contains("Answer: ______________________"), "{ws}");
    // Working space under every problem.
    assert_eq!(
        ws.matches("________________________________________________________________")
            .count(),
        9
    );
    // No answers anywhere: no result text, no key-only wording.
    assert!(!ws.contains("78.5398"), "{ws}");
    assert!(!ws.contains("ANSWER KEY"), "{ws}");
    // The listing (import view) also shows problems only.
    let listing = set_listing(&set);
    assert!(listing.contains("3 problem(s)"), "{listing}");
    assert!(!listing.contains("78.5398"), "{listing}");
}

#[test]
fn answer_key_has_answers_and_one_line_working() {
    let set = sample_set();
    let key = answer_key_text(&set);
    assert!(key.starts_with("ANSWER KEY — TEACHER COPY"), "{key}");
    // Same numbering as the worksheet.
    assert!(key.contains("1. Add Two Fractions with"), "{key}");
    assert!(
        key.contains("2. Circle (Area, Circumference, Diameter) with"),
        "{key}"
    );
    // Answers present: fraction-add 1/2 + 1/4 = 3/4.
    assert!(key.contains("Answer: "), "{key}");
    assert!(key.contains("3/4"), "{key}");
    // Circle area shows in the key.
    assert!(key.contains("78.5398"), "{key}");
    // One-line working: the substituted formula line.
    assert!(key.contains("Working: "), "{key}");
    let working_lines: Vec<&str> = key.lines().filter(|l| l.contains("Working: ")).collect();
    assert_eq!(working_lines.len(), 3);
    for l in working_lines {
        assert!(!l.trim().is_empty());
    }
}

// ---------- seeded generation (#23) ----------

#[test]
fn seeded_generation_is_deterministic() {
    let a = generate_set(&["circle", "percent-of"], 7, 6, "Class set").unwrap();
    let b = generate_set(&["circle", "percent-of"], 7, 6, "Class set").unwrap();
    assert_eq!(set_to_text(&a), set_to_text(&b));
    assert_eq!(a.entries.len(), 6);
    assert_eq!(a.seed_note, Some(7));
    // Round-robin over the given calculators.
    assert_eq!(a.entries[0].calc_id, "circle");
    assert_eq!(a.entries[1].calc_id, "percent-of");
    // A different seed makes a different set.
    let c = generate_set(&["circle", "percent-of"], 8, 6, "Class set").unwrap();
    assert_ne!(set_to_text(&a), set_to_text(&c));
    // Every generated entry actually runs.
    for e in &a.entries {
        assert!(calcdex::teachers::run_entry(e).is_ok());
    }
    // Ineligible calculators are refused by name.
    assert!(generate_set(&["convert-length"], 7, 3, "x").is_err());
    assert!(generate_set(&["circle"], 7, 0, "x").is_err());
    assert!(generate_set(&["nope"], 7, 3, "x").is_err());
}

// ---------- bulk marking (#26) ----------

#[test]
fn marking_fixture_totals_are_exact() {
    let set = sample_set();
    // Q1: fraction-add 1/2 + 1/4 = 3/4 (exact). Q2: circle r=5,
    // first result value = area 78.5398... (0.5% tolerance).
    // Q3: percent-of 25% of 80 = 20.
    let csv = "Name,Q1,Q2,Q3\n\
               Ada,3/4,78.54,20\n\
               Ben,0.75,314.16,25\n\
               Cy,,100,\n";
    let outcome = mark_set(&set, csv).unwrap();
    assert_eq!(outcome.rows.len(), 3);
    // Ada: all three right. Fraction as fraction; decimal works too.
    assert_eq!(outcome.rows[0].marks, vec![true, true, true]);
    assert_eq!(outcome.rows[0].score, 3);
    // Ben: 0.75 == 3/4 right; 314.16 is the 4x slip (diameter used
    // as radius squared area) — wrong; 25 wrong.
    assert_eq!(outcome.rows[1].marks, vec![true, false, false]);
    assert_eq!(outcome.rows[1].score, 1);
    // Cy: blank fraction wrong, 100 wrong, blank percent wrong.
    assert_eq!(outcome.rows[2].marks, vec![false, false, false]);
    assert_eq!(outcome.rows[2].score, 0);
    // Summary lines exact.
    assert_eq!(
        outcome.summary,
        vec![
            "Ada: 3/3".to_string(),
            "Ben: 1/3".to_string(),
            "Cy: 0/3".to_string()
        ]
    );
    // Output CSV shape: header + one row per student, right/wrong
    // per problem + score column.
    let lines: Vec<&str> = outcome.output_csv.lines().collect();
    assert_eq!(lines[0], "Student,Q1,Q2,Q3,Score");
    assert_eq!(lines[1], "Ada,right,right,right,3/3");
    assert_eq!(lines[2], "Ben,right,wrong,wrong,1/3");
    assert_eq!(lines[3], "Cy,wrong,wrong,wrong,0/3");
    // No rows at all is an error, not an empty success.
    assert!(mark_set(&set, "").is_err());
}

// ---------- formula sheets (#29) ----------

#[test]
fn formula_sheet_covers_the_category() {
    let calcs = all_calcs();
    let math_count = calcs.iter().filter(|c| c.category == "Math").count();
    let sheet = formula_sheet_text(&calcs, Some("Math")).unwrap();
    let formula_lines = sheet.matches("  Formula: ").count();
    assert!(
        formula_lines >= math_count,
        "{formula_lines} formula lines for {math_count} Math calculators"
    );
    assert!(
        sheet.contains("Circle (Area, Circumference, Diameter) (circle)"),
        "{sheet}"
    );
    assert!(sheet.contains("r = "), "{sheet}");
    // All-categories sheet contains every calculator's block.
    let all = formula_sheet_text(&calcs, None).unwrap();
    assert_eq!(all.matches("  Formula: ").count(), calcs.len());
    assert!(all.contains("== Finance =="), "category headings");
    // Unknown category is a clear error.
    assert!(formula_sheet_text(&calcs, Some("Alchemy")).is_err());
}

// ---------- course packs (#24) ----------

#[test]
fn packs_are_well_formed_and_sized() {
    let calcs = all_calcs();
    assert_eq!(COURSE_PACK_NAMES, ["GCSE", "A-Level", "AP", "Undergrad"]);
    let valid: BTreeSet<&str> = calcs.iter().map(|c| c.id).collect();
    for (name, ids) in COURSE_PACKS {
        assert!(COURSE_PACK_NAMES.contains(name));
        assert!(
            ids.len() >= 20,
            "pack {name} has only {} calculators",
            ids.len()
        );
        for id in *ids {
            assert!(valid.contains(id), "pack {name} names unknown id {id}");
        }
        let members = pack_members(&calcs, name);
        assert_eq!(members.len(), ids.len());
    }
    // Spot checks from the tagging rule: core school topics in
    // GCSE; logs are A-Level/AP/Undergrad, not GCSE; kitchen
    // converters are in no pack.
    let circle = find_calc("circle").unwrap();
    let cp = packs_for(&circle);
    assert!(cp.contains(&"GCSE") && cp.contains(&"A-Level"));
    let log = find_calc("log").unwrap();
    let lp = packs_for(&log);
    assert!(!lp.contains(&"GCSE"), "log packs: {lp:?}");
    assert!(lp.contains(&"A-Level") && lp.contains(&"Undergrad"));
    let flour = find_calc("grams-cups-flour").unwrap();
    assert!(packs_for(&flour).is_empty());
}

// ---------- saved examples library (#28) ----------

#[test]
fn examples_round_trip_with_escapes() {
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let mut list: Vec<SavedExample> = Vec::new();
    upsert_example(
        &mut list,
        SavedExample {
            title: "Circle | demo \\ tricky".to_string(),
            calc_id: "circle".to_string(),
            note: "Period 3; watch the radius/diameter slip".to_string(),
            inputs: vec!["5".to_string()],
        },
    );
    upsert_example(
        &mut list,
        SavedExample {
            title: "Age demo".to_string(),
            calc_id: "age".to_string(),
            note: String::new(),
            inputs: vec!["04/23/1988".to_string(), "10/25/2026".to_string()],
        },
    );
    let text = serialize_examples(&list);
    let back = parse_examples(&text, &valid);
    assert_eq!(back, list);
    // Re-saving a title replaces it.
    upsert_example(
        &mut list,
        SavedExample {
            title: "Age demo".to_string(),
            calc_id: "age".to_string(),
            note: "updated".to_string(),
            inputs: vec!["01/01/2000".to_string(), "01/01/2020".to_string()],
        },
    );
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].note, "updated");
    // Unknown ids are dropped on parse, never fatal.
    let bad = "# CalcDex saved examples\nGhost | nope | x | 1\n";
    assert!(parse_examples(bad, &valid).is_empty());
}

// ---------- demonstration masking (#25) ----------

#[test]
fn demo_masking_hides_results_keeps_inputs() {
    let calc = find_calc("circle").unwrap();
    let out = run_calc("circle", &[5.0]).unwrap();
    assert!(out.contains("78.5398"), "{out}");
    let masked = mask_result_text(&out, &[5.0]);
    assert!(!masked.contains("78.5398"), "{masked}");
    assert!(!masked.contains("31.4159"), "{masked}");
    assert!(masked.contains("•••"), "{masked}");
    // Numbers equal to an input stay readable: compound
    // interest's steps echo the principal 1000, while its result
    // 1628.89 is masked.
    let out2 = run_calc("compound-interest", &[1000.0, 5.0, 10.0]).unwrap();
    assert!(out2.contains("1628.89"), "{out2}");
    let masked2 = mask_result_text(&out2, &[1000.0, 5.0, 10.0]);
    assert!(!masked2.contains("1628.89"), "{masked2}");
    assert!(masked2.contains("1000"), "{masked2}");
    // Nothing to mask: text without numbers passes through.
    assert_eq!(
        mask_result_text("No numbers here.", &[1.0]),
        "No numbers here."
    );
    // problem_line still states the problem openly.
    let line = problem_line(&calc, &["5".to_string()]);
    assert!(line.contains("Radius = 5") || line.contains("5"), "{line}");
}

// ---------- projection setting (#21) ----------

#[test]
fn projection_setting_round_trips_and_old_files_load() {
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let s = Settings {
        projection: true,
        ..Settings::default()
    };
    let text = serialize_settings(&s);
    assert!(text.contains("projection=on"), "{text}");
    assert!(parse_settings(&text, &valid).projection);
    // A v0.5.0 settings file (no projection key) loads with the
    // default OFF — backward compatible.
    let old = "# CalcDex settings\nsteps=deeper\nfavourite=circle\n";
    let parsed = parse_settings(old, &valid);
    assert!(!parsed.projection);
    assert!(parsed.steps_deeper);
    // And the legacy sig_figs line still loads too.
    let legacy = "sig_figs=3\n";
    assert_eq!(
        parse_settings(legacy, &valid).display.mode,
        calcdex::rigour::DisplayMode::SigFigs
    );
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
fn cli_v060_end_to_end() {
    let dir = std::env::temp_dir().join(format!("calcdex-v060-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stdout = |o: &std::process::Output| String::from_utf8_lossy(&o.stdout).to_string();
    let set = dir.join("week4.txt");
    let set_s = set.to_string_lossy().to_string();

    // set new / add / open (#22, #30)
    let out = cli(&["set", "new", &set_s, "Week", "4"], &dir);
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains("Created empty problem set 'Week 4'"));
    let out = cli(&["set", "add", &set_s, "circle", "5"], &dir);
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains("Added problem 1"));
    let out = cli(&["set", "add", &set_s, "percent-of", "25", "80"], &dir);
    assert!(out.status.success());
    let out = cli(&["set", "open", &set_s], &dir);
    let t = stdout(&out);
    assert!(t.contains("2 problem(s)"), "{t}");
    assert!(t.contains("Circle"), "{t}");
    assert!(!t.contains("78.5398"), "open must not show answers: {t}");

    // set worksheet (#27): problems + space, no answers.
    let ws = dir.join("ws.txt");
    let out = cli(&["set", "worksheet", &set_s, &ws.to_string_lossy()], &dir);
    assert!(out.status.success(), "{}", stdout(&out));
    let wst = std::fs::read_to_string(&ws).unwrap();
    assert!(wst.contains("Name: ______________________"));
    assert!(!wst.contains("78.5398"), "{wst}");

    // set key (#22): headed teacher copy with answers.
    let key = dir.join("key.txt");
    let out = cli(&["set", "key", &set_s, &key.to_string_lossy()], &dir);
    assert!(out.status.success(), "{}", stdout(&out));
    let keyt = std::fs::read_to_string(&key).unwrap();
    assert!(keyt.contains("ANSWER KEY — TEACHER COPY"), "{keyt}");
    assert!(keyt.contains("78.5398"), "{keyt}");
    // Overwrite needs --force.
    let out = cli(&["set", "key", &set_s, &key.to_string_lossy()], &dir);
    assert!(!out.status.success());
    let out = cli(
        &["--force", "set", "key", &set_s, &key.to_string_lossy()],
        &dir,
    );
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(std::path::Path::new(&format!("{}.bak", key.display())).exists());

    // set generate (#23): deterministic file bytes.
    let gen = dir.join("gen.txt");
    let gen_s = gen.to_string_lossy().to_string();
    let out = cli(
        &[
            "set", "generate", &gen_s, "--seed", "7", "--count", "4", "circle",
        ],
        &dir,
    );
    assert!(out.status.success(), "{}", stdout(&out));
    let gen_bytes = std::fs::read(&gen).unwrap();
    let gen_lib = generate_set(&["circle"], 7, 4, "Problem set from seed 7").unwrap();
    assert_eq!(gen_bytes, set_to_text(&gen_lib).into_bytes());

    // --mark (#26): fixture through the binary.
    let answers = dir.join("answers.csv");
    std::fs::write(&answers, "Name,Q1,Q2\nAda,78.54,20\nBen,1,2\n").unwrap();
    let out = cli(&["--mark", &set_s, &answers.to_string_lossy()], &dir);
    let t = stdout(&out);
    assert!(out.status.success(), "{t}");
    assert!(t.contains("Ada: 2/2"), "{t}");
    assert!(t.contains("Ben: 0/2"), "{t}");
    let marked = std::fs::read_to_string(format!("{}.marked.csv", answers.display())).unwrap();
    assert!(marked.contains("Ada,right,right,2/2"), "{marked}");

    // --pack (#24).
    let out = cli(&["--pack", "GCSE", "--list"], &dir);
    let t = stdout(&out);
    assert!(out.status.success(), "{t}");
    assert!(t.contains("circle"), "{t}");
    assert!(!t.contains("grams-cups-flour"), "{t}");
    let out = cli(&["--pack", "Alchemy"], &dir);
    assert!(!out.status.success());

    // --describe names the packs.
    let out = cli(&["--describe", "circle"], &dir);
    assert!(stdout(&out).contains("Course packs: "), "{}", stdout(&out));
    assert!(stdout(&out).contains("GCSE"));

    // --projection (#21) writes the shared setting.
    let out = cli(&["--projection", "on"], &dir);
    assert!(out.status.success(), "{}", stdout(&out));
    let settings_text = std::fs::read_to_string(dir.join("settings.txt")).unwrap();
    assert!(settings_text.contains("projection=on"), "{settings_text}");
    let out = cli(&["--projection"], &dir);
    assert!(stdout(&out).contains("is ON"), "{}", stdout(&out));
    let out = cli(&["--projection", "off"], &dir);
    assert!(out.status.success());

    // --demo / --reveal (#25).
    let out = cli(&["circle", "5", "--demo"], &dir);
    let t = stdout(&out);
    assert!(out.status.success(), "{t}");
    assert!(!t.contains("78.5398"), "demo printed the answer: {t}");
    assert!(t.contains("Demonstration mode"), "{t}");
    let out = cli(&["circle", "5", "--demo", "--reveal"], &dir);
    assert!(stdout(&out).contains("78.5398"), "{}", stdout(&out));
    // ...but the log holds the real result either way.
    let log = cli(&["--log"], &dir);
    assert!(stdout(&log).contains("78.5398"), "{}", stdout(&log));

    // Examples (#28): save from a run, list, open.
    let out = cli(
        &[
            "circle",
            "5",
            "--save-example",
            "Circle demo",
            "--note",
            "Period 3",
        ],
        &dir,
    );
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains("Saved example 'Circle demo'"));
    let out = cli(&["--examples"], &dir);
    let t = stdout(&out);
    assert!(t.contains("Circle demo"), "{t}");
    assert!(t.contains("Period 3"), "{t}");
    let out = cli(&["--open-example", "Circle demo"], &dir);
    let t = stdout(&out);
    assert!(t.contains("Worked solution:"), "{t}");
    assert!(t.contains("78.5398"), "{t}");

    // Formula sheet (#29) through the binary.
    let sheet = dir.join("sheet.txt");
    let out = cli(&["--formula-sheet", "Math", &sheet.to_string_lossy()], &dir);
    assert!(out.status.success(), "{}", stdout(&out));
    let sheet_text = std::fs::read_to_string(&sheet).unwrap();
    assert!(sheet_text.contains("Formula: A = pi x r^2"), "{sheet_text}");
}

#[test]
fn help_text_covers_the_teachers_tools() {
    assert!(calcdex::rigour::HELP_TEXT.contains("FOR TEACHERS"));
    assert!(calcdex::rigour::HELP_TEXT.contains("PROJECTION"));
    assert!(calcdex::rigour::HELP_TEXT.contains("SETS"));
}

#[test]
fn set_entries_hold_display_tokens_for_dates() {
    // A date entry stores the merged MM/DD/YYYY tokens and runs.
    let mut set = ProblemSet::new("Dates");
    set_add(&mut set, "age", &["04/23/1988".into(), "10/25/2026".into()]).unwrap();
    let text = set_to_text(&set);
    assert!(text.contains("age | 04/23/1988 | 10/25/2026"), "{text}");
    let back = parse_set(&text).unwrap();
    assert_eq!(back.entries, set.entries);
    let key = answer_key_text(&back);
    assert!(key.contains("38 years"), "{key}");
    // The problem line uses the merged field labels.
    let e: SetEntry = back.entries[0].clone();
    let line = calcdex::teachers::entry_problem_line(&e).unwrap();
    assert!(line.contains("Birth Date = 04/23/1988"), "{line}");
}

// ---------- history rail under projection zoom (field fix) ----------

#[test]
fn history_rail_visibility_is_judged_on_physical_width() {
    use calcdex::history_rail_visible;
    // Normal size: the 1100 pt threshold applies as before.
    assert!(history_rail_visible(1200.0, 1.0));
    assert!(!history_rail_visible(1000.0, 1.0));
    assert!(!history_rail_visible(760.0, 1.0));
    // The field case: a wide physical window at 1.5x projection zoom
    // has ~1045 pt of layout width, yet the rail must stay visible.
    assert!(history_rail_visible(1045.0, 1.5));
    // A genuinely small screen stays collapsed even zoomed.
    assert!(!history_rail_visible(700.0, 1.5));
}
