//! Read-only room history arguments.

use crate::output::{self, Mode};
use clap::Args;
use formation_core::Env;
use formation_core::app::mail::{self, Command, LogOptions};
use std::process::ExitCode;

/// List room mail, newest first.
#[derive(Args)]
pub(super) struct Log {
    /// Include envelopes sent or received by this seat.
    #[arg(long, id = "log_seat", value_name = "SEAT")]
    seat: Option<String>,
    /// Include envelopes involving this other seat.
    #[arg(long)]
    with: Option<String>,
    /// Include envelopes stored at or after this RFC 3339 timestamp.
    #[arg(long)]
    since: Option<String>,
    /// Maximum number of envelopes or threads.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(i64).range(1..))]
    limit: i64,
    /// Exclude envelopes produced by watches.
    #[arg(long)]
    no_watch: bool,
    /// Print whole bodies.
    #[arg(long)]
    full: bool,
    /// Group envelopes linked by a reply or a request.
    #[arg(long)]
    threads: bool,
}

pub(super) fn run(env: &Env, args: Log, mode: Mode) -> ExitCode {
    let options = LogOptions {
        seat: args.seat,
        with: args.with,
        since: args.since,
        limit: args.limit,
        no_watch: args.no_watch,
        full: args.full,
        threads: args.threads,
    };
    mail::exec(env, Command::Log(options), |result| {
        output::mail_result(result, mode)
    })
}
