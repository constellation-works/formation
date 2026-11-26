//! Best-effort wake after committing new mail.

use crate::composition::is_hook_room;
use crate::runtime::Ctx;
use cw_base::Error;
use formation_store::SessionId;

pub(super) fn after_commit(ctx: &Ctx<'_>, recipient: &str, queue: &dyn Fn(&str) -> bool) {
    match claim(ctx, recipient) {
        Ok(Some(session)) => {
            // A failed call may already have reached the local queue. Keep
            // the claim until lifecycle activity starts a new idle period.
            if !queue(session.as_str()) {
                tracing::warn!("Codex queue wake failed; mail remains available to the hook");
            }
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(
            code = error.code(),
            "could not check Codex wake eligibility"
        ),
    }
}

fn claim(ctx: &Ctx<'_>, recipient: &str) -> Result<Option<SessionId>, Error> {
    let mail = ctx.mail()?;
    let Some(session) = mail.store.read()?.seats().idle_codex(recipient)? else {
        return Ok(None);
    };

    // A session may have moved to another room without clearing this room's seat.
    let claimed = mail.store.write(|tx| {
        Ok(tx.messages().count_undelivered(recipient)? > 0
            && is_hook_room(ctx.env, &mail.room, &session)?
            && tx.seats().claim_codex_wake(recipient, &session)?)
    })?;
    Ok(claimed.then_some(session))
}
