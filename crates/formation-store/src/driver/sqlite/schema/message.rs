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
