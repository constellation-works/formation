//! Private per-seat folders under `<room directory>/seats/`.

use crate::error::Failure;
use std::path::{Path, PathBuf};

/// Create `<room>/seats/<seat>/` with owner-only modes if missing and return its absolute path.
///
/// An existing folder and its contents are left as they are. The caller has validated `seat`
/// as an identifier, so it is one path component.
pub(crate) fn ensure(room: &Path, seat: &str) -> Result<PathBuf, Failure> {
    let path = std::path::absolute(room.join("seats").join(seat))
        .map_err(|source| Failure::SeatDirPath(room.join("seats").join(seat), source))?;
    // The path is printed as a JSON string, which cannot carry a non-UTF-8 path.
    if path.to_str().is_none() {
        let source = std::io::Error::from(std::io::ErrorKind::InvalidData);
        return Err(Failure::SeatDirPath(path, source));
    }
    create(&path).map_err(|source| Failure::SeatDir(path.clone(), source))?;
    Ok(path)
}

fn create(path: &Path) -> Result<(), cw_fs::Error> {
    cw_fs::create_private_dir(path)?;
    if let Some(parent) = path.parent() {
        cw_fs::inspect_dir(parent)?;
    }
    cw_fs::inspect_dir(path)?;
    Ok(())
}
