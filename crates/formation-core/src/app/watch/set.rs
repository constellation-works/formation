//! The room's watch set: add, remove, pause, resume and list.

use super::poll::{LISTED_SEATS, unknown_recipient};
use super::{Error, SeatRef, caller_ref, refused, rule};
use crate::Env;
use crate::runtime;
use crate::runtime::{App, Ctx, RoomAccess};
use formation_store::contract::{NewWatch, WatchEntry, WatchState, rfc3339};
use formation_store::{Store, Tx};
use std::time::Duration;

/// A watch set operation.
pub enum Command {
    /// Add a watch, or replace the one with this recipient and name.
    Add {
        /// Watch name.
        name: String,
        /// Joined recipient seat; the calling seat when absent.
        to: Option<String>,
        /// Interval between reads (minimum 10 seconds).
        every: Duration,
        /// Command words: one word is shell text, several are quoted into one command line.
        command: Vec<String>,
    },
    /// Drop a watch and its remembered fingerprints.
    Remove {
        /// Watch name.
        name: String,
        /// Joined recipient seat; the calling seat when absent.
        to: Option<String>,
    },
    /// Keep a watch but stop reading it.
    Pause {
        /// Watch name.
        name: String,
        /// Joined recipient seat; the calling seat when absent.
        to: Option<String>,
    },
    /// Read a paused watch again.
    Resume {
        /// Watch name.
        name: String,
        /// Joined recipient seat; the calling seat when absent.
        to: Option<String>,
    },
    /// List the room's watches.
    List,
}

/// The state of the process that reads the set.
pub enum Reader {
    /// No process is recorded as reading the set.
    None,
}

/// One watch as listed.
pub struct Entry {
    /// Watch name.
    pub name: String,
    /// Recipient seat.
    pub to: String,
    /// Command run through `sh -c`.
    pub command: String,
    /// Working directory the command runs in.
    pub cwd: String,
    /// Seconds between reads.
    pub every: i64,
    /// Whether the reader runs it.
    pub state: WatchState,
    /// Seat that set the watch.
    pub added_by: String,
    /// When it was set, as RFC 3339 UTC.
    pub added_at: Option<String>,
    /// When the reader last read it, as RFC 3339 UTC, or `None` when it has not.
    pub last_read: Option<String>,
    /// What the last read found, or `None` when there has been none.
    pub last_result: Option<String>,
    /// Fingerprints remembered for the watch.
    pub sent: i64,
}

/// Watch set outcomes, independent of CLI presentation.
pub enum Outcome {
    /// A watch was added or replaced.
    Added {
        /// Watch name.
        name: String,
        /// Recipient seat.
        to: String,
        /// Whether an existing watch was replaced.
        replaced: bool,
        /// Seconds between reads.
        every: u64,
        /// Command run through `sh -c`.
        command: String,
        /// Working directory the command runs in.
        cwd: String,
    },
    /// A watch was dropped.
    Removed {
        /// Watch name.
        name: String,
        /// Recipient seat.
        to: String,
        /// Fingerprints forgotten with it.
        forgotten: usize,
    },
    /// A watch was paused or resumed.
    State {
        /// Watch name.
        name: String,
        /// Recipient seat.
        to: String,
        /// The state the watch is now in.
        state: WatchState,
        /// Whether the call changed it.
        changed: bool,
    },
    /// The room's watches, ordered by name and then recipient.
    Listed {
        /// Who reads the set.
        reader: Reader,
        /// The watches.
        watches: Vec<Entry>,
    },
}

struct Set;

impl App for Set {
    type Command = Command;

    type Outcome = Outcome;

    fn preflight(cmd: &Command) -> Result<(), Error> {
        match cmd {
            Command::Add {
                name,
                to,
                every,
                command,
            } => {
                rule::watch_name(name)?;
                rule::every(*every)?;
                if let Some(to) = to {
                    rule::seat_name(to)?;
                }
                rule::watch_command(command)?;
            }
            Command::Remove { name, to }
            | Command::Pause { name, to }
            | Command::Resume { name, to } => {
                rule::watch_name(name)?;
                if let Some(to) = to {
                    rule::seat_name(to)?;
                }
            }
            Command::List => {}
        }
        Ok(())
    }

    fn room_access(_: &Command) -> RoomAccess<'_> {
        RoomAccess::Existing
    }

    fn audit(cmd: &Command) -> Option<&'static str> {
        match cmd {
            Command::Add { .. } => Some("watch add"),
            Command::Remove { .. } => Some("watch remove"),
            Command::Pause { .. } => Some("watch pause"),
            Command::Resume { .. } => Some("watch resume"),
            Command::List => None,
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command) -> Result<Outcome, Error> {
        let mail = ctx.mail()?;
        let seat = caller_ref(&mail.caller)?;
        let store = &mail.store;
        let now = (ctx.clock)().unix_timestamp();
        match cmd {
            Command::Add {
                name,
                to,
                every,
                command,
            } => add(
                store,
                seat,
                now,
                Request {
                    name,
                    to,
                    every,
                    command: rule::watch_command(&command)?,
                    cwd: rule::watch_cwd(ctx.cwd)?.to_owned(),
                },
            ),
            Command::Remove { name, to } => remove(store, seat, name, to),
            Command::Pause { name, to } => set_state(store, seat, name, to, WatchState::Paused),
            Command::Resume { name, to } => set_state(store, seat, name, to, WatchState::On),
            Command::List => list(store, seat),
        }
    }

    fn target(out: &Outcome) -> Option<String> {
        match out {
            Outcome::Added { name, to, .. }
            | Outcome::Removed { name, to, .. }
            | Outcome::State { name, to, .. } => Some(format!("{to}/{name}")),
            Outcome::Listed { .. } => None,
        }
    }
}

/// Run a watch set operation and hand its typed outcome to the CLI surface.
pub fn exec<R>(env: &Env, cmd: Command, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<Set, R>(env, cmd, finish)
}

struct Request {
    name: String,
    to: Option<String>,
    every: Duration,
    command: String,
    cwd: String,
}

fn add(store: &Store, seat: SeatRef<'_>, now: i64, request: Request) -> Result<Outcome, Error> {
    let every = request.every.as_secs();
    let stored_every =
        i64::try_from(every).map_err(|_| refused("invalid_interval", "--every is too large"))?;
    store.write(|tx| {
        let caller = caller_in(tx, seat)?;
        let recipient = recipient_in(tx, &caller, request.to.clone())?;
        let replaced = tx.watch().put(&NewWatch {
            recipient: &recipient,
            name: &request.name,
            command: &request.command,
            cwd: &request.cwd,
            every: stored_every,
            added_by: &caller,
            added_at: now,
        })?;
        Ok(Outcome::Added {
            name: request.name.clone(),
            to: recipient,
            replaced,
            every,
            command: request.command.clone(),
            cwd: request.cwd.clone(),
        })
    })
}

fn remove(
    store: &Store,
    seat: SeatRef<'_>,
    name: String,
    to: Option<String>,
) -> Result<Outcome, Error> {
    store.write(|tx| {
        let caller = caller_in(tx, seat)?;
        let recipient = recipient_in(tx, &caller, to)?;
        let forgotten = tx
            .watch()
            .remove(&recipient, &name)?
            .ok_or_else(|| unknown_watch(&recipient, &name))?;
        Ok(Outcome::Removed {
            name: name.clone(),
            to: recipient,
            forgotten,
        })
    })
}

fn set_state(
    store: &Store,
    seat: SeatRef<'_>,
    name: String,
    to: Option<String>,
    state: WatchState,
) -> Result<Outcome, Error> {
    store.write(|tx| {
        let caller = caller_in(tx, seat)?;
        let recipient = recipient_in(tx, &caller, to)?;
        let changed = tx
            .watch()
            .set_state(&recipient, &name, state)?
            .ok_or_else(|| unknown_watch(&recipient, &name))?;
        Ok(Outcome::State {
            name: name.clone(),
            to: recipient,
            state,
            changed,
        })
    })
}

fn list(store: &Store, seat: SeatRef<'_>) -> Result<Outcome, Error> {
    let snapshot = store.read()?;
    let joined = match seat {
        SeatRef::Name(name) => snapshot.seats().contains(name)?,
        SeatRef::Session(session) => snapshot.seats().by_session(session)?.is_some(),
    };
    if !joined {
        return Err(unjoined(seat));
    }
    Ok(Outcome::Listed {
        reader: Reader::None,
        watches: snapshot.watch().list()?.into_iter().map(entry).collect(),
    })
}

fn entry(watch: WatchEntry) -> Entry {
    Entry {
        name: watch.name,
        to: watch.recipient,
        command: watch.command,
        cwd: watch.cwd,
        every: watch.every,
        state: watch.state,
        added_by: watch.added_by,
        added_at: rfc3339(watch.added_at),
        last_read: watch.last_read.and_then(rfc3339),
        last_result: watch.last_result,
        sent: watch.remembered,
    }
}

fn caller_in(tx: &Tx<'_>, seat: SeatRef<'_>) -> Result<String, Error> {
    match seat {
        SeatRef::Name(name) => match tx.seats().seen(name)? {
            Some(_) => Ok(name.to_owned()),
            None => Err(unjoined(seat)),
        },
        SeatRef::Session(session) => tx
            .seats()
            .by_session(session)?
            .ok_or_else(|| unjoined(seat)),
    }
}

fn recipient_in(tx: &Tx<'_>, caller: &str, to: Option<String>) -> Result<String, Error> {
    let Some(to) = to else {
        return Ok(caller.to_owned());
    };
    if tx.seats().seen(&to)?.is_none() {
        return Err(unknown_recipient(tx.seats().names(LISTED_SEATS + 1)?, &to));
    }
    Ok(to)
}

fn unjoined(seat: SeatRef<'_>) -> Error {
    match seat {
        SeatRef::Name(_) => refused(
            "unknown_seat",
            "seat has not joined this room; run `formation mail join` first",
        ),
        SeatRef::Session(_) => refused(
            "seat_required",
            "seat required: pass --as <seat> or join this room from your session",
        ),
    }
}

fn unknown_watch(recipient: &str, name: &str) -> Error {
    refused(
        "unknown_watch",
        format!("no watch `{name}` for {recipient}; run `formation watch list` to see the set"),
    )
}
