//! Mail command output: wire shapes and human views.

use super::common::Mode;
use formation_core::app::mail::{LogMessage, Outcome, SentMessage};
use serde_json::{Value, json};
use std::io::{self, Write};
use std::process::ExitCode;

/// Render one mail result in the resolved mode.
pub(crate) fn result(result: Result<Outcome, formation_core::Error>, mode: Mode) -> ExitCode {
    match result {
        Ok(Outcome::SentHistory { messages, total }) => sent_history(&messages, total, mode),
        Ok(out) if mode == Mode::Human => human(&out),
        Ok(out) => super::common::json::write_json(&payload(&out)),
        Err(error) => failure(&error, mode),
    }
}

/// Render a mail failure on stderr: one plain line for a person, one JSON object otherwise.
pub(crate) fn failure(error: &formation_core::Error, mode: Mode) -> ExitCode {
    if mode == Mode::Human {
        super::common::error::plain_error(error)
    } else {
        super::common::error::error(error)
    }
}

fn payload(out: &Outcome) -> Value {
    match out {
        Outcome::Joined {
            seat,
            replaced_session,
            seat_dir,
        } => json!({
            "seat": seat,
            "rebound": replaced_session.is_some(),
            "replaced_session": replaced_session,
            "seat_dir": seat_dir,
        }),
        Outcome::Seats(seats) => json!({"seats": seats
            .iter()
            .map(|seat| json!({
                "name": seat.name,
                "undelivered": seat.undelivered,
                "unacknowledged": seat.unacknowledged,
                "held": seat.held,
                "seen": seat.seen,
            }))
            .collect::<Vec<_>>()}),
        Outcome::Sent { id, receipt, held } => {
            json!({"id": id, "receipt": receipt, "held": held})
        }
        Outcome::Inbox(messages) => json!({"messages": messages}),
        Outcome::SentHistory { messages, total } => sent_history_payload(messages, *total),
        Outcome::Log { messages, .. } => json!({"messages": messages}),
        Outcome::Show(message) => json!({"message": message}),
        Outcome::Ack(receipt) => json!({"acknowledged": receipt}),
    }
}

fn human(out: &Outcome) -> ExitCode {
    let empty = match out {
        Outcome::Seats(seats) => seats.is_empty().then_some("No joined seats."),
        Outcome::Inbox(messages) => messages.is_empty().then_some("No unacknowledged mail."),
        Outcome::Joined { .. }
        | Outcome::Sent { .. }
        | Outcome::SentHistory { .. }
        | Outcome::Log { .. }
        | Outcome::Show(_)
        | Outcome::Ack(_) => None,
    };
    if let Some(notice) = empty {
        let _ = writeln!(io::stderr().lock(), "{notice}");
    }
    let mut stdout = io::stdout().lock();
    finish(write_human(&mut stdout, out))
}

fn finish(result: io::Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::common::error::failure(&error),
    }
}

fn write_human(out: &mut impl Write, outcome: &Outcome) -> io::Result<()> {
    match outcome {
        Outcome::Joined {
            seat,
            replaced_session,
            seat_dir,
        } => {
            match replaced_session {
                Some(session) => writeln!(out, "Rebound seat {seat} (replaced session {session})")?,
                None => writeln!(out, "Joined as {seat}")?,
            }
            writeln!(out, "Seat folder: {}", seat_dir.display())
        }
        Outcome::Seats(seats) => {
            let rows: Vec<[String; 5]> = seats
                .iter()
                .map(|seat| {
                    [
                        seat.name.clone(),
                        seat.undelivered.to_string(),
                        seat.unacknowledged.to_string(),
                        match seat.held {
                            Some(true) => "yes",
                            Some(false) => "no",
                            None => "-",
                        }
                        .to_owned(),
                        seat.seen.clone().unwrap_or_else(|| "-".to_owned()),
                    ]
                })
                .collect();
            write_table(
                out,
                ["SEAT", "UNDELIVERED", "UNACKNOWLEDGED", "HELD", "SEEN"],
                &rows,
            )
        }
        Outcome::Sent { id, receipt, held } => writeln!(
            out,
            "Sent {id} as receipt {receipt}; recipient is {}",
            if *held { "present" } else { "not present" },
        ),
        Outcome::Inbox(messages) => {
            for (index, message) in messages.iter().enumerate() {
                if index > 0 {
                    writeln!(out)?;
                }
                writeln!(
                    out,
                    "Receipt {} from {}{} (id {})",
                    message.receipt,
                    message.sender,
                    if message.terminal {
                        ", sent from a terminal"
                    } else {
                        ""
                    },
                    message.id,
                )?;
                writeln!(
                    out,
                    "Sent {}, delivered {}",
                    message.sent.as_deref().unwrap_or("-"),
                    message.delivered.as_deref().unwrap_or("-"),
                )?;
                if let Some(name) = &message.watch_name {
                    let added_by = message.watch_added_by.as_deref().unwrap_or("-");
                    writeln!(out, "Watch {name}, added by {added_by}")?;
                }
                for line in message.body.lines() {
                    writeln!(out, "  {line}")?;
                }
            }
            Ok(())
        }
        Outcome::SentHistory { messages, .. } => write_sent_history(out, messages, true),
        Outcome::Log { messages, full } => {
            let rows: Vec<[String; 7]> = messages
                .iter()
                .map(|message| {
                    [
                        message.sent.clone().unwrap_or_else(|| "-".to_owned()),
                        match &message.watch_name {
                            Some(name) => format!("{} (watch {})", message.sender, name),
                            None => message.sender.clone(),
                        },
                        message.recipient.clone(),
                        message.state.as_str().to_owned(),
                        message.receipt.to_string(),
                        message.id.clone(),
                        message.body.lines().next().unwrap_or("").to_owned(),
                    ]
                })
                .collect();
            write_table(
                out,
                ["TIME", "FROM", "TO", "STATE", "RECEIPT", "ID", "FIRST LINE"],
                &rows,
            )?;
            if *full {
                for message in messages {
                    writeln!(out, "\nReceipt {}:\n{}", message.receipt, message.body)?;
                }
            }
            Ok(())
        }
        Outcome::Show(message) => write_log_message(out, message),
        Outcome::Ack(receipt) => writeln!(out, "Acknowledged receipt {receipt}"),
    }
}

/// Write a header and rows with columns padded to their widest cell; the last is left unpadded.
fn write_table<const N: usize>(
    out: &mut impl Write,
    header: [&str; N],
    rows: &[[String; N]],
) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let widths: [usize; N] = std::array::from_fn(|column| {
        rows.iter()
            .map(|row| row[column].chars().count())
            .max()
            .unwrap_or(0)
            .max(header[column].len())
    });
    let line = |cells: Vec<&str>| {
        let last = cells.len() - 1;
        let text: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(column, cell)| {
                if column == last {
                    (*cell).to_owned()
                } else {
                    format!("{cell:<width$}", width = widths[column])
                }
            })
            .collect();
        text.join("  ")
    };
    writeln!(out, "{}", line(header.to_vec()))?;
    for row in rows {
        writeln!(out, "{}", line(row.iter().map(String::as_str).collect()))?;
    }
    Ok(())
}

fn sent_history(messages: &[SentMessage], total: i64, mode: Mode) -> ExitCode {
    if messages.is_empty() {
        let _ = writeln!(io::stderr().lock(), "No sent mail for this seat.");
    }
    if mode == Mode::Json {
        return super::common::json::write_json(&sent_history_payload(messages, total));
    }
    let mut stdout = io::stdout().lock();
    let result = write_sent_history(&mut stdout, messages, mode == Mode::Human);
    if result.is_ok() && is_truncated(messages.len(), total) {
        let _ = writeln!(
            io::stderr().lock(),
            "Showing {} of {total} sent messages.",
            messages.len(),
        );
    }
    finish(result)
}

fn sent_history_payload(messages: &[SentMessage], total: i64) -> Value {
    json!({
        "messages": messages,
        "total": total,
        "truncated": is_truncated(messages.len(), total),
    })
}

fn is_truncated(count: usize, total: i64) -> bool {
    i64::try_from(count).is_ok_and(|count| count < total)
}

fn write_sent_history(
    out: &mut impl Write,
    messages: &[SentMessage],
    terminal: bool,
) -> io::Result<()> {
    if terminal && !messages.is_empty() {
        let receipt_width = messages
            .iter()
            .map(|message| message.receipt.to_string().len())
            .max()
            .unwrap_or(0)
            .max(7);
        let id_width = messages
            .iter()
            .map(|message| message.id.len())
            .max()
            .unwrap_or(0)
            .max(2);
        let recipient_width = messages
            .iter()
            .map(|message| message.recipient.len())
            .max()
            .unwrap_or(0)
            .max(9);
        writeln!(
            out,
            "{:>receipt_width$}  {:id_width$}  {:recipient_width$}  {:20}  STATE",
            "RECEIPT", "ID", "RECIPIENT", "SENT",
        )?;
        for message in messages {
            writeln!(
                out,
                "{:>receipt_width$}  {:id_width$}  {:recipient_width$}  {:20}  {}",
                message.receipt,
                message.id,
                message.recipient,
                message.sent.as_deref().unwrap_or("-"),
                message.state.as_str(),
            )?;
        }
    } else {
        for message in messages {
            writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}",
                message.receipt,
                message.id,
                message.recipient,
                message.sent.as_deref().unwrap_or("-"),
                message.state.as_str(),
            )?;
        }
    }
    Ok(())
}

fn write_log_message(out: &mut impl Write, message: &LogMessage) -> io::Result<()> {
    writeln!(out, "Receipt {} (id {})", message.receipt, message.id)?;
    writeln!(out, "From {} to {}", message.sender, message.recipient)?;
    writeln!(
        out,
        "Sent {}, delivered {}, state {}",
        message.sent.as_deref().unwrap_or("-"),
        message.delivered.as_deref().unwrap_or("-"),
        message.state.as_str()
    )?;
    if message.terminal {
        writeln!(out, "Sent from a terminal")?;
    }
    if let Some(name) = &message.watch_name {
        writeln!(
            out,
            "Watch {}, added by {}",
            name,
            message.watch_added_by.as_deref().unwrap_or("-")
        )?;
    }
    writeln!(out, "{}", message.body)
}
