//! v0.8.0 — engineers & professional polish: the electronics pack
//! (#41), tolerance stack-up (#42), safety factor (#43), the
//! materials table (#44), the mechanical pack (#45) and log
//! polish (#49: search, filter, pin, filtered export).
//!
//! Known answers were computed by hand from the formulas (and the
//! brief's vectors), independently of the code under test.

use calcdex::engineers::{
    decode_bands, encode_bands, find_material, material_by_code, resistor_colour_run, MATERIALS,
};
use calcdex::rigour::HELP_TEXT;
use calcdex::{
    all_calcs, append_entry_to, filter_entries, find_calc, load_history_from, log_export_text,
    parse_entry, run_calc, run_text_calc, save_history_to, serialize_entry, set_entry_pinned_at,
    LogEntry, LogFilter,
};

fn run(id: &str, args: &[f64]) -> String {
    run_calc(id, args).unwrap_or_else(|e| panic!("{id} {args:?} failed: {e}"))
}

fn run_err(id: &str, args: &[f64]) -> String {
    run_calc(id, args).expect_err(&format!("{id} {args:?} should fail"))
}

// ---------- registry ----------

#[test]
fn registry_grew_to_316_with_the_seven_new_calculators() {
    let calcs = all_calcs();
    assert_eq!(calcs.len(), 316);
    for id in [
        "resistor-colour-code",
        "resistors-series-parallel",
        "voltage-divider",
        "tolerance-stack-up",
        "safety-factor",
        "bolt-torque",
        "beam-bending",
    ] {
        let c = find_calc(id).unwrap_or_else(|| panic!("missing {id}"));
        assert!(!c.desc.is_empty(), "{id} has no description");
    }
    // Category placement (DECISIONS.md): electronics in Physics,
    // the mechanical pack in Construction.
    assert_eq!(find_calc("voltage-divider").unwrap().category, "Physics");
    assert_eq!(find_calc("beam-bending").unwrap().category, "Construction");
    assert_eq!(find_calc("bolt-torque").unwrap().category, "Construction");
}

// ---------- #41a resistor colour code ----------

#[test]
fn resistor_colour_decode_known_answers() {
    // The brief's vectors.
    let (v, tol) = decode_bands(&["red", "red", "brown", "gold"]).unwrap();
    assert_eq!((v, tol), (220.0, 5.0));
    let (v, tol) = decode_bands(&["yellow", "violet", "red", "gold"]).unwrap();
    assert_eq!((v, tol), (4700.0, 5.0));
    // Five bands: 105 x 1, +-1%.
    let (v, tol) = decode_bands(&["brown", "black", "green", "black", "brown"]).unwrap();
    assert_eq!((v, tol), (105.0, 1.0));
    // Metallic multipliers: 47 x 0.1 = 4.7.
    let (v, _) = decode_bands(&["yellow", "violet", "gold", "gold"]).unwrap();
    assert!((v - 4.7).abs() < 1e-12, "{v}");
    // Through the calculator's text interface.
    let out = run_text_calc("resistor-colour-code", "red red brown gold", true).unwrap();
    assert!(out.contains("220 \u{3a9}"), "{out}");
    assert!(out.contains("209 \u{3a9}"), "{out}"); // 220 - 5%
    assert!(out.contains("231 \u{3a9}"), "{out}"); // 220 + 5%
    let out = run_text_calc("resistor-colour-code", "yellow violet red gold", true).unwrap();
    assert!(out.contains("4.7 k\u{3a9}"), "{out}");
}

#[test]
fn resistor_colour_encode_and_teaching_errors() {
    let out = resistor_colour_run("4700").unwrap();
    assert!(out.contains("yellow violet red gold"), "{out}");
    let out = resistor_colour_run("4.7k").unwrap();
    assert!(out.contains("yellow violet red gold"), "{out}");
    let out = resistor_colour_run("220 silver").unwrap();
    assert!(out.contains("red red brown silver"), "{out}");
    // 105 needs the 5-band form.
    let (bands, _) = encode_bands(105.0, "gold").unwrap();
    assert_eq!(bands.join(" "), "brown black green black gold");
    // 1234 has no exact 4- or 5-band form: a teaching refusal.
    let e = resistor_colour_run("1234").unwrap_err();
    assert!(e.contains("cannot be shown exactly"), "{e}");
    // Unknown colour is named.
    let e = resistor_colour_run("red red pink gold").unwrap_err();
    assert!(e.contains("'pink'"), "{e}");
    // Gold cannot be a digit band.
    let e = resistor_colour_run("gold red brown gold").unwrap_err();
    assert!(e.contains("cannot be a value band"), "{e}");
    // Wrong band count teaches the shape.
    let e = resistor_colour_run("red red").unwrap_err();
    assert!(e.contains("4 or 5 colour bands"), "{e}");
}

// ---------- #41b/#41c electronics calculators ----------

#[test]
fn series_parallel_and_divider_known_answers() {
    let out = run("resistors-series-parallel", &[100.0, 200.0, 300.0]);
    assert!(out.contains("Series total = 600 \u{3a9}"), "{out}");
    assert!(
        out.contains("Parallel equivalent = 54.545455 \u{3a9}"),
        "{out}"
    );
    // A zero resistor shorts the parallel combination.
    let out = run("resistors-series-parallel", &[100.0, 0.0]);
    assert!(out.contains("Parallel equivalent = 0 \u{3a9}"), "{out}");
    assert!(out.contains("short circuit"), "{out}");
    let e = run_err("resistors-series-parallel", &[100.0, -5.0]);
    assert!(e.contains("cannot be negative"), "{e}");

    // 12 V across 1k + 2k: Vout = 8 V, I = 4 mA, P1 = 16 mW, P2 = 32 mW.
    let out = run("voltage-divider", &[12.0, 1000.0, 2000.0]);
    assert!(out.contains("Vout = 8 V"), "{out}");
    assert!(
        out.contains("Current through both resistors = 0.004 A"),
        "{out}"
    );
    assert!(out.contains("R1 dissipates 0.016 W"), "{out}");
    assert!(out.contains("R2 dissipates 0.032 W"), "{out}");
    let e = run_err("voltage-divider", &[12.0, 0.0, 2000.0]);
    assert!(e.contains("short circuit"), "{e}");
}

// ---------- #42 tolerance stack-up ----------

#[test]
fn tolerance_stack_up_known_answers() {
    // The brief's vector: 10+-0.1, 20+-0.2, 5+-0.05.
    let out = run("tolerance-stack-up", &[10.0, 0.1, 20.0, 0.2, 5.0, 0.05]);
    assert!(out.contains("Total = 35 \u{b1} 0.35 worst case"), "{out}");
    assert!(out.contains("from 34.65 to 35.35"), "{out}");
    // RSS = sqrt(0.01 + 0.04 + 0.0025) = sqrt(0.0525) = 0.229129...
    assert!(out.contains("RSS estimate = 35 \u{b1} 0.229129"), "{out}");
    let e = run_err("tolerance-stack-up", &[10.0, -0.1]);
    assert!(e.contains("cannot be negative"), "{e}");
}

// ---------- #43 safety factor ----------

#[test]
fn safety_factor_both_modes_and_the_failure_verdict() {
    let out = run("safety-factor", &[1.0, 250.0, 100.0, 0.0, 0.0]);
    assert!(out.contains("Applied stress = 100"), "{out}");
    assert!(out.contains("= 2.5"), "{out}");
    assert!(out.contains("MoS = FoS - 1 = 1.5"), "{out}");
    assert!(out.contains("grow by 150%"), "{out}");
    // Mode 2: load 5000 over area 50 = stress 100 — same answer.
    let out2 = run("safety-factor", &[2.0, 250.0, 0.0, 5000.0, 50.0]);
    assert!(out2.contains("Applied stress = 100"), "{out2}");
    assert!(out2.contains("= 2.5"), "{out2}");
    // FoS below 1 states the failure prediction plainly.
    let out = run("safety-factor", &[1.0, 100.0, 250.0, 0.0, 0.0]);
    assert!(out.contains("predicted to fail"), "{out}");
    assert!(out.contains("already 2.5 times the strength"), "{out}");
    // Teaching errors.
    assert!(run_err("safety-factor", &[3.0, 250.0, 100.0, 0.0, 0.0]).contains("Mode must be 1"));
    assert!(run_err("safety-factor", &[1.0, 0.0, 100.0, 0.0, 0.0]).contains("more than zero"));
    assert!(run_err("safety-factor", &[2.0, 250.0, 0.0, 5000.0, 0.0]).contains("zero area"));
}

// ---------- #44 materials table ----------

#[test]
fn materials_table_values_and_lookup() {
    assert_eq!(MATERIALS.len(), 12);
    let copper = find_material("copper").expect("copper");
    assert_eq!(copper.density, 8960.0);
    assert_eq!(copper.resistivity, Some(1.68e-8));
    let steel = find_material("steel").expect("steel alias");
    assert_eq!(steel.youngs, 200.0);
    assert_eq!(steel.density, 7850.0);
    let stainless = find_material("Stainless Steel 304").expect("by full name");
    assert_eq!(stainless.resistivity, Some(7.2e-7));
    // Insulators are honest: no resistivity figure.
    let glass = find_material("glass").expect("glass");
    assert_eq!(glass.resistivity, None);
    // Codes are the 1-based table positions, as --materials prints.
    assert_eq!(find_material("5").unwrap().name, "Mild steel (low-carbon)");
    assert_eq!(
        material_by_code(5.0).unwrap().name,
        "Mild steel (low-carbon)"
    );
    assert!(material_by_code(13.0).is_err());
    assert!(material_by_code(2.5).is_err());
    assert!(find_material("unobtainium").is_none());
}

// ---------- #45 mechanical pack ----------

#[test]
fn bolt_torque_known_answer() {
    // The brief's vector: K 0.2, D 10 mm, F 10,000 N -> 20 N·m.
    let out = run("bolt-torque", &[0.2, 10.0, 10000.0]);
    assert!(out.contains("Torque T = 20 N\u{b7}m"), "{out}");
    assert!(out.contains("14.751243 ft\u{b7}lbf"), "{out}");
    assert!(run_err("bolt-torque", &[0.0, 10.0, 10000.0]).contains("nut factor"));
}

#[test]
fn beam_known_answers_and_material_path() {
    // The brief's vector: L 2, P 1000, E 200 GPa, 0.05 x 0.1 m rect.
    let out = run("beam-bending", &[2.0, 1000.0, 200.0, 0.0, 1.0, 0.05, 0.1]);
    assert!(out.contains("I = 4.166667e-6 m^4"), "{out}");
    assert!(
        out.contains("Max moment M = P x L / 4 = 500 N\u{b7}m"),
        "{out}"
    );
    assert!(out.contains("= 6 MPa"), "{out}");
    assert!(out.contains("= 0.0002 m = 0.2 mm"), "{out}");
    // The same beam with E taken from the table (code 5 = mild
    // steel, E 200 GPa) gives the same numbers and names the source.
    let out2 = run("beam-bending", &[2.0, 1000.0, 0.0, 5.0, 1.0, 0.05, 0.1]);
    assert!(out2.contains("Mild steel (low-carbon)"), "{out2}");
    assert!(out2.contains("= 6 MPa"), "{out2}");
    assert!(out2.contains("typical value"), "{out2}");
    // Circle section runs: I = pi d^4 / 64.
    let out = run("beam-bending", &[1.0, 100.0, 200.0, 0.0, 2.0, 0.02, 0.0]);
    assert!(out.contains("circle of diameter 0.02 m"), "{out}");
    assert!(out.contains("I = 7.853982e-9 m^4"), "{out}");
    // Errors teach.
    assert!(
        run_err("beam-bending", &[2.0, 1000.0, 0.0, 99.0, 1.0, 0.05, 0.1])
            .contains("Material code")
    );
    assert!(
        run_err("beam-bending", &[2.0, 1000.0, 200.0, 0.0, 3.0, 0.05, 0.1])
            .contains("Section must be 1")
    );
    assert!(run_err("beam-bending", &[2.0, 1000.0, 200.0, 0.0, 1.0, 0.05, 0.0]).contains("height"));
}

// ---------- #49 log polish (library) ----------

fn entry(ts: &str, id: &str, name: &str, result: &str) -> LogEntry {
    LogEntry::plain(
        ts.into(),
        id.into(),
        name.into(),
        "5".into(),
        result.into(),
        "5".into(),
    )
}

#[test]
fn pin_serialization_round_trip_and_legacy_formats() {
    // A pinned entry gains the 9th field and round-trips.
    let mut e = entry("2026-10-26 09:00", "circle", "Circle", "Area = 78.539816.");
    e.pinned = true;
    let line = serialize_entry(&e);
    assert!(line.ends_with("|1"), "{line}");
    assert_eq!(line.matches('|').count(), 8, "{line}");
    assert_eq!(parse_entry(&line), Some(e));
    // An unpinned entry keeps the v0.7.0 8-field shape exactly.
    let e = entry("2026-10-26 09:00", "circle", "Circle", "Area = 78.539816.");
    let line = serialize_entry(&e);
    assert_eq!(line.matches('|').count(), 7, "{line}");
    assert!(!line.ends_with("|1"), "{line}");
    assert_eq!(parse_entry(&line), Some(e.clone()));
    // Legacy formats still parse, unpinned: the v0.1.1 six-field
    // line and the v0.7.0 eight-field line.
    let legacy6 = "2025-10-25 21:14|circle|Circle|5|5|Area = 78.539816.";
    let p = parse_entry(legacy6).expect("6-field line must parse");
    assert!(!p.pinned && p.tag.is_empty() && p.note.is_empty());
    let legacy8 = "2026-10-26 09:00|circle|Circle|5|5|Area = 78.539816.|tag-x|a note";
    let p = parse_entry(legacy8).expect("8-field line must parse");
    assert!(!p.pinned && p.tag == "tag-x" && p.note == "a note");
    // A pinned line with tag/note keeps all three.
    let pinned9 = "2026-10-26 09:00|circle|Circle|5|5|Area = 78.539816.|tag-x|a note|1";
    let p = parse_entry(pinned9).expect("9-field line must parse");
    assert!(p.pinned && p.tag == "tag-x" && p.note == "a note");
}

#[test]
fn pinned_entries_survive_the_cap() {
    let dir = std::env::temp_dir().join(format!("calcdex-v080-cap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("history.txt");
    // 500 unpinned entries, oldest first.
    let entries: Vec<LogEntry> = (0..500)
        .map(|i| entry("2026-10-26 09:00", "circle", &format!("Entry {i}"), "ok"))
        .collect();
    save_history_to(&path, &entries).unwrap();
    // Pin the OLDEST entry (it is #500 counting from the newest).
    let updated = set_entry_pinned_at(&path, 499, true).unwrap();
    assert!(updated.pinned);
    assert_eq!(updated.name, "Entry 0");
    // The pre-edit backup holds the pre-pin state.
    let backup = load_history_from(&dir.join("history-edit-backup.txt"));
    assert_eq!(backup.len(), 500);
    assert!(!backup[0].pinned);
    // Two more runs: unpinned count would hit 502 — the two oldest
    // UNPINNED entries go, the pinned oldest stays.
    append_entry_to(&path, entry("2026-10-26 10:00", "circle", "New 1", "ok")).unwrap();
    let after = append_entry_to(&path, entry("2026-10-26 10:01", "circle", "New 2", "ok")).unwrap();
    assert_eq!(after.len(), 501, "500 unpinned + 1 pinned");
    assert_eq!(after[0].name, "Entry 0");
    assert!(after[0].pinned);
    assert_eq!(after[1].name, "Entry 2");
    assert_eq!(after[after.len() - 1].name, "New 2");
    // Unpinning works through the same path.
    let n = after.len();
    let updated = set_entry_pinned_at(&path, n - 1, false).unwrap();
    assert!(!updated.pinned);
}

#[test]
fn log_search_and_filters_with_global_numbers() {
    let entries = vec![
        entry("2026-10-24 08:00", "circle", "Circle", "Area = 78.539816."),
        entry(
            "2026-10-25 09:00",
            "bmi",
            "Body Mass Index (BMI)",
            "BMI: 22.86.",
        ),
        entry("2026-10-25 10:00", "circle", "Circle", "Area = 12.566371."),
    ];
    let mut tagged = entries.clone();
    tagged[1].tag = "clinic".into();
    tagged[2].pinned = true;

    // Text search is a case-insensitive substring over the entry.
    let f = LogFilter {
        text: Some("bmi: 22".into()),
        ..Default::default()
    };
    let m = filter_entries(&tagged, &f);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].1.id, "bmi");
    // Global numbers count from the newest over the WHOLE log:
    // the BMI entry is #2 even when it is the only match.
    assert_eq!(m[0].0, 2);
    // Calculator filter: both circle runs, oldest first.
    let f = LogFilter {
        calc_id: Some("circle".into()),
        ..Default::default()
    };
    let m = filter_entries(&tagged, &f);
    assert_eq!(m.len(), 2);
    assert_eq!((m[0].0, m[1].0), (3, 1));
    // Tag and date filters.
    let f = LogFilter {
        tag: Some("CLINIC".into()),
        ..Default::default()
    };
    assert_eq!(filter_entries(&tagged, &f).len(), 1);
    let f = LogFilter {
        date: Some("2026-10-25".into()),
        ..Default::default()
    };
    assert_eq!(filter_entries(&tagged, &f).len(), 2);
    // Pinned only.
    let f = LogFilter {
        pinned_only: true,
        ..Default::default()
    };
    let m = filter_entries(&tagged, &f);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].0, 1);
    // The empty filter matches everything.
    assert_eq!(filter_entries(&tagged, &LogFilter::default()).len(), 3);
    let _ = entries;
}

#[test]
fn log_export_text_is_the_filtered_set() {
    let mut entries = vec![
        entry("2026-10-24 08:00", "circle", "Circle", "Area = 78.539816."),
        entry(
            "2026-10-25 09:00",
            "bmi",
            "Body Mass Index (BMI)",
            "BMI: 22.86.",
        ),
    ];
    entries[0].pinned = true;
    let f = LogFilter {
        calc_id: Some("circle".into()),
        ..Default::default()
    };
    let text = log_export_text(&entries, &f);
    assert!(text.contains("CalcDex calculation log export"), "{text}");
    assert!(text.contains("Filter: calculator circle"), "{text}");
    assert!(text.contains("1 of 2 entries match"), "{text}");
    assert!(text.contains("[pinned]"), "{text}");
    assert!(text.contains("Area = 78.539816."), "{text}");
    assert!(!text.contains("BMI: 22.86."), "{text}");
    // The whole-log export holds both.
    let text = log_export_text(&entries, &LogFilter::default());
    assert!(text.contains("2 of 2 entries match"), "{text}");
}

// ---------- help text coverage ----------

#[test]
fn help_text_mentions_the_engineers_batch() {
    assert!(HELP_TEXT.contains("FOR ENGINEERS"));
    assert!(HELP_TEXT.contains("MATERIALS"));
    assert!(HELP_TEXT.contains("pinned"));
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
fn cli_v080_end_to_end() {
    let dir = std::env::temp_dir().join(format!("calcdex-v080-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stdout = |o: &std::process::Output| String::from_utf8_lossy(&o.stdout).to_string();
    let stderr = |o: &std::process::Output| String::from_utf8_lossy(&o.stderr).to_string();

    // #41 through the binary: decode by calculator id, and the
    // series/parallel + divider calculators.
    let out = cli(
        &["resistor-colour-code", "red", "red", "brown", "gold"],
        &dir,
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("220 \u{3a9}"), "{}", stdout(&out));
    let out = cli(&["resistor-colour-code", "4700"], &dir);
    assert!(
        stdout(&out).contains("yellow violet red gold"),
        "{}",
        stdout(&out)
    );
    let out = cli(&["voltage-divider", "12", "1000", "2000"], &dir);
    assert!(stdout(&out).contains("Vout = 8 V"), "{}", stdout(&out));
    let out = cli(
        &["beam-bending", "2", "1000", "0", "5", "1", "0.05", "0.1"],
        &dir,
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("= 6 MPa"), "{}", stdout(&out));

    // #44: the materials listing and a lookup.
    let out = cli(&["--materials"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let t = stdout(&out);
    assert!(t.contains("Copper (annealed)"), "{t}");
    assert!(t.contains("8960"), "{t}");
    assert!(t.contains("typical"), "{t}");
    let out = cli(&["--materials", "stainless"], &dir);
    assert!(stdout(&out).contains("7.2e-7"), "{}", stdout(&out));
    let out = cli(&["--materials", "unobtainium"], &dir);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("No material"), "{}", stderr(&out));

    // #49: three runs, then search / filter / pin / export.
    for _ in 0..2 {
        let out = cli(&["circle", "5"], &dir);
        assert!(out.status.success());
    }
    let out = cli(&["bmi", "70", "175"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    // Search matches the BMI result text.
    let out = cli(&["--log-search", "BMI"], &dir);
    let t = stdout(&out);
    assert!(t.contains("Body Mass Index"), "{t}");
    assert!(t.contains("#1"), "{t}"); // BMI is the newest entry
                                      // Filter by calculator id: both circle runs, global numbers.
    let out = cli(&["--log-filter", "calc=circle"], &dir);
    let t = stdout(&out);
    assert!(t.contains("2 of"), "{t}");
    assert!(t.contains("#2"), "{t}");
    assert!(t.contains("#3"), "{t}");
    // Pin the oldest entry (#3) and read it back.
    let out = cli(&["--log-pin", "3"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let out = cli(&["--log"], &dir);
    let t = stdout(&out);
    assert!(t.contains("Pinned: yes"), "{t}");
    let out = cli(&["--log-filter", "pinned=yes"], &dir);
    assert!(stdout(&out).contains("1 of"), "{}", stdout(&out));
    let out = cli(&["--log-unpin", "3"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let out = cli(&["--log-filter", "pinned=yes"], &dir);
    assert!(stdout(&out).contains("0 of"), "{}", stdout(&out));
    // Export the filtered set to a file.
    let exp = dir.join("export.txt");
    let out = cli(
        &["--log-export", exp.to_str().unwrap(), "--calc", "circle"],
        &dir,
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let text = std::fs::read_to_string(&exp).unwrap();
    assert!(text.contains("Filter: calculator circle"), "{text}");
    assert!(text.contains("Area = 78.539816."), "{text}");
    // A bad filter key is a teaching error.
    let out = cli(&["--log-filter", "bogus=1"], &dir);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("Unknown log filter key"),
        "{}",
        stderr(&out)
    );
    // Pin out of range is refused.
    let out = cli(&["--log-pin", "99"], &dir);
    assert!(!out.status.success());
}
