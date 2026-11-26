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
}

/// A successful roster check.
pub type Outcome = Summary;

struct Roster;

impl App for Roster {
    type Command = Command;

    type Outcome = Outcome;

    fn room_access(_: &Command) -> RoomAccess<'_> {
        RoomAccess::None
    }

    fn audit(_: &Command) -> Option<&'static str> {
        None
    }

    fn run(_: &Ctx<'_>, Command::Check(bytes): Command) -> Result<Outcome, cw_base::Error> {
        check(&bytes)
            .map_err(|error| cw_base::Error::new(cw_base::Kind::Usage, "invalid_roster", error))
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
