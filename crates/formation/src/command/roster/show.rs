//! Roster detail arguments.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::roster::{self, Command};
use std::process::ExitCode;

/// Show a room's roster and seat presence.
#[derive(Args)]
pub(super) struct Show {
    /// Room name or directory path; defaults to the joined session room.
    room: Option<String>,
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(env: &Env, args: Show) -> ExitCode {
    let env = env.clone().with_mail_options(args.room, None);
    roster::exec(&env, Command::Show, |result| {
        output::roster_result(result, output::resolve_mode(args.json))
    })
}
