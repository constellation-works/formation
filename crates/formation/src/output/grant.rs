//! Grant command output: wire shapes and human tables.

use formation_core::app::grant::{EffectiveEntry, EffectiveSeat, GrantView, Outcome};
use serde_json::{Value, json};
use std::io::{self, Write};
use std::process::ExitCode;

const ROOM_HEADER: [&str; 9] = [
    "ID",
    "EFFECT",
    "TO",
    "CLASS",
    "WORKSPACE",
    "FOR",
    "ENTERED BY",
    "FLAGS",
    "LIMIT",
];

const SEAT_HEADER: [&str; 5] = ["CLASS", "WORKSPACE", "FROM", "FLAGS", "LIMIT"];

pub(crate) fn write(out: &Outcome) -> ExitCode {
    super::common::json::write_json(&payload(out))
}

pub(crate) fn room_list(grants: &[GrantView], json: bool) -> ExitCode {
    if grants.is_empty() {
        let _ = writeln!(io::stderr().lock(), "No active grants in this room.");
    }
    if json {
        return super::common::json::write_json(&room_list_payload(grants));
    }
    if grants.is_empty() {
        return ExitCode::SUCCESS;
    }
    match write_room_table(&mut io::stdout().lock(), grants) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::common::error::failure(&error),
    }
}

pub(crate) fn seat_list(effective: &EffectiveSeat, json: bool) -> ExitCode {
    if effective.settles.is_empty() && effective.withheld.is_empty() {
        let _ = writeln!(
            io::stderr().lock(),
            "No authority entries for {}.",
            effective.seat
        );
    }
    if json {
        return super::common::json::write_json(&seat_list_payload(effective));
    }
    if effective.settles.is_empty() && effective.withheld.is_empty() {
        return ExitCode::SUCCESS;
    }
    match write_seat_tables(&mut io::stdout().lock(), effective) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::common::error::failure(&error),
    }
}

fn payload(out: &Outcome) -> Value {
    match out {
        Outcome::Recorded { grant, notified } => {
            let mut value = grant_payload(grant);
            if let Some(map) = value.as_object_mut() {
                map.insert("notified".to_owned(), json!(notified));
            }
            value
        }
        Outcome::Revoked {
            id,
            revoked_by,
            revoked_at,
            notified,
        } => json!({
            "id": id,
            "revoked": true,
            "revoked_by": revoked_by,
            "revoked_at": revoked_at,
            "notified": notified,
        }),
        Outcome::RoomList(grants) => room_list_payload(grants),
        Outcome::SeatList(effective) => seat_list_payload(effective),
        Outcome::Checked { seat, role, entry } => json!({
            "allowed": true,
            "seat": seat,
            "role": role,
            "class": entry.class,
            "workspace": entry.workspace,
            "from": entry.from_kind(),
            "origin": entry.origin.as_str(),
            "grant_id": entry.grant_id,
            "limit": entry.limit,
            "self_entered": entry.self_entered,
        }),
    }
}

fn room_list_payload(grants: &[GrantView]) -> Value {
    json!({
        "grants": grants.iter().map(grant_payload).collect::<Vec<_>>(),
    })
}

fn grant_payload(grant: &GrantView) -> Value {
    json!({
        "id": grant.id,
        "effect": grant.effect.as_str(),
        "to": grant.to,
        "target_kind": grant.target_kind.as_str(),
        "class": grant.class,
        "workspace": grant.workspace,
        "limit": grant.limit,
        "for": grant.lifetime.as_str(),
        "source": grant.source,
        "entered_by": grant.entered_by,
        "entered_at": grant.entered_at,
        "self_entered": grant.self_entered,
    })
}

fn seat_list_payload(effective: &EffectiveSeat) -> Value {
    json!({
        "seat": effective.seat,
        "role": effective.role,
        "workspace": effective.workspace,
        "settles": effective.settles.iter().map(effective_payload).collect::<Vec<_>>(),
        "withheld": effective.withheld.iter().map(effective_payload).collect::<Vec<_>>(),
    })
}

fn effective_payload(entry: &EffectiveEntry) -> Value {
    json!({
        "class": entry.class,
        "workspace": entry.workspace,
        "from": entry.from_kind(),
        "origin": entry.origin.as_str(),
        "grant_id": entry.grant_id,
        "limit": entry.limit,
        "self_entered": entry.self_entered,
    })
}

fn write_room_table(out: &mut impl Write, grants: &[GrantView]) -> io::Result<()> {
    let rows: Vec<[String; 9]> = grants
        .iter()
        .map(|grant| {
            [
                grant.id.to_string(),
                grant.effect.as_str().to_owned(),
                grant.to.clone(),
                grant.class.clone(),
                grant.workspace.clone(),
                grant.lifetime.as_str().to_owned(),
                grant.entered_by.clone().unwrap_or_else(|| "-".to_owned()),
                flags_cell(grant.self_entered).to_owned(),
                grant.limit.clone().unwrap_or_else(|| "-".to_owned()),
            ]
        })
        .collect();
    write_table(out, &ROOM_HEADER, &rows, &[0])
}

fn write_seat_tables(out: &mut impl Write, effective: &EffectiveSeat) -> io::Result<()> {
    if !effective.settles.is_empty() {
        let rows = effective_rows(&effective.settles);
        write_table(out, &SEAT_HEADER, &rows, &[])?;
    }
    if !effective.withheld.is_empty() {
        if !effective.settles.is_empty() {
            writeln!(out)?;
        }
        writeln!(out, "Withheld:")?;
        let rows = effective_rows(&effective.withheld);
        write_table(out, &SEAT_HEADER, &rows, &[])?;
    }
    Ok(())
}

fn effective_rows(entries: &[EffectiveEntry]) -> Vec<[String; 5]> {
    entries
        .iter()
        .map(|entry| {
            let from = match entry.grant_id {
                Some(id) => format!("{} {id}", entry.from_kind()),
                None => entry.from_kind().to_owned(),
            };
            [
                entry.class.clone(),
                entry.workspace.clone(),
                from,
                flags_cell(entry.self_entered).to_owned(),
                entry.limit.clone().unwrap_or_else(|| "-".to_owned()),
            ]
        })
        .collect()
}

fn flags_cell(self_entered: bool) -> &'static str {
    if self_entered { "self-entered" } else { "-" }
}

fn write_table<const COLS: usize>(
    out: &mut impl Write,
    header: &[&str; COLS],
    rows: &[[String; COLS]],
    numeric: &[usize],
) -> io::Result<()> {
    let widths: Vec<usize> = (0..COLS)
        .map(|column| {
            rows.iter()
                .map(|row| row[column].chars().count())
                .chain([header[column].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let header_row: [String; COLS] = header.map(str::to_owned);
    for row in std::iter::once(&header_row).chain(rows) {
        let mut line = String::new();
        for (column, cell) in row.iter().enumerate() {
            if column > 0 {
                line.push_str("  ");
            }
            let width = widths[column];
            let last = column + 1 == COLS;
            match (numeric.contains(&column), last) {
                (true, _) => line.push_str(&format!("{cell:>width$}")),
                (false, true) => line.push_str(cell),
                (false, false) => line.push_str(&format!("{cell:<width$}")),
            }
        }
        writeln!(out, "{line}")?;
    }
    Ok(())
}
