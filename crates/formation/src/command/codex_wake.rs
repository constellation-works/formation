//! Submit the fixed wake line through the supervised subprocess boundary.

use super::probe::{run_process, stop_requested};
use formation_core::app::watch::{ProbeLimits, ProbeOutput};
use std::ffi::OsString;
use std::process::Command;
use std::time::Duration;

const WAKE_LINE: &str = "Formation: mail is waiting for this seat. \
    Formation queued this line. The operator did not type it.";

// Codex resolves its local queue and desktop daemon from these account paths.
pub(crate) const CHILD_ENV: &[&str] = &["PATH", "HOME", "CODEX_HOME", "TMPDIR", "XDG_RUNTIME_DIR"];

pub(crate) fn queue(session: &str, env: &[(&str, OsString)]) -> bool {
    let mut process = Command::new("codex");
    process
        .args(["queue", "--thread", session, "--message", WAKE_LINE])
        .env_clear()
        .envs(env.iter().map(|(name, value)| (*name, value)));
    let limits = ProbeLimits {
        probe_timeout: Duration::from_secs(5),
        command_remaining: None,
    };
    matches!(
        run_process(process, limits, &stop_requested),
        ProbeOutput::Success { .. }
    )
}
