//! CLI composition, dispatch and output.

mod cli;

mod command;

mod output;

use clap::Parser;
use command::{Command, codex_wake};
use std::process::ExitCode;

/// Run Formation using one snapshot of its environment.
pub fn run(env: formation_core::Env) -> ExitCode {
    let args = match cli::Cli::try_parse() {
        Ok(args) => args,
        Err(error)
            if error.use_stderr()
                && std::env::args_os().nth(1).is_some_and(|arg| arg == "hook") =>
        {
            return output::hook_usage(&error);
        }
        Err(error) => error.exit(),
    };
    if !matches!(args.command, Command::Hook(_)) {
        output::init_logging();
    }
    let child_env: Vec<_> = codex_wake::CHILD_ENV
        .iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (*name, value)))
        .collect();
    let queue = |session: &str| codex_wake::queue(session, &child_env);
    match args.command {
        Command::Roster(args) => command::roster::run(&env, args),
        Command::Mail(args) => command::mail::run(env, args, &queue),
        Command::Hook(args) => command::hook::run(&env, args),
        Command::Watch(args) => command::watch::run(env, args, &queue),
    }
}
