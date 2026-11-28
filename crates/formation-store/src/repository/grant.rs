//! Room grant queries and row mapping.

use crate::contract::{
    GrantEffect, GrantLifetime, GrantOrigin, GrantRecord, GrantTargetKind, HouseClass, NewGrant,
    RevokeStatus,
};
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, Row, params};

const SELECT_COLUMNS: &str = "id, effect, target_kind, target, class, workspace, limit_text, \
    lifetime, source, entered_by, entered_at, entered_at_ns, revoked_at, revoked_by, origin";

/// The same columns for a room from before the house grants file, where every grant is the room's.
const SELECT_COLUMNS_ROOM_ONLY: &str = "id, effect, target_kind, target, class, workspace, \
    limit_text, lifetime, source, entered_by, entered_at, entered_at_ns, revoked_at, revoked_by, \
    'room'";

/// Read room grants in a committed snapshot.
pub struct GrantRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) available: bool,
    pub(crate) house: bool,
}

impl GrantRead<'_> {
    /// Active (non-revoked) grants ordered by id ascending.
    pub fn active(&self) -> Result<Vec<GrantRecord>, Error> {
        if !self.available {
            return Ok(Vec::new());
        }
        let columns = if self.house {
            SELECT_COLUMNS
        } else {
            SELECT_COLUMNS_ROOM_ONLY
        };
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {columns} FROM room_grant WHERE revoked_at IS NULL ORDER BY id ASC"
            ))
            .map_err(error::sql)?;
        stmt.query_map([], map_row)
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }

    /// Classes the house grants file added, ordered by name.
    pub fn house_classes(&self) -> Result<Vec<HouseClass>, Error> {
        if !self.house {
            return Ok(Vec::new());
        }
        house_classes(self.conn)
    }
}

/// Mutate room grants inside a write transaction.
pub struct GrantMut<'a> {
    pub(crate) conn: &'a Connection,
}

impl GrantMut<'_> {
    /// Active (non-revoked) grants ordered by id ascending.
    pub fn active(&self) -> Result<Vec<GrantRecord>, Error> {
        GrantRead {
            conn: self.conn,
            available: true,
            house: true,
        }
        .active()
    }

    /// Classes the house grants file added, ordered by name.
    pub fn house_classes(&self) -> Result<Vec<HouseClass>, Error> {
        house_classes(self.conn)
    }

    /// Replace the classes the house grants file added.
    pub fn replace_house_classes(&self, classes: &[HouseClass]) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM room_house_catalog", [])
            .map_err(error::sql)?;
        for class in classes {
            self.conn
                .execute(
                    "INSERT INTO room_house_catalog(name, covers, holder, note)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![class.name, class.covers, class.holder, class.note],
                )
                .map_err(error::sql)?;
        }
        Ok(())
    }

    /// Revoke every active house entry and return how many there were.
    pub fn revoke_house(&self, revoked_by: Option<&str>, revoked_at: i64) -> Result<usize, Error> {
        self.conn
            .execute(
                "UPDATE room_grant SET revoked_at = ?1, revoked_by = ?2
                  WHERE origin = 'house_file' AND revoked_at IS NULL",
                params![revoked_at, revoked_by],
            )
            .map_err(error::sql)
    }

    /// Insert a new grant or withhold and return the stored record.
    pub fn insert(&self, entry: &NewGrant<'_>) -> Result<GrantRecord, Error> {
        self.conn
            .execute(
                "INSERT INTO room_grant(
                    effect, target_kind, target, class, workspace, limit_text,
                    lifetime, source, entered_by, entered_at, entered_at_ns, origin
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    entry.effect.as_str(),
                    entry.target_kind.as_str(),
                    entry.target,
                    entry.class,
                    entry.workspace,
                    entry.limit,
                    entry.lifetime.as_str(),
                    entry.source,
                    entry.entered_by,
                    entry.entered_at,
                    entry.entered_at_ns,
                    entry.origin.as_str(),
                ],
            )
            .map_err(error::sql)?;
        let id = self.conn.last_insert_rowid();
        Ok(GrantRecord {
            id,
            effect: entry.effect,
            target_kind: entry.target_kind,
            target: entry.target.to_owned(),
            class: entry.class.to_owned(),
            workspace: entry.workspace.to_owned(),
            limit: entry.limit.map(str::to_owned),
            lifetime: entry.lifetime,
            origin: entry.origin,
            source: entry.source.to_owned(),
            entered_by: entry.entered_by.map(str::to_owned),
            entered_at: entry.entered_at,
            entered_at_ns: entry.entered_at_ns,
            revoked_at: None,
            revoked_by: None,
        })
    }

    /// Mark an active grant revoked, or distinguish already-revoked from missing.
    pub fn revoke(
        &self,
        id: i64,
        revoked_by: Option<&str>,
        revoked_at: i64,
    ) -> Result<RevokeStatus, Error> {
        let Some(existing) = self
            .conn
            .query_row(
                &format!("SELECT {SELECT_COLUMNS} FROM room_grant WHERE id = ?1"),
                [id],
                map_row,
            )
            .optional()
            .map_err(error::sql)?
        else {
            return Ok(RevokeStatus::NotFound);
        };
        if existing.revoked_at.is_some() {
            return Ok(RevokeStatus::AlreadyRevoked);
        }
        self.conn
            .execute(
                "UPDATE room_grant SET revoked_at = ?2, revoked_by = ?3 WHERE id = ?1",
                params![id, revoked_at, revoked_by],
            )
            .map_err(error::sql)?;
        Ok(RevokeStatus::Revoked(Box::new(GrantRecord {
            revoked_at: Some(revoked_at),
            revoked_by: revoked_by.map(str::to_owned),
            ..existing
        })))
    }
}

fn house_classes(conn: &Connection) -> Result<Vec<HouseClass>, Error> {
    let mut stmt = conn
        .prepare("SELECT name, covers, holder, note FROM room_house_catalog ORDER BY name ASC")
        .map_err(error::sql)?;
    stmt.query_map([], |row| {
        Ok(HouseClass {
            name: row.get(0)?,
            covers: row.get(1)?,
            holder: row.get(2)?,
            note: row.get(3)?,
        })
    })
    .map_err(error::sql)?
    .collect::<Result<_, _>>()
    .map_err(error::sql)
}

fn map_row(row: &Row<'_>) -> cw_sqlite::rusqlite::Result<GrantRecord> {
    let effect_raw: String = row.get(1)?;
    let target_kind_raw: String = row.get(2)?;
    let lifetime_raw: String = row.get(7)?;
    let origin_raw: String = row.get(14)?;
    let effect = GrantEffect::from_wire(&effect_raw).ok_or_else(|| {
        cw_sqlite::rusqlite::Error::FromSqlConversionFailure(
            1,
            cw_sqlite::rusqlite::types::Type::Text,
            format!("invalid grant effect: {effect_raw}").into(),
        )
    })?;
    let target_kind = GrantTargetKind::from_wire(&target_kind_raw).ok_or_else(|| {
        cw_sqlite::rusqlite::Error::FromSqlConversionFailure(
            2,
            cw_sqlite::rusqlite::types::Type::Text,
            format!("invalid grant target_kind: {target_kind_raw}").into(),
        )
    })?;
    let lifetime = GrantLifetime::from_wire(&lifetime_raw).ok_or_else(|| {
        cw_sqlite::rusqlite::Error::FromSqlConversionFailure(
            7,
            cw_sqlite::rusqlite::types::Type::Text,
            format!("invalid grant lifetime: {lifetime_raw}").into(),
        )
    })?;
    let origin = GrantOrigin::from_wire(&origin_raw).ok_or_else(|| {
        cw_sqlite::rusqlite::Error::FromSqlConversionFailure(
            14,
            cw_sqlite::rusqlite::types::Type::Text,
            format!("invalid grant origin: {origin_raw}").into(),
        )
    })?;
    Ok(GrantRecord {
        id: row.get(0)?,
        effect,
        target_kind,
        target: row.get(3)?,
        class: row.get(4)?,
        workspace: row.get(5)?,
        limit: row.get(6)?,
        lifetime,
        origin,
        source: row.get(8)?,
        entered_by: row.get(9)?,
        entered_at: row.get(10)?,
        entered_at_ns: row.get(11)?,
        revoked_at: row.get(12)?,
        revoked_by: row.get(13)?,
    })
}
