//! Owns the command's output streams and exit status.

use formation_core::Summary;
use formation_core::app::mail::SentMessage;
use serde_json::{Value, json};
use std::fmt::Display;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

pub(crate) enum HistoryFormat {
    Auto,
    Json,
}

pub(crate) fn failure(error: &dyn Display) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "formation: {error}");
    ExitCode::FAILURE
}

pub(crate) fn mail_error(value: &serde_json::Value) -> ExitCode {
    let mut err = io::stderr().lock();
    let _ = serde_json::to_writer(&mut err, value)
        .map_err(io::Error::from)
        .and_then(|()| writeln!(err));
    ExitCode::FAILURE
}

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
        Err(error) => failure(&error),
    }
}

pub(crate) fn mail(value: &serde_json::Value) -> ExitCode {
    let mut out = io::stdout().lock();
    match serde_json::to_writer(&mut out, value)
        .map_err(io::Error::from)
        .and_then(|()| writeln!(out))
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => failure(&error),
    }
}

pub(crate) fn hook_usage(error: &clap::Error) -> ExitCode {
    let message = error.to_string();
    hook_error(
        "hook_usage",
        message.lines().next().unwrap_or("invalid hook arguments"),
    )
}

pub(crate) fn hook_error(code: &str, message: impl Display) -> ExitCode {
    mail_error(&json!({"code": code, "error": message.to_string()}))
}

pub(crate) fn hook_json(value: &Value) -> ExitCode {
    let mut out = io::stdout().lock();
    match serde_json::to_writer(&mut out, value)
        .map_err(io::Error::from)
        .and_then(|()| writeln!(out))
        .and_then(|()| out.flush())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => hook_error("hook_output", error),
    }
}

pub(crate) fn hook_rewake(text: &str) -> ExitCode {
    let result = {
        let mut out = io::stderr().lock();
        out.write_all(text.as_bytes()).and_then(|()| out.flush())
    };
    match result {
        Ok(()) => ExitCode::from(2),
        Err(error) => hook_error("hook_output", error),
    }
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
        return mail(&super::payload::sent_history(messages, total));
    }
    let stdout = io::stdout();
    let terminal = stdout.is_terminal();
    let mut out = stdout.lock();
    let result = write_sent_history(&mut out, messages, terminal);
    match result {
        Ok(()) => {
            if super::payload::is_truncated(messages.len(), total) {
                let _ = writeln!(
                    io::stderr().lock(),
                    "Showing {} of {total} sent messages.",
                    messages.len(),
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => failure(&error),
    }
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
