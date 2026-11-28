//! Shared error formatting and stderr output for Formation commands.

use serde_json::Value;
use std::fmt::Display;
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn error(error: &formation_core::Error) -> ExitCode {
    write_json_error(&error_payload(error))
}

pub(crate) fn failure(error: &dyn Display) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "formation: {error}");
    ExitCode::FAILURE
}

pub(crate) fn write_json_error(value: &Value) -> ExitCode {
    let mut err = io::stderr().lock();
    let _ = serde_json::to_writer(&mut err, value)
        .map_err(io::Error::from)
        .and_then(|()| writeln!(err));
    ExitCode::FAILURE
}

fn error_payload(error: &formation_core::Error) -> Value {
    serde_json::json!({
        "code": error.code(),
        "error": error.to_string(),
    })
}

/// Print `error: <message>` for a person reading the terminal.
pub(crate) fn plain_error(error: &formation_core::Error) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "error: {error}");
    ExitCode::FAILURE
}
