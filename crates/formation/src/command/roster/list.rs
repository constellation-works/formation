//! Room listing arguments.

use crate::output;
use clap::{Args, ValueEnum};
use formation_core::Env;
use formation_core::app::roster::{self, Command, Status};
use std::process::ExitCode;

/// List rooms in the Formation home, newest first.
#[derive(Args)]
pub(super) struct List {
    /// Keep rooms of this status. Defaults to both statuses.
    #[arg(long, value_enum)]
    status: Option<StatusArg>,
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum StatusArg {
    Active,
    Closed,
}

impl From<StatusArg> for Status {
    fn from(status: StatusArg) -> Self {
        match status {
            StatusArg::Active => Self::Active,
            StatusArg::Closed => Self::Closed,
        }
    }
}

pub(super) fn run(env: &Env, args: List) -> ExitCode {
    roster::exec(env, Command::List(args.status.map(Into::into)), |result| {
        output::roster_result(result, output::resolve_mode(args.json))
    })
}
