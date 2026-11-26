//! `formation watch check` argument mapping.

use super::probe::ShellProbeRunner;
use crate::command::common::probe::SignalGuard;
use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch::check;
use std::process::ExitCode;

/// Run a watch command once and print its items without changing the room.
#[derive(Args)]
#[command(after_help = "\
Runs the command once through `sh -c` from the current directory, under the reader's 60-second
limit. It prints the items as JSON and changes nothing: no watch is added, the reader's lease is
not taken, no fingerprint is remembered and no mail is sent. A malformed line, a failed run and a
run past the limit exit 1 with the reason on stderr.

Examples:
  formation watch check -- ./scripts/ci-read.sh owner/name main")]
pub(super) struct Check {
    /// Command to run through `sh -c` from the current directory.
    #[arg(last = true, required = true, value_name = "COMMAND")]
    command: Vec<String>,
}

pub(super) fn run(env: &Env, args: Check) -> ExitCode {
    let _signals = SignalGuard::install();
    let runner = ShellProbeRunner;
    check::exec(
        env,
        check::Command {
            words: args.command,
            runner: &runner,
        },
        |result| match result {
            Ok(out) => output::watch_checked(&out),
            Err(error) => output::error(&error),
        },
    )
}
