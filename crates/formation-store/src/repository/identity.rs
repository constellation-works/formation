//! Reads the winning private credential when multiple joins create the same identity.

use crate::error::Failure;
use sha2::{Digest, Sha256};
use std::io::ErrorKind;
use std::path::Path;

const KEY_BYTES: usize = 32;

pub(crate) fn digest(path: &Path, create: bool) -> Result<Vec<u8>, Failure> {
    if create {
        create_key_if_missing(path)?;
    }

    let key = cw_fs::read(path, cw_fs::Cap::bytes(KEY_BYTES as u64))?;
    if key.len() != KEY_BYTES {
        return Err(Failure::InvalidKey);
    }

    Ok(Sha256::digest(key).to_vec())
}

fn create_key_if_missing(path: &Path) -> Result<(), Failure> {
    if cw_fs::inspect_file(path)? {
        return Ok(());
    }

    let mut candidate = [0; KEY_BYTES];
    getrandom::fill(&mut candidate).map_err(Failure::Entropy)?;

    match cw_fs::create_new(path, &candidate) {
        Ok(()) => Ok(()),
        Err(cw_fs::Error::Io { source, .. }) if source.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}
