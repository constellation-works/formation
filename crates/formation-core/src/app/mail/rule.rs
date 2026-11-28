//! Pure mail validation.

use super::{ADMIRAL, Error, refused};

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

/// No session joins the admiral's seat: it is in every room already.
pub(super) fn unreserved(name: &str) -> Result<(), Error> {
    if name == ADMIRAL {
        return Err(refused(
            "reserved_seat",
            "`admiral` is the operator's reserved seat; join under another name",
        ));
    }
    Ok(())
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
