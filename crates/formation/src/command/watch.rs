//! Convert `formation watch` CLI arguments into the watch application command.

use super::probe::{ShellProbeRunner, SignalGuard};
use crate::output;
use clap::{Args, Subcommand};
use formation_core::Env;
use formation_core::app::watch;
use formation_core::app::watch::{reader, set};
use std::process::ExitCode;
use std::time::Duration;

#[derive(Args)]
pub(crate) struct Watch {
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
    /// Read the room's watches, or wait as a standby while another reader holds the lease.
    Run,
    /// Stop the room's reader and all its standbys.
    Stop,
    /// Block until a caller-supplied probe returns new items or a read failure.
    Wait {
        /// Probe to run as <name>=<command>; repeat for multiple probes.
        #[arg(long = "probe", required = true, value_name = "NAME=COMMAND")]
        probes: Vec<String>,
        /// Seconds between polls (minimum 10).
        #[arg(
            long,
            default_value_t = watch::DEFAULT_EVERY_SECONDS,
            value_name = "SECONDS",
            value_parser = clap::value_parser!(u64).range(watch::MIN_EVERY_SECONDS..),
        )]
        every: u64,
        /// Seconds to wait (zero runs a single pass).
        #[arg(long, default_value_t = watch::DEFAULT_WAIT_TIMEOUT_SECONDS, value_name = "SECONDS")]
        timeout: u64,
    },
}

pub(crate) fn run(env: Env, args: Watch, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let env = env.with_mail_options(args.room, args.seat);
    match args.action {
        Action::Run => run_reader(&env, queue),
        Action::Wait {
            probes,
            every,
            timeout,
        } => wait(&env, probes, every, timeout),
        Action::Add {
            name,
            to,
            every,
            command,
        } => run_set(
            &env,
            set::Command::Add {
                name,
                to,
                every: Duration::from_secs(every),
                command,
            },
            false,
        ),
        Action::Remove { name, to } => run_set(&env, set::Command::Remove { name, to }, false),
        Action::Pause { name, to } => run_set(&env, set::Command::Pause { name, to }, false),
        Action::Resume { name, to } => run_set(&env, set::Command::Resume { name, to }, false),
        Action::List { json } => run_set(&env, set::Command::List, json),
        Action::Stop => run_set(&env, set::Command::Stop, false),
    }
}

fn run_reader(env: &Env, queue: &dyn Fn(&str) -> bool) -> ExitCode {
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

fn wait(env: &Env, probes: Vec<String>, every: u64, timeout: u64) -> ExitCode {
    let _signals = SignalGuard::install();
    let runner = ShellProbeRunner;
    let cmd = watch::Command {
        probes,
        every: Duration::from_secs(every),
        timeout: Some(Duration::from_secs(timeout)),
        runner: &runner,
    };

    watch::exec(env, cmd, |result| match result {
        Ok(out) => output::watch_write(&output::watch_payload(out)),
        Err(error) => output::error(&error),
    })
}

fn run_set(env: &Env, cmd: set::Command, json: bool) -> ExitCode {
    set::exec(env, cmd, |result| match result {
        Ok(set::Outcome::Listed { reader, watches }) => output::watch_list(&reader, &watches, json),
        Ok(out) => output::watch_write(&output::watch_set_payload(&out)),
        Err(error) => output::error(&error),
    })
}
