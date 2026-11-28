//! Every app enters through the same composition, dispatch and audit boundary.

use super::Ctx;
use super::audit::AuditGuard;
use crate::composition::Caller;
use crate::{Env, composition};
use cw_base::Error;

pub(crate) enum RoomAccess<'a> {
    None,
    Join(&'a str),
    Existing,
    /// An existing room whose caller may also be the admiral.
    Member,
    OptionalSeat,
    /// An existing room whose caller is optional and may be the admiral.
    OptionalMember,
    Hook(&'a str),
}

pub(crate) trait App {
    type Command;

    type Outcome;

    fn preflight(_: &Self::Command) -> Result<(), Error> {
        Ok(())
    }

    fn room_access(cmd: &Self::Command) -> RoomAccess<'_>;

    fn audit(cmd: &Self::Command) -> Option<&'static str>;

    /// The audit name of a command the admiral calls. An app that admits the admiral also names
    /// here the commands that write for every caller but audit no seat's call.
    fn admiral_audit(cmd: &Self::Command) -> Option<&'static str> {
        Self::audit(cmd)
    }

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
    if let Err(error) = A::preflight(&cmd) {
        return finish(Err(error));
    }
    let mail = match A::room_access(&cmd) {
        RoomAccess::None => None,
        RoomAccess::Join(seat) => match composition::compose_join(env, seat) {
            Ok(ctx) => Some(ctx),
            Err(error) => return finish(Err(error)),
        },
        RoomAccess::Existing => match composition::compose_existing(env, false) {
            Ok(ctx) => Some(ctx),
            Err(error) => return finish(Err(error)),
        },
        RoomAccess::Member => match composition::compose_existing(env, true) {
            Ok(ctx) => Some(ctx),
            Err(error) => return finish(Err(error)),
        },
        RoomAccess::OptionalSeat => match composition::compose_optional_seat(env, false) {
            Ok(ctx) => Some(ctx),
            Err(error) => return finish(Err(error)),
        },
        RoomAccess::OptionalMember => match composition::compose_optional_seat(env, true) {
            Ok(ctx) => Some(ctx),
            Err(error) => return finish(Err(error)),
        },
        RoomAccess::Hook(session) => match composition::compose_hook(env, session) {
            Ok(ctx) => ctx,
            Err(error) => return finish(Err(error)),
        },
    };
    let ctx = Ctx {
        env,
        mail,
        actor: env.actor.as_deref(),
        cwd: env.cwd.as_deref(),
        clock: time::OffsetDateTime::now_utc,
    };
    let mut guard = ctx.mail.as_ref().and_then(|mail| {
        let name = if matches!(mail.caller, Caller::Admiral) {
            A::admiral_audit(&cmd)
        } else {
            A::audit(&cmd)
        };
        name.map(|name| AuditGuard::new(&mail.store, name, &ctx))
    });
    let result = A::run(&ctx, cmd);
    if let Some(guard) = &mut guard {
        guard.record(result.as_ref().map(A::target).map_err(Error::code));
    }
    finish(result)
}
