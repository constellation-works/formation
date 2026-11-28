//! Pure mail validation.

use super::{Error, refused};

pub(super) fn name(name: &str) -> Result<(), Error> {
    if name.len() <= 96 && formation_store::contract::identifier(name) {
        Ok(())
    } else {
        Err(refused(
            "invalid_identifier",
            "invalid identifier (use 1-96 letters, digits, hyphens or underscores)",
        ))
    }
}

pub(super) fn body(text: &str) -> Result<(), Error> {
    if text.trim().is_empty() || text.len() > 8192 {
        return Err(refused(
            "invalid_body",
            "body must contain text and fit within 8192 bytes",
        ));
    }
    Ok(())
}
