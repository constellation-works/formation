//! Read one envelope.

use crate::output::{self, Mode};
use clap::Args;
use formation_core::Env;
use formation_core::app::mail::{self, Command};
use std::process::ExitCode;

/// Show one envelope in full.
#[derive(Args)]
pub(super) struct Show {
    /// Durable receipt number.
    receipt: i64,
}

pub(super) fn run(env: &Env, args: Show, mode: Mode) -> ExitCode {
    mail::exec(env, Command::Show(args.receipt), |result| {
        output::mail_result(result, mode)
    })
}
