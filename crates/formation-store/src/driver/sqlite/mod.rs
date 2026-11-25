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
