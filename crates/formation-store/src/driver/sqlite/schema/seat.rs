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

pub(super) const HOOK_STATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE seats ADD COLUMN hook_provider TEXT CHECK(hook_provider IN ('claude', 'codex'));
     ALTER TABLE seats ADD COLUMN hook_busy INTEGER CHECK(hook_busy IN (0, 1));
     ALTER TABLE seats ADD COLUMN wake_requested INTEGER NOT NULL DEFAULT 0
         CHECK(wake_requested IN (0, 1));",
);

pub(super) const HOOK_WAITER: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE hook_waiter(
        seat TEXT PRIMARY KEY,
        token INTEGER NOT NULL CHECK(token >= 1),
        session_id TEXT,
        pid INTEGER,
        heartbeat INTEGER,
        expires_at INTEGER,
        CHECK(
            (session_id IS NULL AND pid IS NULL AND heartbeat IS NULL AND expires_at IS NULL)
            OR (session_id IS NOT NULL AND pid IS NOT NULL
                AND heartbeat IS NOT NULL AND expires_at IS NOT NULL)
        )
    ) STRICT;",
);

/// Applied roster, independent of joined session bindings.
pub(super) const ROSTER: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE IF NOT EXISTS roster_seat(
        name TEXT PRIMARY KEY, role TEXT NOT NULL, workspace TEXT, reports_to TEXT
    ) STRICT;",
);

/// Roster display fields and application stamp.
pub(super) const ROSTER_DETAILS: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE roster_seat ADD COLUMN model TEXT;
     ALTER TABLE roster_seat ADD COLUMN effort TEXT;
     ALTER TABLE roster_seat ADD COLUMN record TEXT;
     CREATE TABLE roster_application (applied_at INTEGER NOT NULL, applied_by TEXT) STRICT;",
);

/// The admiral's seat, present in every room and bound to no session.
///
/// A seat that an earlier build let a session join under the reserved name loses its binding.
pub(super) const ADMIRAL: cw_sqlite::Step = cw_sqlite::Step::sql(
    "INSERT INTO seats (name, session_id, seen) VALUES ('admiral', NULL, 0)
         ON CONFLICT(name) DO UPDATE SET session_id = NULL, hook_provider = NULL,
             hook_busy = NULL, wake_requested = 0, seen = 0;
     UPDATE hook_waiter SET session_id = NULL, pid = NULL, heartbeat = NULL, expires_at = NULL
      WHERE seat = 'admiral';",
);
