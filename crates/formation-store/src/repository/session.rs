//! Private session-to-room pointer files under `<home>/sessions/`.

use crate::contract::SessionId;
use crate::error::Failure;
use std::path::Path;

const MAX_SESSION_BYTES: u64 = 4096;

pub(crate) fn read(sessions_dir: &Path, session: &SessionId) -> Result<Option<String>, Failure> {
    if !cw_fs::inspect_dir(sessions_dir)? {
        return Ok(None);
    }

    let path = sessions_dir.join(session.as_str());
    if !cw_fs::inspect_file(&path)? {
        return Ok(None);
    }

    let bytes = cw_fs::read(&path, cw_fs::Cap::bytes(MAX_SESSION_BYTES))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| Failure::InvalidSessionFile(path.clone()))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(Failure::InvalidSessionFile(path));
    }

    Ok(Some(trimmed.to_owned()))
}

pub(crate) fn write(sessions_dir: &Path, session: &SessionId, room: &str) -> Result<(), Failure> {
    if !cw_fs::inspect_dir(sessions_dir)? {
        cw_fs::create_private_dir(sessions_dir)?;
        let _ = cw_fs::inspect_dir(sessions_dir)?;
    }

    let path = sessions_dir.join(session.as_str());
    let payload = format!("{room}\n");
    cw_fs::write_atomic(&path, payload.as_bytes())?;
    Ok(())
}
