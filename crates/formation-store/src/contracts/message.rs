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
}
