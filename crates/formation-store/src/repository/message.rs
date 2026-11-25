//! Message queries and row mapping.

use crate::contract::{DeliveryState, Message, SentMessage, rfc3339};
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, Row, params};

const INBOX_PAGE_SIZE: i64 = 32;

/// Envelopes visible in a committed read.
pub struct Messages<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) has_sent_column: bool,
    pub(crate) has_delivered_column: bool,
}

impl Messages<'_> {
    /// Oldest pending messages for a recipient.
    pub fn unread(&self, seat: &str) -> Result<Vec<Message>, Error> {
        let (sent, delivered) = self.timestamp_columns();
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT receipt, id, sender, body, {sent}, {delivered}
                   FROM messages
                  WHERE recipient = ?1 AND ack = 0
                  ORDER BY receipt
                  LIMIT ?2"
            ))
            .map_err(error::sql)?;

        statement
            .query_map(params![seat, INBOX_PAGE_SIZE], envelope)
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }

    /// Whether a recipient has mail eligible for hook delivery.
    pub fn has_undelivered(&self, seat: &str) -> Result<bool, Error> {
        let (_, delivered) = self.timestamp_columns();
        self.conn
            .query_row(
                &format!(
                    "SELECT EXISTS (SELECT 1 FROM messages
                      WHERE recipient = ? AND ack = 0 AND {delivered} IS NULL)"
                ),
                [seat],
                |row| row.get(0),
            )
            .map_err(error::sql)
    }

    /// Newest sends by this seat, with the total before limiting.
    pub fn sent(&self, seat: &str, limit: i64) -> Result<(i64, Vec<SentMessage>), Error> {
        let (sent, delivered) = self.timestamp_columns();
        let total = self
            .conn
            .query_row(
                "SELECT count(*) FROM messages WHERE sender = ?",
                [seat],
                |row| row.get(0),
            )
            .map_err(error::sql)?;
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT receipt, id, recipient, {sent}, ack, {delivered} IS NOT NULL
                   FROM messages WHERE sender = ? ORDER BY receipt DESC LIMIT ?"
            ))
            .map_err(error::sql)?;
        let messages = statement
            .query_map(params![seat, limit], |row| {
                let state = match (row.get(4)?, row.get(5)?) {
                    (true, _) => DeliveryState::Acked,
                    (false, true) => DeliveryState::Delivered,
                    (false, false) => DeliveryState::Stored,
                };
                Ok(SentMessage {
                    receipt: row.get(0)?,
                    id: row.get(1)?,
                    recipient: row.get(2)?,
                    sent: row.get::<_, Option<i64>>(3)?.and_then(rfc3339),
                    state,
                })
            })
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)?;
        Ok((total, messages))
    }

    fn timestamp_columns(&self) -> (&'static str, &'static str) {
        let sent = if self.has_sent_column { "sent" } else { "NULL" };
        let delivered = if self.has_delivered_column {
            "delivered"
        } else {
            "NULL"
        };
        (sent, delivered)
    }
}

fn envelope(row: &Row<'_>) -> cw_sqlite::rusqlite::Result<Message> {
    Ok(Message {
        receipt: row.get(0)?,
        id: row.get(1)?,
        sender: row.get(2)?,
        body: row.get(3)?,
        sent: row.get::<_, Option<i64>>(4)?.and_then(rfc3339),
        delivered: row.get::<_, Option<i64>>(5)?.and_then(rfc3339),
    })
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
    /// Count mail eligible for hook delivery in this transaction.
    pub fn count_undelivered(&self, seat: &str) -> Result<i64, Error> {
        self.conn
            .query_row(
                "SELECT count(*) FROM messages
                  WHERE recipient = ? AND ack = 0 AND delivered IS NULL",
                [seat],
                |row| row.get(0),
            )
            .map_err(error::sql)
    }

    /// Visit eligible mail in receipt order until the visitor returns false.
    pub fn visit_undelivered(
        &self,
        seat: &str,
        mut visit: impl FnMut(Message) -> bool,
    ) -> Result<(), Error> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT receipt, id, sender, body, sent, delivered FROM messages
                  WHERE recipient = ? AND ack = 0 AND delivered IS NULL ORDER BY receipt",
            )
            .map_err(error::sql)?;
        for message in statement.query_map([seat], envelope).map_err(error::sql)? {
            if !visit(message.map_err(error::sql)?) {
                break;
            }
        }
        Ok(())
    }

    /// Record the hook delivery of a selected receipt in the current transaction.
    pub fn deliver(&self, receipt: i64, now: i64) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE messages SET delivered = ?2 WHERE receipt = ?1",
                params![receipt, now],
            )
            .map(|_| ())
            .map_err(error::sql)
    }

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

    /// Insert an envelope stored at `sent` (Unix seconds) and return its durable receipt.
    pub fn insert(
        &self,
        sender: &str,
        to: &str,
        id: &str,
        body: &str,
        sent: i64,
    ) -> Result<i64, Error> {
        self.conn
            .execute(
                "INSERT INTO messages (sender, recipient, id, body, sent)
                 VALUES (?, ?, ?, ?, ?)",
                params![sender, to, id, body, sent],
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
