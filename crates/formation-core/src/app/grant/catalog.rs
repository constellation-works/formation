//! Single-source catalog of built-in Formation authority classes.

use formation_store::Role;

/// Built-in authority classes in canonical scope order.
pub const CATALOG: &[ClassSpec] = &[
    ClassSpec {
        name: "repair",
        covers: "Retry a check, repair a session-owned pull request or, in repair mode, a red \
                 or conflicting one, clear an unambiguous pilot finding, file a fix or a friction",
        default_holder: DefaultHolder::SergeantOrCaptain,
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "ship",
        covers: "Approve and ship a fix, an approved design, a non-breaking refactor, an \
                 improvement or a small scoped feature",
        default_holder: DefaultHolder::Role(Role::Captain),
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "land",
        covers: "Land a reviewed pull request",
        default_holder: DefaultHolder::Role(Role::Captain),
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "staffing",
        covers: "One reinforcement, up to a squad plus one lieutenant per workspace",
        default_holder: DefaultHolder::Role(Role::Chief),
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "design",
        covers: "Approve a design or a major feature",
        default_holder: DefaultHolder::Admiral,
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "breaking",
        covers: "A breaking change",
        default_holder: DefaultHolder::Admiral,
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "direction",
        covers: "Product scope, architecture, security, material cost, an irreversible action",
        default_holder: DefaultHolder::Admiral,
        grantable: true,
        note: Some("poor grant candidate for irreversible actions; limit to reversible choices"),
    },
    ClassSpec {
        name: "release",
        covers: "A release, a tag, publishing",
        default_holder: DefaultHolder::Admiral,
        grantable: true,
        note: Some("poor grant candidate; releases and publishing are hard to reverse"),
    },
    ClassSpec {
        name: "operator",
        covers: "A decision that needs the operator's identity, a routine change, a plugin \
                 install, starting a drain",
        default_holder: DefaultHolder::Admiral,
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "override",
        covers: "Put a tracker's operator override in front of one refused command, for an act in \
                 the seat's own rows",
        default_holder: DefaultHolder::Nobody,
        grantable: true,
        note: None,
    },
    ClassSpec {
        name: "credentials",
        covers: "Credentials and sign-ins",
        default_holder: DefaultHolder::Admiral,
        grantable: false,
        note: Some("never granted"),
    },
    ClassSpec {
        name: "grant",
        covers: "Giving or taking back authority",
        default_holder: DefaultHolder::Admiral,
        grantable: false,
        note: Some("never granted"),
    },
];

/// Default holder of an authority class before any room or house-file grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultHolder {
    /// Held by sergeants, or by a workspace's captain when that workspace has no sergeant.
    SergeantOrCaptain,
    /// Held by a specific formation role.
    Role(Role),
    /// Held by the admiral by default.
    Admiral,
    /// Held by nobody by default; requires an explicit grant.
    Nobody,
}

impl DefaultHolder {
    /// Human-readable description of the default holder for diagnostics.
    pub fn label(&self) -> &'static str {
        match self {
            Self::SergeantOrCaptain => "sergeant (or captain where none)",
            Self::Role(role) => role.as_str(),
            Self::Admiral => "admiral",
            Self::Nobody => "nobody",
        }
    }
}

/// One authority class entry in the built-in catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassSpec {
    /// Stable class identifier.
    pub name: &'static str,
    /// Description of decisions covered by this class.
    pub covers: &'static str,
    /// Default holder before any room or house-file grant.
    pub default_holder: DefaultHolder,
    /// Whether this class may be granted to a seat or rank.
    pub grantable: bool,
    /// Optional guidance note for grant candidates.
    pub note: Option<&'static str>,
}

/// Look up a built-in class by name.
pub fn find(name: &str) -> Option<&'static ClassSpec> {
    CATALOG.iter().find(|spec| spec.name == name)
}

/// Comma-separated list of built-in class names for diagnostics.
pub fn names_list() -> String {
    CATALOG
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(", ")
}
