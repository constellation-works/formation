//! `formation watch add` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch;
use formation_core::app::watch::set;
use std::process::ExitCode;
use std::time::Duration;

/// Add a watch to the room's set, or replace the one with the same name and recipient.
#[derive(Args)]
#[command(after_help = "\
A replaced watch keeps its remembered fingerprints. One word after `--` is shell text, as
written; several words are quoted into one command line, so each stays one argument.

Examples:
  formation watch add ci --every 300 -- ./scripts/ci-read.sh owner/name main
  formation watch add release --to captain --every 60 -- ./scripts/release-read.sh")]
pub(super) struct Add {
    /// Watch name (1 to 32 lowercase ASCII letters, digits or hyphens).
    name: String,
    /// Joined seat to mail new items to (default: the calling seat).
    #[arg(long, value_name = "SEAT")]
    to: Option<String>,
    /// Seconds between reads (minimum 10).
    #[arg(
        long,
        default_value_t = watch::DEFAULT_EVERY_SECONDS,
        value_name = "SECONDS",
        value_parser = clap::value_parser!(u64).range(watch::MIN_EVERY_SECONDS..),
    )]
    every: u64,
    /// Command to run through `sh -c` from the current directory.
    #[arg(last = true, required = true, value_name = "COMMAND")]
    command: Vec<String>,
}

pub(super) fn run(env: &Env, args: Add) -> ExitCode {
    let cmd = set::Command::Add {
        name: args.name,
        to: args.to,
        every: Duration::from_secs(args.every),
        command: args.command,
    };
    set::exec(env, cmd, |result| match result {
        Ok(out) => output::watch_write(&output::watch_set_payload(&out)),
        Err(error) => output::error(&error),
    })
}
