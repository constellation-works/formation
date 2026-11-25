//! Transactional storage for one private, same-user Formation room.
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use std::{path::Path, time::Duration};
use thiserror::Error;

/// Store or protocol failure.
#[derive(Debug, Error)]
pub enum Error {
    /// SQLite operation failed.
    #[error("mail store: {0}")]
    Sql(#[from] rusqlite::Error),
    /// Mail operation rejected.
    #[error("{0}")]
    Rejected(&'static str),
}

/// An unread envelope. Sequence numbers are scoped to one room.
#[derive(Serialize)]
pub struct Message {
    /// Receipt used to acknowledge this envelope.
    pub receipt: i64,
    /// Sender's unique request identifier.
    pub id: String,
    /// Authenticated sending seat.
    pub sender: String,
    /// Message contents.
    pub body: String,
}

/// One connection to a room.
pub struct Room(Connection);
impl Room {
    /// Open or initialize the local SQLite database.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS seats(
                name TEXT PRIMARY KEY,
                identity BLOB UNIQUE NOT NULL,
                seen INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS messages(
                receipt INTEGER PRIMARY KEY,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                id TEXT NOT NULL,
                body TEXT NOT NULL,
                ack INTEGER NOT NULL DEFAULT 0,
                UNIQUE(sender,id)
            );
            CREATE INDEX IF NOT EXISTS unread ON messages(recipient,ack,receipt);
            COMMIT;",
        )?;
        Ok(Self(conn))
    }

    /// Register an identity or refresh its existing seat.
    pub fn join(&self, name: &str, key: &[u8]) -> Result<(), Error> {
        let changed = self.0.execute(
            "INSERT INTO seats(name, identity, seen) VALUES (?1, ?2, unixepoch())
             ON CONFLICT(name) DO UPDATE SET seen=unixepoch()
             WHERE identity=excluded.identity",
            params![name, key],
        )?;
        if changed == 0 {
            return Err(Error::Rejected("seat already belongs to another identity"));
        }
        Ok(())
    }

    /// Authenticate an identity and renew its 90-second presence.
    pub fn heartbeat(&self, key: &[u8]) -> Result<String, Error> {
        self.0
            .query_row(
                "UPDATE seats SET seen=unixepoch() WHERE identity=? RETURNING name",
                [key],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(Error::Rejected("join this room first"))
    }

    /// List recently active seats.
    pub fn who(&self) -> Result<Vec<String>, Error> {
        let mut q = self
            .0
            .prepare("SELECT name FROM seats WHERE seen > unixepoch()-90 ORDER BY name")?;
        Ok(q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?)
    }

    /// Insert once per sender and request ID. Replays return the same receipt.
    pub fn send(&mut self, sender: &str, to: &str, id: &str, body: &str) -> Result<i64, Error> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(i64, String, String)> = tx
            .query_row(
                "SELECT receipt,recipient,body FROM messages WHERE sender=? AND id=?",
                params![sender, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((receipt, recipient, text)) = prior {
            if recipient == to && text == body {
                return Ok(receipt);
            }
            return Err(Error::Rejected(
                "message ID already used with different content or recipient",
            ));
        }
        let inserted = tx.execute(
            "INSERT INTO messages(sender, recipient, id, body)
             SELECT ?1, name, ?3, ?4 FROM seats WHERE name=?2 AND seen>unixepoch()-90",
            params![sender, to, id, body],
        )?;
        if inserted == 0 {
            return Err(Error::Rejected("recipient is offline or unknown"));
        }
        let receipt = tx.last_insert_rowid();
        tx.commit()?;
        Ok(receipt)
    }

    /// Read up to 32 oldest unacknowledged messages, without consuming them.
    pub fn inbox(&self, seat: &str) -> Result<Vec<Message>, Error> {
        let mut q = self.0.prepare(
            "SELECT receipt, id, sender, body FROM messages
             WHERE recipient=? AND ack=0 ORDER BY receipt LIMIT 32",
        )?;
        Ok(q.query_map([seat], |r| {
            Ok(Message {
                receipt: r.get(0)?,
                id: r.get(1)?,
                sender: r.get(2)?,
                body: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?)
    }

    /// Acknowledge the receiving seat's message, including on retry.
    pub fn ack(&self, seat: &str, receipt: i64) -> Result<(), Error> {
        if self.0.execute(
            "UPDATE messages SET ack=1 WHERE recipient=? AND receipt=?",
            params![seat, receipt],
        )? == 0
        {
            return Err(Error::Rejected("receipt not found in this inbox"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_blocks_new_mail_and_preserves_replays() -> Result<(), Error> {
        let mut room = Room::open(Path::new(":memory:"))?;
        room.join("chief", b"sender")?;
        room.join("captain", b"recipient")?;
        let receipt = room.send("chief", "captain", "first", "hello")?;
        room.0.execute(
            "UPDATE seats SET seen=unixepoch()-90 WHERE name='captain'",
            [],
        )?;
        assert_eq!(room.who()?, ["chief"]);
        assert!(matches!(
            room.send("chief", "captain", "second", "more"),
            Err(Error::Rejected(_))
        ));
        assert_eq!(room.send("chief", "captain", "first", "hello")?, receipt);
        room.heartbeat(b"recipient")?;
        assert_eq!(room.inbox("captain")?[0].receipt, receipt);
        Ok(())
    }
}
