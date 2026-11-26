//! Supervised shell probe execution for watch commands.

use crate::command::common::probe::{run_process, stop_requested};
use formation_core::app::watch::{ProbeLimits, ProbeOutput, ProbeRunner};
use std::path::Path;
use std::process::Command as ProcessCommand;

/// Runs caller-supplied probe commands through `sh -c` in isolated process groups.
pub(super) struct ShellProbeRunner;

impl ProbeRunner for ShellProbeRunner {
    fn run(
        &self,
        command: &str,
        cwd: Option<&Path>,
        limits: ProbeLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> ProbeOutput {
        run_probe(command, cwd, limits, &|| self.stopped() || cancelled())
    }

    fn stopped(&self) -> bool {
        stop_requested()
    }
}

fn run_probe(
    command: &str,
    cwd: Option<&Path>,
    limits: ProbeLimits,
    cancelled: &dyn Fn() -> bool,
) -> ProbeOutput {
    let mut process = ProcessCommand::new("sh");
    process.arg("-c").arg(command);
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }
    run_process(process, limits, cancelled)
}
