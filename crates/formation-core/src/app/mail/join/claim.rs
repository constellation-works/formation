//! One join's ownership of its session's route.

use super::{Error, SessionId, Store};
use cw_base::Kind;
use formation_store::{SessionPointer, SessionRoute};
use std::path::{Path, PathBuf};

/// One session's route, held for the length of a join.
///
/// The session lock is taken before the prior pointer is read and released only once the join
/// has committed or the pointer is back. Joins of one session therefore run one at a time,
/// across rooms, and a failed join cannot put a stale pointer over a later successful one.
/// Lock order: the session lock, then a room database's write lock. Dropping a claim that was
/// neither kept nor abandoned puts the prior pointer back.
pub(super) struct Claim {
    route: SessionRoute,
    prior: SessionPointer,
    replacing: bool,
}

impl Claim {
    /// Take the session's lock and read the pointer a failed join must put back.
    pub(super) fn take(sessions_dir: &Path, session: &SessionId) -> Result<Self, Error> {
        let route = Store::lock_session(sessions_dir, session)?;
        let prior = route.pointer()?;
        Ok(Self {
            route,
            prior,
            replacing: false,
        })
    }

    /// Point the session at `room`.
    pub(super) fn record(&mut self, room: &str) -> Result<(), Error> {
        // Marked first: a write that fails part way may already have replaced the pointer.
        self.replacing = true;
        self.route.record(room)
    }

    fn restore(&mut self) -> Result<(), Error> {
        if !std::mem::take(&mut self.replacing) {
            return Ok(());
        }
        self.route.restore(&self.prior)
    }

    /// The join committed: the recorded pointer stays.
    pub(super) fn keep(mut self) {
        self.replacing = false;
    }

    /// The join failed with `join`: put the prior pointer back while the lock is still held.
    /// Returns `join`, or an error naming both failures when the pointer cannot be put back.
    pub(super) fn abandon(mut self, join: Error) -> Error {
        match self.restore() {
            Ok(()) => join,
            Err(restore) => Unrestored {
                path: self.route.path().to_owned(),
                join,
                restore,
            }
            .into(),
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            tracing::warn!(error=%error, "could not restore the session route");
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "join failed: {join}; the session route {} could not be restored: {restore}; \
     it may name the room of the failed join, so fix the cause and run `formation mail join` \
     again",
    path.display()
)]
struct Unrestored {
    path: PathBuf,
    join: Error,
    restore: Error,
}

impl From<Unrestored> for Error {
    fn from(error: Unrestored) -> Self {
        Self::new(Kind::Io, "session_route_unrestored", error)
    }
}
