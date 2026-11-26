//! `formation mail sent` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::process::ExitCode;

/// List this seat's most recent sends and their delivery state.
#[derive(Args)]
pub(super) struct Sent {
    /// Maximum number of envelopes to list (positive).
    #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(i64).range(1..))]
    limit: i64,
    /// Print a JSON document.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(env: &Env, args: Sent) -> ExitCode {
    let format = if args.json {
        output::HistoryFormat::Json
    } else {
        output::HistoryFormat::Auto
    };
    mail::exec(env, Command::Sent(args.limit), |result| match result {
        Ok(mail::Outcome::SentHistory { messages, total }) => {
            output::sent_history(&messages, total, format)
        }
        Ok(out) => output::mail_write(&output::mail_payload(out)),
        Err(error) => output::error(&error),
    })
}
