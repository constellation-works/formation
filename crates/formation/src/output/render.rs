//! Owns the command's output streams and exit status.

use formation_core::Summary;
use std::fmt::Display;
use std::io::{self, Write};
use std::process::ExitCode;

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
