//! Room-scoped requests, advice, delegated settlements and routing.

mod change;

mod rule;

use crate::Env;
use crate::composition::Caller;
use crate::runtime;
use crate::runtime::{App, Ctx, RoomAccess};
use cw_base::Error;
use formation_store::Snapshot;
pub use formation_store::contract::{
    Request, RequestAdvice, RequestOption, RequestSettlement, RequestState,
};

/// Data supplied when opening a request.
pub struct Open {
    /// Authority class of the decision.
    pub class: String,
    /// Pointer to the task, pull request or other record.
    pub record: String,
    /// One-sentence ask.
    pub ask: String,
    /// Two to four choices supplied as `label: text`.
    pub options: Vec<String>,
    /// Recommended option label.
    pub recommendation: String,
    /// Whether this request is holding up work.
    pub blocking: bool,
    /// Target workspace: `all` or a workspace in the applied roster; absent keeps the asker's own.
    pub workspace: Option<String>,
    /// Option to settle with the caller's own authority, when opening and settling at once.
    pub settle: Option<String>,
}

/// Filters applied to room requests.
pub struct Filter {
    /// Current holder of an open request, using `admiral` for the operator.
    pub waiting: Option<String>,
    /// Seat whose settlements to show.
    pub settled_by: Option<String>,
    /// Inclusive lower bound on activity time, in RFC 3339.
    pub since: Option<String>,
}

/// One write to the room's request records.
pub enum Change {
    /// Open a request.
    Open(Open),
    /// Change a request already in the room.
    Existing {
        /// Request identifier.
        id: String,
        /// Change to the request.
        action: Action,
    },
}

/// A change to a request already in the room.
pub enum Action {
    /// Add advice without changing the current holder or decision.
    Advise {
        /// Recommended option label.
        option: String,
        /// Explanation of the recommendation.
        note: String,
    },
    /// Record a choice under the caller's effective authority.
    Settle {
        /// Chosen option label.
        option: String,
        /// Optional explanation of the choice.
        note: Option<String>,
        /// Admiral answer source; absence means delegated settlement.
        source: Option<String>,
    },
    /// Overturn a settlement and return it to the admiral.
    Reopen {
        /// Whether the caller explicitly records the admiral's action.
        admiral: bool,
    },
    /// Move an open request to its holder's parent, or to the admiral.
    Pass {
        /// Optional recommended option label.
        option: Option<String>,
        /// Optional explanation of the recommendation.
        note: Option<String>,
    },
}

/// A request operation.
pub enum Command<'a> {
    /// Change a request and wake an eligible recipient after mail commits.
    Change {
        /// Requested change.
        change: Change,
        /// Queue a fixed wake for an idle Codex session.
        queue: &'a dyn Fn(&str) -> bool,
    },
    /// Carry open requests from a previous room.
    Carry(String),
    /// Read matching requests.
    List(Filter),
    /// Read one request by identifier.
    Show(String),
}

/// Request data returned to a surface.
pub enum Outcome {
    /// A single stored request.
    Record(Box<Request>),
    /// All requests matching a list query.
    List(Vec<Request>),
}

struct RequestApp<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> App for RequestApp<'a> {
    type Command = Command<'a>;

    type Outcome = Outcome;

    fn room_access<'cmd>(_: &'cmd Command<'a>) -> RoomAccess<'cmd> {
        RoomAccess::OptionalSeat
    }

    fn audit(cmd: &Command<'a>) -> Option<&'static str> {
        match cmd {
            Command::Change { change, .. } => Some(match change {
                Change::Open(_) => "request open",
                Change::Existing { action, .. } => match action {
                    Action::Advise { .. } => "request advise",
                    Action::Settle { .. } => "request settle",
                    Action::Pass { .. } => "request pass",
                    Action::Reopen { .. } => "request reopen",
                },
            }),
            Command::Carry(_) => Some("request carry"),
            Command::List(_) | Command::Show(_) => None,
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command<'a>) -> Result<Outcome, Error> {
        match cmd {
            Command::Change { change, queue } => {
                change::run(ctx, change, queue).map(|request| Outcome::Record(Box::new(request)))
            }
            Command::Carry(from) => change::carry(ctx, &from).map(Outcome::List),
            Command::List(filter) => {
                let snapshot = read_snapshot(ctx)?;
                let filter = rule::ListFilter::parse(filter)?;
                let records = snapshot.requests().list()?;
                Ok(Outcome::List(
                    records
                        .into_iter()
                        .filter(|request| filter.matches(request))
                        .collect(),
                ))
            }
            Command::Show(id) => {
                let snapshot = read_snapshot(ctx)?;
                let request = rule::require_request(&id, snapshot.requests().get(&id)?)?;
                Ok(Outcome::Record(Box::new(request)))
            }
        }
    }

    fn target(out: &Outcome) -> Option<String> {
        match out {
            Outcome::Record(request) => Some(request.id.clone()),
            Outcome::List(_) => None,
        }
    }
}

/// Run a request operation and hand its typed result to the surface.
pub fn exec<R>(env: &Env, cmd: Command<'_>, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<RequestApp<'_>, R>(env, cmd, finish)
}

fn read_snapshot(ctx: &Ctx<'_>) -> Result<Snapshot, Error> {
    let room = ctx.mail()?;
    let snapshot = room.store.read()?;
    let roster = super::grant::require_roster_snapshot(&snapshot)?;
    if let Caller::Explicit(name) = &room.caller {
        super::grant::find_roster_seat(&roster, name)?;
    }
    Ok(snapshot)
}
