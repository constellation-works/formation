//! `formation mail who` argument mapping.

use crate::output::{self, Mode};
use clap::Args;
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::process::ExitCode;

/// List every joined seat, not only those seen lately: undelivered, unacknowledged, held, seen.
#[derive(Args)]
pub(super) struct Who;

pub(super) fn run(env: &Env, mode: Mode) -> ExitCode {
    mail::exec(env, Command::Who, |result| {
        output::mail_result(result, mode)
    })
}
