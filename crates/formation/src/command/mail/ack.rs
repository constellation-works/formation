//! `formation mail ack` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::process::ExitCode;

/// Mark a receipt in this seat's inbox as handled.
#[derive(Args)]
pub(super) struct Ack {
    /// Numeric receipt from the inbox.
    receipt: i64,
}

pub(super) fn run(env: &Env, args: Ack) -> ExitCode {
    mail::exec(env, Command::Ack(args.receipt), |result| match result {
        Ok(out) => output::mail_write(&output::mail_payload(out)),
        Err(error) => output::error(&error),
    })
}
