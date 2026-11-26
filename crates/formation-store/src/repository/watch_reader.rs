//! Reader registrations and the room's single watch lease.

use crate::contract::{WatchLeaseStatus, WatchParticipant, WatchRole};
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, Row, params};

/// Lease operations within a room write transaction.
pub struct WatchLease<'a> {
    pub(crate) conn: &'a Connection,
}

impl WatchLease<'_> {
    /// Register a new invocation, reopen a stopped room, and claim a vacant lease.
    pub fn register(
        &self,
        seat: &str,
        pid: u32,
        now: i64,
        expires_at: i64,
    ) -> Result<(i64, WatchRole), Error> {
        self.conn
            .execute("UPDATE watch_reader SET stopped_at = NULL", [])
            .map_err(error::sql)?;
        let id = self
            .conn
            .query_row(
                "INSERT INTO watch_participant (seat, pid, heartbeat, expires_at)
                 VALUES (?1, ?2, ?3, ?4) RETURNING id",
                params![seat, pid, now, expires_at],
                |row| row.get(0),
            )
            .map_err(error::sql)?;
        Ok((id, self.claim(id, now)?))
    }

    /// Observe or claim the lease, optionally renewing the caller's live registration.
    ///
    /// An expired or stopped invocation cannot rejoin. A fresh run gets a new identity.
    pub fn pulse(
        &self,
        id: i64,
        now: i64,
        renew_until: Option<i64>,
    ) -> Result<Option<WatchRole>, Error> {
        if let Some(expires_at) = renew_until {
            self.conn
                .execute(
                    "UPDATE watch_participant SET heartbeat = ?2, expires_at = ?3
                       WHERE id = ?1 AND expires_at > ?2",
                    params![id, now, expires_at],
                )
                .map_err(error::sql)?;
        }
        let reader = self
            .conn
            .query_row(
                "SELECT reader_id FROM watch_reader
                  WHERE EXISTS (
                    SELECT 1 FROM watch_participant WHERE id = ?1 AND expires_at > ?2
                  )",
                params![id, now],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(error::sql)?;
        let Some(reader) = reader else {
            return Ok(None);
        };
        if reader == Some(id) {
            return Ok(Some(WatchRole::Reader));
        }
        self.claim(id, now).map(Some)
    }

    /// Check the lease in the same transaction that commits a probe result.
    pub fn holds_lease(&self, id: i64, now: i64) -> Result<bool, Error> {
        holds_lease(self.conn, id, now)
    }

    /// Release only this invocation's registration and any lease it still holds.
    pub fn leave(&self, id: i64, now: i64) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE watch_reader SET reader_id = NULL, released_at = (
                    SELECT min(expires_at, ?2) FROM watch_participant WHERE id = ?1
                 ) WHERE reader_id = ?1",
                params![id, now],
            )
            .map_err(error::sql)?;
        self.conn
            .execute("DELETE FROM watch_participant WHERE id = ?1", [id])
            .map_err(error::sql)?;
        Ok(())
    }

    /// Claim one notice for an eligible seat while the reader is absent.
    pub fn claim_stopped_notice(&self, seat: &str, now: i64) -> Result<Option<(i64, i64)>, Error> {
        let state: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT count(*), coalesce((SELECT p.expires_at FROM watch_participant p
                 WHERE p.id = r.reader_id), r.released_at, min(w.added_at))
               FROM watch_set w CROSS JOIN watch_reader r
              WHERE w.state = 'on' AND r.stopped_at IS NULL
                AND NOT EXISTS (SELECT 1 FROM watch_participant p
                  WHERE p.id = r.reader_id AND p.expires_at > ?2)
                AND EXISTS (SELECT 1 FROM watch_set eligible WHERE eligible.state = 'on'
                  AND (eligible.recipient = ?1 OR eligible.added_by = ?1))
              HAVING count(*) > 0",
                params![seat, now],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(error::sql)?;
        if let Some(state) = state {
            let inserted = self
                .conn
                .execute(
                    "INSERT OR IGNORE INTO watch_notice (seat) VALUES (?1)",
                    [seat],
                )
                .map_err(error::sql)?;
            if inserted == 1 {
                return Ok(Some(state));
            }
        }
        Ok(None)
    }

    /// Mark the room stopped and invalidate every existing invocation.
    pub fn stop(&self, now: i64) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE watch_reader SET reader_id = NULL, stopped_at = coalesce(stopped_at, ?1)",
                [now],
            )
            .map_err(error::sql)?;
        self.conn
            .execute("DELETE FROM watch_participant", [])
            .map_err(error::sql)?;
        Ok(())
    }

    fn claim(&self, id: i64, now: i64) -> Result<WatchRole, Error> {
        let claimed = self
            .conn
            .execute(
                "UPDATE watch_reader SET reader_id = ?1, released_at = NULL
                  WHERE NOT EXISTS (
                    SELECT 1 FROM watch_participant
                     WHERE id = watch_reader.reader_id AND expires_at > ?2
                  )",
                params![id, now],
            )
            .map_err(error::sql)?;
        self.conn
            .execute(
                "DELETE FROM watch_participant WHERE expires_at <= ?1
                   AND id IS NOT (SELECT reader_id FROM watch_reader)",
                [now],
            )
            .map_err(error::sql)?;
        if claimed == 1 {
            self.conn
                .execute("DELETE FROM watch_notice", [])
                .map_err(error::sql)?;
            Ok(WatchRole::Reader)
        } else {
            Ok(WatchRole::Standby)
        }
    }
}

pub(super) fn holds_lease(conn: &Connection, id: i64, now: i64) -> Result<bool, Error> {
    conn.query_row(
        "SELECT EXISTS (
            SELECT 1 FROM watch_reader AS r JOIN watch_participant AS p ON r.reader_id = p.id
             WHERE p.id = ?1 AND p.expires_at > ?2
         )",
        params![id, now],
        |row| row.get(0),
    )
    .map_err(error::sql)
}

pub(super) fn has_stopped_notice(conn: &Connection, seat: &str, now: i64) -> Result<bool, Error> {
    conn.query_row(
        "SELECT EXISTS (
            SELECT 1
              FROM watch_set w CROSS JOIN watch_reader r
             WHERE w.state = 'on' AND r.stopped_at IS NULL
               AND (w.recipient = ?1 OR w.added_by = ?1)
               AND NOT EXISTS (SELECT 1 FROM watch_notice WHERE seat = ?1)
               AND NOT EXISTS (
                   SELECT 1 FROM watch_participant p
                    WHERE p.id = r.reader_id AND p.expires_at > ?2
               )
         )",
        params![seat, now],
        |row| row.get(0),
    )
    .map_err(error::sql)
}

pub(super) fn readers(conn: &Connection, now: i64) -> Result<WatchLeaseStatus, Error> {
    let (reader_id, stopped_at, released_at) = conn
        .query_row(
            "SELECT reader_id, stopped_at, released_at FROM watch_reader",
            [],
            |row| Ok((row.get::<_, Option<i64>>(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(error::sql)?;
    let mut query = conn
        .prepare(
            "SELECT id, seat, pid, heartbeat, expires_at FROM watch_participant
              WHERE id = ?1 OR expires_at > ?2 ORDER BY id",
        )
        .map_err(error::sql)?;
    let participants = query
        .query_map(params![reader_id, now], participant)
        .map_err(error::sql)?;
    let mut readers = WatchLeaseStatus {
        stopped_at,
        released_at,
        ..WatchLeaseStatus::default()
    };
    for row in participants {
        let entry = row.map_err(error::sql)?;
        if Some(entry.id) == reader_id {
            readers.reader = Some(entry);
        } else {
            readers.standbys.push(entry);
        }
    }
    Ok(readers)
}

fn participant(row: &Row<'_>) -> cw_sqlite::rusqlite::Result<WatchParticipant> {
    Ok(WatchParticipant {
        id: row.get(0)?,
        seat: row.get(1)?,
        pid: row.get(2)?,
        heartbeat: row.get(3)?,
        expires_at: row.get(4)?,
    })
}
