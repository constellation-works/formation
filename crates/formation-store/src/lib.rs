//! Formation record contracts and transactional storage.

pub mod contract;

/// Compatibility path for existing callers.
pub use contract as contracts;

mod driver;

mod error;

mod repository;

mod store;

pub use contract::roster::{DecodeError, Host, Role, Roster, Route, Seat, decode};
pub use contract::{InvalidSessionId, SessionId};
pub use repository::{
    HookWaiter, HookWaiterRead, PriorDelivery, Seats, SeatsRead, Snapshot, Tx, Watch, WatchLease,
    WatchRead,
};
pub use store::Store;
