//! Convert clap arguments to application commands.
use crate::output;
use clap::{Args, Subcommand};
use formation_core::{
    Env,
    apps::mail::{self, Command},
};
use std::{path::PathBuf, process::ExitCode, time::Duration};
#[derive(Args)]
pub(crate) struct Mail {
    /// Existing private directory shared by this shift (mode 0700; default: $FORMATION_ROOM).
    #[arg(long)]
    room: Option<PathBuf>,
    /// Private seat credential file. Join creates it if absent (default: $FORMATION_IDENTITY).
    #[arg(long)]
    identity: Option<PathBuf>,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Register this seat or refresh its presence.
    Join {
        /// Unique seat name for this identity.
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
    let env = env.with_mail_paths(args.room, args.identity);
    let cmd = match args.action {
        Action::Join { name } => Command::Join(name),
        Action::Who => Command::Who,
        Action::Send { to, id, body } => Command::Send { to, id, body },
        Action::Inbox { wait } => Command::Inbox(Duration::from_secs(wait)),
        Action::Ack { receipt } => Command::Ack(receipt),
    };
    mail::exec(&env, cmd, |result| match result {
        Ok(out) => output::mail(&output::payload::mail(out)),
        Err(e) => output::error(&e),
    })
}
