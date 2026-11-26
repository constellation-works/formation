//! `formation mail inbox` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::process::ExitCode;
use std::time::Duration;

/// Read unread messages or wait for up to 900 seconds.
#[derive(Args)]
pub(super) struct Inbox {
    /// Seconds to wait for the first unread message.
    #[arg(long, default_value_t=0, value_parser=clap::value_parser!(u64).range(0..=900))]
    wait: u64,
}

pub(super) fn run(env: &Env, args: Inbox) -> ExitCode {
    mail::exec(
        env,
        Command::Inbox(Duration::from_secs(args.wait)),
        |result| match result {
            Ok(out) => output::mail_write(&output::mail_payload(out)),
            Err(error) => output::error(&error),
        },
    )
}
