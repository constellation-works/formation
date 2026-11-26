//! Seat bindings and last-seen timestamps.

use crate::contract::{HookProvider, Mailbox, SessionId};
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};

/// Seats visible in a committed read.
pub struct SeatsRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) has_session_column: bool,
    pub(crate) has_hook_state: bool,
}

impl SeatsRead<'_> {
    /// Whether a seat with this name has joined.
    pub fn contains(&self, name: &str) -> Result<bool, Error> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM seats WHERE name = ?)",
                [name],
                |row| row.get(0),
            )
            .map_err(error::sql)
    }

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

    /// Current idle Codex binding when no wake has been requested for this idle period.
    pub fn idle_codex(&self, name: &str) -> Result<Option<SessionId>, Error> {
        if !self.has_hook_state {
            return Ok(None);
        }
        self.conn
            .query_row(
                "SELECT session_id FROM seats
                 WHERE name = ?1 AND session_id IS NOT NULL
                   AND hook_provider = 'codex' AND hook_busy = 0 AND wake_requested = 0",
                [name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map(|session| session.and_then(|id| SessionId::try_new(id).ok()))
            .map_err(error::sql)
    }

    /// Whether `name` has joined this room.
    pub fn exists(&self, name: &str) -> Result<bool, Error> {
        self.conn
            .query_row("SELECT 1 FROM seats WHERE name = ?1", [name], |_| Ok(()))
            .optional()
            .map(|row| row.is_some())
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
    /// Look up the seat currently bound to `session` inside a write transaction.
    pub fn by_session(&self, session: &SessionId) -> Result<Option<String>, Error> {
        SeatsRead {
            conn: self.conn,
            has_session_column: true,
            has_hook_state: true,
        }
        .by_session(session)
    }

    /// Record a lifecycle event only for the session that still holds a seat.
    pub fn mark_hook(
        &self,
        session: &SessionId,
        provider: HookProvider,
        busy: bool,
    ) -> Result<bool, Error> {
        let provider = match provider {
            HookProvider::Claude => "claude",
            HookProvider::Codex => "codex",
        };
        self.conn
            .execute(
                "UPDATE seats SET hook_provider = ?2, hook_busy = ?3,
                     wake_requested = CASE WHEN hook_provider = ?2 AND hook_busy = 0 AND ?3 = 0
                                           THEN wake_requested ELSE 0 END
                 WHERE session_id = ?1",
                params![session.as_str(), provider, busy],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }

    /// Reserve one wake if the observed binding is still current and idle.
    pub fn claim_codex_wake(&self, name: &str, session: &SessionId) -> Result<bool, Error> {
        self.conn
            .execute(
                "UPDATE seats SET wake_requested = 1
                 WHERE name = ?1 AND session_id = ?2
                   AND hook_provider = 'codex' AND hook_busy = 0 AND wake_requested = 0",
                params![name, session.as_str()],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }

    /// Bind or rebind a seat to an optional session, returning any replaced session id.
    ///
    /// Joining does not renew presence: a new seat reads as never seen.
    pub fn bind(&self, name: &str, session: Option<&SessionId>) -> Result<Option<String>, Error> {
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
                    "UPDATE seats SET session_id = NULL, hook_provider = NULL,
                                      hook_busy = NULL, wake_requested = 0
                     WHERE session_id = ?1 AND name != ?2",
                    params![session_value, name],
                )
                .map_err(error::sql)?;
            self.conn
                .execute(
                    "UPDATE hook_waiter
                        SET session_id = NULL, pid = NULL, heartbeat = NULL, expires_at = NULL
                      WHERE session_id = ?1 AND seat != ?2",
                    params![session_value, name],
                )
                .map_err(error::sql)?;
        }

        let changed = self
            .conn
            .execute(
                "INSERT INTO seats (name, session_id, seen) VALUES (?1, ?2, 0)
                 ON CONFLICT(name) DO UPDATE SET session_id = excluded.session_id,
                     hook_provider = NULL, hook_busy = NULL, wake_requested = 0
                 WHERE seats.session_id IS NOT excluded.session_id",
                params![name, session_str],
            )
            .map_err(error::sql)?;
        if changed == 1 {
            self.conn
                .execute(
                    "UPDATE hook_waiter
                        SET session_id = NULL, pid = NULL, heartbeat = NULL, expires_at = NULL
                      WHERE seat = ?1 AND session_id IS NOT ?2",
                    params![name, session_str],
                )
                .map_err(error::sql)?;
        }

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

    /// Unix seconds when a seat last renewed its presence (0 for never), or `None` if not joined.
    pub fn seen(&self, name: &str) -> Result<Option<i64>, Error> {
        self.conn
            .query_row("SELECT seen FROM seats WHERE name = ?", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(error::sql)
    }

    /// Up to `limit` joined seat names, ordered alphabetically.
    pub fn names(&self, limit: usize) -> Result<Vec<String>, Error> {
        let mut statement = self
            .conn
            .prepare("SELECT name FROM seats ORDER BY name LIMIT ?")
            .map_err(error::sql)?;
        statement
            .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| row.get(0))
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }

    /// Every joined seat with its unread count, ordered alphabetically.
    pub fn mailboxes(&self) -> Result<Vec<Mailbox>, Error> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT name, seen,
                        (SELECT count(*) FROM messages
                          WHERE recipient = seats.name AND ack = 0)
                   FROM seats ORDER BY name",
            )
            .map_err(error::sql)?;
        statement
            .query_map([], |row| {
                Ok(Mailbox {
                    name: row.get(0)?,
                    seen: row.get(1)?,
                    unread: row.get(2)?,
                })
            })
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }
}
