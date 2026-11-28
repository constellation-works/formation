//! Read the caller's room status.

use clap::Args;
use formation_core::Env;
use formation_core::app::status;
use std::process::ExitCode;

/// Show a read-only overview of the room.
#[derive(Args)]
pub(crate) struct Status {
    /// Room name or directory path.
    #[arg(long, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Calling seat (defaults to the session seat or terminal admiral).
    #[arg(long = "as", value_name = "SEAT")]
    seat: Option<String>,
    /// Print JSON, overriding $FORMATION_FORMAT.
    #[arg(long)]
    json: bool,
}

pub(crate) fn run(env: Env, args: Status) -> ExitCode {
    let mode = crate::output::resolve_mode(args.json);
    let env = env.with_mail_options(args.room, args.seat);
    status::exec(&env, |result| crate::output::status(result, mode))
}
