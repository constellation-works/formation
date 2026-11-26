//! Translate CLI arguments into Formation app commands.

pub(crate) mod common;

pub(crate) mod hook;

pub(crate) mod mail;

pub(crate) mod roster;

pub(crate) mod watch;

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Inspect a shift roster.
    Roster(roster::Roster),
    /// Exchange messages in a private local room.
    Mail(mail::Mail),
    /// Hand unread mail to a Claude Code or Codex session.
    Hook(hook::Hook),
    /// Manage the room's watches and wait for probe results.
    Watch(watch::Watch),
}
