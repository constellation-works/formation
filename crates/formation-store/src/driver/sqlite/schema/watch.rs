//! Watch handover and probe failure table migration.

pub(super) const CREATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE IF NOT EXISTS watch_handover(
        seat TEXT NOT NULL,
        probe TEXT NOT NULL,
        fingerprint TEXT NOT NULL,
        handed_at INTEGER NOT NULL,
        PRIMARY KEY(seat, probe, fingerprint)
    );
    CREATE TABLE IF NOT EXISTS watch_failure(
        seat TEXT NOT NULL,
        probe TEXT NOT NULL,
        error TEXT NOT NULL,
        failed_at INTEGER NOT NULL,
        PRIMARY KEY(seat, probe)
    );",
);
