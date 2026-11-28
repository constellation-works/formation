//! Roster application arguments.

use super::input;
use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::roster::Command;
use std::process::ExitCode;

/// Apply a version 2 JSON roster to an existing room.
#[derive(Args)]
pub(super) struct Apply {
    /// Room name or directory path.
    #[arg(long, required = true)]
    room: String,
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(env: &Env, args: Apply) -> ExitCode {
    let env = env.clone().with_mail_options(Some(args.room), None);
    input::run(&env, Command::Apply, output::resolve_mode(args.json))
}
