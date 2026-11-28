//! Room discovery and close decisions.

use super::caller_seat;
use crate::composition;
use crate::runtime::{Ctx, held};
use cw_base::{Error, Kind};
use formation_store::Store;
use formation_store::contract::rfc3339;
use std::collections::BTreeSet;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The two room lifecycle states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// No close has been recorded.
    Active,
    /// The first close has been recorded.
    Closed,
}

impl Status {
    pub(super) fn from_closed(closed: bool) -> Self {
        if closed { Self::Closed } else { Self::Active }
    }

    /// The public status token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Closed => "closed",
        }
    }
}

/// The room and its first close.
#[derive(serde::Serialize)]
pub struct Close {
    /// Room name or path.
    pub room: String,
    /// Close timestamp.
    pub closed_at: String,
    /// Closing seat.
    pub closed_by: String,
}

/// Counts for the list's seats column.
#[derive(serde::Serialize)]
pub struct SeatCounts {
    /// Planned seats whose last presence is still fresh.
    pub held: usize,
    /// Planned seat count, absent when no roster has been applied.
    pub planned: Option<usize>,
    /// Joined seats, including any outside the roster.
    pub joined: usize,
}

/// A room's lifecycle and current presence summary.
#[derive(serde::Serialize)]
pub struct Room {
    /// Room name or path.
    pub room: String,
    /// Lifecycle status.
    pub status: Status,
    /// Opening timestamp, if recorded.
    pub opened_at: Option<String>,
    /// Close timestamp, if closed.
    pub closed_at: Option<String>,
    /// Closing seat, if closed.
    pub closed_by: Option<String>,
    /// Planned, held and joined seat counts.
    pub seats: SeatCounts,
    /// Workspaces named by the roster, in alphabetical order.
    pub workspaces: Vec<String>,
    /// Latest presence of any joined seat.
    pub last_seen: Option<String>,
}

/// A room that could not be included in the listing.
pub struct Unreadable {
    /// Room name or path.
    pub room: String,
    /// Why the room could not be read.
    pub error: Error,
}

/// The result of scanning rooms under the Formation home.
pub struct Listing {
    /// Rooms matching the filter, newest first.
    pub rooms: Vec<Room>,
    /// Rooms that could not be read.
    pub unreadable: Vec<Unreadable>,
    /// Requested status, if any.
    pub filter: Option<Status>,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    #[error("room {0} is closed")]
    RoomClosed(String),
    #[error("join room {0} from your session before closing it")]
    SeatRequired(String),
    #[error("room {0} has an out-of-range close timestamp")]
    InvalidCloseTime(String),
    #[error("room {0} has an invalid opening timestamp")]
    InvalidOpenTime(String, #[source] time::error::Parse),
}

impl From<Failure> for Error {
    fn from(error: Failure) -> Self {
        let (kind, code) = match &error {
            Failure::RoomClosed(_) => (Kind::Refused, "room_closed"),
            Failure::SeatRequired(_) => (Kind::Refused, "seat_required"),
            Failure::InvalidCloseTime(_) => (Kind::Io, "invalid_close_time"),
            Failure::InvalidOpenTime(..) => (Kind::Io, "invalid_open_time"),
        };
        Self::new(kind, code, error)
    }
}

pub(super) fn close(ctx: &Ctx<'_>) -> Result<Close, Error> {
    let mail = ctx.mail()?;
    // A prior close can be reported without opening a writer. The transaction
    // rechecks the stamp when a strict snapshot is unavailable or still active.
    let (at, closed_by) = if let Ok(snapshot) = mail.store.read_strict()
        && let Some(stamp) = snapshot.closure()?
    {
        stamp
    } else {
        mail.store.write(|tx| {
            if let Some(stamp) = tx.closure()? {
                return Ok(stamp);
            }
            let seat = caller_seat(tx, &mail.caller)?
                .ok_or_else(|| Failure::SeatRequired(mail.room.clone()))?;
            if tx.seats().seen(&seat)?.is_none() {
                return Err(Failure::SeatRequired(mail.room.clone()).into());
            }
            let now = (ctx.clock)().unix_timestamp();
            tx.close_room(now, &seat)?;
            Ok((now, seat))
        })?
    };
    let closed_at = rfc3339(at).ok_or_else(|| Failure::InvalidCloseTime(mail.room.clone()))?;
    Ok(Close {
        room: mail.room.clone(),
        closed_at,
        closed_by,
    })
}

pub(super) fn list(ctx: &Ctx<'_>, filter: Option<Status>) -> Result<Listing, Error> {
    let now = (ctx.clock)().unix_timestamp();
    let mut rooms = Vec::new();
    let mut unreadable = Vec::new();
    for candidate in composition::discover_rooms(ctx.env)? {
        let room = candidate.name;
        match candidate
            .store
            .and_then(|store| summarize(&room, &store, now))
        {
            Ok(item) if filter.is_none_or(|status| item.1.status == status) => rooms.push(item),
            Ok(_) => {}
            Err(error) => unreadable.push(Unreadable { room, error }),
        }
    }
    rooms.sort_unstable_by(|(left_at, left), (right_at, right)| {
        right_at
            .cmp(left_at)
            .then_with(|| left.room.cmp(&right.room))
    });
    Ok(Listing {
        rooms: rooms.into_iter().map(|(_, room)| room).collect(),
        unreadable,
        filter,
    })
}

pub(super) fn closure_fields(closure: Option<(i64, String)>) -> (Option<String>, Option<String>) {
    match closure {
        Some((at, by)) => (rfc3339(at), Some(by)),
        None => (None, None),
    }
}

fn summarize(room: &str, store: &Store, now: i64) -> Result<(Option<OffsetDateTime>, Room), Error> {
    let snapshot = store.read_strict()?;
    let closure = snapshot.closure()?;
    let status = Status::from_closed(closure.is_some());
    let roster = snapshot.roster().seats()?;
    let application = snapshot.roster().application()?;
    let opened_at = match application {
        Some((at, _)) => rfc3339(at),
        None => snapshot.first_join()?,
    };
    let opened = opened_at
        .as_deref()
        .map(|stamp| OffsetDateTime::parse(stamp, &Rfc3339))
        .transpose()
        .map_err(|error| Failure::InvalidOpenTime(room.to_owned(), error))?;
    let planned = (application.is_some() || !roster.is_empty()).then_some(roster.len());
    let mut presence = snapshot.seats().presence()?;
    presence.retain(|(name, _)| name != formation_store::contract::ADMIRAL);
    let held = presence
        .iter()
        .filter(|(name, seen)| {
            held(*seen, now) && roster.binary_search_by(|seat| seat.name.cmp(name)).is_ok()
        })
        .count();
    let last_seen = presence
        .iter()
        .map(|(_, seen)| *seen)
        .max()
        .filter(|seen| *seen > 0)
        .and_then(rfc3339);
    let workspaces: BTreeSet<_> = roster
        .into_iter()
        .filter_map(|seat| seat.workspace)
        .collect();
    let (closed_at, closed_by) = closure_fields(closure);
    Ok((
        opened,
        Room {
            room: room.to_owned(),
            status,
            opened_at: opened.and_then(|at| rfc3339(at.unix_timestamp())),
            closed_at,
            closed_by,
            seats: SeatCounts {
                held,
                planned,
                joined: presence.len(),
            },
            workspaces: workspaces.into_iter().collect(),
            last_seen,
        },
    ))
}
