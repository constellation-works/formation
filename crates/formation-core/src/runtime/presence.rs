//! Presence policy shared by room operations.

const PRESENCE_SECONDS: i64 = 90;

/// Whether a seat last seen at `seen` (0 when never) holds its mailbox at `now`.
pub(crate) fn held(seen: i64, now: i64) -> bool {
    seen > now - PRESENCE_SECONDS
}
