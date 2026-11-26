//! Watch set, leased reader and blocking probe wait.

pub mod check;

mod poll;

pub mod reader;

mod rule;

pub mod set;

use crate::Env;
use crate::composition::Caller;
use crate::runtime;
use crate::runtime::App;
use crate::runtime::Ctx;
use crate::runtime::RoomAccess;
use cw_base::{Error, Kind};
use formation_store::SessionId;
use formation_store::Store;
pub use rule::{
    DEFAULT_EVERY_SECONDS, DEFAULT_WAIT_TIMEOUT_SECONDS, MIN_EVERY_SECONDS, PROBE_TIMEOUT_SECONDS,
};
use std::path::Path;
use std::time::Duration;

/// Execution bounds for a single probe run.
#[derive(Clone, Copy, Debug)]
pub struct ProbeLimits {
    /// Per-probe execution limit (60 seconds); exceeding it is a failed read.
    pub probe_timeout: Duration,
    /// Remaining overall command timeout, if any; exceeding it cancels cleanly.
    pub command_remaining: Option<Duration>,
}

/// Result of running one probe command through the host adapter.
pub enum ProbeOutput {
    /// Probe exited 0 with raw stdout bytes.
    Success {
        /// Captured standard output.
        stdout: Vec<u8>,
    },
    /// Probe failed (non-zero exit, killed on probe timeout, spawn error, or output limit).
    Failed(String),
    /// Probe was cancelled because the watch command was signalled or its overall timeout elapsed.
    Cancelled,
}

/// Host port for spawning probe commands and observing termination signals.
pub trait ProbeRunner {
    /// Run `command` through `sh -c` under the given timeout limits.
    fn run(
        &self,
        command: &str,
        cwd: Option<&Path>,
        limits: ProbeLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> ProbeOutput;

    /// Whether the watch process has received a termination signal.
    fn stopped(&self) -> bool;
}

/// A blocking watch wait.
pub struct Command<'a> {
    /// Probe specifications as `<name>=<command>`.
    pub probes: Vec<String>,
    /// Polling interval (minimum 10 seconds, default 120 seconds).
    pub every: Duration,
    /// Overall timeout in seconds (default 300, zero for a single pass).
    pub timeout: Option<Duration>,
    /// Host probe runner and signal observer.
    pub runner: &'a (dyn ProbeRunner + Sync),
}

/// One newly handed item from a probe.
pub struct Item {
    /// Probe name.
    pub probe: String,
    /// Item fingerprint.
    pub fingerprint: String,
    /// Item text (truncated to 400 characters ending in `…` if longer).
    pub text: String,
}

/// One newly reported probe read failure.
pub struct Failure {
    /// Probe name.
    pub probe: String,
    /// First-line failure description.
    pub error: String,
}

/// Result of a blocking wait.
pub struct Outcome {
    /// Newly handed items (at most 20).
    pub items: Vec<Item>,
    /// Newly reported probe failures.
    pub failed: Vec<Failure>,
    /// New items not yet handed over because of the 20-item batch limit.
    pub remaining: usize,
    /// Whether the command expired with nothing to report.
    pub expired: bool,
}

#[derive(Clone, Copy)]
pub(super) enum SeatRef<'a> {
    Name(&'a str),
    Session(&'a SessionId),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Refusal(String);

struct Watch<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> App for Watch<'a> {
    type Command = Command<'a>;

    type Outcome = Outcome;

    fn preflight(cmd: &Command<'a>) -> Result<(), Error> {
        rule::parse_probes(&cmd.probes)?;
        rule::every(cmd.every)?;
        Ok(())
    }

    fn room_access<'cmd>(_: &'cmd Command<'a>) -> RoomAccess<'cmd> {
        RoomAccess::Existing
    }

    fn audit(_: &Command<'a>) -> Option<&'static str> {
        None
    }

    fn run(ctx: &Ctx<'_>, cmd: Command<'a>) -> Result<Outcome, Error> {
        let mail = ctx.mail()?;
        poll::run(ctx, &mail.store, caller_ref(&mail.caller)?, cmd)
    }
}

fn caller_ref(caller: &Caller) -> Result<SeatRef<'_>, Error> {
    match caller {
        Caller::Explicit(name) => Ok(SeatRef::Name(name)),
        Caller::Session(session) => Ok(SeatRef::Session(session)),
        Caller::Join { .. } => Err(Error::new(
            Kind::Internal,
            "missing_context",
            std::io::Error::other("seat context unavailable"),
        )),
    }
}

/// Run a watch invocation and hand its typed outcome to the CLI surface.
pub fn exec<R>(env: &Env, cmd: Command<'_>, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<Watch<'_>, R>(env, cmd, finish)
}

fn refused(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(Kind::Refused, code, Refusal(message.into()))
}
