//! `formation grant add` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::grant;
use std::process::ExitCode;

/// Grant an authority class to a rank or seat for the day.
#[derive(Args)]
#[command(after_help = "\
Classes `grant` and `credentials` stay with the admiral and are never granted.
Every joined seat whose effective authority changes receives one mail envelope.

Examples:
  formation grant add --to chief --class direction --workspace all \\
    --limit \"reversible workspace settings\" --for day --source \"advisor chat 12:00\"
  formation grant add --to orbit-captain --class operator --workspace orbit \\
    --for day --source \"chief chat 13:15\"")]
pub(super) struct Add {
    /// Rank (e.g. chief, captain) or seat name in the applied roster.
    #[arg(long, required = true, value_name = "RANK|SEAT")]
    to: String,
    /// Authority class from the catalog.
    #[arg(long, required = true, value_name = "CLASS")]
    class: String,
    /// Workspace identifier, or `all` for every workspace in the room.
    #[arg(long, default_value = grant::ALL_WORKSPACES, value_name = "WORKSPACE|all")]
    workspace: String,
    /// Optional condition in words narrowing this grant.
    #[arg(long, value_name = "TEXT")]
    limit: Option<String>,
    /// Lifetime of the grant (`day`; standing grants come from a grants file).
    #[arg(long = "for", default_value = "day", value_name = "LIFETIME")]
    lifetime: String,
    /// Where the admiral gave this direction (required).
    #[arg(long, required = true, value_name = "TEXT")]
    source: String,
}

pub(super) fn run(env: &Env, args: Add, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let cmd = grant::Command::Add {
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
