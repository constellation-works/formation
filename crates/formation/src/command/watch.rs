//! Convert `formation watch` CLI arguments into the watch application command.

use super::probe::{ShellProbeRunner, SignalGuard};
use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Args)]
pub(crate) struct Watch {
    /// Room name or directory path (default: $FORMATION_ROOM or session file).
    #[arg(long, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Seat name for this command (default: seat recorded for this session).
    #[arg(long = "as", value_name = "SEAT")]
    seat: Option<String>,
    /// Probe to run as <name>=<command>; repeat for multiple probes.
    #[arg(long = "probe", required = true, value_name = "NAME=COMMAND")]
    probes: Vec<String>,
    /// Joined recipient seat to mail new items to instead of returning JSON.
    #[arg(long, value_name = "SEAT")]
    to: Option<String>,
    /// Seconds between polls (minimum 10).
    #[arg(
        long,
        default_value_t = watch::DEFAULT_EVERY_SECONDS,
        value_name = "SECONDS",
        value_parser = clap::value_parser!(u64).range(watch::MIN_EVERY_SECONDS..),
    )]
    every: u64,
    /// Seconds to run before exiting (default: 300 without --to, none with --to).
    #[arg(long, value_name = "SECONDS")]
    timeout: Option<u64>,
}

pub(crate) fn run(env: Env, args: Watch) -> ExitCode {
    let env = env.with_mail_options(args.room, args.seat);
    let _signals = SignalGuard::install();
    let runner = ShellProbeRunner;
    let cmd = watch::Command {
        probes: args.probes,
        to: args.to,
        every: Duration::from_secs(args.every),
        timeout: args.timeout.map(Duration::from_secs),
        runner: &runner,
    };

    watch::exec(&env, cmd, |result| match result {
        Ok(out) => output::watch_write(&output::watch_payload(out)),
        Err(error) => output::error(&error),
    })
}
