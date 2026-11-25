//! Translate CLI arguments into Formation app commands.
use clap::Subcommand;
pub(crate) mod mail;
pub(crate) mod roster;
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Inspect a shift roster.
    Roster(roster::Roster),
    /// Exchange messages in a private local room.
    Mail(mail::Mail),
}
