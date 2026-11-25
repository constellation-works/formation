//! Local-room contracts through independent CLI processes.
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

use cw_testkit::TempRoot;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

struct Room(TempRoot);

impl Room {
    fn new() -> Self {
        Self(TempRoot::new("FORMATION").unwrap())
    }

    fn command(&self, seat: &str) -> Command {
        let mut cmd = self.0.command(env!("CARGO_BIN_EXE_formation"));
        cmd.arg("mail")
            .arg("--room")
            .arg(self.0.path())
            .arg("--as")
            .arg(seat);
        cmd
    }

    fn run(&self, seat: &str, args: &[&str]) -> Output {
        self.command(seat).args(args).output().unwrap()
    }

    fn ok(&self, seat: &str, args: &[&str]) -> Value {
        let out = self.run(seat, args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

fn formation_with_home(root: &TempRoot, home: &Path) -> Command {
    let mut cmd = root.command(env!("CARGO_BIN_EXE_formation"));
    cmd.env("FORMATION_HOME", home);
    cmd
}

fn run_ok(cmd: &mut Command) -> Value {
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn expect_refusal(out: &Output, expected_code: &str, flag_hint: Option<&str>) {
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr.clone()).unwrap();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "expected one line on stderr, got: {stderr}");
    let body: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(body["code"], expected_code);
    if let Some(hint) = flag_hint {
        let message = body["error"].as_str().unwrap();
        assert!(
            message.contains(hint),
            "expected {hint} in error message: {message}"
        );
    }
}

#[test]
fn recipient_owns_ack_and_retries_preserve_receipts() {
    let room = Room::new();
    room.ok("chief", &["join", "chief"]);
    room.ok("captain", &["join", "captain"]);
    let names: Vec<Value> = room.ok("chief", &["who"])["seats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|seat| seat["name"].clone())
        .collect();
    assert_eq!(names, ["captain", "chief"]);
    let send = ["send", "captain", "--id", "alert-1", "--body", "a probe"];
    let receipt = room.ok("chief", &send);
    assert_eq!(room.ok("chief", &send), receipt);
    assert!(
        !room
            .run(
                "chief",
                &["send", "captain", "--id", "alert-1", "--body", "different"]
            )
            .status
            .success()
    );
    assert!(
        !room
            .run(
                "chief",
                &["send", "missing", "--id", "absent", "--body", "hello"]
            )
            .status
            .success()
    );
    let id = receipt["receipt"].to_string();
    assert!(!room.run("chief", &["ack", &id]).status.success());
    let read = room.ok("captain", &["inbox"]);
    assert_eq!(read["messages"][0]["body"], "a probe");
    assert_eq!(read, room.ok("captain", &["inbox"]));
    room.ok("captain", &["ack", &id]);
    room.ok("captain", &["ack", &id]);
    assert_eq!(room.ok("chief", &send)["receipt"], receipt["receipt"]);
    assert_eq!(
        room.ok("captain", &["inbox"])["messages"],
        serde_json::json!([])
    );
}

#[test]
fn mail_waits_for_an_absent_seat_and_reports_whether_it_is_held() {
    use std::process::Stdio;
    let room = Room::new();
    room.ok("chief", &["join", "chief"]);
    room.ok("captain", &["join", "captain"]);

    let absent = room.ok(
        "chief",
        &["send", "captain", "--id", "m-1", "--body", "while away"],
    );
    assert_eq!(absent["held"], false);
    let inbox = room.ok("captain", &["inbox"]);
    assert_eq!(inbox["messages"][0]["body"], "while away");
    let sent = inbox["messages"][0]["sent"].as_str().unwrap();
    assert!(is_rfc3339_utc(sent), "sent is not RFC 3339 UTC: {sent}");
    room.ok("captain", &["ack", &absent["receipt"].to_string()]);

    let waiter = room
        .command("captain")
        .args(["inbox", "--wait", "10"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    while !held(&room.ok("chief", &["who"]), "captain") {
        assert!(start.elapsed().as_secs() < 8, "waiter never held its seat");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let waiting = room.ok(
        "chief",
        &["send", "captain", "--id", "m-2", "--body", "while waiting"],
    );
    assert_eq!(waiting["held"], true);
    let read: Value = {
        let out = waiter.wait_with_output().unwrap();
        assert!(out.status.success());
        serde_json::from_slice(&out.stdout).unwrap()
    };
    assert_eq!(read["messages"][0]["id"], "m-2");
}

fn is_rfc3339_utc(text: &str) -> bool {
    let shape = b"dddd-dd-ddTdd:dd:ddZ";
    text.len() == shape.len()
        && text.bytes().zip(shape).all(|(got, want)| match want {
            b'd' => got.is_ascii_digit(),
            _ => got == *want,
        })
}

fn held(who: &Value, name: &str) -> bool {
    who["seats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|seat| seat["name"] == name)
        .unwrap()["held"]
        == true
}

#[test]
fn send_to_a_name_nobody_joined_lists_the_joined_seats() {
    let room = Room::new();
    room.ok("chief", &["join", "chief"]);
    room.ok("captain", &["join", "captain"]);
    let out = room.run(
        "chief",
        &["send", "ghost", "--id", "g-1", "--body", "anyone there"],
    );
    expect_refusal(&out, "unknown_recipient", Some("captain, chief"));
}

#[test]
fn who_lists_every_joined_seat_with_unread_held_and_seen() {
    let room = Room::new();
    room.ok("chief", &["join", "chief"]);
    room.ok("captain", &["join", "captain"]);
    room.ok(
        "chief",
        &["send", "captain", "--id", "w-1", "--body", "pending"],
    );

    let who = room.ok("chief", &["who"]);
    let seats = who["seats"].as_array().unwrap();
    assert_eq!(seats.len(), 2);
    assert_eq!(seats[0]["name"], "captain");
    assert_eq!(seats[0]["unread"], 1);
    assert_eq!(seats[0]["held"], false);
    assert_eq!(seats[0]["seen"], Value::Null);
    assert_eq!(seats[1]["name"], "chief");
    assert_eq!(seats[1]["unread"], 0);
    assert_eq!(seats[1]["held"], true);
    assert!(is_rfc3339_utc(seats[1]["seen"].as_str().unwrap()));
}

#[test]
fn concurrent_duplicate_sends_wake_a_waiter_once() {
    use std::process::Stdio;
    let room = Room::new();
    room.ok("chief", &["join", "chief"]);
    room.ok("captain", &["join", "captain"]);
    let sentry = room
        .command("captain")
        .args(["inbox", "--wait", "5"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut sends = Vec::new();
    for _ in 0..4 {
        sends.push(
            room.command("chief")
                .args(["send", "captain", "--id", "concurrent", "--body", "hello"])
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    for send in sends {
        let out = send.wait_with_output().unwrap();
        assert!(out.status.success());
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["receipt"], 1);
    }
    let output = sentry.wait_with_output().unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["messages"].as_array().unwrap().len(), 1);
    assert_eq!(value["messages"][0]["id"], "concurrent");
}

#[test]
fn named_room_mail_flow_works_with_as_and_no_key_file() {
    let root = TempRoot::new("FORMATION").unwrap();
    let home = root.path().join("formation-home");

    let joined_chief = run_ok(
        formation_with_home(&root, &home).args(["mail", "join", "chief", "--room", "trial"]),
    );
    assert_eq!(joined_chief["seat"], "chief");
    assert_eq!(joined_chief["rebound"], false);

    run_ok(formation_with_home(&root, &home).args(["mail", "join", "captain", "--room", "trial"]));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let room_dir = home.join("rooms").join("trial");
        let dir_mode = std::fs::metadata(&room_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700);
        let db_mode = std::fs::metadata(room_dir.join("inbox.sqlite3"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(db_mode, 0o600);
    }

    let sent = run_ok(formation_with_home(&root, &home).args([
        "mail", "send", "captain", "--id", "check-1", "--body", "ready", "--room", "trial", "--as",
        "chief",
    ]));
    let receipt = sent["receipt"].to_string();

    let inbox = run_ok(
        formation_with_home(&root, &home)
            .args(["mail", "inbox", "--room", "trial", "--as", "captain"]),
    );
    assert_eq!(inbox["messages"][0]["sender"], "chief");
    assert_eq!(inbox["messages"][0]["body"], "ready");

    let ack = run_ok(formation_with_home(&root, &home).args([
        "mail", "ack", &receipt, "--room", "trial", "--as", "captain",
    ]));
    assert_eq!(ack["acknowledged"], sent["receipt"]);
}

#[test]
fn session_env_resolves_room_and_seat_without_flags() {
    for env_var in ["CLAUDE_CODE_SESSION_ID", "CODEX_THREAD_ID"] {
        let root = TempRoot::new("FORMATION").unwrap();
        let home = root.path().join("formation-home");

        run_ok(
            formation_with_home(&root, &home)
                .env(env_var, "session-chief-1")
                .args(["mail", "join", "chief", "--room", "trial"]),
        );
        run_ok(
            formation_with_home(&root, &home)
                .env(env_var, "session-captain-1")
                .args(["mail", "join", "captain", "--room", "trial"]),
        );
        run_ok(
            formation_with_home(&root, &home)
                .env(env_var, "session-chief-1")
                .args([
                    "mail", "send", "captain", "--id", "note-1", "--body", "ping",
                ]),
        );

        let inbox = run_ok(
            formation_with_home(&root, &home)
                .env(env_var, "session-captain-1")
                .args(["mail", "inbox"]),
        );
        assert_eq!(inbox["messages"].as_array().unwrap().len(), 1);
        assert_eq!(inbox["messages"][0]["sender"], "chief");
        assert_eq!(inbox["messages"][0]["body"], "ping");
    }
}

#[test]
fn missing_room_or_seat_or_unknown_room_refuses_without_creating_files() {
    let root = TempRoot::new("FORMATION").unwrap();
    let home = root.path().join("formation-home");

    let no_room = formation_with_home(&root, &home)
        .args(["mail", "inbox"])
        .output()
        .unwrap();
    expect_refusal(&no_room, "room_required", Some("--room"));
    assert!(!home.exists());

    let missing_room = formation_with_home(&root, &home)
        .args(["mail", "inbox", "--room", "ghost"])
        .output()
        .unwrap();
    expect_refusal(&missing_room, "room_not_found", None);
    assert!(!home.exists());

    run_ok(formation_with_home(&root, &home).args(["mail", "join", "chief", "--room", "trial"]));
    let room_dir = home.join("rooms").join("trial");
    let mut before: Vec<_> = std::fs::read_dir(&room_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    before.sort();

    let no_seat = formation_with_home(&root, &home)
        .args(["mail", "inbox", "--room", "trial"])
        .output()
        .unwrap();
    expect_refusal(&no_seat, "seat_required", Some("--as"));

    let mut after: Vec<_> = std::fs::read_dir(&room_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    after.sort();
    assert_eq!(before, after);
    assert!(!home.join("sessions").exists());
}

#[test]
fn second_session_rebinds_taken_seat_and_evicts_first() {
    let root = TempRoot::new("FORMATION").unwrap();
    let home = root.path().join("formation-home");

    let first = run_ok(
        formation_with_home(&root, &home)
            .env("CLAUDE_CODE_SESSION_ID", "sess-first")
            .args(["mail", "join", "chief", "--room", "trial"]),
    );
    assert_eq!(first["rebound"], false);
    assert_eq!(first["replaced_session"], Value::Null);

    let second = run_ok(
        formation_with_home(&root, &home)
            .env("CLAUDE_CODE_SESSION_ID", "sess-second")
            .args(["mail", "join", "chief", "--room", "trial"]),
    );
    assert_eq!(second["seat"], "chief");
    assert_eq!(second["rebound"], true);
    assert_eq!(second["replaced_session"], "sess-first");

    let evicted = formation_with_home(&root, &home)
        .env("CLAUDE_CODE_SESSION_ID", "sess-first")
        .args(["mail", "inbox"])
        .output()
        .unwrap();
    expect_refusal(&evicted, "seat_required", Some("--as"));

    let active = run_ok(
        formation_with_home(&root, &home)
            .env("CLAUDE_CODE_SESSION_ID", "sess-second")
            .args(["mail", "inbox"]),
    );
    assert_eq!(active["messages"], serde_json::json!([]));
}
