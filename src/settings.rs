//! Persistent settings (v0.3.0, items #1 / #46 / #48): favourites,
//! recent calculators, per-calculator remembered inputs, and the
//! sig-figs choice — one simple documented text file shared by the
//! window and the command line, at `data\settings.txt` beside the
//! executable (v0.9.0 portable data folder; the pre-v0.9.0
//! %APPDATA%\CalcDex file is copied across on first run — see
//! `crate::paths`).
//!
//! Format (one record per line, `#` comments allowed):
//!   schema_version=1      (the file-format version, v0.9.0; a
//!                          file with no such line is version 0
//!                          and is migrated in memory on load; a
//!                          version NEWER than this build loads
//!                          as defaults with a warning, and this
//!                          build will not overwrite the file)
//!   window=<x>,<y>,<w>,<h>  (last window position + inner size,
//!                          v0.9.0; implausible values are ignored)
//!   display_format=sig-figs:3   (off | sig-figs:N | decimals:N |
//!                                engineering | auto; v0.4.0 — the
//!                                old sig_figs= line still loads)
//!   rounding=half-up      (half-up | bankers | truncate)
//!   update_check_startup=off    (offer-only update check; off
//!                                unless the user switches it on)
//!   steps=standard        (standard | deeper; v0.5.0 #12)
//!   projection=off        (on | off; v0.6.0 #21 classroom
//!                          projection mode: larger text + higher
//!                          contrast in the window)
//!   favourite=<calc id>
//!   recent=<id>,<id>,...  (most recent first, max 8)
//!   inputs.<calc id>=<token>;;<token>;;...
//!   constant.<name>=<value>   (v0.7.0 #38: a user-defined
//!                                constant for the Expression
//!                                Calculator; names are validated
//!                                by rigour::validate_constant_name,
//!                                so a line naming a built-in
//!                                constant or function never loads)
//! Input tokens are the display tokens of a run (dates as
//! MM/DD/YYYY), separated by ";;" exactly like the calculation log's
//! restore field; backslash escapes `\\`, `\;` and `\n` inside a
//! token. Unknown ids and malformed lines are ignored on load — a
//! corrupt or missing file NEVER breaks startup; the app falls back
//! to defaults (see `load_settings`).

use crate::rigour::{DisplayFormat, DisplayMode, RoundingMode};
use std::collections::{BTreeSet, HashMap};

pub const RECENT_CAP: usize = 8;

/// The settings file-format version this build writes. Files
/// declaring a NEWER version are never overwritten by this build
/// (see `save_settings_to`) and load as defaults with a warning
/// (see `load_settings_report_from`).
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    /// Unified display format (v0.4.0 #1/#2/#3/#37): mode,
    /// figure count, and the rounding mode (#9).
    pub display: DisplayFormat,
    /// Offer-only update check at startup (default OFF — the
    /// program makes no network connection unless this is on or
    /// the user presses Check for updates).
    pub update_check_startup: bool,
    /// Deeper steps mode (v0.5.0, program #12): the steps area
    /// shows the four labelled stages instead of the raw output.
    pub steps_deeper: bool,
    /// Classroom projection mode (v0.6.0, program #21): the
    /// window renders at ~1.5x zoom with brightened muted/hint
    /// colours so the back of a classroom can read it.
    pub projection: bool,
    pub favourites: BTreeSet<String>,
    /// Most recent first.
    pub recent: Vec<String>,
    /// calc id -> display tokens of its last run (item #48).
    pub remembered: HashMap<String, Vec<String>>,
    /// User-defined constants (v0.7.0 #38): (name, value) pairs
    /// in definition order; names are stored already validated +
    /// lowercased by rigour::validate_constant_name.
    pub constants: Vec<(String, f64)>,
    /// Last window geometry (v0.9.0): outer position (x, y) and
    /// inner size (w, h) in points, restored on the next launch.
    /// Values are range-checked at parse time; anything
    /// implausible is dropped rather than restored.
    pub window: Option<(f32, f32, f32, f32)>,
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Split a token list on unescaped ";;".
fn split_tokens(s: &str) -> Vec<String> {
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
            out.push(unesc(&cur));
            cur.clear();
            i += 2;
        } else {
            cur.push(b[i]);
            i += 1;
        }
    }
    out.push(unesc(&cur));
    out
}

pub fn serialize_settings(s: &Settings) -> String {
    let mut out = String::new();
    out.push_str("# CalcDex settings — see settings.rs for the format.\n");
    out.push_str(&format!("schema_version={SETTINGS_SCHEMA_VERSION}\n"));
    if let Some((x, y, w, h)) = s.window {
        out.push_str(&format!("window={x},{y},{w},{h}\n"));
    }
    out.push_str(&format!("display_format={}\n", s.display.as_setting()));
    out.push_str(&format!("rounding={}\n", s.display.rounding.as_setting()));
    out.push_str(&format!(
        "update_check_startup={}\n",
        if s.update_check_startup { "on" } else { "off" }
    ));
    out.push_str(if s.steps_deeper {
        "steps=deeper\n"
    } else {
        "steps=standard\n"
    });
    out.push_str(if s.projection {
        "projection=on\n"
    } else {
        "projection=off\n"
    });
    for (name, value) in &s.constants {
        out.push_str(&format!("constant.{name}={value}\n"));
    }
    for f in &s.favourites {
        out.push_str(&format!("favourite={f}\n"));
    }
    if !s.recent.is_empty() {
        out.push_str(&format!("recent={}\n", s.recent.join(",")));
    }
    let mut ids: Vec<&String> = s.remembered.keys().collect();
    ids.sort();
    for id in ids {
        let toks: Vec<String> = s.remembered[id].iter().map(|t| esc(t)).collect();
        out.push_str(&format!("inputs.{id}={}\n", toks.join(";;")));
    }
    out
}

/// Parse settings text. Anything unrecognised is skipped; ids that
/// are not in `valid_ids` are dropped (a settings file from another
/// version can never resurrect a removed calculator).
pub fn parse_settings(text: &str, valid_ids: &BTreeSet<String>) -> Settings {
    let mut s = Settings::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("schema_version=") {
            // The format version line: consumed by
            // `declared_schema_version`; it carries no setting.
            continue;
        } else if let Some(v) = line.strip_prefix("window=") {
            // Last window geometry (v0.9.0): four numbers,
            // range-checked so a corrupt or off-screen value is
            // ignored instead of restored.
            let nums: Vec<f32> = v.split(',').filter_map(|p| p.trim().parse().ok()).collect();
            if nums.len() == 4 {
                let (x, y, w, h) = (nums[0], nums[1], nums[2], nums[3]);
                if (-32000.0..=32000.0).contains(&x)
                    && (-32000.0..=32000.0).contains(&y)
                    && (320.0..=10000.0).contains(&w)
                    && (240.0..=10000.0).contains(&h)
                {
                    s.window = Some((x, y, w, h));
                }
            }
        } else if let Some(v) = line.strip_prefix("display_format=") {
            if let Ok(f) = DisplayFormat::parse(v) {
                // Keep an already-read rounding choice.
                let rounding = s.display.rounding;
                s.display = DisplayFormat { rounding, ..f };
            }
        } else if let Some(v) = line.strip_prefix("rounding=") {
            if let Some(r) = RoundingMode::parse(v) {
                s.display.rounding = r;
            }
        } else if let Some(v) = line.strip_prefix("update_check_startup=") {
            s.update_check_startup = v == "on" || v == "true";
        } else if let Some(v) = line.strip_prefix("steps=") {
            s.steps_deeper = v.trim() == "deeper";
        } else if let Some(v) = line.strip_prefix("projection=") {
            s.projection = v.trim() == "on" || v.trim() == "true";
        } else if let Some(v) = line.strip_prefix("sig_figs=") {
            // Legacy (v0.3.0) line: only applies when no explicit
            // display_format line has set a mode.
            if s.display.mode == DisplayMode::Off && v != "off" {
                if let Some(n) = v.parse::<u8>().ok().filter(|n| (1..=10).contains(n)) {
                    s.display.mode = DisplayMode::SigFigs;
                    s.display.n = Some(n);
                }
            }
        } else if let Some(v) = line.strip_prefix("favourite=") {
            if valid_ids.contains(v) {
                s.favourites.insert(v.to_string());
            }
        } else if let Some(v) = line.strip_prefix("recent=") {
            s.recent = v
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| valid_ids.contains(x))
                .take(RECENT_CAP)
                .collect();
        } else if let Some(rest) = line.strip_prefix("constant.") {
            // User-defined constants (#38): a line loads only if
            // the name still validates (no built-in collision, no
            // function name) and the value is a finite number —
            // anything else is skipped like any other corrupt line.
            if let Some((name, value)) = rest.split_once('=') {
                if let (Ok(name), Ok(v)) = (
                    crate::rigour::validate_constant_name(name),
                    value.trim().parse::<f64>(),
                ) {
                    if v.is_finite() {
                        s.constants.retain(|(n, _)| n != &name);
                        s.constants.push((name, v));
                    }
                }
            }
        } else if let Some(rest) = line.strip_prefix("inputs.") {
            if let Some((id, toks)) = rest.split_once('=') {
                if valid_ids.contains(id) && !toks.is_empty() {
                    s.remembered.insert(id.to_string(), split_tokens(toks));
                }
            }
        }
    }
    s
}

/// Where the settings live: `settings.txt` in the app's data
/// folder (v0.9.0: `data\` beside the executable, with the legacy
/// `%APPDATA%\CalcDex` copy-migration and fallbacks — see
/// `crate::paths`; same rule as the history log).
pub fn settings_path() -> std::path::PathBuf {
    crate::paths::data_file("settings.txt")
}

/// The schema version a settings text declares: the value of its
/// `schema_version=` line, or 0 when the line is absent (every
/// pre-v0.9.0 file) or unreadable.
pub fn declared_schema_version(text: &str) -> u32 {
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("schema_version=") {
            return v.trim().parse::<u32>().unwrap_or(0);
        }
    }
    0
}

/// How a settings load went (v0.9.0 schema handling), for callers
/// and tests that care; plain `load_settings_from` discards it.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingsLoadNote {
    /// No readable file: defaults, nothing to migrate.
    Fresh,
    /// The file already declared the current schema version.
    Current,
    /// The file declared an older version (0 = no line at all)
    /// and was migrated in memory; the next save writes the
    /// current version.
    MigratedFrom(u32),
    /// The file declared a version NEWER than this build: the
    /// settings loaded as defaults (with a warning printed) and
    /// this build will not overwrite the file.
    FutureVersion(u32),
}

/// Load settings from `path` and report how the load went.
/// Missing/corrupt -> defaults. Never fails: persistence
/// problems must never break startup.
pub fn load_settings_report_from(
    path: &std::path::Path,
    valid_ids: &BTreeSet<String>,
) -> (Settings, SettingsLoadNote) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (Settings::default(), SettingsLoadNote::Fresh);
    };
    let declared = declared_schema_version(&text);
    if declared > SETTINGS_SCHEMA_VERSION {
        eprintln!(
            "CalcDex: {} declares schema_version {declared}, which is newer than this build understands ({SETTINGS_SCHEMA_VERSION}) — loading defaults instead; the file has NOT been changed.",
            path.display()
        );
        return (
            Settings::default(),
            SettingsLoadNote::FutureVersion(declared),
        );
    }
    let note = if declared == SETTINGS_SCHEMA_VERSION {
        SettingsLoadNote::Current
    } else {
        SettingsLoadNote::MigratedFrom(declared)
    };
    (parse_settings(&text, valid_ids), note)
}

/// Load settings from `path`; missing/corrupt -> defaults. Never
/// fails: persistence problems must never break startup.
pub fn load_settings_from(path: &std::path::Path, valid_ids: &BTreeSet<String>) -> Settings {
    load_settings_report_from(path, valid_ids).0
}

pub fn load_settings(valid_ids: &BTreeSet<String>) -> Settings {
    load_settings_from(&settings_path(), valid_ids)
}

pub fn save_settings_to(path: &std::path::Path, s: &Settings) -> std::io::Result<()> {
    // Never clobber a settings file written by a NEWER build:
    // its schema may hold things this build would silently drop.
    if let Ok(existing) = std::fs::read_to_string(path) {
        let declared = declared_schema_version(&existing);
        if declared > SETTINGS_SCHEMA_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "settings file declares schema_version {declared} (newer than this build's {SETTINGS_SCHEMA_VERSION}); not overwriting it"
                ),
            ));
        }
    }
    // Atomic save (v0.9.0): temp file + flush + rename.
    crate::paths::atomic_write(path, serialize_settings(s).as_bytes())
}

pub fn save_settings(s: &Settings) -> std::io::Result<()> {
    save_settings_to(&settings_path(), s)
}

impl Settings {
    /// Record a run of `id`: it becomes the most-recent calculator
    /// and its display tokens are remembered (item #48).
    pub fn note_run(&mut self, id: &str, tokens: Vec<String>) {
        self.recent.retain(|r| r != id);
        self.recent.insert(0, id.to_string());
        self.recent.truncate(RECENT_CAP);
        self.remembered.insert(id.to_string(), tokens);
    }

    pub fn toggle_favourite(&mut self, id: &str) -> bool {
        if self.favourites.contains(id) {
            self.favourites.remove(id);
            false
        } else {
            self.favourites.insert(id.to_string());
            true
        }
    }
}
