//! Room request rows and their stored options, advice and settlement.

use crate::contract::{Request, RequestSettlement, RequestState};
use crate::error;
use crate::error::Failure;
use cw_base::Error;
use cw_sqlite::rusqlite::types::Type;
use cw_sqlite::rusqlite::{Connection, Error as SqlError, OptionalExtension, Row, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use time::OffsetDateTime;

const COLUMNS: &str = "id, asker, class, workspace, record, ask, options, recommendation,
    blocking, opened_at, updated_at, holder, settlement, advice, overturned";

/// Requests visible in a room snapshot.
pub struct RequestRead<'a> {
    pub(crate) conn: &'a Connection,
    pub(crate) available: bool,
    pub(crate) history: bool,
}

impl RequestRead<'_> {
    /// Look up a request in this room. Rooms predating requests have no records.
    pub fn get(&self, id: &str) -> Result<Option<Request>, Error> {
        if !self.available {
            return Ok(None);
        }
        let columns = if self.history {
            COLUMNS
        } else {
            "id, asker, class, workspace, record, ask, options, recommendation,
             blocking, opened_at, updated_at, holder, settlement, advice, '[]'"
        };
        let query = format!("SELECT {columns} FROM room_request WHERE id = ?");
        self.conn
            .query_row(&query, [id], decode)
            .optional()
            .map_err(error::sql)
    }

    /// Read the room's requests in creation order, newest first.
    pub fn list(&self) -> Result<Vec<Request>, Error> {
        if !self.available {
            return Ok(Vec::new());
        }
        let columns = if self.history {
            COLUMNS
        } else {
            "id, asker, class, workspace, record, ask, options, recommendation,
             blocking, opened_at, updated_at, holder, settlement, advice, '[]'"
        };
        let query = format!("SELECT {columns} FROM room_request ORDER BY opened_at DESC, id");
        let mut statement = self.conn.prepare(&query).map_err(error::sql)?;
        statement
            .query_map([], decode)
            .map_err(error::sql)?
            .collect::<Result<_, _>>()
            .map_err(error::sql)
    }
}

/// Requests inside a room write transaction.
pub struct RequestMut<'a> {
    pub(crate) conn: &'a Connection,
}

impl RequestMut<'_> {
    /// Generate a random suffix so identifiers can survive a move between rooms.
    pub fn new_suffix(&self) -> Result<String, Error> {
        self.conn
            .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
            .map_err(error::sql)
    }

    /// Look up the request under the same lock that will write the change.
    pub fn get(&self, id: &str) -> Result<Option<Request>, Error> {
        RequestRead {
            conn: self.conn,
            available: true,
            history: true,
        }
        .get(id)
    }

    /// Insert a new request. An existing request with the same identifier is never replaced.
    pub fn insert(&self, request: &Request) -> Result<(), Error> {
        let (holder, settlement) = state_parts(&request.state)?;

        self.conn
            .execute(
                &format!(
                    "INSERT INTO room_request ({COLUMNS})
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                ),
                params![
                    request.id,
                    request.asker,
                    request.class,
                    request.workspace,
                    request.record,
                    request.ask,
                    encode(&request.options)?,
                    request.recommendation,
                    request.blocking,
                    encode_time(request.opened_at)?,
                    encode_time(request.updated_at)?,
                    holder,
                    settlement,
                    encode(&request.advice)?,
                    encode(&request.overturned)?,
                ],
            )
            .map(|_| ())
            .map_err(error::sql)
    }

    /// Record a change to advice, holder or settlement on a request read in this transaction.
    pub fn update(&self, request: &Request) -> Result<(), Error> {
        let (holder, settlement) = state_parts(&request.state)?;

        let changed = self
            .conn
            .execute(
                "UPDATE room_request SET updated_at = ?, holder = ?, settlement = ?,
                 advice = ?, overturned = ? WHERE id = ?",
                params![
                    encode_time(request.updated_at)?,
                    holder,
                    settlement,
                    encode(&request.advice)?,
                    encode(&request.overturned)?,
                    request.id,
                ],
            )
            .map_err(error::sql)?;
        if changed == 0 {
            return Err(error::sql(SqlError::QueryReturnedNoRows));
        }
        Ok(())
    }
}

fn decode(row: &Row<'_>) -> Result<Request, SqlError> {
    let settlement = row
        .get::<_, Option<String>>(12)?
        .map(|raw| decode_json::<RequestSettlement>(&raw, 12))
        .transpose()?;
    let state = match settlement {
        Some(settlement) => RequestState::Settled(settlement),
        None => RequestState::Open {
            holder: row.get(11)?,
        },
    };

    Ok(Request {
        id: row.get(0)?,
        asker: row.get(1)?,
        class: row.get(2)?,
        workspace: row.get(3)?,
        record: row.get(4)?,
        ask: row.get(5)?,
        options: decode_json(&row.get::<_, String>(6)?, 6)?,
        recommendation: row.get(7)?,
        blocking: row.get(8)?,
        opened_at: decode_time(row, 9)?,
        updated_at: decode_time(row, 10)?,
        state,
        advice: decode_json(&row.get::<_, String>(13)?, 13)?,
        overturned: decode_json(&row.get::<_, String>(14)?, 14)?,
    })
}

fn state_parts(state: &RequestState) -> Result<(Option<&str>, Option<String>), Error> {
    match state {
        RequestState::Open { holder } => Ok((holder.as_deref(), None)),
        RequestState::Settled(settlement) => Ok((None, Some(encode(settlement)?))),
    }
}

fn encode(value: &impl Serialize) -> Result<String, Error> {
    serde_json::to_string(value)
        .map_err(Failure::RequestEncode)
        .map_err(Into::into)
}

fn decode_json<T: DeserializeOwned>(raw: &str, index: usize) -> Result<T, SqlError> {
    serde_json::from_str(raw)
        .map_err(|error| SqlError::FromSqlConversionFailure(index, Type::Text, Box::new(error)))
}

fn encode_time(value: OffsetDateTime) -> Result<i64, Error> {
    i64::try_from(value.unix_timestamp_nanos())
        .map_err(|error| Failure::RequestTimestamp(error).into())
}

fn decode_time(row: &Row<'_>, index: usize) -> Result<OffsetDateTime, SqlError> {
    let nanos: i64 = row.get(index)?;
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos))
        .map_err(|error| SqlError::FromSqlConversionFailure(index, Type::Integer, Box::new(error)))
}
