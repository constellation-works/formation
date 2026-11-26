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
    /// Validate one version-1 JSON roster on stdin (maximum 256 KiB).
    #[command(after_help = "Example: formation roster check --json < examples/roster.json")]
    Check {
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn run(env: &Env, args: Roster) -> ExitCode {
    let RosterAction::Check { json } = args.action;
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
    roster::exec(env, roster::Command::Check(bytes), |result| match result {
        Ok(summary) => output::summary(&summary, json),
        Err(error) => output::failure(&error),
    })
}
