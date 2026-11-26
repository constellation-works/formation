//! Open the Formation database with a bounded write wait.

mod schema;

use std::path::Path;
use std::time::Duration;

pub(crate) fn open(root: &Path) -> cw_sqlite::Db {
    cw_sqlite::Db::new(root.join("inbox.sqlite3"), schema::MIGRATIONS)
        .lock_wait(Duration::from_secs(5))
}

pub(crate) fn has_session_column(version: u32) -> bool {
    version >= schema::SESSION_SCHEMA_VERSION
}

pub(crate) fn has_sent_column(version: u32) -> bool {
    version >= schema::SENT_SCHEMA_VERSION
}

pub(crate) fn has_delivered_column(version: u32) -> bool {
    version >= schema::DELIVERED_SCHEMA_VERSION
}

pub(crate) fn has_watch_set(version: u32) -> bool {
    version >= schema::WATCH_SET_SCHEMA_VERSION
}

pub(crate) fn has_watch_reader(version: u32) -> bool {
    version >= schema::WATCH_READER_SCHEMA_VERSION
}

pub(crate) fn has_hook_state(version: u32) -> bool {
    version >= schema::HOOK_STATE_SCHEMA_VERSION
}
