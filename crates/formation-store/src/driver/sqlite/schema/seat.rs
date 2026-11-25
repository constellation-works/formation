//! Seats table migrations.

pub(super) const CREATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE IF NOT EXISTS seats(
                name TEXT PRIMARY KEY,
                identity BLOB UNIQUE NOT NULL,
                seen INTEGER NOT NULL
            );",
);

pub(super) const SESSION_BINDING: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE seats_next(
                name TEXT PRIMARY KEY,
                session_id TEXT UNIQUE,
                seen INTEGER NOT NULL
            ) STRICT;
            INSERT INTO seats_next(name, seen) SELECT name, seen FROM seats;
            DROP TABLE seats;
            ALTER TABLE seats_next RENAME TO seats;",
);
