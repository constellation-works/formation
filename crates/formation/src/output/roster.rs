//! Roster command output: wire shape and human view.

use formation_core::Summary;
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn summary(summary: &Summary, json: bool) -> ExitCode {
    let mut out = io::stdout().lock();
    let result = if json {
        serde_json::to_writer(&mut out, summary)
            .map_err(io::Error::from)
            .and_then(|()| writeln!(out))
    } else {
        writeln!(
            out,
            "{} seats, {} directed routes",
            summary.seats, summary.routes
        )
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::common::error::failure(&error),
    }
}

/// Render a room roster.
pub(crate) fn view(view: &formation_core::app::roster::View, json: bool) -> ExitCode {
    if json {
        return super::common::json::write_json(&serde_json::json!(view));
    }
    let mut out = io::stdout().lock();
    let result = write_view(&mut out, view);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::common::error::failure(&error),
    }
}

fn write_view(out: &mut impl Write, view: &formation_core::app::roster::View) -> io::Result<()> {
    if view.roster_applied {
        writeln!(
            out,
            "room {}, roster applied {} by {}",
            view.room,
            view.applied_at.as_deref().unwrap_or("unknown"),
            view.applied_by.as_deref().unwrap_or("unknown")
        )?;
        writeln!(out, "admiral")?;
        children(out, &view.seats, None, "")?;
    } else {
        writeln!(out, "room {}, no roster applied", view.room)?;
    }
    for seat in &view.not_on_roster {
        if view.roster_applied {
            write!(out, "not on the roster: ")?;
        }
        writeln!(out, "{} ({})", seat.name, state(seat))?;
    }
    Ok(())
}

fn children(
    out: &mut impl Write,
    seats: &[formation_core::app::roster::ViewSeat],
    parent: Option<&str>,
    indent: &str,
) -> io::Result<()> {
    let matches: Vec<_> = seats
        .iter()
        .filter(|seat| seat.reports_to.as_deref() == parent)
        .collect();
    for (index, seat) in matches.iter().enumerate() {
        let last = index + 1 == matches.len();
        let live = seat
            .live
            .as_ref()
            .map_or_else(|| "not joined".to_owned(), state);
        writeln!(
            out,
            "{}{} {}  {}  {}  {}  {}  {}  {}",
            indent,
            if last { "└─" } else { "├─" },
            seat.name,
            seat.role,
            seat.workspace.as_deref().unwrap_or("—"),
            seat.model.as_deref().unwrap_or("—"),
            seat.effort.as_deref().unwrap_or("—"),
            seat.record.as_deref().unwrap_or("—"),
            live
        )?;
        children(
            out,
            seats,
            Some(&seat.name),
            &format!("{}{}", indent, if last { "   " } else { "│  " }),
        )?;
    }
    Ok(())
}

fn state(seat: &formation_core::app::mail::SeatStatus) -> String {
    if seat.held {
        format!("held, {} unread", seat.undelivered)
    } else {
        format!(
            "seen {}, {} unread",
            seat.seen.as_deref().unwrap_or("never"),
            seat.undelivered
        )
    }
}
