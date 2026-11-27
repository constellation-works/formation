//! `formation grant list` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::grant;
use std::process::ExitCode;

/// List active room grants, or what one seat settles now.
#[derive(Args)]
#[command(after_help = "\
Without `--seat`, lists every active grant and withhold recorded in the room.
With `--seat`, lists what that seat settles now and where each line came from.

Examples:
  formation grant list
  formation grant list --seat chief --json")]
pub(super) struct List {
    /// Show effective authority for one seat in the applied roster.
    #[arg(long, value_name = "SEAT")]
    seat: Option<String>,
    /// Print a JSON document.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(env: &Env, args: List) -> ExitCode {
    grant::exec(
        env,
        grant::Command::List { seat: args.seat },
        |result| match result {
            Ok(grant::Outcome::RoomList(grants)) => output::grant_room_list(&grants, args.json),
            Ok(grant::Outcome::SeatList(effective)) => {
                output::grant_seat_list(&effective, args.json)
            }
            Ok(out) => output::grant_write(&out),
            Err(error) => output::error(&error),
        },
    )
}
