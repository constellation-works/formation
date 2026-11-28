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

const HUMAN: [(&str, &str); 1] = [("FORMATION_FORMAT", "human")];

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
        ("help/mail-log.txt", vec!["mail", "log", "--help"]),
        ("help/mail-show.txt", vec!["mail", "show", "--help"]),
        ("help/mail-ack.txt", vec!["mail", "ack", "--help"]),
        ("help/hook.txt", vec!["hook", "--help"]),
        ("help/roster.txt", vec!["roster", "--help"]),
        ("help/roster-check.txt", vec!["roster", "check", "--help"]),
        ("help/roster-apply.txt", vec!["roster", "apply", "--help"]),
        ("help/roster-show.txt", vec!["roster", "show", "--help"]),
        ("help/roster-list.txt", vec!["roster", "list", "--help"]),
        ("help/roster-close.txt", vec!["roster", "close", "--help"]),
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
        ("help/grant-load.txt", vec!["grant", "load", "--help"]),
        ("help/grant-list.txt", vec!["grant", "list", "--help"]),
        ("help/grant-check.txt", vec!["grant", "check", "--help"]),
        ("help/request.txt", vec!["request", "--help"]),
        ("help/request-open.txt", vec!["request", "open", "--help"]),
        (
            "help/request-advise.txt",
            vec!["request", "advise", "--help"],
        ),
        (
            "help/request-settle.txt",
            vec!["request", "settle", "--help"],
        ),
        (
            "help/request-reopen.txt",
            vec!["request", "reopen", "--help"],
        ),
        ("help/request-carry.txt", vec!["request", "carry", "--help"]),
        ("help/request-pass.txt", vec!["request", "pass", "--help"]),
        ("help/request-list.txt", vec!["request", "list", "--help"]),
        ("help/request-show.txt", vec!["request", "show", "--help"]),
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

/// Like `in_room`, with environment variables set and the command's whole result returned.
fn in_room_env(
    root: &TempRoot,
    cwd: &std::path::Path,
    env: &[(&str, &str)],
    args: &[&str],
) -> Output {
    let mut cmd = root.command(env!("CARGO_BIN_EXE_formation"));
    cmd.args(args).current_dir(cwd);
    for (name, value) in env {
        cmd.env(name, value);
    }
    cmd.output().unwrap()
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
    let human = in_room_env(
        &root,
        &cwd,
        &HUMAN,
        &["mail", "who", "--room", &room, "--as", "chief"],
    );
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    goldens
        .check(
            "output/mail-who.txt",
            &redact_timestamps(std::str::from_utf8(&human.stdout).unwrap()),
        )
        .unwrap();
}

#[test]
fn mail_inbox_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().to_str().unwrap().to_owned();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    for seat in ["captain", "chief"] {
        in_room(&root, &cwd, &["mail", "join", seat, "--room", &room]);
    }
    for (id, body) in [("in-1", "CI finished"), ("in-2", "first line\nsecond line")] {
        in_room(
            &root,
            &cwd,
            &[
                "mail", "send", "captain", "--room", &room, "--as", "chief", "--id", id, "--body",
                body,
            ],
        );
    }
    let inbox = ["mail", "inbox", "--room", &room, "--as", "captain"];
    let json = in_room(&root, &cwd, &inbox);
    goldens
        .check("output/mail-inbox.json", &redact_timestamps(&json))
        .unwrap();
    let human = in_room_env(&root, &cwd, &HUMAN, &inbox);
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    goldens
        .check(
            "output/mail-inbox.txt",
            &redact_timestamps(std::str::from_utf8(&human.stdout).unwrap()),
        )
        .unwrap();
}

#[test]
fn mail_room_required_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    for (name, env, args) in [
        (
            "output/mail-room-required.json",
            &[][..],
            &["mail", "who"][..],
        ),
        (
            "output/mail-room-required.txt",
            &HUMAN[..],
            &["mail", "who"][..],
        ),
        (
            "output/mail-room-required.json",
            &HUMAN[..],
            &["mail", "who", "--json"][..],
        ),
    ] {
        let result = in_room_env(&root, &cwd, env, args);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        goldens
            .check(name, std::str::from_utf8(&result.stderr).unwrap())
            .unwrap();
    }
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
            "--as",
            "0-chief",
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

    let house = serde_json::json!({
        "version": 1,
        "grants": [{
            "effect": "grant",
            "to": "captain",
            "class": "operator",
            "workspace": "site",
            "limit": "answering a tracker decision that a later change can undo",
            "source": "house grants file"
        }]
    });
    let mut load = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(["grant", "load", "--room", &room, "--as", "0-chief"])
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    load.stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&house).unwrap())
        .unwrap();
    let loaded = load.wait_with_output().unwrap();
    assert!(loaded.status.success());
    goldens
        .check(
            "output/grant-load.json",
            &String::from_utf8(loaded.stdout).unwrap(),
        )
        .unwrap();

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

#[test]
fn request_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().join("trial");
    let room = room.to_str().unwrap();
    for seat in ["chief", "app-captain"] {
        let out = run(&["mail", "join", seat, "--room", room], b"");
        assert!(out.status.success(), "{out:?}");
    }
    let roster = br#"{"version":2,"routes":[],"seats":[
        {"name":"chief","role":"chief","host":"headless"},
        {"name":"app-captain","role":"captain","host":"headless",
         "workspace":"app","reports_to":"chief"}
    ]}"#;
    let out = run(&["roster", "apply", "--room", room], roster);
    assert!(out.status.success(), "{out:?}");
    let out = run(
        &[
            "request",
            "open",
            "--room",
            room,
            "--as",
            "app-captain",
            "--json",
            "--class",
            "land",
            "--record",
            "reviewed-change",
            "--ask",
            "Land the reviewed change?",
            "--option",
            "a: Land",
            "--option",
            "b: Wait",
            "--recommend",
            "a",
            "--settle",
            "a",
        ],
        b"",
    );
    assert!(out.status.success(), "{out:?}");
    let opened: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = opened["id"].as_str().unwrap();
    let normalize = |out: Output| {
        assert!(out.status.success(), "{out:?}");
        redact_timestamps(std::str::from_utf8(&out.stdout).unwrap()).replace(id, "<request>")
    };
    goldens
        .check("output/request-show.json", &normalize(out))
        .unwrap();
    let shown = run(&["request", "show", id, "--room", room], b"");
    goldens
        .check("output/request-show.txt", &normalize(shown))
        .unwrap();
}

#[test]
fn roster_show_compares_planned_and_joined_without_touching_presence() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    let room = "show-room";
    let session = [("CLAUDE_CODE_SESSION_ID", "show-session")];
    let joined = in_room_env(
        &root,
        &cwd,
        &session,
        &["mail", "join", "chief", "--room", room],
    );
    assert!(joined.status.success());
    in_room(&root, &cwd, &["mail", "join", "extra", "--room", room]);
    let no_roster = in_room(&root, &cwd, &["roster", "show", room, "--json"]);
    assert!(no_roster.contains("\"roster_applied\":false"));
    let roster = br#"{"version":2,"routes":[],"seats":[
        {"name":"chief","role":"chief","host":"headless",
         "model":"opus","effort":"high","record":"record-id"},
        {"name":"worker","role":"worker","host":"headless","reports_to":"chief"}]}"#;
    let mut child = root
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(["roster", "apply", "--room", room])
        .env("CLAUDE_CODE_SESSION_ID", "show-session")
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(roster).unwrap();
    assert!(child.wait_with_output().unwrap().status.success());
    let room_path = root.home().join(".formation/rooms").join(room);
    let store = formation_store::Store::open(&room_path).unwrap();
    in_room(
        &root,
        &cwd,
        &[
            "mail", "send", "extra", "--room", room, "--as", "chief", "--id", "one", "--body",
            "hello",
        ],
    );
    let who: serde_json::Value = serde_json::from_str(&in_room(
        &root,
        &cwd,
        &["mail", "who", "--room", room, "--as", "chief", "--json"],
    ))
    .unwrap();
    let before = store.read().unwrap().seats().mailboxes().unwrap();
    let shown = in_room_env(&root, &cwd, &session, &["roster", "show", "--json"]);
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(value["applied_by"], "chief");
    assert_eq!(value["seats"][0]["model"], "opus");
    assert!(value["seats"][1]["live"].is_null());
    assert_eq!(value["not_on_roster"][0]["name"], "extra");
    // The admiral's seat heads `mail who` and is the tree's root, never "not on the roster".
    assert_eq!(who["seats"][0]["name"], "admiral");
    assert_eq!(value["not_on_roster"].as_array().unwrap().len(), 1);
    assert_eq!(value["seats"][0]["live"], who["seats"][1]);
    assert_eq!(value["not_on_roster"][0], who["seats"][2]);
    let after = store.read().unwrap().seats().mailboxes().unwrap();
    assert_eq!(
        before.iter().map(|seat| seat.seen).collect::<Vec<_>>(),
        after.iter().map(|seat| seat.seen).collect::<Vec<_>>()
    );
    let human = in_room(&root, &cwd, &["roster", "show", room]);
    goldens
        .check("output/roster-show.txt", &redact_timestamps(&human))
        .unwrap();
    goldens
        .check(
            "output/roster-show.json",
            &redact_timestamps(std::str::from_utf8(&shown.stdout).unwrap()),
        )
        .unwrap();
    let missing = run(&["roster", "show"], b"");
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("room_required"));
}

#[test]
fn roster_close_and_list_preserve_the_first_close_and_mail() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let cwd = &root.home();
    let rooms = cwd.join(".formation/rooms");
    let session = [("CLAUDE_CODE_SESSION_ID", "chief-session")];
    let joined = in_room_env(
        &root,
        cwd,
        &session,
        &["mail", "join", "chief", "--room", "earlier"],
    );
    assert!(joined.status.success(), "{joined:?}");
    in_room(&root, cwd, &["mail", "join", "solo", "--room", "later"]);
    let applied = apply_roster(&root, "earlier");
    assert!(applied.status.success(), "{applied:?}");
    let store = formation_store::Store::open(&rooms.join("earlier")).unwrap();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    store
        .write(|tx| {
            tx.roster().stamp(now - 600, Some("chief"))?;
            tx.seats().touch("chief", now)?;
            Ok(())
        })
        .unwrap();

    let close = in_room_env(&root, cwd, &session, &["roster", "close"]);
    assert!(close.status.success(), "{close:?}");
    let close_text = String::from_utf8(close.stdout).unwrap();
    goldens
        .check("output/roster-close.txt", &redact_timestamps(&close_text))
        .unwrap();
    let after_close = room_files(&rooms.join("earlier"));
    let repeat = in_room(&root, cwd, &["roster", "close", "--room", "earlier"]);
    assert_eq!(repeat, close_text);
    assert_eq!(after_close, room_files(&rooms.join("earlier")));
    let close_json = in_room(
        &root,
        cwd,
        &["roster", "close", "--room", "earlier", "--json"],
    );
    goldens
        .check("output/roster-close.json", &redact_timestamps(&close_json))
        .unwrap();
    let closed: serde_json::Value = serde_json::from_str(&close_json).unwrap();
    assert_eq!(closed["closed_by"], "chief");

    let show = in_room(&root, cwd, &["roster", "show", "earlier"]);
    goldens
        .check("output/roster-show-closed.txt", &redact_timestamps(&show))
        .unwrap();
    let show = in_room(&root, cwd, &["roster", "show", "earlier", "--json"]);
    let show: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(show["status"], "closed");
    assert_eq!(show["closed_at"], closed["closed_at"]);
    let chief = show["seats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|seat| seat["name"] == "chief")
        .unwrap();
    assert_eq!(chief["live"]["held"], false);
    assert!(chief["live"]["seen"].is_string());

    let refused = apply_roster(&root, "earlier");
    assert!(!refused.status.success());
    let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(error["code"], "room_closed");
    in_room(&root, cwd, &["mail", "join", "late", "--room", "earlier"]);
    in_room(
        &root,
        cwd,
        &[
            "mail",
            "send",
            "late",
            "--room",
            "earlier",
            "--as",
            "chief",
            "--id",
            "one",
            "--body",
            "mail after close",
        ],
    );
    let inbox = in_room(
        &root,
        cwd,
        &["mail", "inbox", "--room", "earlier", "--as", "late"],
    );
    assert!(inbox.contains("mail after close"));
    let latest = now + 60;
    store.write(|tx| tx.seats().touch("late", latest)).unwrap();

    let before = room_files(&cwd.join(".formation"));
    let list = in_room(&root, cwd, &["roster", "list"]);
    goldens
        .check("output/roster-list.txt", &redact_timestamps(&list))
        .unwrap();
    let list = in_room(&root, cwd, &["roster", "list", "--json"]);
    goldens
        .check("output/roster-list.json", &redact_timestamps(&list))
        .unwrap();
    let list: Vec<serde_json::Value> = serde_json::from_str(&list).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["room"], "later");
    assert_eq!(list[1]["room"], "earlier");
    assert_eq!(list[1]["closed_at"], closed["closed_at"]);
    assert_eq!(list[1]["closed_by"], "chief");
    assert_eq!(
        list[0]["seats"],
        serde_json::json!({"held":0,"planned":null,"joined":1})
    );
    assert_eq!(
        list[1]["seats"],
        serde_json::json!({"held":1,"planned":2,"joined":2})
    );
    assert_eq!(
        list[1]["last_seen"],
        formation_store::contract::rfc3339(latest).unwrap(),
    );
    for (index, status) in ["active", "closed"].into_iter().enumerate() {
        let filtered = in_room(
            &root,
            cwd,
            &["roster", "list", "--status", status, "--json"],
        );
        let filtered: Vec<serde_json::Value> = serde_json::from_str(&filtered).unwrap();
        assert_eq!(filtered, list[index..=index]);
    }
    assert_eq!(before, room_files(&cwd.join(".formation")));
}

#[test]
fn roster_list_leaves_empty_and_unreadable_rooms_alone() {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    let root = TempRoot::new("FORMATION").unwrap();
    let cwd = &root.home();
    let home = cwd.join(".formation");
    let empty = in_room_env(&root, cwd, &[], &["roster", "list", "--json"]);
    assert!(empty.status.success(), "{empty:?}");
    assert_eq!(empty.stdout, b"[]\n");
    assert!(!home.exists());
    let external = cwd.join("outside");
    let external = external.to_str().unwrap();
    let session = [("CLAUDE_CODE_SESSION_ID", "external-session")];
    let joined = in_room_env(
        &root,
        cwd,
        &session,
        &["mail", "join", "chief", "--room", external],
    );
    assert!(joined.status.success(), "{joined:?}");
    let closed = in_room_env(
        &root,
        cwd,
        &session,
        &["roster", "close", "--room", external],
    );
    assert!(closed.status.success(), "{closed:?}");
    let shown = in_room(&root, cwd, &["roster", "show", external]);
    assert!(shown.contains("closed"));
    let shown = in_room(&root, cwd, &["roster", "show", external, "--json"]);
    let shown: serde_json::Value = serde_json::from_str(&shown).unwrap();
    assert!(!shown["roster_applied"].as_bool().unwrap());
    in_room(&root, cwd, &["mail", "join", "chief", "--room", "healthy"]);
    let unjoined = in_room_env(&root, cwd, &[], &["roster", "close", "--room", "healthy"]);
    assert!(!unjoined.status.success(), "{unjoined:?}");
    assert!(String::from_utf8_lossy(&unjoined.stderr).contains("seat_required"));
    in_room(
        &root,
        cwd,
        &["mail", "join", "chief", "--room", "missing-wal"],
    );
    std::fs::remove_file(home.join("rooms/missing-wal/inbox.sqlite3-wal")).unwrap();
    let broken = home.join("rooms/broken");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&broken)
        .unwrap();
    let db = broken.join("inbox.sqlite3");
    std::fs::write(&db, b"not a room database").unwrap();
    std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o600)).unwrap();

    let before = room_files(&home);
    let output = in_room_env(&root, cwd, &[], &["roster", "list", "--json"]);
    assert!(output.status.success(), "{output:?}");
    let list: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["room"], "healthy");
    assert_eq!(list[0]["status"], "active");
    let diagnostics = String::from_utf8(output.stderr).unwrap();
    let lines: Vec<_> = diagnostics.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("room broken:"));
    assert!(lines[1].contains("room missing-wal:"));
    assert_eq!(before, room_files(&home));
}

fn apply_roster(root: &TempRoot, room: &str) -> Output {
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
        .write_all(
            br#"{"version":2,"routes":[],"seats":[
        {"name":"chief","role":"chief","host":"headless"},
        {"name":"captain","role":"captain","host":"headless","workspace":"app"}]}"#,
        )
        .unwrap();
    child.wait_with_output().unwrap()
}

fn room_files(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            files.extend(room_files(&entry.path()));
        } else {
            files.insert(entry.path(), std::fs::read(entry.path()).unwrap());
        }
    }
    files
}

#[cfg(unix)]
#[test]
fn mail_log_and_show_output() {
    let goldens = Goldens::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens"));
    let root = TempRoot::new("FORMATION").unwrap();
    let room = root.path().to_str().unwrap();
    let cwd = std::fs::canonicalize(root.home()).unwrap();
    for seat in ["alpha", "beta", "gamma"] {
        in_room(&root, &cwd, &["mail", "join", seat, "--room", room]);
    }
    for (sender, recipient, id, body, reply) in [
        ("alpha", "beta", "one", "first\nsecond", &[][..]),
        ("beta", "gamma", "two", "other", &["--re", "1"][..]),
    ] {
        let mut args = vec![
            "mail", "send", recipient, "--as", sender, "--room", room, "--id", id, "--body", body,
        ];
        args.extend_from_slice(reply);
        in_room(&root, &cwd, &args);
    }
    for (name, args, env) in [
        (
            "output/mail-log.txt",
            vec!["mail", "log", "--as", "admiral", "--room", room],
            &HUMAN[..],
        ),
        (
            "output/mail-log.json",
            vec!["mail", "log", "--as", "admiral", "--room", room, "--json"],
            &[][..],
        ),
        (
            "output/mail-log-threads.txt",
            vec![
                "mail",
                "log",
                "--as",
                "admiral",
                "--room",
                room,
                "--threads",
            ],
            &HUMAN[..],
        ),
        (
            "output/mail-log-threads.json",
            vec![
                "mail",
                "log",
                "--as",
                "admiral",
                "--room",
                room,
                "--threads",
                "--json",
            ],
            &[][..],
        ),
        (
            "output/mail-show.txt",
            vec!["mail", "show", "1", "--as", "beta", "--room", room],
            &HUMAN[..],
        ),
        (
            "output/mail-show.json",
            vec![
                "mail", "show", "2", "--as", "beta", "--room", room, "--json",
            ],
            &[][..],
        ),
    ] {
        let mut cmd = root.command(env!("CARGO_BIN_EXE_formation"));
        cmd.current_dir(&cwd).args(&args);
        for (key, value) in env {
            cmd.env(key, value);
        }
        let output = support::on_terminal(cmd);
        assert!(output.status.success(), "{:?}", output.stderr);
        goldens
            .check(name, &redact_timestamps(&output.stdout))
            .unwrap();
    }
    let read = |seat: &str, extra: &[&str]| {
        let mut args = vec!["mail", "log", "--as", seat, "--room", room, "--json"];
        args.extend_from_slice(extra);
        let mut cmd = root.command(env!("CARGO_BIN_EXE_formation"));
        cmd.current_dir(&cwd).args(&args);
        let result = support::on_terminal(cmd);
        assert!(result.status.success(), "{:?}", result.stderr);
        serde_json::from_str::<serde_json::Value>(&result.stdout).unwrap()
    };
    let store = formation_store::Store::open(root.path()).unwrap();
    let before_presence = store.read().unwrap().seats().presence().unwrap();
    let before_messages = read("admiral", &[]);

    assert_eq!(
        read("admiral", &[])["messages"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        read("alpha", &["--seat", "gamma"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        read("admiral", &["--with", "gamma"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        read("admiral", &["--limit", "1"])["messages"][0]["id"],
        "two"
    );
    assert_eq!(
        read("admiral", &["--since", "2100-01-01T00:00:00Z"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let denied = in_room_env(
        &root,
        &cwd,
        &[],
        &["mail", "show", "2", "--as", "alpha", "--room", room],
    );
    assert!(!denied.status.success());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("mail_forbidden"));
    let missing = in_room_env(
        &root,
        &cwd,
        &[],
        &["mail", "show", "999", "--as", "alpha", "--room", room],
    );
    assert!(String::from_utf8_lossy(&missing.stderr).contains("unknown_receipt"));
    assert_eq!(
        before_presence,
        store.read().unwrap().seats().presence().unwrap()
    );
    assert_eq!(before_messages, read("admiral", &[]));
    store
        .write(|tx| {
            let receipt = tx
                .messages()
                .insert("beta", "gamma", "watch", "probe\nfull", 100)?;
            tx.messages().mark_watch(receipt, "probe-watch", "beta")?;
            for index in 0..50 {
                tx.messages()
                    .insert("beta", "gamma", &format!("bulk-{index}"), "bulk", 100)?;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(
        read("admiral", &[])["messages"].as_array().unwrap().len(),
        50
    );
    let watches = read("admiral", &["--seat", "gamma", "--limit", "100"]);
    assert!(
        watches["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["watch_name"] == "probe-watch")
    );
    let no_watch = read("admiral", &["--no-watch", "--limit", "100"]);
    assert!(
        no_watch["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|message| message["watch_name"].is_null())
    );
    let mut cmd = root.command(env!("CARGO_BIN_EXE_formation"));
    cmd.current_dir(&cwd)
        .args(["mail", "log", "--room", room, "--full", "--limit", "100"]);
    assert!(support::on_terminal(cmd).stdout.contains("probe\nfull"));
}
