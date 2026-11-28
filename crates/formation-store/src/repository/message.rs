//! Message queries and row mapping.

use crate::contract::{DeliveryState, LogMessage, Message, SentMessage, rfc3339};
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, Row, params};

const INBOX_PAGE_SIZE: i64 = 32;

/// Constraints on a room history query.
pub struct LogQuery<'a> {
    /// Limit to mail involving this caller.
    pub caller: Option<&'a str>,
    /// Limit to mail involving this seat.
    pub seat: Option<&'a str>,
    /// Limit to mail involving this other seat.
    pub with: Option<&'a str>,
    /// Earliest storage time in Unix seconds.
    pub since: Option<i64>,
    /// Exclude watch mail.
    pub no_watch: bool,
    /// Maximum number of results.
    pub limit: i64,
    /// Select one receipt.
    pub receipt: Option<i64>,
}

/// Envelopes visible in a committed read.
pub struct Messages<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) has_sent_column: bool,
    pub(crate) has_delivered_column: bool,
    pub(crate) has_watch_mail: bool,
    pub(crate) has_terminal_mail: bool,
}

impl Messages<'_> {
    /// Oldest pending messages for a recipient.
    pub fn unread(&self, seat: &str) -> Result<Vec<Message>, Error> {
        let (sent, delivered) = self.timestamp_columns();
        let watch = if self.has_watch_mail {
            "watch_name, watch_added_by"
        } else {
            "NULL, NULL"
        };
        let terminal = if self.has_terminal_mail {
            "terminal"
        } else {
            "0"
        };
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT receipt, id, sender, body, {sent}, {delivered}, {watch}, {terminal}
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

    /// Read matching room history, constrained before applying the limit.
    pub fn log(&self, query: &LogQuery<'_>) -> Result<Vec<LogMessage>, Error> {
        let (sent, delivered) = self.timestamp_columns();
        let watch = if self.has_watch_mail {
            "watch_name"
        } else {
            "NULL"
        };
        let added = if self.has_watch_mail {
            "watch_added_by"
        } else {
            "NULL"
        };
        let terminal = if self.has_terminal_mail {
            "terminal"
        } else {
            "0"
        };
        let sql = format!(
            "SELECT receipt, id, sender, recipient, body, {sent}, ack,
            {delivered}, {watch}, {added}, {terminal} FROM messages
            WHERE (?1 IS NULL OR sender = ?1 OR recipient = ?1)
              AND (?2 IS NULL OR sender = ?2 OR recipient = ?2)
              AND (?3 IS NULL OR sender = ?3 OR recipient = ?3)
              AND (?4 IS NULL OR {sent} >= ?4)
              AND (?5 = 0 OR {watch} IS NULL)
              AND (?6 IS NULL OR receipt = ?6)
            ORDER BY receipt DESC LIMIT ?7"
        );
        let mut statement = self.conn.prepare(&sql).map_err(error::sql)?;
        statement
            .query_map(
                params![
                    query.caller,
                    query.seat,
                    query.with,
                    query.since,
                    query.no_watch,
                    query.receipt,
                    query.limit
                ],
                |row| {
                    let ack: bool = row.get(6)?;
                    let delivered: Option<i64> = row.get(7)?;
                    Ok(LogMessage {
                        receipt: row.get(0)?,
                        id: row.get(1)?,
                        sender: row.get(2)?,
                        recipient: row.get(3)?,
                        body: row.get(4)?,
                        sent: row.get::<_, Option<i64>>(5)?.and_then(rfc3339),
                        state: if ack {
                            DeliveryState::Acked
                        } else if delivered.is_some() {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::Stored
                        },
                        watch_name: row.get(8)?,
                        watch_added_by: row.get(9)?,
                        terminal: row.get(10)?,
                        delivered: delivered.and_then(rfc3339),
                    })
                },
            )
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
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
                "SELECT receipt, id, sender, body, sent, delivered, watch_name, watch_added_by,
                        terminal
                   FROM messages
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

    /// Attach trusted watch provenance to a newly inserted envelope.
    pub fn mark_watch(&self, receipt: i64, name: &str, added_by: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE messages SET watch_name = ?2, watch_added_by = ?3 WHERE receipt = ?1",
                params![receipt, name, added_by],
            )
            .map(|_| ())
            .map_err(error::sql)
    }

    /// Record that the admiral sent a newly inserted envelope from a terminal.
    pub fn mark_terminal(&self, receipt: i64) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE messages SET terminal = 1 WHERE receipt = ?1",
                [receipt],
            )
            .map(|_| ())
            .map_err(error::sql)
    }

    /// Associate a newly inserted envelope with the request that produced it.
    pub fn mark_request(&self, receipt: i64, request_id: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE messages SET request_id = ?2 WHERE receipt = ?1",
                params![receipt, request_id],
            )
            .map(|_| ())
            .map_err(error::sql)
    }

    /// Acknowledge any unread envelopes associated with `request_id` in `seat`'s mailbox.
    pub fn ack_request(&self, seat: &str, request_id: &str) -> Result<usize, Error> {
        self.conn
            .execute(
                "UPDATE messages SET ack = 1
                  WHERE recipient = ?1 AND request_id = ?2 AND ack = 0",
                params![seat, request_id],
            )
            .map_err(error::sql)
    }

    /// Next monotonic receipt number within this write transaction.
    pub fn next_receipt(&self) -> Result<i64, Error> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(receipt), 0) + 1 FROM messages",
                [],
                |row| row.get(0),
            )
            .map_err(error::sql)
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

fn envelope(row: &Row<'_>) -> cw_sqlite::rusqlite::Result<Message> {
    Ok(Message {
        receipt: row.get(0)?,
        id: row.get(1)?,
        sender: row.get(2)?,
        body: row.get(3)?,
        watch_name: row.get(6)?,
        watch_added_by: row.get(7)?,
        terminal: row.get(8)?,
        sent: row.get::<_, Option<i64>>(4)?.and_then(rfc3339),
        delivered: row.get::<_, Option<i64>>(5)?.and_then(rfc3339),
    })
}
