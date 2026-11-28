//! `formation grant check` argument mapping.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::grant;
use std::process::ExitCode;

/// Check whether a seat may settle an authority class in a workspace.
#[derive(Args)]
#[command(after_help = "\
Exits 0 with the covering entry as JSON when authorized, or exits 1 with a refusal on stderr.

Examples:
  formation grant check --seat chief --class direction
  formation grant check --seat site-captain --class land --workspace site")]
pub(super) struct Check {
    /// Seat name in the applied roster.
    #[arg(long, required = true, value_name = "SEAT")]
    seat: String,
    /// Authority class from the catalog.
    #[arg(long, required = true, value_name = "CLASS")]
    class: String,
    /// Workspace to check (default: the seat's workspace, or `all`).
    #[arg(long, value_name = "WORKSPACE|all")]
    workspace: Option<String>,
}

pub(super) fn run(env: &Env, args: Check) -> ExitCode {
    let cmd = grant::Command::Check {
        seat: args.seat,
        class: args.class,
        workspace: args.workspace,
    };
    grant::exec(env, cmd, |result| match result {
        Ok(out) => output::grant_write(&out),
        Err(error) => output::error(&error),
    })
}
