//! `formation mail send` argument mapping.

use crate::command::common::probe::SignalGuard;
use crate::output::{self, Mode};
use clap::Args;
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::process::ExitCode;

/// Send to a joined seat, present or not. Reuse the ID when retrying.
#[derive(Args)]
pub(super) struct Send {
    /// Joined recipient seat name.
    to: String,
    /// Caller-generated message identifier.
    #[arg(long)]
    id: String,
    /// Message text (up to 8192 bytes).
    #[arg(long)]
    body: String,
    /// Reply to a receipt from your mail log.
    #[arg(long = "re", value_name = "RECEIPT")]
    reply_to: Option<i64>,
}

pub(super) fn run(env: &Env, args: Send, queue: &dyn Fn(&str) -> bool, mode: Mode) -> ExitCode {
    let _signals = SignalGuard::install();
    let cmd = Command::Send {
        to: args.to,
        id: args.id,
        body: args.body,
        reply_to: args.reply_to,
        queue,
    };
    mail::exec(env, cmd, |result| output::mail_result(result, mode))
}
