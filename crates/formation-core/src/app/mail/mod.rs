//! Local seat and inbox operations.

mod ack;

mod inbox;

mod join;

mod rule;

mod send;

mod who;

use crate::Env;
use crate::composition::Caller;
use crate::runtime;
use crate::runtime::App;
use crate::runtime::Ctx;
use crate::runtime::RoomAccess;
use cw_base::{Error, Kind};
use formation_store::SessionId;
use formation_store::Store;
use formation_store::Tx;
use formation_store::contract::Message;
use std::time::Duration;

/// A mail operation.
pub enum Command {
    /// Register a seat.
    Join(String),
    /// List online seats.
    Who,
    /// Submit an envelope.
    Send {
        /// Recipient.
        to: String,
        /// Request identifier.
        id: String,
        /// Contents.
        body: String,
    },
    /// Inspect the inbox with a bounded wait.
    Inbox(Duration),
    /// Acknowledge a receipt.
    Ack(i64),
}

/// Mail outcomes, independent of CLI presentation.
pub enum Outcome {
    /// Registered seat, with the replaced session id when taken over.
    Joined {
        /// Seat name that joined the room.
        seat: String,
        /// Previous session id when a different session took over the seat.
        replaced_session: Option<String>,
    },
    /// Online seat names.
    Online(Vec<String>),
    /// Request ID and receipt.
    Sent(String, i64),
    /// Pending envelopes.
    Inbox(Vec<Message>),
    /// Handled receipt.
    Ack(i64),
}

#[derive(Clone, Copy)]
pub(super) enum SeatRef<'a> {
    Name(&'a str),
    Session(&'a SessionId),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Refusal(&'static str);

fn refused(code: &'static str, message: &'static str) -> Error {
    Error::new(Kind::Refused, code, Refusal(message))
}

fn touch(tx: &Tx<'_>, caller: SeatRef<'_>, now: i64) -> Result<String, Error> {
    match caller {
        SeatRef::Name(name) => {
            if tx.seats().touch(name, now)? {
                Ok(name.to_owned())
            } else {
                Err(refused(
                    "unknown_seat",
                    "seat has not joined this room; run `formation mail join` first",
                ))
            }
        }
        SeatRef::Session(session) => tx.seats().touch_session(session, now)?.ok_or_else(|| {
            refused(
                "seat_required",
                "seat required: pass --as <seat> or join this room from your session",
            )
        }),
    }
}

struct Mail;

impl App for Mail {
    type Command = Command;

    type Outcome = Outcome;

    fn preflight(cmd: &Command) -> Result<(), Error> {
        if let Command::Join(name) = cmd {
            rule::name(name)?;
        }
        Ok(())
    }

    fn room_access(cmd: &Command) -> RoomAccess<'_> {
        match cmd {
            Command::Join(name) => RoomAccess::Join(name),
            Command::Who | Command::Send { .. } | Command::Inbox(_) | Command::Ack(_) => {
                RoomAccess::Existing
            }
        }
    }

    fn audit(cmd: &Command) -> Option<&'static str> {
        match cmd {
            Command::Join(_) => Some("mail join"),
            Command::Ack(_) => Some("mail ack"),
            Command::Send { .. } => Some("mail send"),
            Command::Who | Command::Inbox(_) => None,
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command) -> Result<Outcome, Error> {
        let mail = ctx.mail()?;
        let now = (ctx.clock)().unix_timestamp();

        if let Command::Join(name) = cmd {
            let Caller::Join {
                session,
                sessions_dir,
                room_record,
            } = &mail.caller
            else {
                return Err(Error::new(
                    Kind::Internal,
                    "missing_context",
                    std::io::Error::other("join context unavailable"),
                ));
            };
            let outcome = join::run(&mail.store, session.as_ref(), now, name)?;
            if let (Some(session), Some(sessions_dir)) = (session.as_ref(), sessions_dir.as_deref())
            {
                Store::record_session(sessions_dir, session, room_record)?;
            }
            return Ok(outcome);
        }

        let seat = match &mail.caller {
            Caller::Explicit(name) => SeatRef::Name(name),
            Caller::Session(session) => SeatRef::Session(session),
            Caller::Join { .. } => {
                return Err(Error::new(
                    Kind::Internal,
                    "missing_context",
                    std::io::Error::other("seat context unavailable"),
                ));
            }
        };

        match cmd {
            Command::Join(_) => Err(Error::new(
                Kind::Internal,
                "missing_context",
                std::io::Error::other("join already handled"),
            )),
            Command::Who => who::run(&mail.store, seat, now),
            Command::Send { to, id, body } => send::run(&mail.store, seat, now, &to, id, &body),
            Command::Inbox(wait) => inbox::run(ctx, &mail.store, seat, wait),
            Command::Ack(receipt) => ack::run(&mail.store, seat, now, receipt),
        }
    }

    fn target(out: &Outcome) -> Option<String> {
        match out {
            Outcome::Joined { seat, .. } => Some(seat.clone()),
            Outcome::Sent(_, receipt) | Outcome::Ack(receipt) => Some(receipt.to_string()),
            Outcome::Online(_) | Outcome::Inbox(_) => None,
        }
    }
}

/// Run one mail operation and hand its typed outcome to the surface.
pub fn exec<R>(env: &Env, cmd: Command, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<Mail, R>(env, cmd, finish)
}

#[cfg(test)]
mod tests;
