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
        Err(error) => super::shared::failure(&error),
    }
}
