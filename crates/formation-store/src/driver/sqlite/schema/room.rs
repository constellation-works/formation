//! Durable room lifecycle.

pub(super) const CLOSE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE room_close (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        closed_at INTEGER NOT NULL,
        closed_by TEXT NOT NULL
    ) STRICT;",
);
