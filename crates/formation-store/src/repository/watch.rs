//! Handed watch fingerprints and active probe failure records.

use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, params};

/// Watch deduplication and failure records in one write transaction.
pub struct Watch<'a> {
    pub(crate) conn: &'a Connection,
}

impl Watch<'_> {
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
