//! Roster validation arguments.

use super::input;
use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::roster::Command;
use std::process::ExitCode;

/// Validate one version 1 or 2 JSON roster on stdin (maximum 256 KiB).
#[derive(Args)]
#[command(after_help = "Example: formation roster check --json < examples/roster.json")]
pub(super) struct Check {
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(env: &Env, args: Check) -> ExitCode {
    input::run(env, Command::Check, output::resolve_mode(args.json))
}
