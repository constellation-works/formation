//! Offline roster application.

mod rule;

use crate::Env;
use crate::runtime;
use crate::runtime::App;
use crate::runtime::Ctx;
use crate::runtime::RoomAccess;
pub use rule::{Error, Summary, check};

/// Offline roster commands.
pub enum Command {
    /// Check a versioned roster.
    Check(Vec<u8>),
    /// Apply a version 2 roster to an existing room.
    Apply(Vec<u8>),
    /// Show a room without changing presence.
    Show,
}

/// A successful roster check.
pub enum Outcome {
    /// Validation summary.
    Summary(Summary),
    /// Current roster view.
    View(View),
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
            Command::Check(_) => RoomAccess::None,
            Command::Apply(_) | Command::Show => RoomAccess::OptionalSeat,
        }
    }

    fn audit(cmd: &Command) -> Option<&'static str> {
        match cmd {
            Command::Check(_) | Command::Show => None,
            Command::Apply(_) => Some("roster apply"),
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command) -> Result<Outcome, cw_base::Error> {
        if matches!(cmd, Command::Show) {
            return show(ctx).map(Outcome::View);
        }
        let (bytes, apply) = match cmd {
            Command::Check(bytes) => (bytes, false),
            Command::Apply(bytes) => (bytes, true),
            Command::Show => unreachable!(),
        };
        let roster =
            formation_store::decode(&bytes).map_err(|error| roster_error(Error::Decode(error)))?;
        let summary = rule::validate(&roster).map_err(roster_error)?;
        if apply {
            if roster.version != 2 {
                return Err(cw_base::Error::new(
                    cw_base::Kind::Usage,
                    "roster_version_required",
                    std::io::Error::other("apply requires roster version 2"),
                ));
            }
            let mail = ctx.mail()?;
            mail.store.write(|tx| {
                let seat = match &mail.caller {
                    crate::composition::Caller::Session(id) => tx.seats().by_session(id)?,
                    crate::composition::Caller::Explicit(name) if !name.is_empty() => {
                        Some(name.clone())
                    }
                    crate::composition::Caller::Join { .. }
                    | crate::composition::Caller::Explicit(_)
                    | crate::composition::Caller::None => None,
                };
                tx.roster().replace(&roster)?;
                tx.roster()
                    .stamp((ctx.clock)().unix_timestamp(), seat.as_deref())
            })?;
        }
        Ok(Outcome::Summary(summary))
    }
}

/// Run through the shared execution boundary without opening a room.
pub fn exec<R>(
    env: &Env,
    cmd: Command,
    finish: impl FnOnce(Result<Outcome, cw_base::Error>) -> R,
) -> R {
    runtime::exec::<Roster, R>(env, cmd, finish)
}

fn roster_error(error: Error) -> cw_base::Error {
    let code = match error {
        Error::UnknownParent => "unknown_parent",
        Error::ParentCycle => "roster_cycle",
        Error::Decode(_) | Error::Invalid(_) => "invalid_roster",
    };
    cw_base::Error::new(cw_base::Kind::Usage, code, error)
}

fn show(ctx: &Ctx<'_>) -> Result<View, cw_base::Error> {
    let mail = ctx.mail()?;
    let snapshot = mail.store.read()?;
    let roster = snapshot.roster().seats()?;
    let stamp = snapshot.roster().application()?;
    let mut live = crate::app::mail::who::statuses(
        snapshot.seats().mailboxes()?,
        (ctx.clock)().unix_timestamp(),
    );
    let roster_applied = !roster.is_empty();
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
    Ok(View {
        room: mail.room.clone(),
        applied_at,
        applied_by,
        roster_applied,
        seats,
        not_on_roster: live,
    })
}
