//! Authority grant table migration.

/// Day grants and withholds stored in the room.
pub(super) const CREATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE IF NOT EXISTS room_grant(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        effect TEXT NOT NULL CHECK(effect IN ('grant', 'withhold')),
        target_kind TEXT NOT NULL CHECK(target_kind IN ('seat', 'rank')),
        target TEXT NOT NULL,
        class TEXT NOT NULL,
        workspace TEXT NOT NULL,
        limit_text TEXT,
        lifetime TEXT NOT NULL CHECK(lifetime IN ('day', 'standing')),
        source TEXT NOT NULL,
        entered_by TEXT,
        entered_at INTEGER NOT NULL,
        entered_at_ns INTEGER NOT NULL DEFAULT 0
            CHECK(entered_at_ns >= 0 AND entered_at_ns < 1000000000),
        revoked_at INTEGER,
        revoked_by TEXT
    ) STRICT;",
);

/// Origin of each grant and the classes a house grants file adds to the catalog.
pub(super) const HOUSE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE room_grant ADD COLUMN origin TEXT NOT NULL DEFAULT 'room'
         CHECK(origin IN ('room', 'house_file'));
     CREATE TABLE room_house_catalog(
        name TEXT PRIMARY KEY,
        covers TEXT NOT NULL,
        holder TEXT NOT NULL,
        note TEXT
    ) STRICT;",
);
