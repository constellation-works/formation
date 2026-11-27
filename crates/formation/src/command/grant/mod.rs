//! Convert `formation grant` CLI arguments into grant application commands.

mod add;

mod check;

mod list;

mod revoke;

mod withhold;

use clap::{Args, Subcommand};
use formation_core::Env;
use std::process::ExitCode;

#[derive(Args)]
pub(crate) struct Grant {
    /// Room name or directory path (default: $FORMATION_ROOM or session file).
    #[arg(long, global = true, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Seat name for this command (default: seat recorded for this session, or none outside one).
    #[arg(id = "as_seat", long = "as", global = true, value_name = "SEAT")]
    as_seat: Option<String>,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Add(add::Add),
    Withhold(withhold::Withhold),
    Revoke(revoke::Revoke),
    List(list::List),
    Check(check::Check),
}

pub(crate) fn run(env: Env, args: Grant, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let env = env.with_mail_options(args.room, args.as_seat);
    match args.action {
        Action::Add(args) => add::run(&env, args, queue),
        Action::Withhold(args) => withhold::run(&env, args, queue),
        Action::Revoke(args) => revoke::run(&env, args, queue),
        Action::List(args) => list::run(&env, args),
        Action::Check(args) => check::run(&env, args),
    }
}
