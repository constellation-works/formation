//! The house grants file: wire format, version check and validation.
//!
//! The file holds standing grants and withholds in the grant's own fields, plus the classes a
//! house adds to the catalog. It adds a class and never removes one, and it cannot make
//! `grant` or `credentials` grantable.

use super::catalog::{self, CATALOG, ClassSpec, DefaultHolder};
use super::rule;
use cw_base::{Error, Kind};
use formation_store::RosterSeat;
use formation_store::contract::{GrantEffect, GrantTargetKind, HouseClass, identifier};
use serde::Deserialize;
use std::collections::BTreeSet;

/// The only file version this build reads.
pub const VERSION: u32 = 1;

const MAX_CLASSES: usize = 64;

const MAX_ENTRIES: usize = 256;

const MAX_NAME_LEN: usize = 96;

const MAX_TEXT_LEN: usize = 4096;

#[derive(Deserialize)]
struct Probe {
    version: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(default)]
    classes: Vec<WireClass>,
    #[serde(default)]
    remove_classes: Vec<String>,
    #[serde(default)]
    grants: Vec<WireGrant>,
    #[serde(rename = "version")]
    _version: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireClass {
    name: String,
    covers: String,
    holder: Option<String>,
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireGrant {
    effect: WireEffect,
    to: String,
    class: String,
    workspace: Option<String>,
    limit: Option<String>,
    source: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WireEffect {
    Grant,
    Withhold,
}

/// One validated standing grant or withhold, ready to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) effect: GrantEffect,
    pub(super) target_kind: GrantTargetKind,
    pub(super) target: String,
    pub(super) class: String,
    pub(super) workspace: String,
    pub(super) limit: Option<String>,
    pub(super) source: String,
}

/// A validated house file.
#[derive(Debug)]
pub(super) struct HouseFile {
    pub(super) classes: Vec<HouseClass>,
    pub(super) entries: Vec<Entry>,
}

/// A decoded but not yet validated house file.
pub(super) struct Decoded(Wire);

impl Decoded {
    /// Check every rule against the catalog and the applied roster.
    pub(super) fn validate(self, roster: &[RosterSeat]) -> Result<HouseFile, Error> {
        let wire = self.0;
        if !wire.remove_classes.is_empty() {
            return Err(rule::refused(
                "class_not_removable",
                "a house file may add a class and may not remove one",
            ));
        }
        if wire.classes.len() > MAX_CLASSES || wire.grants.len() > MAX_ENTRIES {
            return Err(rule::refused(
                "grants_file_too_large",
                format!(
                    "a grants file holds at most {MAX_CLASSES} classes and {MAX_ENTRIES} grants"
                ),
            ));
        }
        let classes = classes(wire.classes)?;
        let known = catalog::with_house(&classes);
        let entries = entries(wire.grants, &known, roster)?;
        Ok(HouseFile { classes, entries })
    }
}

/// Decode the bytes and refuse an unknown version or a malformed document.
pub(super) fn decode(bytes: &[u8]) -> Result<Decoded, Error> {
    let probe: Probe = serde_json::from_slice(bytes).map_err(invalid)?;
    match probe.version.as_ref().and_then(serde_json::Value::as_u64) {
        Some(version) if version == u64::from(VERSION) => {}
        _ => {
            let seen = probe
                .version
                .map_or_else(|| "missing".to_owned(), |value| value.to_string());
            return Err(unknown_version(&seen));
        }
    }
    serde_json::from_slice(bytes).map(Decoded).map_err(invalid)
}

fn classes(wire: Vec<WireClass>) -> Result<Vec<HouseClass>, Error> {
    let mut seen = BTreeSet::new();
    let mut classes = Vec::new();
    for class in wire {
        if class.name.len() > MAX_NAME_LEN || !identifier(&class.name) {
            return Err(rule::refused(
                "invalid_class",
                "invalid class name (use 1-96 ASCII letters, digits, hyphens or underscores)",
            ));
        }
        if CATALOG.iter().any(|spec| spec.name == class.name) || !seen.insert(class.name.clone()) {
            return Err(rule::refused(
                "class_exists",
                format!(
                    "class `{}` already exists; a house file adds classes and cannot change one",
                    class.name
                ),
            ));
        }
        let holder = class.holder.unwrap_or_else(|| "admiral".to_owned());
        if DefaultHolder::from_wire(&holder).is_none() {
            return Err(rule::refused(
                "invalid_class",
                format!(
                    "class `{}` names holder `{holder}`; use `admiral`, `nobody` or a rank",
                    class.name
                ),
            ));
        }
        let covers = text(&class.covers, "`covers`")?;
        let note = class
            .note
            .as_deref()
            .map(|note| text(note, "`note`"))
            .transpose()?;
        classes.push(HouseClass {
            name: class.name,
            covers,
            holder,
            note,
        });
    }
    classes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(classes)
}

fn entries(
    wire: Vec<WireGrant>,
    known: &[ClassSpec<'_>],
    roster: &[RosterSeat],
) -> Result<Vec<Entry>, Error> {
    let mut entries: Vec<Entry> = Vec::new();
    for grant in wire {
        rule::validate_class_for_mutation(&grant.class, known)?;
        let workspace = rule::validate_workspace(grant.workspace.as_deref().unwrap_or("all"))?;
        let limit = rule::validate_limit(grant.limit.as_deref())?;
        let source = rule::validate_source(&grant.source)?;
        let (target_kind, target) = rule::resolve_holder(&grant.to, roster)?;
        let entry = Entry {
            effect: match grant.effect {
                WireEffect::Grant => GrantEffect::Grant,
                WireEffect::Withhold => GrantEffect::Withhold,
            },
            target_kind,
            target,
            class: grant.class,
            workspace: workspace.to_owned(),
            limit: limit.map(str::to_owned),
            source: source.to_owned(),
        };
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn text(value: &str, field: &str) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_TEXT_LEN {
        return Err(rule::refused(
            "invalid_class",
            format!("{field} must hold 1-{MAX_TEXT_LEN} bytes"),
        ));
    }
    Ok(trimmed.to_owned())
}

fn invalid(error: serde_json::Error) -> Error {
    Error::new(Kind::Usage, "invalid_grants_file", error)
}

fn unknown_version(seen: &str) -> Error {
    rule::refused(
        "unknown_grants_version",
        format!("grants file version {seen} is not supported; this build reads version {VERSION}"),
    )
}
