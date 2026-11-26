//! The watch set, handed watch fingerprints and active probe failure records.

use crate::contract::{NewWatch, WatchEntry, WatchLeaseStatus, WatchState};
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::types::Type;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, Row, params};

/// The watch set visible in a committed read.
pub struct WatchRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) has_set: bool,
    pub(crate) has_reader: bool,
}

impl WatchRead<'_> {
    /// Reader state and standbys as of `now`, without upgrading older rooms.
    pub fn readers(&self, now: i64) -> Result<WatchLeaseStatus, Error> {
        if self.has_reader {
            super::watch_reader::readers(self.conn, now)
        } else {
            Ok(WatchLeaseStatus::default())
        }
    }

    /// Whether the invocation still owns a live reader lease.
    pub fn holds_lease(&self, id: i64, now: i64) -> Result<bool, Error> {
        if self.has_reader {
            super::watch_reader::holds_lease(self.conn, id, now)
        } else {
            Ok(false)
        }
    }

    /// Every watch, ordered by name and then recipient.
    ///
    /// A room that predates the watch set has none.
    pub fn list(&self) -> Result<Vec<WatchEntry>, Error> {
        if self.has_set {
            list(self.conn, self.has_reader)
        } else {
            Ok(Vec::new())
        }
    }
}

/// Watch deduplication and failure records in one write transaction.
pub struct Watch<'a> {
    pub(crate) conn: &'a Connection,
}

impl Watch<'_> {
    /// Every watch, ordered by name and then recipient.
    pub fn list(&self) -> Result<Vec<WatchEntry>, Error> {
        list(self.conn, true)
    }

    /// Record a read if the watch still has the definition the reader ran.
    pub fn record_read(&self, watch: &WatchEntry, now: i64, result: &str) -> Result<bool, Error> {
        self.conn
            .execute(
                "UPDATE watch_set SET last_read = ?4, last_result = ?5
                   WHERE recipient = ?1 AND name = ?2 AND revision = ?3 AND state = 'on'",
                params![watch.recipient, watch.name, watch.revision, now, result],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }

    fn next_revision(&self) -> Result<i64, Error> {
        self.conn
            .query_row(
                "UPDATE watch_reader SET revision = revision + 1 RETURNING revision",
                [],
                |row| row.get(0),
            )
            .map_err(error::sql)
    }

    /// Set a watch, replacing one with the same recipient and name in place.
    ///
    /// A replacement is switched on and its last read is forgotten, because that read ran the
    /// old command. Remembered fingerprints are kept. Returns `true` when a watch was replaced.
    pub fn put(&self, watch: &NewWatch<'_>) -> Result<bool, Error> {
        let replaced = self
            .conn
            .query_row(
                "SELECT 1 FROM watch_set WHERE recipient = ?1 AND name = ?2",
                params![watch.recipient, watch.name],
                |_| Ok(()),
            )
            .optional()
            .map_err(error::sql)?
            .is_some();
        let revision = self.next_revision()?;
        self.conn
            .execute(
                "INSERT INTO watch_set
                    (recipient, name, command, cwd, every, state, added_by, added_at, revision)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'on', ?6, ?7, ?8)
                 ON CONFLICT(recipient, name) DO UPDATE SET
                    command = excluded.command,
                    cwd = excluded.cwd,
                    every = excluded.every,
                    state = 'on',
                    added_by = excluded.added_by,
                    added_at = excluded.added_at,
                    last_read = NULL,
                    last_result = NULL,
                    revision = excluded.revision",
                params![
                    watch.recipient,
                    watch.name,
                    watch.command,
                    watch.cwd,
                    watch.every,
                    watch.added_by,
                    watch.added_at,
                    revision,
                ],
            )
            .map_err(error::sql)?;
        Ok(replaced)
    }

    /// Drop a watch with its remembered fingerprints and failure record.
    ///
    /// Returns the number of fingerprints forgotten, or `None` when there is no such watch.
    pub fn remove(&self, recipient: &str, name: &str) -> Result<Option<usize>, Error> {
        let removed = self
            .conn
            .execute(
                "DELETE FROM watch_set WHERE recipient = ?1 AND name = ?2",
                params![recipient, name],
            )
            .map_err(error::sql)?;
        if removed == 0 {
            return Ok(None);
        }
        let forgotten = self
            .conn
            .execute(
                "DELETE FROM watch_handover WHERE seat = ?1 AND probe = ?2",
                params![recipient, name],
            )
            .map_err(error::sql)?;
        self.clear_failure(recipient, name)?;
        Ok(Some(forgotten))
    }

    /// Switch a watch on or off.
    ///
    /// Returns `None` when there is no such watch, else whether the state changed.
    pub fn set_state(
        &self,
        recipient: &str,
        name: &str,
        state: WatchState,
    ) -> Result<Option<bool>, Error> {
        let revision = self.next_revision()?;
        let changed = self
            .conn
            .execute(
                "UPDATE watch_set SET state = ?3, revision = ?4
                  WHERE recipient = ?1 AND name = ?2 AND state != ?3",
                params![recipient, name, state.as_str(), revision],
            )
            .map_err(error::sql)?;
        if changed == 1 {
            return Ok(Some(true));
        }
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM watch_set WHERE recipient = ?1 AND name = ?2",
                params![recipient, name],
                |_| Ok(()),
            )
            .optional()
            .map_err(error::sql)?
            .is_some();
        Ok(exists.then_some(false))
    }

    /// Whether `fingerprint` has already been handed over for `seat` and `probe`.
    pub fn is_handed(&self, seat: &str, probe: &str, fingerprint: &str) -> Result<bool, Error> {
        self.conn
            .query_row(
                "SELECT 1 FROM watch_handover
                  WHERE seat = ?1 AND probe = ?2 AND fingerprint = ?3",
                params![seat, probe, fingerprint],
                |_| Ok(()),
            )
            .optional()
            .map(|row| row.is_some())
            .map_err(error::sql)
    }

    /// Record `fingerprint` as handed over for `seat` and `probe`.
    ///
    /// Returns `true` when this call inserted the row and `false` when it was already present.
    pub fn mark_handed(
        &self,
        seat: &str,
        probe: &str,
        fingerprint: &str,
        handed_at: i64,
    ) -> Result<bool, Error> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO watch_handover (seat, probe, fingerprint, handed_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![seat, probe, fingerprint, handed_at],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }

    /// Record an active failure for `seat` and `probe` if none is currently recorded.
    ///
    /// Returns `true` when this failure transitions the probe into failed state and should be
    /// reported, or `false` when a failure was already active.
    pub fn record_failure(
        &self,
        seat: &str,
        probe: &str,
        message: &str,
        failed_at: i64,
    ) -> Result<bool, Error> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO watch_failure (seat, probe, error, failed_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![seat, probe, message, failed_at],
            )
            .map(|affected| affected == 1)
            .map_err(error::sql)
    }

    /// Clear any active failure for `seat` and `probe` after a successful read.
    pub fn clear_failure(&self, seat: &str, probe: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "DELETE FROM watch_failure WHERE seat = ?1 AND probe = ?2",
                params![seat, probe],
            )
            .map(|_| ())
            .map_err(error::sql)
    }
}

fn list(conn: &Connection, has_reader: bool) -> Result<Vec<WatchEntry>, Error> {
    let revision = if has_reader { "revision" } else { "0" };
    let mut statement = conn
        .prepare(&format!(
            "SELECT recipient, name, command, cwd, every, state, added_by, added_at,
                    last_read, last_result,
                    (SELECT count(*) FROM watch_handover
                      WHERE seat = watch_set.recipient AND probe = watch_set.name), {revision}
               FROM watch_set ORDER BY name, recipient"
        ))
        .map_err(error::sql)?;
    statement
        .query_map([], entry)
        .map_err(error::sql)?
        .collect::<Result<_, _>>()
        .map_err(error::sql)
}

fn entry(row: &Row<'_>) -> cw_sqlite::rusqlite::Result<WatchEntry> {
    let token: String = row.get(5)?;
    let state = WatchState::parse(&token).ok_or_else(|| {
        cw_sqlite::rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            format!("unknown watch state `{token}`").into(),
        )
    })?;
    Ok(WatchEntry {
        recipient: row.get(0)?,
        name: row.get(1)?,
        command: row.get(2)?,
        cwd: row.get(3)?,
        every: row.get(4)?,
        state,
        added_by: row.get(6)?,
        added_at: row.get(7)?,
        last_read: row.get(8)?,
        last_result: row.get(9)?,
        remembered: row.get(10)?,
        revision: row.get(11)?,
    })
}
