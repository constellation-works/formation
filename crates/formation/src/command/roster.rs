//! Read the bounded wire document and call the offline app.

use crate::output;
use clap::{Args, Subcommand};
use formation_core::Env;
use formation_core::app::roster;
use std::io::{self, Read};
use std::process::ExitCode;

const MAX_BYTES: u64 = 256 * 1024;

#[derive(Args)]
pub(crate) struct Roster {
    #[command(subcommand)]
    action: RosterAction,
}

#[derive(Subcommand)]
enum RosterAction {
    /// Validate one version 1 or 2 JSON roster on stdin (maximum 256 KiB).
    #[command(after_help = "Example: formation roster check --json < examples/roster.json")]
    Check {
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show the roster and live seats in a room.
    Show {
        /// Room name or directory path; defaults to the joined session room.
        room: Option<String>,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Apply a version 2 JSON roster to an existing room.
    Apply {
        /// Room name or directory path.
        #[arg(long, required = true)]
        room: String,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn run(env: &Env, args: Roster) -> ExitCode {
    if let RosterAction::Show { ref room, json } = args.action {
        let env = env.clone().with_mail_options(room.clone(), None);
        return roster::exec(&env, roster::Command::Show, |result| match result {
            Ok(roster::Outcome::View(view)) => output::roster_view(&view, json),
            Ok(roster::Outcome::Summary(_)) => unreachable!(),
            Err(error) => output::error(&error),
        });
    }
    let (json, room) = match args.action {
        RosterAction::Check { json } => (json, None),
        RosterAction::Apply { json, room } => (json, Some(room)),
        RosterAction::Show { .. } => unreachable!(),
    };
    let mut bytes = Vec::new();
    if let Err(error) = io::stdin()
        .lock()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
    {
        return output::failure(&error);
    }
    if bytes.len() as u64 > MAX_BYTES {
        return output::failure(&"roster exceeds 256 KiB");
    }
    let apply = room.is_some();
    let command = if apply {
        roster::Command::Apply(bytes)
    } else {
        roster::Command::Check(bytes)
    };
    let env = if let Some(room) = room {
        // Capture the same process environment with the explicit room override.
        env.clone().with_mail_options(Some(room), None)
    } else {
        return roster::exec(env, command, |result| match result {
            Ok(roster::Outcome::Summary(summary)) => output::summary(&summary, json),
            Ok(roster::Outcome::View(_)) => unreachable!(),
            Err(error) => output::error(&error),
        });
    };
    roster::exec(&env, command, |result| match result {
        Ok(roster::Outcome::Summary(summary)) => output::summary(&summary, json),
        Ok(roster::Outcome::View(_)) => unreachable!(),
        Err(error) => output::error(&error),
    })
}
