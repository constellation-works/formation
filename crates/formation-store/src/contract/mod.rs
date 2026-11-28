//! Formation record shapes.

mod audit;

mod grant;

mod message;

mod request;

mod watch;

pub mod roster;

pub use audit::{AuditEvent, AuditStatus};
pub use grant::{
    GrantEffect, GrantLifetime, GrantOrigin, GrantRecord, GrantTargetKind, HouseClass, NewGrant,
    RevokeStatus,
};
pub use message::{DeliveryState, LogMessage, Mailbox, Message, SentMessage, rfc3339};
pub use request::{Request, RequestAdvice, RequestOption, RequestSettlement, RequestState};
use std::fmt;
pub use watch::{
    NewWatch, WatchEntry, WatchHandover, WatchLeaseStatus, WatchParticipant, WatchRole, WatchState,
};

/// The reserved seat of the operator, present in every room and joined by no session.
pub const ADMIRAL: &str = "admiral";

/// Provider observed in a host hook for a session binding.
#[derive(Clone, Copy)]
pub enum HookProvider {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
}

/// Invalid caller session identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidSessionId;

impl fmt::Display for InvalidSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid session id (use 1-128 ASCII letters, digits or hyphens)")
    }
}

impl std::error::Error for InvalidSessionId {}

/// Validated caller session identifier safe to use as a filename.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
    /// Validate and construct a session identifier.
    ///
    /// # Errors
    /// Returns [`InvalidSessionId`] when the value is not 1 to 128 ASCII letters, digits or
    /// hyphens.
    pub fn try_new(value: impl Into<String>) -> Result<Self, InvalidSessionId> {
        let value = value.into();
        if session_id(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidSessionId)
        }
    }

    /// Borrow the validated session identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a name is an ASCII seat or workspace identifier.
pub fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Whether a value is 1 to 128 ASCII letters, digits or hyphens.
pub fn session_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
