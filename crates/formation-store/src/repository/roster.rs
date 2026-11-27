//! Applied roster records, separate from session bindings.

use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, params};

/// One applied seat.
#[derive(Debug, PartialEq, Eq)]
pub struct RosterSeat {
    /// Seat name.
    pub name: String,
    /// Role wire name.
    pub role: String,
    /// Workspace, if any.
    pub workspace: Option<String>,
    /// Parent seat, if any.
    pub reports_to: Option<String>,
}

/// Read applied seats in a snapshot.
pub struct RosterRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) available: bool,
}

impl RosterRead<'_> {
    /// List seats in name order. Older rooms have no roster.
    pub fn seats(&self) -> Result<Vec<RosterSeat>, Error> {
        if !self.available {
            return Ok(Vec::new());
        }
        let mut stmt = self
            .conn
            .prepare("SELECT name, role, workspace, reports_to FROM roster_seat ORDER BY name")
            .map_err(error::sql)?;
        stmt.query_map([], |row| {
            Ok(RosterSeat {
                name: row.get(0)?,
                role: row.get(1)?,
                workspace: row.get(2)?,
                reports_to: row.get(3)?,
            })
        })
        .map_err(error::sql)?
        .collect::<Result<_, _>>()
        .map_err(error::sql)
    }
}

/// Replace applied seats inside a room transaction.
pub struct RosterSeats<'a> {
    pub(crate) conn: &'a Connection,
}

impl RosterSeats<'_> {
    /// Replace the entire applied roster atomically.
    pub fn replace(&self, roster: &crate::Roster) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM roster_seat", [])
            .map_err(error::sql)?;
        for seat in &roster.seats {
            self.conn
                .execute(
                    "INSERT INTO roster_seat (name, role, workspace, reports_to)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        seat.name,
                        seat.role.as_str(),
                        seat.workspace,
                        seat.reports_to
                    ],
                )
                .map_err(error::sql)?;
        }
        Ok(())
    }
}
