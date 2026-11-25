//! Local seat and inbox operations.
mod ack;
mod inbox;
mod join;
mod rules;
mod send;
mod who;
use crate::{
    Env,
    runtime::{self, App, Ctx},
};
use cw_base::{Error, Kind};
use formation_store::{Store, Tx, contracts::Message};
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
    /// Registered seat.
    Joined(String),
    /// Online seat names.
    Online(Vec<String>),
    /// Request ID and receipt.
    Sent(String, i64),
    /// Pending envelopes.
    Inbox(Vec<Message>),
    /// Handled receipt.
    Ack(i64),
}
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Refusal(&'static str);
fn refused(code: &'static str, message: &'static str) -> Error {
    Error::new(Kind::Refused, code, Refusal(message))
}
fn touch(tx: &Tx<'_>, key: &[u8], now: i64) -> Result<String, Error> {
    tx.seats()
        .touch(key, now)?
        .ok_or_else(|| refused("unknown_identity", "join this room first"))
}
struct Mail;
impl App for Mail {
    const NEEDS_ROOM: bool = true;
    type Command = Command;
    type Outcome = Outcome;
    fn audit(cmd: &Command) -> Option<&'static str> {
        match cmd {
            Command::Join(_) => Some("mail join"),
            Command::Ack(_) => Some("mail ack"),
            Command::Send { .. } => Some("mail send"),
            Command::Who | Command::Inbox(_) => None,
        }
    }
    fn run(ctx: &Ctx<'_>, cmd: Command) -> Result<Outcome, Error> {
        let (store, identity) = ctx.mail()?;
        if let Command::Join(name) = &cmd {
            rules::name(name)?;
        }
        let key = store.identity(identity, matches!(cmd, Command::Join(_)))?;
        let now = (ctx.clock)().unix_timestamp();
        match cmd {
            Command::Join(name) => join::run(store, &key, now, name),
            Command::Who => who::run(store, &key, now),
            Command::Send { to, id, body } => send::run(store, &key, now, &to, id, &body),
            Command::Inbox(wait) => inbox::run(ctx, store, &key, wait),
            Command::Ack(receipt) => ack::run(store, &key, now, receipt),
        }
    }
    fn target(out: &Outcome) -> Option<String> {
        match out {
            Outcome::Joined(s) => Some(s.clone()),
            Outcome::Sent(_, r) | Outcome::Ack(r) => Some(r.to_string()),
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
