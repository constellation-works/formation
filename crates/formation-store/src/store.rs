//! The room facade. SQLite handles and file mechanics stay within this crate.

use crate::contract::SessionId;
use crate::driver::sqlite;
use crate::error::Failure;
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

    /// Read the room recorded for `session`, if any.
    pub fn session_room(sessions_dir: &Path, session: &SessionId) -> Result<Option<String>, Error> {
        crate::repository::session::read(sessions_dir, session).map_err(Into::into)
    }

    /// Record the room for `session`, creating the sessions directory if absent.
    pub fn record_session(
        sessions_dir: &Path,
        session: &SessionId,
        room: &str,
    ) -> Result<(), Error> {
        crate::repository::session::write(sessions_dir, session, room).map_err(Into::into)
    }

    /// Open one committed snapshot.
    pub fn read(&self) -> Result<Snapshot, Error> {
        Ok(Snapshot {
            reader: sqlite::open(&self.root).read().map_err(Failure::from)?,
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
