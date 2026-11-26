//! Convert `formation watch` CLI arguments into watch application commands.

mod add;

mod check;

mod list;

mod pause;

mod probe;

mod remove;

mod resume;

mod run;

mod stop;

mod wait;

use clap::{Args, Subcommand};
use formation_core::Env;
use std::process::ExitCode;

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
    Add(add::Add),
    Remove(remove::Remove),
    Pause(pause::Pause),
    Resume(resume::Resume),
    List(list::List),
    Run(run::Run),
    Stop(stop::Stop),
    Check(check::Check),
    Wait(wait::Wait),
}

pub(crate) fn run(env: Env, args: Watch, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let env = env.with_mail_options(args.room, args.seat);
    match args.action {
        Action::Add(args) => add::run(&env, args),
        Action::Remove(args) => remove::run(&env, args),
        Action::Pause(args) => pause::run(&env, args),
        Action::Resume(args) => resume::run(&env, args),
        Action::List(args) => list::run(&env, args),
        Action::Run(_) => run::run(&env, queue),
        Action::Stop(_) => stop::run(&env),
        Action::Check(args) => check::run(&env, args),
        Action::Wait(args) => wait::run(&env, args),
    }
}
