//! Open the Formation database with a bounded write wait.
mod schema;
use std::{path::Path, time::Duration};
pub(crate) fn open(root: &Path) -> cw_sqlite::Db {
    cw_sqlite::Db::new(root.join("inbox.sqlite3"), schema::MIGRATIONS)
        .lock_wait(Duration::from_secs(5))
}
