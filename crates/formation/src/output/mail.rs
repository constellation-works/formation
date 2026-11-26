//! Mail command output: wire shapes and human views.

use formation_core::app::mail::{Outcome, SentMessage};
use serde_json::{Value, json};
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

pub(crate) enum HistoryFormat {
    Auto,
    Json,
}

pub(crate) fn payload(out: Outcome) -> Value {
    match out {
        Outcome::Joined {
            seat,
            replaced_session,
        } => json!({
            "seat": seat,
            "rebound": replaced_session.is_some(),
            "replaced_session": replaced_session,
        }),
        Outcome::Seats(seats) => json!({"seats": seats
            .into_iter()
            .map(|seat| json!({
                "name": seat.name,
                "unread": seat.unread,
                "held": seat.held,
                "seen": seat.seen,
            }))
            .collect::<Vec<_>>()}),
        Outcome::Sent { id, receipt, held } => {
            json!({"id": id, "receipt": receipt, "held": held})
        }
        Outcome::Inbox(messages) => json!({"messages": messages}),
        Outcome::SentHistory { messages, total } => sent_history_payload(&messages, total),
        Outcome::Ack(receipt) => json!({"acknowledged": receipt}),
    }
}

pub(crate) fn write(value: &Value) -> ExitCode {
    super::common::json::write_json(value)
}

pub(crate) fn sent_history(
    messages: &[SentMessage],
    total: i64,
    format: HistoryFormat,
) -> ExitCode {
    if messages.is_empty() {
        let _ = writeln!(io::stderr().lock(), "No sent mail for this seat.");
    }
    if matches!(format, HistoryFormat::Json) {
        return super::common::json::write_json(&sent_history_payload(messages, total));
    }
    let stdout = io::stdout();
    let terminal = stdout.is_terminal();
    let mut out = stdout.lock();
    let result = write_sent_history(&mut out, messages, terminal);
    match result {
        Ok(()) => {
            if is_truncated(messages.len(), total) {
                let _ = writeln!(
                    io::stderr().lock(),
                    "Showing {} of {total} sent messages.",
                    messages.len(),
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => super::common::error::failure(&error),
    }
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
