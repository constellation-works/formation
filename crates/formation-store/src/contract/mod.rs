//! Formation record shapes.

mod audit;

mod message;

pub mod roster;

pub use audit::{AuditEvent, AuditStatus};
pub use message::Message;

/// Whether a name is an ASCII seat or workspace identifier.
pub fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
