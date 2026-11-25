//! Fail-open audit guard owned by the runtime.

use super::Ctx;
use formation_store::Store;
use formation_store::contract::AuditEvent;
use formation_store::contract::AuditStatus;
use std::time::Instant;

pub(super) struct AuditGuard<'a> {
    store: &'a Store,
    event: AuditEvent,
    start: Instant,
}

impl<'a> AuditGuard<'a> {
    pub(super) fn new(store: &'a Store, command: &str, ctx: &Ctx<'_>) -> Self {
        Self {
            store,
            start: Instant::now(),
            event: AuditEvent {
                at: (ctx.clock)(),
                actor: ctx.actor.map(str::to_owned),
                command: command.into(),
                status: AuditStatus::Failure,
                target: None,
                error_code: None,
                duration_ms: 0,
            },
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
        self.event.duration_ms =
            u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX);
        if let Err(err) = self.store.write(|tx| tx.audit().append(&self.event)) {
            tracing::warn!(error=%err,"could not record Formation audit event");
        }
    }
}
