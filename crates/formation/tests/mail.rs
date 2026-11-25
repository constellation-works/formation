//! Local-room contracts through independent CLI processes.
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

use cw_testkit::TempRoot;
use serde_json::Value;
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
            .arg("--identity")
            .arg(self.0.path().join(seat));
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

#[test]
fn recipient_owns_ack_and_retries_preserve_receipts() {
    let room = Room::new();
    room.ok("a", &["join", "chief"]);
    room.ok("b", &["join", "captain"]);
    assert!(!room.run("c", &["join", "captain"]).status.success());
    assert_eq!(
        room.ok("a", &["who"])["online"],
        serde_json::json!(["captain", "chief"])
    );
    let send = ["send", "captain", "--id", "alert-1", "--body", "a probe"];
    let receipt = room.ok("a", &send);
    assert_eq!(room.ok("a", &send), receipt);
    assert!(
        !room
            .run(
                "a",
                &["send", "captain", "--id", "alert-1", "--body", "different"]
            )
            .status
            .success()
    );
    assert!(
        !room
            .run(
                "a",
                &["send", "missing", "--id", "absent", "--body", "hello"]
            )
            .status
            .success()
    );
    let id = receipt["receipt"].to_string();
    assert!(!room.run("a", &["ack", &id]).status.success());
    let read = room.ok("b", &["inbox"]);
    assert_eq!(read["messages"][0]["body"], "a probe");
    assert_eq!(read, room.ok("b", &["inbox"]));
    room.ok("b", &["ack", &id]);
    room.ok("b", &["ack", &id]);
    assert_eq!(room.ok("a", &send), receipt);
    assert_eq!(room.ok("b", &["inbox"])["messages"], serde_json::json!([]));
}

#[test]
fn concurrent_duplicate_sends_wake_a_waiter_once() {
    use std::process::Stdio;
    let room = Room::new();
    room.ok("a", &["join", "chief"]);
    room.ok("b", &["join", "captain"]);
    let sentry = room
        .command("b")
        .args(["inbox", "--wait", "5"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut sends = Vec::new();
    for _ in 0..4 {
        sends.push(
            room.command("a")
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
