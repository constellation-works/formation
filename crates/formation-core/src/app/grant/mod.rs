//! Room-scoped authority grants, withholds, effective lists and checks.

pub mod catalog;

mod house;

mod load;

mod rule;

#[cfg(test)]
mod tests;

use crate::Env;
use crate::composition::Caller;
use crate::runtime;
use crate::runtime::App;
use crate::runtime::Ctx;
use crate::runtime::RoomAccess;
pub use catalog::{CATALOG, ClassSpec, DefaultHolder};
use cw_base::{Error, Kind};
pub use formation_store::contract::{GrantEffect, GrantLifetime, GrantOrigin, GrantTargetKind};
use formation_store::contract::{
    GrantOrigin as Origin, GrantRecord, HouseClass, NewGrant, RevokeStatus, rfc3339,
};
use formation_store::{RosterSeat, Snapshot, Store, Tx};
pub use load::Loaded;
pub use rule::{ALL_WORKSPACES, EffectiveEntry, EffectiveSeat};
use std::collections::HashSet;

/// A grant operation.
pub enum Command<'a> {
    /// Add a day grant to a rank or seat.
    Add {
        /// Rank or seat name.
        to: String,
        /// Authority class name.
        class: String,
        /// Workspace name or `all`.
        workspace: String,
        /// Optional condition in words.
        limit: Option<String>,
        /// Lifetime (`day`).
        lifetime: String,
        /// Where the admiral gave this direction.
        source: String,
        /// Queue a fixed wake for an idle Codex session.
        queue: &'a dyn Fn(&str) -> bool,
    },
    /// Withhold an authority class from a rank or seat.
    Withhold {
        /// Rank or seat name.
        to: String,
        /// Authority class name.
        class: String,
        /// Workspace name or `all`.
        workspace: String,
        /// Optional condition in words.
        limit: Option<String>,
        /// Lifetime (`day`).
        lifetime: String,
        /// Where the admiral gave this direction.
        source: String,
        /// Queue a fixed wake for an idle Codex session.
        queue: &'a dyn Fn(&str) -> bool,
    },
    /// Revoke an active grant or withhold by identifier.
    Revoke {
        /// Grant identifier.
        id: i64,
        /// Queue a fixed wake for an idle Codex session.
        queue: &'a dyn Fn(&str) -> bool,
    },
    /// Load the house grants file from its bytes, replacing the earlier house entries.
    Load {
        /// The grants file, a JSON document.
        bytes: Vec<u8>,
        /// Queue a fixed wake for an idle Codex session.
        queue: &'a dyn Fn(&str) -> bool,
    },
    /// List active room grants, or one seat's effective authority.
    List {
        /// Optional seat whose effective authority should be listed.
        seat: Option<String>,
    },
    /// Check whether a seat may settle a class in a workspace.
    Check {
        /// Seat name in the applied roster.
        seat: String,
        /// Authority class name.
        class: String,
        /// Optional workspace override.
        workspace: Option<String>,
    },
}

/// One active grant or withhold as listed by `grant list` and returned by `add`/`withhold`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantView {
    /// Room grant identifier.
    pub id: i64,
    /// Whether this record grants or withholds authority.
    pub effect: GrantEffect,
    /// Target rank or seat name.
    pub to: String,
    /// Whether `to` names a seat or a rank.
    pub target_kind: GrantTargetKind,
    /// Authority class identifier.
    pub class: String,
    /// Workspace identifier or `"all"`.
    pub workspace: String,
    /// Optional condition in words.
    pub limit: Option<String>,
    /// Lifetime (`day` or `standing`).
    pub lifetime: GrantLifetime,
    /// Operator direction source text.
    pub source: String,
    /// Seat that entered the grant; `None` only for a record from an earlier build.
    pub entered_by: Option<String>,
    /// When entered, as RFC 3339 UTC.
    pub entered_at: Option<String>,
    /// Whether the holder is the caller's own seat or rank.
    pub self_entered: bool,
    /// Where the entry lives: the room for a day entry, the house file for a standing one.
    pub origin: Origin,
}

/// Grant outcomes, independent of CLI presentation.
pub enum Outcome {
    /// A grant or withhold was added.
    Recorded {
        /// Stored record view.
        grant: GrantView,
        /// Joined seats mailed because their effective list changed.
        notified: Vec<String>,
    },
    /// A grant or withhold was revoked.
    Revoked {
        /// Revoked grant identifier.
        id: i64,
        /// Seat that revoked it; `None` only for a record from an earlier build.
        revoked_by: Option<String>,
        /// When revoked, as RFC 3339 UTC.
        revoked_at: Option<String>,
        /// Joined seats mailed because their effective list changed.
        notified: Vec<String>,
    },
    /// The house grants file was loaded.
    Loaded(Loaded),
    /// Active grants and withholds in the room.
    RoomList(Vec<GrantView>),
    /// Effective authority for one seat.
    SeatList(EffectiveSeat),
    /// A `grant check` succeeded.
    Checked {
        /// Checked seat name.
        seat: String,
        /// Checked seat role.
        role: String,
        /// Winning entry covering the class and workspace.
        entry: EffectiveEntry,
    },
}

struct MutationInput<'a> {
    effect: GrantEffect,
    to: &'a str,
    class: &'a str,
    workspace: &'a str,
    limit: Option<&'a str>,
    lifetime: &'a str,
    source: &'a str,
}

/// The seats a change can reach and the classes in force after it.
struct Audience<'a> {
    roster: &'a [RosterSeat],
    joined: &'a HashSet<String>,
    classes: &'a [ClassSpec<'a>],
}

/// What changed, for the mail each affected seat receives.
enum Change<'a> {
    /// One room grant or withhold was added or revoked.
    Grant {
        grant: &'a GrantRecord,
        action: &'a str,
        self_entered: bool,
    },
    /// The house grants file was loaded.
    Load { entries: usize, classes: usize },
}

struct GrantApp<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> App for GrantApp<'a> {
    type Command = Command<'a>;

    type Outcome = Outcome;

    fn room_access<'cmd>(_: &'cmd Command<'a>) -> RoomAccess<'cmd> {
        RoomAccess::OptionalSeat
    }

    fn audit(cmd: &Command<'a>) -> Option<&'static str> {
        match cmd {
            Command::Add { .. } => Some("grant add"),
            Command::Withhold { .. } => Some("grant withhold"),
            Command::Revoke { .. } => Some("grant revoke"),
            Command::Load { .. } => Some("grant load"),
            Command::List { .. } | Command::Check { .. } => None,
        }
    }

    fn run(ctx: &Ctx<'_>, cmd: Command<'a>) -> Result<Outcome, Error> {
        let mail = ctx.mail()?;
        let store = &mail.store;
        let now = (ctx.clock)();
        let now_sec = now.unix_timestamp();
        let now_ns = i64::from(now.nanosecond());

        match cmd {
            Command::Add {
                to,
                class,
                workspace,
                limit,
                lifetime,
                source,
                queue,
            } => {
                let (outcome, notified) = record_grant(
                    store,
                    &mail.caller,
                    now_sec,
                    now_ns,
                    &MutationInput {
                        effect: GrantEffect::Grant,
                        to: &to,
                        class: &class,
                        workspace: &workspace,
                        limit: limit.as_deref(),
                        lifetime: &lifetime,
                        source: &source,
                    },
                )?;
                wake_notified(ctx, &notified, queue);
                Ok(outcome)
            }
            Command::Withhold {
                to,
                class,
                workspace,
                limit,
                lifetime,
                source,
                queue,
            } => {
                let (outcome, notified) = record_grant(
                    store,
                    &mail.caller,
                    now_sec,
                    now_ns,
                    &MutationInput {
                        effect: GrantEffect::Withhold,
                        to: &to,
                        class: &class,
                        workspace: &workspace,
                        limit: limit.as_deref(),
                        lifetime: &lifetime,
                        source: &source,
                    },
                )?;
                wake_notified(ctx, &notified, queue);
                Ok(outcome)
            }
            Command::Revoke { id, queue } => {
                let (outcome, notified) = revoke_grant(store, &mail.caller, now_sec, id)?;
                wake_notified(ctx, &notified, queue);
                Ok(outcome)
            }
            Command::Load { bytes, queue } => {
                let (outcome, notified) = load::load(store, &mail.caller, now_sec, now_ns, &bytes)?;
                wake_notified(ctx, &notified, queue);
                Ok(outcome)
            }
            Command::List { seat } => list_grants(store, &mail.caller, seat.as_deref()),
            Command::Check {
                seat,
                class,
                workspace,
            } => check_grant(store, &mail.caller, &seat, &class, workspace.as_deref()),
        }
    }

    fn target(out: &Outcome) -> Option<String> {
        match out {
            Outcome::Recorded { grant, .. } => Some(grant.id.to_string()),
            Outcome::Revoked { id, .. } => Some(id.to_string()),
            Outcome::Loaded(_)
            | Outcome::RoomList(_)
            | Outcome::SeatList(_)
            | Outcome::Checked { .. } => None,
        }
    }
}

/// Run a grant operation and hand its typed outcome to the CLI surface.
pub fn exec<R>(env: &Env, cmd: Command<'_>, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<GrantApp<'_>, R>(env, cmd, finish)
}

fn record_grant(
    store: &Store,
    caller: &Caller,
    now_sec: i64,
    now_ns: i64,
    input: &MutationInput<'_>,
) -> Result<(Outcome, Vec<String>), Error> {
    store.write(|tx| {
        let roster = require_roster_tx(tx)?;
        let entered_by = require_joined_caller_tx(tx, caller, &roster)?;

        let house = tx.grants().house_classes()?;
        let classes = catalog::with_house(&house);
        rule::validate_class_for_mutation(input.class, &classes)?;
        let workspace = rule::validate_workspace(input.workspace)?;
        let lifetime = rule::validate_lifetime(input.lifetime)?;
        let source = rule::validate_source(input.source)?;
        let limit = rule::validate_limit(input.limit)?;
        let (target_kind, target) = rule::resolve_holder(input.to, &roster)?;

        let joined: HashSet<String> = tx.seats().all_names()?.into_iter().collect();
        let before_grants = tx.grants().active()?;
        let before_map = snapshot_effective(&roster, &joined, &before_grants, &classes);

        let inserted = tx.grants().insert(&NewGrant {
            effect: input.effect,
            target_kind,
            target: &target,
            class: input.class,
            workspace,
            limit,
            lifetime,
            origin: Origin::Room,
            source,
            entered_by: Some(entered_by.as_str()),
            entered_at: now_sec,
            entered_at_ns: now_ns,
        })?;

        let after_grants = tx.grants().active()?;
        let self_entered = rule::is_self_entered(&inserted, &roster);
        let action = match input.effect {
            GrantEffect::Grant => "added",
            GrantEffect::Withhold => "withheld",
        };
        let notified = notify_changed(
            tx,
            &Audience {
                roster: &roster,
                joined: &joined,
                classes: &classes,
            },
            &before_map,
            &after_grants,
            &Change::Grant {
                grant: &inserted,
                action,
                self_entered,
            },
            Some(entered_by.as_str()),
            now_sec,
        )?;

        Ok((
            Outcome::Recorded {
                grant: grant_view(inserted, self_entered),
                notified: notified.clone(),
            },
            notified,
        ))
    })
}

fn revoke_grant(
    store: &Store,
    caller: &Caller,
    now_sec: i64,
    id: i64,
) -> Result<(Outcome, Vec<String>), Error> {
    store.write(|tx| {
        let roster = require_roster_tx(tx)?;
        let revoked_by = require_joined_caller_tx(tx, caller, &roster)?;

        let house = tx.grants().house_classes()?;
        let classes = catalog::with_house(&house);
        let joined: HashSet<String> = tx.seats().all_names()?.into_iter().collect();
        let before_grants = tx.grants().active()?;
        if before_grants
            .iter()
            .any(|grant| grant.id == id && grant.origin == Origin::HouseFile)
        {
            return Err(rule::refused(
                "grant_standing",
                format!(
                    "grant `{id}` is a house-file entry; change the grants file and load it again"
                ),
            ));
        }
        let before_map = snapshot_effective(&roster, &joined, &before_grants, &classes);

        let revoked = match tx.grants().revoke(id, Some(revoked_by.as_str()), now_sec)? {
            RevokeStatus::Revoked(record) => *record,
            RevokeStatus::NotFound => {
                return Err(rule::refused(
                    "unknown_grant",
                    format!("no grant `{id}` in this room; run `formation grant list` to see ids"),
                ));
            }
            RevokeStatus::AlreadyRevoked => {
                return Err(rule::refused(
                    "grant_already_revoked",
                    format!("grant `{id}` is already revoked"),
                ));
            }
        };

        let after_grants = tx.grants().active()?;
        let self_entered = rule::is_self_entered(&revoked, &roster);
        let notified = notify_changed(
            tx,
            &Audience {
                roster: &roster,
                joined: &joined,
                classes: &classes,
            },
            &before_map,
            &after_grants,
            &Change::Grant {
                grant: &revoked,
                action: "revoked",
                self_entered,
            },
            Some(revoked_by.as_str()),
            now_sec,
        )?;

        Ok((
            Outcome::Revoked {
                id,
                revoked_by: Some(revoked_by),
                revoked_at: rfc3339(now_sec),
                notified: notified.clone(),
            },
            notified,
        ))
    })
}

fn list_grants(store: &Store, caller: &Caller, seat: Option<&str>) -> Result<Outcome, Error> {
    let snapshot = store.read()?;
    let roster = require_roster_snapshot(&snapshot)?;
    validate_caller_snapshot(&snapshot, caller, &roster)?;
    let grants = snapshot.grants().active()?;

    if let Some(seat_name) = seat {
        let roster_seat = find_roster_seat(&roster, seat_name)?;
        let house = snapshot.grants().house_classes()?;
        let classes = catalog::with_house(&house);
        let effective = rule::effective_for_seat(roster_seat, &roster, &grants, &classes);
        Ok(Outcome::SeatList(effective))
    } else {
        let views = grants
            .into_iter()
            .map(|record| {
                let self_entered = rule::is_self_entered(&record, &roster);
                grant_view(record, self_entered)
            })
            .collect();
        Ok(Outcome::RoomList(views))
    }
}

fn check_grant(
    store: &Store,
    caller: &Caller,
    seat: &str,
    class: &str,
    workspace: Option<&str>,
) -> Result<Outcome, Error> {
    let snapshot = store.read()?;
    let roster = require_roster_snapshot(&snapshot)?;
    validate_caller_snapshot(&snapshot, caller, &roster)?;
    let grants = snapshot.grants().active()?;

    let roster_seat = find_roster_seat(&roster, seat)?;
    let workspace = match workspace {
        Some(value) => rule::validate_workspace(value)?,
        None => roster_seat.workspace.as_deref().unwrap_or(ALL_WORKSPACES),
    };
    let house = snapshot.grants().house_classes()?;
    let entry = check_authority(roster_seat, class, workspace, &roster, &grants, &house)?;
    Ok(Outcome::Checked {
        seat: roster_seat.name.clone(),
        role: roster_seat.role.clone(),
        entry,
    })
}

pub(super) fn check_authority(
    seat: &RosterSeat,
    class: &str,
    workspace: &str,
    roster: &[RosterSeat],
    grants: &[GrantRecord],
    house: &[HouseClass],
) -> Result<EffectiveEntry, Error> {
    let classes = catalog::with_house(house);
    let spec = rule::validate_class_for_check(class, &classes, grants)?;
    let winner = rule::evaluate_one(seat, class, workspace, roster, grants, &classes);
    match winner {
        Some(entry) if entry.effect == GrantEffect::Grant => Ok(entry),
        other => {
            let holders = roster.iter().filter(|candidate| {
                rule::evaluate_one(candidate, class, workspace, roster, grants, &classes)
                    .is_some_and(|entry| entry.effect == GrantEffect::Grant)
            });
            Err(rule::check_refusal(
                seat,
                class,
                workspace,
                spec,
                other.as_ref(),
                holders,
            ))
        }
    }
}

pub(super) fn validate_class(
    class: &str,
    grants: &[GrantRecord],
    house: &[HouseClass],
) -> Result<(), Error> {
    let classes = catalog::with_house(house);
    rule::validate_class_for_check(class, &classes, grants).map(|_| ())
}

pub(super) fn require_roster_tx(tx: &Tx<'_>) -> Result<Vec<RosterSeat>, Error> {
    let seats = tx.roster().seats()?;
    if seats.is_empty() {
        return Err(no_roster());
    }
    Ok(seats)
}

pub(super) fn require_roster_snapshot(snapshot: &Snapshot) -> Result<Vec<RosterSeat>, Error> {
    let seats = snapshot.roster().seats()?;
    if seats.is_empty() {
        return Err(no_roster());
    }
    Ok(seats)
}

fn no_roster() -> Error {
    rule::refused(
        "no_roster",
        "no roster applied to this room; run `formation roster apply --room <room>` first",
    )
}

pub(super) fn find_roster_seat<'a>(
    roster: &'a [RosterSeat],
    name: &str,
) -> Result<&'a RosterSeat, Error> {
    roster.iter().find(|seat| seat.name == name).ok_or_else(|| {
        rule::refused(
            "unknown_seat",
            format!("seat `{name}` is not in the applied roster"),
        )
    })
}

/// Resolve the joined seat that enters a grant change.
///
/// A missing caller is a missing identity, not an admiral: a grant change
/// needs a seat joined to this room until a verified terminal caller exists.
fn require_joined_caller_tx(
    tx: &Tx<'_>,
    caller: &Caller,
    roster: &[RosterSeat],
) -> Result<String, Error> {
    match caller {
        Caller::Explicit(name) => {
            if tx.seats().seen(name)?.is_some() {
                Ok(name.clone())
            } else if roster.iter().any(|seat| seat.name == *name) {
                Err(rule::refused(
                    "seat_required",
                    format!(
                        "seat `{name}` has not joined this room; run `formation mail join` first"
                    ),
                ))
            } else {
                Err(rule::refused(
                    "unknown_seat",
                    format!("seat `{name}` is not in the applied roster or joined to this room"),
                ))
            }
        }
        Caller::Session(session) => tx.seats().by_session(session)?.ok_or_else(seat_required),
        Caller::None | Caller::Admiral => Err(seat_required()),
        Caller::Join { .. } => Err(Error::new(
            Kind::Internal,
            "missing_context",
            std::io::Error::other("grant caller context unavailable"),
        )),
    }
}

fn seat_required() -> Error {
    rule::refused(
        "seat_required",
        "pass --as <seat> or join this room from your session before changing a grant",
    )
}

fn validate_caller_snapshot(
    snapshot: &Snapshot,
    caller: &Caller,
    roster: &[RosterSeat],
) -> Result<(), Error> {
    match caller {
        Caller::Explicit(name) => {
            if roster.iter().any(|seat| seat.name == *name) || snapshot.seats().contains(name)? {
                Ok(())
            } else {
                Err(rule::refused(
                    "unknown_seat",
                    format!("seat `{name}` is not in the applied roster or joined to this room"),
                ))
            }
        }
        Caller::Session(_) | Caller::None | Caller::Admiral => Ok(()),
        Caller::Join { .. } => Err(Error::new(
            Kind::Internal,
            "missing_context",
            std::io::Error::other("grant caller context unavailable"),
        )),
    }
}

fn snapshot_effective(
    roster: &[RosterSeat],
    joined: &HashSet<String>,
    grants: &[GrantRecord],
    classes: &[ClassSpec<'_>],
) -> Vec<(String, EffectiveSeat)> {
    roster
        .iter()
        .filter(|seat| joined.contains(&seat.name))
        .map(|seat| {
            (
                seat.name.clone(),
                rule::effective_for_seat(seat, roster, grants, classes),
            )
        })
        .collect()
}

fn notify_changed(
    tx: &Tx<'_>,
    audience: &Audience<'_>,
    before: &[(String, EffectiveSeat)],
    after_grants: &[GrantRecord],
    change: &Change<'_>,
    actor_seat: Option<&str>,
    now_sec: i64,
) -> Result<Vec<String>, Error> {
    let sender = actor_seat.unwrap_or("formation");
    let mut next_seq = tx.messages().next_receipt()?;
    let mut notified = Vec::new();

    for (seat_name, before_effective) in before {
        if !audience.joined.contains(seat_name) {
            continue;
        }
        let Some(roster_seat) = audience.roster.iter().find(|seat| seat.name == *seat_name) else {
            continue;
        };
        let after_effective =
            rule::effective_for_seat(roster_seat, audience.roster, after_grants, audience.classes);
        if *before_effective == after_effective {
            continue;
        }

        let (body, label) = match change {
            Change::Grant {
                grant,
                action,
                self_entered,
            } => (
                rule::format_change_mail(seat_name, action, grant, *self_entered, &after_effective),
                format!("grant-{}-{action}", grant.id),
            ),
            Change::Load { entries, classes } => (
                rule::format_load_mail(seat_name, *entries, *classes, &after_effective),
                "grants-load".to_owned(),
            ),
        };
        let mut message_id = format!("{label}-{next_seq}");
        while tx.messages().prior_delivery(sender, &message_id)?.is_some() {
            next_seq += 1;
            message_id = format!("{label}-{next_seq}");
        }
        next_seq += 1;

        tx.messages()
            .insert(sender, seat_name, &message_id, &body, now_sec)?;
        notified.push(seat_name.clone());
    }

    Ok(notified)
}

fn wake_notified(ctx: &Ctx<'_>, notified: &[String], queue: &dyn Fn(&str) -> bool) {
    for seat in notified {
        super::wake::after_commit(ctx, seat, queue);
    }
}

fn grant_view(record: GrantRecord, self_entered: bool) -> GrantView {
    GrantView {
        id: record.id,
        effect: record.effect,
        to: record.target,
        target_kind: record.target_kind,
        class: record.class,
        workspace: record.workspace,
        limit: record.limit,
        lifetime: record.lifetime,
        source: record.source,
        entered_by: record.entered_by,
        entered_at: rfc3339(record.entered_at),
        self_entered,
        origin: record.origin,
    }
}
