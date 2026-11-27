//! Append-only schema steps. The first two adopt pre-versioned Formation rooms.

mod audit;

mod grant;

mod message;

mod request;

mod seat;

mod watch;

pub(crate) const SESSION_SCHEMA_VERSION: u32 = 4;

pub(crate) const SENT_SCHEMA_VERSION: u32 = 5;

pub(crate) const DELIVERED_SCHEMA_VERSION: u32 = 6;

pub(crate) const WATCH_SET_SCHEMA_VERSION: u32 = 8;

pub(crate) const WATCH_READER_SCHEMA_VERSION: u32 = 9;

pub(crate) const HOOK_STATE_SCHEMA_VERSION: u32 = 10;

pub(crate) const WATCH_MAIL_SCHEMA_VERSION: u32 = 11;

pub(crate) const HOOK_WAITER_SCHEMA_VERSION: u32 = 13;

pub(crate) const GRANT_SCHEMA_VERSION: u32 = 15;

pub(crate) const REQUEST_SCHEMA_VERSION: u32 = 16;

pub(super) const MIGRATIONS: &[cw_sqlite::Step] = &[
    seat::CREATE,
    message::CREATE,
    audit::CREATE,
    seat::SESSION_BINDING,
    message::SENT,
    message::DELIVERED,
    watch::CREATE,
    watch::SET,
    watch::READER,
    seat::HOOK_STATE,
    message::WATCH_MAIL,
    watch::ABSENCE,
    seat::HOOK_WAITER,
    seat::ROSTER,
    grant::CREATE,
    request::CREATE,
];
