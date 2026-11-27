//! Offline roster application.

mod rule;

use crate::Env;
use crate::runtime;
use crate::runtime::App;
use crate::runtime::Ctx;
use crate::runtime::RoomAccess;
pub use rule::{Error, Summary, check};

/// Offline roster commands.
pub enum Command {
    /// Check a versioned roster.
    Check(Vec<u8>),
    /// Apply a version 2 roster to an existing room.
    Apply(Vec<u8>),
}

/// A successful roster check.
pub type Outcome = Summary;

struct Roster;

impl App for Roster {
    type Command = Command;

    type Outcome = Outcome;

    fn room_access(cmd: &Command) -> RoomAccess<'_> {
        match cmd {
            Command::Check(_) => RoomAccess::None,
            Command::Apply(_) => RoomAccess::Room,
        }
    }

    fn audit(cmd: &Command) -> Option<&'static str> {
        match cmd {
            Command::Check(_) => None,
            Command::Apply(_) => Some("roster apply"),
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command) -> Result<Outcome, cw_base::Error> {
        let (bytes, apply) = match cmd {
            Command::Check(bytes) => (bytes, false),
            Command::Apply(bytes) => (bytes, true),
        };
        let roster =
            formation_store::decode(&bytes).map_err(|error| roster_error(Error::Decode(error)))?;
        let summary = rule::validate(&roster).map_err(roster_error)?;
        if apply {
            if roster.version != 2 {
                return Err(cw_base::Error::new(
                    cw_base::Kind::Usage,
                    "roster_version_required",
                    std::io::Error::other("apply requires roster version 2"),
                ));
            }
            ctx.mail()?.store.write(|tx| tx.roster().replace(&roster))?;
        }
        Ok(summary)
    }
}

/// Run through the shared execution boundary without opening a room.
pub fn exec<R>(
    env: &Env,
    cmd: Command,
    finish: impl FnOnce(Result<Outcome, cw_base::Error>) -> R,
) -> R {
    runtime::exec::<Roster, R>(env, cmd, finish)
}

fn roster_error(error: Error) -> cw_base::Error {
    let code = match error {
        Error::UnknownParent => "unknown_parent",
        Error::ParentCycle => "roster_cycle",
        Error::Decode(_) | Error::Invalid(_) => "invalid_roster",
    };
    cw_base::Error::new(cw_base::Kind::Usage, code, error)
}
