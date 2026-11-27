//! Request records as structured JSON, detail text and list rows.

use formation_core::Error;
use formation_core::app::request::{Outcome, Request, RequestAdvice, RequestOption};
use formation_core::app::request::{RequestSettlement, RequestState};
use serde::Serialize;
use std::io::{self, Write};
use std::process::ExitCode;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const HEADER: [&str; 11] = [
    "ID",
    "STATE",
    "HOLDER",
    "CLASS",
    "ASK",
    "OPTIONS",
    "RECOMMEND",
    "SETTLED BY",
    "CHOICE",
    "GRANT",
    "LIMIT",
];

#[derive(Serialize)]
struct Payload<'a> {
    id: &'a str,
    state: &'static str,
    holder: Option<&'a str>,
    holder_kind: Option<&'static str>,
    asker: &'a str,
    class: &'a str,
    workspace: &'a str,
    record: &'a str,
    ask: &'a str,
    options: &'a [RequestOption],
    recommendation: &'a str,
    blocking: bool,
    #[serde(with = "time::serde::rfc3339")]
    opened_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
    advice: &'a [RequestAdvice],
    settlement: Option<&'a RequestSettlement>,
}

impl<'a> Payload<'a> {
    fn new(record: &'a Request) -> Self {
        let (state, holder, holder_kind, settlement) = match &record.state {
            RequestState::Open { holder: Some(seat) } => {
                ("open", Some(seat.as_str()), Some("seat"), None)
            }
            RequestState::Open { holder: None } => ("open", Some("admiral"), Some("admiral"), None),
            RequestState::Settled(settlement) => ("settled", None, None, Some(settlement)),
        };
        Self {
            id: &record.id,
            state,
            holder,
            holder_kind,
            asker: &record.asker,
            class: &record.class,
            workspace: &record.workspace,
            record: &record.record,
            ask: &record.ask,
            options: &record.options,
            recommendation: &record.recommendation,
            blocking: record.blocking,
            opened_at: record.opened_at,
            updated_at: record.updated_at,
            advice: &record.advice,
            settlement,
        }
    }

    fn row(&self) -> [String; HEADER.len()] {
        let options = self
            .options
            .iter()
            .map(|option| format!("{}: {}", option.label, option.text))
            .collect::<Vec<_>>()
            .join(" | ");
        let recommendation = self
            .advice
            .last()
            .map_or(self.recommendation, |advice| advice.option.as_str());
        let settlement = self.settlement;
        let limit = settlement
            .and_then(|settlement| settlement.limit.as_deref())
            .map_or_else(|| "-".to_owned(), |limit| limit.escape_debug().to_string());
        [
            self.id.to_owned(),
            self.state.to_owned(),
            self.holder.unwrap_or("-").to_owned(),
            self.class.to_owned(),
            self.ask.to_owned(),
            options,
            recommendation.to_owned(),
            settlement.map_or("-", |settled| &settled.seat).to_owned(),
            settlement.map_or("-", |settled| &settled.option).to_owned(),
            settlement.map_or_else(|| "-".to_owned(), grant),
            limit,
        ]
    }
}

pub(crate) fn write(result: Result<Outcome, Error>, json: bool) -> ExitCode {
    match result {
        Ok(Outcome::Record(record)) => {
            let payload = Payload::new(&record);
            if json {
                return write_json(&payload);
            }
            finish_io(write_detail(&mut io::stdout().lock(), &payload))
        }
        Ok(Outcome::List(records)) => {
            let payloads: Vec<_> = records.iter().map(Payload::new).collect();
            if payloads.is_empty() {
                let _ = writeln!(io::stderr().lock(), "No requests match in this room.");
            }
            if json {
                return write_json(&payloads);
            }

            let (stdout, terminal) = super::common::stdout();
            finish_io(write_rows(&mut stdout.lock(), &payloads, terminal))
        }
        Err(error) if json => super::error(&error),
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn write_json(payload: &impl Serialize) -> ExitCode {
    match serde_json::to_value(payload) {
        Ok(value) => super::common::json::write_json(&value),
        Err(error) => super::failure(&error),
    }
}

fn write_detail(out: &mut impl Write, record: &Payload<'_>) -> io::Result<()> {
    writeln!(out, "Request: {}", record.id)?;
    writeln!(out, "State: {}", record.state)?;
    if let Some(holder) = record.holder {
        writeln!(out, "Holder: {holder}")?;
    }
    writeln!(out, "Asker: {}", record.asker)?;
    writeln!(out, "Class: {} ({})", record.class, record.workspace)?;
    writeln!(out, "Record: {}", record.record)?;
    writeln!(out, "Opened: {}", timestamp(record.opened_at)?)?;
    writeln!(out, "Updated: {}", timestamp(record.updated_at)?)?;
    writeln!(out, "Blocking: {}", record.blocking)?;
    writeln!(out, "Ask: {}", record.ask)?;
    for option in record.options {
        writeln!(out, "  {}: {}", option.label, option.text)?;
    }
    writeln!(out, "Recommended by asker: {}", record.recommendation)?;
    for advice in record.advice {
        writeln!(
            out,
            "Advice from {} at {}: {}",
            advice.seat,
            timestamp(advice.at)?,
            advice.option
        )?;
        if let Some(note) = &advice.note {
            writeln!(out, "  {note}")?;
        }
    }
    if let Some(settlement) = record.settlement {
        writeln!(
            out,
            "Settled by {} at {}: {} (grant {})",
            settlement.seat,
            timestamp(settlement.at)?,
            settlement.option,
            grant(settlement)
        )?;
        if let Some(limit) = &settlement.limit {
            writeln!(out, "Limit: {}", limit.escape_debug())?;
        }
        if let Some(note) = &settlement.note {
            writeln!(out, "Note: {note}")?;
        }
    }
    Ok(())
}

fn write_rows(out: &mut impl Write, records: &[Payload<'_>], terminal: bool) -> io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    let rows: Vec<_> = records.iter().map(Payload::row).collect();
    if !terminal {
        for row in rows {
            writeln!(out, "{}", row.join("\t"))?;
        }
        return Ok(());
    }

    let mut widths = HEADER.map(str::len);
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    write_row(out, HEADER, &widths)?;
    for row in &rows {
        write_row(out, row.each_ref().map(String::as_str), &widths)?;
    }
    Ok(())
}

fn write_row(
    out: &mut impl Write,
    cells: [&str; HEADER.len()],
    widths: &[usize; HEADER.len()],
) -> io::Result<()> {
    let columns = cells.len();
    for (index, (cell, width)) in cells.into_iter().zip(widths).enumerate() {
        if index + 1 == columns {
            writeln!(out, "{cell}")?;
        } else if HEADER[index] == "GRANT" {
            write!(out, "{cell:>width$}  ")?;
        } else {
            write!(out, "{cell:width$}  ")?;
        }
    }
    Ok(())
}

fn grant(settlement: &RequestSettlement) -> String {
    settlement
        .grant_id
        .map_or_else(|| "catalog".to_owned(), |id| id.to_string())
}

fn timestamp(value: OffsetDateTime) -> io::Result<String> {
    value.format(&Rfc3339).map_err(io::Error::other)
}

fn finish_io(result: io::Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::failure(&error),
    }
}
