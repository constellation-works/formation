//! `grant load`: replace the house entries of a room with the grants file's.

use super::house::{self, Entry, HouseFile};
use super::{
    Audience, Change, Outcome, catalog, notify_changed, require_joined_caller_tx,
    require_roster_tx, snapshot_effective,
};
use crate::composition::Caller;
use cw_base::Error;
use formation_store::Store;
use formation_store::contract::{GrantLifetime, GrantOrigin, GrantRecord, HouseClass, NewGrant};
use std::collections::HashSet;

/// What a `grant load` changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded {
    /// The file version that was read.
    pub version: u32,
    /// Classes the file adds to the catalog.
    pub classes: usize,
    /// Standing grants and withholds the file holds.
    pub grants: usize,
    /// Earlier house entries this load replaced.
    pub replaced: usize,
    /// Whether the file matched what the room already held, so nothing changed.
    pub unchanged: bool,
    /// Joined seats mailed because their effective list changed.
    pub notified: Vec<String>,
}

type Key<'a> = (
    &'static str,
    &'static str,
    &'a str,
    &'a str,
    &'a str,
    Option<&'a str>,
    &'a str,
);

pub(super) fn load(
    store: &Store,
    caller: &Caller,
    now_sec: i64,
    now_ns: i64,
    bytes: &[u8],
) -> Result<(Outcome, Vec<String>), Error> {
    let decoded = house::decode(bytes)?;
    store.write(|tx| {
        let roster = require_roster_tx(tx)?;
        let loaded_by = require_joined_caller_tx(tx, caller, &roster)?;
        let file = decoded.validate(&roster)?;

        let before_house = tx.grants().house_classes()?;
        let before_grants = tx.grants().active()?;
        let held = before_grants
            .iter()
            .filter(|grant| grant.origin == GrantOrigin::HouseFile)
            .count();
        let mut loaded = Loaded {
            version: house::VERSION,
            classes: file.classes.len(),
            grants: file.entries.len(),
            replaced: held,
            unchanged: false,
            notified: Vec::new(),
        };
        if unchanged(&file, &before_house, &before_grants) {
            loaded.replaced = 0;
            loaded.unchanged = true;
            return Ok((Outcome::Loaded(loaded), Vec::new()));
        }

        let joined: HashSet<String> = tx.seats().all_names()?.into_iter().collect();
        let before_classes = catalog::with_house(&before_house);
        let before = snapshot_effective(&roster, &joined, &before_grants, &before_classes);

        tx.grants()
            .revoke_house(Some(loaded_by.as_str()), now_sec)?;
        tx.grants().replace_house_classes(&file.classes)?;
        for entry in &file.entries {
            tx.grants().insert(&NewGrant {
                effect: entry.effect,
                target_kind: entry.target_kind,
                target: &entry.target,
                class: &entry.class,
                workspace: &entry.workspace,
                limit: entry.limit.as_deref(),
                lifetime: GrantLifetime::Standing,
                origin: GrantOrigin::HouseFile,
                source: &entry.source,
                entered_by: Some(loaded_by.as_str()),
                entered_at: now_sec,
                entered_at_ns: now_ns,
            })?;
        }

        let after_grants = tx.grants().active()?;
        let after_classes = catalog::with_house(&file.classes);
        let notified = notify_changed(
            tx,
            &Audience {
                roster: &roster,
                joined: &joined,
                classes: &after_classes,
            },
            &before,
            &after_grants,
            &Change::Load {
                entries: loaded.grants,
                classes: loaded.classes,
            },
            Some(loaded_by.as_str()),
            now_sec,
        )?;
        loaded.notified.clone_from(&notified);
        Ok((Outcome::Loaded(loaded), notified))
    })
}

/// Whether the room already holds exactly these classes and entries.
fn unchanged(file: &HouseFile, classes: &[HouseClass], active: &[GrantRecord]) -> bool {
    if file.classes != classes {
        return false;
    }
    let mut held: Vec<_> = active
        .iter()
        .filter(|grant| grant.origin == GrantOrigin::HouseFile)
        .map(record_key)
        .collect();
    let mut wanted: Vec<_> = file.entries.iter().map(entry_key).collect();
    held.sort();
    wanted.sort();
    held == wanted
}

fn record_key(grant: &GrantRecord) -> Key<'_> {
    (
        grant.effect.as_str(),
        grant.target_kind.as_str(),
        &grant.target,
        &grant.class,
        &grant.workspace,
        grant.limit.as_deref(),
        &grant.source,
    )
}

fn entry_key(entry: &Entry) -> Key<'_> {
    (
        entry.effect.as_str(),
        entry.target_kind.as_str(),
        &entry.target,
        &entry.class,
        &entry.workspace,
        entry.limit.as_deref(),
        &entry.source,
    )
}
