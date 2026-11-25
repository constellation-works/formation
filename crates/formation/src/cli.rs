//! Root command declaration.

use crate::command::Command;
use clap::Parser;

/// Coordinate local agent inboxes and check proposed shift rosters.
#[derive(Parser)]
#[command(version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}
