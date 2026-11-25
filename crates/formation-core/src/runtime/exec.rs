//! Every app enters through the same composition, dispatch and audit boundary.
use super::{Ctx, audit::AuditGuard};
use crate::{Env, composition};
use cw_base::Error;
pub(crate) trait App {
    type Command;
    type Outcome;
    const NEEDS_ROOM: bool;
    fn audit(cmd: &Self::Command) -> Option<&'static str>;
    fn run(ctx: &Ctx<'_>, cmd: Self::Command) -> Result<Self::Outcome, Error>;
    fn target(_: &Self::Outcome) -> Option<String> {
        None
    }
}
pub(crate) fn exec<A: App, R>(
    env: &Env,
    cmd: A::Command,
    finish: impl FnOnce(Result<A::Outcome, Error>) -> R,
) -> R {
    let mail = if A::NEEDS_ROOM {
        match composition::compose(env) {
            Ok(ctx) => Some(ctx),
            Err(e) => return finish(Err(e)),
        }
    } else {
        None
    };
    let ctx = Ctx {
        mail,
        actor: env.actor.as_deref(),
        clock: time::OffsetDateTime::now_utc,
    };
    let mut guard = ctx
        .mail
        .as_ref()
        .and_then(|(store, _)| A::audit(&cmd).map(|name| AuditGuard::new(store, name, &ctx)));
    let result = A::run(&ctx, cmd);
    if let Some(guard) = &mut guard {
        guard.record(result.as_ref().map(A::target).map_err(Error::code));
    }
    finish(result)
}
