//! `formation mail sent` argument mapping.

use crate::output::{self, Mode};
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
}

pub(super) fn run(env: &Env, args: Sent, mode: Mode) -> ExitCode {
    mail::exec(env, Command::Sent(args.limit), |result| {
        output::mail_result(result, mode)
    })
}
