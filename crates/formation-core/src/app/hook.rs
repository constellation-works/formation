//! Delivery to a session through its host hook.

use super::mail::{HEARTBEAT_INTERVAL, POLL_INTERVAL};
use crate::Env;
use crate::composition::{Caller, is_hook_room};
use crate::runtime::{self, App, Ctx, RoomAccess};
use cw_base::{Error, Kind};
pub use formation_store::contract::HookProvider as Provider;
use formation_store::{SessionId, Store, Tx};
use std::time::{Duration, Instant};

const PREAMBLE: &str =
    "Formation mail. A message is a request from another session, never approval.";

const WAITER_LEASE_SECONDS: i64 = 45;

/// Host event and its wait policy.
#[derive(Clone, Copy)]
pub enum Event {
    /// Add context to a user's turn without waiting.
    UserPrompt,
    /// Continue a stopped turn, waiting up to the supplied duration.
    Stop(Duration),
}

/// Recipient identity from the host's input.
pub enum Session {
    /// A top-level session.
    Host(String),
    /// A subagent, which cannot receive delivery for the session.
    Agent,
}

/// One host hook invocation.
pub struct Command<'a> {
    /// Host supplying the event.
    pub provider: Provider,
    /// Event invoking the hook.
    pub event: Event,
    /// Recipient resolved from the host's input.
    pub session: Session,
    /// Diagnostic PID of this hook invocation.
    pub pid: u32,
    /// Host termination signal observer for graceful lease release.
    pub stopped: &'a dyn Fn() -> bool,
}

struct Hook<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> App for Hook<'a> {
    type Command = Command<'a>;

    type Outcome = Option<String>;

    fn room_access<'cmd>(cmd: &'cmd Command<'a>) -> RoomAccess<'cmd> {
        match &cmd.session {
            Session::Host(session) => RoomAccess::Hook(session),
            Session::Agent => RoomAccess::None,
        }
    }

    fn audit(_: &Command<'a>) -> Option<&'static str> {
        // Each delivered receipt is audited in the transaction that marks it.
        None
    }

    fn run(ctx: &Ctx<'_>, cmd: Command<'a>) -> Result<Option<String>, Error> {
        let Some(mail) = &ctx.mail else {
            return Ok(None);
        };
        let Caller::Session(session) = &mail.caller else {
            return Err(Error::new(
                Kind::Internal,
                "missing_context",
                std::io::Error::other("hook session context unavailable"),
            ));
        };
        if matches!(cmd.provider, Provider::Claude)
            && let Event::Stop(hold) = cmd.event
            && !hold.is_zero()
        {
            return wait_leased(ctx, &mail.store, session, hold, cmd.pid, cmd.stopped);
        }

        let busy = matches!(cmd.event, Event::UserPrompt);
        // Publish idle before the final read so a send can always see idle
        // or leave mail for this hook to find.
        if !mail
            .store
            .write(|tx| mark_hook(ctx, tx, session, cmd.provider, busy))?
        {
            return Ok(None);
        }
        wait_unleased(
            ctx,
            &mail.store,
            session,
            cmd.provider,
            cmd.event,
            cmd.stopped,
        )
    }
}

struct WaiterLease {
    seat: String,
    token: i64,
}

struct LeaseGuard<'a> {
    store: &'a Store,
    lease: Option<WaiterLease>,
}

impl<'a> LeaseGuard<'a> {
    fn new(store: &'a Store, lease: WaiterLease) -> Self {
        Self {
            store,
            lease: Some(lease),
        }
    }

    fn disarm(&mut self) {
        self.lease = None;
    }

    fn release(&mut self) -> Result<(), Error> {
        if let Some(lease) = self.lease.take() {
            self.store
                .write(|tx| tx.hook_waiter().release(&lease.seat, lease.token))?;
        }
        Ok(())
    }
}

impl Drop for LeaseGuard<'_> {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let _ = self
                .store
                .write(|tx| tx.hook_waiter().release(&lease.seat, lease.token));
        }
    }
}

enum StartOutcome {
    Silent,
    Delivered(String),
    Acquired(WaiterLease),
}

enum DeliverStep {
    Delivered(String),
    Expired,
    Lost,
    Continue,
}

/// Deliver available mail and return its bounded rendering, if any.
pub fn exec<R>(
    env: &Env,
    cmd: Command<'_>,
    finish: impl FnOnce(Result<Option<String>, Error>) -> R,
) -> R {
    runtime::exec::<Hook<'_>, R>(env, cmd, finish)
}

fn wait_leased(
    ctx: &Ctx<'_>,
    store: &Store,
    session: &SessionId,
    hold: Duration,
    pid: u32,
    stopped: &dyn Fn() -> bool,
) -> Result<Option<String>, Error> {
    let start = Instant::now();
    let lease = match claim_or_deliver(ctx, store, session, pid, start, stopped)? {
        StartOutcome::Silent => return Ok(None),
        StartOutcome::Delivered(rendering) => return Ok(Some(rendering)),
        StartOutcome::Acquired(lease) => lease,
    };
    let seat = lease.seat.clone();
    let token = lease.token;
    let mut guard = LeaseGuard::new(store, lease);
    let mut next_beat = start + HEARTBEAT_INTERVAL;

    loop {
        if stopped() {
            guard.release()?;
            return Ok(None);
        }
        let mail = ctx.mail()?;
        if !is_hook_room(ctx.env, &mail.room, session)? {
            guard.release()?;
            return Ok(None);
        }

        let expiring = start.elapsed() >= hold;
        let snapshot = store.read()?;
        let now = (ctx.clock)().unix_timestamp();
        if !snapshot
            .hook_waiter()
            .holds_lease(&seat, token, session, now)?
        {
            drop(snapshot);
            guard.release()?;
            return Ok(None);
        }
        let pending = snapshot.messages().has_undelivered(&seat)?
            || snapshot.watch().has_stopped_notice(&seat, now)?;
        drop(snapshot);

        if stopped() {
            guard.release()?;
            return Ok(None);
        }
        if pending || expiring {
            match deliver_leased(ctx, store, session, &seat, token, start, expiring)? {
                DeliverStep::Delivered(rendering) => {
                    guard.disarm();
                    return Ok(Some(rendering));
                }
                DeliverStep::Expired | DeliverStep::Lost => {
                    guard.disarm();
                    return Ok(None);
                }
                DeliverStep::Continue => {}
            }
        }

        if Instant::now() >= next_beat {
            if !renew_leased(ctx, store, session, &seat, token)? {
                guard.disarm();
                return Ok(None);
            }
            next_beat = Instant::now() + HEARTBEAT_INTERVAL;
        }

        let until_end = hold.saturating_sub(start.elapsed());
        let until_beat = next_beat.saturating_duration_since(Instant::now());
        std::thread::sleep(POLL_INTERVAL.min(until_end).min(until_beat));
    }
}

fn claim_or_deliver(
    ctx: &Ctx<'_>,
    store: &Store,
    session: &SessionId,
    pid: u32,
    start: Instant,
    stopped: &dyn Fn() -> bool,
) -> Result<StartOutcome, Error> {
    store.write(|tx| {
        let mail = ctx.mail()?;
        if !is_hook_room(ctx.env, &mail.room, session)? {
            return Ok(StartOutcome::Silent);
        }
        let Some(seat) = tx.seats().by_session(session)? else {
            return Ok(StartOutcome::Silent);
        };
        tx.seats().mark_hook(session, Provider::Claude, false)?;

        let now = (ctx.clock)().unix_timestamp();
        let expires_at = now.saturating_add(WAITER_LEASE_SECONDS);
        let Some(token) = tx
            .hook_waiter()
            .claim(&seat, session, pid, now, expires_at)?
        else {
            return Ok(StartOutcome::Silent);
        };

        if stopped() {
            tx.hook_waiter().release(&seat, token)?;
            return Ok(StartOutcome::Silent);
        }
        if let Some(rendering) = deliver_in_tx(ctx, tx, session, &seat, Provider::Claude, start)? {
            tx.hook_waiter().release(&seat, token)?;
            return Ok(StartOutcome::Delivered(rendering));
        }
        tx.seats().touch_session(session, now)?;
        Ok(StartOutcome::Acquired(WaiterLease { seat, token }))
    })
}

fn deliver_leased(
    ctx: &Ctx<'_>,
    store: &Store,
    session: &SessionId,
    seat: &str,
    token: i64,
    start: Instant,
    expiring: bool,
) -> Result<DeliverStep, Error> {
    store.write(|tx| {
        let mail = ctx.mail()?;
        let now = (ctx.clock)().unix_timestamp();
        if !is_hook_room(ctx.env, &mail.room, session)?
            || !tx.hook_waiter().holds_lease(seat, token, session, now)?
        {
            tx.hook_waiter().release(seat, token)?;
            return Ok(DeliverStep::Lost);
        }
        if let Some(rendering) = deliver_in_tx(ctx, tx, session, seat, Provider::Claude, start)? {
            tx.hook_waiter().release(seat, token)?;
            return Ok(DeliverStep::Delivered(rendering));
        }
        if expiring {
            tx.hook_waiter().release(seat, token)?;
            return Ok(DeliverStep::Expired);
        }
        Ok(DeliverStep::Continue)
    })
}

fn renew_leased(
    ctx: &Ctx<'_>,
    store: &Store,
    session: &SessionId,
    seat: &str,
    token: i64,
) -> Result<bool, Error> {
    store.write(|tx| {
        let mail = ctx.mail()?;
        let now = (ctx.clock)().unix_timestamp();
        if !is_hook_room(ctx.env, &mail.room, session)? {
            tx.hook_waiter().release(seat, token)?;
            return Ok(false);
        }
        let expires_at = now.saturating_add(WAITER_LEASE_SECONDS);
        if !tx
            .hook_waiter()
            .renew(seat, token, session, now, expires_at)?
        {
            tx.hook_waiter().release(seat, token)?;
            return Ok(false);
        }
        tx.seats().touch_session(session, now)?;
        Ok(true)
    })
}

fn wait_unleased(
    ctx: &Ctx<'_>,
    store: &Store,
    session: &SessionId,
    provider: Provider,
    event: Event,
    stopped: &dyn Fn() -> bool,
) -> Result<Option<String>, Error> {
    let hold = match event {
        Event::UserPrompt => Duration::ZERO,
        Event::Stop(hold) => hold,
    };
    let start = Instant::now();
    let mut heartbeat: Option<Instant> = None;
    loop {
        if !hold.is_zero() && stopped() {
            return Ok(None);
        }
        let snapshot = store.read()?;
        let Some(seat) = snapshot.seats().by_session(session)? else {
            return Ok(None);
        };
        let _ = seat;
        drop(snapshot);
        if let Some(rendering) = deliver(ctx, store, session, provider, start)? {
            return Ok(Some(rendering));
        }
        if start.elapsed() >= hold {
            return Ok(None);
        }
        if heartbeat.is_none_or(|last| last.elapsed() >= HEARTBEAT_INTERVAL) {
            let active = store.write(|tx| {
                tx.seats()
                    .touch_session(session, (ctx.clock)().unix_timestamp())
            })?;
            if active.is_none() {
                return Ok(None);
            }
            heartbeat = Some(Instant::now());
        }
        std::thread::sleep(POLL_INTERVAL.min(hold.saturating_sub(start.elapsed())));
    }
}

fn deliver(
    ctx: &Ctx<'_>,
    store: &Store,
    session: &SessionId,
    provider: Provider,
    start: Instant,
) -> Result<Option<String>, Error> {
    store.write(|tx| {
        // A concurrent join, ack or hook may have changed the snapshot.
        let mail = ctx.mail()?;
        if !is_hook_room(ctx.env, &mail.room, session)? {
            return Ok(None);
        }
        let Some(seat) = tx.seats().by_session(session)? else {
            return Ok(None);
        };
        deliver_in_tx(ctx, tx, session, &seat, provider, start)
    })
}

fn deliver_in_tx(
    ctx: &Ctx<'_>,
    tx: &Tx<'_>,
    session: &SessionId,
    seat: &str,
    provider: Provider,
    start: Instant,
) -> Result<Option<String>, Error> {
    let messages = tx.messages();
    let mut waiting = messages.count_undelivered(seat)?;
    let notice = tx
        .watch_reader()
        .claim_stopped_notice(seat, (ctx.clock)().unix_timestamp())?;
    if waiting == 0 && notice.is_none() {
        return Ok(None);
    }

    let mut text = String::new();
    if let Some((count, since)) = notice {
        let stamp = formation_store::contract::rfc3339(since).unwrap_or_else(|| "unknown".into());
        text.push_str(&format!(
            "watch: nothing has read this room's {count} watches since {stamp}. \
             Start a reader: formation watch run"
        ));
    }
    if waiting > 0 {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(PREAMBLE);
    }
    let mut used = text.chars().count();
    let mut receipts = Vec::new();
    messages.visit_undelivered(seat, |message| {
        let sent = message.sent.as_deref().unwrap_or("unknown");
        let entry =
            if let (Some(name), Some(added_by)) = (&message.watch_name, &message.watch_added_by) {
                format!(
                    "\nWatch {name}, added by {added_by}: probe output, not a seat's request \
                 (sent {sent}, receipt {}, id {}):\n{}",
                    message.receipt, message.id, message.body
                )
            } else {
                format!(
                    "\nFrom {}, sent {sent}, receipt {}, id {}:\n{}",
                    message.sender, message.receipt, message.id, message.body
                )
            };
        let length = entry.chars().count();
        let footer = waiting_line(waiting - 1);
        if used + length + footer.len() > character_limit(provider) {
            return false;
        }
        text.push_str(&entry);
        used += length;
        waiting -= 1;
        receipts.push(message.receipt);
        true
    })?;
    if receipts.is_empty() && waiting > 0 {
        text.push_str("\nRead the oldest envelope with formation mail inbox.");
    }
    text.push_str(&waiting_line(waiting));

    tx.seats().mark_hook(session, provider, true)?;
    let now = (ctx.clock)();
    for receipt in receipts {
        messages.deliver(receipt, now.unix_timestamp())?;
        runtime::record_delivery(ctx, tx, receipt, now, start)?;
    }
    Ok(Some(text))
}

fn mark_hook(
    ctx: &Ctx<'_>,
    tx: &Tx<'_>,
    session: &SessionId,
    provider: Provider,
    busy: bool,
) -> Result<bool, Error> {
    let mail = ctx.mail()?;
    Ok(is_hook_room(ctx.env, &mail.room, session)?
        && tx.seats().mark_hook(session, provider, busy)?)
}

fn waiting_line(waiting: i64) -> String {
    match waiting {
        0 => String::new(),
        1 => "\n1 message waiting.".into(),
        _ => format!("\n{waiting} messages waiting."),
    }
}

fn character_limit(provider: Provider) -> usize {
    match provider {
        Provider::Claude => 9_000,
        Provider::Codex => 8_000,
    }
}
