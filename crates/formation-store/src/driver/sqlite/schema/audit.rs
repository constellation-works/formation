//! Audit table migration.
pub(super) const CREATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE audit (seq INTEGER PRIMARY KEY, event TEXT NOT NULL) STRICT;",
);
