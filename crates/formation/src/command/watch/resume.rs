//! `formation watch resume` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch::set;
use std::process::ExitCode;

/// Read a paused watch again.
#[derive(Args)]
pub(super) struct Resume {
    /// Watch name.
    name: String,
    /// Seat the watch mails (default: the calling seat).
    #[arg(long, value_name = "SEAT")]
    to: Option<String>,
}

pub(super) fn run(env: &Env, args: Resume) -> ExitCode {
    let cmd = set::Command::Resume {
        name: args.name,
        to: args.to,
    };
    set::exec(env, cmd, |result| match result {
        Ok(out) => output::watch_write(&output::watch_set_payload(&out)),
        Err(error) => output::error(&error),
    })
}
