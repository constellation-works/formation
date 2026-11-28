//! Append-only command outcomes.

use crate::contract::AuditEvent;
use crate::error;
use crate::error::Failure;
use cw_sqlite::rusqlite::{Connection, OptionalExtension};

/// Audit records inside one write.
pub struct Audit<'a> {
    pub(crate) conn: &'a Connection,
}

impl Audit<'_> {
    /// Persist one completed command's outcome without its payload.
    pub fn append(&self, event: &AuditEvent) -> Result<(), cw_base::Error> {
        let encoded = serde_json::to_string(event).map_err(Failure::from)?;
        self.conn
            .execute("INSERT INTO audit(event) VALUES(?)", [encoded])
            .map_err(error::sql)?;
        Ok(())
    }
}

pub(super) fn first_join(conn: &Connection) -> Result<Option<String>, cw_base::Error> {
    conn.query_row(
        "SELECT json_extract(event, '$.at') FROM audit
         WHERE json_extract(event, '$.command') = 'mail join'
           AND json_extract(event, '$.status') = 'success'
         ORDER BY seq LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(error::sql)
}
