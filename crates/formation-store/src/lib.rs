//! Formation record contracts and transactional storage.

pub mod contract;

/// Compatibility path for existing callers.
pub use contract as contracts;

mod driver;

mod error;

mod repository;

mod store;

pub use contract::roster::{DecodeError, Host, Role, Roster, Route, Seat, decode};
pub use repository::{PriorDelivery, Snapshot, Tx};
pub use store::Store;
