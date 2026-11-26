//! Watch handover, probe failure and watch set table migrations.

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

pub(super) const SET: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE watch_set(
        recipient TEXT NOT NULL,
        name TEXT NOT NULL,
        command TEXT NOT NULL,
        cwd TEXT NOT NULL,
        every INTEGER NOT NULL,
        state TEXT NOT NULL,
        added_by TEXT NOT NULL,
        added_at INTEGER NOT NULL,
        last_read INTEGER,
        last_result TEXT,
        PRIMARY KEY(recipient, name),
        CHECK(state IN ('on', 'paused')),
        CHECK(every >= 10)
    ) STRICT;",
);

pub(super) const READER: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE watch_set ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
     CREATE TABLE watch_participant(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        seat TEXT NOT NULL,
        pid INTEGER NOT NULL,
        heartbeat INTEGER NOT NULL,
        expires_at INTEGER NOT NULL
     ) STRICT;
     CREATE TABLE watch_reader(
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        reader_id INTEGER,
        stopped_at INTEGER,
        released_at INTEGER,
        revision INTEGER NOT NULL DEFAULT 0
     ) STRICT;
     INSERT INTO watch_reader (singleton) VALUES (1);",
);

pub(super) const ABSENCE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE watch_handover ADD COLUMN absent_reads INTEGER NOT NULL DEFAULT 0
         CHECK(absent_reads >= 0);",
);
