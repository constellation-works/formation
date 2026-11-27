//! `formation grant withhold` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::grant;
use std::process::ExitCode;

/// Withhold an authority class from a rank or seat for the day.
#[derive(Args)]
#[command(after_help = "\
Withholding removes a catalog default or broader grant for the named scope.
Every joined seat whose effective authority changes receives one mail envelope.

Examples:
  formation grant withhold --to captain --class land --workspace site \\
    --for day --source \"advisor chat 12:30\"")]
pub(super) struct Withhold {
    /// Rank (e.g. chief, captain) or seat name in the applied roster.
    #[arg(long, required = true, value_name = "RANK|SEAT")]
    to: String,
    /// Authority class from the catalog.
    #[arg(long, required = true, value_name = "CLASS")]
    class: String,
    /// Workspace identifier, or `all` for every workspace in the room.
    #[arg(long, default_value = grant::ALL_WORKSPACES, value_name = "WORKSPACE|all")]
    workspace: String,
    /// Optional condition in words explaining the withhold.
    #[arg(long, value_name = "TEXT")]
    limit: Option<String>,
    /// Lifetime of the withhold (`day`; standing rules come from a grants file).
    #[arg(long = "for", default_value = "day", value_name = "LIFETIME")]
    lifetime: String,
    /// Where the admiral gave this direction (required).
    #[arg(long, required = true, value_name = "TEXT")]
    source: String,
}

pub(super) fn run(env: &Env, args: Withhold, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let cmd = grant::Command::Withhold {
        to: args.to,
        class: args.class,
        workspace: args.workspace,
        limit: args.limit,
        lifetime: args.lifetime,
        source: args.source,
        queue,
    };
    grant::exec(env, cmd, |result| match result {
        Ok(out) => output::grant_write(&out),
        Err(error) => output::error(&error),
    })
}
