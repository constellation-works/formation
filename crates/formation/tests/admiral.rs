//! The admiral: a terminal call with no agent session, through independent CLI processes.
#![cfg(unix)]
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

#[allow(dead_code, reason = "this crate needs only the terminal helper")]
mod support;

use cw_testkit::TempRoot;
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Output, Stdio};

const CHIEF: &str = "chief-session";

const CAPTAIN: &str = "captain-session";

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
            .env_remove("CODEX_THREAD_ID");
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

    fn ok(&self, session: &str, args: &[&str]) -> Value {
        let mut full = vec!["--room", "trial", "--json"];
        full.extend_from_slice(args);
        let out = self.piped(Some(session), &full);
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn apply_roster(&self) {
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
        let mut child = self
            .command(None)
            .args(["roster", "apply", "--room", "trial"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&roster).unwrap())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{out:?}");
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

    home.apply_roster();
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
        text.ends_with(&format!(
            "\nFrom admiral, sent from a terminal at {stamp}, receipt {}, id go-1:\nGo ahead.",
            sent["receipt"]
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
fn a_room_from_the_current_release_lists_the_admiral_once_opened() {
    let home = Home::new();
    // Undo the admiral steps, so the file is what release 19 wrote.
    home.sql(
        "DROP TABLE room_house_catalog;
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
