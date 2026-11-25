//! Private credential bytes are stored using cw-fs's no-clobber creation.
use crate::error::Failure;
use sha2::{Digest, Sha256};
use std::{io::ErrorKind, path::Path};
pub(crate) fn digest(path: &Path, create: bool) -> Result<Vec<u8>, Failure> {
    if create && !cw_fs::inspect_file(path)? {
        let mut key = [0; 32];
        getrandom::fill(&mut key).map_err(Failure::Entropy)?;
        match cw_fs::create_new(path, &key) {
            Ok(()) => (),
            Err(cw_fs::Error::Io { source, .. }) if source.kind() == ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        }
    }
    let key = cw_fs::read(path, cw_fs::Cap::bytes(32))?;
    if key.len() != 32 {
        return Err(Failure::InvalidKey);
    }
    Ok(Sha256::digest(key).to_vec())
}
