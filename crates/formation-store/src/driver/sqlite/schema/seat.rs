//! Seats table migration.

pub(super) const CREATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE IF NOT EXISTS seats(
                name TEXT PRIMARY KEY,
                identity BLOB UNIQUE NOT NULL,
                seen INTEGER NOT NULL
            );",
);
