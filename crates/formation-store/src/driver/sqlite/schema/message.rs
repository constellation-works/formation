//! Messages table migration.

pub(super) const CREATE: cw_sqlite::Step = cw_sqlite::Step::sql(
    "CREATE TABLE IF NOT EXISTS messages(
                receipt INTEGER PRIMARY KEY,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                id TEXT NOT NULL,
                body TEXT NOT NULL,
                ack INTEGER NOT NULL DEFAULT 0,
                UNIQUE(sender,id)
            );
            CREATE INDEX IF NOT EXISTS unread ON messages(recipient,ack,receipt);",
);

pub(super) const SENT: cw_sqlite::Step =
    cw_sqlite::Step::sql("ALTER TABLE messages ADD COLUMN sent INTEGER;");

pub(super) const DELIVERED: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE messages ADD COLUMN delivered INTEGER;
     CREATE INDEX messages_undelivered ON messages(recipient, receipt)
      WHERE ack = 0 AND delivered IS NULL;
     CREATE INDEX messages_sent ON messages(sender, receipt);",
);

/// Trusted producer metadata and per-seat stopped-reader receipts.
pub(super) const WATCH_MAIL: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE messages ADD COLUMN watch_name TEXT;
     ALTER TABLE messages ADD COLUMN watch_added_by TEXT;
     CREATE TABLE watch_notice (seat TEXT PRIMARY KEY) STRICT;",
);

/// Whether the admiral sent an envelope from a terminal.
pub(super) const TERMINAL: cw_sqlite::Step = cw_sqlite::Step::sql(
    "ALTER TABLE messages ADD COLUMN terminal INTEGER NOT NULL DEFAULT 0
         CHECK(terminal IN (0, 1));",
);

/// Request identifier carried by an envelope that routes a request to the admiral.
pub(super) const REQUEST: cw_sqlite::Step =
    cw_sqlite::Step::sql("ALTER TABLE messages ADD COLUMN request_id TEXT;");
