//! CLI composition, dispatch and output.
mod cli;
mod commands;
mod output;
use clap::Parser;
use commands::Command;
use std::process::ExitCode;
/// Run Formation using one snapshot of its environment.
pub fn run(env: formation_core::Env) -> ExitCode {
    let args = cli::Cli::parse();
    output::init_logging();
    match args.command {
        Command::Roster(args) => commands::roster::run(&env, args),
        Command::Mail(args) => commands::mail::run(env, args),
    }
}
