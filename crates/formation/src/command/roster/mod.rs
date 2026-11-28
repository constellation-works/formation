//! Roster verbs and dispatch.

mod apply;

mod check;

mod close;

mod input;

mod list;

mod show;

use clap::{Args, Subcommand};
use formation_core::Env;
use std::process::ExitCode;

#[derive(Args)]
pub(crate) struct Roster {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Check(check::Check),
    Show(show::Show),
    Apply(apply::Apply),
    List(list::List),
    Close(close::Close),
}

pub(crate) fn run(env: &Env, args: Roster) -> ExitCode {
    match args.action {
        Action::Check(args) => check::run(env, args),
        Action::Show(args) => show::run(env, args),
        Action::Apply(args) => apply::run(env, args),
        Action::List(args) => list::run(env, args),
        Action::Close(args) => close::run(env, args),
    }
}
