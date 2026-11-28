//! The room facade. SQLite handles and file mechanics stay within this crate.

use crate::contract::SessionId;
use crate::driver::sqlite;
use crate::error::Failure;
use crate::repository::SessionRoute;
use crate::repository::Snapshot;
use crate::repository::Tx;
use cw_base::Error;
use std::path::{Path, PathBuf};

/// A private, same-user local room.
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Bind to an existing private room without creating files.
    pub fn open(root: &Path) -> Result<Self, Error> {
        if !cw_fs::inspect_dir(root).map_err(Failure::from)? {
            return Err(Failure::RoomNotFound(root.to_owned()).into());
        }
        if !cw_fs::inspect_file(&root.join("inbox.sqlite3")).map_err(Failure::from)? {
            return Err(Failure::RoomNotFound(root.to_owned()).into());
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }

    /// Open an existing private room or create it with owner-only permissions.
    pub fn create(root: &Path) -> Result<Self, Error> {
        if !cw_fs::inspect_dir(root).map_err(Failure::from)? {
            cw_fs::create_private_dir(root).map_err(Failure::from)?;
            let _ = cw_fs::inspect_dir(root).map_err(Failure::from)?;
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }

    /// List room directory entries under the supplied rooms root without creating it.
    pub fn room_names(rooms: &Path) -> Result<Vec<String>, Error> {
        cw_fs::list(rooms).map_err(|error| Failure::from(error).into())
    }

    /// Read the room recorded for `session`, if any.
    pub fn session_room(sessions_dir: &Path, session: &SessionId) -> Result<Option<String>, Error> {
        crate::repository::session::read(sessions_dir, session).map_err(Into::into)
    }

    /// Take ownership of `session`'s pointer, creating the sessions directory if absent.
    /// Every pointer write goes through the returned hold.
    ///
    /// # Errors
    /// Busy when another holder keeps the session past the lock deadline.
    pub fn lock_session(sessions_dir: &Path, session: &SessionId) -> Result<SessionRoute, Error> {
        SessionRoute::lock(sessions_dir, session).map_err(Into::into)
    }

    /// Create the seat's private folder in this room if missing and return its absolute path.
    ///
    /// The seat name must already be a valid identifier. Existing contents are never touched.
    pub fn seat_dir(&self, seat: &str) -> Result<PathBuf, Error> {
        crate::repository::seat_dir::ensure(&self.root, seat).map_err(Into::into)
    }

    /// Open one committed snapshot.
    pub fn read(&self) -> Result<Snapshot, Error> {
        Ok(Snapshot {
            reader: sqlite::open(&self.root).read().map_err(Failure::from)?,
        })
    }

    /// Open one committed snapshot without creating or updating WAL sidecars.
    pub fn read_strict(&self) -> Result<Snapshot, Error> {
        Ok(Snapshot {
            reader: sqlite::open(&self.root)
                .read_strict()
                .map_err(Failure::from)?,
        })
    }

    /// Commit one operation atomically; application refusals roll it back.
    pub fn write<T>(&self, apply: impl FnOnce(&Tx<'_>) -> Result<T, Error>) -> Result<T, Error> {
        struct Abort(Error);
        impl From<cw_sqlite::Error> for Abort {
            fn from(error: cw_sqlite::Error) -> Self {
                Self(Failure::from(error).into())
            }
        }
        sqlite::open(&self.root)
            .write(|conn| apply(&Tx { conn }).map_err(Abort))
            .map_err(|error| error.0)
    }
}
