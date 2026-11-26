//! `formation watch wait` argument mapping.

use super::probe::ShellProbeRunner;
use crate::command::common::probe::SignalGuard;
use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch;
use std::process::ExitCode;
use std::time::Duration;

/// Block until a caller-supplied probe returns new items or a read failure.
#[derive(Args)]
pub(super) struct Wait {
    /// Probe to run as <name>=<command>; repeat for multiple probes.
    #[arg(long = "probe", required = true, value_name = "NAME=COMMAND")]
    probes: Vec<String>,
    /// Seconds between polls (minimum 10).
    #[arg(
        long,
        default_value_t = watch::DEFAULT_EVERY_SECONDS,
        value_name = "SECONDS",
        value_parser = clap::value_parser!(u64).range(watch::MIN_EVERY_SECONDS..),
    )]
    every: u64,
    /// Seconds to wait (zero runs a single pass).
    #[arg(
        long,
        default_value_t = watch::DEFAULT_WAIT_TIMEOUT_SECONDS,
        value_name = "SECONDS"
    )]
    timeout: u64,
}

pub(super) fn run(env: &Env, args: Wait) -> ExitCode {
    let _signals = SignalGuard::install();
    let runner = ShellProbeRunner;
    let cmd = watch::Command {
        probes: args.probes,
        every: Duration::from_secs(args.every),
        timeout: Some(Duration::from_secs(args.timeout)),
        runner: &runner,
    };

    watch::exec(env, cmd, |result| match result {
        Ok(out) => output::watch_write(&output::watch_payload(out)),
        Err(error) => output::error(&error),
    })
}
