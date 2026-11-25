//! A durable inbox envelope.

use serde::Serialize;

/// An unread envelope. Sequence numbers are scoped to one room.
#[derive(Serialize)]
pub struct Message {
    /// Receipt used to acknowledge this envelope.
    pub receipt: i64,
    /// Sender's unique request identifier.
    pub id: String,
    /// Authenticated sending seat.
    pub sender: String,
    /// Message contents.
    pub body: String,
    /// When the envelope was stored, as RFC 3339 UTC. Null for envelopes stored by earlier builds.
    pub sent: Option<String>,
}

/// One joined seat's mailbox, as listed for the room.
pub struct Mailbox {
    /// Seat name.
    pub name: String,
    /// Envelopes addressed to the seat and not acknowledged.
    pub unread: i64,
    /// Unix seconds when the seat last renewed its presence, or 0 when it never has.
    pub seen: i64,
}

/// Format Unix seconds as RFC 3339 UTC, or `None` when out of range.
pub fn rfc3339(unix: i64) -> Option<String> {
    time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()?
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
}
