//! Message queries and row mapping.
use crate::{contracts::Message, error::sql};
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};
/// Envelopes visible in a committed read.
pub struct Messages<'a> {
    pub(crate) conn: &'a Connection,
}
impl Messages<'_> {
    /// Oldest pending messages for a recipient.
    pub fn unread(&self, seat: &str) -> Result<Vec<Message>, Error> {
        let mut q=self.conn.prepare("SELECT receipt,id,sender,body FROM messages WHERE recipient=? AND ack=0 ORDER BY receipt LIMIT 32").map_err(sql)?;
        q.query_map([seat], |r| {
            Ok(Message {
                receipt: r.get(0)?,
                id: r.get(1)?,
                sender: r.get(2)?,
                body: r.get(3)?,
            })
        })
        .map_err(sql)?
        .collect::<Result<_, _>>()
        .map_err(sql)
    }
}
/// Envelopes within a write transaction.
pub struct MessagesMut<'a> {
    pub(crate) conn: &'a Connection,
}
impl MessagesMut<'_> {
    /// Previous receipt, destination and body for a sender's request.
    pub fn previous(&self, sender: &str, id: &str) -> Result<Option<(i64, String, String)>, Error> {
        self.conn
            .query_row(
                "SELECT receipt,recipient,body FROM messages WHERE sender=? AND id=?",
                params![sender, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(sql)
    }
    /// Insert an envelope and return its durable receipt.
    pub fn insert(&self, sender: &str, to: &str, id: &str, body: &str) -> Result<i64, Error> {
        self.conn
            .execute(
                "INSERT INTO messages(sender,recipient,id,body) VALUES(?,?,?,?)",
                params![sender, to, id, body],
            )
            .map_err(sql)?;
        Ok(self.conn.last_insert_rowid())
    }
    /// Record acknowledgement by the addressed seat.
    pub fn ack(&self, seat: &str, receipt: i64) -> Result<bool, Error> {
        self.conn
            .execute(
                "UPDATE messages SET ack=1 WHERE recipient=? AND receipt=?",
                params![seat, receipt],
            )
            .map(|n| n == 1)
            .map_err(sql)
    }
}
