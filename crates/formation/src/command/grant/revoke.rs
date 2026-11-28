//! `formation grant revoke` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::grant;
use std::process::ExitCode;

/// Revoke an active grant or withhold by identifier.
#[derive(Args)]
#[command(after_help = "\
Every joined seat whose effective authority changes receives one mail envelope.

Example:
  formation grant revoke 1")]
pub(super) struct Revoke {
    /// Grant identifier printed by `grant add`, `grant withhold` or `grant list`.
    #[arg(value_name = "GRANT", value_parser = clap::value_parser!(i64).range(1..))]
    grant: i64,
}

pub(super) fn run(env: &Env, args: Revoke, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let cmd = grant::Command::Revoke {
        id: args.grant,
        queue,
    };
    grant::exec(env, cmd, |result| match result {
        Ok(out) => output::grant_write(&out),
        Err(error) => output::error(&error),
    })
}
