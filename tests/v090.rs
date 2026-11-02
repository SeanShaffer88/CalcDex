//! v0.9.0 conformance batch — portable data folder, settings
//! schema versioning (the v0 -> v1 migration fixture), atomic
//! saves, the purge planner behind --clean/--uninstall, window
//! geometry persistence, and the stats median error fix.
//! Everything lives in the library; the GUI/CLI are thin callers.

use calcdex::paths::{atomic_write, data_file, purge_candidates, KNOWN_FILES};
use calcdex::settings::{
    declared_schema_version, load_settings_report_from, parse_settings, save_settings_to,
    serialize_settings, Settings, SettingsLoadNote, SETTINGS_SCHEMA_VERSION,
};
use calcdex::{all_calcs, run_calc};
use std::collections::BTreeSet;

fn valid_ids() -> BTreeSet<String> {
    all_calcs().iter().map(|c| c.id.to_string()).collect()
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("calcdex-v090-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

// ---------- schema versioning ----------

#[test]
fn schema_v0_file_migrates_in_memory() {
    // A pre-v0.9.0 settings file: no schema_version line, and the
    // legacy v0.3.0 sig_figs= line that must keep loading.
    let dir = temp_dir("v0");
    let path = dir.join("settings.txt");
    std::fs::write(
        &path,
        "# CalcDex settings\nsig_figs=3\nfavourite=bmi\nupdate_check_startup=on\n",
    )
    .expect("write v0 fixture");
    assert_eq!(
        declared_schema_version(&std::fs::read_to_string(&path).unwrap()),
        0
    );
    let (settings, note) = load_settings_report_from(&path, &valid_ids());
    assert_eq!(note, SettingsLoadNote::MigratedFrom(0));
    // The legacy line still applies (the informal compat is kept).
    assert_eq!(settings.display.mode, calcdex::rigour::DisplayMode::SigFigs);
    assert_eq!(settings.display.n, Some(3));
    assert!(settings.favourites.contains("bmi"));
    assert!(settings.update_check_startup);
    // And the next save stamps the current version.
    save_settings_to(&path, &settings).expect("save migrated settings");
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(declared_schema_version(&text), SETTINGS_SCHEMA_VERSION);
    let (_, note) = load_settings_report_from(&path, &valid_ids());
    assert_eq!(note, SettingsLoadNote::Current);
}

#[test]
fn schema_future_file_loads_safe_and_is_never_overwritten() {
    let dir = temp_dir("future");
    let path = dir.join("settings.txt");
    let future_text = "schema_version=99\nfavourite=bmi\n";
    std::fs::write(&path, future_text).expect("write future fixture");
    let (settings, note) = load_settings_report_from(&path, &valid_ids());
    assert_eq!(note, SettingsLoadNote::FutureVersion(99));
    // Defaults, not the future file's contents.
    assert_eq!(settings, Settings::default());
    // Saving over it is refused, and the file is byte-identical.
    assert!(save_settings_to(&path, &Settings::default()).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), future_text);
}

#[test]
fn schema_version_line_round_trips() {
    let text = serialize_settings(&Settings::default());
    assert!(text.contains(&format!("schema_version={SETTINGS_SCHEMA_VERSION}")));
    assert_eq!(declared_schema_version(&text), SETTINGS_SCHEMA_VERSION);
    assert_eq!(declared_schema_version("favourite=bmi\n"), 0);
    assert_eq!(declared_schema_version("schema_version=abc\n"), 0);
}

// ---------- window geometry ----------

#[test]
fn window_geometry_round_trips_and_validates() {
    let s = Settings {
        window: Some((100.0, 200.0, 1180.0, 760.0)),
        ..Default::default()
    };
    let text = serialize_settings(&s);
    let back = parse_settings(&text, &valid_ids());
    assert_eq!(back.window, Some((100.0, 200.0, 1180.0, 760.0)));
    // Implausible values are dropped, never restored.
    let bad = parse_settings("window=-99999,0,10,10\n", &valid_ids());
    assert_eq!(bad.window, None);
    let bad2 = parse_settings("window=0,0,1180\n", &valid_ids());
    assert_eq!(bad2.window, None);
}

// ---------- portable data + purge planner + atomic writes ----------

#[test]
fn purge_candidates_lists_only_existing_known_files() {
    let dir = temp_dir("purge");
    std::fs::write(dir.join("settings.txt"), "x").unwrap();
    std::fs::write(dir.join("history.txt"), "x").unwrap();
    std::fs::write(dir.join("unrelated.txt"), "x").unwrap();
    let plan = purge_candidates(&dir);
    let names: Vec<String> = plan
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["settings.txt", "history.txt"]);
    // Planning touched nothing.
    assert!(dir.join("settings.txt").exists());
    assert!(dir.join("unrelated.txt").exists());
    // Every known file is a purge candidate when present.
    for name in KNOWN_FILES {
        std::fs::write(dir.join(name), "x").unwrap();
    }
    assert_eq!(purge_candidates(&dir).len(), KNOWN_FILES.len());
}

#[test]
fn atomic_write_replaces_and_leaves_no_temp() {
    let dir = temp_dir("atomic");
    let path = dir.join("settings.txt");
    atomic_write(&path, b"first").expect("first write");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");
    atomic_write(&path, b"second").expect("replace write");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .collect();
    assert!(leftovers.is_empty());
}

#[test]
fn data_file_names_the_known_files() {
    for name in KNOWN_FILES {
        assert!(data_file(name).ends_with(name));
    }
}

#[test]
fn first_run_copies_legacy_files_into_data_beside_the_exe() {
    // Process-level migration: a copy of the binary in a temp
    // dir (so its data folder is the temp dir's `data\`), with
    // APPDATA pointing at a legacy folder holding pre-v0.9.0
    // files. A settings-writing command must copy the legacy
    // settings across, keep its contents, and leave the legacy
    // originals byte-identical.
    // The second exe must sit on the same filesystem as the real
    // binary so it can be HARD-LINKED (the debug binary is far
    // too large to copy into the temp folder): put the whole
    // fixture in a scratch dir beside the real binary.
    let real_exe = std::path::PathBuf::from(env!("CARGO_BIN_EXE_CalcDex"));
    let dir = real_exe
        .parent()
        .unwrap()
        .join(format!("calcdex-v090-migration-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let legacy = dir.join("AppData").join("CalcDex");
    std::fs::create_dir_all(&legacy).unwrap();
    let legacy_settings = "# CalcDex settings\nfavourite=bmi\n";
    std::fs::write(legacy.join("settings.txt"), legacy_settings).unwrap();
    std::fs::write(legacy.join("history.txt"), "sentinel-history\n").unwrap();
    let exe = dir.join("CalcDex.exe");
    if std::fs::hard_link(&real_exe, &exe).is_err() {
        std::fs::copy(&real_exe, &exe).unwrap();
    }
    let out = std::process::Command::new(&exe)
        .args(["--favourites"])
        .env("APPDATA", dir.join("AppData"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The favourite from the legacy file is live (migration read it).
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("bmi"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    // Copies landed in data\ beside the copied exe ...
    let copied = std::fs::read_to_string(dir.join("data").join("settings.txt")).unwrap();
    assert!(copied.contains("favourite=bmi"), "{copied}");
    assert!(dir.join("data").join("history.txt").exists());
    // ... and the legacy originals are untouched.
    assert_eq!(
        std::fs::read_to_string(legacy.join("settings.txt")).unwrap(),
        legacy_settings
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------- stats median fix ----------

// ---------- CLI maintenance commands (slices 4–5) ----------

/// Run the real binary with the data folder pointed at `dir`.
fn run_cli(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_CalcDex"))
        .args(args)
        .env("CALCDEX_DATA_DIR", dir)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn version_flag_prints_name_and_version() {
    let dir = temp_dir("version");
    let out = run_cli(&dir, &["--version"]);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        format!("CalcDex {}", env!("CARGO_PKG_VERSION"))
    );
    let out = run_cli(&dir, &["--version", "--json"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("\"name\":\"CalcDex\""), "{text}");
    assert!(
        text.contains(&format!("\"version\":\"{}\"", env!("CARGO_PKG_VERSION"))),
        "{text}"
    );
}

#[test]
fn json_run_prints_one_object() {
    let dir = temp_dir("json-run");
    let out = run_cli(&dir, &["bmi", "70", "175", "--json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let text = text.trim();
    assert!(text.starts_with('{') && text.ends_with('}'), "{text}");
    assert!(text.contains("\"ok\":true"), "{text}");
    assert!(text.contains("\"calculator\":\"bmi\""), "{text}");
    assert!(text.contains("\"inputs\":[70,175]"), "{text}");
    assert!(text.contains("22.86"), "{text}");
    // The human decorations are gone in JSON mode.
    assert!(!text.contains("Result:"), "{text}");
    // Expression runs honour --json too.
    let out = run_cli(&dir, &["expr", "2+3", "--json"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("\"calculator\":\"expression-calculator\""),
        "{text}"
    );
    assert!(text.contains("Result: 5"), "{text}");
    // A failing run is a JSON error object: a calculation error
    // keeps the run exit code 1 ...
    let out = run_cli(&dir, &["bmi", "70", "0", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("\"ok\":false"), "{text}");
    assert!(text.contains("over 0"), "{text}");
    // ... and an argument-shape error keeps the usage code 2.
    let out = run_cli(&dir, &["median", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("\"ok\":false"), "{text}");
    assert!(text.contains("at least one"), "{text}");
}

#[test]
fn clean_dry_run_lists_and_removes_nothing() {
    let dir = temp_dir("clean-dry");
    std::fs::write(dir.join("settings.txt"), "x").unwrap();
    std::fs::write(dir.join("history.txt"), "x").unwrap();
    std::fs::write(dir.join("unrelated.txt"), "keep me").unwrap();
    let out = run_cli(&dir, &["--clean", "--dry-run"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("settings.txt"), "{text}");
    assert!(text.contains("history.txt"), "{text}");
    assert!(!text.contains("unrelated.txt"), "{text}");
    assert!(dir.join("settings.txt").exists());
    assert!(dir.join("history.txt").exists());
}

#[test]
fn clean_silent_removes_only_calcdex_files() {
    let dir = temp_dir("clean-silent");
    for name in KNOWN_FILES {
        std::fs::write(dir.join(name), "x").unwrap();
    }
    std::fs::write(dir.join("unrelated.txt"), "keep me").unwrap();
    let out = run_cli(&dir, &["--clean", "--silent"]);
    assert!(out.status.success());
    for name in KNOWN_FILES {
        assert!(!dir.join(name).exists(), "{name} should be removed");
    }
    assert!(dir.join("unrelated.txt").exists());
    // A second clean is a calm no-op.
    let out = run_cli(&dir, &["--clean", "--silent"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Nothing to remove"));
}

#[test]
fn clean_without_silent_asks_and_default_answer_keeps_files() {
    let dir = temp_dir("clean-prompt");
    std::fs::write(dir.join("settings.txt"), "x").unwrap();
    // stdin is /dev/null (EOF at once): the prompt's default is
    // "no", so nothing is removed and the exit code stays 0.
    let out = run_cli(&dir, &["--clean"]);
    assert!(out.status.success());
    assert!(dir.join("settings.txt").exists());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Nothing removed"));
}

#[test]
fn clean_json_reports_removed_files() {
    let dir = temp_dir("clean-json");
    std::fs::write(dir.join("settings.txt"), "x").unwrap();
    let out = run_cli(&dir, &["--clean", "--silent", "--json"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let text = text.trim();
    assert!(text.starts_with('{') && text.ends_with('}'), "{text}");
    assert!(text.contains("\"ok\":true"), "{text}");
    assert!(text.contains("\"action\":\"clean\""), "{text}");
    assert!(text.contains("\"removed\":[\"settings.txt\"]"), "{text}");
    assert!(!dir.join("settings.txt").exists());
}

#[test]
fn uninstall_explains_program_file_removal() {
    let dir = temp_dir("uninstall");
    std::fs::write(dir.join("settings.txt"), "x").unwrap();
    let out = run_cli(&dir, &["--uninstall", "--silent"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Program files are not removed"), "{text}");
    assert!(!dir.join("settings.txt").exists());
}

#[test]
fn median_of_empty_list_is_an_error_not_a_panic() {
    // Through the public dispatch (its own guard) ...
    let result = run_calc("median", &[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("at least one"));
    // ... and calling the calculator closure directly, which is
    // the site that used to index/unwrap an empty list.
    let calc = calcdex::find_calc("median").expect("median calc exists");
    let direct = (calc.run)(&[]);
    assert!(direct.is_err());
    assert!(direct.unwrap_err().contains("at least one value"));
    // A normal median still computes.
    assert!(run_calc("median", &[3.0, 1.0, 2.0])
        .unwrap()
        .contains("Median: 2"));
}
