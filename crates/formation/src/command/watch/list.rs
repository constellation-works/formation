//! `formation watch list` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch::set;
use std::process::ExitCode;

/// List every watch in the room and whether anything reads the set.
#[derive(Args)]
pub(super) struct List {
    /// Print a JSON document.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(env: &Env, args: List) -> ExitCode {
    set::exec(env, set::Command::List, |result| match result {
        Ok(set::Outcome::Listed { reader, watches }) => {
            output::watch_list(&reader, &watches, args.json)
        }
        Ok(out) => output::watch_write(&output::watch_set_payload(&out)),
        Err(error) => output::error(&error),
    })
}
