//! Status output in text and JSON.

use super::Mode;
use formation_core::app::status::View;
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn write(result: Result<View, formation_core::Error>, mode: Mode) -> ExitCode {
    let view = match result {
        Ok(view) => view,
        Err(error) => return super::mail_failure(&error, mode),
    };
    if mode == Mode::Json {
        return super::common::json::write_json(&serde_json::json!(view));
    }
    match render(&mut io::stdout().lock(), &view) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::failure(&error),
    }
}

fn render(out: &mut impl Write, view: &View) -> io::Result<()> {
    let roster = &view.roster;
    write!(out, "room {}, {}", roster.room, roster.status.as_str())?;
    if roster.roster_applied {
        write!(
            out,
            ", roster applied {}",
            roster.applied_at.as_deref().unwrap_or("unknown")
        )?;
    }
    writeln!(out)?;
    if roster.roster_applied {
        let held = roster
            .seats
            .iter()
            .filter(|seat| {
                seat.live
                    .as_ref()
                    .is_some_and(|seat| seat.held == Some(true))
            })
            .count();
        let missing = roster
            .seats
            .iter()
            .filter(|seat| seat.live.is_none())
            .map(|seat| seat.name.as_str())
            .collect::<Vec<_>>();
        let extra = roster
            .not_on_roster
            .iter()
            .map(|seat| seat.name.as_str())
            .collect::<Vec<_>>();
        write!(out, "seats     {} of {} held", held, roster.seats.len())?;
        if !missing.is_empty() {
            write!(out, "; not joined: {}", missing.join(", "))?;
        }
        if !extra.is_empty() {
            write!(out, "; not on the roster: {}", extra.join(", "))?;
        }
        writeln!(out)?;
    } else {
        let joined = roster
            .not_on_roster
            .iter()
            .map(|seat| seat.name.as_str())
            .collect::<Vec<_>>();
        writeln!(
            out,
            "seats     {} joined: {}",
            joined.len(),
            joined.join(", ")
        )?;
    }
    writeln!(
        out,
        "inbox     {} waiting on {}",
        view.inbox.len(),
        view.caller
    )?;
    let total: usize = view.requests.values().sum();
    let holders = view
        .requests
        .iter()
        .map(|(name, count)| format!("{count} with {name}"))
        .collect::<Vec<_>>();
    writeln!(
        out,
        "requests  {} open{}",
        total,
        if holders.is_empty() {
            String::new()
        } else {
            format!(": {}", holders.join(", "))
        }
    )?;
    let state = if view.reader.active.is_some() {
        "live"
    } else if view.reader.stopped {
        "stopped"
    } else {
        "not live"
    };
    writeln!(out, "watches   reader {}, {} on", state, view.watches_on)?;
    for (index, message) in view.recent.iter().enumerate() {
        writeln!(
            out,
            "{} {} {} → {}  {}",
            if index == 0 { "recent   " } else { "         " },
            message.sent.as_deref().unwrap_or("unknown"),
            message.sender,
            message.recipient,
            message.body.lines().next().unwrap_or("")
        )?;
    }
    Ok(())
}
