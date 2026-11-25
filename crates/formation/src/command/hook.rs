//! Decode host inputs and map the delivery outcome to the host protocol.

use crate::output;
use clap::{Args, ValueEnum};
use formation_core::Env;
use formation_core::app::hook::{self, Command, Event, Session};
use serde_json::Value;
use std::io::{self, Read};
use std::process::ExitCode;
use std::time::Duration;

const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Args)]
#[command(after_help = concat!(
    "Reads the host's JSON from stdin (maximum 1 MiB). ",
    "A Claude Stop delivery exits 2 with the mail on stderr. Errors exit 1.\n\n",
    "Examples:\n  formation hook --provider claude --event user-prompt\n",
    "  formation hook --provider codex --event stop --hold 30"
))]
pub(crate) struct Hook {
    /// Host that supplies the hook input.
    #[arg(long, value_enum)]
    provider: Provider,
    /// Lifecycle event that invoked this command.
    #[arg(long, value_enum)]
    event: HookEvent,
    /// Seconds a Stop hook may wait for mail (nonnegative).
    #[arg(long, default_value_t = 0, value_name = "SECONDS")]
    hold: u64,
}

#[derive(Clone, Copy, ValueEnum)]
enum Provider {
    Claude,
    Codex,
}

#[derive(Clone, Copy, ValueEnum)]
enum HookEvent {
    UserPrompt,
    Stop,
}

pub(crate) fn run(env: &Env, args: Hook) -> ExitCode {
    let event = match args.event {
        HookEvent::UserPrompt if args.hold > 0 => {
            return output::hook_error("hook_usage", "--hold is only for --event stop");
        }
        HookEvent::UserPrompt => Event::UserPrompt,
        HookEvent::Stop => Event::Stop(Duration::from_secs(args.hold)),
    };
    let mut bytes = Vec::new();
    if let Err(error) = io::stdin()
        .lock()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
    {
        return output::hook_error("hook_stdin", format!("cannot read hook stdin: {error}"));
    }
    if bytes.len() as u64 > MAX_BYTES {
        return output::hook_error("hook_input_too_large", "hook stdin exceeds 1 MiB");
    }

    let input = match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(input)) => input,
        Ok(_) => return output::hook_error("hook_input", "hook stdin must be a JSON object"),
        Err(error) => {
            return output::hook_error("hook_input", format!("invalid hook JSON: {error}"));
        }
    };
    let session = if input.contains_key("agent_id") {
        Session::Agent
    } else if let Some(session) = input.get("session_id").and_then(Value::as_str) {
        Session::Host(session.to_owned())
    } else {
        return output::hook_error("hook_input", "hook JSON requires a string session_id");
    };
    let provider = match args.provider {
        Provider::Claude => hook::Provider::Claude,
        Provider::Codex => hook::Provider::Codex,
    };
    hook::exec(
        env,
        Command {
            provider,
            event,
            session,
        },
        |result| match result {
            Ok(None) => ExitCode::SUCCESS,
            Ok(Some(text)) => match (args.event, args.provider) {
                (HookEvent::UserPrompt, _) => output::hook_json(&output::payload::prompt(&text)),
                (HookEvent::Stop, Provider::Claude) => output::hook_rewake(&text),
                (HookEvent::Stop, Provider::Codex) => {
                    output::hook_json(&output::payload::stop(&text))
                }
            },
            Err(error) => output::error(&error),
        },
    )
}
