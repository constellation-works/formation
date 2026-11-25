//! Message queries and row mapping.

use crate::contract::Message;
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};

const INBOX_PAGE_SIZE: i64 = 32;

/// Envelopes visible in a committed read.
pub struct Messages<'a> {
    pub(crate) conn: &'a Connection,
}

impl Messages<'_> {
    /// Oldest pending messages for a recipient.
    pub fn unread(&self, seat: &str) -> Result<Vec<Message>, Error> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT receipt, id, sender, body
                   FROM messages
                  WHERE recipient = ?1 AND ack = 0
                  ORDER BY receipt
                  LIMIT ?2",
            )
            .map_err(error::sql)?;

        statement
            .query_map(params![seat, INBOX_PAGE_SIZE], |row| {
                Ok(Message {
                    receipt: row.get(0)?,
                    id: row.get(1)?,
                    sender: row.get(2)?,
                    body: row.get(3)?,
                })
            })
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }
}

/// Original values needed to distinguish a retry from a conflicting send.
pub struct PriorDelivery {
    /// Stable receipt assigned to the original send.
    pub receipt: i64,
    /// Addressed seat.
    pub recipient: String,
    /// Payload that must match on replay.
    pub body: String,
}

/// Envelopes within a write transaction.
pub struct MessagesMut<'a> {
    pub(crate) conn: &'a Connection,
}

impl MessagesMut<'_> {
    /// Previous receipt, destination and body for a sender's request.
    pub fn previous(&self, sender: &str, id: &str) -> Result<Option<(i64, String, String)>, Error> {
        Ok(self
            .prior_delivery(sender, id)?
            .map(|prior| (prior.receipt, prior.recipient, prior.body)))
    }

    /// Look up the original send inside the current write transaction.
    pub fn prior_delivery(&self, sender: &str, id: &str) -> Result<Option<PriorDelivery>, Error> {
        self.conn
            .query_row(
                "SELECT receipt, recipient, body
                   FROM messages
                  WHERE sender = ? AND id = ?",
                params![sender, id],
                |row| {
                    Ok(PriorDelivery {
                        receipt: row.get(0)?,
                        recipient: row.get(1)?,
                        body: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(error::sql)
    }

    /// Insert an envelope and return its durable receipt.
    pub fn insert(&self, sender: &str, to: &str, id: &str, body: &str) -> Result<i64, Error> {
        self.conn
            .execute(
                "INSERT INTO messages (sender, recipient, id, body)
                 VALUES (?, ?, ?, ?)",
                params![sender, to, id, body],
            )
            .map_err(error::sql)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record acknowledgement by the addressed seat.
    pub fn ack(&self, seat: &str, receipt: i64) -> Result<bool, Error> {
        self.conn
            .execute(
                "UPDATE messages SET ack = 1
                  WHERE recipient = ? AND receipt = ?",
                params![seat, receipt],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }
}
