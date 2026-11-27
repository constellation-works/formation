//! Record repositories over read and write transactions.

mod audit;

mod grant;

mod hook_waiter;

mod message;

mod request;

mod seat;

mod roster;

pub(crate) mod seat_dir;

pub(crate) mod session;

mod watch;

mod watch_reader;

use crate::driver::sqlite;
pub use audit::Audit;
use cw_sqlite::rusqlite::Connection;
pub use grant::{GrantMut, GrantRead};
pub use hook_waiter::{HookWaiter, HookWaiterRead};
pub use message::{Messages, MessagesMut, PriorDelivery};
pub use request::{RequestMut, RequestRead};
pub use roster::{RosterRead, RosterSeat, RosterSeats};
pub use seat::{Seats, SeatsRead};
pub use session::{SessionPointer, SessionRoute};
pub use watch::{Watch, WatchRead};
pub use watch_reader::WatchLease;

/// One committed version of the room.
pub struct Snapshot {
    pub(crate) reader: cw_sqlite::Reader,
}

impl Snapshot {
    /// Applied roster visible at the snapshot.
    pub fn roster(&self) -> RosterRead<'_> {
        RosterRead {
            conn: &self.reader,
            available: self.reader.version() >= 14,
        }
    }

    /// Room grants visible at the snapshot.
    pub fn grants(&self) -> GrantRead<'_> {
        GrantRead {
            conn: &self.reader,
            available: sqlite::has_grant(self.reader.version()),
        }
    }

    /// Room requests visible at the snapshot.
    pub fn requests(&self) -> RequestRead<'_> {
        RequestRead {
            conn: &self.reader,
            available: sqlite::has_request(self.reader.version()),
        }
    }

    /// Seats visible at the snapshot.
    pub fn seats(&self) -> SeatsRead<'_> {
        SeatsRead {
            conn: &self.reader,
            has_session_column: sqlite::has_session_column(self.reader.version()),
            has_hook_state: sqlite::has_hook_state(self.reader.version()),
        }
    }

    /// The watch set visible at the snapshot.
    pub fn watch(&self) -> WatchRead<'_> {
        WatchRead {
            conn: &self.reader,
            has_set: sqlite::has_watch_set(self.reader.version()),
            has_reader: sqlite::has_watch_reader(self.reader.version()),
            has_notice: sqlite::has_watch_mail(self.reader.version()),
        }
    }

    /// Hook waiter leases visible at the snapshot.
    pub fn hook_waiter(&self) -> HookWaiterRead<'_> {
        HookWaiterRead {
            conn: &self.reader,
            has_table: sqlite::has_hook_waiter(self.reader.version()),
        }
    }

    /// Messages visible at the snapshot.
    pub fn messages(&self) -> Messages<'_> {
        Messages {
            conn: &self.reader,
            has_sent_column: sqlite::has_sent_column(self.reader.version()),
            has_delivered_column: sqlite::has_delivered_column(self.reader.version()),
            has_watch_mail: sqlite::has_watch_mail(self.reader.version()),
        }
    }
}

/// One atomic room update.
pub struct Tx<'a> {
    pub(crate) conn: &'a Connection,
}

impl Tx<'_> {
    /// Applied roster records in this transaction.
    pub fn roster(&self) -> RosterSeats<'_> {
        RosterSeats { conn: self.conn }
    }

    /// Room grant records in this transaction.
    pub fn grants(&self) -> GrantMut<'_> {
        GrantMut { conn: self.conn }
    }

    /// Room request records in this transaction.
    pub fn requests(&self) -> RequestMut<'_> {
        RequestMut { conn: self.conn }
    }

    /// Seat records in this transaction.
    pub fn seats(&self) -> Seats<'_> {
        Seats { conn: self.conn }
    }

    /// Per-seat hook waiter lease records in this transaction.
    pub fn hook_waiter(&self) -> HookWaiter<'_> {
        HookWaiter { conn: self.conn }
    }

    /// Message records in this transaction.
    pub fn messages(&self) -> MessagesMut<'_> {
        MessagesMut { conn: self.conn }
    }

    /// Lease and standby records in this transaction.
    pub fn watch_reader(&self) -> WatchLease<'_> {
        WatchLease { conn: self.conn }
    }

    /// Audit records in this transaction.
    pub fn audit(&self) -> Audit<'_> {
        Audit { conn: self.conn }
    }

    /// The watch set, deduplication and failure records in this transaction.
    pub fn watch(&self) -> Watch<'_> {
        Watch { conn: self.conn }
    }
}
