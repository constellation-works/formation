//! Roster command output: wire shape and human view.

use super::Mode;
use formation_core::Summary;
use formation_core::app::roster::{Close, Listing, Outcome, Room, Status, View, ViewSeat};
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn result(result: Result<Outcome, formation_core::Error>, mode: Mode) -> ExitCode {
    match result {
        Ok(Outcome::Summary(summary)) => summary_output(&summary, mode),
        Ok(Outcome::View(view)) => view_output(&view, mode),
        Ok(Outcome::Listed(list)) => list_output(&list, mode),
        Ok(Outcome::Closed(close)) => close_output(&close, mode),
        Err(error) => super::common::error::error(&error),
    }
}

fn summary_output(summary: &Summary, mode: Mode) -> ExitCode {
    let mut out = io::stdout().lock();
    let result = if mode == Mode::Json {
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
    finished(result)
}

/// Render a room roster.
fn view_output(view: &View, mode: Mode) -> ExitCode {
    if mode == Mode::Json {
        return super::common::json::write_json(&serde_json::json!(view));
    }
    finished(write_view(&mut io::stdout().lock(), view))
}

fn close_output(close: &Close, mode: Mode) -> ExitCode {
    if mode == Mode::Json {
        return super::common::json::write_json(&serde_json::json!(close));
    }
    finished(writeln!(
        io::stdout().lock(),
        "room {} closed {} by {}",
        close.room,
        close.closed_at,
        close.closed_by
    ))
}

fn list_output(list: &Listing, mode: Mode) -> ExitCode {
    {
        let mut stderr = io::stderr().lock();
        for failure in &list.unreadable {
            let room = failure.room.escape_debug();
            let message = failure.error.to_string();
            let message = message.lines().collect::<Vec<_>>().join(" ");
            let _ = writeln!(stderr, "formation: could not read room {room}: {message}");
        }
        if list.rooms.is_empty() {
            let status = list.filter.map_or("", |status| match status {
                Status::Active => " active",
                Status::Closed => " closed",
            });
            let _ = writeln!(stderr, "No{status} rooms found in the Formation home.");
        }
    }
    if mode == Mode::Json {
        return super::common::json::write_json(&serde_json::json!(list.rooms));
    }
    if list.rooms.is_empty() {
        return ExitCode::SUCCESS;
    }
    finished(write_list(&mut io::stdout().lock(), &list.rooms))
}

fn finished(result: io::Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::common::error::failure(&error),
    }
}

fn write_list(out: &mut impl Write, rooms: &[Room]) -> io::Result<()> {
    const HEADER: [&str; 8] = [
        "ROOM",
        "STATUS",
        "OPENED",
        "CLOSED",
        "CLOSED BY",
        "SEATS",
        "WORKSPACES",
        "LAST SEEN",
    ];
    let rows: Vec<_> = rooms.iter().map(room_cells).collect();
    let widths: [usize; 8] = std::array::from_fn(|column| {
        rows.iter().fold(HEADER[column].len(), |width, row| {
            width.max(row[column].chars().count())
        })
    });
    let header = HEADER.map(str::to_owned);
    for row in std::iter::once(&header).chain(&rows) {
        for (column, cell) in row.iter().enumerate() {
            let width = widths[column];
            match column {
                5 => write!(out, "{cell:>width$}  ")?,
                7 => writeln!(out, "{cell}")?,
                _ => write!(out, "{cell:<width$}  ")?,
            }
        }
    }
    Ok(())
}

fn room_cells(room: &Room) -> [String; 8] {
    let seats = match room.seats.planned {
        Some(planned) => format!("{}/{planned} held", room.seats.held),
        None => format!("{} joined (no roster)", room.seats.joined),
    };
    let workspaces = if room.workspaces.is_empty() {
        "-".to_owned()
    } else {
        room.workspaces.join(", ").escape_debug().to_string()
    };
    [
        room.room.escape_debug().to_string(),
        room.status.as_str().to_owned(),
        room.opened_at.as_deref().unwrap_or("-").to_owned(),
        room.closed_at.as_deref().unwrap_or("-").to_owned(),
        room.closed_by.as_deref().unwrap_or("-").to_owned(),
        seats,
        workspaces,
        room.last_seen.as_deref().unwrap_or("-").to_owned(),
    ]
}

fn write_view(out: &mut impl Write, view: &View) -> io::Result<()> {
    write!(out, "room {}, {}", view.room, view.status.as_str())?;
    if view.status == Status::Closed {
        write!(
            out,
            " {} by {}",
            view.closed_at.as_deref().unwrap_or("unknown"),
            view.closed_by.as_deref().unwrap_or("unknown")
        )?;
    }
    if view.roster_applied {
        writeln!(
            out,
            ", roster applied {} by {}",
            view.applied_at.as_deref().unwrap_or("unknown"),
            view.applied_by.as_deref().unwrap_or("unknown")
        )?;
        writeln!(out, "admiral")?;
        children(out, &view.seats, None, "")?;
    } else {
        writeln!(out, ", no roster applied")?;
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
    seats: &[ViewSeat],
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
