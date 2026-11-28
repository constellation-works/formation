//! Unit tests for the class catalog, precedence rules and self-entered detection.
#![allow(
    clippy::unwrap_used,
    reason = "unit tests panic on unexpected setup errors"
)]

use super::catalog::CATALOG;
use super::rule::{effective_for_seat, evaluate_one, is_self_entered};
use formation_store::RosterSeat;
use formation_store::contract::{
    GrantEffect, GrantLifetime, GrantOrigin, GrantRecord, GrantTargetKind,
};

fn seat(name: &str, role: &str, workspace: Option<&str>, reports_to: Option<&str>) -> RosterSeat {
    RosterSeat {
        name: name.to_owned(),
        role: role.to_owned(),
        workspace: workspace.map(str::to_owned),
        model: None,
        effort: None,
        record: None,
        reports_to: reports_to.map(str::to_owned),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "test fixture constructs full grant record with explicit fields"
)]
fn make_grant(
    id: i64,
    effect: GrantEffect,
    target_kind: GrantTargetKind,
    target: &str,
    class: &str,
    workspace: &str,
    origin: GrantOrigin,
    entered_by: Option<&str>,
    entered_at: i64,
    entered_at_ns: i64,
) -> GrantRecord {
    GrantRecord {
        id,
        effect,
        target_kind,
        target: target.to_owned(),
        class: class.to_owned(),
        workspace: workspace.to_owned(),
        limit: None,
        lifetime: GrantLifetime::Day,
        origin,
        source: "test source".to_owned(),
        entered_by: entered_by.map(str::to_owned),
        entered_at,
        entered_at_ns,
        revoked_at: None,
        revoked_by: None,
    }
}

#[test]
fn catalog_matches_scope_and_marks_ungrantable_classes() {
    let names: Vec<&str> = CATALOG.iter().map(|spec| spec.name).collect();
    assert_eq!(
        names,
        [
            "repair",
            "ship",
            "land",
            "staffing",
            "design",
            "breaking",
            "direction",
            "release",
            "operator",
            "override",
            "credentials",
            "grant",
        ]
    );
    for spec in CATALOG {
        let expected_grantable = !matches!(spec.name, "grant" | "credentials");
        assert_eq!(spec.grantable, expected_grantable, "{}", spec.name);
    }
}

#[test]
fn repair_default_moves_to_sergeant_only_where_staffed() {
    let roster = vec![
        seat("0-chief", "chief", None, None),
        seat("orbit-capt", "captain", Some("orbit"), Some("0-chief")),
        seat("orbit-sgt", "sergeant", Some("orbit"), Some("orbit-capt")),
        seat("site-capt", "captain", Some("site"), Some("0-chief")),
    ];

    let orbit_capt = effective_for_seat(&roster[1], &roster, &[], CATALOG);
    let orbit_sgt = effective_for_seat(&roster[2], &roster, &[], CATALOG);
    let site_capt = effective_for_seat(&roster[3], &roster, &[], CATALOG);

    fn classes(eff: &super::EffectiveSeat) -> Vec<&str> {
        eff.settles
            .iter()
            .map(|entry| entry.class.as_str())
            .collect()
    }
    assert_eq!(classes(&orbit_capt), ["ship", "land"]);
    assert_eq!(classes(&orbit_sgt), ["repair"]);
    assert_eq!(classes(&site_capt), ["repair", "ship", "land"]);
}

#[test]
fn precedence_follows_seat_workspace_layer_time_and_withhold_tie() {
    let roster = vec![
        seat("0-chief", "chief", None, None),
        seat("site-capt", "captain", Some("site"), Some("0-chief")),
    ];
    let site_capt = &roster[1];

    // 1. Room over house file over catalog.
    let house_withhold = make_grant(
        1,
        GrantEffect::Withhold,
        GrantTargetKind::Rank,
        "captain",
        "land",
        "all",
        GrantOrigin::HouseFile,
        None,
        50,
        0,
    );
    let room_grant = make_grant(
        2,
        GrantEffect::Grant,
        GrantTargetKind::Rank,
        "captain",
        "land",
        "all",
        GrantOrigin::Room,
        None,
        40,
        0,
    );
    let winner = evaluate_one(
        site_capt,
        "land",
        "site",
        &roster,
        std::slice::from_ref(&house_withhold),
        CATALOG,
    )
    .unwrap();
    assert_eq!(winner.effect, GrantEffect::Withhold);
    let winner = evaluate_one(
        site_capt,
        "land",
        "site",
        &roster,
        &[house_withhold, room_grant],
        CATALOG,
    )
    .unwrap();
    assert_eq!(winner.effect, GrantEffect::Grant);

    // 2. Specific workspace over `all`.
    let rank_all_grant = make_grant(
        1,
        GrantEffect::Grant,
        GrantTargetKind::Rank,
        "captain",
        "direction",
        "all",
        GrantOrigin::Room,
        None,
        200,
        0,
    );
    let rank_site_withhold = make_grant(
        2,
        GrantEffect::Withhold,
        GrantTargetKind::Rank,
        "captain",
        "direction",
        "site",
        GrantOrigin::Room,
        None,
        100,
        0,
    );
    let winner = evaluate_one(
        site_capt,
        "direction",
        "site",
        &roster,
        &[rank_all_grant.clone(), rank_site_withhold.clone()],
        CATALOG,
    )
    .unwrap();
    assert_eq!(winner.effect, GrantEffect::Withhold);

    // 3. Seat over rank (even when rank is workspace-specific and seat is `all`).
    let seat_all_grant = make_grant(
        3,
        GrantEffect::Grant,
        GrantTargetKind::Seat,
        "site-capt",
        "direction",
        "all",
        GrantOrigin::Room,
        None,
        50,
        0,
    );
    let winner = evaluate_one(
        site_capt,
        "direction",
        "site",
        &roster,
        &[rank_site_withhold, seat_all_grant],
        CATALOG,
    )
    .unwrap();
    assert_eq!(winner.effect, GrantEffect::Grant);
    assert_eq!(winner.grant_id, Some(3));

    // 4. Later over earlier when equal on seat/workspace/layer.
    let earlier_withhold = make_grant(
        1,
        GrantEffect::Withhold,
        GrantTargetKind::Rank,
        "chief",
        "direction",
        "all",
        GrantOrigin::Room,
        None,
        100,
        10,
    );
    let later_grant = make_grant(
        2,
        GrantEffect::Grant,
        GrantTargetKind::Rank,
        "chief",
        "direction",
        "all",
        GrantOrigin::Room,
        None,
        100,
        20,
    );
    let winner = evaluate_one(
        &roster[0],
        "direction",
        "all",
        &roster,
        &[earlier_withhold, later_grant],
        CATALOG,
    )
    .unwrap();
    assert_eq!(winner.effect, GrantEffect::Grant);

    // 5. Withhold over grant entered at the exact same time, regardless of id order.
    let same_time_withhold = make_grant(
        1,
        GrantEffect::Withhold,
        GrantTargetKind::Rank,
        "chief",
        "direction",
        "all",
        GrantOrigin::Room,
        None,
        100,
        500,
    );
    let same_time_grant = make_grant(
        2,
        GrantEffect::Grant,
        GrantTargetKind::Rank,
        "chief",
        "direction",
        "all",
        GrantOrigin::Room,
        None,
        100,
        500,
    );
    let winner = evaluate_one(
        &roster[0],
        "direction",
        "all",
        &roster,
        &[same_time_withhold, same_time_grant],
        CATALOG,
    )
    .unwrap();
    assert_eq!(winner.effect, GrantEffect::Withhold);
    assert_eq!(winner.grant_id, Some(1));
}

#[test]
fn self_entered_matches_caller_seat_or_rank() {
    let roster = vec![
        seat("0-chief", "chief", None, None),
        seat("site-capt", "captain", Some("site"), Some("0-chief")),
    ];

    let rank_self = make_grant(
        1,
        GrantEffect::Grant,
        GrantTargetKind::Rank,
        "chief",
        "direction",
        "all",
        GrantOrigin::Room,
        Some("0-chief"),
        100,
        0,
    );
    assert!(is_self_entered(&rank_self, &roster));

    let seat_self = make_grant(
        2,
        GrantEffect::Grant,
        GrantTargetKind::Seat,
        "site-capt",
        "direction",
        "site",
        GrantOrigin::Room,
        Some("site-capt"),
        100,
        0,
    );
    assert!(is_self_entered(&seat_self, &roster));

    let entered_for_other = make_grant(
        3,
        GrantEffect::Grant,
        GrantTargetKind::Rank,
        "captain",
        "direction",
        "site",
        GrantOrigin::Room,
        Some("0-chief"),
        100,
        0,
    );
    assert!(!is_self_entered(&entered_for_other, &roster));

    let outside_session = make_grant(
        4,
        GrantEffect::Grant,
        GrantTargetKind::Rank,
        "chief",
        "direction",
        "all",
        GrantOrigin::Room,
        None,
        100,
        0,
    );
    assert!(!is_self_entered(&outside_session, &roster));
}

#[test]
fn audit_metadata_holds_identifiers_only() {
    use super::{Command, GrantApp, GrantView, Outcome};
    use crate::runtime::App;

    let queue = |_: &str| false;
    let add_cmd = Command::Add {
        to: "chief".to_owned(),
        class: "direction".to_owned(),
        workspace: "all".to_owned(),
        limit: Some("secret limit words".to_owned()),
        lifetime: "day".to_owned(),
        source: "private admiral chat text".to_owned(),
        queue: &queue,
    };
    assert_eq!(GrantApp::audit(&add_cmd), Some("grant add"));

    let list_cmd = Command::List { seat: None };
    assert_eq!(GrantApp::audit(&list_cmd), None);

    let check_cmd = Command::Check {
        seat: "0-chief".to_owned(),
        class: "direction".to_owned(),
        workspace: None,
    };
    assert_eq!(GrantApp::audit(&check_cmd), None);

    let recorded = Outcome::Recorded {
        grant: GrantView {
            id: 42,
            effect: GrantEffect::Grant,
            to: "chief".to_owned(),
            target_kind: GrantTargetKind::Rank,
            class: "direction".to_owned(),
            workspace: "all".to_owned(),
            limit: Some("secret limit words".to_owned()),
            lifetime: GrantLifetime::Day,
            source: "private admiral chat text".to_owned(),
            entered_by: None,
            entered_at: None,
            self_entered: false,
        },
        notified: Vec::new(),
    };
    let target = GrantApp::target(&recorded).unwrap();
    assert_eq!(target, "42");
    assert!(!target.contains("secret"));
    assert!(!target.contains("private"));
}
