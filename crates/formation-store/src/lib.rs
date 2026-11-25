//! Formation record contracts and transactional storage.
pub mod contracts;
mod driver;
mod error;
mod repository;
mod store;
pub use contracts::roster::{DecodeError, Host, Role, Roster, Route, Seat, decode};
pub use repository::{Snapshot, Tx};
pub use store::Store;
