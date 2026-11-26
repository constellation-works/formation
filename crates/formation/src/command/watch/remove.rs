//! `formation watch remove` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch::set;
use std::process::ExitCode;

/// Drop a watch and forget its remembered fingerprints.
#[derive(Args)]
pub(super) struct Remove {
    /// Watch name.
    name: String,
    /// Seat the watch mails (default: the calling seat).
    #[arg(long, value_name = "SEAT")]
    to: Option<String>,
}

pub(super) fn run(env: &Env, args: Remove) -> ExitCode {
    let cmd = set::Command::Remove {
        name: args.name,
        to: args.to,
    };
    set::exec(env, cmd, |result| match result {
        Ok(out) => output::watch_write(&output::watch_set_payload(&out)),
        Err(error) => output::error(&error),
    })
}
