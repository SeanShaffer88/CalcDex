//! Versioned, atomic state file for the RPN session.
//! Format (text, `schema_version=1`): angle, disp, lift, stack, lastx, var lines.

use super::display::Disp;
use super::{Angle, RpnError, Session, MAX_VARS};
use crate::paths::{atomic_write, data_file};

pub const STATE_SCHEMA_VERSION: u32 = 1;

pub fn valid_state_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn state_file_name(name: &str) -> String {
    format!("rpn-{name}.txt")
}

pub fn serialize(s: &Session) -> String {
    let mut out = format!("schema_version={STATE_SCHEMA_VERSION}\n");
    out.push_str(&format!("angle={}\n", s.angle.label()));
    out.push_str(&format!("disp={}\n", s.disp.to_file()));
    out.push_str(&format!("lift={}\n", u8::from(s.lift)));
    out.push_str(&format!(
        "stack={:e}|{:e}|{:e}|{:e}\n",
        s.stack[0], s.stack[1], s.stack[2], s.stack[3]
    ));
    out.push_str(&format!("lastx={:e}\n", s.lastx));
    for (k, v) in &s.vars {
        out.push_str(&format!("var={k}|{v:e}\n"));
    }
    out
}

pub enum ParseOutcome {
    Ok(Box<Session>),
    /// Written by a newer CalcDex: must not be overwritten.
    Newer(u32),
    Bad(String),
}

fn num(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|v| v.is_finite())
}

pub fn parse(text: &str) -> ParseOutcome {
    let mut s = Session::new();
    let mut version: Option<u32> = None;
    let mut seen_stack = false;
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return ParseOutcome::Bad(format!("line {} has no '='", i + 1));
        };
        match k {
            "schema_version" => match v.parse::<u32>() {
                Ok(n) => version = Some(n),
                Err(_) => return ParseOutcome::Bad("bad schema_version".into()),
            },
            "angle" => match Angle::from_label(v) {
                Some(a) => s.angle = a,
                None => return ParseOutcome::Bad(format!("bad angle '{v}'")),
            },
            "disp" => match Disp::from_file(v) {
                Some(d) => s.disp = d,
                None => return ParseOutcome::Bad(format!("bad disp '{v}'")),
            },
            "lift" => s.lift = v == "1",
            "stack" => {
                let parts: Vec<&str> = v.split('|').collect();
                if parts.len() != 4 {
                    return ParseOutcome::Bad("stack needs 4 values".into());
                }
                for (j, p) in parts.iter().enumerate() {
                    match num(p) {
                        Some(x) => s.stack[j] = x,
                        None => return ParseOutcome::Bad(format!("bad stack value '{p}'")),
                    }
                }
                seen_stack = true;
            }
            "lastx" => match num(v) {
                Some(x) => s.lastx = x,
                None => return ParseOutcome::Bad("bad lastx".into()),
            },
            "var" => {
                let Some((name, val)) = v.rsplit_once('|') else {
                    return ParseOutcome::Bad("var line needs name|value".into());
                };
                let Some(x) = num(val) else {
                    return ParseOutcome::Bad(format!("bad value for variable '{name}'"));
                };
                if !super::valid_var_name(name) || s.vars.len() >= MAX_VARS {
                    return ParseOutcome::Bad(format!("bad variable '{name}'"));
                }
                s.vars.insert(name.to_string(), x);
            }
            _ => {} // unknown keys from a same-version writer are ignored
        }
    }
    match version {
        Some(n) if n > STATE_SCHEMA_VERSION => ParseOutcome::Newer(n),
        Some(_) if seen_stack => ParseOutcome::Ok(Box::new(s)),
        Some(_) => ParseOutcome::Bad("no stack line".into()),
        None => ParseOutcome::Bad("no schema_version line".into()),
    }
}

pub struct Loaded {
    pub session: Session,
    pub note: Option<String>,
    /// True when the file must not be written back (newer version).
    pub read_only: bool,
}

/// Load named state from the data folder. Missing file = fresh
/// session. Unparseable file is quarantined (renamed `.bad`), never
/// silently reset.
pub fn load(name: &str) -> Result<Loaded, RpnError> {
    if !valid_state_name(name) {
        return Err(RpnError::new(
            "RPN-106",
            "State names use letters, digits, '_' and '-' (max 32).",
        ));
    }
    let path = data_file(&state_file_name(name));
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Loaded {
                session: Session::new(),
                note: None,
                read_only: false,
            })
        }
        Err(e) => {
            return Err(RpnError::new(
                "RPN-107",
                format!("Cannot read state '{name}': {e}"),
            ))
        }
    };
    match parse(&text) {
        ParseOutcome::Ok(s) => Ok(Loaded {
            session: *s,
            note: None,
            read_only: false,
        }),
        ParseOutcome::Newer(n) => Ok(Loaded {
            session: Session::new(),
            note: Some(format!(
                "State '{name}' was written by a newer CalcDex (schema {n}); using defaults and not overwriting it."
            )),
            read_only: true,
        }),
        ParseOutcome::Bad(why) => {
            let bad = path.with_extension("txt.bad");
            let moved = std::fs::rename(&path, &bad).is_ok();
            Ok(Loaded {
                session: Session::new(),
                note: Some(format!(
                    "State '{name}' could not be read ({why}); {} Starting fresh.",
                    if moved {
                        "the file was kept as a .bad copy."
                    } else {
                        "it could not be moved aside."
                    }
                )),
                read_only: false,
            })
        }
    }
}

pub fn save(name: &str, s: &Session) -> Result<(), RpnError> {
    if !valid_state_name(name) {
        return Err(RpnError::new(
            "RPN-106",
            "State names use letters, digits, '_' and '-' (max 32).",
        ));
    }
    atomic_write(&data_file(&state_file_name(name)), serialize(s).as_bytes())
        .map_err(|e| RpnError::new("RPN-107", format!("Cannot save state '{name}': {e}")))
}
