//! `formation grant load` argument mapping.

use crate::command::common::stdin;
use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::grant;
use std::process::ExitCode;

const MAX_BYTES: u64 = 256 * 1024;

/// Load the house grants file from standard input as the room's standing entries.
#[derive(Args)]
#[command(after_help = "\
Reads a version 1 JSON grants file from standard input: standing grants and withholds in the
grant's own fields, and the classes a house adds to the catalog. Loading replaces the room's
earlier house entries and leaves its day grants as they were; loading the same file again
changes nothing. A file cannot grant `grant` or `credentials`, remove a class or change a
catalog class.

Every joined seat whose effective authority changes receives one mail envelope.

Example:
  formation grant load --room g-12 < grants.json")]
pub(super) struct Load;

pub(super) fn run(env: &Env, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    match stdin::read_bounded(MAX_BYTES, "grants file") {
        Ok(bytes) => grant::exec(
            env,
            grant::Command::Load { bytes, queue },
            |result| match result {
                Ok(out) => output::grant_write(&out),
                Err(error) => output::error(&error),
            },
        ),
        Err(error) => output::failure(&error),
    }
}
