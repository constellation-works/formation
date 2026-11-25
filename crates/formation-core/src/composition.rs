//! Process environment captured once and composed per invocation.

use cw_base::{Error, Kind};
use formation_store::Store;
use std::path::PathBuf;

/// Host inputs for one invocation.
#[derive(Default)]
pub struct Env {
    room: Option<PathBuf>,
    identity: Option<PathBuf>,
    pub(crate) actor: Option<String>,
}

impl Env {
    /// Capture Formation's process inputs.
    pub fn from_process() -> Self {
        Self {
            room: std::env::var_os("FORMATION_ROOM").map(Into::into),
            identity: std::env::var_os("FORMATION_IDENTITY").map(Into::into),
            actor: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .ok(),
        }
    }

    /// Apply explicit command-line paths over the captured environment.
    pub fn with_mail_paths(mut self, room: Option<PathBuf>, identity: Option<PathBuf>) -> Self {
        self.room = room.or(self.room);
        self.identity = identity.or(self.identity);
        self
    }
}

#[derive(Debug, thiserror::Error)]
#[error("mail requires --{0} or its FORMATION environment variable")]
struct Missing(&'static str);

pub(crate) fn compose(env: &Env) -> Result<(Store, PathBuf), Error> {
    let path = env
        .room
        .as_ref()
        .ok_or_else(|| Error::new(Kind::Config, "room_required", Missing("room")))?;
    let identity = env
        .identity
        .as_ref()
        .ok_or_else(|| Error::new(Kind::Config, "identity_required", Missing("identity")))?;
    Ok((Store::open(path)?, identity.clone()))
}
