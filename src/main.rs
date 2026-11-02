#![cfg_attr(windows, windows_subsystem = "windows")]
//! CalcDex — GUI shell + CLI. All real work lives in the library's
//! calculator registry, dispatched identically here, so there are no
//! GUI-only calculators.

mod win32;

use calcdex::engineers::{find_material, resistivity_text, MATERIALS};
use calcdex::features::{batch_run, csv_row, headline_of, search_ranked, vary_run};
use calcdex::meta::{
    defaults_for, display_fields, formula_for, glossary_for, hint_for, pack_members, packs_for,
    COURSE_PACKS, COURSE_PACK_NAMES,
};
use calcdex::rigour::{
    annotate_display, check_for_updates, constant_value_str, format_copy, headline_display,
    set_custom_constants, spec_verdict_lines, validate_constant_name, AngleMode, CopyFormat,
    DisplayFormat, DisplayMode, RoundingMode, CONSTANTS, HELP_TEXT,
};
use calcdex::settings::{load_settings, save_settings as save_settings_file, Settings};
use calcdex::students::{
    check_answer, deeper_steps, default_export_path, exact_line, exact_line_for_expression,
    plot_csv, practice_eligible, practice_problem, practice_problem_seeded, practice_random,
    sample_function, solution_text, solution_text_for_text, PracticeProblem, Rng,
    PRACTICE_EXCLUSION_NOTE,
};
use calcdex::teachers::{
    answer_key_text, entry_problem_line, formula_sheet_text, generate_set, load_examples,
    make_entry as set_make_entry, mark_set, marked_path_for, mask_result_text, problem_line,
    read_set, run_entry, set_add, set_listing, set_to_text, upsert_example, worksheet_text,
    write_set, ProblemSet, SavedExample, SetEntry, DEMO_HIDDEN,
};
use calcdex::{
    all_calcs, annotate_entry_at, append_history, clear_history_with_backup, date_group_at,
    display_tokens, expand_args, filter_entries, history_backup_exists, history_path,
    lab_record_text, load_history, log_export_text, make_entry, now_stamp, parse_value_list,
    restore_history_backup, run_calc, run_text_calc, set_entry_pinned_at, today_str, Calc,
    LogEntry, LogFilter, DATE_HINT,
};
use std::collections::BTreeSet;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        std::process::exit(cli(&args));
    }
    // v0.9.0 single instance: a second window launch prints a
    // short note and exits 0. The guard is held for the whole
    // run; dropping it at process end releases the name.
    let _instance_guard = match win32::acquire_single_instance() {
        Some(guard) => guard,
        None => {
            println!("CalcDex is already running — switch to the open window instead.");
            return Ok(());
        }
    };
    // v0.9.0: restore the last window position/size from the
    // settings file (App::new loads the same file again for the
    // full state). Implausible values were dropped at parse time,
    // so whatever survives here is safe to apply.
    let ids: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let saved = load_settings(&ids);
    let viewport = egui::ViewportBuilder::default()
        .with_title("CalcDex — every kind of calculation, offline")
        .with_icon(std::sync::Arc::new(egui::IconData {
            rgba: include_bytes!("../assets/icon-64.rgba").to_vec(),
            width: 64,
            height: 64,
        }))
        .with_min_inner_size([760.0, 520.0]);
    let viewport = match saved.window {
        Some((x, y, w, h)) => viewport.with_position([x, y]).with_inner_size([w, h]),
        None => viewport.with_inner_size([1180.0, 760.0]),
    };
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "CalcDex",
        options,
        Box::new(|cc| {
            configure_style(&cc.egui_ctx);
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}

/// CLI parity with the GUI:
///   CalcDex --list
///   CalcDex --search loan
///   CalcDex compound-interest 1000 5 10 12
///   CalcDex age 04/23/1988 10/25/2026      (dates as ONE MM/DD/YYYY argument)
///   CalcDex --log / --clear-log
/// Print the log entries matching a filter (item #49), newest
/// first, each with its GLOBAL entry number (1 = newest over the
/// whole log) so --log-pin / --log-tag / --log-note numbers stay
/// unambiguous from a filtered view.
fn print_log_matches(entries: &[LogEntry], filter: &LogFilter) {
    let matched = filter_entries(entries, filter);
    println!(
        "Log filter: {} — {} of {} entries match, newest first:",
        filter.describe(),
        matched.len(),
        entries.len()
    );
    for (num, e) in matched.iter().rev() {
        println!("\n#{}  {}  {} ({})", num, e.timestamp, e.name, e.id);
        println!("  Inputs: {}", e.inputs);
        println!("  Result: {}", e.result);
        if e.pinned {
            println!("  Pinned: yes");
        }
        if !e.tag.is_empty() {
            println!("  Tag: {}", e.tag);
        }
        if !e.note.is_empty() {
            println!("  Note: {}", e.note);
        }
    }
}

/// Parse `--log-filter` key=value pairs (item #49). Keys: calc,
/// tag, date, pinned (yes/no), text.
fn parse_log_filter_pairs(pairs: &[String]) -> Result<LogFilter, String> {
    let mut filter = LogFilter::default();
    for pair in pairs {
        let Some((key, value)) = pair.split_once('=') else {
            return Err(format!(
                "'{pair}' is not key=value — filters look like calc=circle, tag=clinic, date=2026-10-25, pinned=yes, text=burette."
            ));
        };
        match key {
            "calc" => filter.calc_id = Some(value.to_string()),
            "tag" => filter.tag = Some(value.to_string()),
            "date" => filter.date = Some(value.to_string()),
            "text" => filter.text = Some(value.to_string()),
            "pinned" => match value {
                "yes" => filter.pinned_only = true,
                "no" => {}
                _ => return Err(format!("pinned must be yes or no, got '{value}'.")),
            },
            _ => {
                return Err(format!(
                "Unknown log filter key '{key}' — the keys are calc, tag, date, pinned and text."
            ))
            }
        }
    }
    Ok(filter)
}

/// Parse the filter flags that may follow `--log-export <file>`
/// (item #49): --search <text>, --calc <id>, --tag <tag>,
/// --date <YYYY-MM-DD>, --pinned.
fn parse_log_export_flags(rest: &[String]) -> Result<LogFilter, String> {
    let mut filter = LogFilter::default();
    let mut i = 0;
    while i < rest.len() {
        let flag = rest[i].as_str();
        let need = |i: usize| -> Result<String, String> {
            rest.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value after it."))
        };
        match flag {
            "--search" => {
                filter.text = Some(need(i)?);
                i += 2;
            }
            "--calc" => {
                filter.calc_id = Some(need(i)?);
                i += 2;
            }
            "--tag" => {
                filter.tag = Some(need(i)?);
                i += 2;
            }
            "--date" => {
                filter.date = Some(need(i)?);
                i += 2;
            }
            "--pinned" => {
                filter.pinned_only = true;
                i += 1;
            }
            _ => {
                return Err(format!(
                    "Unknown --log-export flag '{flag}' — after the file name use --search <text>, --calc <id>, --tag <tag>, --date <YYYY-MM-DD> or --pinned."
                ))
            }
        }
    }
    Ok(filter)
}

fn cli(args: &[String]) -> i32 {
    let calcs = all_calcs();
    // v0.7.0 #38: install the settings file's user-defined
    // constants so expression runs resolve them (the window
    // installs the same table at startup).
    {
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let settings = load_settings(&valid);
        set_custom_constants(settings.constants);
    }
    // Global display/copy/spec flags (items #1/#2/#3/#4/#9/#10 +
    // batch --force): they may appear anywhere and only change how
    // a result is SHOWN — the full-precision result (and the log)
    // are never altered.
    let mut display = DisplayFormat::default();
    let mut copy_format: Option<CopyFormat> = None;
    let mut spec: Option<(f64, f64)> = None;
    let mut force = false;
    // v0.5.0 students batch: --steps (#12), --check (#13),
    // --practice/--seed/--answer (#15), --plot (#17), --export (#18).
    let mut steps_deeper = false;
    let mut check_ans: Option<String> = None;
    let mut export_path: Option<String> = None;
    let mut practice = false;
    let mut seed: Option<u64> = None;
    let mut answer_ans: Option<String> = None;
    let mut plot: Option<(f64, f64)> = None;
    // v0.6.0 teachers batch: --demo/--reveal (#25) and
    // --save-example/--note (#28) — presentation + library flags
    // that may appear anywhere, like the display flags.
    let mut demo = false;
    let mut reveal = false;
    // v0.9.0 maintenance modifiers: --json (machine-readable
    // result for runs and for --clean/--uninstall), --dry-run and
    // --silent (for --clean/--uninstall). Like the other global
    // flags they may appear anywhere in the arguments.
    let mut json = false;
    let mut dry_run = false;
    let mut silent = false;
    let mut save_example: Option<String> = None;
    let mut ex_note = String::new();
    let mut args: Vec<String> = args.to_vec();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        let mut take = 0usize;
        match flag {
            "--sig-figs" => match args.get(i + 1).and_then(|v| v.parse::<u8>().ok()) {
                Some(n) if (1..=10).contains(&n) => {
                    display.mode = DisplayMode::SigFigs;
                    display.n = Some(n);
                    take = 2;
                }
                _ => {
                    eprintln!("--sig-figs needs a whole number from 1 to 10, e.g. --sig-figs 3.");
                    return 2;
                }
            },
            "--decimals" => match args.get(i + 1).and_then(|v| v.parse::<u8>().ok()) {
                Some(n) if n <= 10 => {
                    display.mode = DisplayMode::Decimals;
                    display.n = Some(n);
                    take = 2;
                }
                _ => {
                    eprintln!("--decimals needs a whole number from 0 to 10, e.g. --decimals 2.");
                    return 2;
                }
            },
            "--format" => {
                match args.get(i + 1) {
                    Some(v) => match DisplayFormat::parse(v) {
                        Ok(f) => {
                            let rounding = display.rounding;
                            display = DisplayFormat { rounding, ..f };
                            take = 2;
                        }
                        Err(e) => {
                            eprintln!("Error: {e}");
                            return 2;
                        }
                    },
                    None => {
                        eprintln!("--format needs a value: off, sig-figs:N, decimals:N, engineering or auto.");
                        return 2;
                    }
                }
            }
            "--rounding" => match args.get(i + 1).and_then(|v| RoundingMode::parse(v)) {
                Some(r) => {
                    display.rounding = r;
                    take = 2;
                }
                _ => {
                    eprintln!("--rounding needs one of: half-up, bankers, truncate.");
                    return 2;
                }
            },
            "--as" => match args.get(i + 1).and_then(|v| CopyFormat::parse(v)) {
                Some(f) => {
                    copy_format = Some(f);
                    take = 2;
                }
                _ => {
                    eprintln!("--as needs one of: plain, latex, scientific, csv.");
                    return 2;
                }
            },
            "--spec" => {
                let mn = args.get(i + 1).and_then(|v| v.parse::<f64>().ok());
                let mx = args.get(i + 2).and_then(|v| v.parse::<f64>().ok());
                match (mn, mx) {
                    (Some(mn), Some(mx)) if mn <= mx => {
                        spec = Some((mn, mx));
                        take = 3;
                    }
                    _ => {
                        eprintln!("--spec needs two numbers, min then max (min <= max), e.g. --spec 10 20.");
                        return 2;
                    }
                }
            }
            "--force" => {
                force = true;
                take = 1;
            }
            "--steps" => match args.get(i + 1).map(|s| s.as_str()) {
                Some("deeper") => {
                    steps_deeper = true;
                    take = 2;
                }
                Some("standard") => {
                    steps_deeper = false;
                    take = 2;
                }
                _ => {
                    eprintln!("--steps needs standard or deeper.");
                    return 2;
                }
            },
            "--check" => match args.get(i + 1) {
                Some(v) => {
                    check_ans = Some(v.clone());
                    take = 2;
                }
                None => {
                    eprintln!("--check needs your answer, e.g. --check 42 or --check 3/4.");
                    return 2;
                }
            },
            "--export" => match args.get(i + 1) {
                Some(v) => {
                    export_path = Some(v.clone());
                    take = 2;
                }
                None => {
                    eprintln!("--export needs a file path.");
                    return 2;
                }
            },
            "--practice" => {
                practice = true;
                take = 1;
            }
            "--seed" => match args.get(i + 1).and_then(|v| v.parse::<u64>().ok()) {
                Some(n) => {
                    seed = Some(n);
                    take = 2;
                }
                _ => {
                    eprintln!("--seed needs a whole number, e.g. --seed 7.");
                    return 2;
                }
            },
            "--answer" => match args.get(i + 1) {
                Some(v) => {
                    answer_ans = Some(v.clone());
                    take = 2;
                }
                None => {
                    eprintln!("--answer needs your answer.");
                    return 2;
                }
            },
            "--plot" => {
                let from = args.get(i + 1).and_then(|v| v.parse::<f64>().ok());
                let to = args.get(i + 2).and_then(|v| v.parse::<f64>().ok());
                match (from, to) {
                    (Some(f), Some(t)) if f < t => {
                        plot = Some((f, t));
                        take = 3;
                    }
                    _ => {
                        eprintln!(
                            "--plot needs two numbers, from then to (from < to), e.g. --plot -5 5."
                        );
                        return 2;
                    }
                }
            }
            "--demo" => {
                demo = true;
                take = 1;
            }
            "--reveal" => {
                reveal = true;
                take = 1;
            }
            "--json" => {
                json = true;
                take = 1;
            }
            "--dry-run" => {
                dry_run = true;
                take = 1;
            }
            "--silent" => {
                silent = true;
                take = 1;
            }
            "--save-example" => match args.get(i + 1) {
                Some(v) => {
                    save_example = Some(v.clone());
                    take = 2;
                }
                None => {
                    eprintln!("--save-example needs a title in quotes, e.g. --save-example \"Circle demo\".");
                    return 2;
                }
            },
            "--note" => match args.get(i + 1) {
                Some(v) => {
                    ex_note = v.clone();
                    take = 2;
                }
                None => {
                    eprintln!(
                        "--note needs the note text in quotes (it belongs to --save-example)."
                    );
                    return 2;
                }
            },
            _ => {}
        }
        if take > 0 {
            args.drain(i..i + take);
        } else {
            i += 1;
        }
    }
    if args.is_empty() {
        // Only display flags were given: nothing to run.
        eprintln!("Nothing to run — give a calculator id or --help.");
        return 2;
    }
    let args = &args[..];
    if args[0] == "--version" {
        if json {
            println!(
                "{{\"ok\":true,\"name\":\"CalcDex\",\"version\":\"{}\"}}",
                env!("CARGO_PKG_VERSION")
            );
        } else {
            println!("CalcDex {}", env!("CARGO_PKG_VERSION"));
        }
        return 0;
    }
    if args[0] == "--clean" || args[0] == "--uninstall" {
        return cli_clean(args[0] == "--uninstall", dry_run, silent, json);
    }
    if args[0] == "--list" || args[0] == "list" {
        for c in &calcs {
            println!("{:<38} [{}] {}", c.id, c.category, c.name);
        }
        println!("\n{} calculators total.", calcs.len());
        return 0;
    }
    if args[0] == "--search" || args[0] == "search" {
        // Ranked search with synonyms (item #47).
        let q = args.get(1).cloned().unwrap_or_default();
        for idx in search_ranked(&calcs, &q) {
            let c = &calcs[idx];
            println!("{:<38} [{}] {}", c.id, c.category, c.name);
        }
        return 0;
    }
    if args[0] == "--log" || args[0] == "log" {
        let entries = load_history();
        if entries.is_empty() {
            println!("The calculation log is empty. Run any calculator and its result is saved here automatically.");
            return 0;
        }
        println!(
            "Calculation log ({} entries, newest first — #1 is the newest; --log-tag / --log-note use these numbers):",
            entries.len()
        );
        for (i, e) in entries.iter().rev().enumerate() {
            println!("\n#{}  {}  {} ({})", i + 1, e.timestamp, e.name, e.id);
            println!("  Inputs: {}", e.inputs);
            println!("  Result: {}", e.result);
            if e.pinned {
                println!("  Pinned: yes");
            }
            if !e.tag.is_empty() {
                println!("  Tag: {}", e.tag);
            }
            if !e.note.is_empty() {
                println!("  Note: {}", e.note);
            }
        }
        return 0;
    }
    if args[0] == "--log-search" {
        // Log polish (item #49): text search over the whole log.
        // Matches print newest first with their GLOBAL entry
        // numbers, so --log-pin / --log-tag numbers stay exact.
        let text = args[1..].join(" ");
        if text.trim().is_empty() {
            eprintln!("Usage: CalcDex --log-search <text> — the text is matched against each entry's name, inputs, result, tag and note.");
            return 2;
        }
        let entries = load_history();
        let filter = LogFilter {
            text: Some(text),
            ..Default::default()
        };
        print_log_matches(&entries, &filter);
        return 0;
    }
    if args[0] == "--log-filter" {
        // Log polish (item #49): key=value filters, combinable.
        if args.len() < 2 {
            eprintln!("Usage: CalcDex --log-filter <key>=<value> [...] — keys: calc=<calculator id>, tag=<tag>, date=<YYYY-MM-DD>, pinned=yes|no, text=<words>.");
            return 2;
        }
        let filter = match parse_log_filter_pairs(&args[1..]) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error: {e}");
                return 2;
            }
        };
        let entries = load_history();
        print_log_matches(&entries, &filter);
        return 0;
    }
    if args[0] == "--log-pin" || args[0] == "--log-unpin" {
        // Log polish (item #49): pinning protects an entry from
        // the 500-entry trim. Same numbering and pre-edit backup
        // as --log-tag / --log-note.
        let pin = args[0] == "--log-pin";
        let Some(num) = args.get(1) else {
            eprintln!(
                "Usage: CalcDex {} <entry-number> — the number is the # shown by --log (1 = newest).",
                args[0]
            );
            return 2;
        };
        let Ok(n) = num.parse::<usize>() else {
            eprintln!("'{num}' is not an entry number — use the # shown by --log.");
            return 2;
        };
        if n == 0 {
            eprintln!("Entry numbers start at 1 (the newest entry). See: CalcDex --log");
            return 2;
        }
        return match set_entry_pinned_at(&history_path(), n - 1, pin) {
            Ok(e) => {
                println!(
                    "Entry #{} ({} — {}) {}pinned.",
                    n,
                    e.timestamp,
                    e.name,
                    if pin { "" } else { "un" }
                );
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "--log-export" {
        // Log polish (item #49): export the FILTERED set (or the
        // whole log with no filter flags) as plain text.
        let Some(path) = args.get(1) else {
            eprintln!("Usage: CalcDex --log-export <file> [--search <text>] [--calc <id>] [--tag <tag>] [--date <YYYY-MM-DD>] [--pinned]");
            return 2;
        };
        let filter = match parse_log_export_flags(&args[2..]) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error: {e}");
                return 2;
            }
        };
        let entries = load_history();
        if entries.is_empty() {
            println!("The calculation log is empty — there is nothing to export yet. Run any calculator and it is recorded here.");
            return 0;
        }
        let matched = filter_entries(&entries, &filter).len();
        return match write_guarded(path, &log_export_text(&entries, &filter), force) {
            Ok(()) => {
                println!(
                    "Log export written to {path} ({matched} of {} entries, oldest first).",
                    entries.len()
                );
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "--clear-log" {
        // Clearing first writes a backup copy (undo: --restore-log).
        return match clear_history_with_backup() {
            Ok(()) => {
                println!("Calculation log cleared. The cleared entries were backed up first — CalcDex --restore-log undoes this.");
                0
            }
            Err(e) => {
                eprintln!("Could not clear the log: {e}");
                1
            }
        };
    }
    if args[0] == "--restore-log" {
        return match restore_history_backup() {
            Ok(entries) => {
                println!(
                    "Restored {} log entr{} from the cleared-log backup.",
                    entries.len(),
                    if entries.len() == 1 { "y" } else { "ies" }
                );
                0
            }
            Err(e) => {
                eprintln!("Could not restore the log: {e}");
                1
            }
        };
    }
    if args[0] == "--log-tag" || args[0] == "--log-note" {
        // Lab notebook (item #39): annotate one entry. Entry
        // numbers count from the newest, exactly as --log shows.
        // Before the edit, the log is backed up to
        // history-edit-backup.txt (the house backup-first rule).
        let is_tag = args[0] == "--log-tag";
        let (Some(num), Some(_)) = (args.get(1), args.get(2)) else {
            eprintln!(
                "Usage: CalcDex {} <entry-number> <{}> — the number is the # shown by --log (1 = newest). An empty text clears it.",
                args[0],
                if is_tag { "tag" } else { "note text" }
            );
            return 2;
        };
        let Ok(n) = num.parse::<usize>() else {
            eprintln!("'{num}' is not an entry number — use the # shown by --log.");
            return 2;
        };
        if n == 0 {
            eprintln!("Entry numbers start at 1 (the newest entry). See: CalcDex --log");
            return 2;
        }
        let text = args[2..].join(" ");
        let (tag, note) = if is_tag {
            (Some(text.as_str()), None)
        } else {
            (None, Some(text.as_str()))
        };
        return match annotate_entry_at(&history_path(), n - 1, tag, note) {
            Ok(e) => {
                println!(
                    "Entry #{} ({} — {}) {}: {}",
                    n,
                    e.timestamp,
                    e.name,
                    if is_tag { "tag" } else { "note" },
                    if text.is_empty() {
                        "(cleared)".to_string()
                    } else {
                        text
                    }
                );
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "--lab-export" {
        // The lab record (item #39): the whole log as one
        // plain-text document, oldest first.
        let Some(path) = args.get(1) else {
            eprintln!("Usage: CalcDex --lab-export <file>");
            return 2;
        };
        let entries = load_history();
        if entries.is_empty() {
            println!("The calculation log is empty — there is nothing to export yet. Run any calculator and it is recorded here.");
            return 0;
        }
        return match write_guarded(path, &lab_record_text(&entries), force) {
            Ok(()) => {
                println!(
                    "Lab record written to {path} ({} entries, oldest first).",
                    entries.len()
                );
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "--help-guide" {
        println!("{HELP_TEXT}");
        return 0;
    }
    if args[0] == "--check-update" {
        println!("{}", check_for_updates());
        return 0;
    }
    if args[0] == "--describe" {
        let Some(id) = args.get(1) else {
            eprintln!("--describe needs a calculator id, e.g. CalcDex --describe bmi");
            return 2;
        };
        let Some(calc) = calcs.iter().find(|c| c.id == id.as_str()) else {
            eprintln!("No calculator called '{id}'. Try: CalcDex --list");
            return 2;
        };
        println!("{} [{}]  id: {}", calc.name, calc.category, calc.id);
        println!("{}", calc.desc);
        println!("Formula: {}", formula_for(calc));
        let packs = packs_for(calc);
        if !packs.is_empty() {
            println!("Course packs: {}", packs.join(", "));
        }
        let glossary = glossary_for(calc);
        if !glossary.is_empty() {
            println!("Symbols:");
            for (sym, meaning) in glossary {
                println!("  {sym} = {meaning}");
            }
        }
        println!("Fields (numbered from 0):");
        for (fi, (label, hint)) in describe_fields(calc).iter().enumerate() {
            println!("  {fi}. {label} — {hint}");
        }
        if calc.text_run.is_some() {
            println!(
                "Example: CalcDex expr \"{}\"   (or: CalcDex {} --example)",
                calc.example_text, calc.id
            );
        } else {
            println!(
                "Example: CalcDex {} --example   (inputs: {})",
                calc.id,
                display_tokens(calc, calc.example).join(", ")
            );
        }
        return 0;
    }
    if args[0] == "--batch" {
        // Batch calculation from CSV (item #31).
        let (Some(id), Some(input_path), Some(output_path)) =
            (args.get(1), args.get(2), args.get(3))
        else {
            eprintln!("Usage: CalcDex --batch <calculator-id> <input.csv> <output.csv>");
            return 2;
        };
        let Some(calc) = calcs.iter().find(|c| c.id == id.as_str()) else {
            eprintln!("No calculator called '{id}'. Try: CalcDex --list");
            return 2;
        };
        if calc.text_run.is_some() {
            eprintln!(
                "'{}' takes text, not CSV rows — batch needs a numeric calculator.",
                calc.name
            );
            return 2;
        }
        // Overwrite protection: an existing output file is only
        // replaced with --force, and then it is backed up first.
        if std::path::Path::new(output_path).exists() {
            if !force {
                eprintln!("{output_path} already exists — refusing to overwrite it. Add --force to replace it (the old file is backed up to {output_path}.bak first).");
                return 1;
            }
            let bak = format!("{output_path}.bak");
            if let Err(e) = std::fs::copy(output_path, &bak) {
                eprintln!("Could not back up {output_path} to {bak}: {e}");
                return 1;
            }
        }
        let text = match std::fs::read_to_string(input_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Could not read {input_path}: {e}");
                return 1;
            }
        };
        return match batch_run(calc, &text) {
            Ok(outcome) => {
                if let Err(e) = std::fs::write(output_path, &outcome.output_csv) {
                    eprintln!("Could not write {output_path}: {e}");
                    return 1;
                }
                println!(
                    "{} rows: {} ok, {} errors — see {}",
                    outcome.ok + outcome.errors,
                    outcome.ok,
                    outcome.errors,
                    output_path
                );
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "set" {
        // Problem-set verbs (items #22/#23/#27/#30).
        return cli_set(&args[1..], force, seed);
    }
    if args[0] == "--pack" {
        // Course packs (item #24): list one pack's calculators.
        let Some(pack) = args.get(1) else {
            eprintln!(
                "Usage: CalcDex --pack <{}> [--list]",
                COURSE_PACK_NAMES.join("|")
            );
            return 2;
        };
        if !COURSE_PACK_NAMES.contains(&pack.as_str()) {
            eprintln!(
                "No course pack called '{pack}'. Packs: {}.",
                COURSE_PACK_NAMES.join(", ")
            );
            return 2;
        }
        let members = pack_members(&calcs, pack);
        println!("{pack} pack — {} calculators:", members.len());
        for i in members {
            let c = &calcs[i];
            println!("{:<38} [{}] {}", c.id, c.category, c.name);
        }
        return 0;
    }
    if args[0] == "--projection" {
        // Classroom projection mode (item #21): the shared
        // settings file is the source of truth for both surfaces.
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let mut settings = load_settings(&valid);
        match args.get(1).map(|s| s.as_str()) {
            Some("on") | Some("off") => {
                settings.projection = args[1] == "on";
                return match save_settings_file(&settings) {
                    Ok(()) => {
                        println!(
                            "Classroom projection mode is now {}. While it is on, the window renders at about 1.5x zoom with brightened muted/hint colours. (Settings file: {})",
                            if settings.projection { "ON" } else { "OFF" },
                            calcdex::settings::settings_path().display()
                        );
                        0
                    }
                    Err(e) => {
                        eprintln!("Could not save settings: {e}");
                        1
                    }
                };
            }
            None => {
                println!(
                    "Classroom projection mode is {} (settings file: {}). Change it: CalcDex --projection on|off",
                    if settings.projection { "ON" } else { "OFF" },
                    calcdex::settings::settings_path().display()
                );
                return 0;
            }
            _ => {
                eprintln!("Usage: CalcDex --projection [on|off]");
                return 2;
            }
        }
    }
    if args[0] == "--formula-sheet" {
        // Printable formula sheet (item #29).
        let (Some(cat), Some(out_path)) = (args.get(1), args.get(2)) else {
            eprintln!("Usage: CalcDex --formula-sheet <Category|All> <file>");
            return 2;
        };
        let cat_opt = if cat == "All" {
            None
        } else {
            Some(cat.as_str())
        };
        return match formula_sheet_text(&calcs, cat_opt) {
            Ok(text) => match write_guarded(out_path, &text, force) {
                Ok(()) => {
                    let n = if cat == "All" {
                        calcs.len()
                    } else {
                        calcs.iter().filter(|c| c.category == cat).count()
                    };
                    println!("Formula sheet for {cat} written to {out_path} ({n} calculators).");
                    0
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            },
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "--mark" {
        // Bulk marking (item #26).
        let (Some(set_file), Some(answers)) = (args.get(1), args.get(2)) else {
            eprintln!("Usage: CalcDex --mark <set-file> <answers.csv> [marked.csv]");
            return 2;
        };
        let set = match read_set(std::path::Path::new(set_file)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        };
        let text = match std::fs::read_to_string(answers) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Could not read {answers}: {e}");
                return 1;
            }
        };
        return match mark_set(&set, &text) {
            Ok(outcome) => {
                let out_path = args.get(3).cloned().unwrap_or_else(|| {
                    marked_path_for(std::path::Path::new(answers))
                        .display()
                        .to_string()
                });
                match write_guarded(&out_path, &outcome.output_csv, force) {
                    Ok(()) => {
                        for line in &outcome.summary {
                            println!("{line}");
                        }
                        println!("Marked CSV written to {out_path}.");
                        0
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        1
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "--examples" {
        // The saved class examples library (item #28).
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let list = load_examples(&valid);
        println!(
            "Examples library: {}",
            calcdex::teachers::examples_path().display()
        );
        if list.is_empty() {
            println!("No saved examples yet. Save one from any run: CalcDex <id> <numbers> --save-example \"<title>\" [--note \"<note>\"]");
            return 0;
        }
        println!("Saved examples ({}):", list.len());
        for ex in &list {
            let name = calcs
                .iter()
                .find(|c| c.id == ex.calc_id)
                .map(|c| c.name)
                .unwrap_or("");
            println!("  {} — {} ({})", ex.title, name, ex.calc_id);
            if !ex.note.is_empty() {
                println!("    Note: {}", ex.note);
            }
            println!("    Inputs: {}", ex.inputs.join(", "));
        }
        return 0;
    }
    if args[0] == "--open-example" {
        // Open a saved example (item #28): print its problem and
        // its worked solution.
        if args.len() < 2 {
            eprintln!("Usage: CalcDex --open-example \"<title>\"");
            return 2;
        }
        let title = args[1..].join(" ");
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let list = load_examples(&valid);
        let Some(ex) = list.iter().find(|e| e.title == title) else {
            eprintln!("No saved example called '{title}'. See: CalcDex --examples");
            return 2;
        };
        let entry = SetEntry {
            calc_id: ex.calc_id.clone(),
            inputs: ex.inputs.clone(),
        };
        return match run_entry(&entry) {
            Ok((calc, _args, out)) => {
                println!("{} — {}", ex.title, calc.name);
                if !ex.note.is_empty() {
                    println!("Note: {}", ex.note);
                }
                match entry_problem_line(&entry) {
                    Ok(line) => println!("Problem: {line}"),
                    Err(_) => println!("Inputs: {}", ex.inputs.join(", ")),
                }
                println!("\nWorked solution:\n{out}");
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    if args[0] == "--materials" {
        // The materials table (item #44): typical published values.
        // No argument lists the whole table with the codes the
        // beam calculator takes; an argument looks one material up
        // by name, alias or code.
        match args.get(1) {
            None => {
                println!("Materials — typical published values (real alloys and batches vary; use the manufacturer's data for critical work):");
                for (i, mat) in MATERIALS.iter().enumerate() {
                    println!(
                        "\n{:>2}. {} — density {} kg/m^3, resistivity {}, Young's modulus {} GPa",
                        i + 1,
                        mat.name,
                        calcdex::n(mat.density),
                        resistivity_text(mat),
                        calcdex::n(mat.youngs)
                    );
                }
                println!("\nThe beam calculator takes a material's Young's modulus by code: CalcDex beam-bending 2 1000 0 <code> 1 0.05 0.1");
            }
            Some(q) => match find_material(q) {
                Some(mat) => {
                    let code = MATERIALS.iter().position(|m| m.name == mat.name).unwrap() + 1;
                    println!(
                        "{} (code {}): density {} kg/m^3, resistivity {}, Young's modulus {} GPa — typical values.",
                        mat.name,
                        code,
                        calcdex::n(mat.density),
                        resistivity_text(mat),
                        calcdex::n(mat.youngs)
                    );
                }
                None => {
                    eprintln!(
                        "No material matches '{q}' — CalcDex --materials lists the table (names, aliases and codes 1-{}).",
                        MATERIALS.len()
                    );
                    return 2;
                }
            },
        }
        return 0;
    }
    if args[0] == "--constants" {
        // The constants library (item #38): the CODATA built-ins,
        // then the user's own constants, marked as theirs.
        println!("Constants for the Expression Calculator (CalcDex expr \"h*c/500e-9\"):");
        println!("\nBuilt in (CODATA 2018):");
        for c in CONSTANTS {
            println!(
                "  {:<12} {} = {}  [{}]",
                c.symbol,
                c.name,
                constant_value_str(c.value),
                c.unit
            );
        }
        println!("  {:<12} Pi = 3.14159265358979", "pi");
        println!("  {:<12} Euler's number = 2.71828182845905", "e");
        println!("  {:<12} Tau (2 x pi) = 6.28318530717959", "tau");
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let settings = load_settings(&valid);
        if settings.constants.is_empty() {
            println!("\nYour constants: none yet. Define one: CalcDex --define my_g=9.81");
        } else {
            println!("\nYour constants (from the settings file):");
            for (name, value) in &settings.constants {
                println!("  {:<12} (your constant) = {}", name, calcdex::n(*value));
            }
        }
        return 0;
    }
    if args[0] == "--define" {
        // Define (or redefine) a user constant (item #38) in the
        // shared settings file. Built-in names are refused by
        // validate_constant_name — a CODATA value can never be
        // shadowed by a user value.
        let Some(spec) = args.get(1) else {
            eprintln!("Usage: CalcDex --define <name>=<value>, e.g. CalcDex --define my_g=9.81");
            return 2;
        };
        let Some((name_raw, value_raw)) = spec.split_once('=') else {
            eprintln!("--define needs name=value, e.g. CalcDex --define my_g=9.81");
            return 2;
        };
        let name = match validate_constant_name(name_raw) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Error: {e}");
                return 2;
            }
        };
        let value = match value_raw.trim().parse::<f64>() {
            Ok(v) if v.is_finite() => v,
            _ => {
                eprintln!("'{value_raw}' is not a number — --define needs name=<number>, e.g. CalcDex --define my_g=9.81");
                return 2;
            }
        };
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let mut settings = load_settings(&valid);
        let replacing = settings.constants.iter().any(|(n, _)| *n == name);
        settings.constants.retain(|(n, _)| *n != name);
        settings.constants.push((name.clone(), value));
        return match save_settings_file(&settings) {
            Ok(()) => {
                set_custom_constants(settings.constants.clone());
                println!(
                    "Constant {name} = {} {} in the settings file. Use it in the Expression Calculator: CalcDex expr \"{name} * 2\"",
                    calcdex::n(value),
                    if replacing { "replaced" } else { "saved" }
                );
                0
            }
            Err(e) => {
                eprintln!("Could not save settings: {e}");
                1
            }
        };
    }
    if args[0] == "--favourites" {
        // Favourites + recent from the SAME settings file the window
        // uses — it is the shared source of truth for both.
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let settings = load_settings(&valid);
        println!(
            "Settings file: {}",
            calcdex::settings::settings_path().display()
        );
        if settings.favourites.is_empty() {
            println!("No favourites yet. Add one: CalcDex --favourite <calculator-id>");
        } else {
            println!("Favourites ({}):", settings.favourites.len());
            for id in &settings.favourites {
                let name = calcs
                    .iter()
                    .find(|c| c.id == id.as_str())
                    .map(|c| c.name)
                    .unwrap_or("");
                println!("  {:<38} {}", id, name);
            }
        }
        if settings.recent.is_empty() {
            println!("No recent calculators yet — run any calculator and it is remembered here.");
        } else {
            println!("Recent (most recent first):");
            for id in &settings.recent {
                let name = calcs
                    .iter()
                    .find(|c| c.id == id.as_str())
                    .map(|c| c.name)
                    .unwrap_or("");
                println!("  {:<38} {}", id, name);
            }
        }
        println!(
            "Remembered inputs: {} calculator(s) have saved inputs in the settings file.",
            settings.remembered.len()
        );
        return 0;
    }
    if args[0] == "--favourite" || args[0] == "--unfavourite" {
        let add = args[0] == "--favourite";
        let Some(id) = args.get(1) else {
            eprintln!("Usage: CalcDex {} <calculator-id>", args[0]);
            return 2;
        };
        let Some(calc) = calcs.iter().find(|c| c.id == id.as_str()) else {
            eprintln!("No calculator called '{id}'. Try: CalcDex --list");
            return 2;
        };
        let valid: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        let mut settings = load_settings(&valid);
        if add {
            settings.favourites.insert(id.clone());
        } else {
            settings.favourites.remove(id);
        }
        return match save_settings_file(&settings) {
            Ok(()) => {
                println!(
                    "{} {} the favourites.",
                    calc.name,
                    if add { "added to" } else { "removed from" }
                );
                0
            }
            Err(e) => {
                eprintln!("Could not save settings: {e}");
                1
            }
        };
    }
    if args[0] == "--help" || args[0] == "help" || args[0] == "-h" {
        println!("CalcDex \u{2014} every kind of calculation, offline.\n\nNo arguments: opens the window.\n--list                       list all calculators\n--search <words>             find calculators (ranked, with synonyms: 'home loan' finds the mortgage calculator)\n--describe <id>              show a calculator's fields, hints and example\n--version                    print the version and exit\n--clean                      delete CalcDex's own files (settings, calculation log, class examples and their backups) from the data folder; asks first unless --silent\n--uninstall                  like --clean, and explains how the program files themselves are removed (the installer's uninstaller, or deleting the portable folder)\n--dry-run                    with --clean/--uninstall: list what would be removed, remove nothing\n--silent                     with --clean/--uninstall: skip the confirmation prompt\n--json                       with a run, --version, --clean or --uninstall: print the result as one JSON object on stdout\n--help-guide                 print the built-in help guide (same text as the window's HELP panel / F1)\n--log                        print the calculation log (every successful run, window or command line, is saved there)\n--clear-log                  empty the calculation log (a backup is written first)\n--restore-log                undo the last --clear-log (restores the backup)\n--log-tag <n> <tag>          tag log entry #n (numbers as --log shows them, 1 = newest; the log is backed up before the edit)\n--log-note <n> <note...>     annotate log entry #n (same numbering, same pre-edit backup)\n--lab-export <file>          export the whole log as a plain-text lab record (oldest first, tags and notes included)\n--log-search <text>          print the log entries whose name, inputs, result, tag or note contain the text (entry numbers stay the global ones --log shows)\n--log-filter <key>=<value>   print the log entries matching calc=<id>, tag=<tag>, date=<YYYY-MM-DD>, pinned=yes or text=<words>; several pairs combine\n--log-pin <n>                pin log entry #n so the 500-entry trim never drops it (the log is backed up before the edit)\n--log-unpin <n>              remove entry #n's pin again\n--log-export <file> [flags]  export the entries matching the flags as plain text (flags: --search <text>, --calc <id>, --tag <tag>, --date <YYYY-MM-DD>, --pinned; no flags = the whole log)\n--materials [name or code]   list the built-in materials table (typical density, resistivity, Young's modulus) or look one material up\n--constants                  list the constants library: the CODATA 2018 built-ins plus your own defined constants\n--define <name>=<value>      define your own constant for the Expression Calculator (saved in the shared settings file; built-in names are refused)\n--check-update               ask github.com for the newest release and report it (never downloads anything)\n--favourites                 list favourites + recent (from the shared settings file)
--pack <GCSE|A-Level|AP|Undergrad> [--list]  list one course pack's calculators (item #24)
--projection [on|off]        show or set classroom projection mode in the shared settings file (the window renders about 1.5x with higher contrast)
--demo [--reveal]            with a run: print the working with result values hidden (demonstration mode); --reveal shows them. The log always stores the real result
set new <file> [name...]     create an empty problem set file
set add <file> <id> <inputs...>  add one problem (calculator + its inputs) to a set file
set key <file> <key-file>    write the teacher answer key for a set (answers + one-line working)
set worksheet <file> <out>   write the student worksheet for a set (problems + working space, no answers)
set generate <file> --seed <n> --count <k> <id> [more ids...] [--name <name...>]  generate a set from a seed — the same seed always makes the identical set
set open <file>              list a set file's problems (never answers)
--mark <set-file> <answers.csv> [marked.csv]  mark a class CSV (name, then answers in problem order) against a set; prints per-student totals
--formula-sheet <Category|All> <file>  write a printable formula sheet (formula + symbol meanings per calculator)
--save-example \"<title>\" [--note \"<note>\"]  with a run: save it as a named class example in the examples library
--examples                   list the saved class examples
--open-example \"<title>\"     print a saved example's problem + worked solution\n--favourite <id>             add a calculator to the favourites in the settings file\n--unfavourite <id>           remove it again\n--batch <id> <in.csv> <out.csv>  run one calculator over a CSV file (header row names the fields; see README). An existing output file needs --force (backed up to .bak first)\n--force                      allow --batch to overwrite its output file (with backup)\n--sig-figs <n>               also show the headline rounded to n significant figures (display only; may appear anywhere)\n--decimals <n>               also show the headline to n decimal places (display only)\n--format <mode[:n]>         the unified display format: off, sig-figs:3, decimals:2, engineering, auto (auto = precision inferred from your inputs), exact (fractions labelled exact, pi geometry also shows its exact form like 25*pi, rational expressions worked out exactly)\n--steps <standard|deeper>  deeper steps: the formula, the formula with your values in it, the working, and the result, in labelled stages\n--rounding <mode>            half-up, bankers or truncate — applies to the display formats above\n--as <format>                print a run's result as plain, latex, scientific or csv\n--spec <min> <max>           after a run, print PASS/FAIL for each result line against these limits (limits included)\n<calculator-id> <numbers...> run one calculator; numbers in the order of its labelled inputs\n<calculator-id> --example    run the calculator's built-in teaching example\n<calculator-id> --vary <field> <start> <end> <step> <other numbers...>\n                             sweep one input (field numbered from 0, as in --describe);\n                             the other numbers are the remaining inputs in order\nexpr \"<expression>\"          evaluate a free expression, e.g. CalcDex expr \"sqrt(2)/2\" (--degrees/--radians choose the trig mode; degrees is the default)\n<calculator-id> <numbers> --check <answer>\n                             check your own answer against the run (a number, or a fraction like 3/4 for the fraction calculators)\n<calculator-id> <numbers> --export <path>\n                             save the worked solution (name, formula, inputs, steps, result) as a plain-text file\n<calculator-id> --practice --seed <n> [--answer <x>]\n                             print a practice problem made from the seed; with --answer, check your answer and show the worked solution\nexpr \"<expression in x>\" --plot <from> <to>\n                             print the plot as CSV x,y sample points (the command-line form of the plot; a blank y means the function is not defined there)\n\nDates: a date is entered as ONE argument in MM/DD/YYYY form \u{2014} for example\n  CalcDex age 04/23/1988 10/25/2026\nThe older three-number form still works: CalcDex age 1988 4 23 2026 10 25\n\nNumbers may use scientific notation (e.g. 6.022e23, 1.5e-3), and many fields accept typed quantities in quotes (e.g. CalcDex bmi \"154 lb\" \"5 ft 9 in\") — they are converted into the field's unit. Results show the formula, substituted values, and full precision.\n\nExpressions may use the CODATA constants by name (CalcDex expr \"h*c/500e-9\"), your own --define constants, and measurements with uncertainties (CalcDex expr \"10 \u{b1} 0.2 + 5 \u{b1} 0.1\" — also written 10 +/- 0.2): the uncertainty is carried through every operation and the propagation steps are shown. List calculators accept values separated by commas, spaces, tabs, semicolons or new lines — a pasted column in quotes works as one argument.\n\nExample: CalcDex compound-interest 1000 5 10");
        return 0;
    }
    // expr "<expression>" (item #5): free-expression shortcut alias.
    if args[0] == "expr" {
        let mut degrees = true;
        let parts: Vec<String> = args[1..]
            .iter()
            .filter(|a| {
                if a.as_str() == "--degrees" {
                    degrees = true;
                    false
                } else if a.as_str() == "--radians" {
                    degrees = false;
                    false
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        if parts.is_empty() {
            eprintln!("Usage: CalcDex expr \"<expression>\" [--degrees|--radians]");
            return 2;
        }
        let text = parts.join(" ");
        let mode = if degrees {
            AngleMode::Degrees
        } else {
            AngleMode::Radians
        };
        if let Some((from, to)) = plot {
            return match plot_csv(&text, from, to, 100, mode) {
                Ok(csv) => {
                    print!("{csv}");
                    0
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            };
        }
        return cli_run_text(
            "expression-calculator",
            &text,
            degrees,
            &display,
            copy_format,
            spec,
            &check_ans,
            &export_path,
            demo,
            reveal,
            json,
        );
    }
    let id = &args[0];
    let Some(calc) = calcs.iter().find(|c| c.id == id.as_str()) else {
        eprintln!("No calculator called '{id}'. Try: CalcDex --list");
        return 2;
    };
    // Text calculator by id (item #5): the remaining arguments,
    // joined, are the expression; --degrees/--radians pick the
    // trig mode (degrees is the default).
    if calc.text_run.is_some() {
        let mut degrees = true;
        let mut parts: Vec<String> = Vec::new();
        let mut example = false;
        for a in &args[1..] {
            match a.as_str() {
                "--degrees" => degrees = true,
                "--radians" => degrees = false,
                "--example" => example = true,
                _ => parts.push(a.clone()),
            }
        }
        let text = if example {
            calc.example_text.to_string()
        } else {
            parts.join(" ")
        };
        if text.trim().is_empty() {
            eprintln!("Usage: CalcDex {} \"<expression>\" [--degrees|--radians], or CalcDex expr \"<expression>\"", calc.id);
            return 2;
        }
        let mode = if degrees {
            AngleMode::Degrees
        } else {
            AngleMode::Radians
        };
        if let Some((from, to)) = plot {
            return match plot_csv(&text, from, to, 100, mode) {
                Ok(csv) => {
                    print!("{csv}");
                    0
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            };
        }
        return cli_run_text(
            id,
            &text,
            degrees,
            &display,
            copy_format,
            spec,
            &check_ans,
            &export_path,
            demo,
            reveal,
            json,
        );
    }
    // <id> --practice --seed N [--answer X] (v0.5.0 item #15):
    // the problem is re-made from the seed for the check, so the
    // CLI form is stateless exactly like the window's tally note.
    if practice {
        let Some(seed) = seed else {
            eprintln!("--practice needs --seed <n> so the same problem can be re-made for checking, e.g. CalcDex {id} --practice --seed 7");
            return 2;
        };
        if !practice_eligible(calc) {
            eprintln!(
                "'{}' is not used for practice problems. {PRACTICE_EXCLUSION_NOTE}",
                calc.name
            );
            return 2;
        }
        let Some(p) = practice_problem_seeded(calc, seed) else {
            eprintln!(
                "Could not make a practice problem for '{}' with seed {seed}.",
                calc.name
            );
            return 1;
        };
        println!("Practice — {} (seed {seed})", calc.name);
        println!("{}", p.question);
        if let Some(ans) = &answer_ans {
            return match run_calc(calc.id, &p.args) {
                Ok(out) => {
                    match check_answer(calc, &out, ans) {
                        Ok(o) => println!("{}", o.message),
                        Err(e) => {
                            eprintln!("Check: {e}");
                            return 2;
                        }
                    }
                    println!("\nWorked solution:\n{out}");
                    0
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            };
        }
        return 0;
    }
    // <id> --example (item #11)
    if args.get(1).map(|s| s.as_str()) == Some("--example") {
        let nums = calc.example.to_vec();
        return match run_calc(id, &nums) {
            Ok(out) => {
                if !json {
                    println!(
                        "Example — {} with inputs: {}",
                        calc.name,
                        display_tokens(calc, &nums).join(", ")
                    );
                }
                let auto_inputs: Vec<String> = display_tokens(calc, &nums);
                let mut rc = cli_after_run(
                    calc,
                    &nums,
                    &out,
                    &display,
                    copy_format,
                    spec,
                    steps_deeper,
                    &check_ans,
                    &export_path,
                    &auto_inputs,
                    demo,
                    reveal,
                    json,
                );
                let entry = make_entry(calc, &nums, &out);
                let _ = append_history(entry);
                cli_note_run(calc, &nums);
                if let Some(title) = &save_example {
                    let rc2 = cli_save_example(title, &ex_note, calc, &nums);
                    if rc2 != 0 {
                        rc = rc2;
                    }
                }
                rc
            }
            Err(e) => {
                if json {
                    println!(
                        "{{\"ok\":false,\"calculator\":\"{}\",\"error\":\"{}\"}}",
                        json_escape(id),
                        json_escape(&e)
                    );
                }
                eprintln!("Error: {e}");
                1
            }
        };
    }
    // <id> --vary <field> <start> <end> <step> <other numbers...> (item #16)
    if args.get(1).map(|s| s.as_str()) == Some("--vary") {
        let spec: Vec<&String> = args.iter().skip(2).collect();
        if spec.len() < 4 {
            eprintln!("Usage: CalcDex {id} --vary <field> <start> <end> <step> <other numbers...>");
            return 2;
        }
        let parse = |s: &String| s.parse::<f64>();
        let (Ok(field), Ok(start), Ok(end), Ok(step)) = (
            spec[0].parse::<usize>(),
            parse(spec[1]),
            parse(spec[2]),
            parse(spec[3]),
        ) else {
            eprintln!("--vary needs: a field number, then start, end and step as numbers.");
            return 2;
        };
        let mut others: Vec<f64> = Vec::new();
        for s in &spec[4..] {
            match s.parse::<f64>() {
                Ok(v) => others.push(v),
                Err(_) => {
                    eprintln!("Error: '{}' is not a number.", s);
                    return 2;
                }
            }
        }
        if calc.list || others.len() + 1 != calc.inputs.len() {
            eprintln!(
                "Error: '{}' needs {} other number(s) around the varied field.",
                calc.name,
                calc.inputs.len().saturating_sub(1)
            );
            return 2;
        }
        let mut base = others.clone();
        if field > base.len() {
            eprintln!("Error: there is no field {field} on '{}'.", calc.name);
            return 2;
        }
        base.insert(field, 0.0);
        return match vary_run(calc, &base, field, start, end, step) {
            Ok(rows) => {
                println!("{}", csv_row(&["value".to_string(), "result".to_string()]));
                for r in &rows {
                    let cell = match &r.result {
                        Ok(out) => headline_of(out).to_string(),
                        Err(e) => format!("ERROR: {e}"),
                    };
                    println!("{}", csv_row(&[calcdex::n(r.value), cell]));
                }
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }
    let nums = match expand_args(calc, &args[1..]) {
        Ok(v) => v,
        Err(e) => {
            if json {
                println!(
                    "{{\"ok\":false,\"calculator\":\"{}\",\"error\":\"{}\"}}",
                    json_escape(id),
                    json_escape(&e)
                );
            }
            eprintln!("Error: {e}");
            return 2;
        }
    };
    match run_calc(id, &nums) {
        Ok(out) => {
            let mut rc = cli_after_run(
                calc,
                &nums,
                &out,
                &display,
                copy_format,
                spec,
                steps_deeper,
                &check_ans,
                &export_path,
                &args[1..],
                demo,
                reveal,
                json,
            );
            // Every successful CLI run is saved to the same log the window uses.
            let entry = make_entry(calc, &nums, &out);
            let _ = append_history(entry);
            cli_note_run(calc, &nums);
            if let Some(title) = &save_example {
                let rc2 = cli_save_example(title, &ex_note, calc, &nums);
                if rc2 != 0 {
                    rc = rc2;
                }
            }
            rc
        }
        Err(e) => {
            if json {
                println!(
                    "{{\"ok\":false,\"calculator\":\"{}\",\"error\":\"{}\"}}",
                    json_escape(id),
                    json_escape(&e)
                );
            }
            eprintln!("Error: {e}");
            1
        }
    }
}

/// Escape a string for a JSON string literal. Hand-rolled: the
/// crate set carries no JSON dependency and the CLI's `--json`
/// output is small flat objects.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// `--clean` / `--uninstall` (v0.9.0): remove CalcDex's own
/// files — settings, the calculation log, the class examples and
/// their backups — from the CURRENT data location only (see
/// `calcdex::paths`). Program files are never touched: an
/// installed build is removed by its installer's uninstaller and
/// a portable build by deleting its folder, which `--uninstall`
/// says out loud. Refuses while a CalcDex window is running.
/// Without `--silent` it asks on stdin first (the prompt goes to
/// stderr, so `--json` output on stdout stays parseable). Exit
/// codes match the rest of the CLI: 0 = done / nothing to do /
/// the user said no, 1 = refused or a removal failed.
fn cli_clean(uninstall: bool, dry_run: bool, silent: bool, json: bool) -> i32 {
    let action = if uninstall { "uninstall" } else { "clean" };
    let note = "Program files are not removed: use the installer's uninstaller, or delete the portable folder.";
    if win32::another_instance_running() {
        if json {
            println!(
                "{{\"ok\":false,\"action\":\"{action}\",\"error\":\"{}\"}}",
                json_escape("CalcDex is running — close the window first, then try again.")
            );
        } else {
            eprintln!("CalcDex is running — close the window first, then try again.");
        }
        return 1;
    }
    let dir = calcdex::paths::data_dir();
    let plan = calcdex::paths::purge_candidates(&dir);
    let names: Vec<String> = plan
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let names_json = names
        .iter()
        .map(|n| format!("\"{}\"", json_escape(n)))
        .collect::<Vec<_>>()
        .join(",");
    let dir_json = json_escape(&dir.to_string_lossy());
    if plan.is_empty() {
        if json {
            println!(
                "{{\"ok\":true,\"action\":\"{action}\",\"data_dir\":\"{dir_json}\",\"removed\":[]}}"
            );
        } else {
            println!("Nothing to remove — no CalcDex files in {}.", dir.display());
            if uninstall {
                println!("{note}");
            }
        }
        return 0;
    }
    if dry_run {
        if json {
            println!(
                "{{\"ok\":true,\"action\":\"{action}\",\"dry_run\":true,\"data_dir\":\"{dir_json}\",\"would_remove\":[{names_json}]}}"
            );
        } else {
            println!(
                "Would remove {} file(s) from {}:",
                plan.len(),
                dir.display()
            );
            for p in &plan {
                println!("  {}", p.display());
            }
            println!("Nothing was removed (--dry-run).");
            if uninstall {
                println!("{note}");
            }
        }
        return 0;
    }
    if !silent {
        if !json {
            println!(
                "About to remove {} file(s) from {}:",
                plan.len(),
                dir.display()
            );
            for p in &plan {
                println!("  {}", p.display());
            }
        }
        eprint!("Remove these files? [y/N] ");
        use std::io::Write;
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        if !matches!(line.trim(), "y" | "Y" | "yes" | "Yes" | "YES") {
            if json {
                println!(
                    "{{\"ok\":true,\"action\":\"{action}\",\"cancelled\":true,\"removed\":[]}}"
                );
            } else {
                println!("Nothing removed.");
            }
            return 0;
        }
    }
    let mut removed: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for (p, name) in plan.iter().zip(names.iter()) {
        match std::fs::remove_file(p) {
            Ok(()) => removed.push(name.clone()),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    let ok = errors.is_empty();
    if json {
        let removed_json = removed
            .iter()
            .map(|n| format!("\"{}\"", json_escape(n)))
            .collect::<Vec<_>>()
            .join(",");
        let errors_json = errors
            .iter()
            .map(|n| format!("\"{}\"", json_escape(n)))
            .collect::<Vec<_>>()
            .join(",");
        let note_json = if uninstall {
            format!(",\"note\":\"{}\"", json_escape(note))
        } else {
            String::new()
        };
        println!(
            "{{\"ok\":{ok},\"action\":\"{action}\",\"data_dir\":\"{dir_json}\",\"removed\":[{removed_json}],\"errors\":[{errors_json}]{note_json}}}"
        );
    } else {
        for name in &removed {
            println!("Removed {name}");
        }
        for e in &errors {
            eprintln!("Could not remove {e}");
        }
        println!(
            "Removed {} of {} file(s) from {}.",
            removed.len(),
            plan.len(),
            dir.display()
        );
        if uninstall {
            println!("{note}");
        }
    }
    if ok {
        0
    } else {
        1
    }
}

/// Save a run as a named class example (#28: --save-example)
/// in the shared examples library — the same file the window's
/// SETS view reads. Saving an existing title replaces it.
fn cli_save_example(title: &str, note: &str, calc: &Calc, nums: &[f64]) -> i32 {
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let mut list = load_examples(&valid);
    upsert_example(
        &mut list,
        SavedExample {
            title: title.to_string(),
            calc_id: calc.id.to_string(),
            note: note.to_string(),
            inputs: display_tokens(calc, nums),
        },
    );
    match calcdex::teachers::save_examples(&list) {
        Ok(()) => {
            println!(
                "Saved example '{title}' ({}) in the examples library: {}",
                calc.name,
                calcdex::teachers::examples_path().display()
            );
            0
        }
        Err(e) => {
            eprintln!("Could not save the example: {e}");
            1
        }
    }
}

/// Write a generated teacher document (set / key / worksheet /
/// marked CSV), refusing to overwrite an existing file unless
/// --force was given — and then backing the old file up to
/// <name>.bak first (the destructive-action rule, same as --batch).
/// The log export's starting path (item #49):
/// calcdex-log-export.txt beside the history file.
fn default_log_export_path() -> String {
    match history_path().parent() {
        Some(dir) => dir
            .join("calcdex-log-export.txt")
            .to_string_lossy()
            .into_owned(),
        None => "calcdex-log-export.txt".to_string(),
    }
}

fn write_guarded(path: &str, text: &str, force: bool) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if p.exists() {
        if !force {
            return Err(format!(
                "{path} already exists — refusing to overwrite it. Add --force to replace it (the old file is backed up to {path}.bak first)."
            ));
        }
        let bak = format!("{path}.bak");
        std::fs::copy(p, &bak).map_err(|e| format!("Could not back up {path} to {bak}: {e}"))?;
    }
    std::fs::write(p, text).map_err(|e| format!("Could not write {path}: {e}"))
}

/// The `set ...` verbs (items #22/#23/#27/#30): new / add / key /
/// worksheet / generate / open over problem-set files.
/// `seed` is the global --seed value (required by generate).
fn cli_set(args: &[String], force: bool, seed: Option<u64>) -> i32 {
    let usage = "Usage:\n  CalcDex set new <file> [set name...]\n  CalcDex set add <file> <calculator-id> <inputs...>\n  CalcDex set key <file> <key-file>\n  CalcDex set worksheet <file> <worksheet-file>\n  CalcDex set generate <file> --seed <n> --count <k> <calculator-id> [more ids...] [--name <set name...>]\n  CalcDex set open <file>";
    let Some(verb) = args.first().map(|s| s.as_str()) else {
        eprintln!("{usage}");
        return 2;
    };
    let need = |i: usize, what: &str| -> Result<String, i32> {
        args.get(i).cloned().ok_or_else(|| {
            eprintln!("set {verb} needs {what}.\n{usage}");
            2
        })
    };
    match verb {
        "new" => {
            let file = match need(1, "a file path") {
                Ok(v) => v,
                Err(rc) => return rc,
            };
            let name = if args.len() > 2 {
                args[2..].join(" ")
            } else {
                "Untitled set".to_string()
            };
            let set = ProblemSet::new(&name);
            match write_guarded(&file, &set_to_text(&set), force) {
                Ok(()) => {
                    println!("Created empty problem set '{name}' in {file}. Add problems: CalcDex set add {file} <calculator-id> <inputs...>");
                    0
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            }
        }
        "add" => {
            let file = match need(1, "a set file") {
                Ok(v) => v,
                Err(rc) => return rc,
            };
            let id = match need(2, "a calculator id") {
                Ok(v) => v,
                Err(rc) => return rc,
            };
            if args.len() < 4 {
                eprintln!("set add needs at least one input for the calculator.\n{usage}");
                return 2;
            }
            let inputs: Vec<String> = args[3..].to_vec();
            let mut set = match read_set(std::path::Path::new(&file)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return 1;
                }
            };
            match set_add(&mut set, &id, &inputs) {
                Ok(()) => {
                    let line = set
                        .entries
                        .last()
                        .and_then(|e| calcdex::teachers::entry_problem_line(e).ok());
                    match write_set(std::path::Path::new(&file), &set) {
                        Ok(()) => {
                            println!(
                                "Added problem {} — {}",
                                set.entries.len(),
                                line.unwrap_or(id)
                            );
                            0
                        }
                        Err(e) => {
                            eprintln!("Could not write {file}: {e}");
                            1
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            }
        }
        "key" | "worksheet" => {
            let file = match need(1, "a set file") {
                Ok(v) => v,
                Err(rc) => return rc,
            };
            let out = match need(2, "an output file") {
                Ok(v) => v,
                Err(rc) => return rc,
            };
            let set = match read_set(std::path::Path::new(&file)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return 1;
                }
            };
            let (text, what) = if verb == "key" {
                (answer_key_text(&set), "Answer key")
            } else {
                (worksheet_text(&set), "Worksheet")
            };
            match write_guarded(&out, &text, force) {
                Ok(()) => {
                    println!("{what} for '{}' written to {out}.", set.name);
                    0
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            }
        }
        "generate" => {
            let file = match need(1, "a file path") {
                Ok(v) => v,
                Err(rc) => return rc,
            };
            let Some(seed) = seed else {
                eprintln!("set generate needs --seed <n> (before or after the verb), e.g. CalcDex set generate class.txt --seed 7 --count 10 circle percent-of");
                return 2;
            };
            let mut count: Option<usize> = None;
            let mut name: Option<String> = None;
            let mut ids: Vec<String> = Vec::new();
            let rest = &args[2..];
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--count" => match rest.get(i + 1).and_then(|v| v.parse::<usize>().ok()) {
                        Some(k) => {
                            count = Some(k);
                            i += 2;
                        }
                        None => {
                            eprintln!("--count needs a whole number, e.g. --count 10.");
                            return 2;
                        }
                    },
                    "--name" => {
                        name = Some(rest[i + 1..].join(" "));
                        break;
                    }
                    other => {
                        ids.push(other.to_string());
                        i += 1;
                    }
                }
            }
            let Some(count) = count else {
                eprintln!("set generate needs --count <k>.\n{usage}");
                return 2;
            };
            let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
            let name = name.unwrap_or_else(|| format!("Problem set from seed {seed}"));
            match generate_set(&id_refs, seed, count, &name) {
                Ok(set) => match write_guarded(&file, &set_to_text(&set), force) {
                    Ok(()) => {
                        println!(
                            "Generated {} problem(s) from seed {seed} into {file}.",
                            set.entries.len()
                        );
                        0
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        1
                    }
                },
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            }
        }
        "open" => {
            let file = match need(1, "a set file") {
                Ok(v) => v,
                Err(rc) => return rc,
            };
            match read_set(std::path::Path::new(&file)) {
                Ok(set) => {
                    print!("{}", set_listing(&set));
                    0
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    1
                }
            }
        }
        _ => {
            eprintln!("Unknown set verb '{verb}'.\n{usage}");
            2
        }
    }
}

/// Print a result, adding the sig-figs headline showing when the
/// global --sig-figs flag is on (item #1). The log always stores
/// the un-annotated full-precision text.
/// A successful CLI run updates the SAME settings file the window
/// uses: the calculator joins Recent and its inputs are remembered
/// (items #46/#48 — the settings file is the shared source of truth).
/// Best-effort: a settings write failure never fails the calculation.
fn cli_note_run(calc: &Calc, nums: &[f64]) {
    let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
    let mut settings = load_settings(&valid);
    settings.note_run(calc.id, display_tokens(calc, nums));
    let _ = save_settings_file(&settings);
}

/// Shared post-run presentation for a numeric run: deeper-steps
/// staging (#12), the display format / copy format / spec lines,
/// the Exact-mode line (#14), the check verdict (#13) and the
/// worked-solution export (#18). Demonstration mode (#25: --demo
/// without --reveal) masks every result value in what is printed
/// and exported — the exact line and the check are skipped, since
/// both would reveal the answer. Returns 1 only when an export
/// write failed, else 0.
#[allow(clippy::too_many_arguments)]
fn cli_after_run(
    calc: &Calc,
    nums: &[f64],
    out: &str,
    display: &DisplayFormat,
    copy_format: Option<CopyFormat>,
    spec: Option<(f64, f64)>,
    steps_deeper: bool,
    check_ans: &Option<String>,
    export_path: &Option<String>,
    auto_inputs: &[String],
    demo: bool,
    reveal: bool,
    json: bool,
) -> i32 {
    if json {
        // --json: one machine-readable object on stdout; the
        // human decorations (display annotation, spec verdicts,
        // exact line, check verdict) are skipped. Demo mode
        // still masks the result. The export side effect is
        // honoured with the real text, exactly like the log.
        let shown = if demo && !reveal {
            mask_result_text(out, nums)
        } else {
            out.to_string()
        };
        let inputs: Vec<String> = nums.iter().map(|v| format!("{v}")).collect();
        println!(
            "{{\"ok\":true,\"calculator\":\"{}\",\"name\":\"{}\",\"inputs\":[{}],\"result\":\"{}\"}}",
            json_escape(calc.id),
            json_escape(calc.name),
            inputs.join(","),
            json_escape(&shown)
        );
        if let Some(path) = export_path {
            let text = solution_text(calc, nums, out, steps_deeper);
            return match std::fs::write(path, text) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("Could not write the solution file {path}: {e}");
                    1
                }
            };
        }
        return 0;
    }
    if demo && !reveal {
        if steps_deeper {
            println!("{}", mask_result_text(&deeper_steps(calc, nums, out), nums));
        } else {
            println!("{}", mask_result_text(out, nums));
        }
        println!("(Demonstration mode: result values are hidden — add --reveal to show them. The calculation log stores the real result.)");
        if let Some(path) = export_path {
            let text = solution_text(calc, nums, &mask_result_text(out, nums), false);
            return match std::fs::write(path, text) {
                Ok(()) => {
                    println!("Worked solution (answers hidden) written to {path}.");
                    0
                }
                Err(e) => {
                    eprintln!("Could not write the solution file {path}: {e}");
                    1
                }
            };
        }
        return 0;
    }
    if steps_deeper {
        println!("{}", deeper_steps(calc, nums, out));
        if let Some((mn, mx)) = spec {
            for line in spec_verdict_lines(out, mn, mx) {
                println!("{line}");
            }
        }
    } else {
        cli_print(out, display, copy_format, spec, auto_inputs);
    }
    if display.mode == DisplayMode::Exact && copy_format.is_none() {
        println!("{}", exact_line(calc, nums));
    }
    if let Some(ans) = check_ans {
        match check_answer(calc, out, ans) {
            Ok(o) => println!("{}", o.message),
            Err(e) => eprintln!("Check: {e}"),
        }
    }
    if let Some(path) = export_path {
        let text = solution_text(calc, nums, out, steps_deeper);
        return match std::fs::write(path, text) {
            Ok(()) => {
                println!("Worked solution written to {path}.");
                0
            }
            Err(e) => {
                eprintln!("Could not write the solution file {path}: {e}");
                1
            }
        };
    }
    0
}

/// Print a run's result with the global display options applied:
/// `--as` re-formats the whole text (#4); otherwise the display
/// format adds its one annotation line (#1/#2/#3/#37); `--spec`
/// verdicts follow (#10). The log always stores the plain,
/// full-precision text.
fn cli_print(
    out: &str,
    display: &DisplayFormat,
    copy_format: Option<CopyFormat>,
    spec: Option<(f64, f64)>,
    auto_inputs: &[String],
) {
    match copy_format {
        Some(f) => println!("{}", format_copy(out, f)),
        None => {
            if display.is_off() {
                println!("{out}");
            } else {
                println!("{}", annotate_display(out, display, auto_inputs));
            }
        }
    }
    if let Some((mn, mx)) = spec {
        for line in spec_verdict_lines(out, mn, mx) {
            println!("{line}");
        }
    }
}

/// Run a text calculator from the CLI (item #5) and log it: the
/// log is the expression calculator's paper tape.
#[allow(clippy::too_many_arguments)]
fn cli_run_text(
    id: &str,
    text: &str,
    degrees: bool,
    display: &DisplayFormat,
    copy_format: Option<CopyFormat>,
    spec: Option<(f64, f64)>,
    check_ans: &Option<String>,
    export_path: &Option<String>,
    demo: bool,
    reveal: bool,
    json: bool,
) -> i32 {
    let Some(calc) = all_calcs().into_iter().find(|c| c.id == id) else {
        eprintln!("No calculator called '{id}'.");
        return 2;
    };
    match run_text_calc(id, text, degrees) {
        Ok(out) => {
            if demo && !reveal {
                // Demonstration mode (#25): an expression's result
                // values are masked (an expression has no separate
                // input fields, so every number in its output is a
                // result value); exact/check are skipped — both
                // would reveal the answer.
                let masked = mask_result_text(&out, &[]);
                if json {
                    println!(
                        "{{\"ok\":true,\"calculator\":\"{}\",\"name\":\"{}\",\"input\":\"{}\",\"result\":\"{}\"}}",
                        json_escape(calc.id),
                        json_escape(calc.name),
                        json_escape(text),
                        json_escape(&masked)
                    );
                } else {
                    println!("{masked}");
                    println!("(Demonstration mode: result values are hidden — add --reveal to show them. The calculation log stores the real result.)");
                }
                let mut rc = 0;
                if let Some(path) = export_path {
                    let text_out = solution_text_for_text(calc.name, text, &masked);
                    match std::fs::write(path, text_out) {
                        Ok(()) => println!("Worked solution (answers hidden) written to {path}."),
                        Err(e) => {
                            eprintln!("Could not write the solution file {path}: {e}");
                            rc = 1;
                        }
                    }
                }
                let entry = LogEntry {
                    timestamp: now_stamp(),
                    id: calc.id.to_string(),
                    name: calc.name.to_string(),
                    inputs: text.to_string(),
                    result: out.clone(),
                    restore: text.to_string(),
                    tag: String::new(),
                    note: String::new(),
                    pinned: false,
                };
                let _ = append_history(entry);
                return rc;
            }
            let mut rc = 0;
            if json {
                // --json: one object on stdout; the human
                // decorations (exact line, check verdict) are
                // skipped, the export + log still happen below.
                println!(
                    "{{\"ok\":true,\"calculator\":\"{}\",\"name\":\"{}\",\"input\":\"{}\",\"result\":\"{}\"}}",
                    json_escape(calc.id),
                    json_escape(calc.name),
                    json_escape(text),
                    json_escape(&out)
                );
            } else {
                cli_print(&out, display, copy_format, spec, &[]);
                if display.mode == DisplayMode::Exact && copy_format.is_none() {
                    let mode = if degrees {
                        AngleMode::Degrees
                    } else {
                        AngleMode::Radians
                    };
                    println!("{}", exact_line_for_expression(text, mode));
                }
                if let Some(ans) = check_ans {
                    match check_answer(&calc, &out, ans) {
                        Ok(o) => println!("{}", o.message),
                        Err(e) => eprintln!("Check: {e}"),
                    }
                }
            }
            if let Some(path) = export_path {
                let text_out = solution_text_for_text(calc.name, text, &out);
                match std::fs::write(path, text_out) {
                    Ok(()) => println!("Worked solution written to {path}."),
                    Err(e) => {
                        eprintln!("Could not write the solution file {path}: {e}");
                        rc = 1;
                    }
                }
            }
            let entry = LogEntry {
                timestamp: now_stamp(),
                id: calc.id.to_string(),
                name: calc.name.to_string(),
                inputs: text.to_string(),
                result: out.clone(),
                restore: text.to_string(),
                tag: String::new(),
                note: String::new(),
                pinned: false,
            };
            let _ = append_history(entry);
            // Remember the expression + recency in the shared settings.
            let valid: BTreeSet<String> = all_calcs().iter().map(|c| c.id.to_string()).collect();
            let mut settings = load_settings(&valid);
            settings.note_run(calc.id, vec![text.to_string()]);
            let _ = save_settings_file(&settings);
            rc
        }
        Err(e) => {
            if json {
                println!(
                    "{{\"ok\":false,\"calculator\":\"{}\",\"error\":\"{}\"}}",
                    json_escape(id),
                    json_escape(&e)
                );
            }
            eprintln!("Error: {e}");
            1
        }
    }
}

/// (label, hint) per user-visible field, for --describe: date groups
/// collapse to their merged label + the date hint.
fn describe_fields(calc: &Calc) -> Vec<(String, String)> {
    if calc.list {
        return vec![(
            calc.inputs
                .first()
                .copied()
                .unwrap_or("Numbers")
                .to_string(),
            hint_for(calc, 0),
        )];
    }
    let fields = display_fields(calc);
    let mut hints: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < calc.inputs.len() {
        if date_group_at(calc, i).is_some() {
            hints.push(hint_for(calc, i));
            i += 3;
        } else if calc
            .date_groups
            .iter()
            .any(|g| i > g.start && i < g.start + 3)
        {
            i += 1;
        } else {
            hints.push(hint_for(calc, i));
            i += 1;
        }
    }
    fields.into_iter().zip(hints).collect()
}

// ---------- Terminal theme (Purgeon exact values) ----------
// --bg #0c0c0c, --panel #121212, --raised #181818, --border #262626,
// --text #d0d0d0, --muted #7a7a7a, --green #4ade80, --green-sel #163322,
// --amber #e5c07b, --cyan #56b6c2, --violet #b294f0, --red #f26d6d, radius 0.
fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    for font in style.text_styles.values_mut() {
        font.family = egui::FontFamily::Monospace;
    }
    ctx.set_style(style);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(0x12, 0x12, 0x12);
    visuals.window_fill = egui::Color32::from_rgb(0x0c, 0x0c, 0x0c);
    visuals.extreme_bg_color = egui::Color32::from_rgb(0x0c, 0x0c, 0x0c);
    visuals.faint_bg_color = egui::Color32::from_rgb(0x18, 0x18, 0x18);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0x16, 0x33, 0x22);
    visuals.selection.stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x4a, 0xde, 0x80));
    visuals.hyperlink_color = egui::Color32::from_rgb(0x56, 0xb6, 0xc2);
    // Muted grey is reserved for true secondary text: field hint /
    // placeholder text (egui's "weak" colour), never for list content.
    visuals.weak_text_color = Some(MUTED);
    let zero = egui::CornerRadius::ZERO;
    for w in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        w.corner_radius = zero;
        w.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x26, 0x26, 0x26));
    }
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x18, 0x18, 0x18);
    // Hover must be clearly visible (v0.1.1 used #1a1a1a, barely
    // distinguishable from the #121212 panel); pressed gets green-dim.
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x2a, 0x2a, 0x2a);
    visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0xd0, 0xd0, 0xd0));
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0x1d, 0x3b, 0x27);
    visuals.widgets.active.fg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x4a, 0xde, 0x80));
    ctx.set_visuals(visuals);
    // Slightly larger base text (13 pt, top of the Terminal range) and a
    // consistent spacing rhythm. Sizes are in egui points, so they scale
    // together when the user zooms the UI (Ctrl and + / -).
    ctx.style_mut(|style| {
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::monospace(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::monospace(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::monospace(11.5));
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size = egui::vec2(48.0, 26.0);
    });
}

/// Classroom projection mode (v0.6.0 #21): the whole window is
/// rendered at 1.5x zoom and the muted/hint greys are brightened
/// (weak text via the visuals; HINT/MUTED label colours via the
/// App's hint_col()/muted_col()), on near-black panels — the
/// palette family is unchanged, only lifted. Called from update()
/// whenever the persisted setting changes (including at startup).
fn apply_projection(ctx: &egui::Context, on: bool) {
    configure_style(ctx);
    if on {
        let mut visuals = ctx.style().visuals.clone();
        visuals.weak_text_color = Some(MUTED_PROJ);
        visuals.panel_fill = egui::Color32::from_rgb(0x0a, 0x0a, 0x0a);
        visuals.window_fill = egui::Color32::BLACK;
        visuals.extreme_bg_color = egui::Color32::BLACK;
        ctx.set_visuals(visuals);
        ctx.set_zoom_factor(1.5);
    } else {
        ctx.set_zoom_factor(1.0);
    }
}

fn configure_fonts(ctx: &egui::Context) {
    // Prefer the real Cascadia Mono / Cascadia Code / Consolas font files
    // when the machine has them (Windows ships Cascadia with Windows
    // Terminal and Consolas always); load the file as font data, like
    // DevTextTools loads Segoe fonts. Otherwise egui's bundled monospace
    // is used — the family stays Monospace either way.
    let candidates = [
        ("cascadia", "C:\\Windows\\Fonts\\CascadiaMono.ttf"),
        ("cascadia", "C:\\Windows\\Fonts\\CascadiaCode.ttf"),
        ("consolas", "C:\\Windows\\Fonts\\consola.ttf"),
        (
            "dejavu-mono",
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        ),
    ];
    let mut fonts = egui::FontDefinitions::default();
    for (name, path) in candidates {
        if fonts.font_data.contains_key(name) {
            continue;
        }
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                name.to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
                if let Some(list) = fonts.families.get_mut(&family) {
                    list.insert(0, name.to_owned());
                }
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}

// ---------- App ----------
//
// v0.2.0 layout — Design 3 (SEAN's pick from the 8 research mockups,
// Microsoft-Calculator-inspired):
//   * a slim MODE RAIL on the far left: the categories are the modes,
//     each with its Alt+ shortcut hint, plus the search box on top;
//   * the CENTRE: a chip strip of the active mode's calculators, the
//     selected calculator's form, a prominent DISPLAY for the result,
//     and the worked steps as a footer block;
//   * a HISTORY rail docked on the right (the calculation log), which
//     collapses below the width threshold documented at
//     `HISTORY_RAIL_MIN_WIDTH` — there the header LOG toggle switches
//     the centre to the log view instead, as it did in v0.1.x.

/// One mode-rail entry: a category (or "All") as a mode.
struct Mode {
    /// Full category name ("All" or e.g. "Date & Time").
    name: String,
    /// Short label shown in the slim rail.
    short: &'static str,
    /// Shortcut hint printed on the row, e.g. "Alt+1".
    hint: String,
    /// The key that, with Alt held, selects this mode.
    key: egui::Key,
}

/// Curated mode order (the registry's own presentation order, not
/// alphabetical): general first, then the school/university subjects,
/// then the applied sets.
const MODE_ORDER: &[(&str, &str)] = &[
    ("All", "ALL"),
    ("Math", "MATH"),
    ("Finance", "FINANCE"),
    ("Health", "HEALTH"),
    ("Physics", "PHYSICS"),
    ("Chemistry", "CHEM"),
    ("Statistics", "STATS"),
    ("Conversion", "CONVERT"),
    ("Construction", "BUILD"),
    ("Date & Time", "DATE"),
    ("Everyday", "DAILY"),
    ("Sports", "SPORTS"),
    ("Food", "FOOD"),
    ("Ecology", "ECOLOGY"),
    ("Other", "OTHER"),
];

/// Alt+ keys in mode order: digits first (1..9, 0), then Q W E R T —
/// fifteen modes total. These are the hints printed on the rail, and
/// they are the set the handler below implements — nothing else is
/// printed anywhere.
const MODE_KEYS: &[egui::Key] = &[
    egui::Key::Num1,
    egui::Key::Num2,
    egui::Key::Num3,
    egui::Key::Num4,
    egui::Key::Num5,
    egui::Key::Num6,
    egui::Key::Num7,
    egui::Key::Num8,
    egui::Key::Num9,
    egui::Key::Num0,
    egui::Key::Q,
    egui::Key::W,
    egui::Key::E,
    egui::Key::R,
    egui::Key::T,
];

fn mode_hint(key: egui::Key) -> String {
    let name = match key {
        egui::Key::Num0 => "0",
        egui::Key::Num1 => "1",
        egui::Key::Num2 => "2",
        egui::Key::Num3 => "3",
        egui::Key::Num4 => "4",
        egui::Key::Num5 => "5",
        egui::Key::Num6 => "6",
        egui::Key::Num7 => "7",
        egui::Key::Num8 => "8",
        egui::Key::Num9 => "9",
        egui::Key::Q => "Q",
        egui::Key::W => "W",
        egui::Key::E => "E",
        egui::Key::R => "R",
        egui::Key::T => "T",
        _ => "?",
    };
    format!("Alt+{name}")
}

fn build_modes(present: &BTreeSet<String>) -> Vec<Mode> {
    let mut modes = Vec::new();
    let mut keys = MODE_KEYS.iter();
    for (name, short) in MODE_ORDER {
        if *name != "All" && !present.contains(*name) {
            continue;
        }
        let Some(key) = keys.next() else { break };
        modes.push(Mode {
            name: name.to_string(),
            short,
            hint: mode_hint(*key),
            key: *key,
        });
    }
    modes
}

struct App {
    calcs: Vec<Calc>,
    modes: Vec<Mode>,
    category: String, // "All" or a category name
    search: String,
    selected: Option<usize>, // index into calcs
    inputs: Vec<String>,
    /// One text per date group of the selected calculator (MM/DD/YYYY).
    date_texts: Vec<String>,
    list_input: String,
    output: String,
    /// History rail visible (wide windows) / log view shown (narrow).
    show_log: bool,
    log_entries: Vec<LogEntry>,
    /// v0.3.0: persistent settings (favourites, recent, remembered
    /// inputs, sig-figs) — items #1/#46/#48.
    settings: Settings,
    /// Ctrl+K (or `/`) asked for the search box to take focus.
    focus_search: bool,
    /// Batch panel (item #31) replaces the centre when true.
    show_batch: bool,
    batch_in: String,
    batch_out: String,
    batch_summary: String,
    /// Vary-one-input state (item #16).
    vary_field: usize,
    vary_start: String,
    vary_end: String,
    vary_step: String,
    vary_rows: Vec<(f64, Result<String, String>)>,
    vary_error: String,
    /// v0.4.0: built-in HELP panel (standing rule) replaces the
    /// centre when true.
    show_help: bool,
    /// Update-check verdict (button or startup check), shown in
    /// the HELP panel. The check itself runs on a thread.
    help_msg: String,
    update_rx: Option<std::sync::mpsc::Receiver<String>>,
    /// Expression calculator angle mode (#5): true = degrees.
    expr_degrees: bool,
    /// Per-result display override (#2): re-renders the CURRENT
    /// result without changing the global setting.
    override_format: Option<DisplayFormat>,
    override_n: usize,
    /// Spec check state (#10).
    spec_min: String,
    spec_max: String,
    spec_lines: Vec<String>,
    /// Batch overwrite consent (standing rule: undo/protection).
    batch_force: bool,
    /// A cleared-log backup exists (RESTORE CLEARED LOG shows).
    backup_exists: bool,
    /// "Copied as ..." confirmation line under the display.
    copy_msg: String,
    /// v0.5.0 students batch (#12/#13/#15/#17/#18): args of the
    /// last successful run — deeper steps, check and export all
    /// work on the run that is on screen.
    last_args: Vec<f64>,
    check_text: String,
    check_msg: String,
    check_ok: bool,
    export_path: String,
    export_msg: String,
    show_practice: bool,
    practice_seed: String,
    practice_rng: Rng,
    practice_current: Option<PracticeProblem>,
    practice_answer: String,
    practice_msg: String,
    practice_reveal: String,
    practice_attempted: u32,
    practice_correct: u32,
    plot_from: String,
    plot_to: String,
    /// v0.6.0 teachers batch: the projection style currently
    /// applied to the context (None until the first frame applies
    /// the persisted setting).
    applied_projection: Option<bool>,
    /// Demonstration mode (#25), window-session only: results are
    /// masked until REVEAL is pressed for the current result.
    demo: bool,
    demo_revealed: bool,
    /// The SETS view (#22/#26/#27/#28/#29) replaces the centre.
    show_sets: bool,
    set_name: String,
    set_seed_note: Option<u64>,
    set_entries: Vec<SetEntry>,
    set_path: String,
    set_out_path: String,
    set_msg: String,
    gen_seed: String,
    gen_count: String,
    gen_ids: String,
    mark_answers: String,
    mark_out: String,
    mark_msg: String,
    sheet_category: String,
    sheet_path: String,
    /// The saved class examples library (#28), loaded from the
    /// data folder's examples.txt at startup.
    examples: Vec<SavedExample>,
    ex_title: String,
    ex_note: String,
    ex_msg: String,
    /// Title whose DELETE is armed (the confirm step).
    ex_delete_armed: Option<String>,
    /// v0.7.0 scientists batch (#38): the CONSTANTS view
    /// replaces the centre; the define-your-own fields and the
    /// delete arming live here.
    show_constants: bool,
    const_name: String,
    const_value: String,
    const_msg: String,
    const_delete_armed: Option<String>,
    /// Lab-notebook editor state (#39) for the log views.
    log_edit: LogEdit,
    /// v0.8.0 engineers batch (#44): the MATERIALS view
    /// replaces the centre.
    show_materials: bool,
    /// Log polish view state (#49) for the log views.
    log_view: LogView,
    /// v0.9.0: last-seen window geometry (outer position + inner
    /// size), captured every frame and written into the settings
    /// on exit so the next launch reopens where this one closed.
    window_rect: Option<(f32, f32, f32, f32)>,
}

impl App {
    fn new() -> App {
        let calcs = all_calcs();
        let mut set: BTreeSet<String> = BTreeSet::new();
        for c in &calcs {
            set.insert(c.category.to_string());
        }
        let ids: BTreeSet<String> = calcs.iter().map(|c| c.id.to_string()).collect();
        // Corrupt/missing settings -> defaults inside load_settings;
        // startup can never be broken by the settings file.
        let settings = load_settings(&ids);
        // v0.7.0 #38: install the settings file's user constants
        // into the expression parser's process table (the CLI
        // installs the same table before running).
        set_custom_constants(settings.constants.clone());
        // The saved class examples library (#28) — a missing or
        // corrupt file simply means an empty library.
        let examples = load_examples(&ids);
        // Offer-only update check at startup — ONLY when the user
        // has switched it on (default off; the program otherwise
        // makes no network connection at all). Runs on a thread;
        // the verdict appears in the HELP panel.
        let update_rx = if settings.update_check_startup {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(check_for_updates());
            });
            Some(rx)
        } else {
            None
        };
        App {
            settings,
            update_rx,
            modes: build_modes(&set),
            calcs,
            category: "All".to_string(),
            search: String::new(),
            selected: None,
            inputs: Vec::new(),
            date_texts: Vec::new(),
            list_input: String::new(),
            output: String::new(),
            show_log: true,
            log_entries: load_history(),
            focus_search: false,
            show_batch: false,
            batch_in: String::new(),
            batch_out: String::new(),
            batch_summary: String::new(),
            vary_field: 0,
            vary_start: String::new(),
            vary_end: String::new(),
            vary_step: String::new(),
            vary_rows: Vec::new(),
            vary_error: String::new(),
            show_help: false,
            help_msg: String::new(),
            expr_degrees: true,
            override_format: None,
            override_n: 3,
            spec_min: String::new(),
            spec_max: String::new(),
            spec_lines: Vec::new(),
            batch_force: false,
            backup_exists: history_backup_exists(),
            copy_msg: String::new(),
            last_args: Vec::new(),
            check_text: String::new(),
            check_msg: String::new(),
            check_ok: false,
            export_path: String::new(),
            export_msg: String::new(),
            show_practice: false,
            practice_seed: "7".to_string(),
            practice_rng: Rng::new(7),
            practice_current: None,
            practice_answer: String::new(),
            practice_msg: String::new(),
            practice_reveal: String::new(),
            practice_attempted: 0,
            practice_correct: 0,
            plot_from: "-10".to_string(),
            plot_to: "10".to_string(),
            applied_projection: None,
            demo: false,
            demo_revealed: false,
            show_sets: false,
            set_name: "Untitled set".to_string(),
            set_seed_note: None,
            set_entries: Vec::new(),
            set_path: String::new(),
            set_out_path: String::new(),
            set_msg: String::new(),
            gen_seed: "7".to_string(),
            gen_count: "10".to_string(),
            gen_ids: String::new(),
            mark_answers: String::new(),
            mark_out: String::new(),
            mark_msg: String::new(),
            sheet_category: "Math".to_string(),
            sheet_path: String::new(),
            examples,
            ex_title: String::new(),
            ex_note: String::new(),
            ex_msg: String::new(),
            ex_delete_armed: None,
            show_constants: false,
            show_materials: false,
            const_name: String::new(),
            const_value: String::new(),
            const_msg: String::new(),
            const_delete_armed: None,
            log_edit: LogEdit::default(),
            log_view: LogView {
                export_path: default_log_export_path(),
                ..Default::default()
            },
            window_rect: None,
        }
    }

    fn persist_settings(&self) {
        let _ = save_settings_file(&self.settings);
    }

    fn select(&mut self, idx: usize) {
        self.selected = Some(idx);
        // A new calculator starts with its answer hidden again
        // in demonstration mode (#25: masking is per result).
        self.demo_revealed = false;
        self.inputs = vec![String::new(); self.calcs[idx].inputs.len()];
        self.list_input.clear();
        self.output.clear();
        self.vary_rows.clear();
        self.vary_error.clear();
        self.vary_field = 0;
        self.vary_start.clear();
        self.vary_end.clear();
        self.vary_step.clear();
        // One merged text field per date group.
        self.date_texts = vec![String::new(); self.calcs[idx].date_groups.len()];
        self.spec_lines.clear();
        self.copy_msg.clear();
        self.last_args.clear();
        self.check_msg.clear();
        self.export_msg.clear();
        self.export_path = default_export_path(&self.calcs[idx]);
        // Item #48: reopening a calculator restores the values last
        // used in it. Item #50: otherwise the fields open on the
        // smart defaults — visible and editable, never hidden.
        let toks: Vec<String> = match self.settings.remembered.get(self.calcs[idx].id) {
            Some(t) => t.clone(),
            None => display_tokens(&self.calcs[idx], defaults_for(&self.calcs[idx])),
        };
        self.fill_from_tokens(&toks);
        // Text calculator (#5): the field holds the remembered
        // expression, or the built-in example to start from.
        if self.calcs[idx].text_run.is_some() {
            self.list_input = match self.settings.remembered.get(self.calcs[idx].id) {
                Some(t) => t.first().cloned().unwrap_or_default(),
                None => self.calcs[idx].example_text.to_string(),
            };
        }
        // Sensible defaults: the Age calculator's End Date and the
        // Countdown's From Date start as today's date.
        let today = today_str();
        for (gi, g) in self.calcs[idx].date_groups.iter().enumerate() {
            let prefill = (self.calcs[idx].id == "age" && g.label == "End Date")
                || (self.calcs[idx].id == "countdown-days" && g.label == "From Date");
            if prefill {
                self.date_texts[gi] = today.clone();
            }
        }
    }

    /// Fill the form from display tokens (one per user-visible
    /// field; dates as MM/DD/YYYY). Shared by select (#48/#50),
    /// log restore and TRY AN EXAMPLE (#11).
    fn fill_from_tokens(&mut self, toks: &[String]) {
        let Some(idx) = self.selected else { return };
        if self.calcs[idx].text_run.is_some() {
            // The restore token IS the expression text.
            self.list_input = toks.first().cloned().unwrap_or_default();
            return;
        }
        if self.calcs[idx].list {
            self.list_input = toks.join(", ");
            return;
        }
        let calc = &self.calcs[idx];
        let mut ti = 0usize;
        let mut gi = 0usize;
        for i in 0..calc.inputs.len() {
            if date_group_at(calc, i).is_some() {
                if let Some(t) = toks.get(ti) {
                    if gi < self.date_texts.len() {
                        self.date_texts[gi] = t.clone();
                    }
                }
                gi += 1;
                ti += 1;
            } else if calc
                .date_groups
                .iter()
                .any(|g| i > g.start && i < g.start + 3)
            {
                // inside a date triple: nothing to fill
            } else {
                if let Some(t) = toks.get(ti) {
                    self.inputs[i] = t.clone();
                }
                ti += 1;
            }
        }
    }

    /// TRY AN EXAMPLE (item #11): fill the built-in teaching example
    /// and run it immediately — a complete worked example in 1 click.
    fn run_example(&mut self) {
        let Some(idx) = self.selected else { return };
        if self.calcs[idx].text_run.is_some() {
            self.list_input = self.calcs[idx].example_text.to_string();
            self.calculate();
            return;
        }
        let toks = display_tokens(&self.calcs[idx], self.calcs[idx].example);
        self.fill_from_tokens(&toks);
        self.calculate();
    }

    /// Reset the form to the smart defaults (items #48/#50).
    fn reset_defaults(&mut self) {
        let Some(idx) = self.selected else { return };
        if self.calcs[idx].text_run.is_some() {
            self.list_input = self.calcs[idx].example_text.to_string();
            self.output.clear();
            return;
        }
        let toks = display_tokens(&self.calcs[idx], defaults_for(&self.calcs[idx]));
        self.fill_from_tokens(&toks);
        self.output.clear();
        // Re-apply the today prefills, as on select.
        let today = today_str();
        for (gi, g) in self.calcs[idx].date_groups.iter().enumerate() {
            let prefill = (self.calcs[idx].id == "age" && g.label == "End Date")
                || (self.calcs[idx].id == "countdown-days" && g.label == "From Date");
            if prefill {
                self.date_texts[gi] = today.clone();
            }
        }
    }

    /// Re-open a calculator from a log entry, with its inputs restored
    /// (date fields refilled in their merged MM/DD/YYYY form).
    fn restore(&mut self, entry: &LogEntry) {
        let Some(idx) = self.calcs.iter().position(|c| c.id == entry.id) else {
            return;
        };
        self.select(idx);
        let toks: Vec<String> = if entry.restore.is_empty() {
            Vec::new()
        } else {
            entry.restore.split(";;").map(|t| t.to_string()).collect()
        };
        self.fill_from_tokens(&toks);
    }

    fn visible(&self, c: &Calc) -> bool {
        if self.category == "Favourites" {
            if !self.settings.favourites.contains(c.id) {
                return false;
            }
        } else if let Some(pack) = self.category.strip_prefix("Pack: ") {
            // Course pack mode (#24): membership comes from the
            // meta.rs pack tables, never a hardcoded list here.
            let member = COURSE_PACKS
                .iter()
                .find(|(name, _)| *name == pack)
                .map(|(_, ids)| ids.contains(&c.id))
                .unwrap_or(false);
            if !member {
                return false;
            }
        } else if self.category != "All" && c.category != self.category {
            return false;
        }
        if self.search.trim().is_empty() {
            return true;
        }
        let q = self.search.to_lowercase();
        c.name.to_lowercase().contains(&q)
            || c.id.contains(&q)
            || c.desc.to_lowercase().contains(&q)
    }

    /// Parse the form into the registry's numeric vector. Error
    /// strings are exactly the ones the GUI has always shown.
    fn parse_inputs(&mut self) -> Result<Vec<f64>, String> {
        let Some(idx) = self.selected else {
            return Err("No calculator selected.".into());
        };
        let calc = &self.calcs[idx];
        let mut nums: Vec<f64> = Vec::new();
        if calc.list {
            // v0.7.0 #32: the shared value-list parser — the
            // exact one the command line uses, so a pasted column
            // (one value per line) parses identically here.
            nums = parse_value_list(&self.list_input)?;
        } else {
            let mut gi = 0usize;
            let mut i = 0usize;
            while i < calc.inputs.len() {
                if let Some(g) = date_group_at(calc, i) {
                    let raw = self.date_texts.get(gi).cloned().unwrap_or_default();
                    match calcdex::parse_date_str(&raw) {
                        Ok((y, m, d)) => {
                            nums.push(y as f64);
                            nums.push(m as f64);
                            nums.push(d as f64);
                        }
                        Err(e) => {
                            return Err(format!("Error in {}: {e}", g.label));
                        }
                    }
                    gi += 1;
                    i += 3;
                } else {
                    // v0.4.0 #6/#7: a field with a known unit family
                    // also accepts a typed quantity ("5 ft 3 in",
                    // "1.2 kg"), converted into the field's unit;
                    // a mismatched unit warns instead of running.
                    let raw = self.inputs[i].clone();
                    let siblings = self.inputs.clone();
                    match calcdex::rigour::parse_field_value(calc, i, &raw, &siblings) {
                        Ok(v) => nums.push(v),
                        Err(e) => {
                            if e.contains("is not a number") || e.contains("not a quantity") {
                                return Err(format!(
                                    "'{}' needs a number.",
                                    calc.inputs.get(i).copied().unwrap_or("Input")
                                ));
                            }
                            return Err(e);
                        }
                    }
                    i += 1;
                }
            }
        }
        Ok(nums)
    }

    fn calculate(&mut self) {
        let Some(idx) = self.selected else {
            return;
        };
        // A fresh run hides its answer again in demo mode (#25).
        self.demo_revealed = false;
        // Text calculator (#5): the field holds an expression; its
        // runs join the same log — the log is its paper tape.
        if self.calcs[idx].text_run.is_some() {
            let text = self.list_input.trim().to_string();
            let calc = &self.calcs[idx];
            match run_text_calc(calc.id, &text, self.expr_degrees) {
                Ok(out) => {
                    let entry = LogEntry {
                        timestamp: now_stamp(),
                        id: calc.id.to_string(),
                        name: calc.name.to_string(),
                        inputs: text.clone(),
                        result: out.clone(),
                        restore: text.clone(),
                        tag: String::new(),
                        note: String::new(),
                        pinned: false,
                    };
                    if let Ok(entries) = append_history(entry.clone()) {
                        self.log_entries = entries;
                    } else {
                        self.log_entries.push(entry);
                    }
                    self.settings.note_run(calc.id, vec![text]);
                    self.persist_settings();
                    self.spec_lines.clear();
                    self.last_args.clear();
                    self.check_msg.clear();
                    self.output = out;
                }
                Err(e) => {
                    self.output = format!("Error: {e}");
                }
            }
            return;
        }
        let nums = match self.parse_inputs() {
            Ok(v) => v,
            Err(e) => {
                self.output = e;
                return;
            }
        };
        let calc = &self.calcs[idx];
        match run_calc(calc.id, &nums) {
            Ok(out) => {
                // Auto-save every successful calculation to the log; the
                // history rail reads this same in-memory list, so the new
                // entry appears there immediately.
                let entry = make_entry(calc, &nums, &out);
                if let Ok(entries) = append_history(entry) {
                    self.log_entries = entries;
                } else {
                    self.log_entries.push(make_entry(calc, &nums, &out));
                }
                // Items #46/#48: remember this calculator's inputs and
                // its place in the Recent list.
                let toks = display_tokens(calc, &nums);
                self.settings.note_run(calc.id, toks);
                self.persist_settings();
                self.spec_lines.clear();
                self.last_args = nums.clone();
                self.check_msg.clear();
                self.output = out;
            }
            Err(e) => {
                self.output = format!("Error: {e}");
            }
        }
    }

    /// VARY (item #16): sweep one input across start/end/step using
    /// the values currently in the form for the other inputs.
    fn do_vary(&mut self) {
        self.vary_rows.clear();
        self.vary_error.clear();
        let Some(idx) = self.selected else { return };
        let (Ok(start), Ok(end), Ok(step)) = (
            self.vary_start.trim().parse::<f64>(),
            self.vary_end.trim().parse::<f64>(),
            self.vary_step.trim().parse::<f64>(),
        ) else {
            self.vary_error =
                "VARY needs start, end and step as numbers — e.g. start 0, end 10, step 2."
                    .to_string();
            return;
        };
        let base = match self.parse_inputs() {
            Ok(v) => v,
            Err(e) => {
                self.vary_error = e;
                return;
            }
        };
        let calc = &self.calcs[idx];
        match vary_run(calc, &base, self.vary_field, start, end, step) {
            Ok(rows) => {
                self.vary_rows = rows.into_iter().map(|r| (r.value, r.result)).collect();
            }
            Err(e) => {
                self.vary_error = e;
            }
        }
    }

    /// BATCH (item #31): run the selected calculator over the CSV
    /// file at batch_in and write the results file at batch_out.
    fn do_batch(&mut self) {
        let Some(idx) = self.selected else {
            self.batch_summary = "Pick a calculator first (mode rail + chip strip).".to_string();
            return;
        };
        let calc = &self.calcs[idx];
        if calc.text_run.is_some() {
            self.batch_summary =
                "The Expression Calculator takes text, not CSV rows — pick a numeric calculator for batch."
                    .to_string();
            return;
        }
        // Overwrite protection: an existing output file is only
        // replaced when the OVERWRITE box is ticked, and then the
        // old file is first backed up to <name>.bak.
        let out_path = self.batch_out.trim().to_string();
        if std::path::Path::new(&out_path).exists() {
            if !self.batch_force {
                self.batch_summary = format!(
                    "{out_path} already exists — refusing to overwrite it. Tick OVERWRITE EXISTING OUTPUT to replace it (the old file is backed up to {out_path}.bak first)."
                );
                return;
            }
            let bak = format!("{out_path}.bak");
            if let Err(e) = std::fs::copy(&out_path, &bak) {
                self.batch_summary = format!("Could not back up {out_path} to {bak}: {e}");
                return;
            }
        }
        let text = match std::fs::read_to_string(self.batch_in.trim()) {
            Ok(t) => t,
            Err(e) => {
                self.batch_summary = format!("Could not read {}: {e}", self.batch_in.trim());
                return;
            }
        };
        match batch_run(calc, &text) {
            Ok(outcome) => match std::fs::write(self.batch_out.trim(), &outcome.output_csv) {
                Ok(()) => {
                    self.batch_summary = format!(
                        "{} rows: {} ok, {} errors — see output file",
                        outcome.ok + outcome.errors,
                        outcome.ok,
                        outcome.errors
                    );
                }
                Err(e) => {
                    self.batch_summary = format!("Could not write {}: {e}", self.batch_out.trim());
                }
            },
            Err(e) => {
                self.batch_summary = format!("Error: {e}");
            }
        }
    }
}

const GREEN: egui::Color32 = egui::Color32::from_rgb(0x4a, 0xde, 0x80);
const MUTED: egui::Color32 = egui::Color32::from_rgb(0x7a, 0x7a, 0x7a);
const TEXT: egui::Color32 = egui::Color32::from_rgb(0xd0, 0xd0, 0xd0);
const RED: egui::Color32 = egui::Color32::from_rgb(0xf2, 0x6d, 0x6d);
/// Selection background for rows/chips (Purgeon green-sel #163322).
const SEL_BG: egui::Color32 = egui::Color32::from_rgb(0x16, 0x33, 0x22);
/// Hover background for rows — clearly visible on #121212.
const HOVER_BG: egui::Color32 = egui::Color32::from_rgb(0x26, 0x26, 0x26);
/// Shortcut-hint grey on the mode rail: #9a9a9a measures 6.66:1 on the
/// panel (WCAG AA asks 4.5:1) — brighter than the theme muted grey,
/// which is only for large/secondary print. See docs/AUDIT.md.
const HINT: egui::Color32 = egui::Color32::from_rgb(0x9a, 0x9a, 0x9a);
/// Projection-mode brightened hint colour (v0.6.0 #21): the muted /
/// hint greys are lifted so small text stays readable from the
/// back of a classroom.
const HINT_PROJ: egui::Color32 = egui::Color32::from_rgb(0xd8, 0xd8, 0xd8);
/// Projection-mode brightened muted colour (weak text, plot axes).
const MUTED_PROJ: egui::Color32 = egui::Color32::from_rgb(0xb8, 0xb8, 0xb8);
/// Display background (theme bg #0c0c0c, inset against the panel).
const DISPLAY_BG: egui::Color32 = egui::Color32::from_rgb(0x0c, 0x0c, 0x0c);
/// Raised fill for chips and raised controls (#181818).
const RAISED: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x18);
// The history rail's width threshold lives in the library
// (`calcdex::HISTORY_RAIL_MIN_WIDTH`, with the pure visibility rule
// `calcdex::history_rail_visible`): it is judged on the physical
// window width, so projection zoom never hides the rail.

/// One mode-rail row: short label on the left, shortcut hint on the
/// right, full category name in the hover tooltip. Selected styling
/// matches the v0.1.2 list rows (selection background + green text +
/// 3 pt marker bar), so selection is never colour-alone subtle.
fn mode_row(
    ui: &mut egui::Ui,
    mode: &Mode,
    count: usize,
    selected: bool,
    hint: egui::Color32,
) -> bool {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 0.0, SEL_BG);
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
            0.0,
            GREEN,
        );
    } else if resp.hovered() {
        painter.rect_filled(rect, 0.0, HOVER_BG);
    }
    let cy = rect.center().y;
    painter.text(
        egui::pos2(rect.min.x + 10.0, cy),
        egui::Align2::LEFT_CENTER,
        mode.short,
        egui::FontId::monospace(13.0),
        if selected { GREEN } else { TEXT },
    );
    painter.text(
        egui::pos2(rect.max.x - 8.0, cy),
        egui::Align2::RIGHT_CENTER,
        &mode.hint,
        egui::FontId::monospace(10.5),
        hint,
    );
    let clicked = resp.clicked();
    resp.on_hover_text(format!("{} ({count} calculators)", mode.name));
    clicked
}

/// Write a teacher export file (set, key, worksheet, marked CSV,
/// formula sheet): an existing file is first copied to
/// <name>.bak, so an export can never silently destroy the
/// previous copy (the destructive-action rule). Returns a note
/// for the status line.
fn write_with_backup(path: &std::path::Path, text: &str) -> Result<String, String> {
    let mut note = String::new();
    if path.exists() {
        let bak = format!("{}.bak", path.display());
        match std::fs::copy(path, &bak) {
            Ok(_) => note = format!("The previous copy was backed up to {bak}."),
            Err(e) => return Err(format!("Could not back up {}: {e}", path.display())),
        }
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Err(format!("Could not create {}: {e}", parent.display()));
            }
        }
    }
    match std::fs::write(path, text) {
        Ok(()) => Ok(note),
        Err(e) => Err(format!("Could not write {}: {e}", path.display())),
    }
}

/// What one frame of the history rail reports back to the App:
/// the deferred actions the rail cannot do itself.
#[derive(Default)]
struct HistoryOutcome {
    /// Entry to re-open in its calculator.
    restore: Option<LogEntry>,
    clear: bool,
    restore_cleared: bool,
    /// Annotation save request (stored index, tag, note) (#39).
    save_edit: Option<(usize, String, String)>,
    /// Pin toggle request (stored index, new pinned state) (#49).
    pin: Option<(usize, bool)>,
    /// Export request: the active filter + the target path (#49).
    export: Option<(LogFilter, String)>,
}

/// Lab-notebook editor state (v0.7.0 #39): which log entry is
/// being annotated (its index in the stored, oldest-first list)
/// plus the tag/note drafts in the editor row.
#[derive(Default)]
struct LogEdit {
    entry: Option<usize>,
    tag: String,
    note: String,
}

/// Log polish view state (v0.8.0 #49): the search text, the
/// calc/tag/date filters, the pinned-only toggle and the export
/// path/message for the log views (rail + narrow centre).
#[derive(Default)]
struct LogView {
    search: String,
    calc: String,
    tag: String,
    date: String,
    pinned_only: bool,
    export_path: String,
    export_msg: String,
}

impl LogView {
    fn filter(&self) -> LogFilter {
        let clean = |s: &str| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        };
        LogFilter {
            text: clean(&self.search),
            calc_id: clean(&self.calc),
            tag: clean(&self.tag),
            date: clean(&self.date),
            pinned_only: self.pinned_only,
        }
    }

    fn clear_filters(&mut self) {
        self.search.clear();
        self.calc.clear();
        self.tag.clear();
        self.date.clear();
        self.pinned_only = false;
    }
}

/// The history (calculation log) content, shared by the docked right
/// rail and the narrow-window centre log view. Returns
/// (entry to re-open, clear-log clicked, restore-cleared clicked,
/// annotation save request (stored index, tag, note)).
fn history_ui(
    ui: &mut egui::Ui,
    entries: &[LogEntry],
    backup_exists: bool,
    edit: &mut LogEdit,
    view: &mut LogView,
) -> HistoryOutcome {
    let mut to_restore: Option<LogEntry> = None;
    let mut save_edit: Option<(usize, String, String)> = None;
    let mut pin_req: Option<(usize, bool)> = None;
    let mut export_req: Option<(LogFilter, String)> = None;
    ui.label(
        egui::RichText::new(format!("HISTORY ({})", entries.len()))
            .color(GREEN)
            .strong()
            .size(14.0),
    );
    ui.label(
        egui::RichText::new(
            "Saved calculations, newest first. Every successful calculation \
             (window or command line) is saved automatically. RE-OPEN loads \
             an entry's calculator with its inputs restored. TAG / NOTE \
             annotates an entry — the log doubles as a lab notebook, and \
             CalcDex --lab-export writes it out as a lab record. Search and \
             the filters below narrow the list (the # numbers stay the \
             entry's own); PIN keeps an entry when the 500-entry trim runs; \
             EXPORT FILTERED writes the shown entries to a text file.",
        )
        .color(TEXT),
    );
    ui.add_space(8.0);
    let mut do_clear = false;
    let mut do_restore_cleared = false;
    ui.horizontal_wrapped(|ui| {
        if tip_btn(
            ui,
            egui::RichText::new("CLEAR LOG").color(RED),
            "Empty the calculation log — a backup copy of the cleared entries is saved first, so RESTORE CLEARED LOG can undo it",
        )
        .clicked()
        {
            do_clear = true;
        }
        if backup_exists
            && tip_btn(
                ui,
                egui::RichText::new("RESTORE CLEARED LOG"),
                "Undo the last CLEAR LOG — put the backed-up entries back",
            )
            .clicked()
        {
            do_restore_cleared = true;
        }
    });
    // Log polish (#49): search + filter controls and the
    // filtered export. The filter is computed from the view state
    // every frame; the entry numbers shown stay global.
    let filter = view.filter();
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Search:").color(TEXT).size(12.5));
        ui.add(egui::TextEdit::singleline(&mut view.search).desired_width(170.0))
            .on_hover_text(
                "Show only entries whose name, inputs, result, tag or note contain this text",
            );
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Calc:").color(TEXT).size(12.5));
        ui.add(egui::TextEdit::singleline(&mut view.calc).desired_width(90.0))
            .on_hover_text(
                "Show only one calculator's entries — its id, e.g. circle (CalcDex --list shows the ids)",
            );
        ui.label(egui::RichText::new("Tag:").color(TEXT).size(12.5));
        ui.add(egui::TextEdit::singleline(&mut view.tag).desired_width(90.0))
            .on_hover_text("Show only entries carrying this lab-notebook tag");
        ui.label(egui::RichText::new("Date:").color(TEXT).size(12.5));
        ui.add(egui::TextEdit::singleline(&mut view.date).desired_width(90.0))
            .on_hover_text(
                "Show only entries from one day, typed as YYYY-MM-DD (a shorter YYYY-MM matches the whole month)",
            );
    });
    ui.horizontal_wrapped(|ui| {
        if tip_sel(
            ui,
            view.pinned_only,
            egui::RichText::new("PINNED ONLY").size(12.0),
            "Show only pinned entries — the ones the 500-entry trim never drops",
        )
        .clicked()
        {
            view.pinned_only = !view.pinned_only;
        }
        if tip_btn(
            ui,
            egui::RichText::new("CLEAR FILTERS"),
            "Clear the search text and every filter, so the whole log shows again",
        )
        .clicked()
        {
            view.clear_filters();
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut view.export_path).desired_width(190.0))
            .on_hover_text(
                "The file the filtered log export is written to — if it already exists, its current contents are first copied to a .bak file beside it",
            );
        if tip_btn(
            ui,
            egui::RichText::new("EXPORT FILTERED"),
            "Write the entries the current search and filters show (or the whole log when nothing is filtered) to the file named on the left, as plain text",
        )
        .clicked()
        {
            export_req = Some((filter.clone(), view.export_path.trim().to_string()));
        }
    });
    if !view.export_msg.is_empty() {
        ui.label(egui::RichText::new(&view.export_msg).color(TEXT).size(12.0));
    }
    // The lab-notebook editor row (#39), shown while an entry is
    // being annotated. Saving backs the log up first (the caller
    // goes through the library's annotate path, which does it).
    if let Some(idx) = edit.entry {
        match entries.get(idx) {
            Some(e) => {
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("TAG / NOTE — {} {}", e.timestamp, e.name))
                        .color(GREEN)
                        .strong()
                        .size(12.5),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Tag:").color(TEXT).size(12.5));
                    ui.add(
                        egui::TextEdit::singleline(&mut edit.tag).desired_width(180.0),
                    )
                    .on_hover_text(
                        "A short label for this entry, e.g. titration-3 — an empty tag clears it",
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Note:").color(TEXT).size(12.5));
                    ui.add(egui::TextEdit::singleline(&mut edit.note).desired_width(260.0))
                        .on_hover_text("A free-text note for this entry — an empty note clears it");
                });
                ui.horizontal_wrapped(|ui| {
                    if tip_btn(
                        ui,
                        egui::RichText::new("SAVE TAG / NOTE").color(GREEN),
                        "Save this entry's tag and note — the log is backed up to history-edit-backup.txt before the edit",
                    )
                    .clicked()
                    {
                        save_edit = Some((idx, edit.tag.clone(), edit.note.clone()));
                    }
                    if tip_btn(
                        ui,
                        egui::RichText::new("CANCEL"),
                        "Close the tag/note editor without changing the entry",
                    )
                    .clicked()
                    {
                        edit.entry = None;
                    }
                });
                ui.separator();
            }
            None => {
                edit.entry = None;
            }
        }
    }
    ui.add_space(8.0);
    ui.separator();
    let matched = filter_entries(entries, &filter);
    if !filter.is_empty() {
        ui.label(
            egui::RichText::new(format!(
                "Showing {} of {} entries ({}).",
                matched.len(),
                entries.len(),
                filter.describe()
            ))
            .color(TEXT)
            .size(12.0),
        );
    }
    if entries.is_empty() {
        ui.label(
            egui::RichText::new(
                "The log is empty. Run any calculator and its result appears here.",
            )
            .color(TEXT),
        );
    } else if matched.is_empty() {
        ui.label(
            egui::RichText::new(
                "No entries match the current search and filters — CLEAR FILTERS shows the whole log again.",
            )
            .color(TEXT),
        );
    }
    egui::ScrollArea::vertical()
        .id_salt("history-scroll")
        .show(ui, |ui| {
            for (num, e) in matched.iter().rev() {
                let num = *num;
                let stored_idx = entries.len() - num;
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(format!("#{num}"))
                                .color(TEXT)
                                .size(11.5),
                        );
                        ui.label(egui::RichText::new(&e.timestamp).color(TEXT).size(11.5));
                        ui.label(egui::RichText::new(&e.name).color(GREEN).strong());
                        if e.pinned {
                            ui.label(
                                egui::RichText::new("PINNED")
                                    .color(GREEN)
                                    .strong()
                                    .size(11.5),
                            );
                        }
                        if tip_btn(
                            ui,
                            egui::RichText::new("RE-OPEN"),
                            "Open this entry's calculator with its inputs restored",
                        )
                        .clicked()
                        {
                            to_restore = Some((*e).clone());
                        }
                        if tip_btn(
                            ui,
                            egui::RichText::new("TAG / NOTE"),
                            "Tag or annotate this entry (lab notebook) — the log is backed up before the edit is saved",
                        )
                        .clicked()
                        {
                            edit.entry = Some(stored_idx);
                            edit.tag = e.tag.clone();
                            edit.note = e.note.clone();
                        }
                        if tip_btn(
                            ui,
                            egui::RichText::new(if e.pinned { "UNPIN" } else { "PIN" }),
                            if e.pinned {
                                "Remove this entry's pin — the 500-entry trim may then drop it when newer entries arrive"
                            } else {
                                "Pin this entry — the 500-entry trim never drops a pinned entry (the log is backed up before the change)"
                            },
                        )
                        .clicked()
                        {
                            pin_req = Some((stored_idx, !e.pinned));
                        }
                    });
                    ui.label(egui::RichText::new(format!("Inputs: {}", e.inputs)).color(TEXT));
                    ui.label(egui::RichText::new(format!("Result: {}", e.result)).color(TEXT));
                    if !e.tag.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Tag: {}", e.tag)).color(TEXT),
                        );
                    }
                    if !e.note.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Note: {}", e.note)).color(TEXT),
                        );
                    }
                });
                ui.add_space(4.0);
            }
        });
    HistoryOutcome {
        restore: to_restore,
        clear: do_clear,
        restore_cleared: do_restore_cleared,
        save_edit,
        pin: pin_req,
        export: export_req,
    }
}

// ---------- Tooltip rule (SEAN standing rule, applied v0.3.0) ----------
//
// EVERY interactive control in the window carries a tooltip, no
// exceptions. The enforceable pattern: buttons and selectable
// labels are ONLY built through the helpers below, each of which
// REQUIRES a tooltip argument — a call site cannot forget it.
// Text edits and other widgets get `.on_hover_text(..)` on their
// response at the call site. tests/tooltips.rs scans this file and
// fails if a bare `ui.button(` / `ui.selectable_label(` /
// `egui::Button::new` appears outside these helpers.

/// A plain button with a mandatory tooltip.
fn tip_btn(ui: &mut egui::Ui, text: egui::RichText, tooltip: &str) -> egui::Response {
    ui.add(egui::Button::new(text)).on_hover_text(tooltip)
}

/// A fixed-size button with a mandatory tooltip.
fn tip_btn_sized(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    text: egui::RichText,
    tooltip: &str,
) -> egui::Response {
    ui.add_sized(size, egui::Button::new(text))
        .on_hover_text(tooltip)
}

/// A selectable label (toggle) with a mandatory tooltip.
fn tip_sel(
    ui: &mut egui::Ui,
    selected: bool,
    text: egui::RichText,
    tooltip: &str,
) -> egui::Response {
    ui.selectable_label(selected, text).on_hover_text(tooltip)
}

/// A selectable value (combo-box option) with a mandatory tooltip.
fn tip_sel_value(
    ui: &mut egui::Ui,
    current: &mut usize,
    value: usize,
    label: &str,
    tooltip: &str,
) -> egui::Response {
    ui.selectable_value(current, value, label)
        .on_hover_text(tooltip)
}

/// A calculator chip in the centre strip (custom fill/stroke), with
/// a mandatory tooltip.
fn tip_chip(ui: &mut egui::Ui, name: &str, selected: bool) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(name)
            .color(if selected { GREEN } else { TEXT })
            .size(12.5),
    )
    .fill(if selected { SEL_BG } else { RAISED })
    .stroke(egui::Stroke::new(
        1.0_f32,
        if selected {
            GREEN
        } else {
            egui::Color32::from_rgb(0x26, 0x26, 0x26)
        },
    ));
    ui.add(btn).on_hover_text(format!("Open {name}"))
}

impl eframe::App for App {
    /// v0.9.0: on exit, persist the last-seen window geometry so
    /// the next launch reopens where this one closed.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(rect) = self.window_rect {
            if self.settings.window != Some(rect) {
                self.settings.window = Some(rect);
                self.persist_settings();
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // v0.9.0: track the window geometry (outer position +
        // inner size) for the on_exit save above.
        let rect = ctx.input(|i| {
            let v = i.viewport();
            match (v.outer_rect, v.inner_rect) {
                (Some(outer), Some(inner)) => {
                    Some((outer.min.x, outer.min.y, inner.width(), inner.height()))
                }
                _ => None,
            }
        });
        if rect.is_some() {
            self.window_rect = rect;
        }
        // Classroom projection mode (#21): apply the persisted
        // setting to the context whenever it changes (the first
        // frame counts, so a saved-on setting greets the user).
        if self.applied_projection != Some(self.settings.projection) {
            self.applied_projection = Some(self.settings.projection);
            apply_projection(ctx, self.settings.projection);
        }
        // Alt+ mode shortcuts — the exact set printed on the rail.
        let mut mode_pick: Option<String> = None;
        ctx.input(|i| {
            if i.modifiers.alt {
                for m in &self.modes {
                    if i.key_pressed(m.key) {
                        mode_pick = Some(m.name.clone());
                    }
                }
            }
        });
        if let Some(name) = mode_pick {
            self.category = name;
        }
        // Item #47: Ctrl+K focuses search from anywhere; `/` does
        // too when no field has focus (so it never fights typing).
        // Item #46: Alt+F opens the Favourites mode.
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::K) {
                self.focus_search = true;
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::F) {
                self.category = "Favourites".to_string();
            }
            if i.key_pressed(egui::Key::Slash) && !i.modifiers.ctrl && !i.modifiers.alt {
                self.focus_search = true;
            }
        });
        if self.focus_search {
            // If a text field is actually being edited, `/` belongs
            // to it — egui gives the key to the focused widget first;
            // requesting focus here only moves focus to search, which
            // is Ctrl+K's job. For `/`, only steal when nothing is
            // focused:
            let any_focused = ctx.memory(|m| m.focused().is_some());
            if any_focused && !ctx.input(|i| i.modifiers.ctrl) {
                self.focus_search = false;
            }
        }
        // F1 opens the built-in help (standing rule).
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_help = !self.show_help;
            if self.show_help {
                self.show_practice = false;
                self.show_sets = false;
                self.show_constants = false;
                self.show_materials = false;
            }
        }
        // Update-check verdict arriving from its thread.
        if let Some(rx) = &self.update_rx {
            if let Ok(msg) = rx.try_recv() {
                self.help_msg = msg;
                self.update_rx = None;
            }
        }
        // Drag & drop (standing rule): a dropped .csv opens the
        // BATCH panel with the input path filled.
        let dropped_csv = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .find(|path| {
                    path.extension()
                        .map(|e| e.eq_ignore_ascii_case("csv"))
                        .unwrap_or(false)
                })
        });
        if let Some(path) = dropped_csv {
            self.batch_in = path.display().to_string();
            self.show_batch = true;
            self.show_help = false;
            self.show_practice = false;
            self.show_sets = false;
            self.show_constants = false;
            self.show_materials = false;
            self.batch_summary =
                "Input CSV filled from the dropped file — pick the calculator, check the output path, then RUN BATCH."
                    .to_string();
        }
        // Enter = Calculate (single-line fields; the list box keeps
        // Enter for separating values, and the expression field
        // keeps it for typing).
        let enter_calc = ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && self
                .selected
                .map(|idx| !self.calcs[idx].list && self.calcs[idx].text_run.is_none())
                .unwrap_or(false);

        // Judged on the physical width (layout points x zoom), so the
        // history rail that fits at normal size still fits — and is
        // still shown — in projection mode (field report, 2026-11-02).
        let wide = calcdex::history_rail_visible(ctx.content_rect().width(), ctx.zoom_factor());
        let mut hist_restore: Option<LogEntry> = None;
        let mut hist_clear = false;
        let mut hist_restore_cleared = false;
        let mut hist_save_edit: Option<(usize, String, String)> = None;
        let mut hist_pin: Option<(usize, bool)> = None;
        let mut hist_export: Option<(LogFilter, String)> = None;

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            // Wrapped, not plain horizontal: at projection zoom the
            // logical window is narrower and this long row (title,
            // BATCH/HELP, the FORMAT group, LOG) used to run off the
            // right edge and clip (field report, 2026-11-02).
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("CALCDEX")
                        .color(GREEN)
                        .strong()
                        .size(18.0),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} CALCULATORS - OFFLINE - NO ADS",
                        self.calcs.len()
                    ))
                    .color(TEXT)
                    .size(11.5),
                );
                ui.separator();
                if tip_sel(
                    ui,
                    self.show_batch,
                    egui::RichText::new("BATCH").color(GREEN).size(12.0),
                    "Run one calculator over a CSV file — many rows in one go",
                )
                .clicked()
                {
                    self.show_batch = !self.show_batch;
                    if self.show_batch {
                        self.show_help = false;
                        self.show_practice = false;
                        self.show_sets = false;
                        self.show_constants = false;
                        self.show_materials = false;
                    }
                }
                if tip_btn(
                    ui,
                    egui::RichText::new("HELP").color(GREEN).size(12.0),
                    "Open the built-in help guide (F1) — what CalcDex does, shortcuts, and where your files live",
                )
                .clicked()
                {
                    self.show_help = !self.show_help;
                    if self.show_help {
                        self.show_batch = false;
                        self.show_practice = false;
                        self.show_sets = false;
                        self.show_constants = false;
                        self.show_materials = false;
                    }
                }
                ui.separator();
                // Unified display format (v0.4.0 #1/#2/#3/#37):
                // one setting, five modes; the figure count appears
                // for the two counted modes. Display only — the
                // stored result always keeps full precision.
                ui.label(egui::RichText::new("FORMAT:").color(self.hint_col()).size(11.5));
                let mut mode_pick: Option<DisplayMode> = None;
                for (mode, label, tip) in [
                    (DisplayMode::Off, "Off", "Show full precision (the stored result is never rounded)"),
                    (DisplayMode::SigFigs, "Sig figs", "Also show the headline rounded to a number of significant figures"),
                    (DisplayMode::Decimals, "Decimals", "Also show the headline rounded to a number of decimal places"),
                    (DisplayMode::Engineering, "Engineer", "Also show the headline in engineering notation with SI prefixes (1.63 k, 4.2 M, 350 m)"),
                    (DisplayMode::Auto, "Auto", "Infer the significant figures from the numbers you typed and show the headline to that precision"),
                    (DisplayMode::Exact, "Exact", "Fraction results are labelled exact; pi geometry also shows its exact form (like 25*pi); rational expressions are worked out exactly"),
                ] {
                    if tip_sel(
                        ui,
                        self.settings.display.mode == mode,
                        egui::RichText::new(label).size(12.0),
                        tip,
                    )
                    .clicked()
                    {
                        mode_pick = Some(mode);
                    }
                }
                if let Some(mode) = mode_pick {
                    self.settings.display.mode = mode;
                    if self.settings.display.n.is_none() {
                        self.settings.display.n = Some(3);
                    }
                    self.persist_settings();
                }
                if matches!(
                    self.settings.display.mode,
                    DisplayMode::SigFigs | DisplayMode::Decimals
                ) {
                    let mut n_usize = self.settings.display.n.unwrap_or(3) as usize;
                    let combo = egui::ComboBox::from_id_salt("format-n")
                        .selected_text(format!("{n_usize}"))
                        .width(52.0)
                        .show_ui(ui, |ui| {
                            let range: &[usize] = if self.settings.display.mode
                                == DisplayMode::SigFigs
                            {
                                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                            } else {
                                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                            };
                            for v in range {
                                tip_sel_value(
                                    ui,
                                    &mut n_usize,
                                    *v,
                                    &format!("{v}"),
                                    "How many figures the rounded headline shows",
                                );
                            }
                        });
                    combo
                        .response
                        .on_hover_text("Choose the figure count for the rounded headline");
                    if n_usize != self.settings.display.n.unwrap_or(3) as usize {
                        self.settings.display.n = Some(n_usize as u8);
                        self.persist_settings();
                    }
                }
                ui.separator();
                let log_label = format!("LOG ({})", self.log_entries.len());
                if tip_sel(
                    ui,
                    self.show_log,
                    egui::RichText::new(log_label).color(GREEN).size(12.0),
                    "Show or hide the history rail — every saved calculation, newest first",
                )
                .clicked()
                {
                    self.show_log = !self.show_log;
                    if self.show_log {
                        self.log_entries = load_history();
                    }
                }
            });
            // Second header row (v0.5.0): the steps style (#12)
            // and the PRACTICE view toggle (#15) — kept off row 1
            // so nothing clips at the minimum window width.
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("STEPS:").color(self.hint_col()).size(11.5));
                if tip_sel(
                    ui,
                    !self.settings.steps_deeper,
                    egui::RichText::new("Standard").size(12.0),
                    "Steps show the calculator's full result text",
                )
                .clicked()
                {
                    self.settings.steps_deeper = false;
                    self.persist_settings();
                }
                if tip_sel(
                    ui,
                    self.settings.steps_deeper,
                    egui::RichText::new("Deeper").size(12.0),
                    "Steps show four labelled stages: the formula, the formula with your values in it, the working, and the result",
                )
                .clicked()
                {
                    self.settings.steps_deeper = true;
                    self.persist_settings();
                }
                ui.separator();
                if tip_sel(
                    ui,
                    self.show_practice,
                    egui::RichText::new("PRACTICE").color(GREEN).size(12.0),
                    "Practice mode: generated problems with hidden answers, checked like Check My Answer",
                )
                .clicked()
                {
                    self.show_practice = !self.show_practice;
                    if self.show_practice {
                        self.show_batch = false;
                        self.show_help = false;
                        self.show_sets = false;
                        self.show_constants = false;
                        self.show_materials = false;
                    }
                }
                if self.show_practice {
                    ui.label(
                        egui::RichText::new(
                            "Pick a calculator in the strip, or use RANDOM PROBLEM in the practice view.",
                        )
                        .color(self.hint_col())
                        .size(11.5),
                    );
                }
                ui.separator();
                if tip_sel(
                    ui,
                    self.show_sets,
                    egui::RichText::new("SETS").color(GREEN).size(12.0),
                    "Problem sets: build a set, export its worksheet and answer key, generate a class set from a seed, mark a class CSV, write formula sheets, and manage your saved class examples",
                )
                .clicked()
                {
                    self.show_sets = !self.show_sets;
                    if self.show_sets {
                        self.show_batch = false;
                        self.show_help = false;
                        self.show_practice = false;
                        self.show_constants = false;
                        self.show_materials = false;
                    }
                }
                ui.separator();
                if tip_sel(
                    ui,
                    self.show_constants,
                    egui::RichText::new("CONSTANTS").color(GREEN).size(12.0),
                    "The constants library: CODATA 2018 physical constants with INSERT into the Expression Calculator, plus your own defined constants",
                )
                .clicked()
                {
                    self.show_constants = !self.show_constants;
                    if self.show_constants {
                        self.show_batch = false;
                        self.show_help = false;
                        self.show_practice = false;
                        self.show_sets = false;
                        self.show_materials = false;
                    }
                }
                ui.separator();
                if tip_sel(
                    ui,
                    self.show_materials,
                    egui::RichText::new("MATERIALS").color(GREEN).size(12.0),
                    "The materials table: typical density, resistivity and Young's modulus for common engineering materials, with a button that fills the beam calculator's E from a row",
                )
                .clicked()
                {
                    self.show_materials = !self.show_materials;
                    if self.show_materials {
                        self.show_batch = false;
                        self.show_help = false;
                        self.show_practice = false;
                        self.show_sets = false;
                        self.show_constants = false;
                    }
                }
                ui.separator();
                if tip_sel(
                    ui,
                    self.settings.projection,
                    egui::RichText::new("PROJECTION").size(12.0),
                    "Classroom projection mode: the whole window switches to larger text (about 1.5x) and higher contrast, so the back of a classroom can read it. Saved in the settings file",
                )
                .clicked()
                {
                    self.settings.projection = !self.settings.projection;
                    self.persist_settings();
                }
                if tip_sel(
                    ui,
                    self.demo,
                    egui::RichText::new("DEMO").size(12.0),
                    "Demonstration mode: run a calculator and its result values stay hidden — inputs and working structure show — until you press REVEAL for that result. For presenting live without giving answers away",
                )
                .clicked()
                {
                    self.demo = !self.demo;
                    self.demo_revealed = false;
                }
            });
            // Third header row (v0.6.0): what the teacher modes
            // changed, stated in the window itself — shown only
            // while a mode is on, so the header stays quiet
            // otherwise.
            if self.settings.projection || self.demo {
                ui.horizontal(|ui| {
                    if self.settings.projection {
                        ui.label(
                            egui::RichText::new(
                                "Projection mode: larger text, higher contrast.",
                            )
                            .color(GREEN)
                            .size(11.5),
                        );
                    }
                    if self.demo {
                        ui.label(
                            egui::RichText::new(
                                "Demonstration mode: answers are hidden until REVEAL. The calculation log still stores the real results — it is your record.",
                            )
                            .color(GREEN)
                            .size(11.5),
                        );
                    }
                });
            }
        });

        // ---- Mode rail (far left) ----
        // In projection mode both rails start narrower, so the centre
        // keeps a workable share of the zoomed (logically smaller)
        // window and all three panes stay on screen together.
        let proj = self.settings.projection;
        egui::SidePanel::left("modes")
            .resizable(true)
            .default_width(if proj { 150.0 } else { 178.0 })
            .width_range(if proj { 120.0..=200.0 } else { 140.0..=240.0 })
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("SEARCH")
                        .color(GREEN)
                        .strong()
                        .size(12.0),
                );
                let search_w = (ui.available_width() - 58.0).max(80.0);
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("filter calculators... (Ctrl+K)")
                            .desired_width(search_w),
                    );
                    let resp = resp.on_hover_text(
                        "Type to filter calculators by name — synonyms work too ('home loan' finds the mortgage calculator). Ctrl+K focuses this box.",
                    );
                    if self.focus_search {
                        resp.request_focus();
                        self.focus_search = false;
                    }
                    if tip_btn(
                        ui,
                        egui::RichText::new("Clear"),
                        "Clear the search text",
                    )
                    .clicked()
                    {
                        self.search.clear();
                    }
                });
                ui.add_space(8.0);
                ui.add_space(8.0);
                let fav_n = self.settings.favourites.len();
                if tip_sel(
                    ui,
                    self.category == "Favourites",
                    egui::RichText::new(format!("\u{2605} FAVOURITES ({fav_n})"))
                        .color(if self.category == "Favourites" {
                            GREEN
                        } else {
                            TEXT
                        })
                        .size(12.5),
                    "Your starred calculators plus the recent list (Alt+F)",
                )
                .clicked()
                {
                    self.category = "Favourites".to_string();
                }
                ui.add_space(4.0);
                // Course packs (item #24): the calculator strip
                // filters to the pack's members; membership comes
                // from meta.rs, counts computed from the data.
                ui.label(
                    egui::RichText::new("COURSE PACKS")
                        .color(GREEN)
                        .strong()
                        .size(12.0),
                );
                for pack in COURSE_PACK_NAMES {
                    let n = COURSE_PACKS
                        .iter()
                        .find(|(name, _)| name == pack)
                        .map(|(_, ids)| ids.len())
                        .unwrap_or(0);
                    let cat = format!("Pack: {pack}");
                    if tip_sel(
                        ui,
                        self.category == cat,
                        egui::RichText::new(format!("{pack} ({n})"))
                            .color(if self.category == cat { GREEN } else { TEXT })
                            .size(12.5),
                        "Show only the calculators tagged for this course level",
                    )
                    .clicked()
                    {
                        self.category = cat;
                    }
                }
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("MODES")
                        .color(GREEN)
                        .strong()
                        .size(12.0),
                );
                egui::ScrollArea::vertical()
                    .id_salt("mode-scroll")
                    .show(ui, |ui| {
                        let mut picked: Option<String> = None;
                        for m in &self.modes {
                            let n = if m.name == "All" {
                                self.calcs.len()
                            } else {
                                self.calcs.iter().filter(|c| c.category == m.name).count()
                            };
                            if mode_row(ui, m, n, self.category == m.name, self.hint_col()) {
                                picked = Some(m.name.clone());
                            }
                        }
                        if let Some(name) = picked {
                            self.category = name;
                        }
                    });
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "{} calculators in {} modes. Alt+ a shown key picks a mode.",
                        self.calcs.len(),
                        self.modes.len() - 1
                    ))
                    .color(self.hint_col())
                    .size(11.5),
                );
            });

        // ---- History rail (right; wide windows only) ----
        if self.show_log && wide {
            egui::SidePanel::right("history")
                .resizable(true)
                .default_width(if proj { 260.0 } else { 300.0 })
                .width_range(if proj { 220.0..=340.0 } else { 240.0..=420.0 })
                .show(ctx, |ui| {
                    let out = history_ui(
                        ui,
                        &self.log_entries,
                        self.backup_exists,
                        &mut self.log_edit,
                        &mut self.log_view,
                    );
                    if out.restore.is_some() {
                        hist_restore = out.restore;
                    }
                    hist_clear = hist_clear || out.clear;
                    hist_restore_cleared = hist_restore_cleared || out.restore_cleared;
                    if out.save_edit.is_some() {
                        hist_save_edit = out.save_edit;
                    }
                    if out.pin.is_some() {
                        hist_pin = out.pin;
                    }
                    if out.export.is_some() {
                        hist_export = out.export;
                    }
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_help {
                self.help_ui(ui);
                return;
            }
            if self.show_batch {
                self.batch_ui(ui);
                return;
            }
            if self.show_practice {
                egui::ScrollArea::vertical()
                    .id_salt("practice-scroll")
                    .show(ui, |ui| {
                        self.practice_ui(ui);
                    });
                return;
            }
            if self.show_sets {
                egui::ScrollArea::vertical()
                    .id_salt("sets-scroll")
                    .show(ui, |ui| {
                        self.sets_ui(ui);
                    });
                return;
            }
            if self.show_constants {
                egui::ScrollArea::vertical()
                    .id_salt("constants-scroll")
                    .show(ui, |ui| {
                        self.constants_ui(ui);
                    });
                return;
            }
            if self.show_materials {
                egui::ScrollArea::vertical()
                    .id_salt("materials-scroll")
                    .show(ui, |ui| {
                        self.materials_ui(ui);
                    });
                return;
            }
            // Narrow window + log on: the centre becomes the log view
            // (the v0.1.x behaviour the LOG toggle has always had).
            if self.show_log && !wide {
                let out = history_ui(
                    ui,
                    &self.log_entries,
                    self.backup_exists,
                    &mut self.log_edit,
                    &mut self.log_view,
                );
                if out.restore.is_some() {
                    hist_restore = out.restore;
                }
                hist_clear = hist_clear || out.clear;
                hist_restore_cleared = hist_restore_cleared || out.restore_cleared;
                if out.save_edit.is_some() {
                    hist_save_edit = out.save_edit;
                }
                if out.pin.is_some() {
                    hist_pin = out.pin;
                }
                if out.export.is_some() {
                    hist_export = out.export;
                }
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt("detail-scroll")
                .show(ui, |ui| {
                    self.centre_ui(ui, enter_calc);
                });
        });

        if hist_clear {
            // CLEAR LOG backs the entries up first (undo below).
            let had_entries = !self.log_entries.is_empty();
            let _ = clear_history_with_backup();
            self.log_entries.clear();
            if had_entries {
                self.backup_exists = true;
            }
        }
        if hist_restore_cleared {
            if let Ok(entries) = restore_history_backup() {
                self.log_entries = entries;
            }
            self.backup_exists = history_backup_exists();
        }
        if let Some((stored_idx, tag, note)) = hist_save_edit {
            // Lab notebook (#39): the library backs the log up
            // before editing. Its index counts from the newest;
            // the editor works in stored (oldest-first) indexes.
            if stored_idx < self.log_entries.len() {
                let newest = self.log_entries.len() - 1 - stored_idx;
                if annotate_entry_at(&history_path(), newest, Some(&tag), Some(&note)).is_ok() {
                    self.log_entries = load_history();
                }
            }
            self.log_edit.entry = None;
        }
        if let Some((stored_idx, pinned)) = hist_pin {
            // Log polish (#49): the library backs the log up
            // before the pin change, same as the tag/note edits.
            if stored_idx < self.log_entries.len() {
                let newest = self.log_entries.len() - 1 - stored_idx;
                if set_entry_pinned_at(&history_path(), newest, pinned).is_ok() {
                    self.log_entries = load_history();
                }
            }
        }
        if let Some((filter, path)) = hist_export {
            // Log polish (#49): write the filtered set. An
            // existing target is first copied to <path>.bak (the
            // house backup-before-overwrite rule).
            if path.is_empty() {
                self.log_view.export_msg =
                    "Type the file to write the export to, then press EXPORT FILTERED again."
                        .to_string();
            } else {
                let text = log_export_text(&self.log_entries, &filter);
                let count = filter_entries(&self.log_entries, &filter).len();
                let target = std::path::Path::new(&path);
                let mut backup_note = String::new();
                let mut blocked: Option<String> = None;
                if target.exists() {
                    let bak = format!("{path}.bak");
                    match std::fs::copy(target, &bak) {
                        Ok(_) => {
                            backup_note = format!(" (the previous file was copied to {bak} first)")
                        }
                        Err(e) => {
                            blocked = Some(format!(
                                "Could not back up {path} before overwriting it: {e}"
                            ))
                        }
                    }
                }
                self.log_view.export_msg = match blocked {
                    Some(msg) => msg,
                    None => match std::fs::write(target, text) {
                        Ok(()) => format!(
                            "Exported {count} of {} entries to {path}.{backup_note}",
                            self.log_entries.len()
                        ),
                        Err(e) => format!("Could not write {path}: {e}"),
                    },
                };
            }
        }
        if let Some(e) = hist_restore {
            self.restore(&e);
        }
    }
}

/// Round gridline spacing for the plot: 1, 2 or 5 times a power
/// of ten, at or above the wanted step.
fn nice_step(want: f64) -> f64 {
    if want <= 0.0 || !want.is_finite() {
        return 1.0;
    }
    let mag = 10f64.powf(want.log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if mag * m >= want {
            return mag * m;
        }
    }
    mag * 10.0
}

impl App {
    /// The centre column: calculator chip strip, title + description,
    /// display, form, Calculate, steps footer, VARY section.
    fn centre_ui(&mut self, ui: &mut egui::Ui, enter_calc: bool) {
        // ---- Calculator strip for the active mode ----
        ui.label(
            egui::RichText::new(format!("CALCULATORS — {}", self.category.to_uppercase()))
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        // Recent strip (item #46): in Favourites mode the last 8
        // calculators actually run sit above the strip as chips.
        if self.category == "Favourites" && !self.settings.recent.is_empty() {
            ui.label(
                egui::RichText::new("RECENT")
                    .color(self.hint_col())
                    .size(11.5),
            );
            let mut recent_pick: Option<usize> = None;
            ui.horizontal_wrapped(|ui| {
                for id in self.settings.recent.clone() {
                    if let Some(i) = self.calcs.iter().position(|c| c.id == id) {
                        if tip_btn(
                            ui,
                            egui::RichText::new(self.calcs[i].name)
                                .color(TEXT)
                                .size(12.0),
                            "Recently used — click to open it",
                        )
                        .clicked()
                        {
                            recent_pick = Some(i);
                        }
                    }
                }
            });
            if let Some(i) = recent_pick {
                self.select(i);
            }
            ui.add_space(4.0);
        }
        let mut to_select: Option<usize> = None;
        egui::ScrollArea::vertical()
            .id_salt("strip-scroll")
            .max_height(112.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut any = false;
                    // Item #47: while searching, chips are ranked
                    // (exact name > synonym > description).
                    let order: Vec<usize> = if self.search.trim().is_empty() {
                        (0..self.calcs.len()).collect()
                    } else {
                        search_ranked(&self.calcs, &self.search)
                    };
                    for i in order {
                        let c = &self.calcs[i];
                        if !self.visible(c) {
                            continue;
                        }
                        any = true;
                        let selected = self.selected == Some(i);
                        let resp = tip_chip(ui, c.name, selected);
                        if resp.clicked() {
                            to_select = Some(i);
                        }
                    }
                    if !any {
                        ui.label(
                            egui::RichText::new(if self.category == "Favourites" {
                                "No favourites yet — open a calculator and press its \u{2606} FAVOURITE button. Your recent calculators are listed above."
                            } else {
                                "No calculators match — clear the search or pick another mode."
                            })
                            .color(TEXT),
                        );
                    }
                });
            });
        if let Some(i) = to_select {
            self.select(i);
        }
        ui.add_space(8.0);
        ui.separator();

        let Some(idx) = self.selected else {
            egui::Frame::new()
                .fill(DISPLAY_BG)
                .inner_margin(egui::Margin::same(10))
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    egui::Color32::from_rgb(0x26, 0x26, 0x26),
                ))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("PICK A CALCULATOR")
                            .color(GREEN)
                            .strong()
                            .size(14.0),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Choose one from the strip above — or filter by name with                              the Search box in the rail. Modes switch with Alt+ the                              key printed on the rail.",
                        )
                        .color(TEXT),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Every calculator here also works from the command line — e.g. CalcDex compound-interest 1000 5 10 12")
                            .color(self.hint_col()),
                    );
                });
            return;
        };
        // Borrow fields separately to satisfy the borrow checker.
        let (id, name, category, desc, inputs_labels, is_list, date_groups, is_text) = {
            let c = &self.calcs[idx];
            (
                c.id,
                c.name,
                c.category,
                c.desc,
                c.inputs,
                c.list,
                c.date_groups,
                c.text_run.is_some(),
            )
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(name.to_uppercase())
                    .color(GREEN)
                    .strong()
                    .size(16.0),
            );
            let is_fav = self.settings.favourites.contains(id);
            let star = if is_fav {
                "\u{2605} FAVOURITE"
            } else {
                "\u{2606} FAVOURITE"
            };
            if tip_btn(
                ui,
                egui::RichText::new(star).color(if is_fav { GREEN } else { TEXT }),
                "Star this calculator — it joins the FAVOURITES mode at the top of the rail (click again to unstar)",
            )
            .clicked()
            {
                self.settings.toggle_favourite(id);
                self.persist_settings();
            }
        });
        ui.label(
            egui::RichText::new(format!("[{category}]  id: {id}"))
                .color(TEXT)
                .size(11.5),
        );
        ui.add_space(4.0);
        ui.label(egui::RichText::new(desc).color(TEXT));
        ui.add_space(8.0);

        // ---- Display (Design 3: prominent result zone) ----
        let is_error = self.output.starts_with("Error")
            || self.output.contains("is not a number")
            || self.output.contains("needs a number");
        // Demonstration mode (#25): this result's values are
        // masked until REVEAL is pressed for it.
        let demo_hidden = self.demo && !self.demo_revealed && !self.output.is_empty() && !is_error;
        let mut demo_toggle = false;
        egui::Frame::new()
            .fill(DISPLAY_BG)
            .inner_margin(egui::Margin::same(10))
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(0x26, 0x26, 0x26),
            ))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("DISPLAY — {}", name.to_uppercase()))
                        .color(self.hint_col())
                        .size(11.5),
                );
                if self.output.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "Enter the values below, then press CALCULATE (or Enter).",
                        )
                        .color(self.hint_col())
                        .size(14.0),
                    );
                } else if is_error {
                    ui.label(egui::RichText::new(&self.output).color(RED).size(16.0));
                } else if demo_hidden {
                    ui.label(
                        egui::RichText::new(DEMO_HIDDEN)
                            .color(GREEN)
                            .strong()
                            .size(20.0),
                    );
                    ui.label(
                        egui::RichText::new(
                            "The inputs and the structure of the working are in the steps below — the result values are masked.",
                        )
                        .color(TEXT)
                        .size(12.5),
                    );
                } else {
                    let headline = self.output.lines().next().unwrap_or("");
                    ui.label(
                        egui::RichText::new(headline)
                            .color(GREEN)
                            .strong()
                            .size(24.0),
                    );
                    let fmt = self.effective_format();
                    if let Some(line) =
                        headline_display(&self.output, &fmt, &self.raw_input_strings())
                    {
                        ui.label(egui::RichText::new(line).color(TEXT).size(15.0));
                    }
                    if fmt.mode == DisplayMode::Exact {
                        // Item #14: the exact-form line for this run.
                        let line = if is_text {
                            exact_line_for_expression(
                                &self.list_input,
                                if self.expr_degrees {
                                    AngleMode::Degrees
                                } else {
                                    AngleMode::Radians
                                },
                            )
                        } else {
                            exact_line(&self.calcs[idx], &self.last_args)
                        };
                        ui.label(egui::RichText::new(line).color(TEXT).size(15.0));
                    }
                }
                if self.demo && !self.output.is_empty() && !is_error {
                    ui.add_space(4.0);
                    if demo_hidden {
                        if tip_btn(
                            ui,
                            egui::RichText::new("REVEAL").color(GREEN).strong(),
                            "Show this result's values (only this result — the next run is hidden again)",
                        )
                        .clicked()
                        {
                            demo_toggle = true;
                        }
                    } else if tip_btn(
                        ui,
                        egui::RichText::new("HIDE AGAIN").color(TEXT),
                        "Hide this result's values again",
                    )
                    .clicked()
                    {
                        demo_toggle = true;
                    }
                }
            });
        if demo_toggle {
            self.demo_revealed = !self.demo_revealed;
        }
        ui.add_space(8.0);
        if !self.output.is_empty() && !is_error && !demo_hidden {
            self.result_tools_ui(ui);
            ui.add_space(8.0);
        }

        // ---- Form ----
        // Input rows: the label column takes 38% of the pane width
        // (clamped 130-300 pt) and the field takes what is left
        // (clamped 150-480 pt), recomputed every frame so resizing or
        // zooming reflows instead of clipping. Every centre block —
        // display, form, steps, VARY — spans the full centre width,
        // so the column reads evenly at any size.
        let avail = ui.available_width();
        let label_w = (avail * 0.38).clamp(130.0, 300.0);
        let field_w = (avail - label_w - 16.0).clamp(150.0, 480.0);
        if is_list || is_text {
            ui.label(
                egui::RichText::new(if is_text {
                    "EXPRESSION"
                } else {
                    "NUMBERS (COMMA, SPACE OR NEW LINE SEPARATED — PASTE A COLUMN WORKS)"
                })
                .color(TEXT)
                .size(12.0),
            );
            let resp = ui.add(
                egui::TextEdit::multiline(&mut self.list_input)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            let _ = resp.on_hover_text(if is_text {
                "Type the expression here — e.g. 2*(3+4)^2 or sqrt(2)/2 — then press CALCULATE"
            } else {
                "Type or paste the numbers here — commas, spaces, tabs, semicolons or new lines all separate values, so a copied spreadsheet column works as-is"
            });
            ui.label(
                egui::RichText::new(hint_for(&self.calcs[idx], 0))
                    .color(self.hint_col())
                    .size(11.5),
            );
            if is_text {
                // Angle mode (#5): degrees or radians for trig.
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("ANGLE MODE:").color(self.hint_col()).size(11.5));
                    if tip_sel(
                        ui,
                        self.expr_degrees,
                        egui::RichText::new("DEGREES").size(12.0),
                        "Trigonometry takes degrees (sin(30) = 0.5); inverse trig answers in degrees",
                    )
                    .clicked()
                    {
                        self.expr_degrees = true;
                    }
                    if tip_sel(
                        ui,
                        !self.expr_degrees,
                        egui::RichText::new("RADIANS").size(12.0),
                        "Trigonometry takes radians (sin(pi/2) = 1); inverse trig answers in radians",
                    )
                    .clicked()
                    {
                        self.expr_degrees = false;
                    }
                });
            }
        } else {
            let mut gi = 0usize;
            let mut i = 0usize;
            while i < inputs_labels.len() {
                if let Some(g) = date_groups.iter().find(|g| g.start == i) {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_sized(
                            [label_w, 20.0],
                            egui::Label::new(
                                egui::RichText::new(g.label.to_uppercase())
                                    .color(TEXT)
                                    .size(12.0),
                            ),
                        );
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.date_texts[gi])
                                .hint_text("MM/DD/YYYY")
                                .desired_width(170.0),
                        );
                        let _ = resp.on_hover_text(
                            "Type the date as MM/DD/YYYY (the Today? button fills in today)",
                        );
                        if tip_btn(
                            ui,
                            egui::RichText::new("Today?").color(GREEN),
                            "Fill in today's date",
                        )
                        .clicked()
                        {
                            self.date_texts[gi] = today_str();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_sized([label_w, 20.0], egui::Label::new(""));
                        ui.label(egui::RichText::new(DATE_HINT).color(TEXT).size(11.5));
                    });
                    gi += 1;
                    i += 3;
                } else {
                    let label = inputs_labels[i];
                    ui.horizontal_wrapped(|ui| {
                        ui.add_sized(
                            [label_w, 20.0],
                            egui::Label::new(
                                egui::RichText::new(label.to_uppercase())
                                    .color(TEXT)
                                    .size(12.0),
                            ),
                        );
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.inputs[i]).desired_width(field_w),
                        );
                        let _ =
                            resp.on_hover_text(format!("Type a value for {}", inputs_labels[i]));
                    });
                    ui.horizontal(|ui| {
                        ui.add_sized([label_w, 20.0], egui::Label::new(""));
                        ui.label(
                            egui::RichText::new(hint_for(&self.calcs[idx], i))
                                .color(self.hint_col())
                                .size(11.5),
                        );
                    });
                    i += 1;
                }
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if tip_btn_sized(
                ui,
                egui::vec2(180.0, 36.0),
                egui::RichText::new("CALCULATE").color(GREEN).strong(),
                "Run the calculation with the values above (or press Enter)",
            )
            .clicked()
            {
                self.calculate();
            }
            ui.label(
                egui::RichText::new("Enter = Calculate")
                    .color(self.hint_col())
                    .size(11.5),
            );
            if tip_btn(
                ui,
                egui::RichText::new("TRY AN EXAMPLE").color(TEXT),
                "Fill this calculator's built-in teaching example and run it at once",
            )
            .clicked()
            {
                self.run_example();
            }
            if tip_btn(
                ui,
                egui::RichText::new("RESET").color(TEXT),
                "Put the smart default values back in every field",
            )
            .clicked()
            {
                self.reset_defaults();
            }
        });

        if enter_calc {
            self.calculate();
        }

        // ---- Formula & symbols (item #19): under the form ----
        {
            let calc = &self.calcs[idx];
            let glossary = glossary_for(calc);
            if !glossary.is_empty() {
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("FORMULA & SYMBOLS")
                        .color(GREEN)
                        .strong()
                        .size(12.0),
                );
                ui.label(
                    egui::RichText::new(formula_for(calc))
                        .color(TEXT)
                        .size(13.0),
                );
                for (sym, meaning) in glossary {
                    ui.label(
                        egui::RichText::new(format!("{sym} = {meaning}"))
                            .color(TEXT)
                            .size(12.5),
                    );
                }
            }
        }

        // ---- Check my answer (item #13) ----
        self.check_ui(ui);

        // ---- Steps footer: the FULL result text, never truncated ----
        if !self.output.is_empty() && !is_error {
            ui.add_space(8.0);
            ui.separator();
            ui.label(
                egui::RichText::new("STEPS / FULL RESULT")
                    .color(GREEN)
                    .strong()
                    .size(12.0),
            );
            let fmt = self.effective_format();
            let footer = if self.settings.steps_deeper && !self.last_args.is_empty() {
                // Item #12: the four labelled stages. Stage 3 is
                // the calculator's own output, verbatim.
                deeper_steps(&self.calcs[idx], &self.last_args, &self.output)
            } else if fmt.is_off() {
                self.output.clone()
            } else {
                annotate_display(&self.output, &fmt, &self.raw_input_strings())
            };
            // Demonstration mode (#25): the steps keep their
            // structure and input values; result values are masked.
            let footer = if demo_hidden {
                mask_result_text(&footer, &self.last_args)
            } else {
                footer
            };
            ui.label(egui::RichText::new(footer).color(TEXT).size(13.0));
        }
        self.export_ui(ui);
        self.plot_ui(ui);
        self.vary_ui(ui);
    }

    /// CHECK MY ANSWER (item #13): the student types their own
    /// answer; CalcDex runs the calculator with the form's values
    /// and checks it — exactly for the fraction calculators,
    /// within 0.5% for everything else, with a named likely slip
    /// when the wrong answer matches one.
    fn check_ui(&mut self, ui: &mut egui::Ui) {
        let Some(idx) = self.selected else { return };
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("CHECK MY ANSWER")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.label(
            egui::RichText::new(
                "Work it out yourself first, type your answer, and CalcDex runs the calculator and checks it. The fraction calculators are checked exactly; everything else allows 0.5% for rounding in your working.",
            )
            .color(self.hint_col())
            .size(11.5),
        );
        let mut go = false;
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.check_text)
                    .hint_text("your answer")
                    .desired_width(200.0),
            )
            .on_hover_text("Type your answer — a number, or a fraction like 3/4");
            if tip_btn(
                ui,
                egui::RichText::new("CHECK MY ANSWER").color(GREEN),
                "Run the calculator with the values above and check your answer against its result",
            )
            .clicked()
            {
                go = true;
            }
        });
        if go {
            self.calculate();
            let is_err = self.output.is_empty()
                || self.output.starts_with("Error")
                || self.output.contains("is not a number")
                || self.output.contains("needs a number");
            if is_err {
                self.check_ok = false;
                self.check_msg =
                    "Fill in the calculator's fields first — the check uses the values above."
                        .to_string();
            } else {
                match check_answer(&self.calcs[idx], &self.output, &self.check_text) {
                    Ok(o) => {
                        self.check_ok = o.correct;
                        self.check_msg = o.message;
                    }
                    Err(e) => {
                        self.check_ok = false;
                        self.check_msg = e;
                    }
                }
            }
        }
        if !self.check_msg.is_empty() {
            ui.label(
                egui::RichText::new(&self.check_msg)
                    .color(if self.check_ok {
                        GREEN
                    } else {
                        egui::Color32::from_rgb(0xe5, 0xc0, 0x7b)
                    })
                    .size(13.0),
            );
        }
    }

    /// EXPORT SOLUTION (item #18): write the worked solution —
    /// calculator name, formula, inputs, full steps and result —
    /// as a plain-text file at the typed path (egui has no file
    /// dialog; the path field is the honest pattern used for
    /// BATCH too).
    fn export_ui(&mut self, ui: &mut egui::Ui) {
        let Some(idx) = self.selected else { return };
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("EXPORT WORKED SOLUTION")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        let mut go = false;
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.export_path).desired_width(420.0),
            )
            .on_hover_text("The file the worked solution is written to (plain text)");
            if tip_btn(
                ui,
                egui::RichText::new("EXPORT SOLUTION").color(GREEN),
                "Write the worked solution — name, formula, inputs, steps and result — to the file named on the left",
            )
            .clicked()
            {
                go = true;
            }
        });
        if go {
            if self.output.is_empty() {
                self.calculate();
            }
            let is_err = self.output.is_empty()
                || self.output.starts_with("Error")
                || self.output.contains("is not a number")
                || self.output.contains("needs a number");
            if is_err {
                self.export_msg =
                    "Run the calculator first — there is no result to export yet.".to_string();
            } else {
                // Demonstration mode (#25): an export made while
                // the answer is hidden carries the masked working,
                // never the revealed values.
                let out_for_export = if self.demo && !self.demo_revealed {
                    mask_result_text(&self.output.clone(), &self.last_args.clone())
                } else {
                    self.output.clone()
                };
                let text = if self.calcs[idx].text_run.is_some() {
                    solution_text_for_text(
                        self.calcs[idx].name,
                        &self.list_input.clone(),
                        &out_for_export,
                    )
                } else {
                    solution_text(
                        &self.calcs[idx],
                        &self.last_args.clone(),
                        &out_for_export,
                        self.settings.steps_deeper,
                    )
                };
                let path = self.export_path.trim().to_string();
                self.export_msg = match std::fs::write(&path, text) {
                    Ok(()) => format!("Worked solution written to {path}."),
                    Err(e) => format!("Could not write {path}: {e}"),
                };
            }
        }
        if !self.export_msg.is_empty() {
            ui.label(egui::RichText::new(&self.export_msg).color(TEXT).size(12.5));
        }
    }

    /// PLOT (item #17): a hand-drawn curve for the Expression
    /// Calculator (the expression in x) and the quadratic solver
    /// (a*x^2 + b*x + c built from the form). Sampled at pixel
    /// resolution; undefined points and steep jumps break the
    /// line instead of streaking.
    fn plot_ui(&mut self, ui: &mut egui::Ui) {
        let Some(idx) = self.selected else { return };
        let is_text = self.calcs[idx].text_run.is_some();
        if !is_text && self.calcs[idx].id != "quadratic" {
            return;
        }
        let expr: String = if is_text {
            self.list_input.trim().to_string()
        } else {
            let v: Vec<f64> = self
                .inputs
                .iter()
                .map(|t| t.trim().parse::<f64>().unwrap_or(0.0))
                .collect();
            let g = |i: usize| v.get(i).copied().unwrap_or(0.0);
            format!(
                "{}*x^2+{}*x+{}",
                calcdex::n(g(0)),
                calcdex::n(g(1)),
                calcdex::n(g(2))
            )
        };
        ui.add_space(8.0);
        ui.separator();
        ui.label(egui::RichText::new("PLOT").color(GREEN).strong().size(12.0));
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("x from").color(TEXT).size(12.0));
            ui.add(egui::TextEdit::singleline(&mut self.plot_from).desired_width(70.0))
                .on_hover_text("The left edge of the plot (the smallest x shown)");
            ui.label(egui::RichText::new("to").color(TEXT).size(12.0));
            ui.add(egui::TextEdit::singleline(&mut self.plot_to).desired_width(70.0))
                .on_hover_text("The right edge of the plot (the largest x shown)");
            ui.label(
                egui::RichText::new(format!("y = {expr}"))
                    .color(self.hint_col())
                    .size(11.5),
            );
        });
        let (Ok(from), Ok(to)) = (
            self.plot_from.trim().parse::<f64>(),
            self.plot_to.trim().parse::<f64>(),
        ) else {
            ui.label(
                egui::RichText::new("The plot range needs two numbers, from smaller than to.")
                    .color(RED)
                    .size(12.5),
            );
            return;
        };
        if from >= to {
            ui.label(
                egui::RichText::new("The plot range needs 'from' smaller than 'to'.")
                    .color(RED)
                    .size(12.5),
            );
            return;
        }
        if let Err(e) = calcdex::students::parse_ast(&expr) {
            ui.label(egui::RichText::new(e).color(RED).size(12.5));
            return;
        }
        let mode = if self.expr_degrees {
            AngleMode::Degrees
        } else {
            AngleMode::Radians
        };
        let (rect, _resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width().max(200.0), 220.0),
            egui::Sense::hover(),
        );
        let count = (rect.width() as usize).max(10);
        let samples = match sample_function(&expr, from, to, count, mode) {
            Ok(v) => v,
            Err(_) => return,
        };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, DISPLAY_BG);
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x26, 0x26, 0x26)),
            egui::StrokeKind::Inside,
        );
        let finite: Vec<f64> = samples.iter().filter_map(|(_, y)| *y).collect();
        if finite.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Nothing to plot — the expression gives no values in this range.",
                egui::FontId::monospace(12.0),
                TEXT,
            );
            return;
        }
        let mut ymin = finite.iter().cloned().fold(f64::INFINITY, f64::min);
        let mut ymax = finite.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if ymax - ymin < 1e-9 {
            ymax += 0.5;
            ymin -= 0.5;
        }
        let pad = (ymax - ymin) * 0.08;
        ymin -= pad;
        ymax += pad;
        let x2px = |x: f64| rect.left() + ((x - from) / (to - from)) as f32 * rect.width();
        let y2px = |y: f64| rect.bottom() - ((y - ymin) / (ymax - ymin)) as f32 * rect.height();
        // Gridlines at round values (1/2/5 x 10^k spacing).
        let grid = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x26, 0x26, 0x26));
        let step_x = nice_step((to - from) / 6.0);
        let mut gx = (from / step_x).ceil() * step_x;
        let mut guard = 0;
        while gx <= to && guard < 500 {
            let px = x2px(gx);
            painter.line_segment(
                [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                grid,
            );
            gx += step_x;
            guard += 1;
        }
        let step_y = nice_step((ymax - ymin) / 6.0);
        let mut gy = (ymin / step_y).ceil() * step_y;
        let mut guard = 0;
        while gy <= ymax && guard < 500 {
            let py = y2px(gy);
            painter.line_segment(
                [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                grid,
            );
            gy += step_y;
            guard += 1;
        }
        // Axes through zero, brighter than the grid.
        let axis = egui::Stroke::new(1.0_f32, self.muted_col());
        if from < 0.0 && to > 0.0 {
            let px = x2px(0.0);
            painter.line_segment(
                [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                axis,
            );
        }
        if ymin < 0.0 && ymax > 0.0 {
            let py = y2px(0.0);
            painter.line_segment(
                [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                axis,
            );
        }
        // The curve: consecutive defined samples join; a gap or a
        // near-full-height jump breaks the line (tan/log poles).
        let mut prev: Option<egui::Pos2> = None;
        for (x, y) in &samples {
            match y {
                Some(yv) => {
                    let p = egui::pos2(x2px(*x), y2px(*yv));
                    if let Some(q) = prev {
                        if (p.y - q.y).abs() <= rect.height() * 0.9 {
                            painter.line_segment([q, p], egui::Stroke::new(1.5_f32, GREEN));
                        }
                    }
                    prev = Some(p);
                }
                None => prev = None,
            }
        }
        painter.text(
            egui::pos2(rect.left() + 4.0, rect.bottom() - 4.0),
            egui::Align2::LEFT_BOTTOM,
            format!("x: {} to {}", calcdex::n(from), calcdex::n(to)),
            egui::FontId::monospace(10.5),
            HINT,
        );
        painter.text(
            egui::pos2(rect.left() + 4.0, rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            format!("y: {} to {}", calcdex::n(ymin), calcdex::n(ymax)),
            egui::FontId::monospace(10.5),
            HINT,
        );
    }

    /// PRACTICE (item #15): generated problems for the selected
    /// calculator or a random one; answers hidden until CHECK or
    /// REVEAL; the session tally lives in memory only and says so.
    fn practice_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("PRACTICE")
                .color(GREEN)
                .strong()
                .size(16.0),
        );
        ui.add_space(4.0);
        ui.label(egui::RichText::new(PRACTICE_EXCLUSION_NOTE).color(TEXT));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("SEED:").color(self.hint_col()).size(11.5));
            ui.add(
                egui::TextEdit::singleline(&mut self.practice_seed).desired_width(90.0),
            )
            .on_hover_text(
                "Problems are made from this seed — the same seed always makes the same sequence of problems",
            );
            if tip_btn(
                ui,
                egui::RichText::new("APPLY SEED").color(TEXT),
                "Restart the problem sequence from the seed in the box",
            )
            .clicked()
            {
                match self.practice_seed.trim().parse::<u64>() {
                    Ok(n) => {
                        self.practice_rng = Rng::new(n);
                        self.practice_current = None;
                        self.practice_reveal.clear();
                        self.practice_msg =
                            format!("Seed {n} applied — NEW PROBLEM starts its sequence.");
                    }
                    Err(_) => {
                        self.practice_msg = "The seed must be a whole number.".to_string();
                    }
                }
            }
        });
        let mut new_selected = false;
        let mut new_random = false;
        ui.horizontal_wrapped(|ui| {
            if tip_btn(
                ui,
                egui::RichText::new("NEW PROBLEM (SELECTED)").color(GREEN),
                "Make a problem from the calculator picked in the strip (close PRACTICE to pick another)",
            )
            .clicked()
            {
                new_selected = true;
            }
            if tip_btn(
                ui,
                egui::RichText::new("RANDOM PROBLEM").color(GREEN),
                "Make a problem from a random practice-suitable calculator",
            )
            .clicked()
            {
                new_random = true;
            }
        });
        if new_selected {
            match self.selected {
                Some(idx) => {
                    let calc = &self.calcs[idx];
                    if !practice_eligible(calc) {
                        self.practice_msg = format!(
                            "{} is not used for practice problems. {}",
                            calc.name, PRACTICE_EXCLUSION_NOTE
                        );
                    } else {
                        match practice_problem(calc, &mut self.practice_rng) {
                            Some(p) => {
                                self.practice_current = Some(p);
                                self.practice_reveal.clear();
                                self.practice_msg.clear();
                                self.practice_answer.clear();
                            }
                            None => {
                                self.practice_msg =
                                    "Could not make a problem for that calculator — try another."
                                        .to_string();
                            }
                        }
                    }
                }
                None => {
                    self.practice_msg =
                        "Pick a calculator in the strip first, or press RANDOM PROBLEM."
                            .to_string();
                }
            }
        }
        if new_random {
            match practice_random(&self.calcs, &mut self.practice_rng) {
                Some((_, p)) => {
                    self.practice_current = Some(p);
                    self.practice_reveal.clear();
                    self.practice_msg.clear();
                    self.practice_answer.clear();
                }
                None => {
                    self.practice_msg = "Could not make a problem — try again.".to_string();
                }
            }
        }
        if !self.practice_msg.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&self.practice_msg)
                    .color(TEXT)
                    .size(12.5),
            );
        }
        if let Some(p) = self.practice_current.clone() {
            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new(&p.question).color(TEXT).size(14.0));
            let mut do_check = false;
            let mut do_reveal = false;
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.practice_answer)
                        .hint_text("your answer")
                        .desired_width(200.0),
                )
                .on_hover_text("Type your answer — a number, or a fraction like 3/4");
                if tip_btn(
                    ui,
                    egui::RichText::new("CHECK").color(GREEN),
                    "Check your answer against the worked solution and count it in the tally",
                )
                .clicked()
                {
                    do_check = true;
                }
                if tip_btn(
                    ui,
                    egui::RichText::new("REVEAL").color(TEXT),
                    "Show the worked solution (does not change the tally)",
                )
                .clicked()
                {
                    do_reveal = true;
                }
            });
            if do_check {
                self.practice_attempted += 1;
                let calc = self.calcs.iter().find(|c| c.id == p.calc_id);
                match (calc, run_calc(&p.calc_id, &p.args)) {
                    (Some(c), Ok(out)) => match check_answer(c, &out, &self.practice_answer) {
                        Ok(o) => {
                            if o.correct {
                                self.practice_correct += 1;
                            }
                            self.practice_msg = o.message;
                        }
                        Err(e) => {
                            self.practice_msg = e;
                        }
                    },
                    _ => {
                        self.practice_msg = "Could not re-run that problem.".to_string();
                    }
                }
            }
            if do_reveal {
                if let Ok(out) = run_calc(&p.calc_id, &p.args) {
                    self.practice_reveal = out;
                }
            }
            if !self.practice_reveal.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("WORKED SOLUTION")
                        .color(GREEN)
                        .strong()
                        .size(12.0),
                );
                ui.label(
                    egui::RichText::new(&self.practice_reveal)
                        .color(TEXT)
                        .size(13.0),
                );
            }
        }
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "Session tally: attempted {}, correct {} — kept in memory only, not saved.",
                self.practice_attempted, self.practice_correct
            ))
            .color(self.hint_col())
            .size(11.5),
        );
    }

    /// VARY ONE INPUT (item #16): sweep one numeric input across a
    /// range with the form's other values fixed; the table lands
    /// under the steps footer. Fixed-input calculators only.
    fn vary_ui(&mut self, ui: &mut egui::Ui) {
        let Some(idx) = self.selected else { return };
        if self.calcs[idx].list || self.calcs[idx].text_run.is_some() {
            return;
        }
        let labels: Vec<&str> = self.calcs[idx].inputs.to_vec();
        if self.vary_field >= labels.len() {
            self.vary_field = 0;
        }
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("VARY ONE INPUT")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.label(
            egui::RichText::new(
                "Hold the other inputs as they are and run this calculator across a range of one input (at most 200 rows).",
            )
            .color(self.hint_col())
            .size(11.5),
        );
        let mut run = false;
        ui.horizontal_wrapped(|ui| {
            let combo = egui::ComboBox::from_label("Input")
                .selected_text(labels[self.vary_field])
                .show_ui(ui, |ui| {
                    for (fi, label) in labels.iter().enumerate() {
                        tip_sel_value(
                            ui,
                            &mut self.vary_field,
                            fi,
                            label,
                            "Sweep this input across the range",
                        );
                    }
                });
            combo
                .response
                .on_hover_text("Pick which input the sweep changes");
            ui.add(
                egui::TextEdit::singleline(&mut self.vary_start)
                    .hint_text("start")
                    .desired_width(70.0),
            )
            .on_hover_text("The first value of the sweep");
            ui.add(
                egui::TextEdit::singleline(&mut self.vary_end)
                    .hint_text("end")
                    .desired_width(70.0),
            )
            .on_hover_text("The last value of the sweep");
            ui.add(
                egui::TextEdit::singleline(&mut self.vary_step)
                    .hint_text("step")
                    .desired_width(70.0),
            )
            .on_hover_text("How much the swept input grows by each row");
            if tip_btn(
                ui,
                egui::RichText::new("RUN VARY").color(GREEN),
                "Run the sweep and show the table below",
            )
            .clicked()
            {
                run = true;
            }
        });
        if run {
            self.do_vary();
        }
        if !self.vary_error.is_empty() {
            ui.label(egui::RichText::new(&self.vary_error).color(RED).size(12.5));
        }
        if !self.vary_rows.is_empty() {
            let field_label = labels[self.vary_field];
            egui::ScrollArea::vertical()
                .id_salt("vary-scroll")
                .max_height(180.0)
                .show(ui, |ui| {
                    for (value, result) in &self.vary_rows {
                        let cell = match result {
                            Ok(out) => headline_of(out).to_string(),
                            Err(e) => format!("ERROR: {e}"),
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{field_label} = {}  ->  {cell}",
                                calcdex::n(*value)
                            ))
                            .color(TEXT)
                            .size(12.5),
                        );
                    }
                });
        }
    }

    /// BATCH FROM CSV (item #31): path fields + RUN BATCH + summary.
    /// egui has no native file-picker dialogs, so paths are typed —
    /// stated honestly here and in the README.
    fn batch_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("BATCH FROM CSV")
                .color(GREEN)
                .strong()
                .size(16.0),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Run ONE calculator over many rows of a CSV file. The file's header row must name the calculator's fields in order; every row produces its inputs plus the result (or an ERROR note) in the output file. Paths are typed — this toolkit has no file-picker dialog.",
            )
            .color(TEXT),
        );
        ui.add_space(8.0);
        match self.selected {
            Some(idx) => {
                let c = &self.calcs[idx];
                ui.label(
                    egui::RichText::new(format!("Calculator: {} ({})", c.name, c.id))
                        .color(GREEN)
                        .size(13.0),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Expected header: {}",
                        display_fields(c).join(", ")
                    ))
                    .color(self.hint_col())
                    .size(11.5),
                );
            }
            None => {
                ui.label(
                    egui::RichText::new(
                        "Pick a calculator first — close BATCH, pick one from the chip strip, then open BATCH again.",
                    )
                    .color(TEXT),
                );
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [130.0, 20.0],
                egui::Label::new(egui::RichText::new("INPUT CSV").color(TEXT).size(12.0)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.batch_in)
                    .hint_text("e.g. C:\\Users\\you\\rates.csv")
                    .desired_width(380.0),
            )
            .on_hover_text("Type the full path of the input CSV file");
        });
        ui.horizontal(|ui| {
            ui.add_sized(
                [130.0, 20.0],
                egui::Label::new(egui::RichText::new("OUTPUT CSV").color(TEXT).size(12.0)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.batch_out)
                    .hint_text("e.g. C:\\Users\\you\\results.csv")
                    .desired_width(380.0),
            )
            .on_hover_text("Type the full path the output CSV should be written to");
        });
        ui.horizontal(|ui| {
            if tip_sel(
                ui,
                self.batch_force,
                egui::RichText::new("OVERWRITE EXISTING OUTPUT").size(12.0),
                "Allow RUN BATCH to replace an existing output file — the old file is backed up to <name>.bak first. Off = the batch refuses and nothing is overwritten",
            )
            .clicked()
            {
                self.batch_force = !self.batch_force;
            }
        });
        ui.add_space(8.0);
        let mut run = false;
        if tip_btn_sized(
            ui,
            egui::vec2(180.0, 36.0),
            egui::RichText::new("RUN BATCH").color(GREEN).strong(),
            "Run the batch and write the output CSV",
        )
        .clicked()
        {
            run = true;
        }
        if run {
            self.do_batch();
        }
        if !self.batch_summary.is_empty() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(&self.batch_summary)
                    .color(TEXT)
                    .size(13.0),
            );
        }
    }

    /// The hint/muted label colour in force: brightened while
    /// classroom projection mode (#21) is on.
    fn hint_col(&self) -> egui::Color32 {
        if self.settings.projection {
            HINT_PROJ
        } else {
            HINT
        }
    }

    fn muted_col(&self) -> egui::Color32 {
        if self.settings.projection {
            MUTED_PROJ
        } else {
            MUTED
        }
    }

    /// The display format in force for the current result: the
    /// per-result override (#2) when one is chosen, else the
    /// global header setting.
    fn effective_format(&self) -> DisplayFormat {
        self.override_format.unwrap_or(self.settings.display)
    }

    /// The raw strings currently in the form (Auto mode's input
    /// evidence for #37).
    fn raw_input_strings(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .inputs
            .iter()
            .filter(|x| !x.trim().is_empty())
            .cloned()
            .collect();
        v.extend(
            self.date_texts
                .iter()
                .filter(|x| !x.trim().is_empty())
                .cloned(),
        );
        if !self.list_input.trim().is_empty() {
            v.push(self.list_input.clone());
        }
        v
    }

    /// Start an update check on a thread (the verdict lands in
    /// help_msg). Used by the HELP panel button; the startup check
    /// uses the same path from App::new.
    fn start_update_check(&mut self) {
        if self.update_rx.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(check_for_updates());
        });
        self.update_rx = Some(rx);
        self.help_msg = "Checking for updates…".to_string();
    }

    /// The rows under the display (v0.4.0): per-result VIEW
    /// override (#2), the ROUNDING setting (#9), COPY buttons
    /// (#4) and SPEC CHECK (#10). Shown only with a result up.
    fn result_tools_ui(&mut self, ui: &mut egui::Ui) {
        // VIEW override — re-renders THIS result only.
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("VIEW:").color(self.hint_col()).size(11.5));
            if tip_sel(
                ui,
                self.override_format.is_none(),
                egui::RichText::new("Global").size(12.0),
                "Use the header FORMAT setting for this result",
            )
            .clicked()
            {
                self.override_format = None;
            }
            for (mode, label, tip) in [
                (DisplayMode::SigFigs, "Sig figs", "Re-show this result's headline to a number of significant figures (does not change the global setting)"),
                (DisplayMode::Decimals, "Decimals", "Re-show this result's headline to a number of decimal places (does not change the global setting)"),
                (DisplayMode::Engineering, "Engineer", "Re-show this result's headline in engineering notation with SI prefixes"),
                (DisplayMode::Auto, "Auto", "Re-show this result's headline at the precision inferred from the numbers you typed"),
                (DisplayMode::Exact, "Exact", "Show this result's exact form where one exists (fractions labelled exact, pi geometry as 25*pi, rational expressions exact)"),
            ] {
                let active = self
                    .override_format
                    .map(|f| f.mode == mode)
                    .unwrap_or(false);
                if tip_sel(ui, active, egui::RichText::new(label).size(12.0), tip).clicked() {
                    self.override_format = Some(DisplayFormat {
                        mode,
                        n: Some(self.override_n as u8),
                        rounding: self.settings.display.rounding,
                    });
                }
            }
            if let Some(f) = self.override_format {
                if matches!(f.mode, DisplayMode::SigFigs | DisplayMode::Decimals) {
                    let combo = egui::ComboBox::from_id_salt("override-n")
                        .selected_text(format!("{}", self.override_n))
                        .width(52.0)
                        .show_ui(ui, |ui| {
                            let range: &[usize] = if f.mode == DisplayMode::SigFigs {
                                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                            } else {
                                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                            };
                            for v in range {
                                tip_sel_value(
                                    ui,
                                    &mut self.override_n,
                                    *v,
                                    &format!("{v}"),
                                    "How many figures this result's rounded headline shows",
                                );
                            }
                        });
                    combo
                        .response
                        .on_hover_text("Choose the figure count for this result's rounded headline");
                    if let Some(of) = &mut self.override_format {
                        of.n = Some(self.override_n as u8);
                    }
                }
            }
        });
        // ROUNDING setting (#9) — global, persisted.
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("ROUNDING:")
                    .color(self.hint_col())
                    .size(11.5),
            );
            for (mode, tip) in [
                (
                    RoundingMode::HalfUp,
                    "Halves round away from zero: 2.5 -> 3, -2.5 -> -3 (school rounding)",
                ),
                (
                    RoundingMode::HalfEven,
                    "Banker's rounding: halves round to the even neighbour: 2.5 -> 2, 3.5 -> 4",
                ),
                (
                    RoundingMode::Truncate,
                    "Truncate toward zero: 2.9 -> 2, -2.9 -> -2",
                ),
            ] {
                if tip_sel(
                    ui,
                    self.settings.display.rounding == mode,
                    egui::RichText::new(mode.label()).size(12.0),
                    tip,
                )
                .clicked()
                {
                    self.settings.display.rounding = mode;
                    if let Some(of) = &mut self.override_format {
                        of.rounding = mode;
                    }
                    self.persist_settings();
                }
            }
        });
        // COPY buttons (#4).
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("COPY:")
                    .color(self.hint_col())
                    .size(11.5),
            );
            for (fmt, label, tip) in [
                (
                    CopyFormat::Plain,
                    "Plain",
                    "Copy the result exactly as shown",
                ),
                (
                    CopyFormat::Latex,
                    "LaTeX",
                    "Copy the result as LaTeX: text escaped, numbers wrapped in $...$",
                ),
                (
                    CopyFormat::Scientific,
                    "Scientific",
                    "Copy the result with every number in scientific (e) notation",
                ),
                (
                    CopyFormat::Csv,
                    "CSV",
                    "Copy the result as CSV rows: label,value for each result line",
                ),
            ] {
                if tip_btn(ui, egui::RichText::new(label).size(12.0), tip).clicked() {
                    let text = format_copy(&self.output, fmt);
                    ui.ctx().copy_text(text);
                    self.copy_msg = format!("Copied as {label}.");
                }
            }
            if !self.copy_msg.is_empty() {
                ui.label(
                    egui::RichText::new(&self.copy_msg)
                        .color(self.hint_col())
                        .size(11.5),
                );
            }
        });
        // SPEC CHECK (#10): compare this result against user limits.
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("SPEC CHECK:")
                    .color(self.hint_col())
                    .size(11.5),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.spec_min)
                    .hint_text("min")
                    .desired_width(80.0),
            )
            .on_hover_text("The smallest acceptable value (a result exactly on the limit PASSes)");
            ui.add(
                egui::TextEdit::singleline(&mut self.spec_max)
                    .hint_text("max")
                    .desired_width(80.0),
            )
            .on_hover_text("The largest acceptable value (a result exactly on the limit PASSes)");
            if tip_btn(
                ui,
                egui::RichText::new("CHECK SPEC").color(GREEN),
                "Compare every numeric result line against the min/max limits",
            )
            .clicked()
            {
                let mn = self.spec_min.trim().parse::<f64>();
                let mx = self.spec_max.trim().parse::<f64>();
                match (mn, mx) {
                    (Ok(mn), Ok(mx)) if mn <= mx => {
                        self.spec_lines = spec_verdict_lines(&self.output, mn, mx);
                        if self.spec_lines.is_empty() {
                            self.spec_lines = vec![
                                "No numeric result lines to check — run a calculation first."
                                    .to_string(),
                            ];
                        }
                    }
                    _ => {
                        self.spec_lines =
                            vec!["SPEC CHECK needs a min and a max number, with min <= max."
                                .to_string()];
                    }
                }
            }
        });
        for line in &self.spec_lines {
            let (mark, colour) = if line.starts_with("PASS") {
                ("\u{2713} ", GREEN)
            } else if line.starts_with("FAIL") {
                ("\u{2717} ", RED)
            } else {
                ("", TEXT)
            };
            ui.label(
                egui::RichText::new(format!("{mark}{line}"))
                    .color(colour)
                    .size(12.5),
            );
        }
    }

    /// The current in-memory set as a ProblemSet value.
    fn current_set(&self) -> ProblemSet {
        ProblemSet {
            name: if self.set_name.trim().is_empty() {
                "Untitled set".to_string()
            } else {
                self.set_name.trim().to_string()
            },
            seed_note: self.set_seed_note,
            entries: self.set_entries.clone(),
        }
    }

    /// The display tokens of the form's current values (for ADD
    /// CURRENT CALCULATOR and SAVE CURRENT AS EXAMPLE): the values
    /// are parsed exactly as CALCULATE parses them, so what is
    /// saved is what would run.
    fn current_form_tokens(&mut self) -> Result<Vec<String>, String> {
        let Some(idx) = self.selected else {
            return Err("Pick a calculator first (mode rail + chip strip).".into());
        };
        if self.calcs[idx].text_run.is_some() {
            let t = self.list_input.trim().to_string();
            if t.is_empty() {
                return Err("Type an expression in the form first.".into());
            }
            return Ok(vec![t]);
        }
        let nums = self.parse_inputs()?;
        Ok(display_tokens(&self.calcs[idx], &nums))
    }

    /// SETS view (v0.6.0, items #22/#23/#26/#27/#28/#29/#30):
    /// build a problem set from the calculators, save/open it as
    /// the shared set file, export its worksheet and answer key,
    /// generate a class set from a seed, mark a class CSV, write
    /// formula sheets, and manage the saved class examples
    /// library. Every action calls the same teachers.rs engines
    /// as the `set ...` / `--mark` / `--formula-sheet` commands.
    fn sets_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("PROBLEM SETS")
                .color(GREEN)
                .strong()
                .size(16.0),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "A problem set is an ordered list of problems — a calculator plus concrete inputs — kept in one plain-text file you can read and edit by hand. The set file holds problems only, never answers, so the same file is the student assignment. The answer key and worksheet are separate exports.",
            )
            .color(TEXT),
        );

        // ---- Build the set (#22) ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("BUILD THE SET")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("SET NAME").color(TEXT).size(12.0));
            ui.add(egui::TextEdit::singleline(&mut self.set_name).desired_width(320.0))
                .on_hover_text(
                    "The set's name — printed on the worksheet, the answer key and the set file",
                );
        });
        if let Some(seed) = self.set_seed_note {
            ui.label(
                egui::RichText::new(format!(
                    "Made from seed {seed} — the same seed always makes this set."
                ))
                .color(self.hint_col())
                .size(11.5),
            );
        }
        let mut add_current = false;
        ui.horizontal_wrapped(|ui| {
            if tip_btn(
                ui,
                egui::RichText::new("ADD CURRENT CALCULATOR").color(GREEN),
                "Add the calculator open in the form, with the values currently in its fields, as the next problem in this set",
            )
            .clicked()
            {
                add_current = true;
            }
            ui.label(
                egui::RichText::new("Fill a calculator's fields in the form first, then add it here.")
                    .color(self.hint_col())
                    .size(11.5),
            );
        });
        if add_current {
            match self.selected {
                Some(idx) if self.calcs[idx].text_run.is_some() => {
                    self.set_msg =
                        "Problem sets hold numeric calculators — the Expression Calculator takes text, so it cannot be a set problem."
                            .to_string();
                }
                Some(idx) => {
                    let id = self.calcs[idx].id.to_string();
                    match self.current_form_tokens() {
                        Ok(toks) => match set_make_entry(&id, &toks) {
                            Ok(entry) => {
                                self.set_entries.push(entry);
                                self.set_seed_note = None;
                                self.set_msg = format!(
                                    "Added problem {} — {}",
                                    self.set_entries.len(),
                                    self.calcs[idx].name
                                );
                            }
                            Err(e) => self.set_msg = e,
                        },
                        Err(e) => self.set_msg = e,
                    }
                }
                None => {
                    self.set_msg = "Pick a calculator first (mode rail + chip strip).".to_string();
                }
            }
        }
        if self.set_entries.is_empty() {
            ui.label(
                egui::RichText::new("The set is empty — add the current calculator, generate a set from a seed below, or open a set file.")
                    .color(TEXT),
            );
        } else {
            let mut remove: Option<usize> = None;
            let mut load: Option<usize> = None;
            for (i, entry) in self.set_entries.clone().iter().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    let line = match self.calcs.iter().find(|c| c.id == entry.calc_id) {
                        Some(c) => problem_line(c, &entry.inputs),
                        None => format!("{} ({})", entry.calc_id, entry.inputs.join(", ")),
                    };
                    ui.label(
                        egui::RichText::new(format!("{}. {line}", i + 1))
                            .color(TEXT)
                            .size(12.5),
                    );
                    if tip_btn(
                        ui,
                        egui::RichText::new("LOAD"),
                        "Open this problem's calculator with its inputs filled in, so it can be worked (or edited and re-added)",
                    )
                    .clicked()
                    {
                        load = Some(i);
                    }
                    if tip_btn(
                        ui,
                        egui::RichText::new("REMOVE").color(RED),
                        "Take this problem out of the set",
                    )
                    .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                self.set_entries.remove(i);
                self.set_seed_note = None;
                self.set_msg = format!("Removed problem {}.", i + 1);
            }
            if let Some(i) = load {
                let entry = self.set_entries[i].clone();
                if let Some(idx) = self.calcs.iter().position(|c| c.id == entry.calc_id) {
                    self.select(idx);
                    self.fill_from_tokens(&entry.inputs);
                    self.show_sets = false;
                    self.set_msg = format!("Problem {} loaded into the form.", i + 1);
                }
            }
        }

        // ---- Save / open the set file (#22/#30) ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("SET FILE")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("SET FILE").color(TEXT).size(12.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.set_path)
                    .hint_text("e.g. C:\\Users\\you\\week4-set.txt")
                    .desired_width(380.0),
            )
            .on_hover_text("The file the set is saved to / opened from (plain text; paths are typed — this toolkit has no file-picker dialog)");
        });
        let mut do_save_set = false;
        let mut do_open_set = false;
        ui.horizontal_wrapped(|ui| {
            if tip_btn(
                ui,
                egui::RichText::new("SAVE SET").color(GREEN),
                "Write the current set to the file named above (an existing file is backed up to <name>.bak first)",
            )
            .clicked()
            {
                do_save_set = true;
            }
            if tip_btn(
                ui,
                egui::RichText::new("OPEN SET"),
                "Load a set file into this view — its problems replace the current list, ready to work, export or mark",
            )
            .clicked()
            {
                do_open_set = true;
            }
        });
        if do_save_set {
            let path = self.set_path.trim().to_string();
            if path.is_empty() {
                self.set_msg = "Type the set file's path first.".to_string();
            } else if self.set_entries.is_empty() {
                self.set_msg = "The set is empty — add a problem first.".to_string();
            } else {
                let set = self.current_set();
                self.set_msg =
                    match write_with_backup(std::path::Path::new(&path), &set_to_text(&set)) {
                        Ok(note) => format!(
                            "Set saved to {path} — {} problem(s). {note}",
                            set.entries.len()
                        ),
                        Err(e) => e,
                    };
            }
        }
        if do_open_set {
            let path = self.set_path.trim().to_string();
            if path.is_empty() {
                self.set_msg = "Type the set file's path first.".to_string();
            } else {
                match read_set(std::path::Path::new(&path)) {
                    Ok(set) => {
                        self.set_name = set.name.clone();
                        self.set_seed_note = set.seed_note;
                        self.set_entries = set.entries.clone();
                        self.set_msg = format!("Opened {path} — {} problem(s).", set.entries.len());
                    }
                    Err(e) => self.set_msg = e,
                }
            }
        }

        // ---- Exports: worksheet + answer key (#27/#22) ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("WORKSHEET + ANSWER KEY")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.label(
            egui::RichText::new(
                "The worksheet is the student copy: numbered problems, working space, no answers — print it from any text editor (CalcDex has no print dialog). The answer key is the teacher copy: the same numbering, with each answer and a one-line working.",
            )
            .color(TEXT),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("EXPORT TO").color(TEXT).size(12.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.set_out_path)
                    .hint_text("e.g. C:\\Users\\you\\week4-worksheet.txt")
                    .desired_width(380.0),
            )
            .on_hover_text("The file the worksheet or answer key is written to (an existing file is backed up to <name>.bak first)");
        });
        let mut do_worksheet = false;
        let mut do_key = false;
        ui.horizontal_wrapped(|ui| {
            if tip_btn(
                ui,
                egui::RichText::new("EXPORT WORKSHEET").color(GREEN),
                "Write the student worksheet — problems with working space, no answers",
            )
            .clicked()
            {
                do_worksheet = true;
            }
            if tip_btn(
                ui,
                egui::RichText::new("EXPORT ANSWER KEY").color(GREEN),
                "Write the teacher answer key — the same numbering, with answers and a one-line working for each problem",
            )
            .clicked()
            {
                do_key = true;
            }
        });
        if do_worksheet || do_key {
            let path = self.set_out_path.trim().to_string();
            if path.is_empty() {
                self.set_msg = "Type the export file's path first.".to_string();
            } else if self.set_entries.is_empty() {
                self.set_msg = "The set is empty — add a problem first.".to_string();
            } else {
                let set = self.current_set();
                let (text, what) = if do_worksheet {
                    (worksheet_text(&set), "Worksheet")
                } else {
                    (answer_key_text(&set), "Answer key")
                };
                self.set_msg = match write_with_backup(std::path::Path::new(&path), &text) {
                    Ok(note) => format!("{what} written to {path}. {note}"),
                    Err(e) => e,
                };
            }
        }

        // ---- Generate a class set from a seed (#23) ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("GENERATE FROM A SEED")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.label(
            egui::RichText::new(
                "Make a whole set at once from the practice generator. The same seed, calculators and count always make the identical set — give the class the seed (or the saved set file) and everyone gets the same problems.",
            )
            .color(TEXT),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("SEED").color(TEXT).size(12.0));
            ui.add(egui::TextEdit::singleline(&mut self.gen_seed).desired_width(90.0))
                .on_hover_text("The seed the problems are made from — a whole number");
            ui.label(egui::RichText::new("COUNT").color(TEXT).size(12.0));
            ui.add(egui::TextEdit::singleline(&mut self.gen_count).desired_width(60.0))
                .on_hover_text("How many problems to make");
            ui.label(egui::RichText::new("CALCULATORS").color(TEXT).size(12.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.gen_ids)
                    .hint_text("ids, comma separated — e.g. circle, percent-of")
                    .desired_width(300.0),
            )
            .on_hover_text("The calculator ids to make problems from, separated by commas (ids are shown under each calculator's name in the form)");
        });
        if tip_btn(
            ui,
            egui::RichText::new("GENERATE SET").color(GREEN),
            "Replace the current set with problems generated from the seed — the same seed always makes the identical set",
        )
        .clicked()
        {
            let seed = self.gen_seed.trim().parse::<u64>();
            let count = self.gen_count.trim().parse::<usize>();
            let ids: Vec<String> = self
                .gen_ids
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            match (seed, count) {
                (Ok(seed), Ok(count)) => {
                    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
                    let name = if self.set_name.trim().is_empty()
                        || self.set_name == "Untitled set"
                    {
                        format!("Problem set from seed {seed}")
                    } else {
                        self.set_name.trim().to_string()
                    };
                    match generate_set(&id_refs, seed, count, &name) {
                        Ok(set) => {
                            self.set_name = set.name.clone();
                            self.set_seed_note = set.seed_note;
                            self.set_entries = set.entries.clone();
                            self.set_msg = format!(
                                "Generated {} problem(s) from seed {seed}.",
                                set.entries.len()
                            );
                        }
                        Err(e) => self.set_msg = e,
                    }
                }
                _ => {
                    self.set_msg =
                        "GENERATE needs a whole-number seed and a count of 1 or more.".to_string();
                }
            }
        }

        // ---- Mark a class CSV (#26) ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("MARK A CLASS")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.label(
            egui::RichText::new(
                "The answers CSV holds one row per student: their name, then their answers in problem order. Marking uses the Check My Answer rules — fraction answers exactly, everything else within 0.5%. The marked CSV adds a right/wrong column per problem and a score; the per-student totals appear below.",
            )
            .color(TEXT),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("ANSWERS CSV").color(TEXT).size(12.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.mark_answers)
                    .hint_text("e.g. C:\\Users\\you\\class-answers.csv")
                    .desired_width(340.0),
            )
            .on_hover_text("The CSV of student answers: one row per student — name, then their answers in problem order");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("MARKED CSV").color(TEXT).size(12.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.mark_out)
                    .hint_text("blank = the answers name + .marked.csv")
                    .desired_width(340.0),
            )
            .on_hover_text("Where the marked CSV is written — leave blank to use the answers file's name with .marked.csv added");
        });
        if tip_btn(
            ui,
            egui::RichText::new("RUN MARKING").color(GREEN),
            "Mark the answers CSV against the current set and write the marked CSV",
        )
        .clicked()
        {
            let set = self.current_set();
            let answers_path = self.mark_answers.trim().to_string();
            if answers_path.is_empty() {
                self.mark_msg = "Type the answers CSV path first.".to_string();
            } else if set.entries.is_empty() {
                self.mark_msg =
                    "The current set is empty — marking needs the set the class answered."
                        .to_string();
            } else {
                match std::fs::read_to_string(&answers_path) {
                    Ok(text) => match mark_set(&set, &text) {
                        Ok(outcome) => {
                            let out_path = if self.mark_out.trim().is_empty() {
                                marked_path_for(std::path::Path::new(&answers_path))
                                    .display()
                                    .to_string()
                            } else {
                                self.mark_out.trim().to_string()
                            };
                            match write_with_backup(
                                std::path::Path::new(&out_path),
                                &outcome.output_csv,
                            ) {
                                Ok(note) => {
                                    self.mark_msg = format!(
                                        "Marked {} student(s) — written to {out_path}. {note}\n{}",
                                        outcome.rows.len(),
                                        outcome.summary.join("\n")
                                    );
                                }
                                Err(e) => self.mark_msg = e,
                            }
                        }
                        Err(e) => self.mark_msg = e,
                    },
                    Err(e) => {
                        self.mark_msg = format!("Could not read {answers_path}: {e}");
                    }
                }
            }
        }
        if !self.mark_msg.is_empty() {
            ui.label(egui::RichText::new(&self.mark_msg).color(TEXT).size(12.5));
        }

        // ---- Formula sheet (#29) ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("FORMULA SHEET")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.label(
            egui::RichText::new(
                "A printable reference for one category (or all of them): every calculator's formula and what every symbol means.",
            )
            .color(TEXT),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("CATEGORY").color(TEXT).size(12.0));
            let mut cats: Vec<String> = vec!["All".to_string()];
            for c in &self.calcs {
                if !cats.contains(&c.category.to_string()) {
                    cats.push(c.category.to_string());
                }
            }
            let combo = egui::ComboBox::from_id_salt("sheet-category")
                .selected_text(self.sheet_category.clone())
                .width(160.0)
                .show_ui(ui, |ui| {
                    for cat in &cats {
                        if tip_sel(
                            ui,
                            self.sheet_category == *cat,
                            egui::RichText::new(cat),
                            "Write the formula sheet for this category (All = every category)",
                        )
                        .clicked()
                        {
                            self.sheet_category = cat.clone();
                        }
                    }
                });
            combo
                .response
                .on_hover_text("Which category the formula sheet covers");
            ui.add(
                egui::TextEdit::singleline(&mut self.sheet_path)
                    .hint_text("e.g. C:\\Users\\you\\math-formulas.txt")
                    .desired_width(300.0),
            )
            .on_hover_text("The file the formula sheet is written to (an existing file is backed up to <name>.bak first)");
        });
        if tip_btn(
            ui,
            egui::RichText::new("WRITE FORMULA SHEET").color(GREEN),
            "Write the formula sheet for the chosen category to the file named above",
        )
        .clicked()
        {
            let path = self.sheet_path.trim().to_string();
            if path.is_empty() {
                self.set_msg = "Type the formula sheet's path first.".to_string();
            } else {
                let cat = if self.sheet_category == "All" {
                    None
                } else {
                    Some(self.sheet_category.as_str())
                };
                match formula_sheet_text(&self.calcs, cat) {
                    Ok(text) => {
                        self.set_msg = match write_with_backup(std::path::Path::new(&path), &text) {
                            Ok(note) => format!("Formula sheet written to {path}. {note}"),
                            Err(e) => e,
                        };
                    }
                    Err(e) => self.set_msg = e,
                }
            }
        }

        // ---- Saved class examples library (#28) ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("SAVED CLASS EXAMPLES")
                .color(GREEN)
                .strong()
                .size(12.0),
        );
        ui.label(
            egui::RichText::new(
                "Your own worked examples, saved with a title and a note in the examples library (kept in your CalcDex folder, shared with the command line). Load one to open its calculator with the inputs filled in.",
            )
            .color(TEXT),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("TITLE").color(TEXT).size(12.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.ex_title)
                    .hint_text("e.g. Circle demo for period 3")
                    .desired_width(280.0),
            )
            .on_hover_text("The title the current calculator + its form values are saved under (saving the same title again replaces it)");
            ui.label(egui::RichText::new("NOTE").color(TEXT).size(12.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.ex_note).desired_width(240.0),
            )
            .on_hover_text("A short note about the example (optional)");
        });
        if tip_btn(
            ui,
            egui::RichText::new("SAVE CURRENT AS EXAMPLE").color(GREEN),
            "Save the calculator open in the form, with its current values, under the title above",
        )
        .clicked()
        {
            let title = self.ex_title.trim().to_string();
            if title.is_empty() {
                self.ex_msg = "Give the example a title first.".to_string();
            } else {
                match self.selected {
                    Some(idx) => {
                        let calc_id = self.calcs[idx].id.to_string();
                        match self.current_form_tokens() {
                            Ok(toks) => {
                                let ex = SavedExample {
                                    title: title.clone(),
                                    calc_id,
                                    note: self.ex_note.trim().to_string(),
                                    inputs: toks,
                                };
                                upsert_example(&mut self.examples, ex);
                                self.ex_msg = match calcdex::teachers::save_examples(&self.examples)
                                {
                                    Ok(()) => format!("Saved '{title}' to the examples library."),
                                    Err(e) => format!("Could not save the library: {e}"),
                                };
                            }
                            Err(e) => self.ex_msg = e,
                        }
                    }
                    None => {
                        self.ex_msg =
                            "Pick a calculator first (mode rail + chip strip).".to_string();
                    }
                }
            }
        }
        if !self.ex_msg.is_empty() {
            ui.label(egui::RichText::new(&self.ex_msg).color(TEXT).size(12.5));
        }
        if self.examples.is_empty() {
            ui.label(
                egui::RichText::new("No saved examples yet — fill a calculator's form, give it a title, and save it.")
                    .color(TEXT),
            );
        } else {
            let mut load_ex: Option<SavedExample> = None;
            let mut delete_ex: Option<String> = None;
            for ex in self.examples.clone() {
                ui.horizontal_wrapped(|ui| {
                    let calc_name = self
                        .calcs
                        .iter()
                        .find(|c| c.id == ex.calc_id)
                        .map(|c| c.name)
                        .unwrap_or("");
                    ui.label(
                        egui::RichText::new(format!("{} — {}", ex.title, calc_name))
                            .color(TEXT)
                            .size(12.5),
                    );
                    if !ex.note.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("({})", ex.note))
                                .color(self.hint_col())
                                .size(11.5),
                        );
                    }
                    if tip_btn(
                        ui,
                        egui::RichText::new("LOAD"),
                        "Open this example's calculator with its saved inputs filled in",
                    )
                    .clicked()
                    {
                        load_ex = Some(ex.clone());
                    }
                    let armed = self.ex_delete_armed.as_deref() == Some(ex.title.as_str());
                    if tip_btn(
                        ui,
                        egui::RichText::new(if armed { "CONFIRM DELETE" } else { "DELETE" })
                            .color(RED),
                        if armed {
                            "Press again to delete this example for good"
                        } else {
                            "Delete this example from the library (asks once more before deleting)"
                        },
                    )
                    .clicked()
                    {
                        if armed {
                            delete_ex = Some(ex.title.clone());
                        } else {
                            self.ex_delete_armed = Some(ex.title.clone());
                        }
                    }
                });
            }
            if let Some(ex) = load_ex {
                if let Some(idx) = self.calcs.iter().position(|c| c.id == ex.calc_id) {
                    self.select(idx);
                    self.fill_from_tokens(&ex.inputs);
                    self.show_sets = false;
                    self.ex_msg = format!("Loaded '{}'.", ex.title);
                }
            }
            if let Some(title) = delete_ex {
                self.examples.retain(|e| e.title != title);
                self.ex_delete_armed = None;
                self.ex_msg = match calcdex::teachers::save_examples(&self.examples) {
                    Ok(()) => format!("Deleted '{title}'."),
                    Err(e) => format!("Could not save the library: {e}"),
                };
            }
        }

        if !self.set_msg.is_empty() {
            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new(&self.set_msg).color(TEXT).size(12.5));
        }
    }

    /// CONSTANTS view (v0.7.0, item #38): the CODATA 2018
    /// library with INSERT into the Expression Calculator, plus
    /// The MATERIALS view (v0.8.0, item #44): the built-in table
    /// of typical engineering values — the same rows
    /// `CalcDex --materials` prints, with the same codes the beam
    /// calculator's material field takes. USE E IN BEAM opens
    /// the beam calculator with the row's Young's modulus filled
    /// in as a plain number (fully editable afterwards).
    fn materials_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("MATERIALS")
                .color(GREEN)
                .strong()
                .size(15.0),
        );
        ui.label(
            egui::RichText::new(
                "Typical published values for common engineering materials — real \
                 alloys and batches vary, so use the manufacturer's data for \
                 critical work. Resistivity is given for metals; insulators are \
                 marked as such instead of being given an invented number. The \
                 number at the left of each row is its code: the beam \
                 calculator takes it in its material field when its E field \
                 is 0.",
            )
            .color(TEXT),
        );
        ui.add_space(8.0);
        let mut use_beam: Option<f64> = None;
        for (i, mat) in MATERIALS.iter().enumerate() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("{}. {}", i + 1, mat.name))
                        .color(GREEN)
                        .strong()
                        .size(12.5),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "density {} kg/m^3 \u{b7} resistivity {} \u{b7} E {} GPa",
                        calcdex::n(mat.density),
                        resistivity_text(mat),
                        calcdex::n(mat.youngs)
                    ))
                    .color(TEXT)
                    .size(12.5),
                );
                if tip_btn(
                    ui,
                    egui::RichText::new("USE E IN BEAM"),
                    "Open the Beam calculator with this material's Young's modulus filled into its E field (the material code field is set to 0, so the number is what counts and stays editable)",
                )
                .clicked()
                {
                    use_beam = Some(mat.youngs);
                }
            });
        }
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "On the command line: CalcDex --materials lists this table, \
                 CalcDex --materials copper looks one material up.",
            )
            .color(self.hint_col())
            .size(11.5),
        );
        if let Some(e) = use_beam {
            self.use_material_in_beam(e);
        }
    }

    /// USE E IN BEAM from the MATERIALS view (#44): open the
    /// beam calculator and put the material's Young's modulus in
    /// its E field as a number (material code back to 0 = the
    /// number is used), so the run is identical to typing it.
    fn use_material_in_beam(&mut self, e_gpa: f64) {
        if let Some(idx) = self.calcs.iter().position(|c| c.id == "beam-bending") {
            self.select(idx);
            if self.inputs.len() > 3 {
                self.inputs[2] = calcdex::nd(e_gpa, 4);
                self.inputs[3] = "0".to_string();
            }
        }
        self.show_materials = false;
    }

    /// the user's own defined constants — defined, inserted and
    /// deleted here, stored in the shared settings file (the
    /// CLI's --constants / --define read and write the same list).
    fn constants_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("CONSTANTS")
                .color(GREEN)
                .strong()
                .size(15.0),
        );
        ui.label(
            egui::RichText::new(
                "Physical constants (CODATA 2018) for the Expression Calculator. \
                 INSERT puts a constant into the expression field; the same names \
                 also work typed — h*c/500e-9 is the energy of a 500 nm photon. \
                 A bare e stays Euler's number; the elementary charge is e_charge.",
            )
            .color(TEXT),
        );
        ui.add_space(8.0);
        let mut insert: Option<String> = None;
        for c in CONSTANTS {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(c.symbol)
                        .color(GREEN)
                        .strong()
                        .size(12.5),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} = {} [{}]",
                        c.name,
                        constant_value_str(c.value),
                        c.unit
                    ))
                    .color(TEXT)
                    .size(12.5),
                );
                if tip_btn(
                    ui,
                    egui::RichText::new("INSERT"),
                    "Put this constant into the Expression Calculator's field",
                )
                .clicked()
                {
                    insert = Some(c.symbol.to_string());
                }
            });
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Also built in: pi, e (Euler's number), tau. In an expression, \
                 k_b, n_a, eps0, mu0, m_e, m_p and the other aliases work too — \
                 CalcDex --constants prints every accepted name.",
            )
            .color(self.hint_col())
            .size(11.5),
        );
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("YOUR CONSTANTS")
                .color(GREEN)
                .strong()
                .size(13.0),
        );
        ui.label(
            egui::RichText::new(
                "Your own constants live in the shared settings file, so the \
                 window and the command line use the same ones. A defined name \
                 can never replace a built-in constant or a function name.",
            )
            .color(TEXT),
        );
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Name:").color(TEXT).size(12.5));
            ui.add(egui::TextEdit::singleline(&mut self.const_name).desired_width(150.0))
                .on_hover_text(
                    "The constant's name — letters, digits and underscores, starting with a letter (e.g. my_g). Built-in and function names are refused",
                );
            ui.label(egui::RichText::new("Value:").color(TEXT).size(12.5));
            ui.add(egui::TextEdit::singleline(&mut self.const_value).desired_width(150.0))
                .on_hover_text(
                    "The constant's value — a plain number; scientific notation works (e.g. 1.5e-3)",
                );
            if tip_btn(
                ui,
                egui::RichText::new("DEFINE CONSTANT").color(GREEN),
                "Save this constant in the settings file so expressions can use it by name, in the window and on the command line",
            )
            .clicked()
            {
                match validate_constant_name(&self.const_name) {
                    Ok(name) => match self.const_value.trim().parse::<f64>() {
                        Ok(v) if v.is_finite() => {
                            self.settings.constants.retain(|(n, _)| *n != name);
                            self.settings.constants.push((name.clone(), v));
                            self.persist_settings();
                            set_custom_constants(self.settings.constants.clone());
                            self.const_msg = format!(
                                "Constant {name} = {} saved — use it in the Expression Calculator.",
                                calcdex::n(v)
                            );
                            self.const_name.clear();
                            self.const_value.clear();
                        }
                        _ => {
                            self.const_msg = format!(
                                "'{}' is not a number — a constant's value is a plain number (scientific notation works).",
                                self.const_value.trim()
                            );
                        }
                    },
                    Err(e) => {
                        self.const_msg = e;
                    }
                }
            }
        });
        if !self.const_msg.is_empty() {
            ui.label(egui::RichText::new(&self.const_msg).color(TEXT).size(12.5));
        }
        ui.add_space(4.0);
        if self.settings.constants.is_empty() {
            ui.label(
                egui::RichText::new(
                    "No constants defined yet — fill in a name and a value above, or use CalcDex --define name=value.",
                )
                .color(TEXT),
            );
        } else {
            let mut delete: Option<String> = None;
            for (name, value) in self.settings.constants.clone() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(&name)
                            .color(GREEN)
                            .strong()
                            .size(12.5),
                    );
                    ui.label(
                        egui::RichText::new(format!("= {}", calcdex::n(value)))
                            .color(TEXT)
                            .size(12.5),
                    );
                    if tip_btn(
                        ui,
                        egui::RichText::new("INSERT"),
                        "Put this constant into the Expression Calculator's field",
                    )
                    .clicked()
                    {
                        insert = Some(name.clone());
                    }
                    let armed = self.const_delete_armed.as_deref() == Some(name.as_str());
                    if tip_btn(
                        ui,
                        egui::RichText::new(if armed { "CONFIRM DELETE" } else { "DELETE" })
                            .color(RED),
                        if armed {
                            "Press again to delete this constant for good"
                        } else {
                            "Delete this constant from the settings file (asks once more before deleting)"
                        },
                    )
                    .clicked()
                    {
                        if armed {
                            delete = Some(name.clone());
                        } else {
                            self.const_delete_armed = Some(name.clone());
                        }
                    }
                });
            }
            if let Some(name) = delete {
                self.settings.constants.retain(|(n, _)| *n != name);
                self.const_delete_armed = None;
                self.persist_settings();
                set_custom_constants(self.settings.constants.clone());
                self.const_msg = format!("Constant {name} deleted.");
            }
        }
        if let Some(symbol) = insert {
            self.insert_constant(&symbol);
        }
    }

    /// INSERT from the CONSTANTS view (#38): the target is the
    /// Expression Calculator's field — open that calculator if
    /// another one is current, then append the symbol.
    fn insert_constant(&mut self, symbol: &str) {
        let on_expr = self.selected.map(|i| self.calcs[i].id) == Some("expression-calculator");
        if !on_expr {
            if let Some(idx) = self
                .calcs
                .iter()
                .position(|c| c.id == "expression-calculator")
            {
                self.select(idx);
            }
        }
        let need_space = self
            .list_input
            .chars()
            .last()
            .map(|c| c.is_alphanumeric() || c == ')' || c == '_')
            .unwrap_or(false);
        if need_space {
            self.list_input.push(' ');
        }
        self.list_input.push_str(symbol);
        self.show_constants = false;
        self.show_materials = false;
    }

    /// Built-in HELP panel (standing rule): the compiled-in guide
    /// (same text as --help-guide), plus the offer-only update
    /// check and its startup toggle.
    fn help_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("HELP").color(GREEN).strong().size(16.0));
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("help-scroll")
            .show(ui, |ui| {
                ui.label(egui::RichText::new(HELP_TEXT).color(TEXT).size(12.5));
                ui.add_space(10.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("UPDATES")
                        .color(GREEN)
                        .strong()
                        .size(13.0),
                );
                ui.label(
                    egui::RichText::new(
                        "CalcDex never downloads or installs anything by itself. The button asks github.com for the newest release and reports it here; the startup check is off unless you switch it on.",
                    )
                    .color(TEXT),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if tip_btn(
                        ui,
                        egui::RichText::new("CHECK FOR UPDATES").color(GREEN),
                        "Ask github.com for the newest CalcDex release (sends no personal data; downloads nothing)",
                    )
                    .clicked()
                    {
                        self.start_update_check();
                    }
                    if tip_sel(
                        ui,
                        self.settings.update_check_startup,
                        egui::RichText::new("CHECK AT STARTUP: ON/OFF").size(12.0),
                        "When ON, CalcDex asks github.com for the newest release each time it starts and shows the verdict here. Default OFF — otherwise CalcDex makes no network connection at all",
                    )
                    .clicked()
                    {
                        self.settings.update_check_startup =
                            !self.settings.update_check_startup;
                        self.persist_settings();
                    }
                });
                if !self.help_msg.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(&self.help_msg)
                            .color(TEXT)
                            .size(12.5),
                    );
                }
            });
    }
}
