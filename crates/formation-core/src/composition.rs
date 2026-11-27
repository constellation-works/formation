//! Process environment captured once and composed per invocation.

use cw_base::{Error, Kind};
use formation_store::{InvalidSessionId, SessionId, Store};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Overrides the base Formation directory (default: `~/.formation`).
const ENV_FORMATION_HOME: &str = "FORMATION_HOME";

/// Default room name or path (overridden by `--room`).
const ENV_FORMATION_ROOM: &str = "FORMATION_ROOM";

/// Claude Code session identifier (takes precedence over `CODEX_THREAD_ID`).
const ENV_CLAUDE_SESSION: &str = "CLAUDE_CODE_SESSION_ID";

/// Codex thread identifier (used when `CLAUDE_CODE_SESSION_ID` is unset).
const ENV_CODEX_SESSION: &str = "CODEX_THREAD_ID";

/// User home directory used for `~/.formation` and `~` room paths.
const ENV_HOME: &str = "HOME";

const MAX_IDENTIFIER_LEN: usize = 96;

/// Host inputs for one invocation.
#[derive(Clone, Default)]
pub struct Env {
    home: Option<OsString>,
    user_home: Option<PathBuf>,
    room: Option<OsString>,
    seat: Option<String>,
    claude_session: Option<OsString>,
    codex_session: Option<OsString>,
    pub(crate) actor: Option<String>,
    pub(crate) cwd: Option<PathBuf>,
}

impl Env {
    /// Capture Formation's process inputs.
    pub fn from_process() -> Self {
        Self {
            home: std::env::var_os(ENV_FORMATION_HOME),
            user_home: std::env::var_os(ENV_HOME).map(PathBuf::from),
            room: std::env::var_os(ENV_FORMATION_ROOM),
            seat: None,
            claude_session: std::env::var_os(ENV_CLAUDE_SESSION),
            codex_session: std::env::var_os(ENV_CODEX_SESSION),
            actor: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .ok(),
            cwd: std::env::current_dir().ok(),
        }
    }

    /// Apply explicit command-line mail options over the captured environment.
    pub fn with_mail_options(mut self, room: Option<String>, seat: Option<String>) -> Self {
        if let Some(room) = room {
            self.room = Some(room.into());
        }
        if let Some(seat) = seat {
            self.seat = Some(seat);
        }
        self
    }

    /// Resolve an unnamed join from the caller's open Scribe group.
    ///
    /// # Errors
    /// Refuses when the session or a unique Scribe seat cannot be resolved.
    pub fn scribe_join(
        mut self,
        explicit_room: bool,
        run: impl FnOnce() -> Result<Vec<u8>, String>,
    ) -> Result<(Self, String), Error> {
        let session = resolve_session(&self)?.ok_or(ComposeError::SeatRequired)?;
        let (group, seat) = lookup_scribe_seat(session.as_str(), run)?;
        if !explicit_room {
            self.room = Some(group.into());
        }
        Ok((self, seat))
    }
}

pub(crate) enum Caller {
    Join {
        session: Option<SessionId>,
        sessions_dir: Option<PathBuf>,
        room_record: String,
    },
    Explicit(String),
    Session(SessionId),
}

pub(crate) struct MailContext {
    pub(crate) room: String,
    pub(crate) store: Store,
    pub(crate) caller: Caller,
}

struct ResolvedRoom {
    path: PathBuf,
    record: String,
}

#[derive(Debug, thiserror::Error)]
enum ComposeError {
    #[error("room required: pass --room or set FORMATION_ROOM")]
    RoomRequired,
    #[error("seat required: pass --as <seat> or join this room from your session")]
    SeatRequired,
    #[error("no scribe seat found; join the scribe group first or pass the seat")]
    NoScribeSeat,
    #[error("multiple scribe seats found: {0}")]
    AmbiguousScribeSeat(String),
    #[error("scribe unavailable: {0}; use formation mail join <seat> --room <room>")]
    ScribeUnavailable(String),
    #[error("invalid room name (use 1-96 letters, digits, hyphens or underscores, or a path)")]
    InvalidRoom,
    #[error("unsupported tilde path; use ~/ or an explicit path")]
    UnsupportedTilde,
    #[error("cannot determine home directory; set FORMATION_HOME or HOME")]
    MissingHome,
    #[error("invalid FORMATION_HOME directory")]
    InvalidHome,
    #[error("invalid identifier (use 1-96 letters, digits, hyphens or underscores)")]
    InvalidSeat,
    #[error("--as does not match the seat name passed to join")]
    ConflictingSeat,
    #[error(transparent)]
    InvalidSession(#[from] InvalidSessionId),
    #[error("cannot resolve working directory: {0}")]
    CurrentDir(#[from] std::io::Error),
}

impl From<ComposeError> for Error {
    fn from(error: ComposeError) -> Self {
        let (kind, code) = match &error {
            ComposeError::NoScribeSeat => (Kind::Refused, "no_scribe_seat"),
            ComposeError::AmbiguousScribeSeat(_) => (Kind::Refused, "ambiguous_scribe_seat"),
            ComposeError::ScribeUnavailable(_) => (Kind::Refused, "scribe_unavailable"),
            ComposeError::RoomRequired => (Kind::Refused, "room_required"),
            ComposeError::SeatRequired => (Kind::Refused, "seat_required"),
            ComposeError::InvalidRoom | ComposeError::UnsupportedTilde => {
                (Kind::Refused, "invalid_room")
            }
            ComposeError::MissingHome | ComposeError::InvalidHome => (Kind::Config, "mail_home"),
            ComposeError::InvalidSeat => (Kind::Refused, "invalid_identifier"),
            ComposeError::ConflictingSeat => (Kind::Refused, "conflicting_seat"),
            ComposeError::InvalidSession(_) => (Kind::Refused, "invalid_session_id"),
            ComposeError::CurrentDir(_) => (Kind::Io, "mail_room"),
        };
        Self::new(kind, code, error)
    }
}

pub(crate) fn compose_join(env: &Env, seat: &str) -> Result<MailContext, Error> {
    if let Some(as_seat) = env.seat.as_deref()
        && as_seat != seat
    {
        return Err(ComposeError::ConflictingSeat.into());
    }

    let session = resolve_session(env)?;
    let room = resolve_room(env, session.as_ref())?;
    let sessions_dir = match &session {
        Some(_) => Some(formation_home(env)?.join("sessions")),
        None => None,
    };
    let store = Store::create(&room.path)?;
    Ok(MailContext {
        room: room.record.clone(),
        store,
        caller: Caller::Join {
            session,
            sessions_dir,
            room_record: room.record,
        },
    })
}

pub(crate) fn compose_room(env: &Env) -> Result<MailContext, Error> {
    let room = resolve_room(env, None)?;
    let store = Store::open(&room.path)?;
    Ok(MailContext {
        room: room.record,
        store,
        caller: Caller::Explicit(String::new()),
    })
}

pub(crate) fn compose_existing(env: &Env) -> Result<MailContext, Error> {
    let session = resolve_session(env)?;
    let room = resolve_room(env, session.as_ref())?;
    let store = Store::open(&room.path)?;

    let caller = if let Some(seat) = env.seat.as_deref() {
        validate_identifier(seat, ComposeError::InvalidSeat)?;
        Caller::Explicit(seat.to_owned())
    } else if let Some(session_id) = session {
        let bound = store.read()?.seats().by_session(&session_id)?;
        if bound.is_none() {
            return Err(ComposeError::SeatRequired.into());
        }
        Caller::Session(session_id)
    } else {
        return Err(ComposeError::SeatRequired.into());
    };

    Ok(MailContext {
        room: room.record,
        store,
        caller,
    })
}

pub(crate) fn compose_hook(env: &Env, session: &str) -> Result<Option<MailContext>, Error> {
    let session = SessionId::try_new(session).map_err(ComposeError::from)?;
    let sessions_dir = formation_home(env)?.join("sessions");
    let Some(recorded) = Store::session_room(&sessions_dir, &session)? else {
        return Ok(None);
    };
    let room = parse_room_spec(env, &recorded)?;
    let store = Store::open(&room.path)?;
    Ok(Some(MailContext {
        room: room.record,
        store,
        caller: Caller::Session(session),
    }))
}

pub(crate) fn is_hook_room(env: &Env, room: &str, session: &SessionId) -> Result<bool, Error> {
    let sessions_dir = formation_home(env)?.join("sessions");
    let Some(recorded) = Store::session_room(&sessions_dir, session)? else {
        return Ok(false);
    };
    if recorded == room {
        return Ok(true);
    }
    let bound = parse_room_spec(env, &recorded)?.path.canonicalize();
    let current = parse_room_spec(env, room)?.path.canonicalize();
    Ok(matches!((bound, current), (Ok(bound), Ok(current)) if bound == current))
}

fn resolve_session(env: &Env) -> Result<Option<SessionId>, Error> {
    let raw = env.claude_session.as_ref().or(env.codex_session.as_ref());
    let Some(raw) = raw else {
        return Ok(None);
    };
    let text = raw
        .to_str()
        .ok_or_else(|| Error::from(ComposeError::InvalidSession(InvalidSessionId)))?;
    let session = SessionId::try_new(text).map_err(ComposeError::from)?;
    Ok(Some(session))
}

fn resolve_room(env: &Env, session: Option<&SessionId>) -> Result<ResolvedRoom, Error> {
    if let Some(raw) = env.room.as_ref() {
        let text = raw
            .to_str()
            .ok_or_else(|| Error::from(ComposeError::InvalidRoom))?;
        return parse_room_spec(env, text);
    }

    if let Some(session_id) = session {
        let sessions_dir = formation_home(env)?.join("sessions");
        if let Some(recorded) = Store::session_room(&sessions_dir, session_id)? {
            return parse_room_spec(env, &recorded);
        }
    }

    Err(ComposeError::RoomRequired.into())
}

fn parse_room_spec(env: &Env, raw: &str) -> Result<ResolvedRoom, Error> {
    if raw.is_empty() {
        return Err(ComposeError::InvalidRoom.into());
    }

    let is_path =
        raw.starts_with('.') || raw.starts_with('~') || raw.chars().any(std::path::is_separator);

    if is_path {
        let path = expand_path(env, raw)?;
        let record = absolute_record(&path)?;
        return Ok(ResolvedRoom { path, record });
    }

    validate_identifier(raw, ComposeError::InvalidRoom)?;
    let path = formation_home(env)?.join("rooms").join(raw);
    Ok(ResolvedRoom {
        path,
        record: raw.to_owned(),
    })
}

fn expand_path(env: &Env, raw: &str) -> Result<PathBuf, Error> {
    if raw == "~" {
        return user_home(env);
    }
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        return Ok(user_home(env)?.join(rest));
    }
    if raw.starts_with('~') {
        return Err(ComposeError::UnsupportedTilde.into());
    }
    Ok(PathBuf::from(raw))
}

fn absolute_record(path: &Path) -> Result<String, Error> {
    let full = if path.is_relative() {
        std::env::current_dir()
            .map_err(ComposeError::CurrentDir)?
            .join(path)
    } else {
        path.to_path_buf()
    };
    full.into_os_string()
        .into_string()
        .map_err(|_| ComposeError::InvalidRoom.into())
}

fn formation_home(env: &Env) -> Result<PathBuf, Error> {
    if let Some(home) = env.home.as_ref() {
        if home.is_empty() {
            return Err(ComposeError::InvalidHome.into());
        }
        return Ok(PathBuf::from(home));
    }
    Ok(user_home(env)?.join(".formation"))
}

fn user_home(env: &Env) -> Result<PathBuf, Error> {
    env.user_home
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty())
        .cloned()
        .ok_or_else(|| ComposeError::MissingHome.into())
}

fn validate_identifier(value: &str, error: ComposeError) -> Result<(), Error> {
    if value.len() <= MAX_IDENTIFIER_LEN && formation_store::contract::identifier(value) {
        Ok(())
    } else {
        Err(error.into())
    }
}

fn lookup_scribe_seat(
    session: &str,
    run: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<(String, String), Error> {
    #[derive(serde::Deserialize)]
    struct List {
        items: Vec<Group>,
    }
    #[derive(serde::Deserialize)]
    struct Group {
        id: String,
        closed: Option<serde_json::Value>,
        seats: Vec<Seat>,
    }
    #[derive(serde::Deserialize)]
    struct Seat {
        name: String,
        transcripts: Vec<String>,
    }

    let unavailable = |message: String| -> Error {
        ComposeError::ScribeUnavailable(
            message.lines().next().unwrap_or("unknown error").to_owned(),
        )
        .into()
    };
    let output = run().map_err(&unavailable)?;
    let list: List =
        serde_json::from_slice(&output).map_err(|error| unavailable(error.to_string()))?;
    let mut matches = Vec::new();
    for group in list
        .items
        .into_iter()
        .filter(|group| group.closed.is_none())
    {
        for seat in group.seats {
            if seat.transcripts.iter().any(|id| id == session) {
                matches.push((group.id.clone(), seat.name));
            }
        }
    }
    match matches.len() {
        0 => Err(ComposeError::NoScribeSeat.into()),
        1 => Ok(matches.remove(0)),
        _ => Err(ComposeError::AmbiguousScribeSeat(
            matches
                .iter()
                .map(|(group, seat)| format!("{group}/{seat}"))
                .collect::<Vec<_>>()
                .join(", "),
        )
        .into()),
    }
}
