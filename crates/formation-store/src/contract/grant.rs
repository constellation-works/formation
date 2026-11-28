//! Authority grant record shapes.

use serde::Serialize;

/// Whether a grant record widens or narrows authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantEffect {
    /// Permit settling the class in scope.
    Grant,
    /// Remove a default or broader grant for the class in scope.
    Withhold,
}

impl GrantEffect {
    /// Parse a stored wire token.
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "grant" => Some(Self::Grant),
            "withhold" => Some(Self::Withhold),
            _ => None,
        }
    }

    /// Stable wire token for this effect.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Grant => "grant",
            Self::Withhold => "withhold",
        }
    }
}

/// Whether a grant targets an individual seat or every seat of a rank.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantTargetKind {
    /// One named seat in the applied roster.
    Seat,
    /// Every seat holding the named rank in the applied roster.
    Rank,
}

impl GrantTargetKind {
    /// Parse a stored wire token.
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "seat" => Some(Self::Seat),
            "rank" => Some(Self::Rank),
            _ => None,
        }
    }

    /// Stable wire token for this target kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Seat => "seat",
            Self::Rank => "rank",
        }
    }
}

/// How long a grant remains in effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantLifetime {
    /// Ends with the room.
    Day,
    /// Loaded from the operator's standing grants file.
    Standing,
}

impl GrantLifetime {
    /// Parse a stored wire token.
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "day" => Some(Self::Day),
            "standing" => Some(Self::Standing),
            _ => None,
        }
    }

    /// Stable wire token for this lifetime.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Standing => "standing",
        }
    }
}

/// Layer where an authority rule originated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantOrigin {
    /// Built-in default from the class catalog.
    Catalog,
    /// Standing rule loaded from a house grants file.
    HouseFile,
    /// Day grant or withhold stored in the room.
    Room,
}

impl GrantOrigin {
    /// Parse a stored wire token for a stored layer (`room` or `house_file`).
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "room" => Some(Self::Room),
            "house_file" => Some(Self::HouseFile),
            _ => None,
        }
    }

    /// Stable wire token for this layer.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::HouseFile => "house_file",
            Self::Room => "room",
        }
    }
}

/// One stored grant or withhold record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantRecord {
    /// Monotonic identifier within the room.
    pub id: i64,
    /// Whether this record grants or withholds authority.
    pub effect: GrantEffect,
    /// Whether `target` names a seat or a rank.
    pub target_kind: GrantTargetKind,
    /// Seat name or rank wire name.
    pub target: String,
    /// Authority class identifier.
    pub class: String,
    /// Workspace identifier, or `"all"`.
    pub workspace: String,
    /// Optional condition in words narrowing this record.
    pub limit: Option<String>,
    /// Lifetime (`day` or `standing`).
    pub lifetime: GrantLifetime,
    /// Origin layer for precedence resolution.
    pub origin: GrantOrigin,
    /// Operator direction source text.
    pub source: String,
    /// Seat that entered the record, or `None` outside a session.
    pub entered_by: Option<String>,
    /// Unix seconds when the record was entered.
    pub entered_at: i64,
    /// Sub-second nanoseconds (`0..1_000_000_000`) when entered.
    pub entered_at_ns: i64,
    /// Unix seconds when revoked, if revoked.
    pub revoked_at: Option<i64>,
    /// Seat that revoked the record, or `None` outside a session.
    pub revoked_by: Option<String>,
}

/// Parameters for inserting a new room grant or withhold.
pub struct NewGrant<'a> {
    /// Whether this record grants or withholds authority.
    pub effect: GrantEffect,
    /// Whether `target` names a seat or a rank.
    pub target_kind: GrantTargetKind,
    /// Seat name or rank wire name.
    pub target: &'a str,
    /// Authority class identifier.
    pub class: &'a str,
    /// Workspace identifier, or `"all"`.
    pub workspace: &'a str,
    /// Optional condition in words narrowing this record.
    pub limit: Option<&'a str>,
    /// Lifetime (`day` or `standing`).
    pub lifetime: GrantLifetime,
    /// Layer the record belongs to: the room, or the house grants file.
    pub origin: GrantOrigin,
    /// Operator direction source text.
    pub source: &'a str,
    /// Seat that entered the record, or `None` outside a session.
    pub entered_by: Option<&'a str>,
    /// Unix seconds when entered.
    pub entered_at: i64,
    /// Sub-second nanoseconds (`0..1_000_000_000`) when entered.
    pub entered_at_ns: i64,
}

/// Outcome of attempting to revoke a grant by identifier.
#[derive(Debug, PartialEq, Eq)]
pub enum RevokeStatus {
    /// The active grant was marked revoked.
    Revoked(Box<GrantRecord>),
    /// The grant existed but was already revoked.
    AlreadyRevoked,
    /// No grant with that identifier exists in the room.
    NotFound,
}

/// A class a house grants file adds to the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseClass {
    /// Stable class identifier.
    pub name: String,
    /// Description of the decisions the class covers.
    pub covers: String,
    /// Default holder wire token: `admiral`, `nobody` or a rank.
    pub holder: String,
    /// Optional guidance note for grant candidates.
    pub note: Option<String>,
}
