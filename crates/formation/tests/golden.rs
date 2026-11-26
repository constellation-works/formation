//! Public command contracts exercised through the built executable.
#![allow(
    clippy::unwrap_used,
    reason = "failed integration fixtures should panic"
)]

use cw_testkit::{Goldens, TempRoot};
use std::io::Write;
use std::process::{Output, Stdio};

const EXAMPLE: &[u8] = include_bytes!("../../../examples/roster.json");

fn run(args: &[&str], input: &[u8]) -> Output {
    let root = TempRoot::new("FORMATION").unwrap();
    let mut child = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if !args.iter().any(|arg| matches!(*arg, "-h" | "--help")) {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    child.wait_with_output().unwrap()
}

#[test]
fn help_and_roster_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    for (name, args) in [
        ("help/root.txt", vec!["--help"]),
        ("help/mail.txt", vec!["mail", "--help"]),
        ("help/mail-join.txt", vec!["mail", "join", "--help"]),
        ("help/mail-who.txt", vec!["mail", "who", "--help"]),
        ("help/mail-send.txt", vec!["mail", "send", "--help"]),
        ("help/mail-inbox.txt", vec!["mail", "inbox", "--help"]),
        ("help/mail-sent.txt", vec!["mail", "sent", "--help"]),
        ("help/mail-ack.txt", vec!["mail", "ack", "--help"]),
        ("help/hook.txt", vec!["hook", "--help"]),
        ("help/roster.txt", vec!["roster", "--help"]),
        ("help/roster-check.txt", vec!["roster", "check", "--help"]),
        ("help/watch.txt", vec!["watch", "--help"]),
        ("output/roster.txt", vec!["roster", "check"]),
        ("output/roster.json", vec!["roster", "check", "--json"]),
    ] {
        let result = run(&args, EXAMPLE);
        assert!(result.status.success(), "{:?}", result.stderr);
        assert!(result.stderr.is_empty());
        goldens
            .check(name, std::str::from_utf8(&result.stdout).unwrap())
            .unwrap();
    }
}

#[test]
fn invalid_drafts_fail_before_reporting_success() {
    let roster: serde_json::Value = serde_json::from_slice(EXAMPLE).unwrap();
    for (pointer, replacement, diagnostic) in [
        ("/version", serde_json::json!(2), "version"),
        ("/seats/1/name", serde_json::json!("chief"), "unique"),
        ("/seats/2/workspace", serde_json::Value::Null, "workspace"),
        ("/routes/0/to", serde_json::json!("missing"), "unknown seat"),
        ("/routes/0/to", serde_json::json!("chief"), "distinct"),
        (
            "/seats/0/role",
            serde_json::json!("operator-command"),
            "invalid roster JSON",
        ),
    ] {
        let mut broken = roster.clone();
        *broken.pointer_mut(pointer).unwrap() = replacement;
        let result = run(&["roster", "check"], &serde_json::to_vec(&broken).unwrap());
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert!(String::from_utf8_lossy(&result.stderr).contains(diagnostic));
    }
}

#[test]
fn duplicate_workspace_ownership_fails() {
    let mut roster: serde_json::Value = serde_json::from_slice(EXAMPLE).unwrap();
    roster["seats"][0]["role"] = serde_json::json!("captain");
    roster["seats"][0]["workspace"] = serde_json::json!("orbit");
    let result = run(&["roster", "check"], &serde_json::to_vec(&roster).unwrap());
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("more than one captain"));
}
