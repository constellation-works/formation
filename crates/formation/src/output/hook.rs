//! Hook command output: wire shapes and host-protocol writes.

use serde_json::{Value, json};
use std::fmt::Display;
use std::io::Write;
use std::process::ExitCode;

pub(crate) fn prompt(text: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": text,
        },
    })
}

pub(crate) fn stop(text: &str) -> Value {
    json!({"decision": "block", "reason": text})
}

pub(crate) fn hook_usage(error: &clap::Error) -> ExitCode {
    let message = error.to_string();
    hook_error(
        "hook_usage",
        message.lines().next().unwrap_or("invalid hook arguments"),
    )
}

pub(crate) fn hook_error(code: &str, message: impl Display) -> ExitCode {
    super::common::error::write_json_error(&json!({"code": code, "error": message.to_string()}))
}

pub(crate) fn hook_json(value: &Value) -> ExitCode {
    let mut out = std::io::stdout().lock();
    match serde_json::to_writer(&mut out, value)
        .map_err(std::io::Error::from)
        .and_then(|()| writeln!(out))
        .and_then(|()| out.flush())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => hook_error("hook_output", error),
    }
}

pub(crate) fn hook_rewake(text: &str) -> ExitCode {
    let result = {
        let mut out = std::io::stderr().lock();
        out.write_all(text.as_bytes()).and_then(|()| out.flush())
    };
    match result {
        Ok(()) => ExitCode::from(2),
        Err(error) => hook_error("hook_output", error),
    }
}
