//! Room-scoped authority grants, withholds, effective lists and checks.

pub mod catalog;

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
use formation_store::contract::{GrantRecord, NewGrant, RevokeStatus, rfc3339};
use formation_store::{RosterSeat, Snapshot, Store, Tx};
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
    /// Seat that entered the grant, or `None` outside a session.
    pub entered_by: Option<String>,
    /// When entered, as RFC 3339 UTC.
    pub entered_at: Option<String>,
    /// Whether the holder is the caller's own seat or rank.
    pub self_entered: bool,
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
        /// Seat that revoked it, or `None` outside a session.
        revoked_by: Option<String>,
        /// When revoked, as RFC 3339 UTC.
        revoked_at: Option<String>,
        /// Joined seats mailed because their effective list changed.
        notified: Vec<String>,
    },
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
            Outcome::RoomList(_) | Outcome::SeatList(_) | Outcome::Checked { .. } => None,
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
        let entered_by = resolve_caller_tx(tx, caller, &roster)?;

        rule::validate_class_for_mutation(input.class, CATALOG)?;
        let workspace = rule::validate_workspace(input.workspace)?;
        let lifetime = rule::validate_lifetime(input.lifetime)?;
        let source = rule::validate_source(input.source)?;
        let limit = rule::validate_limit(input.limit)?;
        let (target_kind, target) = rule::resolve_holder(input.to, &roster)?;

        let joined: HashSet<String> = tx.seats().all_names()?.into_iter().collect();
        let before_grants = tx.grants().active()?;
        let before_map = snapshot_effective(&roster, &joined, &before_grants);

        let inserted = tx.grants().insert(&NewGrant {
            effect: input.effect,
            target_kind,
            target: &target,
            class: input.class,
            workspace,
            limit,
            lifetime,
            source,
            entered_by: entered_by.as_deref(),
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
            &roster,
            &joined,
            &before_map,
            &after_grants,
            &inserted,
            action,
            self_entered,
            entered_by.as_deref(),
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
        let revoked_by = resolve_caller_tx(tx, caller, &roster)?;

        let joined: HashSet<String> = tx.seats().all_names()?.into_iter().collect();
        let before_grants = tx.grants().active()?;
        let before_map = snapshot_effective(&roster, &joined, &before_grants);

        let revoked = match tx.grants().revoke(id, revoked_by.as_deref(), now_sec)? {
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
            &roster,
            &joined,
            &before_map,
            &after_grants,
            &revoked,
            "revoked",
            self_entered,
            revoked_by.as_deref(),
            now_sec,
        )?;

        Ok((
            Outcome::Revoked {
                id,
                revoked_by,
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
        let effective = rule::effective_for_seat(roster_seat, &roster, &grants, CATALOG);
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
    let spec = rule::validate_class_for_check(class, CATALOG, &grants)?;
    let query_workspace = match workspace {
        Some(value) => rule::validate_workspace(value)?,
        None => roster_seat.workspace.as_deref().unwrap_or(ALL_WORKSPACES),
    };

    let winner = rule::evaluate_one(
        roster_seat,
        class,
        query_workspace,
        &roster,
        &grants,
        CATALOG,
    );
    match winner {
        Some(entry) if entry.effect == GrantEffect::Grant => Ok(Outcome::Checked {
            seat: roster_seat.name.clone(),
            role: roster_seat.role.clone(),
            entry,
        }),
        other => Err(rule::check_refusal(
            roster_seat,
            class,
            query_workspace,
            spec,
            other.as_ref(),
        )),
    }
}

fn require_roster_tx(tx: &Tx<'_>) -> Result<Vec<RosterSeat>, Error> {
    let seats = tx.roster().seats()?;
    if seats.is_empty() {
        return Err(no_roster());
    }
    Ok(seats)
}

fn require_roster_snapshot(snapshot: &Snapshot) -> Result<Vec<RosterSeat>, Error> {
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

fn find_roster_seat<'a>(roster: &'a [RosterSeat], name: &str) -> Result<&'a RosterSeat, Error> {
    roster.iter().find(|seat| seat.name == name).ok_or_else(|| {
        rule::refused(
            "unknown_seat",
            format!("seat `{name}` is not in the applied roster"),
        )
    })
}

fn resolve_caller_tx(
    tx: &Tx<'_>,
    caller: &Caller,
    roster: &[RosterSeat],
) -> Result<Option<String>, Error> {
    match caller {
        Caller::Explicit(name) => {
            if roster.iter().any(|seat| seat.name == *name) || tx.seats().seen(name)?.is_some() {
                Ok(Some(name.clone()))
            } else {
                Err(rule::refused(
                    "unknown_seat",
                    format!("seat `{name}` is not in the applied roster or joined to this room"),
                ))
            }
        }
        Caller::Session(session) => tx.seats().by_session(session),
        Caller::None => Ok(None),
        Caller::Join { .. } => Err(Error::new(
            Kind::Internal,
            "missing_context",
            std::io::Error::other("grant caller context unavailable"),
        )),
    }
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
        Caller::Session(_) | Caller::None => Ok(()),
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
) -> Vec<(String, EffectiveSeat)> {
    roster
        .iter()
        .filter(|seat| joined.contains(&seat.name))
        .map(|seat| {
            (
                seat.name.clone(),
                rule::effective_for_seat(seat, roster, grants, CATALOG),
            )
        })
        .collect()
}

#[allow(
    clippy::too_many_arguments,
    reason = "transactional notification needs before/after snapshots and grant context"
)]
fn notify_changed(
    tx: &Tx<'_>,
    roster: &[RosterSeat],
    joined: &HashSet<String>,
    before: &[(String, EffectiveSeat)],
    after_grants: &[GrantRecord],
    grant: &GrantRecord,
    action: &str,
    self_entered: bool,
    actor_seat: Option<&str>,
    now_sec: i64,
) -> Result<Vec<String>, Error> {
    let sender = actor_seat.unwrap_or("formation");
    let mut next_seq = tx.messages().next_receipt()?;
    let mut notified = Vec::new();

    for (seat_name, before_effective) in before {
        if !joined.contains(seat_name) {
            continue;
        }
        let Some(roster_seat) = roster.iter().find(|seat| seat.name == *seat_name) else {
            continue;
        };
        let after_effective = rule::effective_for_seat(roster_seat, roster, after_grants, CATALOG);
        if *before_effective == after_effective {
            continue;
        }

        let body =
            rule::format_change_mail(seat_name, action, grant, self_entered, &after_effective);
        let mut message_id = format!("grant-{}-{action}-{next_seq}", grant.id);
        while tx.messages().prior_delivery(sender, &message_id)?.is_some() {
            next_seq += 1;
            message_id = format!("grant-{}-{action}-{next_seq}", grant.id);
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
    }
}
