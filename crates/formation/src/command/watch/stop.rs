//! `formation watch stop` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch::set;
use std::process::ExitCode;

/// Stop the room's reader and all its standbys.
#[derive(Args)]
pub(super) struct Stop;

pub(super) fn run(env: &Env) -> ExitCode {
    set::exec(env, set::Command::Stop, |result| match result {
        Ok(out) => output::watch_write(&output::watch_set_payload(&out)),
        Err(error) => output::error(&error),
    })
}
