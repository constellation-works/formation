//! Private session-to-room pointer files under `<home>/sessions/`.
//!
//! A pointer is `<session id>`. Writers hold the advisory lock on `<session id>.lock` beside
//! it, which is never deleted; a session id cannot hold a `.`, so the two names never collide.
//! Readers take no lock: a pointer is replaced atomically.

use crate::contract::SessionId;
use crate::error::Failure;
use cw_base::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Longer than the room database's lock wait, so a join queued behind one that is waiting for
/// its room outlasts it.
const LOCK_WAIT: Duration = Duration::from_secs(15);

const MAX_SESSION_BYTES: u64 = 4096;

/// A pointer file's bytes as they were read, or its absence.
pub struct SessionPointer(Option<Vec<u8>>);

/// Exclusive ownership of one session's pointer, released when dropped.
pub struct SessionRoute {
    path: PathBuf,
    _lock: cw_fs::Lock,
}

impl SessionRoute {
    pub(crate) fn lock(sessions_dir: &Path, session: &SessionId) -> Result<Self, Failure> {
        if !cw_fs::inspect_dir(sessions_dir)? {
            cw_fs::create_private_dir(sessions_dir)?;
            let _ = cw_fs::inspect_dir(sessions_dir)?;
        }

        let lock_path = sessions_dir.join(format!("{}.lock", session.as_str()));
        let lock = cw_fs::Lock::acquire(&lock_path, "mail join", LOCK_WAIT)?;
        Ok(Self {
            path: sessions_dir.join(session.as_str()),
            _lock: lock,
        })
    }

    /// The pointer file this hold owns.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the pointer exactly as it stands.
    ///
    /// # Errors
    /// Refuses a pointer that is not a private regular file within the size cap.
    pub fn pointer(&self) -> Result<SessionPointer, Error> {
        Ok(SessionPointer(bytes(&self.path)?))
    }

    /// Point the session at `room`.
    ///
    /// # Errors
    /// Fails when the pointer cannot be written durably.
    pub fn record(&self, room: &str) -> Result<(), Error> {
        replace(&self.path, format!("{room}\n").as_bytes()).map_err(Into::into)
    }

    /// Make the pointer what `prior` read, removing it when there was none. A pointer that
    /// already matches is left alone.
    ///
    /// # Errors
    /// Fails when the pointer cannot be read, written or removed.
    pub fn restore(&self, prior: &SessionPointer) -> Result<(), Error> {
        if bytes(&self.path)? == prior.0 {
            return Ok(());
        }
        match &prior.0 {
            Some(payload) => replace(&self.path, payload)?,
            None => cw_fs::remove(&self.path).map_err(Failure::from)?,
        }
        Ok(())
    }
}

pub(crate) fn read(sessions_dir: &Path, session: &SessionId) -> Result<Option<String>, Failure> {
    if !cw_fs::inspect_dir(sessions_dir)? {
        return Ok(None);
    }

    let path = sessions_dir.join(session.as_str());
    let Some(payload) = bytes(&path)? else {
        return Ok(None);
    };

    let text =
        std::str::from_utf8(&payload).map_err(|_| Failure::InvalidSessionFile(path.clone()))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(Failure::InvalidSessionFile(path));
    }

    Ok(Some(trimmed.to_owned()))
}

fn bytes(path: &Path) -> Result<Option<Vec<u8>>, Failure> {
    if !cw_fs::inspect_file(path)? {
        return Ok(None);
    }

    Ok(Some(cw_fs::read(
        path,
        cw_fs::Cap::bytes(MAX_SESSION_BYTES),
    )?))
}

fn replace(path: &Path, payload: &[u8]) -> Result<(), Failure> {
    cw_fs::write_atomic(path, payload)?;
    Ok(())
}
