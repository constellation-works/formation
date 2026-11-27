//! Public command contracts exercised through the built executable.
#![allow(
    clippy::unwrap_used,
    reason = "failed integration fixtures should panic"
)]

mod support;

use cw_testkit::{Goldens, TempRoot, redact_timestamps};
use std::io::Write;
use std::process::{Output, Stdio};
use std::time::Duration;
use support::SpawnedChild;

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
        ("help/roster-apply.txt", vec!["roster", "apply", "--help"]),
        ("help/watch.txt", vec!["watch", "--help"]),
        ("help/watch-add.txt", vec!["watch", "add", "--help"]),
        ("help/watch-remove.txt", vec!["watch", "remove", "--help"]),
        ("help/watch-pause.txt", vec!["watch", "pause", "--help"]),
        ("help/watch-resume.txt", vec!["watch", "resume", "--help"]),
        ("help/watch-list.txt", vec!["watch", "list", "--help"]),
        ("help/watch-run.txt", vec!["watch", "run", "--help"]),
        ("help/watch-stop.txt", vec!["watch", "stop", "--help"]),
        ("help/watch-check.txt", vec!["watch", "check", "--help"]),
        ("help/watch-wait.txt", vec!["watch", "wait", "--help"]),
        ("help/grant.txt", vec!["grant", "--help"]),
        ("help/grant-add.txt", vec!["grant", "add", "--help"]),
        (
            "help/grant-withhold.txt",
            vec!["grant", "withhold", "--help"],
        ),
        ("help/grant-revoke.txt", vec!["grant", "revoke", "--help"]),
        ("help/grant-list.txt", vec!["grant", "list", "--help"]),
        ("help/grant-check.txt", vec!["grant", "check", "--help"]),
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

/// Run `formation <args>` against a shared room from a fixed working directory.
fn in_room(root: &TempRoot, cwd: &std::path::Path, args: &[&str]) -> String {
    let result = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

#[test]
fn mail_who_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().to_str().unwrap().to_owned();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    for seat in ["captain", "chief"] {
        in_room(&root, &cwd, &["mail", "join", seat, "--room", &room]);
    }
    in_room(
        &root,
        &cwd,
        &[
            "mail", "send", "captain", "--room", &room, "--as", "chief", "--id", "who-1", "--body",
            "pending",
        ],
    );
    let who = in_room(
        &root,
        &cwd,
        &["mail", "who", "--room", &room, "--as", "chief"],
    );
    goldens
        .check("output/mail-who.json", &redact_timestamps(&who))
        .unwrap();
}

#[test]
fn watch_list_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().to_str().unwrap().to_owned();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    let run_in_room = |args: &[&str]| in_room(&root, &cwd, args);

    for seat in ["orbit-0-capt", "scribe-0-capt"] {
        run_in_room(&["mail", "join", seat, "--room", &room]);
    }
    let watch = |seat: &str, args: &[&str]| {
        let mut full = vec!["watch", "--room", &room, "--as", seat];
        full.extend_from_slice(args);
        run_in_room(&full)
    };
    watch(
        "orbit-0-capt",
        &["add", "attention", "--", "./attention-read.sh", "orbit"],
    );
    watch(
        "orbit-0-capt",
        &[
            "add",
            "ci",
            "--every",
            "300",
            "--",
            "./ci-read.sh",
            "owner/name",
            "main",
        ],
    );
    watch(
        "scribe-0-capt",
        &[
            "add",
            "ci",
            "--every",
            "300",
            "--",
            "./ci-read.sh",
            "owner/name",
            "main",
        ],
    );
    watch(
        "scribe-0-capt",
        &[
            "add",
            "release",
            "--to",
            "orbit-0-capt",
            "--every",
            "60",
            "--",
            "./release-read.sh",
        ],
    );
    watch(
        "orbit-0-capt",
        &["pause", "release", "--to", "orbit-0-capt"],
    );

    let normalize = |text: String| redact_timestamps(&text).replace(cwd.to_str().unwrap(), "<cwd>");
    goldens
        .check(
            "output/watch-list.txt",
            &normalize(watch("scribe-0-capt", &["list"])),
        )
        .unwrap();
    goldens
        .check(
            "output/watch-list.json",
            &normalize(watch("scribe-0-capt", &["list", "--json"])),
        )
        .unwrap();
}

#[test]
fn watch_reader_list_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().to_str().unwrap();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    for seat in ["alpha", "beta"] {
        in_room(&root, &cwd, &["mail", "join", seat, "--room", room]);
    }
    let watch = |args: &[&str]| {
        let mut full = vec!["watch", "--room", room, "--as", "alpha"];
        full.extend_from_slice(args);
        in_room(&root, &cwd, &full)
    };
    watch(&["add", "sample", "--", "true"]);

    let start = |seat: &str| {
        SpawnedChild::spawn(
            root.command(env!("CARGO_BIN_EXE_formation"))
                .current_dir(&cwd)
                .args(["watch", "run", "--room", room, "--as", seat]),
        )
    };
    let mut reader = start("alpha");
    reader.until(Duration::from_secs(5), || {
        let document: serde_json::Value =
            serde_json::from_str(&watch(&["list", "--json"])).unwrap();
        (document["watches"][0]["last_result"] == "ok, 0 items").then_some(())
    });
    let mut standby = start("beta");
    let pid = standby.pid();
    let live_json = standby.until(Duration::from_secs(5), || {
        let json = watch(&["list", "--json"]);
        let document: serde_json::Value = serde_json::from_str(&json).unwrap();
        (document["reader"]["standbys"][0]["pid"] == pid).then_some(json)
    });
    let normalize = |text: String| {
        let mut text = redact_timestamps(&text).replace(cwd.to_str().unwrap(), "<cwd>");
        for (pid, fixed) in [(reader.pid(), 1), (standby.pid(), 2)] {
            text = text
                .replace(&format!("pid {pid},"), &format!("pid {fixed},"))
                .replace(&format!("\"pid\":{pid},"), &format!("\"pid\":{fixed},"));
        }
        text
    };
    goldens
        .check("output/watch-list-live.json", &normalize(live_json))
        .unwrap();
    goldens
        .check("output/watch-list-live.txt", &normalize(watch(&["list"])))
        .unwrap();

    watch(&["stop"]);
    assert!(reader.wait_within(Duration::from_secs(5)).status.success());
    assert!(standby.wait_within(Duration::from_secs(5)).status.success());
    let stopped =
        |args: &[&str]| redact_timestamps(&watch(args)).replace(cwd.to_str().unwrap(), "<cwd>");
    goldens
        .check(
            "output/watch-list-stopped.json",
            &stopped(&["list", "--json"]),
        )
        .unwrap();
    goldens
        .check("output/watch-list-stopped.txt", &stopped(&["list"]))
        .unwrap();
}

#[test]
fn invalid_drafts_fail_before_reporting_success() {
    let roster: serde_json::Value = serde_json::from_slice(EXAMPLE).unwrap();
    for (pointer, replacement, diagnostic) in [
        ("/version", serde_json::json!(3), "version"),
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

#[test]
fn roster_versions_and_parent_errors() {
    let mut roster: serde_json::Value = serde_json::from_slice(EXAMPLE).unwrap();
    assert!(run(&["roster", "check"], EXAMPLE).status.success());
    roster["version"] = 2.into();
    roster["seats"][1]["role"] = "advisor".into();
    roster["seats"][1]["reports_to"] = "chief".into();
    let encode = |roster: &serde_json::Value| serde_json::to_vec(roster).unwrap();
    assert!(run(&["roster", "check"], &encode(&roster)).status.success());
    roster["seats"][1]["reports_to"] = "missing".into();
    let result = run(&["roster", "check"], &encode(&roster));
    assert!(String::from_utf8_lossy(&result.stderr).contains("unknown_parent"));
    roster["seats"][1]["reports_to"] = "chief".into();
    roster["seats"][0]["reports_to"] = roster["seats"][1]["name"].clone();
    let result = run(&["roster", "check"], &encode(&roster));
    assert!(String::from_utf8_lossy(&result.stderr).contains("roster_cycle"));
}

#[test]
fn roster_apply_replaces_without_changing_joined_seats() {
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().join("room");
    let room_arg = room.to_str().unwrap();
    in_room(
        &root,
        root.path(),
        &["mail", "join", "chief", "--room", room_arg],
    );
    let apply = |input: &[u8], json: bool| {
        let mut command = root.command(env!("CARGO_BIN_EXE_formation"));
        command.args(["roster", "apply", "--room", room_arg]);
        if json {
            command.arg("--json");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    };
    let refused = apply(EXAMPLE, false);
    assert!(String::from_utf8_lossy(&refused.stderr).contains("roster_version_required"));
    let mut roster: serde_json::Value = serde_json::from_slice(EXAMPLE).unwrap();
    roster["version"] = 2.into();
    roster["seats"][1]["reports_to"] = "chief".into();
    let result = apply(&serde_json::to_vec(&roster).unwrap(), true);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["seats"],
        4
    );
    {
        let store = formation_store::Store::open(&room).unwrap();
        let seats = store.read().unwrap().roster().seats().unwrap();
        let aide = seats
            .iter()
            .find(|seat| seat.name == roster["seats"][1]["name"])
            .unwrap();
        assert_eq!(aide.reports_to.as_deref(), Some("chief"));
    }
    roster["seats"] = serde_json::json!([{
        "name": "new", "role": "advisor", "host": "headless", "workspace": "orbit"
    }]);
    roster["routes"] = serde_json::json!([]);
    let result = apply(&serde_json::to_vec(&roster).unwrap(), false);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 seats"));
    let store = formation_store::Store::open(&room).unwrap();
    let snapshot = store.read().unwrap();
    let seats = snapshot.roster().seats().unwrap();
    assert_eq!(seats.len(), 1);
    assert_eq!(
        (
            &*seats[0].name,
            &*seats[0].role,
            seats[0].workspace.as_deref()
        ),
        ("new", "advisor", Some("orbit"))
    );
    assert!(snapshot.seats().contains("chief").unwrap());
}

#[test]
fn roster_check_creates_no_room() {
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().join("missing-room");
    let mut child = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .env("FORMATION_ROOM", &room)
        .args(["roster", "check"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(EXAMPLE).unwrap();
    assert!(child.wait_with_output().unwrap().status.success());
    assert!(!room.exists());
}

#[test]
fn grant_list_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().to_str().unwrap().to_owned();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    for seat in ["0-chief", "orbit-capt", "site-capt"] {
        in_room(&root, &cwd, &["mail", "join", seat, "--room", &room]);
    }
    let roster = serde_json::json!({
        "version": 2,
        "seats": [
            {
                "name": "0-chief",
                "role": "chief",
                "host": "claude-desktop"
            },
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
    });
    let mut apply = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(["roster", "apply", "--room", &room])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    apply
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&roster).unwrap())
        .unwrap();
    assert!(apply.wait_with_output().unwrap().status.success());

    in_room(
        &root,
        &cwd,
        &[
            "grant",
            "add",
            "--room",
            &room,
            "--as",
            "0-chief",
            "--to",
            "chief",
            "--class",
            "direction",
            "--workspace",
            "all",
            "--limit",
            "reversible workspace settings",
            "--for",
            "day",
            "--source",
            "advisor chat 12:00",
        ],
    );
    in_room(
        &root,
        &cwd,
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
    );

    goldens
        .check(
            "output/grant-list-room.txt",
            &in_room(&root, &cwd, &["grant", "list", "--room", &room]),
        )
        .unwrap();
    goldens
        .check(
            "output/grant-list-room.json",
            &redact_timestamps(&in_room(
                &root,
                &cwd,
                &["grant", "list", "--room", &room, "--json"],
            )),
        )
        .unwrap();
    goldens
        .check(
            "output/grant-list-seat.txt",
            &in_room(
                &root,
                &cwd,
                &["grant", "list", "--room", &room, "--seat", "site-capt"],
            ),
        )
        .unwrap();
    goldens
        .check(
            "output/grant-list-seat.json",
            &in_room(
                &root,
                &cwd,
                &[
                    "grant",
                    "list",
                    "--room",
                    &room,
                    "--seat",
                    "site-capt",
                    "--json",
                ],
            ),
        )
        .unwrap();
}
