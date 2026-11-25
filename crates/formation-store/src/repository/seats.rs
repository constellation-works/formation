//! Seat bindings and last-seen timestamps.
use crate::error::sql;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};
/// Seats in one room transaction.
pub struct Seats<'a> {
    pub(crate) conn: &'a Connection,
}
impl Seats<'_> {
    /// Bind or refresh a name when its credential matches.
    pub fn bind(&self, name: &str, key: &[u8], now: i64) -> Result<bool, Error> {
        self.conn
            .execute(
                "INSERT INTO seats(name,identity,seen) VALUES(?1,?2,?3)
            ON CONFLICT(name) DO UPDATE SET seen=excluded.seen WHERE identity=excluded.identity",
                params![name, key, now],
            )
            .map(|n| n == 1)
            .map_err(sql)
    }
    /// Refresh a known credential and return its seat.
    pub fn touch(&self, key: &[u8], now: i64) -> Result<Option<String>, Error> {
        self.conn
            .query_row(
                "UPDATE seats SET seen=?2 WHERE identity=?1 RETURNING name",
                params![key, now],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql)
    }
    /// Return seats seen after the given cutoff.
    pub fn since(&self, cutoff: i64) -> Result<Vec<String>, Error> {
        let mut q = self
            .conn
            .prepare("SELECT name FROM seats WHERE seen>? ORDER BY name")
            .map_err(sql)?;
        q.query_map([cutoff], |r| r.get(0))
            .map_err(sql)?
            .collect::<Result<_, _>>()
            .map_err(sql)
    }
    /// Whether a named seat was seen after the cutoff.
    pub fn present(&self, name: &str, cutoff: i64) -> Result<bool, Error> {
        self.conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM seats WHERE name=? AND seen>?)",
                params![name, cutoff],
                |r| r.get(0),
            )
            .map_err(sql)
    }
}
