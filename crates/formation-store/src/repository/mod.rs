//! Record repositories over read and write transactions.

mod audit;

mod message;

mod seat;

pub(crate) mod session;

use crate::driver::sqlite;
pub use audit::Audit;
use cw_sqlite::rusqlite::Connection;
pub use message::{Messages, MessagesMut, PriorDelivery};
pub use seat::{Seats, SeatsRead};

/// One committed version of the room.
pub struct Snapshot {
    pub(crate) reader: cw_sqlite::Reader,
}

impl Snapshot {
    /// Seats visible at the snapshot.
    pub fn seats(&self) -> SeatsRead<'_> {
        SeatsRead {
            conn: &self.reader,
            has_session_column: sqlite::has_session_column(self.reader.version()),
        }
    }

    /// Messages visible at the snapshot.
    pub fn messages(&self) -> Messages<'_> {
        Messages {
            conn: &self.reader,
            has_sent_column: sqlite::has_sent_column(self.reader.version()),
        }
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
