//! Dry-run one watch command: run it once, parse what it prints, and change nothing in the room.

use super::{Error, ProbeLimits, ProbeOutput, ProbeRunner, refused, rule, set};
use crate::Env;
use crate::runtime::{self, App, Ctx, RoomAccess};
use std::marker::PhantomData;
use std::path::Path;
use std::time::Duration;

/// A dry run of one watch command.
pub struct Command<'a> {
    /// Command words: one word is shell text, several are quoted into one command line.
    pub words: Vec<String>,
    /// Host probe runner and signal observer.
    pub runner: &'a (dyn ProbeRunner + Sync),
}

/// One item the command printed, as the reader would parse it.
pub struct Line {
    /// Item fingerprint.
    pub fingerprint: String,
    /// Item text (truncated to 400 characters ending in `…` if longer).
    pub text: String,
}

/// The items one run of the command reported.
pub struct Outcome {
    /// Parsed items in the order the command printed them, with repeated fingerprints dropped.
    pub items: Vec<Line>,
}

struct Check<'a>(PhantomData<&'a ()>);

impl<'a> App for Check<'a> {
    type Command = Command<'a>;

    type Outcome = Outcome;

    fn preflight(cmd: &Command<'a>) -> Result<(), Error> {
        rule::watch_command(&cmd.words)?;
        Ok(())
    }

    fn room_access<'cmd>(_: &'cmd Command<'a>) -> RoomAccess<'cmd> {
        RoomAccess::Existing
    }

    fn audit(_: &Command<'a>) -> Option<&'static str> {
        None
    }

    fn run(ctx: &Ctx<'_>, cmd: Command<'a>) -> Result<Outcome, Error> {
        let command = rule::watch_command(&cmd.words)?;
        let cwd = rule::watch_cwd(ctx.cwd)?;
        let mail = ctx.mail()?;
        set::caller_in_snapshot(&mail.store.read()?, super::caller_ref(&mail.caller)?)?;
        let limits = ProbeLimits {
            probe_timeout: Duration::from_secs(rule::PROBE_TIMEOUT_SECONDS),
            command_remaining: None,
        };
        match cmd
            .runner
            .run(&command, Some(Path::new(cwd)), limits, &|| false)
        {
            ProbeOutput::Success { stdout } => {
                let lines = rule::parse_output(&stdout).map_err(|message| {
                    refused(
                        "probe_contract",
                        format!("probe output breaks the contract: {message}"),
                    )
                })?;
                Ok(Outcome {
                    items: lines
                        .into_iter()
                        .map(|line| Line {
                            fingerprint: line.fingerprint,
                            text: line.text,
                        })
                        .collect(),
                })
            }
            ProbeOutput::Failed(message) => {
                Err(refused("probe_failed", rule::first_line(&message)))
            }
            ProbeOutput::Cancelled => Err(refused(
                "probe_interrupted",
                "the check was stopped before the command finished",
            )),
        }
    }
}

/// Run a watch command once and hand its typed outcome to the CLI surface.
pub fn exec<R>(env: &Env, cmd: Command<'_>, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<Check<'_>, R>(env, cmd, finish)
}
