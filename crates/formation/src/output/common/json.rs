//! Shared JSON stdout serialization for Formation commands.

use super::error::failure;
use serde_json::Value;
use std::io::{self, Write};
use std::process::ExitCode;

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
