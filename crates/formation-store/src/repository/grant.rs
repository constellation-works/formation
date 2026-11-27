//! Room grant queries and row mapping.

use crate::contract::{
    GrantEffect, GrantLifetime, GrantOrigin, GrantRecord, GrantTargetKind, NewGrant, RevokeStatus,
};
use crate::error;
use cw_base::Error;
use cw_sqlite::rusqlite::{Connection, OptionalExtension, Row, params};

const SELECT_COLUMNS: &str = "id, effect, target_kind, target, class, workspace, limit_text, \
    lifetime, source, entered_by, entered_at, entered_at_ns, revoked_at, revoked_by";

/// Read room grants in a committed snapshot.
pub struct GrantRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) available: bool,
}

impl GrantRead<'_> {
    /// Active (non-revoked) grants ordered by id ascending.
    pub fn active(&self) -> Result<Vec<GrantRecord>, Error> {
        if !self.available {
            return Ok(Vec::new());
        }
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {SELECT_COLUMNS} FROM room_grant WHERE revoked_at IS NULL ORDER BY id ASC"
            ))
            .map_err(error::sql)?;
        stmt.query_map([], map_row)
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
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
        }
        .active()
    }

    /// Insert a new grant or withhold and return the stored record.
    pub fn insert(&self, entry: &NewGrant<'_>) -> Result<GrantRecord, Error> {
        self.conn
            .execute(
                "INSERT INTO room_grant(
                    effect, target_kind, target, class, workspace, limit_text,
                    lifetime, source, entered_by, entered_at, entered_at_ns
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
            origin: GrantOrigin::Room,
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

fn map_row(row: &Row<'_>) -> cw_sqlite::rusqlite::Result<GrantRecord> {
    let effect_raw: String = row.get(1)?;
    let target_kind_raw: String = row.get(2)?;
    let lifetime_raw: String = row.get(7)?;
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
    Ok(GrantRecord {
        id: row.get(0)?,
        effect,
        target_kind,
        target: row.get(3)?,
        class: row.get(4)?,
        workspace: row.get(5)?,
        limit: row.get(6)?,
        lifetime,
        origin: GrantOrigin::Room,
        source: row.get(8)?,
        entered_by: row.get(9)?,
        entered_at: row.get(10)?,
        entered_at_ns: row.get(11)?,
        revoked_at: row.get(12)?,
        revoked_by: row.get(13)?,
    })
}
