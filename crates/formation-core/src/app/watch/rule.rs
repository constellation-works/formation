//! Pure validation, line-contract parsing, and formatting rules for `formation watch`.

use super::{Error, refused};
use std::collections::HashSet;
use std::time::Duration;

/// Default interval between probe polls in seconds.
pub const DEFAULT_EVERY_SECONDS: u64 = 120;

/// Minimum allowed interval between probe polls in seconds.
pub const MIN_EVERY_SECONDS: u64 = 10;

/// Default wait timeout without `--to` in seconds.
pub const DEFAULT_WAIT_TIMEOUT_SECONDS: u64 = 300;

/// Maximum wall-clock runtime for a single probe execution in seconds.
pub const PROBE_TIMEOUT_SECONDS: u64 = 60;

/// Maximum items handed over per report or per mailed envelope.
pub(super) const MAX_BATCH_ITEMS: usize = 20;

const MAX_PROBE_NAME_LEN: usize = 32;

const MAX_SEAT_NAME_LEN: usize = 96;

const MAX_FINGERPRINT_CHARS: usize = 200;

const MAX_TEXT_CHARS: usize = 400;

#[derive(Clone, Debug)]
pub(super) struct ProbeSpec {
    pub(super) name: String,
    pub(super) command: String,
}

#[derive(Clone, Debug)]
pub(super) struct ParsedLine {
    pub(super) fingerprint: String,
    pub(super) text: String,
}

pub(super) fn parse_probes(raw: &[String]) -> Result<Vec<ProbeSpec>, Error> {
    if raw.is_empty() {
        return Err(refused(
            "probe_required",
            "at least one --probe <name>=<command> is required",
        ));
    }

    let mut seen = HashSet::with_capacity(raw.len());
    let mut specs = Vec::with_capacity(raw.len());

    for entry in raw {
        let Some((name, command)) = entry.split_once('=') else {
            return Err(refused(
                "invalid_probe",
                format!("invalid probe `{entry}`: expected <name>=<command>"),
            ));
        };
        probe_name(name)?;
        if !seen.insert(name) {
            return Err(refused(
                "duplicate_probe",
                format!("duplicate probe name `{name}`"),
            ));
        }
        if command.trim().is_empty() {
            return Err(refused(
                "invalid_probe",
                format!("probe `{name}` command must not be empty"),
            ));
        }
        specs.push(ProbeSpec {
            name: name.to_owned(),
            command: command.to_owned(),
        });
    }

    Ok(specs)
}

pub(super) fn probe_name(name: &str) -> Result<(), Error> {
    let valid = !name.is_empty()
        && name.len() <= MAX_PROBE_NAME_LEN
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(refused(
            "invalid_probe_name",
            format!(
                "invalid probe name `{name}` (use 1-32 lowercase ASCII letters, digits or hyphens)"
            ),
        ))
    }
}

pub(super) fn every(interval: Duration) -> Result<(), Error> {
    if interval < Duration::from_secs(MIN_EVERY_SECONDS) {
        return Err(refused(
            "invalid_interval",
            format!("--every must be at least {MIN_EVERY_SECONDS} seconds"),
        ));
    }
    Ok(())
}

pub(super) fn seat_name(name: &str) -> Result<(), Error> {
    if name.len() <= MAX_SEAT_NAME_LEN && formation_store::contract::identifier(name) {
        Ok(())
    } else {
        Err(refused(
            "invalid_identifier",
            "invalid identifier (use 1-96 letters, digits, hyphens or underscores)",
        ))
    }
}

pub(super) fn parse_output(stdout: &[u8]) -> Result<Vec<ParsedLine>, String> {
    let text =
        std::str::from_utf8(stdout).map_err(|_| "probe stdout is not valid UTF-8".to_owned())?;
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        let Some((fingerprint, raw_text)) = line.split_once('\t') else {
            return Err(format!("line {line_no}: expected <fingerprint><TAB><text>"));
        };
        let fingerprint_len = fingerprint.chars().count();
        if fingerprint_len == 0 || fingerprint_len > MAX_FINGERPRINT_CHARS {
            return Err(format!(
                "line {line_no}: fingerprint must be 1 to {MAX_FINGERPRINT_CHARS} characters"
            ));
        }
        if seen.insert(fingerprint.to_owned()) {
            items.push(ParsedLine {
                fingerprint: fingerprint.to_owned(),
                text: truncate_chars(raw_text, MAX_TEXT_CHARS),
            });
        }
    }

    Ok(items)
}

pub(super) fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .find(|candidate| !candidate.trim().is_empty())
        .unwrap_or("probe failed")
        .trim_end_matches('\r');
    truncate_chars(line, MAX_TEXT_CHARS)
}

pub(super) fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let keep = max_chars.saturating_sub(1);
    let prefix: String = chars.by_ref().take(keep).collect();
    match chars.next() {
        None => prefix,
        Some(last) => match chars.next() {
            None => {
                let mut full = prefix;
                full.push(last);
                full
            }
            Some(_) => {
                let mut cut = prefix;
                cut.push('…');
                cut
            }
        },
    }
}

pub(super) fn mail_line(probe: &str, text: &str) -> String {
    format!("{probe} | {text}")
}

pub(super) fn failed_mail_line(probe: &str, error: &str) -> String {
    format!("{probe} | read failed: {error}")
}
