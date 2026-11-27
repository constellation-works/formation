//! Pure validation, precedence evaluation and notification formatting for grants.

use super::catalog::{ClassSpec, DefaultHolder, names_list};
use cw_base::{Error, Kind};
use formation_store::contract::{
    GrantEffect, GrantLifetime, GrantOrigin, GrantRecord, GrantTargetKind, identifier,
};
use formation_store::{Role, RosterSeat};
use std::collections::BTreeSet;

/// Wildcard workspace matching every workspace in the room.
pub const ALL_WORKSPACES: &str = "all";

const MAX_IDENTIFIER_LEN: usize = 96;

const MAX_TEXT_LEN: usize = 4096;

/// One effective authority line for a seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveEntry {
    /// Authority class identifier.
    pub class: String,
    /// Workspace scope (`"all"` or a specific workspace name).
    pub workspace: String,
    /// Layer where the winning entry came from.
    pub origin: GrantOrigin,
    /// Whether the winning entry grants or withholds authority.
    pub effect: GrantEffect,
    /// Winning room grant identifier, or `None` for a catalog default.
    pub grant_id: Option<i64>,
    /// Optional condition in words from the winning grant.
    pub limit: Option<String>,
    /// Whether the winning grant was entered by its own holder.
    pub self_entered: bool,
}

impl EffectiveEntry {
    /// Wire token naming where this line came from (`catalog`, `grant` or `withhold`).
    pub fn from_kind(&self) -> &'static str {
        match (self.origin, self.effect) {
            (GrantOrigin::Catalog, _) => "catalog",
            (_, GrantEffect::Grant) => "grant",
            (_, GrantEffect::Withhold) => "withhold",
        }
    }
}

/// Effective authority resolved for one seat in the applied roster.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveSeat {
    /// Seat name.
    pub seat: String,
    /// Seat role wire name.
    pub role: String,
    /// Seat workspace, if any.
    pub workspace: Option<String>,
    /// Classes and workspaces the seat may settle now.
    pub settles: Vec<EffectiveEntry>,
    /// Classes and workspaces explicitly withheld from the seat.
    pub withheld: Vec<EffectiveEntry>,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Refusal(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Precedence {
    target_rank: u8,
    workspace_rank: u8,
    origin_rank: u8,
    entered_at: i64,
    entered_at_ns: i64,
    effect_rank: u8,
    id: i64,
}

pub(super) fn refused(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(Kind::Refused, code, Refusal(message.into()))
}

pub(super) fn validate_class_for_mutation<'a>(
    class: &str,
    catalog: &'a [ClassSpec],
) -> Result<&'a ClassSpec, Error> {
    let Some(spec) = catalog.iter().find(|entry| entry.name == class) else {
        return Err(refused(
            "unknown_class",
            format!(
                "unknown authority class `{class}`; valid classes: {}",
                names_list()
            ),
        ));
    };
    if !spec.grantable {
        return Err(refused(
            "class_not_grantable",
            format!("class `{class}` stays with the admiral and can never be granted"),
        ));
    }
    Ok(spec)
}

pub(super) fn validate_class_for_check<'a>(
    class: &str,
    catalog: &'a [ClassSpec],
    grants: &[GrantRecord],
) -> Result<Option<&'a ClassSpec>, Error> {
    if let Some(spec) = catalog.iter().find(|entry| entry.name == class) {
        return Ok(Some(spec));
    }
    if grants.iter().any(|grant| grant.class == class) {
        return Ok(None);
    }
    Err(refused(
        "unknown_class",
        format!(
            "unknown authority class `{class}`; valid classes: {}",
            names_list()
        ),
    ))
}

pub(super) fn validate_workspace(workspace: &str) -> Result<&str, Error> {
    if workspace == ALL_WORKSPACES
        || (workspace.len() <= MAX_IDENTIFIER_LEN && identifier(workspace))
    {
        Ok(workspace)
    } else {
        Err(refused(
            "invalid_workspace",
            "invalid workspace (use `all` or 1-96 ASCII letters, digits, hyphens or underscores)",
        ))
    }
}

pub(super) fn validate_lifetime(lifetime: &str) -> Result<GrantLifetime, Error> {
    match lifetime {
        "day" => Ok(GrantLifetime::Day),
        "standing" => Err(refused(
            "invalid_lifetime",
            "standing grants are loaded from a grants file, not added by command; use `--for day`",
        )),
        _ => Err(refused(
            "invalid_lifetime",
            format!("invalid lifetime `{lifetime}`; use `--for day`"),
        )),
    }
}

pub(super) fn validate_source(source: &str) -> Result<&str, Error> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(refused(
            "source_required",
            "`--source` must name where the admiral gave this direction",
        ));
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(refused(
            "invalid_source",
            format!("`--source` exceeds {MAX_TEXT_LEN} bytes"),
        ));
    }
    Ok(trimmed)
}

pub(super) fn validate_limit(limit: Option<&str>) -> Result<Option<&str>, Error> {
    let Some(raw) = limit else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(refused("invalid_limit", "`--limit` cannot be empty"));
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(refused(
            "invalid_limit",
            format!("`--limit` exceeds {MAX_TEXT_LEN} bytes"),
        ));
    }
    Ok(Some(trimmed))
}

pub(super) fn resolve_holder(
    to: &str,
    roster: &[RosterSeat],
) -> Result<(GrantTargetKind, String), Error> {
    if to.len() > MAX_IDENTIFIER_LEN || !identifier(to) {
        return Err(refused(
            "unknown_holder",
            format!("invalid holder `{to}`; name a rank or a seat in the applied roster"),
        ));
    }
    if let Some(role) = Role::from_wire(to) {
        return Ok((GrantTargetKind::Rank, role.as_str().to_owned()));
    }
    if roster.iter().any(|seat| seat.name == to) {
        return Ok((GrantTargetKind::Seat, to.to_owned()));
    }
    Err(refused(
        "unknown_holder",
        format!("`{to}` is neither a known rank nor a seat in the applied roster"),
    ))
}

pub(super) fn is_self_entered(grant: &GrantRecord, roster: &[RosterSeat]) -> bool {
    if grant.effect != GrantEffect::Grant {
        return false;
    }
    let Some(entered_by) = grant.entered_by.as_deref() else {
        return false;
    };
    match grant.target_kind {
        GrantTargetKind::Seat => grant.target == entered_by,
        GrantTargetKind::Rank => roster
            .iter()
            .find(|seat| seat.name == entered_by)
            .is_some_and(|seat| seat.role == grant.target),
    }
}

pub(super) fn effective_for_seat(
    seat: &RosterSeat,
    roster: &[RosterSeat],
    grants: &[GrantRecord],
    catalog: &[ClassSpec],
) -> EffectiveSeat {
    let base_workspace = seat.workspace.as_deref().unwrap_or(ALL_WORKSPACES);
    let mut class_names: Vec<&str> = catalog.iter().map(|spec| spec.name).collect();
    let mut extra_classes = BTreeSet::new();
    for grant in grants {
        if grant.revoked_at.is_none() && !class_names.contains(&grant.class.as_str()) {
            extra_classes.insert(grant.class.as_str());
        }
    }
    class_names.extend(extra_classes);

    let mut settles = Vec::new();
    let mut withheld = Vec::new();

    for class_name in class_names {
        let mut workspaces = Vec::new();
        workspaces.push(base_workspace);
        let mut extra_workspaces = BTreeSet::new();
        for grant in grants {
            if grant.revoked_at.is_some() || grant.class != class_name {
                continue;
            }
            if grant.workspace == ALL_WORKSPACES || grant.workspace == base_workspace {
                continue;
            }
            let applies_extra = match grant.target_kind {
                GrantTargetKind::Seat => grant.target == seat.name,
                GrantTargetKind::Rank => seat.workspace.is_none() && grant.target == seat.role,
            };
            if applies_extra {
                extra_workspaces.insert(grant.workspace.as_str());
            }
        }
        workspaces.extend(extra_workspaces);

        for workspace in workspaces {
            let Some(winner) = evaluate_one(seat, class_name, workspace, roster, grants, catalog)
            else {
                continue;
            };
            match winner.effect {
                GrantEffect::Grant => settles.push(winner),
                GrantEffect::Withhold => withheld.push(winner),
            }
        }
    }

    EffectiveSeat {
        seat: seat.name.clone(),
        role: seat.role.clone(),
        workspace: seat.workspace.clone(),
        settles,
        withheld,
    }
}

pub(super) fn evaluate_one(
    seat: &RosterSeat,
    class: &str,
    query_workspace: &str,
    roster: &[RosterSeat],
    grants: &[GrantRecord],
    catalog: &[ClassSpec],
) -> Option<EffectiveEntry> {
    let mut best: Option<(Precedence, EffectiveEntry)> = None;

    if let Some(spec) = catalog.iter().find(|entry| entry.name == class)
        && holds_catalog_default(seat, spec.default_holder, query_workspace, roster)
    {
        let precedence = Precedence {
            target_rank: 1,
            workspace_rank: 1,
            origin_rank: 1,
            entered_at: i64::MIN,
            entered_at_ns: 0,
            effect_rank: 1,
            id: 0,
        };
        best = Some((
            precedence,
            EffectiveEntry {
                class: class.to_owned(),
                workspace: query_workspace.to_owned(),
                origin: GrantOrigin::Catalog,
                effect: GrantEffect::Grant,
                grant_id: None,
                limit: None,
                self_entered: false,
            },
        ));
    }

    for grant in grants {
        if !grant_matches(grant, seat, class, query_workspace) {
            continue;
        }
        let precedence = grant_precedence(grant);
        let replace = best
            .as_ref()
            .is_none_or(|(current, _)| precedence > *current);
        if replace {
            best = Some((
                precedence,
                EffectiveEntry {
                    class: class.to_owned(),
                    workspace: query_workspace.to_owned(),
                    origin: grant.origin,
                    effect: grant.effect,
                    grant_id: Some(grant.id),
                    limit: grant.limit.clone(),
                    self_entered: is_self_entered(grant, roster),
                },
            ));
        }
    }

    best.map(|(_, entry)| entry)
}

pub(super) fn check_refusal<'a>(
    seat: &RosterSeat,
    class: &str,
    query_workspace: &str,
    spec: Option<&ClassSpec>,
    winner: Option<&EffectiveEntry>,
    holders: impl Iterator<Item = &'a RosterSeat>,
) -> Error {
    let names: Vec<_> = holders.map(|holder| holder.name.as_str()).collect();
    let default = spec.map_or("nobody", |spec| spec.default_holder.label());
    let holder = if names.is_empty() {
        format!("default holder: {default}")
    } else {
        format!("current holders: {}", names.join(", "))
    };
    let withhold = winner
        .filter(|entry| entry.effect == GrantEffect::Withhold)
        .and_then(|entry| entry.grant_id)
        .map_or_else(String::new, |id| format!("withheld by {id}; "));
    refused(
        "not_authorized",
        format!(
            "seat `{}` does not hold `{class}` for workspace `{query_workspace}` \
             ({withhold}{holder})",
            seat.name
        ),
    )
}

pub(super) fn format_change_mail(
    seat: &str,
    action: &str,
    grant: &GrantRecord,
    self_entered: bool,
    after: &EffectiveSeat,
) -> String {
    let mut lines = Vec::new();
    let mut header = format!(
        "Authority changed ({action} #{}): {} {} for {} on workspace {}",
        grant.id,
        grant.effect.as_str(),
        grant.class,
        grant.target,
        grant.workspace
    );
    if self_entered {
        header.push_str(" (self-entered)");
    }
    lines.push(header);
    if let Some(limit) = &grant.limit {
        lines.push(format!("Limit: {limit}"));
    }
    if after.settles.is_empty() {
        lines.push("You now settle: none.".to_owned());
    } else {
        let summary = after
            .settles
            .iter()
            .map(|entry| format!("{} ({})", entry.class, entry.workspace))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("You now settle: {summary}."));
    }
    lines.push(format!(
        "Run `formation grant list --seat {seat}` for details."
    ));
    lines.join("\n")
}

fn holds_catalog_default(
    seat: &RosterSeat,
    holder: DefaultHolder,
    query_workspace: &str,
    roster: &[RosterSeat],
) -> bool {
    match holder {
        DefaultHolder::Admiral | DefaultHolder::Nobody => false,
        DefaultHolder::Role(role) => {
            seat.role == role.as_str() && seat_default_workspace_matches(seat, query_workspace)
        }
        DefaultHolder::SergeantOrCaptain => {
            if seat.role == Role::Sergeant.as_str() {
                seat_default_workspace_matches(seat, query_workspace)
            } else if seat.role == Role::Captain.as_str() {
                let Some(captain_ws) = seat.workspace.as_deref() else {
                    return false;
                };
                if query_workspace != captain_ws {
                    return false;
                }
                let has_sergeant = roster.iter().any(|other| {
                    other.role == Role::Sergeant.as_str()
                        && (other.workspace.as_deref() == Some(captain_ws)
                            || other.reports_to.as_deref() == Some(seat.name.as_str()))
                });
                !has_sergeant
            } else {
                false
            }
        }
    }
}

fn seat_default_workspace_matches(seat: &RosterSeat, query_workspace: &str) -> bool {
    match seat.workspace.as_deref() {
        Some(seat_ws) => query_workspace == seat_ws,
        None => true,
    }
}

fn grant_matches(
    grant: &GrantRecord,
    seat: &RosterSeat,
    class: &str,
    query_workspace: &str,
) -> bool {
    if grant.revoked_at.is_some() || grant.class != class {
        return false;
    }
    match grant.target_kind {
        GrantTargetKind::Seat => {
            grant.target == seat.name
                && (grant.workspace == ALL_WORKSPACES || grant.workspace == query_workspace)
        }
        GrantTargetKind::Rank => {
            if grant.target != seat.role {
                return false;
            }
            match seat.workspace.as_deref() {
                Some(seat_ws) => {
                    query_workspace == seat_ws
                        && (grant.workspace == ALL_WORKSPACES || grant.workspace == seat_ws)
                }
                None => grant.workspace == ALL_WORKSPACES || grant.workspace == query_workspace,
            }
        }
    }
}

fn grant_precedence(grant: &GrantRecord) -> Precedence {
    Precedence {
        target_rank: match grant.target_kind {
            GrantTargetKind::Rank => 1,
            GrantTargetKind::Seat => 2,
        },
        workspace_rank: if grant.workspace == ALL_WORKSPACES {
            1
        } else {
            2
        },
        origin_rank: match grant.origin {
            GrantOrigin::Catalog => 1,
            GrantOrigin::HouseFile => 2,
            GrantOrigin::Room => 3,
        },
        entered_at: grant.entered_at,
        entered_at_ns: grant.entered_at_ns,
        effect_rank: match grant.effect {
            GrantEffect::Grant => 1,
            GrantEffect::Withhold => 2,
        },
        id: grant.id,
    }
}
