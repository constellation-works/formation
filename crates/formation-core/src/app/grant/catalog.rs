//! Single-source catalog of built-in Formation authority classes.

use formation_store::Role;
use formation_store::contract::HouseClass;

/// Built-in authority classes in canonical scope order.
pub const CATALOG: &[ClassSpec<'static>] = &[
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

impl DefaultHolder {
    /// Parse a house file's holder token: `admiral`, `nobody` or a rank name.
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "admiral" => Some(Self::Admiral),
            "nobody" => Some(Self::Nobody),
            _ => Role::from_wire(value).map(Self::Role),
        }
    }
}

/// One authority class: a built-in catalog entry or a class a house file added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassSpec<'a> {
    /// Stable class identifier.
    pub name: &'a str,
    /// Description of decisions covered by this class.
    pub covers: &'a str,
    /// Default holder before any room or house-file grant.
    pub default_holder: DefaultHolder,
    /// Whether this class may be granted to a seat or rank.
    pub grantable: bool,
    /// Optional guidance note for grant candidates.
    pub note: Option<&'a str>,
}

/// Look up a built-in class by name.
pub fn find(name: &str) -> Option<&'static ClassSpec<'static>> {
    CATALOG.iter().find(|spec| spec.name == name)
}

/// The catalog followed by the classes a house file added, all grantable.
pub fn with_house(house: &[HouseClass]) -> Vec<ClassSpec<'_>> {
    let mut classes = CATALOG.to_vec();
    classes.extend(house.iter().map(|class| ClassSpec {
        name: &class.name,
        covers: &class.covers,
        default_holder: DefaultHolder::from_wire(&class.holder).unwrap_or(DefaultHolder::Admiral),
        grantable: true,
        note: class.note.as_deref(),
    }));
    classes
}

/// Comma-separated list of class names for diagnostics.
pub fn names_list(classes: &[ClassSpec<'_>]) -> String {
    classes
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(", ")
}
