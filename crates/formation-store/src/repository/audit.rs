//! Append-only command outcomes.
use crate::{
    contracts::AuditEvent,
    error::{Failure, sql},
};
use cw_sqlite::rusqlite::Connection;
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
            .map_err(sql)?;
        Ok(())
    }
}
