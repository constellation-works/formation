//! Offline validation of a proposed Formation roster.

use formation_store::contracts::{identifier, roster::Role};
use serde::Serialize;
use std::collections::HashSet;
use thiserror::Error;

/// Successful roster check. Counts describe a draft only.
#[derive(Debug, Serialize)]
pub struct Summary {
    /// Format version.
    pub version: u32,
    /// Unique seats in the draft.
    pub seats: usize,
    /// Unique directed routes.
    pub routes: usize,
}

/// A draft could not be used as a roster.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid wire document.
    #[error("invalid roster JSON: {0}")]
    Decode(#[from] formation_store::DecodeError),
    /// Unsupported schema or inconsistent draft entries.
    #[error("{0}")]
    Invalid(&'static str),
}

/// Validate version, seat identity, workspace ownership and route references.
///
/// # Errors
/// Returns a diagnostic when the draft is malformed or inconsistent.
pub fn check(bytes: &[u8]) -> Result<Summary, Error> {
    let roster = formation_store::decode(bytes)?;
    if roster.version != 1 {
        return Err(Error::Invalid("expected roster version 1"));
    }
    if roster.seats.is_empty() {
        return Err(Error::Invalid("expected at least one seat"));
    }
    let mut seats = HashSet::new();
    let mut captains = HashSet::new();
    for seat in &roster.seats {
        if !identifier(&seat.name) || !seats.insert(seat.name.as_str()) {
            return Err(Error::Invalid("seat names must be unique identifiers"));
        }
        if seat.workspace.as_deref().is_some_and(|w| !identifier(w)) {
            return Err(Error::Invalid("invalid workspace identifier"));
        }
        if seat.role == Role::Captain {
            let workspace = seat
                .workspace
                .as_deref()
                .ok_or(Error::Invalid("a captain needs a workspace"))?;
            if !captains.insert(workspace) {
                return Err(Error::Invalid("a workspace has more than one captain"));
            }
        }
    }
    let mut routes = HashSet::new();
    for route in &roster.routes {
        if !seats.contains(route.from.as_str()) || !seats.contains(route.to.as_str()) {
            return Err(Error::Invalid("a route names an unknown seat"));
        }
        if route.from == route.to || !routes.insert((&route.from, &route.to)) {
            return Err(Error::Invalid("expected distinct, unique directed routes"));
        }
    }
    Ok(Summary {
        version: roster.version,
        seats: seats.len(),
        routes: routes.len(),
    })
}
