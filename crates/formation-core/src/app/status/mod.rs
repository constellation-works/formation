//! A read-only view of one room at a single point in time.

use crate::Env;
use crate::composition::Caller;
use crate::runtime::{self, App, Ctx, RoomAccess};
use cw_base::{Error, Kind};
use formation_store::LogQuery;
use formation_store::contract::{ADMIRAL, LogMessage, Message, RequestState};
use std::collections::BTreeMap;

/// The information displayed by status.
#[derive(serde::Serialize)]
pub struct View {
    /// Roster and room lifecycle.
    pub roster: super::roster::View,
    /// Calling seat.
    pub caller: String,
    /// Unacknowledged envelopes addressed to the caller.
    pub inbox: Vec<Message>,
    /// Open request counts by holder.
    pub requests: BTreeMap<String, usize>,
    /// Watch reader state.
    pub reader: super::watch::set::Reader,
    /// Number of enabled watches.
    pub watches_on: usize,
    /// Newest visible envelopes.
    pub recent: Vec<LogMessage>,
}

struct Status;

impl App for Status {
    type Command = ();

    type Outcome = View;

    fn room_access(_: &()) -> RoomAccess<'_> {
        RoomAccess::OptionalMember
    }

    fn audit(_: &()) -> Option<&'static str> {
        None
    }

    fn run(ctx: &Ctx<'_>, _: ()) -> Result<View, Error> {
        let mail = ctx.mail()?;
        let snapshot = mail.store.read_strict()?;
        let seat = match &mail.caller {
            Caller::Admiral => super::mail::SeatRef::Admiral,
            Caller::Explicit(name) => super::mail::SeatRef::Name(name),
            Caller::Session(session) => super::mail::SeatRef::Session(session),
            Caller::None | Caller::Join { .. } => {
                return Err(Error::new(
                    Kind::Refused,
                    "seat_required",
                    std::io::Error::other(
                        "seat required: pass --as <seat> or join this room from your session",
                    ),
                ));
            }
        };
        let visible = super::mail::log::caller(&snapshot, seat)?;
        let caller = visible.as_deref().unwrap_or(ADMIRAL).to_owned();
        let now = (ctx.clock)().unix_timestamp();
        let roster = super::roster::view_snapshot(&snapshot, &mail.room, now)?;
        let inbox = super::mail::inbox::read_snapshot(&snapshot, &caller)?;
        let mut requests = BTreeMap::new();
        for request in super::request::list_snapshot(&snapshot)? {
            if let RequestState::Open { holder } = request.state {
                *requests
                    .entry(holder.unwrap_or_else(|| ADMIRAL.to_owned()))
                    .or_insert(0) += 1;
            }
        }
        let reader = super::watch::set::reader_snapshot(&snapshot, now)?;
        let watches_on = super::watch::set::enabled_count(&snapshot)?;
        let recent = super::mail::log::read_snapshot(
            &snapshot,
            &LogQuery {
                caller: visible.as_deref(),
                seat: None,
                with: None,
                since: None,
                no_watch: false,
                limit: 5,
                receipts: None,
            },
        )?;
        Ok(View {
            roster,
            caller,
            inbox,
            requests,
            reader,
            watches_on,
            recent,
        })
    }
}

/// Read the status of the caller's room without changing it.
pub fn exec<R>(env: &Env, finish: impl FnOnce(Result<View, Error>) -> R) -> R {
    runtime::exec::<Status, R>(env, (), finish)
}
