//! A durable inbox envelope.

use serde::Serialize;

/// An unread envelope. Sequence numbers are scoped to one room.
#[derive(Serialize)]
pub struct Message {
    /// Receipt used to acknowledge this envelope.
    pub receipt: i64,
    /// Sender's unique request identifier.
    pub id: String,
    /// Sending seat recorded on the envelope.
    pub sender: String,
    /// Message contents.
    pub body: String,
    /// Trusted watch producer name, if this is probe mail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_name: Option<String>,
    /// Seat that added the producing watch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_added_by: Option<String>,
    /// When the envelope was stored, as RFC 3339 UTC. Null for envelopes stored by earlier builds.
    pub sent: Option<String>,
    /// When a hook delivered the envelope, as RFC 3339 UTC, if delivered.
    pub delivered: Option<String>,
}

/// Furthest point an envelope has reached.
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryState {
    /// Stored and awaiting delivery.
    Stored,
    /// Handed over by a hook.
    Delivered,
    /// Acknowledged by the recipient.
    Acked,
}

impl DeliveryState {
    /// Stable token used in sent history.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::Delivered => "delivered",
            Self::Acked => "acked",
        }
    }
}

/// A sender's view of a stored envelope.
#[derive(Serialize)]
pub struct SentMessage {
    /// Durable receipt.
    pub receipt: i64,
    /// Sender's unique request identifier.
    pub id: String,
    /// Addressed seat.
    pub recipient: String,
    /// When the envelope was stored, as RFC 3339 UTC, if known.
    pub sent: Option<String>,
    /// Furthest point this envelope has reached.
    pub state: DeliveryState,
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
