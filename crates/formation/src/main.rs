//! Formation command-line entry point.

use clap::{Args, Parser, Subcommand};
use std::io::{self, Read};
use std::process::ExitCode;

mod output;

/// Check a proposed roster for desktop and headless agent sessions.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect a shift roster.
    Roster(Roster),
}

#[derive(Args)]
struct Roster {
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

const MAX_BYTES: u64 = 256 * 1024;

fn main() -> ExitCode {
    let Command::Roster(Roster {
        action: RosterAction::Check { json },
    }) = Cli::parse().command;
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
    match formation_core::check(&bytes) {
        Ok(summary) => output::summary(&summary, json),
        Err(error) => output::failure(&error),
    }
}
