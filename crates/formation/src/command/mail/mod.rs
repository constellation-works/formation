//! Convert `formation mail` CLI arguments into application commands.

mod ack;

mod inbox;

mod join;

mod send;

mod sent;

mod who;

use crate::output;
use clap::{Args, Subcommand};
use formation_core::Env;
use std::process::ExitCode;

#[derive(Args)]
pub(crate) struct Mail {
    /// Room name or directory path (default: $FORMATION_ROOM or session file).
    #[arg(long, global = true, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Seat name for this command (default: seat recorded for this session).
    #[arg(long = "as", global = true, value_name = "SEAT")]
    seat: Option<String>,
    /// Print JSON, overriding $FORMATION_FORMAT (auto, human or json).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Join(join::Join),
    Who(who::Who),
    Send(send::Send),
    Inbox(inbox::Inbox),
    Sent(sent::Sent),
    Ack(ack::Ack),
}

pub(crate) fn run(env: Env, args: Mail, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let explicit_room = args.room.is_some();
    let mode = output::resolve_mode(args.json);
    let env = env.with_mail_options(args.room, args.seat);
    match args.action {
        Action::Join(args) => join::run(env, explicit_room, args, mode),
        Action::Who(_) => who::run(&env, mode),
        Action::Send(args) => send::run(&env, args, queue, mode),
        Action::Inbox(args) => inbox::run(&env, args, mode),
        Action::Sent(args) => sent::run(&env, args, mode),
        Action::Ack(args) => ack::run(&env, args, mode),
    }
}
