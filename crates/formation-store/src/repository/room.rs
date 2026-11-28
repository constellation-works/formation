//! Read and record the close of a room.

use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};

pub(super) fn closure(conn: &Connection) -> Result<Option<(i64, String)>, Error> {
    conn.query_row(
        "SELECT closed_at, closed_by FROM room_close WHERE singleton = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(error::sql)
}

pub(super) fn close(conn: &Connection, at: i64, seat: &str) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO room_close VALUES (1, ?1, ?2)",
        params![at, seat],
    )
    .map_err(error::sql)?;
    Ok(())
}
