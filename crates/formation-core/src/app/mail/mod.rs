//! Local seat and inbox operations.

mod ack;

mod inbox;

mod join;

mod rule;

mod send;

mod sent;

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
pub use formation_store::contract::SentMessage;
use std::path::PathBuf;
use std::time::Duration;

pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub(super) const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

/// A mail operation.
pub enum Command<'a> {
    /// Register a seat.
    Join(String),
    /// List every joined seat.
    Who,
    /// Submit an envelope.
    Send {
        /// Recipient.
        to: String,
        /// Request identifier.
        id: String,
        /// Contents.
        body: String,
        /// Queue a fixed wake for an idle Codex session.
        queue: &'a dyn Fn(&str) -> bool,
    },
    /// Inspect the inbox with a bounded wait.
    Inbox(Duration),
    /// List the caller's newest sent envelopes, up to the supplied limit.
    Sent(i64),
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
        /// Absolute path of the seat's folder in the room directory.
        seat_dir: PathBuf,
    },
    /// Every joined seat, ordered by name.
    Seats(Vec<SeatStatus>),
    /// A stored envelope.
    Sent {
        /// Request identifier.
        id: String,
        /// Durable receipt.
        receipt: i64,
        /// Whether a process was waiting on the recipient's mailbox when the send ran.
        held: bool,
    },
    /// Pending envelopes.
    Inbox(Vec<Message>),
    /// Sent envelopes and the total before applying the limit.
    SentHistory {
        /// Matching envelopes in newest-first order.
        messages: Vec<SentMessage>,
        /// Number of sent envelopes.
        total: i64,
    },
    /// Handled receipt.
    Ack(i64),
}

/// One joined seat as `mail who` lists it.
pub struct SeatStatus {
    /// Seat name.
    pub name: String,
    /// Envelopes addressed to the seat that no hook has delivered and it has not acknowledged.
    pub undelivered: i64,
    /// Envelopes addressed to the seat that it has not acknowledged, delivered or not.
    pub unacknowledged: i64,
    /// Whether a process was waiting on the seat's mailbox within the presence window.
    pub held: bool,
    /// When the seat last renewed its presence, as RFC 3339 UTC, or `None` when it never has.
    pub seen: Option<String>,
}

#[derive(Clone, Copy)]
pub(super) enum SeatRef<'a> {
    Name(&'a str),
    Session(&'a SessionId),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Refusal(String);

struct Mail<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> App for Mail<'a> {
    type Command = Command<'a>;

    type Outcome = Outcome;

    fn preflight(cmd: &Command<'a>) -> Result<(), Error> {
        if let Command::Join(name) = cmd {
            rule::name(name)?;
        }
        Ok(())
    }

    fn room_access<'cmd>(cmd: &'cmd Command<'a>) -> RoomAccess<'cmd> {
        match cmd {
            Command::Join(name) => RoomAccess::Join(name),
            Command::Who
            | Command::Send { .. }
            | Command::Inbox(_)
            | Command::Sent(_)
            | Command::Ack(_) => RoomAccess::Existing,
        }
    }

    fn audit(cmd: &Command<'a>) -> Option<&'static str> {
        match cmd {
            Command::Join(_) => Some("mail join"),
            Command::Ack(_) => Some("mail ack"),
            Command::Send { .. } => Some("mail send"),
            Command::Who | Command::Inbox(_) | Command::Sent(_) => None,
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command<'a>) -> Result<Outcome, Error> {
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
            let route = sessions_dir.as_deref().map(|sessions_dir| join::Route {
                sessions_dir,
                room: room_record,
            });
            let outcome = join::run(&mail.store, session.as_ref(), name, route)?;
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
            Command::Send {
                to,
                id,
                body,
                queue,
            } => {
                let (outcome, inserted) = send::commit(&mail.store, seat, now, &to, id, &body)?;
                if inserted {
                    super::wake::after_commit(ctx, &to, queue);
                }
                Ok(outcome)
            }
            Command::Inbox(wait) => inbox::run(ctx, &mail.store, seat, wait),
            Command::Sent(limit) => sent::run(&mail.store, seat, limit),
            Command::Ack(receipt) => ack::run(&mail.store, seat, now, receipt),
        }
    }

    fn target(out: &Outcome) -> Option<String> {
        match out {
            Outcome::Joined { seat, .. } => Some(seat.clone()),
            Outcome::Sent { receipt, .. } | Outcome::Ack(receipt) => Some(receipt.to_string()),
            Outcome::Seats(_) | Outcome::Inbox(_) | Outcome::SentHistory { .. } => None,
        }
    }
}

/// Run one mail operation and hand its typed outcome to the surface.
pub fn exec<R>(env: &Env, cmd: Command<'_>, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<Mail<'_>, R>(env, cmd, finish)
}

fn refused(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(Kind::Refused, code, Refusal(message.into()))
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

#[cfg(test)]
mod tests;
