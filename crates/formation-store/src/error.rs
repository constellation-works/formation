//! Typed storage failures translated at the store boundary.

use cw_base::{Error, Kind};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Failure {
    #[error(transparent)]
    File(#[from] cw_fs::Error),
    #[error(transparent)]
    Database(#[from] cw_sqlite::Error),
    #[error("mail store: {0}")]
    Query(#[from] cw_sqlite::rusqlite::Error),
    #[error("room not found: {}; run `formation mail join` first", .0.display())]
    RoomNotFound(PathBuf),
    #[error("cannot create seat folder {}", .0.display())]
    SeatDir(PathBuf, #[source] cw_fs::Error),
    #[error("cannot resolve seat folder {}", .0.display())]
    SeatDirPath(PathBuf, #[source] std::io::Error),
    #[error("invalid session file: {}", .0.display())]
    InvalidSessionFile(PathBuf),
    #[error("cannot encode audit event: {0}")]
    Encode(#[from] serde_json::Error),
}

impl From<Failure> for Error {
    fn from(error: Failure) -> Self {
        let (kind, code) = match &error {
            Failure::File(source) => (source.kind(), "mail_file"),
            Failure::Database(source) => (source.kind(), "mail_database"),
            Failure::Query(_) => (Kind::Io, "mail_query"),
            Failure::SeatDir(_, source) => (source.kind(), "seat_dir_unavailable"),
            Failure::SeatDirPath(..) => (Kind::Io, "seat_dir_unavailable"),
            Failure::RoomNotFound(_) => (Kind::NotFound, "room_not_found"),
            Failure::InvalidSessionFile(_) => (Kind::Config, "invalid_session_file"),
            Failure::Encode(_) => (Kind::Internal, "audit_encode"),
        };
        Self::new(kind, code, error)
    }
}

pub(crate) fn sql(error: cw_sqlite::rusqlite::Error) -> Error {
    Failure::from(error).into()
}
