//! Pure mail validation and presence policy.
use super::{Error, refused};
pub(super) const PRESENCE_SECONDS: i64 = 90;
pub(super) fn name(name: &str) -> Result<(), Error> {
    if name.len() <= 96 && formation_store::contracts::identifier(name) {
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
