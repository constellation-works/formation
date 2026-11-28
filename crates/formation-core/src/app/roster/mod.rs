//! Roster validation, presentation and room lifecycle.

mod lifecycle;

mod rule;

use crate::Env;
use crate::composition::Caller;
use crate::runtime;
use crate::runtime::App;
use crate::runtime::Ctx;
use crate::runtime::RoomAccess;
use cw_base::Kind;
use formation_store::Tx;
pub use lifecycle::{Close, Listing, Room, SeatCounts, Status, Unreadable};
pub use rule::{Error, Summary, check};

/// Roster commands.
pub enum Command {
    /// Check a versioned roster.
    Check(Vec<u8>),
    /// Apply a version 2 roster to an existing room.
    Apply(Vec<u8>),
    /// Show a room without changing presence.
    Show,
    /// List rooms in the Formation home, optionally filtered by status.
    List(Option<Status>),
    /// Mark an existing room closed.
    Close,
}

/// A successful roster command.
pub enum Outcome {
    /// Validation summary.
    Summary(Summary),
    /// Current roster view.
    View(View),
    /// Rooms in the Formation home.
    Listed(Listing),
    /// The room's first close.
    Closed(Close),
}

/// Applied seat with its current mailbox state.
#[derive(serde::Serialize)]
pub struct ViewSeat {
    /// Seat name.
    pub name: String,
    /// Rank.
    pub role: String,
    /// Workspace.
    pub workspace: Option<String>,
    /// Reporting parent.
    pub reports_to: Option<String>,
    /// Model.
    pub model: Option<String>,
    /// Effort.
    pub effort: Option<String>,
    /// Record.
    pub record: Option<String>,
    /// Joined state, if joined.
    pub live: Option<crate::app::mail::SeatStatus>,
}

/// A read-only roster and its joined seats.
#[derive(serde::Serialize)]
pub struct View {
    /// Room name or path.
    pub room: String,
    /// Lifecycle status.
    pub status: Status,
    /// Close timestamp, if closed.
    pub closed_at: Option<String>,
    /// Closing seat, if closed.
    pub closed_by: Option<String>,
    /// Application timestamp.
    pub applied_at: Option<String>,
    /// Applying seat, when known.
    pub applied_by: Option<String>,
    /// Whether a roster exists.
    pub roster_applied: bool,
    /// Planned seats.
    pub seats: Vec<ViewSeat>,
    /// Joined seats absent from the roster.
    pub not_on_roster: Vec<crate::app::mail::SeatStatus>,
}

struct Roster;

impl App for Roster {
    type Command = Command;

    type Outcome = Outcome;

    fn room_access(cmd: &Command) -> RoomAccess<'_> {
        match cmd {
            Command::Check(_) | Command::List(_) => RoomAccess::None,
            Command::Apply(_) | Command::Show | Command::Close => RoomAccess::OptionalSeat,
        }
    }

    fn audit(cmd: &Command) -> Option<&'static str> {
        match cmd {
            Command::Check(_) | Command::Show | Command::List(_) | Command::Close => None,
            Command::Apply(_) => Some("roster apply"),
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command) -> Result<Outcome, cw_base::Error> {
        match cmd {
            Command::Check(bytes) => decode(&bytes).map(|(_, summary)| Outcome::Summary(summary)),
            Command::Apply(bytes) => apply(ctx, &bytes).map(Outcome::Summary),
            Command::Show => show(ctx).map(Outcome::View),
            Command::List(status) => lifecycle::list(ctx, status).map(Outcome::Listed),
            Command::Close => lifecycle::close(ctx).map(Outcome::Closed),
        }
    }
}

/// Run through the shared execution boundary.
pub fn exec<R>(
    env: &Env,
    cmd: Command,
    finish: impl FnOnce(Result<Outcome, cw_base::Error>) -> R,
) -> R {
    runtime::exec::<Roster, R>(env, cmd, finish)
}

fn decode(bytes: &[u8]) -> Result<(formation_store::Roster, Summary), cw_base::Error> {
    let roster =
        formation_store::decode(bytes).map_err(|error| roster_error(Error::Decode(error)))?;
    let summary = rule::validate(&roster).map_err(roster_error)?;
    Ok((roster, summary))
}

fn apply(ctx: &Ctx<'_>, bytes: &[u8]) -> Result<Summary, cw_base::Error> {
    let mail = ctx.mail()?;
    let decoded = decode(bytes);
    mail.store.write(|tx| {
        if tx.closure()?.is_some() {
            return Err(lifecycle::Failure::RoomClosed(mail.room.clone()).into());
        }
        let (roster, summary) = decoded?;
        if roster.version != 2 {
            return Err(cw_base::Error::new(
                Kind::Usage,
                "roster_version_required",
                std::io::Error::other("apply requires roster version 2"),
            ));
        }
        let seat = caller_seat(tx, &mail.caller)?;
        tx.roster().replace(&roster)?;
        tx.roster()
            .stamp((ctx.clock)().unix_timestamp(), seat.as_deref())?;
        Ok(summary)
    })
}

fn caller_seat(tx: &Tx<'_>, caller: &Caller) -> Result<Option<String>, cw_base::Error> {
    match caller {
        Caller::Session(id) => tx.seats().by_session(id),
        Caller::Explicit(name) if !name.is_empty() => Ok(Some(name.clone())),
        Caller::Join { .. } | Caller::Explicit(_) | Caller::None | Caller::Admiral => Ok(None),
    }
}

fn roster_error(error: Error) -> cw_base::Error {
    let code = match error {
        Error::UnknownParent => "unknown_parent",
        Error::ParentCycle => "roster_cycle",
        Error::ReservedSeat => "reserved_seat",
        Error::Decode(_) | Error::Invalid(_) => "invalid_roster",
    };
    cw_base::Error::new(cw_base::Kind::Usage, code, error)
}

fn show(ctx: &Ctx<'_>) -> Result<View, cw_base::Error> {
    let mail = ctx.mail()?;
    let snapshot = mail.store.read()?;
    view_snapshot(&snapshot, &mail.room, (ctx.clock)().unix_timestamp())
}

pub(crate) fn view_snapshot(
    snapshot: &formation_store::Snapshot,
    room: &str,
    now: i64,
) -> Result<View, cw_base::Error> {
    let roster = snapshot.roster().seats()?;
    let stamp = snapshot.roster().application()?;
    let closure = snapshot.closure()?;
    let status = Status::from_closed(closure.is_some());
    let mut live = crate::app::mail::who::statuses(snapshot.seats().mailboxes()?, now);
    live.retain(|seat| seat.name != formation_store::contract::ADMIRAL);
    if status == Status::Closed {
        for seat in &mut live {
            seat.held = Some(false);
        }
    }
    let roster_applied = !roster.is_empty() || stamp.is_some();
    let seats = roster
        .into_iter()
        .map(|seat| {
            let state = live
                .iter()
                .position(|item| item.name == seat.name)
                .map(|index| live.remove(index));
            ViewSeat {
                name: seat.name,
                role: seat.role,
                workspace: seat.workspace,
                reports_to: seat.reports_to,
                model: seat.model,
                effort: seat.effort,
                record: seat.record,
                live: state,
            }
        })
        .collect();
    let (applied_at, applied_by) = match stamp {
        Some((at, by)) => (formation_store::contract::rfc3339(at), by),
        None => (None, None),
    };
    let (closed_at, closed_by) = lifecycle::closure_fields(closure);
    Ok(View {
        room: room.to_owned(),
        status,
        closed_at,
        closed_by,
        applied_at,
        applied_by,
        roster_applied,
        seats,
        not_on_roster: live,
    })
}
