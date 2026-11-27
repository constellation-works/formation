//! Request table migration.

pub(super) const CREATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE room_request (
        id TEXT PRIMARY KEY,
        asker TEXT NOT NULL,
        class TEXT NOT NULL,
        workspace TEXT NOT NULL,
        record TEXT NOT NULL,
        ask TEXT NOT NULL,
        options TEXT NOT NULL,
        recommendation TEXT NOT NULL,
        blocking INTEGER NOT NULL CHECK(blocking IN (0, 1)),
        opened_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        holder TEXT,
        settlement TEXT,
        advice TEXT NOT NULL,
        CHECK(holder IS NULL OR settlement IS NULL)
    ) STRICT;",
);
