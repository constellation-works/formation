//! Record repositories over read and write transactions.
mod audit;
pub(crate) mod identities;
mod messages;
mod seats;
pub use audit::Audit;
use cw_sqlite::rusqlite::Connection;
pub use messages::{Messages, MessagesMut};
pub use seats::Seats;

/// One committed version of the room.
pub struct Snapshot {
    pub(crate) reader: cw_sqlite::Reader,
}
impl Snapshot {
    /// Messages visible at the snapshot.
    pub fn messages(&self) -> Messages<'_> {
        Messages { conn: &self.reader }
    }
}
/// One atomic room update.
pub struct Tx<'a> {
    pub(crate) conn: &'a Connection,
}
impl Tx<'_> {
    /// Seat records in this transaction.
    pub fn seats(&self) -> Seats<'_> {
        Seats { conn: self.conn }
    }
    /// Message records in this transaction.
    pub fn messages(&self) -> MessagesMut<'_> {
        MessagesMut { conn: self.conn }
    }
    /// Audit records in this transaction.
    pub fn audit(&self) -> Audit<'_> {
        Audit { conn: self.conn }
    }
}
