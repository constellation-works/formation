//! Formation record contracts and transactional storage.

pub mod contract;

mod driver;

mod error;

mod repository;

mod store;

/// Compatibility path for existing callers.
pub use contract as contracts;
pub use contract::roster::{DecodeError, Host, Role, Roster, Route, Seat, decode};
pub use contract::{InvalidSessionId, SessionId};
pub use repository::{
    GrantMut, GrantRead, HookWaiter, HookWaiterRead, LogQuery, PriorDelivery, RequestMut,
    RequestRead, RosterRead, RosterSeat, RosterSeats, Seats, SeatsRead, SessionPointer,
    SessionRoute, Snapshot, Tx, Watch, WatchLease, WatchRead,
};
pub use store::Store;
