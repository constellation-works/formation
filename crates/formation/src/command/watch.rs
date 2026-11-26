//! Convert `formation watch` CLI arguments into the watch application command.

use super::probe::{ShellProbeRunner, SignalGuard};
use crate::output;
use clap::{Args, Subcommand};
use formation_core::Env;
use formation_core::app::watch;
use formation_core::app::watch::set;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Args)]
#[command(subcommand_negates_reqs = true)]
pub(crate) struct Watch {
    /// Room name or directory path (default: $FORMATION_ROOM or session file).
    #[arg(long, global = true, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Seat name for this command (default: seat recorded for this session).
    #[arg(long = "as", global = true, value_name = "SEAT")]
    seat: Option<String>,
    #[command(subcommand)]
    action: Option<Action>,
    /// Probe to run as <name>=<command>; repeat for multiple probes.
    #[arg(long = "probe", required = true, value_name = "NAME=COMMAND")]
    probes: Vec<String>,
    /// Joined recipient seat to mail new items to instead of returning JSON.
    #[arg(long, value_name = "SEAT")]
    to: Option<String>,
    /// Seconds between polls (minimum 10; default 120).
    #[arg(
        long,
        value_name = "SECONDS",
        value_parser = clap::value_parser!(u64).range(watch::MIN_EVERY_SECONDS..),
    )]
    every: Option<u64>,
    /// Seconds to run before exiting (default: 300 without --to, none with --to).
    #[arg(long, value_name = "SECONDS")]
    timeout: Option<u64>,
}

#[derive(Subcommand)]
enum Action {
    /// Add a watch to the room's set, or replace the one with the same name and recipient.
    #[command(after_help = "\
A replaced watch keeps its remembered fingerprints. One word after `--` is shell text, as
written; several words are quoted into one command line, so each stays one argument.

Examples:
  formation watch add ci --every 300 -- ./scripts/ci-read.sh owner/name main
  formation watch add release --to captain --every 60 -- ./scripts/release-read.sh")]
    Add {
        /// Watch name (1 to 32 lowercase ASCII letters, digits or hyphens).
        name: String,
        /// Joined seat to mail new items to (default: the calling seat).
        #[arg(long, value_name = "SEAT")]
        to: Option<String>,
        /// Seconds between reads (minimum 10).
        #[arg(
            long,
            default_value_t = watch::DEFAULT_EVERY_SECONDS,
            value_name = "SECONDS",
            value_parser = clap::value_parser!(u64).range(watch::MIN_EVERY_SECONDS..),
        )]
        every: u64,
        /// Command to run through `sh -c` from the current directory.
        #[arg(last = true, required = true, value_name = "COMMAND")]
        command: Vec<String>,
    },
    /// Drop a watch and forget its remembered fingerprints.
    Remove {
        /// Watch name.
        name: String,
        /// Seat the watch mails (default: the calling seat).
        #[arg(long, value_name = "SEAT")]
        to: Option<String>,
    },
    /// Keep a watch but stop reading it.
    Pause {
        /// Watch name.
        name: String,
        /// Seat the watch mails (default: the calling seat).
        #[arg(long, value_name = "SEAT")]
        to: Option<String>,
    },
    /// Read a paused watch again.
    Resume {
        /// Watch name.
        name: String,
        /// Seat the watch mails (default: the calling seat).
        #[arg(long, value_name = "SEAT")]
        to: Option<String>,
    },
    /// List every watch in the room and whether anything reads the set.
    List {
        /// Print a JSON document.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn run(env: Env, args: Watch) -> ExitCode {
    let env = env.with_mail_options(args.room, args.seat);
    if let Some(action) = args.action {
        if !args.probes.is_empty()
            || args.to.is_some()
            || args.every.is_some()
            || args.timeout.is_some()
        {
            clap::Error::raw(
                clap::error::ErrorKind::ArgumentConflict,
                "--probe, --to, --every and --timeout belong to `formation watch --probe` and cannot be combined with a subcommand\n",
            )
            .exit();
        }
        return run_set(&env, action);
    }
    let _signals = SignalGuard::install();
    let runner = ShellProbeRunner;
    let cmd = watch::Command {
        probes: args.probes,
        to: args.to,
        every: Duration::from_secs(args.every.unwrap_or(watch::DEFAULT_EVERY_SECONDS)),
        timeout: args.timeout.map(Duration::from_secs),
        runner: &runner,
    };

    watch::exec(&env, cmd, |result| match result {
        Ok(out) => output::watch_write(&output::watch_payload(out)),
        Err(error) => output::error(&error),
    })
}

fn run_set(env: &Env, action: Action) -> ExitCode {
    let json = matches!(action, Action::List { json: true });
    let cmd = match action {
        Action::Add {
            name,
            to,
            every,
            command,
        } => set::Command::Add {
            name,
            to,
            every: Duration::from_secs(every),
            command,
        },
        Action::Remove { name, to } => set::Command::Remove { name, to },
        Action::Pause { name, to } => set::Command::Pause { name, to },
        Action::Resume { name, to } => set::Command::Resume { name, to },
        Action::List { json: _ } => set::Command::List,
    };
    set::exec(env, cmd, |result| match result {
        Ok(set::Outcome::Listed { reader, watches }) => output::watch_list(&reader, &watches, json),
        Ok(out) => output::watch_write(&output::watch_set_payload(&out)),
        Err(error) => output::error(&error),
    })
}
