//! Applied roster records, separate from session bindings.

use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, params};

/// One applied seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RosterSeat {
    /// Seat name.
    pub name: String,
    /// Role wire name.
    pub role: String,
    /// Workspace, if any.
    pub workspace: Option<String>,
    /// Model, if specified.
    pub model: Option<String>,
    /// Effort, if specified.
    pub effort: Option<String>,
    /// Assignment record, if specified.
    pub record: Option<String>,
    /// Parent seat, if any.
    pub reports_to: Option<String>,
}

/// Read applied seats in a snapshot.
pub struct RosterRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) available: bool,
    pub(crate) details: bool,
}

impl RosterRead<'_> {
    /// List seats in name order. Older rooms have no roster.
    pub fn seats(&self) -> Result<Vec<RosterSeat>, Error> {
        if !self.available {
            return Ok(Vec::new());
        }
        let query = if self.details {
            "SELECT name, role, workspace, reports_to, model, effort, record FROM roster_seat ORDER BY name"
        } else {
            "SELECT name, role, workspace, reports_to, NULL, NULL, NULL FROM roster_seat ORDER BY name"
        };
        let mut stmt = self.conn.prepare(query).map_err(error::sql)?;
        stmt.query_map([], |row| {
            Ok(RosterSeat {
                name: row.get(0)?,
                role: row.get(1)?,
                workspace: row.get(2)?,
                model: row.get(4)?,
                effort: row.get(5)?,
                record: row.get(6)?,
                reports_to: row.get(3)?,
            })
        })
        .map_err(error::sql)?
        .collect::<Result<_, _>>()
        .map_err(error::sql)
    }

    /// Application time and seat, if recorded.
    pub fn application(&self) -> Result<Option<(i64, Option<String>)>, Error> {
        use cw_sqlite::rusqlite::OptionalExtension;
        if !self.details {
            return Ok(None);
        }
        self.conn
            .query_row(
                "SELECT applied_at, applied_by FROM roster_application LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(error::sql)
    }
}

/// Replace applied seats inside a room transaction.
pub struct RosterSeats<'a> {
    pub(crate) conn: &'a Connection,
}

impl RosterSeats<'_> {
    /// List seats in name order inside this write transaction.
    pub fn seats(&self) -> Result<Vec<RosterSeat>, Error> {
        RosterRead {
            conn: self.conn,
            available: true,
            details: true,
        }
        .seats()
    }

    /// Record who applied the roster and when.
    pub fn stamp(&self, now: i64, seat: Option<&str>) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM roster_application", [])
            .map_err(error::sql)?;
        self.conn
            .execute(
                "INSERT INTO roster_application VALUES (?1, ?2)",
                params![now, seat],
            )
            .map_err(error::sql)?;
        Ok(())
    }

    /// Replace the entire applied roster atomically.
    pub fn replace(&self, roster: &crate::Roster) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM roster_seat", [])
            .map_err(error::sql)?;
        for seat in &roster.seats {
            self.conn
                .execute(
                    "INSERT INTO roster_seat
                     (name, role, workspace, reports_to, model, effort, record)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        seat.name,
                        seat.role.as_str(),
                        seat.workspace,
                        seat.reports_to,
                        seat.model,
                        seat.effort,
                        seat.record
                    ],
                )
                .map_err(error::sql)?;
        }
        Ok(())
    }
}
