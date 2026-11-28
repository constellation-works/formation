//! Room close arguments.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::roster::{self, Command};
use std::process::ExitCode;

/// Record the room's close as the joined seat.
#[derive(Args)]
pub(super) struct Close {
    /// Room name or directory path. Defaults to the joined session room.
    #[arg(long)]
    room: Option<String>,
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(env: &Env, args: Close) -> ExitCode {
    let env = env.clone().with_mail_options(args.room, None);
    roster::exec(&env, Command::Close, |result| {
        output::roster_result(result, output::resolve_mode(args.json))
    })
}
