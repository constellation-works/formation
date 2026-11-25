//! Seat bindings and last-seen timestamps.

use crate::contract::SessionId;
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};

/// Seats visible in a committed read.
pub struct SeatsRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) has_session_column: bool,
}

impl SeatsRead<'_> {
    /// Look up the seat currently bound to `session`.
    pub fn by_session(&self, session: &SessionId) -> Result<Option<String>, Error> {
        if !self.has_session_column {
            return Ok(None);
        }

        self.conn
            .query_row(
                "SELECT name FROM seats WHERE session_id = ?1",
                [session.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(error::sql)
    }

    /// Return all seat names recorded in this room, ordered alphabetically.
    pub fn names(&self) -> Result<Vec<String>, Error> {
        let mut statement = self
            .conn
            .prepare("SELECT name FROM seats ORDER BY name")
            .map_err(error::sql)?;
        statement
            .query_map([], |row| row.get(0))
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }
}

/// Seats in one room transaction.
pub struct Seats<'a> {
    pub(crate) conn: &'a Connection,
}

impl Seats<'_> {
    /// Bind or rebind a seat to an optional session, returning any replaced session id.
    pub fn bind(
        &self,
        name: &str,
        session: Option<&SessionId>,
        now: i64,
    ) -> Result<Option<String>, Error> {
        let prior: Option<String> = self
            .conn
            .query_row(
                "SELECT session_id FROM seats WHERE name = ?1",
                [name],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(error::sql)?
            .flatten();

        let session_str = session.map(SessionId::as_str);
        if let Some(session_value) = session_str {
            self.conn
                .execute(
                    "UPDATE seats SET session_id = NULL WHERE session_id = ?1 AND name != ?2",
                    params![session_value, name],
                )
                .map_err(error::sql)?;
        }

        self.conn
            .execute(
                "INSERT INTO seats (name, session_id, seen) VALUES (?1, ?2, ?3)
                 ON CONFLICT(name) DO UPDATE
                   SET session_id = excluded.session_id,
                       seen = excluded.seen",
                params![name, session_str, now],
            )
            .map_err(error::sql)?;

        let replaced = match (prior, session_str) {
            (Some(old), Some(new)) if old == new => None,
            (Some(old), _) => Some(old),
            (None, _) => None,
        };
        Ok(replaced)
    }

    /// Refresh a joined seat by name.
    pub fn touch(&self, name: &str, now: i64) -> Result<bool, Error> {
        self.conn
            .execute(
                "UPDATE seats SET seen = ?2 WHERE name = ?1",
                params![name, now],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }

    /// Refresh the seat bound to `session` and return its name.
    pub fn touch_session(&self, session: &SessionId, now: i64) -> Result<Option<String>, Error> {
        self.conn
            .query_row(
                "UPDATE seats SET seen = ?2
                  WHERE session_id = ?1
                  RETURNING name",
                params![session.as_str(), now],
                |row| row.get(0),
            )
            .optional()
            .map_err(error::sql)
    }

    /// Return seats seen after the given cutoff.
    pub fn since(&self, cutoff: i64) -> Result<Vec<String>, Error> {
        let mut statement = self
            .conn
            .prepare("SELECT name FROM seats WHERE seen > ? ORDER BY name")
            .map_err(error::sql)?;
        statement
            .query_map([cutoff], |row| row.get(0))
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }

    /// Whether a named seat was seen after the cutoff.
    pub fn present(&self, name: &str, cutoff: i64) -> Result<bool, Error> {
        self.conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM seats WHERE name = ? AND seen > ?)",
                params![name, cutoff],
                |row| row.get(0),
            )
            .map_err(error::sql)
    }
}
