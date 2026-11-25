//! Convert clap arguments to application commands.

use crate::output;
use clap::{Args, Subcommand};
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Args)]
pub(crate) struct Mail {
    /// Room name or directory path (default: $FORMATION_ROOM or session file).
    #[arg(long, global = true, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Seat name for this command (default: seat recorded for this session).
    #[arg(long = "as", global = true, value_name = "SEAT")]
    seat: Option<String>,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Register this seat or refresh its presence.
    Join {
        /// Seat name to register or rebind.
        name: String,
    },
    /// List seats seen in the last 90 seconds.
    Who,
    /// Send to an online seat. Reuse the ID when retrying.
    Send {
        /// Online recipient seat name.
        to: String,
        /// Caller-generated message identifier.
        #[arg(long)]
        id: String,
        /// Message text (up to 8192 bytes).
        #[arg(long)]
        body: String,
    },
    /// Read unread messages or wait for up to 900 seconds.
    Inbox {
        /// Seconds to wait for the first unread message.
        #[arg(long, default_value_t=0, value_parser=clap::value_parser!(u64).range(0..=900))]
        wait: u64,
    },
    /// Mark a receipt in this seat's inbox as handled.
    Ack {
        /// Numeric receipt from the inbox.
        receipt: i64,
    },
}

pub(crate) fn run(env: Env, args: Mail) -> ExitCode {
    let env = env.with_mail_options(args.room, args.seat);
    let cmd = match args.action {
        Action::Join { name } => Command::Join(name),
        Action::Who => Command::Who,
        Action::Send { to, id, body } => Command::Send { to, id, body },
        Action::Inbox { wait } => Command::Inbox(Duration::from_secs(wait)),
        Action::Ack { receipt } => Command::Ack(receipt),
    };
    mail::exec(&env, cmd, |result| match result {
        Ok(out) => output::mail(&output::payload::mail(out)),
        Err(error) => output::error(&error),
    })
}
