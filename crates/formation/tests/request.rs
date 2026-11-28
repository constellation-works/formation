//! Request behavior through the shipped CLI, joined sessions and ordinary mail.
#![expect(
    clippy::unwrap_used,
    reason = "unrecoverable integration fixture errors"
)]

use cw_testkit::TempRoot;
use serde_json::{Value, json};
use std::process::{Command, Output, Stdio};

const CHIEF: &str = "shift-chief";

const CAPTAIN: &str = "app-captain";

const ADVISOR: &str = "shift-advisor";

const WORKER: &str = "app-worker";

const WEB_CAPTAIN: &str = "web-captain";

const OPEN: &[&str] = &[
    "open",
    "--class",
    "design",
    "--record",
    "task-ref",
    "--ask",
    "Which design fits?",
    "--option",
    "a: Keep it local",
    "--option",
    "b: Share the service",
    "--recommend",
    "a",
    "--blocking",
];

struct Room(TempRoot);

impl Room {
    fn new(joined: &[&str]) -> Self {
        let room = Self(TempRoot::new("FORMATION").unwrap());
        for seat in joined {
            room.join(seat);
        }
        room
    }

    fn command(&self) -> Command {
        let mut cmd = self.0.command(env!("CARGO_BIN_EXE_formation"));
        cmd.env("FORMATION_HOME", self.0.path())
            .env_remove("FORMATION_ROOM")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("ANTIGRAVITY_CONVERSATION_ID");
        cmd
    }

    fn store(&self, room: &str) -> formation_store::Store {
        formation_store::Store::open(&self.0.path().join("rooms").join(room)).unwrap()
    }

    fn clear_roster(&self, room: &str) {
        // Seed an empty applied roster while retaining existing request rows.
        let roster = formation_store::decode(br#"{"version":2,"seats":[],"routes":[]}"#).unwrap();
        self.store(room)
            .write(|tx| tx.roster().replace(&roster))
            .unwrap();
    }

    fn request_debug(&self, room: &str, id: &str) -> String {
        let snapshot = self.store(room).read().unwrap();
        format!("{:#?}", snapshot.requests().get(id).unwrap().unwrap())
    }

    fn join(&self, seat: &str) {
        let out = self
            .command()
            .env("CLAUDE_CODE_SESSION_ID", format!("session-{seat}"))
            .args(["mail", "join", seat, "--room", "trial"])
            .output()
            .unwrap();
        success(out);
    }

    fn apply_roster(&self) {
        self.apply(&self.default_roster());
    }

    fn apply(&self, roster: &Value) {
        let mut command = self.command();
        command.args(["roster", "apply", "--room", "trial"]);
        success(run_with_input(&mut command, roster));
    }

    fn default_roster(&self) -> Value {
        json!({
            "version": 2,
            "seats": [
                {"name": CHIEF, "role": "chief", "host": "claude-desktop"},
                {"name": CAPTAIN, "role": "captain", "host": "claude-desktop",
                 "workspace": "app", "reports_to": CHIEF},
                {"name": ADVISOR, "role": "advisor", "host": "claude-desktop",
                 "reports_to": CHIEF},
                {"name": WORKER, "role": "worker", "host": "headless",
                 "workspace": "app", "reports_to": CAPTAIN}
            ],
            "routes": []
        })
    }

    fn request(&self, seat: Option<&str>, args: &[&str]) -> Output {
        let mut cmd = self.command();
        cmd.arg("request");
        if let Some(seat) = seat {
            cmd.env("CLAUDE_CODE_SESSION_ID", format!("session-{seat}"));
        } else {
            cmd.args(["--room", "trial"]);
        }
        cmd.args(args).output().unwrap()
    }

    fn json(&self, seat: Option<&str>, args: &[&str]) -> Value {
        let mut args = args.to_vec();
        args.push("--json");
        parse(self.request(seat, &args))
    }

    fn refuse(&self, seat: Option<&str>, args: &[&str], code: &str) -> Value {
        let mut args = args.to_vec();
        args.push("--json");
        let out = self.request(seat, &args);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        assert!(out.stdout.is_empty());
        let error: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(error["code"], code, "{error}");
        error
    }

    fn inbox(&self, seat: &str) -> Vec<Value> {
        let out = self
            .command()
            .args(["mail", "inbox", "--room", "trial", "--as", seat])
            .output()
            .unwrap();
        let mut payload = parse(out);
        payload["messages"].as_array_mut().unwrap().split_off(0)
    }

    fn grant(&self, action: &str, seat: &str, workspace: &str) -> Value {
        parse(
            self.command()
                .args([
                    "grant",
                    action,
                    "--room",
                    "trial",
                    "--as",
                    CAPTAIN,
                    "--to",
                    seat,
                    "--class",
                    "design",
                    "--workspace",
                    workspace,
                    "--limit",
                    "reversible settings",
                    "--source",
                    "operator chat",
                ])
                .output()
                .unwrap(),
        )
    }

    fn assert_mail(&self, seat: &str, sender: &str, id: &str) {
        let messages: Vec<_> = self
            .inbox(seat)
            .into_iter()
            .filter(|message| {
                message["sender"] == sender && message["body"].as_str().unwrap().contains(id)
            })
            .collect();
        assert_eq!(messages.len(), 1, "{messages:?}");
    }
}

#[test]
fn open_reports_the_parent_and_delivers_through_the_hook() {
    let room = Room::new(&[CHIEF, CAPTAIN]);
    room.apply_roster();
    let opened = room.json(Some(CAPTAIN), OPEN);
    let id = opened["id"].as_str().unwrap();
    assert!(id.starts_with("req-trial-"), "{id}");
    assert_eq!(opened["holder"], CHIEF);
    assert_eq!(opened["workspace"], "app");
    assert_eq!(opened["record"], "task-ref");
    assert_eq!(opened["blocking"], true);

    let mut command = room.command();
    command.args(["hook", "--provider", "claude", "--event", "user-prompt"]);
    let hook = parse(run_with_input(
        &mut command,
        &json!({"session_id": format!("session-{CHIEF}"), "prompt": "Check requests"}),
    ));
    let context = hook["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains(&format!("\nRequest {id}\n")), "{context}");

    let mut args = OPEN.to_vec();
    args[2] = "credentials";
    let opened_text = success(
        room.command()
            .args(["request", "--room", "trial", "--as", CAPTAIN])
            .args(args)
            .output()
            .unwrap(),
    );
    assert!(opened_text.contains("Request: req-trial-"));
    assert!(opened_text.contains(&format!("Holder: {CHIEF}")));
}

fn seat(name: &str, role: &str, reports_to: Option<&str>) -> Value {
    let mut seat = json!({"name": name, "role": role, "host": "claude-desktop"});
    if role == "captain" {
        seat["workspace"] = json!("app");
    }
    if let Some(parent) = reports_to {
        seat["reports_to"] = json!(parent);
    }
    seat
}

fn roster(seats: &[Value]) -> Value {
    json!({"version": 2, "seats": seats, "routes": []})
}

fn chief_mail(room: &Room, id: &str) -> usize {
    room.inbox(CHIEF)
        .iter()
        .filter(|message| message["body"].as_str().unwrap().contains(id))
        .count()
}

#[test]
fn captain_advisor_and_aide_requests_are_held_by_the_chief() {
    const AIDE: &str = "shift-aide";
    for explicit in [true, false] {
        let parent = explicit.then_some(CHIEF);
        let room = Room::new(&[CHIEF, CAPTAIN, ADVISOR, AIDE]);
        room.apply(&roster(&[
            seat(CHIEF, "chief", None),
            seat(CAPTAIN, "captain", parent),
            seat(ADVISOR, "advisor", parent),
            seat(AIDE, "aide", parent),
        ]));
        for asker in [CAPTAIN, ADVISOR, AIDE] {
            let opened = room.json(Some(asker), OPEN);
            let id = opened["id"].as_str().unwrap();
            assert_eq!(opened["holder"], CHIEF, "{asker} explicit={explicit}");
            assert_eq!(chief_mail(&room, id), 1, "{asker} explicit={explicit}");
            room.assert_mail(CHIEF, asker, id);
            let shown = room.json(None, &["show", id]);
            assert_eq!(shown["holder"], CHIEF);
        }
        let waiting = room.json(None, &["list", "--waiting", CHIEF]);
        assert_eq!(waiting.as_array().unwrap().len(), 3);
    }
}

#[test]
fn a_chief_request_without_a_unique_chief_is_refused_and_changes_nothing() {
    let cases = [
        (
            "no chief",
            roster(&[seat(CAPTAIN, "captain", None)]),
            "chief_required",
        ),
        (
            "two chiefs",
            roster(&[
                seat(CHIEF, "chief", None),
                seat("second-chief", "chief", None),
                seat(CAPTAIN, "captain", None),
            ]),
            "chief_ambiguous",
        ),
        (
            "parent is a captain",
            roster(&[
                seat(CHIEF, "chief", None),
                seat(CAPTAIN, "captain", Some(CHIEF)),
                seat(ADVISOR, "advisor", Some(CAPTAIN)),
            ]),
            "request_parent_not_chief",
        ),
    ];
    for (label, roster, code) in cases {
        let asker = if label == "parent is a captain" {
            ADVISOR
        } else {
            CAPTAIN
        };
        let room = Room::new(&[CHIEF, CAPTAIN, ADVISOR, "second-chief"]);
        room.apply(&roster);
        let before = (room.inbox(CHIEF), room.inbox("second-chief"));
        let error = room.refuse(Some(asker), OPEN, code);
        assert!(error["error"].as_str().is_some(), "{label}: {error}");
        assert_eq!(room.json(None, &["list"]), json!([]), "{label}");
        assert_eq!(
            before,
            (room.inbox(CHIEF), room.inbox("second-chief")),
            "{label}"
        );
        assert!(room.inbox(CAPTAIN).is_empty(), "{label}");
    }
}

#[test]
fn subordinates_keep_their_parent_and_a_chief_request_escalates_to_the_admiral() {
    let room = Room::new(&[CHIEF, CAPTAIN, WORKER]);
    room.apply_roster();
    let opened = room.json(Some(WORKER), OPEN);
    assert_eq!(opened["holder"], CAPTAIN);
    room.assert_mail(CAPTAIN, WORKER, opened["id"].as_str().unwrap());

    let own = room.json(Some(CHIEF), OPEN);
    assert_eq!(own["holder"], "admiral");
    let waiting = room.json(None, &["list", "--waiting", "admiral"]);
    assert_eq!(waiting.as_array().unwrap().len(), 1);
}

#[test]
fn advice_pass_and_settlement_track_authority_and_send_mail() {
    let room = Room::new(&[CHIEF, CAPTAIN, ADVISOR, WORKER]);
    room.apply_roster();

    let first = room.json(Some(CAPTAIN), OPEN);
    let second = room.json(Some(CAPTAIN), OPEN);
    let third = room.json(Some(WORKER), OPEN);
    let first_id = first["id"].as_str().unwrap();
    let second_id = second["id"].as_str().unwrap();
    let third_id = third["id"].as_str().unwrap();

    room.grant("add", ADVISOR, "app");
    let denied = room.refuse(
        Some(CHIEF),
        &["settle", first_id, "--option", "a"],
        "not_authorized",
    );
    assert!(denied["error"].as_str().unwrap().contains(ADVISOR));
    assert_eq!(room.json(None, &["show", first_id]), first);
    assert!(
        room.inbox(CAPTAIN)
            .iter()
            .all(|message| { !message["body"].as_str().unwrap().contains(first_id) })
    );

    let advice = room.json(
        Some(ADVISOR),
        &[
            "advise",
            first_id,
            "--recommend",
            "b",
            "--note",
            "A second view",
        ],
    );
    assert_eq!(advice["advice"][0]["seat"], ADVISOR);
    assert_eq!(advice["advice"][0]["option"], "b");
    assert_eq!(advice["advice"][0]["note"], "A second view");
    let mut without_advice = advice.clone();
    without_advice["advice"] = json!([]);
    assert_eq!(without_advice, first);
    assert_eq!(room.json(None, &["show", first_id]), advice);
    room.assert_mail(CHIEF, ADVISOR, first_id);
    let waiting = room.json(None, &["list", "--waiting", CHIEF]);
    assert_eq!(waiting, json!([second, advice]));

    room.refuse(Some(ADVISOR), &["pass", third_id], "request_not_holder");
    let passed = room.json(Some(CAPTAIN), &["pass", third_id]);
    assert_eq!(passed["holder"], CHIEF);
    room.assert_mail(CHIEF, CAPTAIN, third_id);
    let passed = room.json(
        Some(CHIEF),
        &[
            "pass",
            third_id,
            "--recommend",
            "b",
            "--note",
            "Needs a decision",
        ],
    );
    assert_eq!(passed["holder"], "admiral");
    assert_eq!(passed["advice"][0]["note"], "Needs a decision");

    room.grant("add", CHIEF, "other-workspace");
    room.refuse(
        Some(CHIEF),
        &["settle", first_id, "--option", "a"],
        "not_authorized",
    );
    let grant = room.grant("add", CHIEF, "app");
    let settlement = room.json(
        Some(CHIEF),
        &[
            "settle",
            first_id,
            "--option",
            "a",
            "--note",
            "Keep the smaller scope",
        ],
    );
    assert_eq!(settlement["state"], "settled");
    assert_eq!(settlement["settlement"]["option"], "a");
    assert_eq!(settlement["settlement"]["seat"], CHIEF);
    assert_eq!(settlement["settlement"]["grant_id"], grant["id"]);
    assert_eq!(settlement["settlement"]["limit"], "reversible settings");
    assert_eq!(settlement["settlement"]["note"], "Keep the smaller scope");
    room.refuse(
        Some(CHIEF),
        &["settle", first_id, "--option", "a"],
        "request_settled",
    );
    room.assert_mail(CAPTAIN, CHIEF, first_id);
    room.json(Some(CHIEF), &["settle", second_id, "--option", "b"]);

    assert_eq!(
        room.json(None, &["list", "--waiting", "admiral"]),
        json!([passed])
    );
    let settled_by = room.json(None, &["list", "--settled-by", CHIEF]);
    assert_eq!(settled_by.as_array().unwrap().len(), 2);
    for record in settled_by.as_array().unwrap() {
        assert_eq!(record["settlement"]["grant_id"], grant["id"]);
        assert_eq!(record["settlement"]["limit"], "reversible settings");
    }

    let grant_id = grant["id"].to_string();
    let revoked = room
        .command()
        .args([
            "grant", "revoke", &grant_id, "--room", "trial", "--as", CAPTAIN,
        ])
        .output()
        .unwrap();
    success(revoked);
    assert_eq!(room.json(None, &["show", first_id]), settlement);

    let since = settlement["settlement"]["at"].as_str().unwrap();
    let since = room.json(None, &["list", "--settled-by", CHIEF, "--since", since]);
    assert_eq!(since, settled_by);
    assert_eq!(
        room.json(None, &["list", "--since", "9999-01-01T00:00:00Z"]),
        json!([])
    );
}

#[test]
fn open_and_settle_checks_the_askers_own_effective_authority() {
    let room = Room::new(&[CHIEF, CAPTAIN]);
    room.apply_roster();
    let mut args = OPEN.to_vec();
    args.extend(["--settle", "a"]);
    room.grant("add", CHIEF, "app");
    room.refuse(Some(CAPTAIN), &args, "not_authorized");
    assert_eq!(room.json(None, &["list"]), json!([]));

    room.grant("add", CAPTAIN, "app");
    room.grant("withhold", CAPTAIN, "app");
    room.refuse(Some(CAPTAIN), &args, "not_authorized");
    assert_eq!(room.json(None, &["list"]), json!([]));

    let grant = room.grant("add", CAPTAIN, "app");
    let with_grant = room.json(Some(CAPTAIN), &args);
    assert_eq!(with_grant["settlement"]["grant_id"], grant["id"]);
    assert_eq!(with_grant["settlement"]["limit"], "reversible settings");

    args[2] = "land";
    let opened = room.json(Some(CAPTAIN), &args);
    let id = opened["id"].as_str().unwrap();
    assert_eq!(opened["state"], "settled");
    assert_eq!(opened["holder"], Value::Null);
    assert_eq!(opened["settlement"]["seat"], CAPTAIN);
    assert_eq!(opened["settlement"]["option"], "a");
    assert_eq!(opened["settlement"]["grant_id"], Value::Null);
    assert!(
        room.inbox(CHIEF)
            .iter()
            .all(|message| { !message["body"].as_str().unwrap().contains(id) })
    );
    room.assert_mail(CAPTAIN, CAPTAIN, id);
}

#[test]
fn every_request_command_requires_an_applied_roster() {
    let room = Room::new(&[CAPTAIN]);
    for args in [
        OPEN,
        &["advise", "missing", "--recommend", "a", "--note", "why"],
        &["settle", "missing", "--option", "a"],
        &["pass", "missing"],
        &["list"],
        &["show", "missing"],
    ] {
        room.refuse(Some(CAPTAIN), args, "no_roster");
    }
    room.refuse(None, &["list"], "no_roster");
}

#[test]
fn admiral_settlement_and_reopen_require_a_roster_and_leave_requests_unchanged() {
    let room = Room::new(&[CHIEF, CAPTAIN]);
    room.apply_roster();

    let reopened = room.json(Some(CAPTAIN), OPEN);
    let reopened_id = reopened["id"].as_str().unwrap();
    room.json(
        Some(CHIEF),
        &[
            "settle",
            reopened_id,
            "--option",
            "a",
            "--admiral",
            "--source",
            "operator chat",
        ],
    );

    let unsettled = room.json(Some(CAPTAIN), OPEN);
    let unsettled_id = unsettled["id"].as_str().unwrap();
    let reopened_before = room.request_debug("trial", reopened_id);
    let unsettled_before = room.request_debug("trial", unsettled_id);

    room.clear_roster("trial");
    room.refuse(
        Some(CHIEF),
        &["reopen", reopened_id, "--admiral"],
        "no_roster",
    );
    room.refuse(
        Some(CHIEF),
        &[
            "settle",
            unsettled_id,
            "--option",
            "b",
            "--admiral",
            "--source",
            "operator chat",
        ],
        "no_roster",
    );

    assert_eq!(room.request_debug("trial", reopened_id), reopened_before);
    assert_eq!(room.request_debug("trial", unsettled_id), unsettled_before);
}

#[test]
fn carry_requires_a_destination_roster_and_leaves_both_rooms_unchanged() {
    let room = Room::new(&[CHIEF, CAPTAIN]);
    room.apply_roster();
    let source = room.json(Some(CAPTAIN), OPEN);
    let source_id = source["id"].as_str().unwrap();
    let source_before = room.request_debug("trial", source_id);

    let mut join = room.command();
    join.env(
        "CLAUDE_CODE_SESSION_ID",
        format!("session-{CAPTAIN}-tomorrow"),
    )
    .args(["mail", "join", CAPTAIN, "--room", "tomorrow"]);
    success(join.output().unwrap());

    let destination = room.store("tomorrow");
    let destination_before = destination.read().unwrap().requests().list().unwrap();
    assert!(destination_before.is_empty());

    let mut carry = room.command();
    carry
        .env(
            "CLAUDE_CODE_SESSION_ID",
            format!("session-{CAPTAIN}-tomorrow"),
        )
        .args([
            "request", "carry", "--room", "tomorrow", "--from", "trial", "--json",
        ]);
    let out = carry.output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty());
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(error["code"], "no_roster", "{error}");

    assert!(
        destination
            .read()
            .unwrap()
            .requests()
            .list()
            .unwrap()
            .is_empty()
    );
    assert_eq!(room.request_debug("trial", source_id), source_before);
}

#[test]
fn unauthorized_settlement_names_the_default_admiral_holder() {
    let room = Room::new(&[CHIEF, CAPTAIN]);
    room.apply_roster();
    let opened = room.json(Some(CAPTAIN), OPEN);
    let id = opened["id"].as_str().unwrap();
    let before = room.json(None, &["show", id]);

    let refusal = room.refuse(
        Some(CAPTAIN),
        &["settle", id, "--option", "a"],
        "not_authorized",
    );
    assert!(
        refusal["error"]
            .as_str()
            .unwrap()
            .contains("default holder: admiral"),
        "{refusal}"
    );
    assert_eq!(room.json(None, &["show", id]), before);
}

#[test]
fn a_failed_notification_rolls_back_the_request() {
    let room = Room::new(&[CAPTAIN]);
    room.apply_roster();
    room.refuse(Some(CAPTAIN), OPEN, "unknown_recipient");
    assert_eq!(room.json(None, &["list"]), json!([]));

    room.join(CHIEF);
    let opened = room.json(Some(CAPTAIN), OPEN);
    room.assert_mail(CHIEF, CAPTAIN, opened["id"].as_str().unwrap());
}

#[test]
fn admiral_answers_and_overturns_delegated_settlements() {
    let room = Room::new(&[CHIEF, CAPTAIN]);
    room.apply_roster();
    let first = room.json(Some(CAPTAIN), OPEN);
    let id = first["id"].as_str().unwrap();
    let missing = room.request(Some(CAPTAIN), &["settle", id, "--option", "a", "--admiral"]);
    assert_eq!(missing.status.code(), Some(2));
    let answer = room.json(
        Some(CAPTAIN),
        &[
            "settle",
            id,
            "--option",
            "b",
            "--admiral",
            "--source",
            "captain chat",
        ],
    );
    assert_eq!(answer["settlement"]["seat"], "admiral");
    assert_eq!(answer["settlement"]["source"], "captain chat");
    assert_eq!(answer["settlement"]["entered_by"], CAPTAIN);
    room.refuse(Some(CHIEF), &["reopen", id], "admiral_required");
    let reopened = room.json(Some(CHIEF), &["reopen", id, "--admiral"]);
    assert_eq!(reopened["holder"], "admiral");
    assert_eq!(reopened["overturned"][0]["source"], "captain chat");
    let second = room.json(Some(CAPTAIN), OPEN);
    let id2 = second["id"].as_str().unwrap();
    room.grant("add", CHIEF, "app");
    room.json(Some(CHIEF), &["settle", id2, "--option", "a"]);
    let reopened = room.json(Some(CHIEF), &["reopen", id2, "--admiral"]);
    assert_eq!(reopened["overturned"][0]["seat"], CHIEF);
    let listed = room.json(None, &["list", "--settled-by", CHIEF]);
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["overturned"][0]["seat"], CHIEF);
    room.assert_mail(CHIEF, "admiral", id2);
    room.assert_mail(CAPTAIN, "admiral", id2);
}

#[test]
fn admiral_changes_refuse_a_caller_without_a_joined_seat_and_change_nothing() {
    let room = Room::new(&[CHIEF, CAPTAIN]);
    room.apply_roster();
    let open = room.json(Some(CAPTAIN), OPEN);
    let id = open["id"].as_str().unwrap();
    let settled = room.json(Some(CAPTAIN), OPEN);
    let settled_id = settled["id"].as_str().unwrap();
    let answer = room.json(
        Some(CAPTAIN),
        &[
            "settle",
            settled_id,
            "--option",
            "a",
            "--admiral",
            "--source",
            "captain chat",
        ],
    );
    assert_eq!(answer["settlement"]["entered_by"], CAPTAIN);
    let before = room.json(None, &["list"]);
    let mail = (room.inbox(CHIEF).len(), room.inbox(CAPTAIN).len());

    let settle = [
        "settle",
        id,
        "--option",
        "a",
        "--admiral",
        "--source",
        "chat",
    ];
    let reopen = ["reopen", settled_id, "--admiral"];
    for args in [&settle[..], &reopen[..]] {
        let mut absent = args.to_vec();
        absent.push("--json");
        let out = room.request(None, &absent);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let error: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(error["code"], "seat_required");

        let mut ghost = room.command();
        ghost
            .env("CLAUDE_CODE_SESSION_ID", "session-ghost")
            .args(["request", "--room", "trial"])
            .args(&absent);
        let out = ghost.output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let error: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(error["code"], "seat_required");
    }

    assert_eq!(room.json(None, &["list"]), before);
    assert_eq!(room.json(None, &["show", settled_id]), answer);
    assert_eq!(mail, (room.inbox(CHIEF).len(), room.inbox(CAPTAIN).len()));
}

#[test]
fn carry_preserves_open_history_and_maps_missing_holders() {
    let room = Room::new(&[CHIEF, CAPTAIN, WORKER]);
    room.apply_roster();
    let opened = room.json(Some(WORKER), OPEN);
    let id = opened["id"].as_str().unwrap();
    room.json(
        Some(CAPTAIN),
        &["advise", id, "--recommend", "b", "--note", "context"],
    );
    let closed = room.json(Some(CAPTAIN), OPEN);
    let closed_id = closed["id"].as_str().unwrap();
    room.json(
        Some(CAPTAIN),
        &[
            "settle",
            closed_id,
            "--option",
            "a",
            "--admiral",
            "--source",
            "chat",
        ],
    );
    success(
        room.command()
            .args(["mail", "join", CHIEF, "--room", "tomorrow"])
            .output()
            .unwrap(),
    );
    let roster = json!({
        "version": 2,
        "seats": [{"name": CHIEF, "role": "chief", "host": "claude-desktop"}],
        "routes": []
    });
    let mut command = room.command();
    command.args(["roster", "apply", "--room", "tomorrow"]);
    success(run_with_input(&mut command, &roster));
    let carry = || {
        parse(
            room.command()
                .args([
                    "request", "carry", "--room", "tomorrow", "--from", "trial", "--json",
                ])
                .output()
                .unwrap(),
        )
    };
    let carried = carry();
    assert_eq!(carried.as_array().unwrap().len(), 1);
    assert_eq!(carried[0]["id"], id);
    assert_eq!(carried[0]["holder"], CHIEF);
    assert_eq!(carried[0]["advice"][0]["note"], "context");
    assert_eq!(carry(), json!([]));
}

fn run_with_input(command: &mut Command, input: &Value) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    serde_json::to_writer(child.stdin.take().unwrap(), input).unwrap();
    child.wait_with_output().unwrap()
}

fn success(out: Output) -> String {
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn parse(out: Output) -> Value {
    serde_json::from_str(&success(out)).unwrap()
}

fn two_workspace_roster() -> Value {
    json!({
        "version": 2,
        "seats": [
            {"name": CHIEF, "role": "chief", "host": "claude-desktop"},
            {"name": CAPTAIN, "role": "captain", "host": "claude-desktop",
             "workspace": "app", "reports_to": CHIEF},
            {"name": WEB_CAPTAIN, "role": "captain", "host": "claude-desktop",
             "workspace": "web", "reports_to": CHIEF},
            {"name": ADVISOR, "role": "advisor", "host": "claude-desktop",
             "reports_to": CHIEF}
        ],
        "routes": []
    })
}

fn with_workspace<'a>(workspace: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = OPEN.to_vec();
    args.extend(["--workspace", workspace]);
    args.extend(extra);
    args
}

#[test]
fn open_names_a_roster_workspace_and_a_matching_grant_covers_it() {
    let room = Room::new(&[CHIEF, CAPTAIN, WEB_CAPTAIN, ADVISOR]);
    room.apply(&two_workspace_roster());

    // An advisor with no workspace can name one; the default stays `all`.
    let named = room.json(Some(ADVISOR), &with_workspace("web", &[]));
    assert_eq!(named["workspace"], "web");
    assert_eq!(named["holder"], CHIEF);
    let shown = room.json(None, &["show", named["id"].as_str().unwrap()]);
    assert_eq!(shown["workspace"], "web");
    let text = success(room.request(None, &["show", named["id"].as_str().unwrap()]));
    assert!(text.contains("(web)"), "{text}");
    assert_eq!(room.json(Some(ADVISOR), OPEN)["workspace"], "all");

    // The asker's own workspace is the default; an explicit value overrides it.
    assert_eq!(room.json(Some(CAPTAIN), OPEN)["workspace"], "app");
    assert_eq!(
        room.json(Some(CAPTAIN), &with_workspace("web", &[]))["workspace"],
        "web"
    );
    assert_eq!(
        room.json(Some(CAPTAIN), &with_workspace("all", &[]))["workspace"],
        "all"
    );

    // Grants are evaluated against the named workspace, not the asker's own.
    let settle = ["--settle", "a"];
    room.grant("add", ADVISOR, "app");
    room.refuse(
        Some(ADVISOR),
        &with_workspace("web", &settle),
        "not_authorized",
    );
    let covered = room.json(Some(ADVISOR), &with_workspace("app", &settle));
    assert_eq!(covered["workspace"], "app");
    assert_eq!(covered["state"], "settled");
    assert_eq!(covered["settlement"]["seat"], ADVISOR);
}

#[test]
fn open_refuses_an_unknown_workspace_and_changes_nothing() {
    let room = Room::new(&[CHIEF, CAPTAIN, ADVISOR]);
    room.apply(&two_workspace_roster());
    let before = room.inbox(CHIEF).len();
    for workspace in ["missing", "App", ""] {
        let error = room.refuse(
            Some(ADVISOR),
            &with_workspace(workspace, &[]),
            "unknown_workspace",
        );
        assert!(error["error"].as_str().unwrap().contains("`app`"));
    }
    room.refuse(
        Some(ADVISOR),
        &with_workspace("missing", &["--settle", "a"]),
        "unknown_workspace",
    );
    assert_eq!(room.json(None, &["list"]), json!([]));
    assert_eq!(room.inbox(CHIEF).len(), before);
}
