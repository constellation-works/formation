//! Watch command output: wire shape and JSON write.

use formation_core::app::watch::Outcome as WatchOutcome;
use formation_core::app::watch::set::{Entry, Outcome as SetOutcome, Reader};
use serde_json::{Value, json};
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn payload(out: WatchOutcome) -> Value {
    match out {
        WatchOutcome::Report {
            items,
            failed,
            remaining,
            expired,
        } => json!({
            "items": items
                .into_iter()
                .map(|item| json!({
                    "probe": item.probe,
                    "fingerprint": item.fingerprint,
                    "text": item.text,
                }))
                .collect::<Vec<_>>(),
            "failed": failed
                .into_iter()
                .map(|failure| json!({
                    "probe": failure.probe,
                    "error": failure.error,
                }))
                .collect::<Vec<_>>(),
            "remaining": remaining,
            "expired": expired,
        }),
        WatchOutcome::Mailed {
            to,
            envelopes,
            items,
            failed,
            expired,
        } => json!({
            "to": to,
            "envelopes": envelopes,
            "items": items,
            "failed": failed,
            "expired": expired,
        }),
    }
}

pub(crate) fn write(value: &Value) -> ExitCode {
    super::shared::write_json(value)
}

pub(crate) fn set_payload(out: &SetOutcome) -> Value {
    match out {
        SetOutcome::Added {
            name,
            to,
            replaced,
            every,
            command,
            cwd,
        } => json!({
            "name": name,
            "to": to,
            "replaced": replaced,
            "every_seconds": every,
            "state": "on",
            "command": command,
            "cwd": cwd,
        }),
        SetOutcome::Removed {
            name,
            to,
            forgotten,
        } => json!({"name": name, "to": to, "removed": true, "forgotten": forgotten}),
        SetOutcome::State {
            name,
            to,
            state,
            changed,
        } => json!({"name": name, "to": to, "state": state.as_str(), "changed": changed}),
        SetOutcome::Listed { reader, watches } => list_payload(reader, watches),
    }
}

fn list_payload(reader: &Reader, watches: &[Entry]) -> Value {
    json!({
        "reader": match reader {
            Reader::None => json!({"state": "none"}),
        },
        "watches": watches
            .iter()
            .map(|watch| json!({
                "name": watch.name,
                "to": watch.to,
                "command": watch.command,
                "cwd": watch.cwd,
                "every_seconds": watch.every,
                "state": watch.state.as_str(),
                "added_by": watch.added_by,
                "added_at": watch.added_at,
                "last_read": watch.last_read,
                "last_result": watch.last_result,
                "sent": watch.sent,
            }))
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn list(reader: &Reader, watches: &[Entry], json: bool) -> ExitCode {
    if watches.is_empty() {
        let _ = writeln!(io::stderr().lock(), "No watches in this room.");
    }
    if json {
        return super::shared::write_json(&list_payload(reader, watches));
    }
    match write_list(&mut io::stdout().lock(), reader, watches) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::shared::failure(&error),
    }
}

const HEADER: [&str; 8] = [
    "NAME",
    "TO",
    "EVERY (s)",
    "STATE",
    "LAST READ",
    "RESULT",
    "SENT",
    "ADDED BY",
];

/// Columns that hold numbers are right-aligned.
const NUMERIC: [usize; 2] = [2, 6];

fn write_list(out: &mut impl Write, reader: &Reader, watches: &[Entry]) -> io::Result<()> {
    match reader {
        Reader::None => writeln!(out, "reader: none")?,
    }
    if watches.is_empty() {
        return Ok(());
    }
    let rows: Vec<[String; 8]> = watches
        .iter()
        .map(|watch| {
            [
                watch.name.clone(),
                watch.to.clone(),
                watch.every.to_string(),
                watch.state.as_str().to_owned(),
                watch.last_read.clone().unwrap_or_else(|| "-".to_owned()),
                watch.last_result.clone().unwrap_or_else(|| "-".to_owned()),
                watch.sent.to_string(),
                watch.added_by.clone(),
            ]
        })
        .collect();
    let widths: Vec<usize> = (0..HEADER.len())
        .map(|column| {
            rows.iter()
                .map(|row| row[column].chars().count())
                .chain([HEADER[column].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let header: [String; 8] = HEADER.map(str::to_owned);
    for row in std::iter::once(&header).chain(&rows) {
        let mut line = String::new();
        for (column, cell) in row.iter().enumerate() {
            if column > 0 {
                line.push_str("  ");
            }
            let width = widths[column];
            let last = column + 1 == row.len();
            match (NUMERIC.contains(&column), last) {
                (true, _) => line.push_str(&format!("{cell:>width$}")),
                (false, true) => line.push_str(cell),
                (false, false) => line.push_str(&format!("{cell:<width$}")),
            }
        }
        writeln!(out, "{line}")?;
    }
    Ok(())
}
