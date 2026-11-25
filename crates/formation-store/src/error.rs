//! Typed storage failures translated at the store boundary.

use cw_base::{Error, Kind};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Failure {
    #[error(transparent)]
    File(#[from] cw_fs::Error),
    #[error(transparent)]
    Database(#[from] cw_sqlite::Error),
    #[error("mail store: {0}")]
    Query(#[from] cw_sqlite::rusqlite::Error),
    #[error("identity must be a private 32-byte regular file")]
    InvalidKey,
    #[error("room must be an existing private directory (mode 0700)")]
    MissingRoom,
    #[error("cannot generate a seat credential: {0}")]
    Entropy(getrandom::Error),
    #[error("cannot encode audit event: {0}")]
    Encode(#[from] serde_json::Error),
}

impl From<Failure> for Error {
    fn from(error: Failure) -> Self {
        let (kind, code) = match &error {
            Failure::File(source) => (source.kind(), "mail_file"),
            Failure::Database(source) => (source.kind(), "mail_database"),
            Failure::Query(_) => (Kind::Io, "mail_query"),
            Failure::InvalidKey => (Kind::Config, "mail_identity"),
            Failure::MissingRoom => (Kind::Config, "mail_room"),
            Failure::Entropy(_) => (Kind::Io, "mail_entropy"),
            Failure::Encode(_) => (Kind::Internal, "audit_encode"),
        };
        Self::new(kind, code, error)
    }
}

pub(crate) fn sql(error: cw_sqlite::rusqlite::Error) -> Error {
    Failure::from(error).into()
}
