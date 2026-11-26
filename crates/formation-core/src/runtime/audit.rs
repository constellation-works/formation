//! Fail-open audit guard owned by the runtime.

use super::Ctx;
use cw_base::Error;
use formation_store::contract::AuditEvent;
use formation_store::contract::AuditStatus;
use formation_store::{Store, Tx};
use std::time::Instant;
use time::OffsetDateTime;

pub(super) struct AuditGuard<'a> {
    store: &'a Store,
    event: AuditEvent,
    start: Instant,
}

impl<'a> AuditGuard<'a> {
    pub(super) fn new(store: &'a Store, command: &str, ctx: &Ctx<'_>) -> Self {
        Self {
            store,
            event: event(ctx, command, (ctx.clock)()),
            start: Instant::now(),
        }
    }

    pub(super) fn record(&mut self, result: Result<Option<String>, &str>) {
        match result {
            Ok(target) => {
                self.event.status = AuditStatus::Success;
                self.event.target = target;
            }
            Err(code) => self.event.error_code = Some(code.into()),
        }
    }
}

impl Drop for AuditGuard<'_> {
    fn drop(&mut self) {
        self.event.duration_ms = duration_ms(self.start);
        if let Err(err) = self.store.write(|tx| tx.audit().append(&self.event)) {
            tracing::warn!(error=%err,"could not record Formation audit event");
        }
    }
}

pub(crate) fn record_delivery(
    ctx: &Ctx<'_>,
    tx: &Tx<'_>,
    receipt: i64,
    at: OffsetDateTime,
    start: Instant,
) -> Result<(), Error> {
    let mut event = event(ctx, "hook delivery", at);
    event.status = AuditStatus::Success;
    event.target = Some(receipt.to_string());
    event.duration_ms = duration_ms(start);
    tx.audit().append(&event)
}

fn event(ctx: &Ctx<'_>, command: &str, at: OffsetDateTime) -> AuditEvent {
    AuditEvent {
        at,
        actor: ctx.actor.map(str::to_owned),
        command: command.into(),
        status: AuditStatus::Failure,
        target: None,
        error_code: None,
        duration_ms: 0,
    }
}

fn duration_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}
