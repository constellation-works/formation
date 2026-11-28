//! `formation mail ack` argument mapping.

use crate::output::{self, Mode};
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

pub(super) fn run(env: &Env, args: Ack, mode: Mode) -> ExitCode {
    mail::exec(env, Command::Ack(args.receipt), |result| {
        output::mail_result(result, mode)
    })
}
