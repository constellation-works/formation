//! The room facade. SQLite handles and file mechanics stay within this crate.
use crate::{
    driver::sqlite,
    error::Failure,
    repository::{Snapshot, Tx},
};
use cw_base::Error;
use std::path::{Path, PathBuf};

/// A private, same-user local room.
pub struct Store {
    root: PathBuf,
}
impl Store {
    /// Bind to an existing private room.
    pub fn open(root: &Path) -> Result<Self, Error> {
        if !cw_fs::inspect_dir(root).map_err(Failure::from)? {
            return Err(Failure::MissingRoom.into());
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }
    /// Obtain this seat's credential digest. Only join may create the identity.
    pub fn identity(&self, path: &Path, create: bool) -> Result<Vec<u8>, Error> {
        crate::repository::identities::digest(path, create).map_err(Into::into)
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
            fn from(e: cw_sqlite::Error) -> Self {
                Self(Failure::from(e).into())
            }
        }
        sqlite::open(&self.root)
            .write(|conn| apply(&Tx { conn }).map_err(Abort))
            .map_err(|e| e.0)
    }
}
