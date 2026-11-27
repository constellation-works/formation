//! Integration tests for `formation grant` commands, precedence, mail and refusals.
#![allow(
    clippy::unwrap_used,
    reason = "integration tests panic on unexpected setup errors"
)]

use cw_testkit::TempRoot;
use formation_store::Store;
use formation_store::contract::{GrantEffect, GrantLifetime, GrantTargetKind, NewGrant};
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Output, Stdio};

fn run_bin(root: &TempRoot, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = root.command(env!("CARGO_BIN_EXE_formation"));
    cmd.args(args);
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.output().unwrap()
}

fn ok_json(root: &TempRoot, args: &[&str], env: &[(&str, &str)]) -> Value {
    let out = run_bin(root, args, env);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn err_code(root: &TempRoot, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = run_bin(root, args, env);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{args:?} expected exit 1, stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: Value = serde_json::from_slice(&out.stderr).unwrap();
    doc["code"].as_str().unwrap().to_owned()
}

fn apply_roster(root: &TempRoot, room: &str, roster: &Value) {
    let mut child = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(["roster", "apply", "--room", room])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(roster).unwrap())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "roster apply failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn standard_roster() -> Value {
    json!({
        "version": 2,
        "seats": [
            {
                "name": "0-chief",
                "role": "chief",
                "host": "claude-desktop"
            },
            {
                "name": "0-advisor",
                "role": "advisor",
                "host": "claude-desktop",
                "reports_to": "0-chief"
            },
            {
                "name": "orbit-capt",
                "role": "captain",
                "workspace": "orbit",
                "host": "codex-desktop",
                "reports_to": "0-chief"
            },
            {
                "name": "orbit-sgt",
                "role": "sergeant",
                "workspace": "orbit",
                "host": "headless",
                "reports_to": "orbit-capt"
            },
            {
                "name": "site-capt",
                "role": "captain",
                "workspace": "site",
                "host": "codex-desktop",
                "reports_to": "0-chief"
            }
        ],
        "routes": []
    })
}

fn setup_room() -> (TempRoot, String) {
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().join("room").to_str().unwrap().to_owned();
    for seat in ["0-chief", "orbit-capt", "site-capt"] {
        ok_json(&root, &["mail", "join", seat, "--room", &room], &[]);
    }
    apply_roster(&root, &room, &standard_roster());
    (root, room)
}

#[test]
fn add_check_and_revoke_direction_for_chief() {
    let (root, room) = setup_room();

    assert_eq!(
        err_code(
            &root,
            &[
                "grant",
                "check",
                "--room",
                &room,
                "--seat",
                "0-chief",
                "--class",
                "direction"
            ],
            &[]
        ),
        "not_authorized"
    );

    let added = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--for",
            "day",
            "--source",
            "advisor chat 2026-11-27 12:00",
        ],
        &[],
    );
    let grant_id = added["id"].as_i64().unwrap();
    assert_eq!(added["self_entered"], false);
    assert_eq!(added["entered_by"], Value::Null);
    assert_eq!(added["notified"], json!(["0-chief"]));

    let checked = ok_json(
        &root,
        &[
            "grant",
            "check",
            "--room",
            &room,
            "--seat",
            "0-chief",
            "--class",
            "direction",
        ],
        &[],
    );
    assert_eq!(checked["allowed"], true);
    assert_eq!(checked["from"], "grant");
    assert_eq!(checked["grant_id"], grant_id);

    let id_text = grant_id.to_string();
    let revoked = ok_json(&root, &["grant", "revoke", &id_text, "--room", &room], &[]);
    assert_eq!(revoked["revoked"], true);
    assert_eq!(revoked["notified"], json!(["0-chief"]));

    assert_eq!(
        err_code(
            &root,
            &[
                "grant",
                "check",
                "--room",
                &room,
                "--seat",
                "0-chief",
                "--class",
                "direction"
            ],
            &[]
        ),
        "not_authorized"
    );
    assert_eq!(
        err_code(&root, &["grant", "revoke", &id_text, "--room", &room], &[]),
        "grant_already_revoked"
    );
}

#[test]
fn refuses_ungrantable_unknown_class_and_missing_source() {
    let (root, room) = setup_room();

    for forbidden in ["grant", "credentials"] {
        assert_eq!(
            err_code(
                &root,
                &[
                    "grant",
                    "add",
                    "--room",
                    &room,
                    "--to",
                    "chief",
                    "--class",
                    forbidden,
                    "--workspace",
                    "all",
                    "--for",
                    "day",
                    "--source",
                    "chat",
                ],
                &[]
            ),
            "class_not_grantable"
        );
    }

    assert_eq!(
        err_code(
            &root,
            &[
                "grant",
                "add",
                "--room",
                &room,
                "--to",
                "chief",
                "--class",
                "not-a-class",
                "--workspace",
                "all",
                "--for",
                "day",
                "--source",
                "chat",
            ],
            &[]
        ),
        "unknown_class"
    );

    let missing_source = run_bin(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--for",
            "day",
        ],
        &[],
    );
    assert_eq!(missing_source.status.code(), Some(2));

    assert_eq!(
        err_code(
            &root,
            &[
                "grant",
                "add",
                "--room",
                &room,
                "--to",
                "chief",
                "--class",
                "direction",
                "--workspace",
                "all",
                "--for",
                "day",
                "--source",
                "   ",
            ],
            &[]
        ),
        "source_required"
    );

    assert_eq!(
        err_code(
            &root,
            &[
                "grant",
                "add",
                "--room",
                &room,
                "--to",
                "chief",
                "--class",
                "direction",
                "--workspace",
                "all",
                "--for",
                "standing",
                "--source",
                "chat",
            ],
            &[]
        ),
        "invalid_lifetime"
    );
}

#[test]
fn withhold_removes_land_for_one_workspace_captain_only() {
    let (root, room) = setup_room();

    ok_json(
        &root,
        &[
            "grant",
            "withhold",
            "--room",
            &room,
            "--to",
            "captain",
            "--class",
            "land",
            "--workspace",
            "site",
            "--for",
            "day",
            "--source",
            "advisor chat 12:30",
        ],
        &[],
    );

    assert_eq!(
        err_code(
            &root,
            &[
                "grant",
                "check",
                "--room",
                &room,
                "--seat",
                "site-capt",
                "--class",
                "land"
            ],
            &[]
        ),
        "not_authorized"
    );

    let orbit_check = ok_json(
        &root,
        &[
            "grant",
            "check",
            "--room",
            &room,
            "--seat",
            "orbit-capt",
            "--class",
            "land",
        ],
        &[],
    );
    assert_eq!(orbit_check["allowed"], true);
    assert_eq!(orbit_check["from"], "catalog");

    let site_list = ok_json(
        &root,
        &[
            "grant",
            "list",
            "--room",
            &room,
            "--seat",
            "site-capt",
            "--json",
        ],
        &[],
    );
    let site_settles: Vec<&str> = site_list["settles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["class"].as_str().unwrap())
        .collect();
    assert_eq!(site_settles, ["repair", "ship"]);
    assert_eq!(site_list["withheld"][0]["class"], "land");
    assert_eq!(site_list["withheld"][0]["from"], "withhold");

    let orbit_list = ok_json(
        &root,
        &[
            "grant",
            "list",
            "--room",
            &room,
            "--seat",
            "orbit-capt",
            "--json",
        ],
        &[],
    );
    let orbit_settles: Vec<&str> = orbit_list["settles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["class"].as_str().unwrap())
        .collect();
    assert_eq!(orbit_settles, ["ship", "land"]);
    assert!(orbit_list["withheld"].as_array().unwrap().is_empty());
}

#[test]
fn precedence_and_same_timestamp_tie_in_list_and_check() {
    let (root, room) = setup_room();

    // Rank grant on `all` workspaces, then rank withhold on `site`, then seat grant on `all`.
    ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--to",
            "captain",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--for",
            "day",
            "--source",
            "chat 1",
        ],
        &[],
    );
    ok_json(
        &root,
        &[
            "grant",
            "withhold",
            "--room",
            &room,
            "--to",
            "captain",
            "--class",
            "direction",
            "--workspace",
            "site",
            "--for",
            "day",
            "--source",
            "chat 2",
        ],
        &[],
    );
    assert_eq!(
        err_code(
            &root,
            &[
                "grant",
                "check",
                "--room",
                &room,
                "--seat",
                "site-capt",
                "--class",
                "direction"
            ],
            &[]
        ),
        "not_authorized"
    );
    ok_json(
        &root,
        &[
            "grant",
            "check",
            "--room",
            &room,
            "--seat",
            "orbit-capt",
            "--class",
            "direction",
        ],
        &[],
    );

    // Seat beats rank even when rank names the specific workspace and seat says `all`.
    let seat_grant = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--to",
            "site-capt",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--for",
            "day",
            "--source",
            "chat 3",
        ],
        &[],
    );
    let checked = ok_json(
        &root,
        &[
            "grant",
            "check",
            "--room",
            &room,
            "--seat",
            "site-capt",
            "--class",
            "direction",
        ],
        &[],
    );
    assert_eq!(checked["grant_id"], seat_grant["id"]);

    // Seed an exact same-timestamp grant and withhold to verify tie-breaking through the CLI.
    let store = Store::open(std::path::Path::new(&room)).unwrap();
    store
        .write(|tx| {
            tx.grants().insert(&NewGrant {
                effect: GrantEffect::Withhold,
                target_kind: GrantTargetKind::Rank,
                target: "chief",
                class: "operator",
                workspace: "all",
                limit: None,
                lifetime: GrantLifetime::Day,
                source: "tie withhold",
                entered_by: None,
                entered_at: 9_999_999_999,
                entered_at_ns: 42,
            })?;
            tx.grants().insert(&NewGrant {
                effect: GrantEffect::Grant,
                target_kind: GrantTargetKind::Rank,
                target: "chief",
                class: "operator",
                workspace: "all",
                limit: None,
                lifetime: GrantLifetime::Day,
                source: "tie grant",
                entered_by: None,
                entered_at: 9_999_999_999,
                entered_at_ns: 42,
            })?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        err_code(
            &root,
            &[
                "grant", "check", "--room", &room, "--seat", "0-chief", "--class", "operator"
            ],
            &[]
        ),
        "not_authorized"
    );
}

#[test]
fn self_entered_flag_marks_own_seat_or_rank_in_text_and_json() {
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().join("room").to_str().unwrap().to_owned();
    ok_json(
        &root,
        &["mail", "join", "0-chief", "--room", &room],
        &[("CLAUDE_CODE_SESSION_ID", "sess-chief")],
    );
    ok_json(&root, &["mail", "join", "orbit-capt", "--room", &room], &[]);
    apply_roster(&root, &room, &standard_roster());

    // Entered from chief's bound session for rank `chief` -> self-entered.
    let rank_grant = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--for",
            "day",
            "--source",
            "chief chat",
        ],
        &[("CLAUDE_CODE_SESSION_ID", "sess-chief")],
    );
    assert_eq!(rank_grant["entered_by"], "0-chief");
    assert_eq!(rank_grant["self_entered"], true);

    // Entered by orbit-capt via `--as` for its own seat -> self-entered.
    let seat_grant = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--as",
            "orbit-capt",
            "--to",
            "orbit-capt",
            "--class",
            "operator",
            "--workspace",
            "orbit",
            "--for",
            "day",
            "--source",
            "captain chat",
        ],
        &[],
    );
    assert_eq!(seat_grant["self_entered"], true);

    // Entered by chief for captain -> not self-entered.
    let other_grant = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--to",
            "captain",
            "--class",
            "design",
            "--workspace",
            "orbit",
            "--for",
            "day",
            "--source",
            "chief chat",
        ],
        &[("CLAUDE_CODE_SESSION_ID", "sess-chief")],
    );
    assert_eq!(other_grant["self_entered"], false);

    let room_text = run_bin(&root, &["grant", "list", "--room", &room], &[]);
    assert!(room_text.status.success());
    let text = String::from_utf8(room_text.stdout).unwrap();
    assert_eq!(text.matches("self-entered").count(), 2);

    let seat_text = run_bin(
        &root,
        &["grant", "list", "--room", &room, "--seat", "0-chief"],
        &[],
    );
    assert!(seat_text.status.success());
    assert!(
        String::from_utf8(seat_text.stdout)
            .unwrap()
            .contains("self-entered")
    );
}

#[test]
fn mails_one_envelope_only_to_joined_seats_whose_effective_list_changed() {
    let (root, room) = setup_room();
    let inbox_count = |seat: &str| -> usize {
        let doc = ok_json(
            &root,
            &["mail", "inbox", "--room", &room, "--as", seat],
            &[],
        );
        doc["messages"].as_array().unwrap().len()
    };

    // 1. Add grant for `chief` -> only `0-chief` changes and gets 1 envelope.
    let added = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--for",
            "day",
            "--source",
            "chat 1",
        ],
        &[],
    );
    assert_eq!(inbox_count("0-chief"), 1);
    assert_eq!(inbox_count("orbit-capt"), 0);
    assert_eq!(inbox_count("site-capt"), 0);

    // 2. Withhold `land` for `captain` on `site` -> only `site-capt` gets 1 envelope.
    ok_json(
        &root,
        &[
            "grant",
            "withhold",
            "--room",
            &room,
            "--to",
            "captain",
            "--class",
            "land",
            "--workspace",
            "site",
            "--for",
            "day",
            "--source",
            "chat 2",
        ],
        &[],
    );
    assert_eq!(inbox_count("0-chief"), 1);
    assert_eq!(inbox_count("orbit-capt"), 0);
    assert_eq!(inbox_count("site-capt"), 1);

    // 3. Revoke chief's grant -> only `0-chief` gets 1 more envelope (total 2).
    let id_text = added["id"].as_i64().unwrap().to_string();
    ok_json(&root, &["grant", "revoke", &id_text, "--room", &room], &[]);
    assert_eq!(inbox_count("0-chief"), 2);
    assert_eq!(inbox_count("orbit-capt"), 0);
    assert_eq!(inbox_count("site-capt"), 1);

    // 4. Grant to an unjoined seat (`0-advisor`) -> no joined seat changes, 0 envelopes.
    let unjoined = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--to",
            "0-advisor",
            "--class",
            "design",
            "--workspace",
            "all",
            "--for",
            "day",
            "--source",
            "chat 3",
        ],
        &[],
    );
    assert_eq!(unjoined["notified"], json!([]));
    assert_eq!(inbox_count("0-chief"), 2);
    assert_eq!(inbox_count("orbit-capt"), 0);
    assert_eq!(inbox_count("site-capt"), 1);
}

#[test]
fn every_grant_command_refuses_room_without_applied_roster() {
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().join("room").to_str().unwrap().to_owned();
    ok_json(&root, &["mail", "join", "0-chief", "--room", &room], &[]);

    for args in [
        vec![
            "grant",
            "add",
            "--room",
            &room,
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--for",
            "day",
            "--source",
            "chat",
        ],
        vec![
            "grant",
            "withhold",
            "--room",
            &room,
            "--to",
            "captain",
            "--class",
            "land",
            "--workspace",
            "orbit",
            "--for",
            "day",
            "--source",
            "chat",
        ],
        vec!["grant", "revoke", "1", "--room", &room],
        vec!["grant", "list", "--room", &room],
        vec!["grant", "list", "--room", &room, "--seat", "0-chief"],
        vec![
            "grant",
            "check",
            "--room",
            &room,
            "--seat",
            "0-chief",
            "--class",
            "direction",
        ],
    ] {
        assert_eq!(err_code(&root, &args, &[]), "no_roster", "{args:?}");
    }
}
