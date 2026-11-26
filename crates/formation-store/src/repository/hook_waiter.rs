//! Per-seat hook waiter leases with monotonic fencing tokens.

use crate::contract::SessionId;
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};

/// Hook waiter lease state visible in a committed read.
pub struct HookWaiterRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) has_table: bool,
}

impl HookWaiterRead<'_> {
    /// Whether `(seat, token)` still owns a live waiter lease for `session` at `now`.
    pub fn holds_lease(
        &self,
        seat: &str,
        token: i64,
        session: &SessionId,
        now: i64,
    ) -> Result<bool, Error> {
        if self.has_table {
            holds_lease(self.conn, seat, token, session, now)
        } else {
            Ok(false)
        }
    }
}

/// Per-seat hook waiter lease operations within a room write transaction.
pub struct HookWaiter<'a> {
    pub(crate) conn: &'a Connection,
}

impl HookWaiter<'_> {
    /// Claim the per-seat waiter lease when `session` holds `seat` and no live waiter owns it.
    ///
    /// Returns a strictly monotonic fencing token on acquisition, or `None` when the seat is not
    /// currently bound to `session` or another waiter already holds an unexpired lease for the
    /// same binding.
    pub fn claim(
        &self,
        seat: &str,
        session: &SessionId,
        pid: u32,
        now: i64,
        expires_at: i64,
    ) -> Result<Option<i64>, Error> {
        let bound: bool = self
            .conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM seats WHERE name = ?1 AND session_id = ?2)",
                params![seat, session.as_str()],
                |row| row.get(0),
            )
            .map_err(error::sql)?;
        if !bound {
            return Ok(None);
        }

        self.conn
            .query_row(
                "INSERT INTO hook_waiter (seat, token, session_id, pid, heartbeat, expires_at)
                 VALUES (?1, 1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(seat) DO UPDATE SET
                     token = hook_waiter.token + 1,
                     session_id = excluded.session_id,
                     pid = excluded.pid,
                     heartbeat = excluded.heartbeat,
                     expires_at = excluded.expires_at
                 WHERE hook_waiter.session_id IS NULL
                    OR hook_waiter.expires_at <= excluded.heartbeat
                    OR hook_waiter.session_id != excluded.session_id
                 RETURNING token",
                params![seat, session.as_str(), i64::from(pid), now, expires_at],
                |row| row.get(0),
            )
            .optional()
            .map_err(error::sql)
    }

    /// Renew a live lease held by `(seat, token, session)` while the seat binding still matches.
    ///
    /// An expired, released, superseded, or rebound lease cannot be renewed.
    pub fn renew(
        &self,
        seat: &str,
        token: i64,
        session: &SessionId,
        now: i64,
        expires_at: i64,
    ) -> Result<bool, Error> {
        self.conn
            .execute(
                "UPDATE hook_waiter
                    SET heartbeat = ?4, expires_at = ?5
                  WHERE seat = ?1
                    AND token = ?2
                    AND session_id = ?3
                    AND expires_at > ?4
                    AND EXISTS (
                        SELECT 1 FROM seats WHERE name = ?1 AND session_id = ?3
                    )",
                params![seat, token, session.as_str(), now, expires_at],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }

    /// Check the lease inside a write transaction before committing delivery.
    pub fn holds_lease(
        &self,
        seat: &str,
        token: i64,
        session: &SessionId,
        now: i64,
    ) -> Result<bool, Error> {
        holds_lease(self.conn, seat, token, session, now)
    }

    /// Release only the lease matching `(seat, token)`, retaining `token` so fencing stays
    /// monotonic across future claims.
    pub fn release(&self, seat: &str, token: i64) -> Result<bool, Error> {
        self.conn
            .execute(
                "UPDATE hook_waiter
                    SET session_id = NULL, pid = NULL, heartbeat = NULL, expires_at = NULL
                  WHERE seat = ?1 AND token = ?2 AND session_id IS NOT NULL",
                params![seat, token],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }
}

fn holds_lease(
    conn: &Connection,
    seat: &str,
    token: i64,
    session: &SessionId,
    now: i64,
) -> Result<bool, Error> {
    conn.query_row(
        "SELECT EXISTS (
            SELECT 1
              FROM hook_waiter AS waiter
              JOIN seats ON seats.name = waiter.seat
             WHERE waiter.seat = ?1
               AND waiter.token = ?2
               AND waiter.session_id = ?3
               AND seats.session_id = ?3
               AND waiter.expires_at > ?4
         )",
        params![seat, token, session.as_str(), now],
        |row| row.get(0),
    )
    .map_err(error::sql)
}
