//! Map request subcommands and their flags to application operations.

use crate::output;
use clap::{Args, Subcommand};
use formation_core::Env;
use formation_core::app::request::{self, Action, Change, Command, Filter};
use std::process::ExitCode;

#[derive(Args)]
pub(crate) struct Request {
    /// Room name or directory path (default: $FORMATION_ROOM or session file).
    #[arg(long, global = true, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Joined seat to act as (default: this session's seat).
    #[arg(id = "as_seat", long = "as", global = true, value_name = "SEAT")]
    as_seat: Option<String>,
    /// Print machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    verb: Verb,
}

#[derive(Subcommand)]
enum Verb {
    /// Open a request and send it to your parent.
    Open {
        /// Authority class from the catalog.
        #[arg(long, value_name = "CLASS")]
        class: String,
        /// Pointer to the task, pull request or other record.
        #[arg(long, value_name = "POINTER")]
        record: String,
        /// One-sentence ask.
        #[arg(long, value_name = "TEXT")]
        ask: String,
        /// Choice as "label: text". Supply this flag two to four times.
        #[arg(long = "option", required = true, value_name = "LABEL: TEXT")]
        options: Vec<String>,
        /// Label of the option you recommend.
        #[arg(long = "recommend", value_name = "LABEL")]
        recommendation: String,
        /// Mark the request as blocking the asker's work.
        #[arg(long)]
        blocking: bool,
        /// Open and settle with this option under your own authority.
        #[arg(long, value_name = "LABEL")]
        settle: Option<String>,
    },
    /// Add a recommendation without changing the request's state or holder.
    Advise {
        /// Request identifier.
        #[arg(value_name = "REQUEST")]
        id: String,
        /// Label of the option you recommend.
        #[arg(long = "recommend", value_name = "LABEL")]
        option: String,
        /// Explain the recommendation.
        #[arg(long, value_name = "TEXT")]
        note: String,
    },
    /// Record a decision under your authority and mail the asker.
    Settle {
        /// Request identifier.
        #[arg(value_name = "REQUEST")]
        id: String,
        /// Label of the option you choose.
        #[arg(long, value_name = "LABEL")]
        option: String,
        /// Explain the decision.
        #[arg(long, value_name = "TEXT")]
        note: Option<String>,
    },
    /// Pass a request you hold to your parent, or to the admiral if you have no parent.
    Pass {
        /// Request identifier.
        #[arg(value_name = "REQUEST")]
        id: String,
        /// Label of the option you recommend.
        #[arg(long = "recommend", value_name = "LABEL")]
        option: Option<String>,
        /// Explain the recommendation supplied with --recommend.
        #[arg(long, value_name = "TEXT")]
        note: Option<String>,
    },
    /// List requests, newest first.
    List {
        /// Show open requests held by a seat or by the admiral (default: any state).
        #[arg(long, value_name = "SEAT|admiral")]
        waiting: Option<String>,
        /// Show only decisions made by this seat.
        #[arg(long, value_name = "SEAT")]
        settled_by: Option<String>,
        /// Show activity at or after an RFC 3339 time, or settlements when using --settled-by.
        #[arg(long, value_name = "TIME")]
        since: Option<String>,
    },
    /// Show a request, its options, advice and any settlement.
    Show {
        /// Request identifier.
        #[arg(value_name = "REQUEST")]
        id: String,
    },
}

pub(crate) fn run(env: Env, args: Request, queue: &dyn Fn(&str) -> bool) -> ExitCode {
    let env = env.with_mail_options(args.room, args.as_seat);
    let cmd = match args.verb {
        Verb::Open {
            class,
            record,
            ask,
            options,
            recommendation,
            blocking,
            settle,
        } => Command::Change {
            change: Change::Open(request::Open {
                class,
                record,
                ask,
                options,
                recommendation,
                blocking,
                settle,
            }),
            queue,
        },
        Verb::Advise { id, option, note } => Command::Change {
            change: Change::Existing {
                id,
                action: Action::Advise { option, note },
            },
            queue,
        },
        Verb::Settle { id, option, note } => Command::Change {
            change: Change::Existing {
                id,
                action: Action::Settle { option, note },
            },
            queue,
        },
        Verb::Pass { id, option, note } => Command::Change {
            change: Change::Existing {
                id,
                action: Action::Pass { option, note },
            },
            queue,
        },
        Verb::List {
            waiting,
            settled_by,
            since,
        } => Command::List(Filter {
            waiting,
            settled_by,
            since,
        }),
        Verb::Show { id } => Command::Show(id),
    };
    request::exec(&env, cmd, |result| output::request(result, args.json))
}
