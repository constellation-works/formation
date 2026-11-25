//! Append-only schema steps. The first two adopt pre-versioned Formation rooms.
mod audit;
mod messages;
mod seats;
pub(super) const MIGRATIONS: &[cw_sqlite::Step] = &[seats::CREATE, messages::CREATE, audit::CREATE];
