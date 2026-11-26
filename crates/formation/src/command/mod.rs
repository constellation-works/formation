//! Translate CLI arguments into Formation app commands.

use clap::Subcommand;

pub(crate) mod hook;

pub(crate) mod mail;

mod probe;

pub(crate) mod roster;

pub(crate) mod watch;

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Inspect a shift roster.
    Roster(roster::Roster),
    /// Exchange messages in a private local room.
    Mail(mail::Mail),
    /// Hand unread mail to a Claude Code or Codex session.
    Hook(hook::Hook),
    /// Run probes on an interval and report or mail new items.
    Watch(watch::Watch),
}
