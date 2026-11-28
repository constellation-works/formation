//! Integration tests for `formation grant load`: the house grants file in a room.
#![allow(
    clippy::unwrap_used,
    reason = "integration tests panic on unexpected setup errors"
)]

use cw_testkit::TempRoot;
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Output, Stdio};

fn run(root: &TempRoot, args: &[&str], stdin: Option<&[u8]>) -> Output {
    let mut child = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    if let Some(bytes) = stdin {
        pipe.write_all(bytes).unwrap();
    }
    drop(pipe);
    child.wait_with_output().unwrap()
}

fn ok_json(root: &TempRoot, args: &[&str]) -> Value {
    let out = run(root, args, None);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn ok_text(root: &TempRoot, args: &[&str]) -> String {
    let out = run(root, args, None);
    assert!(out.status.success(), "{args:?} failed");
    String::from_utf8(out.stdout).unwrap()
}

fn err_code(root: &TempRoot, args: &[&str]) -> String {
    let out = run(root, args, None);
    assert_eq!(out.status.code(), Some(1), "{args:?} should be refused");
    let doc: Value = serde_json::from_slice(&out.stderr).unwrap();
    doc["code"].as_str().unwrap().to_owned()
}

fn roster() -> Value {
    json!({
        "version": 2,
        "seats": [
            {"name": "0-chief", "role": "chief", "host": "claude-desktop"},
            {
                "name": "orbit-capt",
                "role": "captain",
                "workspace": "orbit",
                "host": "codex-desktop",
                "reports_to": "0-chief"
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
        ok_json(&root, &["mail", "join", seat, "--room", &room]);
    }
    let bytes = serde_json::to_vec(&roster()).unwrap();
    let out = run(&root, &["roster", "apply", "--room", &room], Some(&bytes));
    assert!(out.status.success());
    (root, room)
}

fn load_args(room: &str) -> Vec<&str> {
    vec!["grant", "load", "--room", room, "--as", "0-chief"]
}

fn load(root: &TempRoot, room: &str, file: &Value) -> Output {
    let bytes = serde_json::to_vec(file).unwrap();
    run(root, &load_args(room), Some(&bytes))
}

fn load_ok(root: &TempRoot, room: &str, file: &Value) -> Value {
    let out = load(root, room, file);
    assert!(
        out.status.success(),
        "load failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn load_code(root: &TempRoot, room: &str, file: &Value) -> String {
    let out = load(root, room, file);
    assert_eq!(out.status.code(), Some(1), "load should be refused");
    let doc: Value = serde_json::from_slice(&out.stderr).unwrap();
    doc["code"].as_str().unwrap().to_owned()
}

fn grants(root: &TempRoot, room: &str) -> Vec<Value> {
    let doc = ok_json(root, &["grant", "list", "--room", room, "--json"]);
    doc["grants"].as_array().unwrap().clone()
}

fn seat_entry(root: &TempRoot, room: &str, seat: &str, class: &str, key: &str) -> Option<Value> {
    let doc = ok_json(
        root,
        &["grant", "list", "--room", room, "--seat", seat, "--json"],
    );
    doc[key]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["class"] == class)
        .cloned()
}

fn file() -> Value {
    json!({
        "version": 1,
        "grants": [
            {
                "effect": "grant",
                "to": "chief",
                "class": "direction",
                "workspace": "all",
                "limit": "a setting a later change can undo",
                "source": "advisor chat 2026-11-27"
            },
            {
                "effect": "withhold",
                "to": "captain",
                "class": "land",
                "workspace": "site",
                "source": "chief chat 2026-11-27"
            }
        ]
    })
}

#[test]
fn load_stores_house_entries_listed_below_a_day_entry_and_above_the_catalog() {
    let (root, room) = setup_room();
    let loaded = load_ok(&root, &room, &file());
    assert_eq!(loaded["loaded"], true);
    assert_eq!(loaded["grants"], 2);
    assert_eq!(loaded["unchanged"], false);

    let listed = grants(&root, &room);
    assert_eq!(listed.len(), 2);
    for entry in &listed {
        assert_eq!(entry["origin"], "house_file");
        assert_eq!(entry["for"], "standing");
        assert_eq!(entry["entered_by"], "0-chief");
        assert_eq!(entry["self_entered"], false);
    }
    assert_eq!(listed[0]["source"], "advisor chat 2026-11-27");
    let text = ok_text(&root, &["grant", "list", "--room", &room]);
    assert!(text.contains("house file"), "{text}");
    assert!(text.contains("standing"), "{text}");

    // The house entry beats the catalog default for the same seat, class and workspace.
    let withheld = seat_entry(&root, &room, "site-capt", "land", "withheld").unwrap();
    assert_eq!(withheld["origin"], "house_file");
    assert_eq!(withheld["from"], "withhold");
    assert!(seat_entry(&root, &room, "site-capt", "land", "settles").is_none());
    assert!(
        ok_text(
            &root,
            &["grant", "list", "--room", &room, "--seat", "site-capt"]
        )
        .contains("house withhold")
    );
    let direction = seat_entry(&root, &room, "0-chief", "direction", "settles").unwrap();
    assert_eq!(direction["origin"], "house_file");
    assert_eq!(direction["limit"], "a setting a later change can undo");

    // A day entry for the same seat, class and workspace beats the house entry.
    ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--as",
            "0-chief",
            "--to",
            "captain",
            "--class",
            "land",
            "--workspace",
            "site",
            "--source",
            "chat 2",
        ],
    );
    let day = seat_entry(&root, &room, "site-capt", "land", "settles").unwrap();
    assert_eq!(day["origin"], "room");
    assert!(seat_entry(&root, &room, "site-capt", "land", "withheld").is_none());
}

#[test]
fn loading_the_same_file_twice_leaves_one_copy_and_a_changed_file_replaces_house_entries() {
    let (root, room) = setup_room();
    let day = ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--as",
            "0-chief",
            "--to",
            "orbit-capt",
            "--class",
            "operator",
            "--workspace",
            "orbit",
            "--source",
            "chat 1",
        ],
    );
    let day_id = day["id"].as_i64().unwrap();

    load_ok(&root, &room, &file());
    let first = grants(&root, &room);
    let again = load_ok(&root, &room, &file());
    assert_eq!(again["unchanged"], true);
    assert_eq!(again["replaced"], 0);
    assert_eq!(grants(&root, &room), first);

    // A file that repeats an entry still stores one copy of it.
    let mut doubled = file();
    let entry = doubled["grants"][0].clone();
    doubled["grants"].as_array_mut().unwrap().push(entry);
    assert_eq!(load_ok(&root, &room, &doubled)["unchanged"], true);

    let changed = json!({
        "version": 1,
        "grants": [{
            "effect": "grant",
            "to": "chief",
            "class": "staffing",
            "workspace": "all",
            "source": "advisor chat 2026-11-28"
        }]
    });
    let replaced = load_ok(&root, &room, &changed);
    assert_eq!(replaced["replaced"], 2);
    assert_eq!(replaced["unchanged"], false);
    let listed = grants(&root, &room);
    let house: Vec<_> = listed
        .iter()
        .filter(|entry| entry["origin"] == "house_file")
        .collect();
    assert_eq!(house.len(), 1);
    assert_eq!(house[0]["class"], "staffing");
    let kept: Vec<_> = listed
        .iter()
        .filter(|entry| entry["origin"] == "room")
        .collect();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0]["id"], day_id);
    assert_eq!(kept[0]["source"], "chat 1");

    // An empty file takes every house entry away and still leaves the day grant.
    load_ok(&root, &room, &json!({"version": 1}));
    let listed = grants(&root, &room);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], day_id);
}

#[test]
fn a_file_the_rules_refuse_changes_nothing() {
    let (root, room) = setup_room();
    load_ok(&root, &room, &file());
    let before = grants(&root, &room);

    let entry = |class: &str| {
        json!({
            "version": 1,
            "grants": [{"effect": "grant", "to": "chief", "class": class, "source": "chat"}]
        })
    };
    let refused = [
        (entry("grant"), "class_not_grantable"),
        (entry("credentials"), "class_not_grantable"),
        (entry("nonesuch"), "unknown_class"),
        (
            json!({"version": 1, "remove_classes": ["release"]}),
            "class_not_removable",
        ),
        (
            json!({"version": 1, "classes": [{"name": "release", "covers": "x"}]}),
            "class_exists",
        ),
        (
            json!({"version": 1, "classes": [
                {"name": "wording", "covers": "x"}, {"name": "wording", "covers": "y"}
            ]}),
            "class_exists",
        ),
        (
            json!({"version": 1, "classes": [{"name": "w", "covers": "x", "holder": "boss"}]}),
            "invalid_class",
        ),
        (
            json!({"version": 2, "grants": []}),
            "unknown_grants_version",
        ),
        (json!({"grants": []}), "unknown_grants_version"),
        (
            json!({"version": 1, "grants": [
                {"effect": "grant", "to": "nobody-here", "class": "ship", "source": "chat"}
            ]}),
            "unknown_holder",
        ),
        (
            json!({"version": 1, "grants": [
                {"effect": "grant", "to": "chief", "class": "ship"}
            ]}),
            "invalid_grants_file",
        ),
        (json!({"version": 1, "extra": true}), "invalid_grants_file"),
    ];
    for (document, code) in refused {
        assert_eq!(load_code(&root, &room, &document), code, "{document}");
        assert_eq!(grants(&root, &room), before, "{document}");
    }

    let out = run(&root, &load_args(&room), Some(b"not json"));
    assert_eq!(out.status.code(), Some(1));
    let doc: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(doc["code"], "invalid_grants_file");
    assert_eq!(grants(&root, &room), before);

    // A house entry is changed through the file only.
    let id = before[0]["id"].as_i64().unwrap().to_string();
    assert_eq!(
        err_code(
            &root,
            &["grant", "revoke", &id, "--room", &room, "--as", "0-chief"]
        ),
        "grant_standing"
    );
    assert_eq!(grants(&root, &room), before);
    assert_eq!(
        err_code(
            &root,
            &[
                "grant", "add", "--room", &room, "--as", "0-chief", "--to", "chief", "--class",
                "ship", "--for", "standing", "--source", "chat"
            ]
        ),
        "invalid_lifetime"
    );
}

#[test]
fn the_two_fixed_classes_stay_ungrantable_with_the_file_loaded_or_not() {
    let (root, room) = setup_room();
    for loaded in [false, true] {
        if loaded {
            load_ok(&root, &room, &file());
        }
        for class in ["grant", "credentials"] {
            assert_eq!(
                err_code(
                    &root,
                    &[
                        "grant", "add", "--room", &room, "--as", "0-chief", "--to", "chief",
                        "--class", class, "--source", "chat"
                    ]
                ),
                "class_not_grantable"
            );
        }
    }
}

#[test]
fn a_class_the_file_adds_is_granted_checked_listed_and_settled_like_a_catalog_class() {
    let (root, room) = setup_room();
    let check = |seat: &str| {
        run(
            &root,
            &[
                "grant", "check", "--room", &room, "--seat", seat, "--class", "wording",
            ],
            None,
        )
    };
    assert_eq!(check("0-chief").status.code(), Some(1));

    let classes = json!({
        "version": 1,
        "classes": [{
            "name": "wording",
            "covers": "A wording fix to documents several repositories share",
            "holder": "chief",
            "note": "changes no version"
        }],
        "grants": [{
            "effect": "grant",
            "to": "captain",
            "class": "wording",
            "workspace": "orbit",
            "source": "chat 2026-11-27"
        }]
    });
    let loaded = load_ok(&root, &room, &classes);
    assert_eq!(loaded["classes"], 1);

    // The default holder and the file's own grant.
    let chief = check("0-chief");
    assert!(chief.status.success());
    let chief: Value = serde_json::from_slice(&chief.stdout).unwrap();
    assert_eq!(chief["from"], "catalog");
    let captain = check("orbit-capt");
    let captain: Value = serde_json::from_slice(&captain.stdout).unwrap();
    assert_eq!(captain["origin"], "house_file");
    assert_eq!(check("site-capt").status.code(), Some(1));
    assert!(seat_entry(&root, &room, "0-chief", "wording", "settles").is_some());

    // A day grant for the class works, then a request in the class is opened and settled.
    ok_json(
        &root,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--as",
            "0-chief",
            "--to",
            "site-capt",
            "--class",
            "wording",
            "--workspace",
            "site",
            "--source",
            "chat 3",
        ],
    );
    assert!(check("site-capt").status.success());
    let opened = ok_json(
        &root,
        &[
            "request",
            "open",
            "--room",
            &room,
            "--as",
            "orbit-capt",
            "--class",
            "wording",
            "--record",
            "doc-1",
            "--ask",
            "Fix the heading?",
            "--option",
            "a: yes",
            "--option",
            "b: no",
            "--recommend",
            "a",
            "--json",
        ],
    );
    let id = opened["id"].as_str().unwrap();
    let settled = ok_json(
        &root,
        &[
            "request", "settle", id, "--room", &room, "--as", "0-chief", "--option", "a", "--json",
        ],
    );
    assert_eq!(settled["settlement"]["seat"], "0-chief");

    // Withdrawing the class from the file takes it from the catalog; the day grant stays.
    load_ok(&root, &room, &json!({"version": 1}));
    assert_eq!(check("0-chief").status.code(), Some(1));
    assert!(check("site-capt").status.success());
}

#[test]
fn load_refuses_a_room_with_no_roster_and_a_caller_with_no_seat() {
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().join("room").to_str().unwrap().to_owned();
    ok_json(&root, &["mail", "join", "0-chief", "--room", &room]);
    assert_eq!(load_code(&root, &room, &file()), "no_roster");

    let (root, room) = setup_room();
    let bytes = serde_json::to_vec(&file()).unwrap();
    let out = run(&root, &["grant", "load", "--room", &room], Some(&bytes));
    assert_eq!(out.status.code(), Some(1));
    assert!(grants(&root, &room).is_empty());
}

#[test]
fn load_mails_each_joined_seat_whose_reach_changed_once() {
    let (root, room) = setup_room();
    let inbox_count = |seat: &str| -> usize {
        let doc = ok_json(&root, &["mail", "inbox", "--room", &room, "--as", seat]);
        doc["messages"].as_array().unwrap().len()
    };
    let loaded = load_ok(&root, &room, &file());
    assert_eq!(loaded["notified"], json!(["0-chief", "site-capt"]));
    assert_eq!(inbox_count("0-chief"), 1);
    assert_eq!(inbox_count("site-capt"), 1);
    assert_eq!(inbox_count("orbit-capt"), 0);

    // The same file again changes nothing and mails no one.
    let again = load_ok(&root, &room, &file());
    assert_eq!(again["notified"], json!([]));
    assert_eq!(inbox_count("0-chief"), 1);
}
