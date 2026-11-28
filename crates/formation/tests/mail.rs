//! Local-room contracts through independent CLI processes.
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

#[cfg(unix)]
mod support;

use cw_testkit::TempRoot;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use support::SpawnedChild;

/// A commit that seats `loser` fails: a deferred foreign key is checked at COMMIT, after the
/// join has written its session pointer.
#[cfg(unix)]
const FAIL_LOSER_COMMIT: &str = "\
    CREATE TABLE parent(id INTEGER PRIMARY KEY);
    CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
    CREATE TRIGGER fail_loser AFTER INSERT ON seats WHEN NEW.name = 'loser'
    BEGIN INSERT INTO child VALUES(1); END;";

/// Hold a room's write lock, create the second path once it is held, and let go when the third
/// path appears.
#[cfg(unix)]
const HOLD_WRITE_LOCK: &str = "\
import os, sqlite3, sys, time
db = sqlite3.connect(sys.argv[1], isolation_level=None)
db.execute('BEGIN IMMEDIATE')
open(sys.argv[2], 'w').close()
deadline = time.monotonic() + 30
while not os.path.exists(sys.argv[3]) and time.monotonic() < deadline:
    time.sleep(0.01)
db.execute('ROLLBACK')
";

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

/// One room in a disposable home, driven from provider sessions or by explicit seat.
struct Home {
    root: TempRoot,
    home: std::path::PathBuf,
}

impl Home {
    fn new() -> Self {
        let root = TempRoot::new("FORMATION").unwrap();
        let home = root.path().join("formation-home");
        let fixture = Self { root, home };
        ok(&fixture.join(None, "probe"));
        fixture
    }

    fn room(&self) -> std::path::PathBuf {
        self.home.join("rooms/trial")
    }

    fn seat_dir(&self, seat: &str) -> std::path::PathBuf {
        self.room().join("seats").join(seat)
    }

    fn pointer(&self, session: &str) -> std::path::PathBuf {
        self.home.join("sessions").join(session)
    }

    /// A `mail` command from `session` with the room only for joins.
    fn command(&self, session: Option<&str>, args: &[&str]) -> Command {
        let mut cmd = formation_with_home(&self.root, &self.home);
        cmd.env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("FORMATION_ROOM");
        if let Some(session) = session {
            cmd.env("CODEX_THREAD_ID", session);
        }
        cmd.arg("mail").args(args);
        cmd
    }

    fn mail(&self, session: Option<&str>, args: &[&str]) -> Output {
        self.command(session, args).output().unwrap()
    }

    fn join(&self, session: Option<&str>, seat: &str) -> Output {
        self.mail(session, &["join", seat, "--room", "trial"])
    }

    fn who(&self) -> Vec<String> {
        let out = self.mail(None, &["--room", "trial", "--as", "probe", "who"]);
        let body: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        body["seats"]
            .as_array()
            .map(|seats| {
                seats
                    .iter()
                    .map(|seat| seat["name"].as_str().unwrap().to_owned())
                    .filter(|name| name != "probe")
                    .collect()
            })
            .unwrap_or_default()
    }

    fn pointer_text(&self, session: &str) -> Option<String> {
        std::fs::read_to_string(self.pointer(session)).ok()
    }

    #[cfg(unix)]
    fn sql(&self, room: &str, sql: &str) {
        let out = self
            .root
            .command("python3")
            .args([
                "-c",
                "import sqlite3 as s,sys; s.connect(sys.argv[1]).executescript(sys.argv[2])",
            ])
            .arg(self.home.join("rooms").join(room).join("inbox.sqlite3"))
            .arg(sql)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }

    /// Run a join while the sessions directory takes no new file, so the pointer write fails.
    /// The session must have joined before: only the pointer write then needs a new file.
    #[cfg(unix)]
    fn join_with_unwritable_pointer(&self, session: &str, seat: &str) -> Output {
        let sessions = self.home.join("sessions");
        set_mode(&sessions, 0o500);
        let probe = std::fs::write(sessions.join("probe"), "");
        let out = self.join(Some(session), seat);
        set_mode(&sessions, 0o700);
        assert!(probe.is_err(), "this test needs an unprivileged user");
        out
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

fn assert_who_envelope(value: &Value) {
    let envelope = value.as_object().unwrap();
    assert_eq!(envelope.len(), 1);
    let seats = value["seats"].as_array().unwrap();
    assert_eq!(seats.len(), 2);
    for seat in seats {
        assert_eq!(seat.as_object().unwrap().len(), 5);
    }
    assert_eq!(seats[0]["name"], "captain");
    assert_eq!(seats[0]["undelivered"], 1);
    assert_eq!(seats[0]["unacknowledged"], 1);
    assert_eq!(seats[0]["held"], false);
    assert_eq!(seats[0]["seen"], Value::Null);
    assert_eq!(seats[1]["name"], "chief");
    assert_eq!(seats[1]["undelivered"], 0);
    assert_eq!(seats[1]["unacknowledged"], 0);
    assert_eq!(seats[1]["held"], true);
    assert!(is_rfc3339_utc(seats[1]["seen"].as_str().unwrap()));
}

fn who_without_seen(value: &Value) -> Value {
    let mut stable = value.clone();
    for seat in stable["seats"].as_array_mut().unwrap() {
        seat.as_object_mut().unwrap().remove("seen");
    }
    stable
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
fn who_lists_every_joined_seat_with_undelivered_unacknowledged_held_and_seen() {
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
    assert_eq!(seats[0]["undelivered"], 1);
    assert_eq!(seats[0]["unacknowledged"], 1);
    assert!(seats[0].get("unread").is_none());
    assert_eq!(seats[0]["held"], false);
    assert_eq!(seats[0]["seen"], Value::Null);
    assert_eq!(seats[1]["name"], "chief");
    assert_eq!(seats[1]["undelivered"], 0);
    assert_eq!(seats[1]["unacknowledged"], 0);
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

#[cfg(unix)]
#[test]
fn join_resolves_scribe_seats_and_refuses_missing_or_ambiguous_results() {
    use std::os::unix::fs::PermissionsExt;
    let root = TempRoot::new("FORMATION").unwrap();
    let home = root.path().join("formation-home");
    let bin = root.path().join("scribe-bin");
    std::fs::create_dir(&bin).unwrap();
    let scribe = bin.join("scribe");
    let fixture = |body: &str| {
        std::fs::write(&scribe, format!("#!/bin/sh\nprintf '%s' '{body}'\n")).unwrap();
        std::fs::set_permissions(&scribe, std::fs::Permissions::from_mode(0o755)).unwrap();
    };
    let join = || {
        let mut cmd = formation_with_home(&root, &home);
        cmd.env("PATH", &bin)
            .env("CLAUDE_CODE_SESSION_ID", "session-one")
            .args(["mail", "join"]);
        cmd
    };
    fixture(
        r#"{"items":[{"id":"closed","closed":"yesterday","seats":[{"name":"old","transcripts":["session-one"]}]},{"id":"shift","closed":null,"seats":[{"name":"captain","transcripts":["session-one"]}]}]}"#,
    );
    assert_eq!(run_ok(&mut join())["seat"], "captain");
    assert!(home.join("rooms/shift/inbox.sqlite3").exists());
    fixture(
        r#"{"items":[{"id":"closed","closed":"yesterday","seats":[{"name":"old","transcripts":["session-one"]}]}]}"#,
    );
    expect_refusal(
        &join().output().unwrap(),
        "no_scribe_seat",
        Some("join the scribe group"),
    );
    fixture(
        r#"{"items":[{"id":"one","closed":null,"seats":[{"name":"chief","transcripts":["session-one"]}]},{"id":"two","closed":null,"seats":[{"name":"captain","transcripts":["session-one"]}]}]}"#,
    );
    let ambiguous = join().output().unwrap();
    expect_refusal(&ambiguous, "ambiguous_scribe_seat", Some("one/chief"));
    expect_refusal(&ambiguous, "ambiguous_scribe_seat", Some("two/captain"));
    std::fs::write(&scribe, "#!/bin/sh\nprintf 'failed\\nmore' >&2\nexit 1\n").unwrap();
    let failed = join().output().unwrap();
    expect_refusal(
        &failed,
        "scribe_unavailable",
        Some("formation mail join <seat> --room <room>"),
    );
    std::fs::remove_file(&scribe).unwrap();
    expect_refusal(
        &join().output().unwrap(),
        "scribe_unavailable",
        Some("formation mail join <seat> --room <room>"),
    );
    assert_eq!(
        run_ok(
            formation_with_home(&root, &home)
                .env("PATH", &bin)
                .args(["mail", "join", "captain", "--room", "trial"])
        )["seat"],
        "captain"
    );
}

#[test]
fn join_creates_the_seat_folder_and_reports_it_again_on_rejoin() {
    let root = TempRoot::new("FORMATION").unwrap();
    let home = root.path().join("formation-home");
    let join = || {
        run_ok(
            formation_with_home(&root, &home)
                .env("CLAUDE_CODE_SESSION_ID", "sess-folder")
                .args(["mail", "join", "chief", "--room", "trial"]),
        )
    };

    let first = join();
    let expected = home.join("rooms/trial/seats/chief");
    assert_eq!(first["seat_dir"], expected.to_str().unwrap());
    assert_eq!(first["seat"], "chief");
    assert_eq!(first["rebound"], false);
    assert_eq!(first["replaced_session"], Value::Null);
    assert!(expected.is_dir());

    std::fs::write(expected.join("held"), "kept").unwrap();
    let second = join();
    assert_eq!(second["seat_dir"], first["seat_dir"]);
    assert_eq!(
        std::fs::read_to_string(expected.join("held")).unwrap(),
        "kept"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&expected).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "seat folder mode {mode:o}");
    }
}

#[test]
fn seat_folder_follows_a_room_path() {
    let room = Room::new();
    let joined = room.ok("chief", &["join", "chief"]);
    let expected = room.0.path().join("seats/chief");
    assert_eq!(joined["seat_dir"], expected.to_str().unwrap());
    assert!(expected.is_dir());
}

#[test]
fn join_fails_when_the_seat_folder_cannot_be_created() {
    let room = Room::new();
    std::fs::create_dir_all(room.0.path()).unwrap();
    let blocker = room.0.path().join("seats");
    std::fs::write(&blocker, "not a directory").unwrap();
    let out = room.run("chief", &["join", "chief"]);
    expect_refusal(
        &out,
        "seat_dir_unavailable",
        Some(blocker.to_str().unwrap()),
    );
}

fn ok(out: &Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn refusal_message(out: &Output, code: &str) -> String {
    expect_refusal(out, code, None);
    let line = String::from_utf8(out.stderr.clone()).unwrap();
    let body: Value = serde_json::from_str(line.trim()).unwrap();
    body["error"].as_str().unwrap().to_owned()
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

#[cfg(unix)]
fn legacy_seat_dir(home: &Home, seat: &str) -> std::path::PathBuf {
    let dir = home.seat_dir(seat);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("held"), "kept").unwrap();
    set_mode(&dir, 0o755);
    dir
}

#[cfg(unix)]
#[test]
fn legacy_seat_folder_refuses_with_the_fix_and_joins_once_corrected() {
    let home = Home::new();
    ok(&home.join(Some("seed"), "seed"));
    let dir = legacy_seat_dir(&home, "legacy");

    let message = refusal_message(
        &home.join(Some("sess-legacy"), "legacy"),
        "seat_dir_unavailable",
    );
    assert!(message.contains(dir.to_str().unwrap()), "{message}");
    assert!(message.contains("0755"), "{message}");
    assert!(message.contains("0700"), "{message}");
    assert!(message.contains("chmod -- 700"), "{message}");
    assert!(message.contains("mail join"), "{message}");
    assert_eq!(home.who(), ["seed"]);
    assert_eq!(home.pointer_text("sess-legacy"), None);
    assert_eq!(mode_of(&dir), 0o755);
    assert_eq!(std::fs::read_to_string(dir.join("held")).unwrap(), "kept");

    set_mode(&dir, 0o700);
    let joined = ok(&home.join(Some("sess-legacy"), "legacy"));
    assert_eq!(joined["seat_dir"], dir.to_str().unwrap());
    assert_eq!(home.who(), ["legacy", "seed"]);
    assert_eq!(home.pointer_text("sess-legacy").as_deref(), Some("trial\n"));
    assert_eq!(std::fs::read_to_string(dir.join("held")).unwrap(), "kept");
}

#[cfg(unix)]
#[test]
fn unsafe_seat_folders_stay_refused() {
    use std::os::unix::fs::symlink;
    let home = Home::new();
    ok(&home.join(Some("seed"), "seed"));
    let target = home.root.path().join("elsewhere");
    std::fs::create_dir(&target).unwrap();
    set_mode(&target, 0o700);
    symlink(&target, home.seat_dir("linked")).unwrap();
    let message = refusal_message(
        &home.join(Some("sess-link"), "linked"),
        "seat_dir_unavailable",
    );
    assert!(message.contains("seat folder"), "{message}");
    assert_eq!(home.who(), ["seed"]);
    assert_eq!(home.pointer_text("sess-link"), None);

    let group = home.seat_dir("group");
    std::fs::create_dir_all(&group).unwrap();
    set_mode(&group, 0o770);
    refusal_message(
        &home.join(Some("sess-group"), "group"),
        "seat_dir_unavailable",
    );
    assert_eq!(mode_of(&group), 0o770);
    assert_eq!(home.who(), ["seed"]);
}

#[cfg(unix)]
#[test]
fn failed_pointer_write_leaves_a_new_seat_unjoined_and_retry_succeeds() {
    let home = Home::new();
    ok(&home.join(Some("seed"), "seed"));
    ok(&home.mail(Some("sess-new"), &["join", "away", "--room", "elsewhere"]));

    let out = home.join_with_unwritable_pointer("sess-new", "fresh");
    let message = refusal_message(&out, "mail_file");
    assert!(message.contains("temporary file"), "{message}");
    assert_eq!(home.who(), ["seed"]);
    assert_eq!(
        home.pointer_text("sess-new").as_deref(),
        Some("elsewhere\n")
    );
    let send = home.mail(
        None,
        &[
            "--room", "trial", "--as", "seed", "send", "fresh", "--id", "x", "--body", "hi",
        ],
    );
    expect_refusal(&send, "unknown_recipient", None);

    ok(&home.join(Some("sess-new"), "fresh"));
    assert_eq!(home.who(), ["fresh", "seed"]);
    assert_eq!(home.pointer_text("sess-new").as_deref(), Some("trial\n"));
}

#[cfg(unix)]
#[test]
fn failed_rebind_keeps_the_owner_the_route_and_pending_mail() {
    let home = Home::new();
    ok(&home.join(Some("seed"), "seed"));
    ok(&home.join(Some("sess-old"), "keeper"));
    ok(&home.mail(
        Some("seed"),
        &["send", "keeper", "--id", "m1", "--body", "pending"],
    ));
    ok(&home.mail(Some("sess-new"), &["join", "away", "--room", "elsewhere"]));

    let out = home.join_with_unwritable_pointer("sess-new", "keeper");
    refusal_message(&out, "mail_file");
    assert_eq!(
        home.pointer_text("sess-new").as_deref(),
        Some("elsewhere\n")
    );
    assert_eq!(home.pointer_text("sess-old").as_deref(), Some("trial\n"));
    let inbox = ok(&home.mail(Some("sess-old"), &["inbox"]));
    assert_eq!(inbox["messages"][0]["body"], "pending");
    let intruder = home.mail(Some("sess-new"), &["--room", "trial", "inbox"]);
    expect_refusal(&intruder, "seat_required", None);

    let rebound = ok(&home.join(Some("sess-new"), "keeper"));
    assert_eq!(rebound["replaced_session"], "sess-old");
    assert_eq!(
        ok(&home.mail(Some("sess-new"), &["inbox"]))["messages"][0]["body"],
        "pending"
    );
}

#[cfg(unix)]
#[test]
fn failed_rebind_at_the_seat_folder_keeps_the_prior_owner() {
    let home = Home::new();
    ok(&home.join(Some("seed"), "seed"));
    ok(&home.join(Some("sess-old"), "keeper"));
    let dir = home.seat_dir("keeper");
    set_mode(&dir, 0o755);

    refusal_message(
        &home.join(Some("sess-new"), "keeper"),
        "seat_dir_unavailable",
    );
    assert_eq!(home.pointer_text("sess-new"), None);
    assert_eq!(home.pointer_text("sess-old").as_deref(), Some("trial\n"));
    ok(&home.mail(Some("sess-old"), &["who"]));

    set_mode(&dir, 0o700);
    let rebound = ok(&home.join(Some("sess-new"), "keeper"));
    assert_eq!(rebound["replaced_session"], "sess-old");
}

#[test]
fn headless_join_still_needs_no_route() {
    let home = Home::new();
    let joined = ok(&home.join(None, "headless"));
    assert_eq!(joined["seat"], "headless");
    assert_eq!(home.who(), ["headless"]);
    assert!(!home.home.join("sessions").exists());
}

#[test]
fn unreadable_prior_pointer_refuses_before_anything_changes() {
    let home = Home::new();
    ok(&home.join(Some("seed"), "seed"));
    let blocker = home.pointer("sess-new");
    std::fs::create_dir(&blocker).unwrap();

    refusal_message(&home.join(Some("sess-new"), "fresh"), "mail_file");
    assert_eq!(home.who(), ["seed"]);

    std::fs::remove_dir(&blocker).unwrap();
    ok(&home.join(Some("sess-new"), "fresh"));
    assert_eq!(home.who(), ["fresh", "seed"]);
}

#[cfg(unix)]
#[test]
fn failed_commit_puts_the_route_back() {
    let home = Home::new();
    ok(&home.join(Some("seed"), "seed"));
    home.sql("trial", FAIL_LOSER_COMMIT);

    let first = home.join(Some("sess-new"), "loser");
    refusal_message(&first, "mail_database");
    assert_eq!(home.pointer_text("sess-new"), None);
    assert_eq!(home.who(), ["seed"]);

    ok(&home.mail(Some("sess-new"), &["join", "away", "--room", "elsewhere"]));
    refusal_message(&home.join(Some("sess-new"), "loser"), "mail_database");
    assert_eq!(
        home.pointer_text("sess-new").as_deref(),
        Some("elsewhere\n")
    );
    assert_eq!(home.who(), ["seed"]);

    home.sql("trial", "DROP TRIGGER fail_loser;");
    ok(&home.join(Some("sess-new"), "loser"));
    assert_eq!(home.pointer_text("sess-new").as_deref(), Some("trial\n"));
}

#[cfg(unix)]
#[test]
fn failing_join_cannot_undo_a_concurrent_join_of_the_same_session() {
    let home = Home::new();
    ok(&home.mail(Some("sess"), &["join", "old", "--room", "prior"]));
    ok(&home.mail(None, &["join", "probe", "--room", "contested"]));
    home.sql("contested", FAIL_LOSER_COMMIT);
    let held = home.root.path().join("held");
    let release = home.root.path().join("release");
    let mut blocker = SpawnedChild::spawn(
        home.root
            .command("python3")
            .args(["-c", HOLD_WRITE_LOCK])
            .arg(home.home.join("rooms/contested/inbox.sqlite3"))
            .arg(&held)
            .arg(&release),
    );
    blocker.until(Duration::from_secs(10), || held.exists().then_some(()));

    // The first join owns the session's route and waits for the contested room's write lock.
    let mut first = SpawnedChild::spawn(
        &mut home.command(Some("sess"), &["join", "loser", "--room", "contested"]),
    );
    let lock = home.pointer("sess.lock");
    let owner = format!("{{\"pid\":{},", first.pid());
    first.until(Duration::from_secs(4), || {
        std::fs::read_to_string(&lock)
            .ok()
            .filter(|record| record.starts_with(&owner))
    });
    let mut second =
        SpawnedChild::spawn(&mut home.command(Some("sess"), &["join", "winner", "--room", "wins"]));
    // Fails if the second join runs to completion while the first still owns the route.
    let observed = std::time::Instant::now() + Duration::from_secs(1);
    second.until(Duration::from_secs(2), || {
        (std::time::Instant::now() >= observed).then_some(())
    });
    std::fs::write(&release, "").unwrap();

    let failed = first.wait_within(Duration::from_secs(10));
    let message = refusal_message(&failed, "mail_database");
    assert!(message.contains("commit"), "{message}");
    ok(&second.wait_within(Duration::from_secs(20)));
    assert!(
        blocker
            .wait_within(Duration::from_secs(10))
            .status
            .success()
    );
    assert_eq!(home.pointer_text("sess").as_deref(), Some("wins\n"));
    ok(&home.mail(Some("sess"), &["who"]));
}

#[cfg(unix)]
#[test]
fn startup_route_check_refuses_a_missing_pointer_despite_a_room_override() {
    let home = Home::new();
    ok(&home.join(Some("sess"), "keeper"));
    // The startup procedure's command, in a session that inherited FORMATION_ROOM.
    let check = || {
        home.root
            .command("env")
            .env("FORMATION_HOME", &home.home)
            .env("FORMATION_ROOM", "trial")
            .env("CODEX_THREAD_ID", "sess")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .args(["-u", "FORMATION_ROOM"])
            .arg(env!("CARGO_BIN_EXE_formation"))
            .args(["mail", "who"])
            .output()
            .unwrap()
    };
    let listed = ok(&check());
    assert!(
        listed["seats"]
            .as_array()
            .unwrap()
            .iter()
            .any(|seat| seat["name"] == "keeper"),
        "{listed}"
    );

    std::fs::rename(home.pointer("sess"), home.root.path().join("aside")).unwrap();
    let overridden = home
        .command(Some("sess"), &["who"])
        .env("FORMATION_ROOM", "trial")
        .output()
        .unwrap();
    ok(&overridden);
    expect_refusal(&check(), "room_required", None);
}

/// Run a mail verb with `FORMATION_FORMAT` set and stdout piped.
fn piped_with(room: &Room, seat: &str, format: Option<&str>, args: &[&str]) -> Output {
    let mut cmd = room.command(seat);
    if let Some(format) = format {
        cmd.env("FORMATION_FORMAT", format);
    }
    cmd.args(args).output().unwrap()
}

/// Run a mail verb with stdout on a real terminal.
#[cfg(unix)]
fn on_terminal(
    room: &Room,
    seat: &str,
    format: Option<&str>,
    args: &[&str],
) -> support::OnTerminal {
    let mut cmd = room.command(seat);
    if let Some(format) = format {
        cmd.env("FORMATION_FORMAT", format);
    }
    cmd.args(args);
    support::on_terminal(cmd)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[cfg(unix)]
#[test]
fn a_terminal_gets_human_output_for_every_verb_and_json_overrides_a_human_preference() {
    let room = Room::new();
    let join = on_terminal(&room, "chief", None, &["join", "chief"]);
    assert!(join.status.success(), "{}", join.stderr);
    let seat_dir = room.0.path().join("seats/chief");
    assert_eq!(
        join.stdout,
        format!("Joined as chief\nSeat folder: {}\n", seat_dir.display())
    );
    assert!(join.stderr.is_empty());
    room.ok("captain", &["join", "captain"]);

    let send = on_terminal(
        &room,
        "chief",
        None,
        &["send", "captain", "--id", "t-1", "--body", "first\nsecond"],
    );
    assert_eq!(
        send.stdout,
        "Sent t-1 as receipt 1; recipient is not present\n"
    );

    let who = on_terminal(&room, "chief", None, &["who"]);
    let lines: Vec<&str> = who.stdout.lines().collect();
    assert_eq!(
        lines[0].split_whitespace().collect::<Vec<_>>(),
        ["SEAT", "UNDELIVERED", "UNACKNOWLEDGED", "HELD", "SEEN"]
    );
    assert_eq!(
        lines[1].split_whitespace().collect::<Vec<_>>(),
        ["captain", "1", "1", "no", "-"]
    );
    assert_eq!(
        lines[2].split_whitespace().collect::<Vec<_>>()[..4],
        ["chief", "0", "0", "yes"]
    );
    assert!(serde_json::from_str::<Value>(&who.stdout).is_err());

    let inbox = on_terminal(&room, "captain", None, &["inbox"]);
    assert!(inbox.stdout.starts_with("Receipt 1 from chief (id t-1)\n"));
    assert!(inbox.stdout.contains("\n  first\n  second\n"));
    assert!(inbox.stdout.contains("delivered -\n"));

    let sent = on_terminal(&room, "chief", None, &["sent"]);
    let header: Vec<&str> = sent
        .stdout
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .collect();
    assert_eq!(header, ["RECEIPT", "ID", "RECIPIENT", "SENT", "STATE"]);

    let ack = on_terminal(&room, "captain", None, &["ack", "1"]);
    assert_eq!(ack.stdout, "Acknowledged receipt 1\n");

    let empty = on_terminal(&room, "captain", None, &["inbox"]);
    assert!(empty.stdout.is_empty());
    assert_eq!(empty.stderr, "No unacknowledged mail.\n");

    for preference in [None, Some("human"), Some("auto"), Some("unrecognized")] {
        let expected = on_terminal(&room, "chief", preference, &["who"]);
        assert!(expected.stdout.starts_with("SEAT"), "{preference:?}");
    }
    for (preference, args) in [
        (None, &["who", "--json"][..]),
        (Some("human"), &["who", "--json"]),
        (Some("human"), &["--json", "who"]),
        (Some("json"), &["who"]),
        (Some("JSON"), &["who"]),
    ] {
        let json = on_terminal(&room, "chief", preference, args);
        let value: Value = serde_json::from_str(&json.stdout).unwrap();
        assert_eq!(
            value["seats"].as_array().unwrap().len(),
            2,
            "{preference:?}"
        );
    }
    let json_sent = on_terminal(&room, "chief", Some("human"), &["sent", "--json"]);
    let value: Value = serde_json::from_str(&json_sent.stdout).unwrap();
    assert_eq!(value["total"], 1);
    assert_eq!(value["truncated"], false);
}

#[test]
fn piped_output_keeps_the_json_envelopes_and_every_verb_accepts_json() {
    let room = Room::new();
    let joined = run_ok(room.command("chief").args(["join", "chief"]));
    let joined_flag = run_ok(room.command("chief").args(["join", "chief", "--json"]));
    assert_eq!(joined, joined_flag);
    assert_eq!(joined["seat"], "chief");
    assert_eq!(joined["rebound"], false);
    assert!(
        joined["seat_dir"]
            .as_str()
            .unwrap()
            .ends_with("seats/chief")
    );
    room.ok("captain", &["join", "captain"]);

    let send = ["send", "captain", "--id", "p-1", "--body", "hello"];
    let sent = run_ok(room.command("chief").args(send));
    let again = run_ok(room.command("chief").args(send).arg("--json"));
    assert_eq!(sent, again);
    assert_eq!(sent["receipt"], 1);
    assert_eq!(sent["held"], false);

    for (verb, key) in [(&["who"][..], "seats"), (&["inbox"][..], "messages")] {
        let seat = if verb == ["who"] { "chief" } else { "captain" };
        let plain = run_ok(room.command(seat).args(verb));
        let flagged = run_ok(room.command(seat).args(verb).arg("--json"));
        let setting = run_ok(
            room.command(seat)
                .env("FORMATION_FORMAT", "json")
                .args(verb),
        );
        let unknown = run_ok(
            room.command(seat)
                .env("FORMATION_FORMAT", "pretty")
                .args(verb),
        );
        assert!(plain[key].is_array());
        if key == "seats" {
            for who in [&plain, &flagged, &setting, &unknown] {
                assert_who_envelope(who);
                assert_eq!(who_without_seen(&plain), who_without_seen(who));
            }

            let mut later_seen = plain.clone();
            later_seen["seats"][1]["seen"] = "2020-01-01T00:00:00Z".into();
            assert_who_envelope(&later_seen);
            assert_ne!(plain["seats"][1]["seen"], later_seen["seats"][1]["seen"]);
            assert_eq!(who_without_seen(&plain), who_without_seen(&later_seen));
        } else {
            assert_eq!(plain, flagged);
            assert_eq!(plain, setting);
            assert_eq!(plain, unknown);
        }
    }
    let inbox = run_ok(room.command("captain").arg("inbox"));
    assert_eq!(inbox["messages"][0]["body"], "hello");
    assert_eq!(inbox["messages"][0]["receipt"], 1);

    let human = piped_with(&room, "captain", Some("human"), &["inbox"]);
    assert!(text(&human.stdout).starts_with("Receipt 1 from chief (id p-1)\n"));

    let tsv = room.run("chief", &["sent"]);
    assert!(text(&tsv.stdout).starts_with("1\tp-1\tcaptain\t"));
    let envelope: Value =
        serde_json::from_slice(&room.run("chief", &["sent", "--json"]).stdout).unwrap();
    assert_eq!(envelope["messages"][0]["id"], "p-1");
    assert_eq!(envelope["total"], 1);
    assert_eq!(envelope["truncated"], false);

    let ack = run_ok(room.command("captain").args(["ack", "1"]));
    let ack_flag = run_ok(room.command("captain").args(["ack", "1", "--json"]));
    assert_eq!(ack, ack_flag);
    assert_eq!(ack["acknowledged"], 1);
}

#[test]
fn failures_are_one_plain_line_for_a_person_and_one_json_object_for_a_machine() {
    let home = Home::new();
    let unset = |format: Option<&str>, extra: &[&str]| {
        let mut cmd = home.command(None, &["who"]);
        cmd.args(extra);
        if let Some(format) = format {
            cmd.env("FORMATION_FORMAT", format);
        }
        cmd.output().unwrap()
    };
    let message = "room required: pass --room or set FORMATION_ROOM";
    for (format, extra) in [
        (None, &[][..]),
        (None, &["--json"]),
        (Some("json"), &[]),
        (Some("human"), &["--json"]),
    ] {
        let out = unset(format, extra);
        assert_eq!(refusal_message(&out, "room_required"), message);
        let line = text(&out.stderr);
        let body: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(body.as_object().unwrap().len(), 2);
    }
    let out = unset(Some("human"), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(text(&out.stderr), format!("error: {message}\n"));
}

#[cfg(unix)]
#[test]
fn failures_on_a_terminal_are_plain_unless_json_is_asked_for() {
    let room = Room::new();
    room.ok("chief", &["join", "chief"]);
    let args = ["send", "nobody", "--id", "x-1", "--body", "hi"];
    let human = on_terminal(&room, "chief", None, &args);
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    assert!(human.stderr.starts_with("error: "), "{}", human.stderr);
    assert_eq!(human.stderr.lines().count(), 1);
    assert!(human.stderr.contains("not joined"), "{}", human.stderr);

    let mut flagged = args.to_vec();
    flagged.push("--json");
    let machine = on_terminal(&room, "chief", Some("human"), &flagged);
    assert_eq!(machine.status.code(), Some(1));
    assert!(machine.stdout.is_empty());
    let body: Value = serde_json::from_str(machine.stderr.trim()).unwrap();
    assert_eq!(body["code"], "unknown_recipient");
    assert_eq!(machine.stderr.lines().count(), 1);
}
