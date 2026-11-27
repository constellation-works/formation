//! Versioned wire types for proposed shift rosters.

use serde::Deserialize;

/// The decoder preserves JSON diagnostics, including line and column.
pub use serde_json::Error as DecodeError;

/// A draft roster describes seats and directed routes. It grants no authority.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roster {
    /// Wire format version.
    pub version: u32,
    /// Named seats in the shift.
    pub seats: Vec<Seat>,
    /// Proposed one-way communication routes.
    pub routes: Vec<Route>,
}

/// A role and the host interface used by its operator.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    /// Unique, case-sensitive seat name.
    pub name: String,
    /// Formation duty.
    pub role: Role,
    /// Where the session is expected to run.
    pub host: Host,
    /// Workspace whose work this seat covers.
    pub workspace: Option<String>,
    /// Parent seat, or None for the admiral.
    pub reports_to: Option<String>,
}

/// Duties that a seat can hold.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// Coordinates the shift.
    Chief,
    /// Relays the operator’s directions.
    Aide,
    /// Owns one workspace.
    Captain,
    /// Leads an assigned unit of work.
    Lieutenant,
    /// Triages reports for a captain.
    Sergeant,
    /// Observes the workspace and reports changes.
    Sentry,
    /// Advises on a subject across units.
    Specialist,
    /// Advises the chief or admiral.
    Advisor,
    /// Executes assigned work.
    Worker,
}

impl Role {
    /// Stable wire name of this role.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Chief => "chief",
            Self::Aide => "aide",
            Self::Captain => "captain",
            Self::Lieutenant => "lieutenant",
            Self::Sergeant => "sergeant",
            Self::Sentry => "sentry",
            Self::Specialist => "specialist",
            Self::Advisor => "advisor",
            Self::Worker => "worker",
        }
    }
}

/// Expected user interface or worker process.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Host {
    /// A session in Claude Desktop.
    ClaudeDesktop,
    /// A session in Codex Desktop.
    CodexDesktop,
    /// A session controlled by a background host.
    Headless,
}

/// One directed pair of seat names.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    /// Sending seat.
    pub from: String,
    /// Receiving seat.
    pub to: String,
}

/// Decode the roster. Semantic checks belong to the core crate.
///
/// # Errors
/// Returns the JSON decoder's diagnostic for malformed input or unknown fields.
pub fn decode(bytes: &[u8]) -> Result<Roster, DecodeError> {
    serde_json::from_slice(bytes)
}
