//! The admiral: a terminal call with no agent session, through independent CLI processes.
#![cfg(unix)]
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

#[allow(dead_code, reason = "this crate needs only the terminal helper")]
mod support;

use cw_testkit::TempRoot;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const CHIEF: &str = "chief-session";

const CAPTAIN: &str = "captain-session";

const OPEN_REQUEST: &[&str] = &[
    "open",
    "--class",
    "design",
    "--record",
    "task-ref",
    "--ask",
    "Share the cache?",
    "--option",
    "a: Share",
    "--option",
    "b: Keep separate",
    "--recommend",
    "a",
];

/// One room, `trial`, in a disposable Formation home with a chief and a captain joined.
struct Home(TempRoot);

impl Home {
    fn new() -> Self {
        let home = Self(TempRoot::new("FORMATION").unwrap());
        for (session, seat) in [(CHIEF, "chief"), (CAPTAIN, "captain")] {
            let out = home.piped(Some(session), &["join", seat, "--room", "trial"]);
            assert!(out.status.success(), "{out:?}");
        }
        home
    }

    fn command(&self, session: Option<&str>) -> Command {
        let mut cmd = self.0.command(env!("CARGO_BIN_EXE_formation"));
        cmd.env("FORMATION_HOME", self.0.path())
            .env_remove("FORMATION_ROOM")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("ANTIGRAVITY_CONVERSATION_ID");
        if let Some(session) = session {
            cmd.env("CODEX_THREAD_ID", session);
        }
        cmd
    }

    /// A mail command with no terminal: stdin is closed and stdout and stderr are pipes.
    fn piped(&self, session: Option<&str>, args: &[&str]) -> Output {
        self.command(session)
            .arg("mail")
            .args(args)
            .output()
            .unwrap()
    }

    /// A mail command in room `trial` with its stdout on a terminal, printing JSON.
    fn terminal(&self, session: Option<&str>, args: &[&str]) -> support::OnTerminal {
        let mut cmd = self.command(session);
        cmd.args(["mail", "--room", "trial", "--json"]).args(args);
        support::on_terminal(cmd)
    }

    fn terminal_default(&self, session: Option<&str>, args: &[&str]) -> support::OnTerminal {
        let mut cmd = self.command(session);
        cmd.args(["mail", "--json"]).args(args);
        support::on_terminal(cmd)
    }

    fn terminal_default_with_room_env(
        &self,
        session: Option<&str>,
        room: &str,
        args: &[&str],
    ) -> support::OnTerminal {
        let mut cmd = self.command(session);
        cmd.env("FORMATION_ROOM", room)
            .args(["mail", "--json"])
            .args(args);
        support::on_terminal(cmd)
    }

    /// Any Formation command with its stdout on a terminal.
    fn terminal_cmd(&self, session: Option<&str>, args: &[&str]) -> support::OnTerminal {
        let mut cmd = self.command(session);
        cmd.args(args);
        support::on_terminal(cmd)
    }

    /// Run a request command as a joined session in room `trial`, printing JSON.
    fn request_ok(&self, session: &str, args: &[&str]) -> Value {
        let out = self
            .command(Some(session))
            .args(["request", "--room", "trial", "--json"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn ok(&self, session: &str, args: &[&str]) -> Value {
        let mut full = vec!["--room", "trial", "--json"];
        full.extend_from_slice(args);
        let out = self.piped(Some(session), &full);
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn apply_roster(&self, room: &str) {
        let roster = json!({
            "version": 2,
            "seats": [
                {"name": "chief", "role": "chief", "host": "claude-desktop"},
                {
                    "name": "captain",
                    "role": "captain",
                    "workspace": "site",
                    "host": "codex-desktop",
                    "reports_to": "chief"
                }
            ],
            "routes": []
        });
        self.apply_roster_value(room, &roster);
    }

    fn apply_roster_value(&self, room: &str, roster: &Value) {
        let mut child = self
            .command(None)
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
        assert!(out.status.success(), "{out:?}");
    }

    fn send_to_admiral(&self, session: &str, room: &str, id: &str, body: &str) -> Value {
        let out = self.piped(
            Some(session),
            &[
                "--room", room, "send", "admiral", "--id", id, "--body", body,
            ],
        );
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn close_room(&self, session: &str, room: &str) {
        let out = self
            .command(Some(session))
            .args(["roster", "close", "--room", room])
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }

    fn snapshot(&self) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                if path.is_dir() {
                    entries.insert(relative, None);
                    visit(root, &path, entries);
                } else {
                    entries.insert(relative, Some(std::fs::read(path).unwrap()));
                }
            }
        }

        let root = self.0.path();
        let mut entries = BTreeMap::new();
        visit(root, root, &mut entries);
        entries
    }

    fn assert_unchanged(&self, before: &BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let after = self.snapshot();
        let changed = before
            .iter()
            .filter_map(|(path, contents)| (after.get(path) != Some(contents)).then_some(path))
            .collect::<Vec<_>>();
        let added = after
            .keys()
            .filter(|path| !before.contains_key(*path))
            .collect::<Vec<_>>();
        assert!(
            changed.is_empty() && added.is_empty(),
            "home files changed: {changed:?}; added: {added:?}"
        );
    }

    fn database(&self) -> std::path::PathBuf {
        self.0.path().join("rooms/trial/inbox.sqlite3")
    }

    fn sql(&self, script: &str) {
        let out = self
            .0
            .command("python3")
            .args([
                "-c",
                "import sqlite3 as s,sys; s.connect(sys.argv[1]).executescript(sys.argv[2])",
            ])
            .arg(self.database())
            .arg(script)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }

    /// Every audit event, oldest first.
    fn audit(&self) -> Vec<Value> {
        let out = self
            .0
            .command("python3")
            .args([
                "-c",
                "import sqlite3 as s,sys,json; print(json.dumps([json.loads(r[0]) for r in \
                 s.connect(sys.argv[1]).execute('SELECT event FROM audit ORDER BY seq')]))",
            ])
            .arg(self.database())
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

fn json_of(out: &support::OnTerminal) -> Value {
    assert!(out.status.success(), "{}", out.stderr);
    serde_json::from_str(&out.stdout).unwrap()
}

fn code(stderr: &[u8]) -> String {
    let error: Value = serde_json::from_slice(stderr).unwrap();
    error["code"].as_str().unwrap().to_owned()
}

fn refused(out: &Output, expected: &str) {
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(code(&out.stderr), expected, "{out:?}");
}

fn terminal_refused(out: &support::OnTerminal, expected: &str) {
    assert_eq!(out.status.code(), Some(1), "{}", out.stderr);
    assert_eq!(code(out.stderr.as_bytes()), expected, "{}", out.stderr);
}

/// The admiral's `mail who` row with no mail: counts and no presence.
fn empty_admiral() -> Value {
    json!({
        "name": "admiral",
        "undelivered": 0,
        "unacknowledged": 0,
        "held": null,
        "seen": null,
    })
}

fn admiral_row(who: &Value) -> Value {
    who["seats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|seat| seat["name"] == "admiral")
        .cloned()
        .unwrap()
}

#[test]
fn a_terminal_call_with_no_session_is_the_admiral_and_the_audit_says_so() {
    let home = Home::new();
    let inbox = json_of(&home.terminal(None, &["inbox"]));
    assert_eq!(inbox, json!({"messages": []}));
    let last = home.audit().pop().unwrap();
    assert_eq!(last["command"], "mail inbox");
    assert_eq!(last["status"], "success");
    assert_eq!(last["caller"], "admiral");
    assert_eq!(last["terminal"], true);

    // Outside an agent session, `--as admiral` is the same as leaving `--as` out.
    json_of(&home.terminal(None, &["--as", "admiral", "inbox"]));
    let last = home.audit().pop().unwrap();
    assert_eq!(
        (&last["command"], &last["caller"]),
        (&json!("mail inbox"), &json!("admiral"))
    );

    let row = admiral_row(&json_of(&home.terminal(None, &["who"])));
    assert_eq!(row, empty_admiral());
}

#[test]
fn the_admiral_inbox_defaults_to_the_only_active_room() {
    let home = Home::new();
    home.apply_roster("trial");
    let sent = home.send_to_admiral(CHIEF, "trial", "default-1", "Read this room.");

    let closed = home.piped(
        Some("old-room-chief"),
        &["join", "chief", "--room", "closed"],
    );
    assert!(closed.status.success(), "{closed:?}");
    home.close_room("old-room-chief", "closed");

    let inbox = json_of(&home.terminal_default(None, &["inbox"]));
    assert_eq!(inbox["messages"][0]["receipt"], sent["receipt"]);
    assert_eq!(inbox["messages"][0]["body"], "Read this room.");
}

#[test]
fn ambiguous_and_missing_active_rooms_are_named_in_room_required_refusals() {
    let home = Home::new();
    let joined = home.piped(
        Some("second-room-chief"),
        &["join", "chief", "--room", "second"],
    );
    assert!(joined.status.success(), "{joined:?}");

    let before = home.snapshot();
    let ambiguous = home.terminal_default(None, &["inbox"]);
    terminal_refused(&ambiguous, "room_required");
    let message = &ambiguous.stderr;
    assert!(message.contains("multiple rooms are active"), "{message}");
    assert!(message.contains("trial"), "{message}");
    assert!(message.contains("second"), "{message}");
    home.assert_unchanged(&before);

    home.close_room(CHIEF, "trial");
    home.close_room("second-room-chief", "second");
    let none = home.terminal_default(None, &["inbox"]);
    terminal_refused(&none, "room_required");
    let message = &none.stderr;
    assert!(message.contains("no room is active"), "{message}");
}

#[test]
fn explicit_room_options_precede_the_default_and_named_seats_still_need_a_room() {
    let home = Home::new();
    home.apply_roster("trial");
    let trial = home.send_to_admiral(CHIEF, "trial", "trial-1", "From trial.");

    let joined = home.piped(
        Some("second-room-chief"),
        &["join", "chief", "--room", "second"],
    );
    assert!(joined.status.success(), "{joined:?}");
    home.apply_roster("second");
    let second = home.send_to_admiral("second-room-chief", "second", "second-1", "From second.");

    let from_environment =
        json_of(&home.terminal_default_with_room_env(None, "second", &["inbox"]));
    assert_eq!(
        from_environment["messages"][0]["receipt"],
        second["receipt"]
    );

    let from_option = json_of(&home.terminal_default_with_room_env(
        None,
        "second",
        &["--room", "trial", "inbox"],
    ));
    assert_eq!(from_option["messages"][0]["receipt"], trial["receipt"]);

    let named = home.terminal_default(None, &["--as", "chief", "inbox"]);
    terminal_refused(&named, "room_required");
    assert!(named.stderr.contains("pass --room"));
}

#[test]
fn a_script_or_an_unjoined_session_still_needs_a_seat_and_an_explicit_seat_is_unchanged() {
    let home = Home::new();
    refused(
        &home.piped(None, &["--room", "trial", "inbox"]),
        "seat_required",
    );
    refused(
        &home.piped(
            None,
            &[
                "--room", "trial", "send", "captain", "--id", "x", "--body", "hi",
            ],
        ),
        "seat_required",
    );
    terminal_refused(
        &home.terminal(Some("stranger"), &["inbox"]),
        "seat_required",
    );

    let before = home.audit().len();
    home.ok(
        CAPTAIN,
        &["send", "chief", "--id", "c-1", "--body", "Report."],
    );
    let chief = json_of(&home.terminal(None, &["--as", "chief", "inbox"]));
    assert_eq!(chief["messages"][0]["sender"], "captain");
    assert!(chief["messages"][0].get("terminal").is_none());
    let events = home.audit();
    assert_eq!(
        events.len(),
        before + 1,
        "only the send is audited: {events:?}"
    );
    assert!(events.iter().all(|event| event.get("caller").is_none()));
}

#[test]
fn no_join_session_or_roster_takes_the_admiral_name() {
    let home = Home::new();
    refused(
        &home.piped(None, &["join", "admiral", "--room", "trial"]),
        "reserved_seat",
    );
    refused(
        &home.piped(Some("fresh"), &["join", "admiral", "--room", "trial"]),
        "reserved_seat",
    );
    terminal_refused(
        &home.terminal(Some(CHIEF), &["--as", "admiral", "inbox"]),
        "reserved_seat",
    );
    refused(
        &home.piped(Some(CHIEF), &["--room", "trial", "--as", "admiral", "who"]),
        "reserved_seat",
    );

    let mut check = home
        .command(None)
        .args(["roster", "check", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let roster = json!({
        "version": 2,
        "seats": [{"name": "admiral", "role": "chief", "host": "claude-desktop"}],
        "routes": []
    });
    check
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&roster).unwrap())
        .unwrap();
    let out = check.wait_with_output().unwrap();
    assert_eq!(code(&out.stderr), "reserved_seat", "{out:?}");
}

#[test]
fn only_the_chief_mails_the_admiral_and_he_reads_and_acknowledges_it() {
    let home = Home::new();
    let send = [
        "send",
        "admiral",
        "--id",
        "a-1",
        "--body",
        "A decision waits.",
    ];
    refused(
        &home.piped(Some(CHIEF), &[&["--room", "trial"][..], &send].concat()),
        "no_roster",
    );

    home.apply_roster("trial");
    let out = home.piped(Some(CAPTAIN), &[&["--room", "trial"][..], &send].concat());
    refused(&out, "admiral_via_chief");
    assert!(String::from_utf8_lossy(&out.stderr).contains("formation request open"));
    assert_eq!(home.ok(CAPTAIN, &["sent"])["total"], 0);

    let sent = home.ok(CHIEF, &send);
    let inbox = json_of(&home.terminal(None, &["inbox"]));
    let messages = inbox["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["sender"], "chief");
    assert_eq!(messages[0]["receipt"], sent["receipt"]);
    let row = admiral_row(&home.ok(CAPTAIN, &["who"]));
    assert_eq!(
        (&row["unacknowledged"], &row["held"]),
        (&json!(1), &Value::Null)
    );

    let receipt = sent["receipt"].to_string();
    let ack = json_of(&home.terminal(None, &["ack", &receipt]));
    assert_eq!(ack["acknowledged"], sent["receipt"]);
    assert_eq!(
        json_of(&home.terminal(None, &["inbox"]))["messages"],
        json!([])
    );
    let ack_event = home
        .audit()
        .into_iter()
        .find(|event| event["command"] == "mail ack")
        .unwrap();
    assert_eq!(
        (&ack_event["caller"], &ack_event["terminal"]),
        (&json!("admiral"), &json!(true))
    );
}

#[test]
fn mail_from_the_terminal_is_the_admirals_and_the_hook_says_it_came_from_a_terminal() {
    let home = Home::new();
    let sent = json_of(&home.terminal(
        None,
        &["send", "captain", "--id", "go-1", "--body", "Go ahead."],
    ));
    let event = home.audit().pop().unwrap();
    assert_eq!(
        (&event["command"], &event["caller"]),
        (&json!("mail send"), &json!("admiral"))
    );
    let events = home.audit().len();
    let history = json_of(&home.terminal(None, &["sent"]));
    assert_eq!(history["total"], 1);
    assert_eq!(home.audit().len(), events, "`mail sent` only reads");
    let original = sent["receipt"].to_string();
    let reply = json_of(&home.terminal(
        None,
        &[
            "send",
            "captain",
            "--id",
            "go-2",
            "--body",
            "Start now.",
            "--re",
            &original,
        ],
    ));

    let mut hook = home
        .command(None)
        .args(["hook", "--provider", "codex", "--event", "user-prompt"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({
        "session_id": CAPTAIN,
        "hook_event_name": "UserPromptSubmit",
        "prompt": "Check the build.",
        "cwd": "/project",
    });
    serde_json::to_writer(hook.stdin.take().unwrap(), &input).unwrap();
    let out = hook.wait_with_output().unwrap();
    assert!(out.status.success(), "{out:?}");
    let document: Value = serde_json::from_slice(&out.stdout).unwrap();
    let text = document["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();

    let inbox = home.ok(CAPTAIN, &["inbox"]);
    let mail = &inbox["messages"][0];
    assert_eq!(mail["sender"], "admiral");
    assert_eq!(mail["terminal"], true);
    assert_eq!(mail["receipt"], sent["receipt"]);
    let stamp = mail["sent"].as_str().unwrap();
    assert!(
        text.contains(&format!(
            "\nFrom admiral, sent from a terminal at {stamp}, receipt {}, id go-1:\nGo ahead.",
            sent["receipt"]
        )),
        "{text}"
    );
    let reply_stamp = inbox["messages"][1]["sent"].as_str().unwrap();
    assert!(
        text.ends_with(&format!(
            "\nFrom admiral, sent from a terminal at {reply_stamp}, receipt {}, id go-2:\n\
             In reply to receipt {original}\nStart now.",
            reply["receipt"]
        )),
        "{text}"
    );

    let mut human = home.command(Some(CAPTAIN));
    human.args(["mail", "--room", "trial", "inbox"]);
    let human = support::on_terminal(human);
    assert!(
        human.stdout.starts_with(&format!(
            "Receipt {} from admiral, sent from a terminal (id go-1)\n",
            sent["receipt"]
        )),
        "{}",
        human.stdout
    );
}

#[test]
fn replies_are_authorized_and_part_of_the_idempotency_key() {
    let home = Home::new();
    let first = home.ok(
        CHIEF,
        &["send", "chief", "--id", "private", "--body", "First."],
    );
    let second = home.ok(
        CHIEF,
        &["send", "captain", "--id", "shared", "--body", "Second."],
    );
    let private = first["receipt"].to_string();
    let shared = second["receipt"].to_string();

    for receipt in [private.as_str(), "9999"] {
        let failure = home.piped(
            Some(CAPTAIN),
            &[
                "--room", "trial", "--json", "send", "chief", "--id", "reply", "--body", "Reply.",
                "--re", receipt,
            ],
        );
        assert_eq!(failure.status.code(), Some(1), "{failure:?}");
        assert!(failure.stdout.is_empty());
        let error: Value = serde_json::from_slice(&failure.stderr).unwrap();
        assert_eq!(error["code"], "invalid_reply_receipt");
    }
    assert_eq!(
        json_of(&home.terminal(None, &["log"]))["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let args = [
        "send", "chief", "--id", "reply", "--body", "Reply.", "--re", &shared,
    ];
    let reply = home.ok(CAPTAIN, &args);
    let receipt = reply["receipt"].to_string();
    home.ok(CHIEF, &["ack", &receipt]);
    assert_eq!(home.ok(CAPTAIN, &args)["receipt"], reply["receipt"]);
    let conflict = home.piped(
        Some(CAPTAIN),
        &[
            "--room", "trial", "--json", "send", "chief", "--id", "reply", "--body", "Reply.",
            "--re", &receipt,
        ],
    );
    let error: Value = serde_json::from_slice(&conflict.stderr).unwrap();
    assert_eq!(error["code"], "message_conflict");
    assert_eq!(
        json_of(&home.terminal(None, &["log"]))["messages"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    json_of(&home.terminal(
        None,
        &[
            "send",
            "captain",
            "--id",
            "admiral-reply",
            "--body",
            "Approved.",
            "--re",
            &private,
        ],
    ));
}

#[test]
fn thread_views_count_only_visible_envelopes_and_keep_their_order() {
    let home = Home::new();
    let other = home.piped(
        Some("observer-session"),
        &["join", "observer", "--room", "trial"],
    );
    assert!(other.status.success(), "{other:?}");
    home.ok(
        CHIEF,
        &[
            "send",
            "captain",
            "--id",
            "first",
            "--body",
            "First line\nMore",
        ],
    );
    home.ok(
        CAPTAIN,
        &[
            "send",
            "observer",
            "--id",
            "second",
            "--body",
            "Hidden from chief",
            "--re",
            "1",
        ],
    );
    home.ok(
        CAPTAIN,
        &["send", "observer", "--id", "plain", "--body", "Unrelated"],
    );
    home.ok(
        "observer-session",
        &[
            "send", "chief", "--id", "last", "--body", "Last", "--re", "2",
        ],
    );
    home.sql("UPDATE messages SET sent = receipt * 10;");
    home.ok(CHIEF, &["ack", "4"]);
    let before = home.snapshot();

    let all = json_of(&home.terminal(None, &["log", "--threads"]));
    assert_eq!(all["total"], 2);
    let thread = &all["threads"][0];
    assert_eq!(thread["count"], 3);
    assert_eq!(thread["latest"], "1970-01-01T00:00:40Z");
    assert_eq!(thread["first_line"], "First line");
    assert_eq!(thread["state"], "acked");
    assert_eq!(thread["seats"], json!(["captain", "chief", "observer"]));
    assert_eq!(all["threads"][1]["count"], 1);
    let limited = json_of(&home.terminal(None, &["log", "--threads", "--limit", "1"]));
    assert_eq!(limited["total"], 2);
    assert_eq!(limited["truncated"], true);
    assert_eq!(limited["threads"], json!([thread]));

    let chief = home.ok(CHIEF, &["log", "--threads"]);
    let visible = &chief["threads"][0];
    assert_eq!(chief["total"], 1);
    assert_eq!(visible["count"], 2);
    assert_eq!(visible["messages"][0]["receipt"], 1);
    assert_eq!(visible["messages"][1]["receipt"], 4);
    for receipt in ["1", "4"] {
        let shown = home.ok(CHIEF, &["show", receipt]);
        assert_eq!(shown["thread"], visible["messages"]);
        assert_eq!(shown["message"]["receipt"].to_string(), receipt);
    }
    home.assert_unchanged(&before);
}

#[test]
fn mail_from_the_previous_schema_reads_as_singletons_and_can_receive_a_reply() {
    let home = Home::new();
    home.ok(
        CHIEF,
        &["send", "captain", "--id", "old-first", "--body", "One"],
    );
    home.ok(
        CHIEF,
        &["send", "captain", "--id", "old-second", "--body", "Two"],
    );
    home.sql("ALTER TABLE messages DROP COLUMN reply_to; PRAGMA user_version = 23;");
    // Restore the WAL companions that a Formation writer leaves in place.
    let database = home.database();
    let store = formation_store::Store::open(database.parent().unwrap()).unwrap();
    store.read().unwrap();

    let files = home.snapshot();
    let before = home.ok(CAPTAIN, &["log", "--threads"]);
    assert_eq!(before["total"], 2);
    assert_eq!(before["threads"][0]["count"], 1);
    assert_eq!(before["threads"][1]["count"], 1);
    assert_eq!(before["threads"][1]["messages"][0]["reply_to"], Value::Null);
    home.assert_unchanged(&files);
    home.ok(
        CAPTAIN,
        &[
            "send", "chief", "--id", "new", "--body", "Three", "--re", "1",
        ],
    );
    let after = home.ok(CAPTAIN, &["log", "--threads"]);
    assert_eq!(after["total"], 2);
    assert_eq!(after["threads"][0]["count"], 2);
    assert_eq!(after["threads"][0]["messages"][0]["body"], "One");
    assert_eq!(after["threads"][0]["messages"][1]["reply_to"], 1);
}

#[test]
fn a_room_from_the_current_release_lists_the_admiral_once_opened() {
    let home = Home::new();
    // Undo the admiral steps, so the file is what release 19 wrote.
    home.sql(
        "ALTER TABLE messages DROP COLUMN reply_to;
         ALTER TABLE messages DROP COLUMN request_id;
         DROP TABLE room_house_catalog;
         ALTER TABLE room_grant DROP COLUMN origin;
         ALTER TABLE messages DROP COLUMN terminal;
         DELETE FROM seats WHERE name = 'admiral';
         PRAGMA user_version = 19;",
    );

    let who = home.ok(CHIEF, &["who"]);
    assert_eq!(admiral_row(&who), empty_admiral());
    let mut text = home.command(Some(CHIEF));
    text.args(["mail", "--room", "trial", "who"]);
    let text = support::on_terminal(text);
    assert!(text.status.success(), "{}", text.stderr);
    let row: Vec<&str> = text
        .stdout
        .lines()
        .nth(1)
        .unwrap()
        .split_whitespace()
        .collect();
    assert_eq!(row, ["admiral", "0", "0", "-", "-"]);
}

#[test]
fn a_passed_request_lands_in_the_admirals_mailbox_and_terminal_settlement_clears_both() {
    let home = Home::new();
    home.apply_roster("trial");

    // Plain chief mail stays unacknowledged when a passed request is settled.
    home.ok(
        CHIEF,
        &[
            "send",
            "admiral",
            "--id",
            "plain-1",
            "--body",
            "Shift update.",
        ],
    );

    let opened = home.request_ok(CAPTAIN, OPEN_REQUEST);
    let id = opened["id"].as_str().unwrap();
    assert_eq!(opened["holder"], "chief");

    let passed = home.request_ok(CHIEF, &["pass", id, "--recommend", "a"]);
    assert_eq!(passed["holder"], "admiral");
    assert_eq!(passed["holder_kind"], "admiral");

    let inbox = json_of(&home.terminal(None, &["inbox"]));
    let messages = inbox["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2, "{messages:?}");
    let req_mail: Vec<_> = messages
        .iter()
        .filter(|message| message["body"].as_str().unwrap().contains(id))
        .collect();
    assert_eq!(req_mail.len(), 1, "{messages:?}");
    assert_eq!(req_mail[0]["sender"], "chief");

    let waiting = json_of(&home.terminal_cmd(
        None,
        &[
            "request",
            "--room",
            "trial",
            "--json",
            "list",
            "--waiting",
            "admiral",
        ],
    ));
    let waiting_rows = waiting.as_array().unwrap();
    assert_eq!(waiting_rows.len(), 1);
    assert_eq!(waiting_rows[0]["id"], id);

    // Advising an admiral-held request from a non-chief seat succeeds without mailing admiral.
    home.request_ok(
        CAPTAIN,
        &["advise", id, "--recommend", "b", "--note", "Extra context"],
    );
    assert_eq!(
        json_of(&home.terminal(None, &["inbox"]))["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Settling from the terminal needs no grant and no `--source`.
    let settled = json_of(&home.terminal_cmd(
        None,
        &[
            "request", "--room", "trial", "--json", "settle", id, "--option", "a",
        ],
    ));
    assert_eq!(settled["state"], "settled");
    assert_eq!(settled["settlement"]["seat"], "admiral");
    assert_eq!(settled["settlement"]["option"], "a");
    assert_eq!(settled["settlement"]["grant_id"], Value::Null);
    assert_eq!(settled["settlement"]["terminal"], true);
    assert!(settled["settlement"].get("source").is_none());
    assert!(settled["settlement"].get("entered_by").is_none());

    let shown_json =
        json_of(&home.terminal_cmd(None, &["request", "--room", "trial", "--json", "show", id]));
    assert_eq!(shown_json["settlement"]["seat"], "admiral");
    assert_eq!(shown_json["settlement"]["terminal"], true);

    let shown_text = home.terminal_cmd(None, &["request", "--room", "trial", "show", id]);
    assert!(shown_text.status.success(), "{}", shown_text.stderr);
    assert!(
        shown_text
            .stdout
            .contains("Settled by admiral (from a terminal) at "),
        "{}",
        shown_text.stdout
    );
    assert!(
        shown_text.stdout.contains(": a (grant admiral)"),
        "{}",
        shown_text.stdout
    );

    // The request's envelope is gone from the admiral's inbox; plain mail stays.
    let inbox_after = json_of(&home.terminal(None, &["inbox"]));
    let after_messages = inbox_after["messages"].as_array().unwrap();
    assert_eq!(after_messages.len(), 1, "{after_messages:?}");
    assert_eq!(after_messages[0]["id"], "plain-1");

    let log = json_of(&home.terminal(None, &["log"]));
    let messages = log["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert!(
        messages[..3]
            .iter()
            .all(|message| message["request_id"] == id)
    );
    assert_eq!(messages[3]["request_id"], Value::Null);
    let threads = json_of(&home.terminal(None, &["log", "--threads"]));
    assert_eq!(threads["total"], 2);
    assert_eq!(threads["threads"][0]["count"], 3);
    assert_eq!(threads["threads"][1]["count"], 1);

    let waiting_after = json_of(&home.terminal_cmd(
        None,
        &[
            "request",
            "--room",
            "trial",
            "--json",
            "list",
            "--waiting",
            "admiral",
        ],
    ));
    assert_eq!(waiting_after, json!([]));

    // The asker receives one terminal-marked envelope from the admiral.
    let captain_inbox = home.ok(CAPTAIN, &["inbox"]);
    let notices: Vec<_> = captain_inbox["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["body"].as_str().unwrap().contains(id))
        .collect();
    assert_eq!(notices.len(), 1, "{captain_inbox}");
    assert_eq!(notices[0]["sender"], "admiral");
    assert_eq!(notices[0]["terminal"], true);

    let settle_event = home
        .audit()
        .into_iter()
        .rev()
        .find(|event| event["command"] == "request settle")
        .unwrap();
    assert_eq!(settle_event["caller"], "admiral");
    assert_eq!(settle_event["terminal"], true);
    assert_eq!(settle_event["target"], id);
}

#[test]
fn request_and_grant_terminal_commands_use_the_only_active_room() {
    let home = Home::new();
    home.apply_roster("trial");

    let opened = home.request_ok(CAPTAIN, OPEN_REQUEST);
    let id = opened["id"].as_str().unwrap();
    home.request_ok(CHIEF, &["pass", id]);

    let waiting = home.terminal_cmd(None, &["request", "--json", "list", "--waiting", "admiral"]);
    assert!(waiting.status.success(), "{}", waiting.stderr);
    let requests: Value = serde_json::from_str(&waiting.stdout).unwrap();
    assert_eq!(requests.as_array().unwrap().len(), 1, "{requests}");
    assert_eq!(requests[0]["id"], id);

    let grant = home.terminal_cmd(
        None,
        &[
            "grant",
            "add",
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
        ],
    );
    assert!(grant.status.success(), "{}", grant.stderr);
    let grant: Value = serde_json::from_str(&grant.stdout).unwrap();
    assert_eq!(grant["source"], "admiral");
}

#[test]
fn a_non_chief_request_without_a_parent_does_not_mail_the_admiral() {
    let home = Home::new();
    let worker = home.piped(
        Some("worker-session"),
        &["join", "worker", "--room", "trial"],
    );
    assert!(worker.status.success(), "{worker:?}");
    home.apply_roster_value(
        "trial",
        &json!({
            "version": 2,
            "seats": [
                {"name": "chief", "role": "chief", "host": "claude-desktop"},
                {"name": "worker", "role": "worker", "host": "headless"}
            ],
            "routes": []
        }),
    );

    let opened = home.request_ok("worker-session", OPEN_REQUEST);
    assert_eq!(opened["state"], "open");

    let inbox = json_of(&home.terminal(None, &["inbox"]));
    assert_eq!(inbox["messages"], json!([]));
}

#[test]
fn terminal_settlement_refuses_chat_only_admiral_source_flags() {
    let home = Home::new();
    home.apply_roster("trial");

    let opened = home.request_ok(CAPTAIN, OPEN_REQUEST);
    let id = opened["id"].as_str().unwrap();
    home.request_ok(CHIEF, &["pass", id]);

    terminal_refused(
        &home.terminal_cmd(
            None,
            &[
                "request",
                "--room",
                "trial",
                "--json",
                "settle",
                id,
                "--option",
                "a",
                "--admiral",
                "--source",
                "chief chat 12:00",
            ],
        ),
        "source_from_terminal",
    );

    let still_open = home.request_ok(CHIEF, &["show", id]);
    assert_eq!(still_open["state"], "open");
    assert_eq!(still_open["holder"], "admiral");
    let inbox = json_of(&home.terminal(None, &["inbox"]));
    assert_eq!(inbox["messages"].as_array().unwrap().len(), 1);
}

#[test]
fn chat_settlement_from_a_joined_seat_keeps_source_and_clears_the_admirals_inbox() {
    let home = Home::new();
    home.apply_roster("trial");

    let opened = home.request_ok(CAPTAIN, OPEN_REQUEST);
    let id = opened["id"].as_str().unwrap();
    home.request_ok(CHIEF, &["pass", id]);
    assert_eq!(
        json_of(&home.terminal(None, &["inbox"]))["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let settled = home.request_ok(
        CHIEF,
        &[
            "settle",
            id,
            "--option",
            "b",
            "--admiral",
            "--source",
            "chief chat 14:00",
        ],
    );
    assert_eq!(settled["settlement"]["seat"], "admiral");
    assert_eq!(settled["settlement"]["source"], "chief chat 14:00");
    assert_eq!(settled["settlement"]["entered_by"], "chief");
    assert!(settled["settlement"].get("terminal").is_none());

    let shown = home.terminal_cmd(None, &["request", "--room", "trial", "show", id]);
    assert!(shown.status.success(), "{}", shown.stderr);
    assert!(shown.stdout.contains("Source: chief chat 14:00"));
    assert!(shown.stdout.contains("Entered by: chief"));
    assert!(!shown.stdout.contains("from a terminal"));

    // Settling through chat also clears the admiral's inbox envelope and waiting queue.
    assert_eq!(
        json_of(&home.terminal(None, &["inbox"]))["messages"],
        json!([])
    );
    assert_eq!(
        home.request_ok(CHIEF, &["list", "--waiting", "admiral"]),
        json!([])
    );

    let captain_inbox = home.ok(CAPTAIN, &["inbox"]);
    let notice = &captain_inbox["messages"][0];
    assert_eq!(notice["sender"], "admiral");
    assert!(notice.get("terminal").is_none());
}

#[test]
fn terminal_reopen_overturns_a_seats_settlement_and_mails_both_seats() {
    let home = Home::new();
    home.apply_roster("trial");

    // Grant design to chief from the terminal, then let chief settle captain's request.
    json_of(&home.terminal_cmd(
        None,
        &[
            "grant",
            "add",
            "--room",
            "trial",
            "--to",
            "chief",
            "--class",
            "design",
            "--workspace",
            "all",
        ],
    ));
    let opened = home.request_ok(CAPTAIN, OPEN_REQUEST);
    let id = opened["id"].as_str().unwrap();
    home.request_ok(CHIEF, &["settle", id, "--option", "a"]);

    let reopened = json_of(&home.terminal_cmd(
        None,
        &["request", "--room", "trial", "--json", "reopen", id],
    ));
    assert_eq!(reopened["state"], "open");
    assert_eq!(reopened["holder"], "admiral");
    assert_eq!(reopened["overturned"][0]["seat"], "chief");
    assert_eq!(reopened["overturned"][0]["option"], "a");

    for seat_session in [CAPTAIN, CHIEF] {
        let inbox = home.ok(seat_session, &["inbox"]);
        let reopened_mail: Vec<_> = inbox["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|message| {
                message["sender"] == "admiral"
                    && message["body"].as_str().unwrap().contains("reopened")
                    && message["body"].as_str().unwrap().contains(id)
            })
            .collect();
        assert_eq!(reopened_mail.len(), 1, "{seat_session}: {inbox}");
        assert_eq!(reopened_mail[0]["terminal"], true);
    }

    let reopen_event = home
        .audit()
        .into_iter()
        .rev()
        .find(|event| event["command"] == "request reopen")
        .unwrap();
    assert_eq!(reopen_event["caller"], "admiral");
    assert_eq!(reopen_event["terminal"], true);
    assert_eq!(reopen_event["target"], id);
}

#[test]
fn grants_from_the_terminal_use_admiral_without_self_entered_and_sessions_cannot_impersonate() {
    let home = Home::new();
    home.apply_roster("trial");

    let added = json_of(&home.terminal_cmd(
        None,
        &[
            "grant",
            "add",
            "--room",
            "trial",
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
        ],
    ));
    let grant_id = added["id"].as_i64().unwrap();
    assert_eq!(added["entered_by"], "admiral");
    assert_eq!(added["source"], "admiral");
    assert_eq!(added["self_entered"], false);
    assert_eq!(added["notified"], json!(["chief"]));

    let listed = json_of(&home.terminal_cmd(None, &["grant", "list", "--room", "trial", "--json"]));
    assert_eq!(listed["grants"][0]["source"], "admiral");
    assert_eq!(listed["grants"][0]["entered_by"], "admiral");
    assert_eq!(listed["grants"][0]["self_entered"], false);

    let table = home.terminal_cmd(None, &["grant", "list", "--room", "trial"]);
    assert!(table.status.success(), "{}", table.stderr);
    assert!(table.stdout.contains("admiral"), "{}", table.stdout);
    assert!(!table.stdout.contains("self-entered"), "{}", table.stdout);

    let chief_mail = home.ok(CHIEF, &["inbox"]);
    assert_eq!(chief_mail["messages"][0]["sender"], "admiral");
    assert_eq!(chief_mail["messages"][0]["terminal"], true);

    let withheld = json_of(&home.terminal_cmd(
        None,
        &[
            "grant",
            "withhold",
            "--room",
            "trial",
            "--to",
            "captain",
            "--class",
            "land",
            "--workspace",
            "site",
        ],
    ));
    assert_eq!(withheld["entered_by"], "admiral");
    assert_eq!(withheld["source"], "admiral");
    assert_eq!(withheld["self_entered"], false);

    let id_text = grant_id.to_string();
    let revoked =
        json_of(&home.terminal_cmd(None, &["grant", "revoke", &id_text, "--room", "trial"]));
    assert_eq!(revoked["revoked"], true);
    assert_eq!(revoked["revoked_by"], "admiral");

    let events = home.audit();
    for command in ["grant add", "grant withhold", "grant revoke"] {
        let event = events
            .iter()
            .find(|event| event["command"] == command)
            .unwrap();
        assert_eq!(event["caller"], "admiral", "{command}");
        assert_eq!(event["terminal"], true, "{command}");
    }

    // Sessions cannot impersonate admiral, and unjoined sessions stay refused on a terminal.
    terminal_refused(
        &home.terminal_cmd(
            Some(CHIEF),
            &[
                "grant",
                "add",
                "--room",
                "trial",
                "--as",
                "admiral",
                "--to",
                "chief",
                "--class",
                "direction",
            ],
        ),
        "reserved_seat",
    );
    terminal_refused(
        &home.terminal_cmd(
            Some("stranger"),
            &[
                "grant",
                "add",
                "--room",
                "trial",
                "--to",
                "chief",
                "--class",
                "direction",
            ],
        ),
        "seat_required",
    );
    terminal_refused(
        &home.terminal_cmd(
            Some(CHIEF),
            &[
                "request", "--room", "trial", "--as", "admiral", "--json", "list",
            ],
        ),
        "reserved_seat",
    );
}
