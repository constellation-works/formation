//! Shared output primitives used by two or more commands.

use serde_json::Value;
use std::fmt::Display;
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn failure(error: &dyn Display) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "formation: {error}");
    ExitCode::FAILURE
}

pub(crate) fn write_json(value: &Value) -> ExitCode {
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

pub(crate) fn write_json_error(value: &Value) -> ExitCode {
    let mut err = io::stderr().lock();
    let _ = serde_json::to_writer(&mut err, value)
        .map_err(io::Error::from)
        .and_then(|()| writeln!(err));
    ExitCode::FAILURE
}

pub(crate) fn error_payload(error: &formation_core::Error) -> Value {
    serde_json::json!({
        "code": error.code(),
        "error": error.to_string(),
    })
}
