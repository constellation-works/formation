//! `formation watch run` argument mapping.

use super::probe::ShellProbeRunner;
use crate::command::common::probe::SignalGuard;
use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::watch::reader;
use std::process::ExitCode;

/// Read the room's watches, or wait as a standby while another reader holds the lease.
#[derive(Args)]
pub(super) struct Run;

pub(super) fn run(env: &Env, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let _signals = SignalGuard::install();
    let runner = ShellProbeRunner;
    reader::exec(
        env,
        reader::Command {
            pid: std::process::id(),
            runner: &runner,
            queue,
        },
        |result| match result {
            Ok(out) => output::watch_exited(&out),
            Err(error) => output::error(&error),
        },
    )
}
