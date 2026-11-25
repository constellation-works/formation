//! Append-only schema steps. The first two adopt pre-versioned Formation rooms.

mod audit;

mod message;

mod seat;

pub(crate) const SESSION_SCHEMA_VERSION: u32 = 4;

pub(crate) const SENT_SCHEMA_VERSION: u32 = 5;

pub(crate) const DELIVERED_SCHEMA_VERSION: u32 = 6;

pub(super) const MIGRATIONS: &[cw_sqlite::Step] = &[
    seat::CREATE,
    message::CREATE,
    audit::CREATE,
    seat::SESSION_BINDING,
    message::SENT,
    message::DELIVERED,
];
