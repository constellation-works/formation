//! Integration tests for `formation watch` through the built executable.
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

mod support;

use cw_testkit::TempRoot;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};
use support::SpawnedChild;

struct Room(TempRoot);

impl Room {
    fn new() -> Self {
        Self(TempRoot::new("FORMATION").unwrap())
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    fn mail_cmd(&self, seat: &str) -> Command {
        let mut cmd = self.0.command(env!("CARGO_BIN_EXE_formation"));
        cmd.arg("mail")
            .arg("--room")
            .arg(self.0.path())
            .arg("--as")
            .arg(seat);
        cmd
    }

    fn watch_cmd(&self, seat: &str) -> Command {
        let mut cmd = self.0.command(env!("CARGO_BIN_EXE_formation"));
        cmd.arg("watch")
            .arg("--room")
            .arg(self.0.path())
            .arg("--as")
            .arg(seat);
        cmd
    }

    fn join(&self, seat: &str) {
        let out = self.mail_cmd(seat).args(["join", seat]).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn inbox(&self, seat: &str) -> Value {
        let out = self.mail_cmd(seat).args(["inbox"]).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn watch_ok(&self, seat: &str, args: &[&str]) -> Value {
        let out = self.watch_cmd(seat).args(args).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn start_reader(&self, seat: &str) -> SpawnedChild {
        SpawnedChild::spawn(self.watch_cmd(seat).arg("run"))
    }

    fn listing(&self, seat: &str) -> Value {
        self.set_ok(seat, &["list", "--json"])
    }

    fn set_run(&self, seat: &str, args: &[&str]) -> Output {
        self.watch_cmd(seat).args(args).output().unwrap()
    }

    fn set_ok(&self, seat: &str, args: &[&str]) -> Value {
        let out = self.set_run(seat, args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn set_refused(&self, seat: &str, args: &[&str]) -> String {
        let out = self.set_run(seat, args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(out.stdout.is_empty());
        let body: Value = serde_json::from_slice(&out.stderr).unwrap();
        body["code"].as_str().unwrap().to_owned()
    }

    fn watches(&self, seat: &str) -> Vec<Value> {
        let listing = self.set_ok(seat, &["list", "--json"]);
        listing["watches"].as_array().unwrap().clone()
    }

    fn script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.0.path().join(name);
        std::fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }
}

#[cfg(unix)]
#[test]
fn item_is_returned_once_and_second_run_expires() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "probe.sh",
        "#!/bin/sh\nprintf 'fp-1\\tbuild red on main\\n'\n",
    );
    let probe = format!("ci={}", script.display());

    let first = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(first["expired"], false);
    assert_eq!(first["remaining"], 0);
    assert_eq!(first["failed"], serde_json::json!([]));
    assert_eq!(
        first["items"],
        serde_json::json!([{
            "probe": "ci",
            "fingerprint": "fp-1",
            "text": "build red on main",
        }])
    );

    let second = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(second["expired"], true);
    assert_eq!(second["remaining"], 0);
    assert_eq!(second["items"], serde_json::json!([]));
    assert_eq!(second["failed"], serde_json::json!([]));
}

#[cfg(unix)]
#[test]
fn failed_probe_reported_once_until_successful_read() {
    let room = Room::new();
    room.join("sentry");
    let mode_file = room.path().join("mode");
    std::fs::write(&mode_file, "fail").unwrap();
    let script = room.script(
        "flaky.sh",
        &format!(
            "#!/bin/sh\nif [ \"$(cat '{}')\" = 'fail' ]; then\n  printf 'upstream down\\nsecond line\\n' >&2\n  exit 1\nfi\nexit 0\n",
            mode_file.display()
        ),
    );
    let probe = format!("ci={}", script.display());

    let first_fail = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(first_fail["expired"], false);
    assert_eq!(first_fail["items"], serde_json::json!([]));
    assert_eq!(
        first_fail["failed"],
        serde_json::json!([{"probe": "ci", "error": "upstream down"}])
    );

    let repeat_fail = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(repeat_fail["expired"], true);
    assert_eq!(repeat_fail["items"], serde_json::json!([]));
    assert_eq!(repeat_fail["failed"], serde_json::json!([]));

    std::fs::write(&mode_file, "ok").unwrap();
    let recovered = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(recovered["expired"], true);
    assert_eq!(recovered["failed"], serde_json::json!([]));

    std::fs::write(&mode_file, "fail").unwrap();
    let fail_again = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(fail_again["expired"], false);
    assert_eq!(
        fail_again["failed"],
        serde_json::json!([{"probe": "ci", "error": "upstream down"}])
    );
}

#[cfg(unix)]
#[test]
fn line_without_tab_is_reported_as_failed_read_with_line_number() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "bad-line.sh",
        "#!/bin/sh\nprintf 'fp-1\\tvalid item\\nmissing-tab-line\\n'\n",
    );
    let probe = format!("ci={}", script.display());

    let out = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(out["expired"], false);
    assert_eq!(out["items"], serde_json::json!([]));
    let failed = out["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["probe"], "ci");
    let message = failed[0]["error"].as_str().unwrap();
    assert!(
        message.contains("line 2"),
        "expected line number in error, got: {message}"
    );
}

#[cfg(unix)]
#[test]
fn reader_mails_lines_and_chunks_25_items_into_20_and_5() {
    let room = Room::new();
    room.join("sentry");
    room.join("captain");
    let script = room.script(
        "twenty-five.sh",
        "#!/bin/sh\nidx=1\nwhile [ \"$idx\" -le 25 ]; do\n  printf 'fp-%02d\\titem %02d\\n' \"$idx\" \"$idx\"\n  idx=$((idx + 1))\ndone\n",
    );
    room.set_ok("captain", &["add", "ci", "--", script.to_str().unwrap()]);
    let mut reader = room.start_reader("sentry");
    let inbox = reader.until(Duration::from_secs(10), || {
        let inbox = room.inbox("captain");
        (inbox["messages"].as_array().unwrap().len() == 2).then_some(inbox)
    });
    room.set_ok("captain", &["stop"]);
    assert!(reader.wait_within(Duration::from_secs(5)).status.success());

    let messages = inbox["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["sender"], "sentry");
    assert_eq!(messages[1]["sender"], "sentry");

    let first_lines: Vec<&str> = messages[0]["body"].as_str().unwrap().lines().collect();
    let second_lines: Vec<&str> = messages[1]["body"].as_str().unwrap().lines().collect();
    assert_eq!(first_lines.len(), 20);
    assert_eq!(second_lines.len(), 5);
    assert_eq!(first_lines[0], "ci | item 01");
    assert_eq!(first_lines[19], "ci | item 20");
    assert_eq!(second_lines[0], "ci | item 21");
    assert_eq!(second_lines[4], "ci | item 25");
}

#[cfg(unix)]
#[test]
fn concurrent_waits_hand_over_each_fingerprint_once_between_them() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "race.sh",
        "#!/bin/sh\nidx=1\nwhile [ \"$idx\" -le 25 ]; do\n  printf 'fp-%02d\\titem %02d\\n' \"$idx\" \"$idx\"\n  idx=$((idx + 1))\ndone\n",
    );
    let probe = format!("ci={}", script.display());

    let mut cmd_a = room.watch_cmd("sentry");
    cmd_a.args(["wait", "--probe", &probe, "--timeout", "0"]);
    let mut cmd_b = room.watch_cmd("sentry");
    cmd_b.args(["wait", "--probe", &probe, "--timeout", "0"]);

    let child_a = SpawnedChild::spawn(&mut cmd_a);
    let child_b = SpawnedChild::spawn(&mut cmd_b);

    let out_a = child_a.wait_within(Duration::from_secs(15));
    let out_b = child_b.wait_within(Duration::from_secs(15));
    assert!(
        out_a.status.success(),
        "{}",
        String::from_utf8_lossy(&out_a.stderr)
    );
    assert!(
        out_b.status.success(),
        "{}",
        String::from_utf8_lossy(&out_b.stderr)
    );

    let body_a: Value = serde_json::from_slice(&out_a.stdout).unwrap();
    let body_b: Value = serde_json::from_slice(&out_b.stdout).unwrap();

    let mut handed = Vec::new();
    for body in [&body_a, &body_b] {
        for item in body["items"].as_array().unwrap() {
            handed.push(item["fingerprint"].as_str().unwrap().to_owned());
        }
    }
    let unique: HashSet<&str> = handed.iter().map(String::as_str).collect();
    assert_eq!(handed.len(), 25, "expected 25 handovers total: {handed:?}");
    assert_eq!(
        unique.len(),
        25,
        "duplicate fingerprint handed over: {handed:?}"
    );
}

#[cfg(unix)]
#[test]
fn wait_returns_20_with_remaining_5_then_other_5() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "batch.sh",
        "#!/bin/sh\nidx=1\nwhile [ \"$idx\" -le 25 ]; do\n  printf 'fp-%02d\\titem %02d\\n' \"$idx\" \"$idx\"\n  idx=$((idx + 1))\ndone\n",
    );
    let probe = format!("ci={}", script.display());

    let first = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(first["expired"], false);
    assert_eq!(first["remaining"], 5);
    let first_items = first["items"].as_array().unwrap();
    assert_eq!(first_items.len(), 20);
    assert_eq!(first_items[0]["fingerprint"], "fp-01");
    assert_eq!(first_items[19]["fingerprint"], "fp-20");

    let second = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(second["expired"], false);
    assert_eq!(second["remaining"], 0);
    let second_items = second["items"].as_array().unwrap();
    assert_eq!(second_items.len(), 5);
    assert_eq!(second_items[0]["fingerprint"], "fp-21");
    assert_eq!(second_items[4]["fingerprint"], "fp-25");
}

#[cfg(unix)]
#[test]
fn concurrent_long_probes_keep_the_reader_lease_and_enforce_the_timeout() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "slow.sh",
        concat!(
            "#!/bin/sh\nsleep 300 </dev/null >/dev/null 2>&1 &\nworker=$!\n",
            "trap 'wait \"$worker\"; exit 143' TERM\n",
            "printf '%s\\n%s\\n' \"$worker\" \"$$\" > pids\nwait",
        ),
    );

    let add = room
        .watch_cmd("sentry")
        .current_dir(room.path())
        .args(["add", "slow", "--", script.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(add.status.success(), "{:?}", add.stderr);
    room.set_ok(
        "sentry",
        &[
            "add",
            "overlap",
            "--",
            "sleep 48; printf 'fp\\tcompleted\\n'",
        ],
    );
    let start = Instant::now();
    let mut reader = room.start_reader("sentry");
    let pid = reader.pid();
    let first_beat = reader.until(Duration::from_secs(5), || {
        let active = room.listing("sentry")["reader"]["active"].clone();
        active["last_beat"].as_str().map(str::to_owned)
    });
    let status = reader.until(Duration::from_secs(75), || {
        let listing = room.listing("sentry");
        assert_eq!(listing["reader"]["active"]["pid"], pid);
        (listing["watches"][1]["last_result"] == "read failed").then_some(listing)
    });
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_secs(59),
        "probe ended too early: {elapsed:?}"
    );
    assert_ne!(status["reader"]["active"]["last_beat"], first_beat);
    assert_eq!(status["watches"][0]["last_result"], "ok, 1 item");
    let inbox = room.inbox("sentry");
    let messages = inbox["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["body"], "overlap | completed");
    assert_eq!(
        messages[1]["body"],
        "slow | read failed: probe timed out after 60 seconds"
    );

    reader.terminate().unwrap();
    assert!(reader.wait_within(Duration::from_secs(5)).status.success());
    let stopped = room.listing("sentry");
    assert_eq!(stopped["reader"]["state"], "none");
    assert!(stopped["reader"]["since"].is_string());

    let pid_file = room.path().join("pids");
    let pid_text = std::fs::read_to_string(&pid_file).unwrap();
    let pids: Vec<libc::pid_t> = pid_text
        .lines()
        .map(|line| line.trim().parse::<libc::pid_t>().unwrap())
        .collect();
    assert_eq!(
        pids.len(),
        2,
        "expected grandchild and shell PIDs: {pid_text}"
    );

    for pid in pids {
        assert!(pid > 0);
        // SAFETY: signal 0 probes process existence without delivering a signal.
        let rc = unsafe { libc::kill(pid, 0) };
        let err = std::io::Error::last_os_error().raw_os_error();
        assert!(
            rc == -1 && err == Some(libc::ESRCH),
            "process {pid} still alive after watch exited (rc={rc}, errno={err:?})"
        );
    }
}

fn rows(watches: &[Value]) -> Vec<(&str, &str, &str)> {
    watches
        .iter()
        .map(|watch| {
            (
                watch["name"].as_str().unwrap(),
                watch["to"].as_str().unwrap(),
                watch["state"].as_str().unwrap(),
            )
        })
        .collect()
}

#[test]
fn same_name_watches_are_scoped_to_each_recipient() {
    let room = Room::new();
    room.join("alpha");
    room.join("beta");
    room.set_ok("alpha", &["add", "ci", "--every", "300", "--", "true"]);
    room.set_ok("beta", &["add", "ci", "--", "true"]);
    room.set_ok("beta", &["add", "rel", "--to", "alpha", "--", "true"]);
    let watches = room.watches("beta");
    assert_eq!(
        rows(&watches),
        [
            ("ci", "alpha", "on"),
            ("ci", "beta", "on"),
            ("rel", "alpha", "on")
        ]
    );
    assert_eq!(watches[0]["every_seconds"], 300);
    assert_eq!(watches[1]["every_seconds"], 120);
    assert_eq!(watches[2]["added_by"], "beta");

    let paused = room.set_ok("alpha", &["pause", "ci"]);
    assert_eq!(
        paused,
        serde_json::json!({"name": "ci", "to": "alpha", "state": "paused", "changed": true})
    );
    assert_eq!(
        rows(&room.watches("alpha")),
        [
            ("ci", "alpha", "paused"),
            ("ci", "beta", "on"),
            ("rel", "alpha", "on")
        ]
    );
    let again = room.set_ok("alpha", &["pause", "ci"]);
    assert_eq!(again["changed"], false);

    let resumed = room.set_ok("beta", &["resume", "ci", "--to", "alpha"]);
    assert_eq!(resumed["state"], "on");
    assert_eq!(resumed["changed"], true);

    room.set_ok("beta", &["remove", "ci"]);
    room.set_ok("alpha", &["remove", "rel"]);
    assert_eq!(rows(&room.watches("beta")), [("ci", "alpha", "on")]);
}

#[test]
fn watch_set_refuses_invalid_names_intervals_and_recipients() {
    let room = Room::new();
    room.join("alpha");
    let too_long = "a".repeat(33);
    for name in ["", "Bad", "has_underscore", "has space", too_long.as_str()] {
        assert_eq!(
            room.set_refused("alpha", &["add", name, "--", "true"]),
            "invalid_watch_name",
            "{name:?}"
        );
    }
    assert_eq!(
        room.set_refused("alpha", &["add", "ci", "--to", "ghost", "--", "true"]),
        "unknown_recipient"
    );
    assert_eq!(
        room.set_refused("alpha", &["add", "ci", "--to", "bad name", "--", "true"]),
        "invalid_identifier"
    );
    assert_eq!(
        room.set_refused("alpha", &["add", "ci", "--", " "]),
        "invalid_command"
    );
    assert_eq!(
        room.set_refused("ghost", &["add", "ci", "--", "true"]),
        "unknown_seat"
    );
    assert_eq!(
        room.set_refused("alpha", &["pause", "missing"]),
        "unknown_watch"
    );
    assert_eq!(
        room.set_refused("alpha", &["resume", "missing"]),
        "unknown_watch"
    );
    assert_eq!(
        room.set_refused("alpha", &["remove", "missing"]),
        "unknown_watch"
    );
    assert_eq!(
        room.set_refused("alpha", &["remove", "missing", "--to", "ghost"]),
        "unknown_recipient"
    );

    for args in [
        &["add", "ci", "--every", "9", "--", "true"][..],
        &["add", "ci", "--every", "soon", "--", "true"],
        &["add", "ci"],
        &["--probe", "a=true"],
        &["--timeout", "5", "list"],
        &["wait", "--probe", "a=true", "--to", "alpha"],
    ] {
        assert_eq!(
            room.set_run("alpha", args).status.code(),
            Some(2),
            "{args:?}"
        );
    }
    assert!(room.watches("alpha").is_empty());
}

#[cfg(unix)]
#[test]
fn watch_round_trips_command_directory_interval_state_and_attribution() {
    let room = Room::new();
    room.join("alpha");
    room.join("beta");
    let dir = std::fs::canonicalize(room.path()).unwrap().join("work");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(
        dir.join("probe.sh"),
        "#!/bin/sh\nprintf '%s|' \"$@\"\npwd\n",
    )
    .unwrap();

    let out = room
        .watch_cmd("alpha")
        .current_dir(&dir)
        .args(["add", "args", "--to", "beta", "--every", "45", "--"])
        .args(["sh", "./probe.sh", "two words", "it's", "$HOME", ""])
        .output()
        .unwrap();
    assert!(out.status.success());
    let added: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(added["replaced"], false);
    assert_eq!(added["to"], "beta");

    let watches = room.watches("beta");
    assert_eq!(watches.len(), 1);
    let watch = &watches[0];
    assert_eq!(watch["name"], "args");
    assert_eq!(watch["to"], "beta");
    assert_eq!(watch["every_seconds"], 45);
    assert_eq!(watch["state"], "on");
    assert_eq!(watch["added_by"], "alpha");
    assert_eq!(watch["cwd"], dir.to_str().unwrap());
    assert!(watch["added_at"].as_str().unwrap().ends_with('Z'));
    assert!(watch["last_read"].is_null());
    assert!(watch["last_result"].is_null());
    assert_eq!(watch["sent"], 0);

    let ran = Command::new("sh")
        .args(["-c", watch["command"].as_str().unwrap()])
        .current_dir(watch["cwd"].as_str().unwrap())
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(ran.stdout).unwrap(),
        format!("two words|it's|$HOME||{}\n", dir.display())
    );

    let single = room
        .watch_cmd("alpha")
        .args(["add", "pipe", "--", "echo a | cat"])
        .output()
        .unwrap();
    assert!(single.status.success());
    let stored = room.watches("alpha");
    assert_eq!(stored[1]["command"], "echo a | cat");
}

#[cfg(unix)]
#[test]
fn replacing_a_watch_keeps_fingerprints_and_removing_it_forgets_them() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "probe.sh",
        "#!/bin/sh\nprintf 'fp-1\\tred\\nfp-2\\tblue\\n'\n",
    );
    let command = script.to_str().unwrap();
    let probe = format!("ci={command}");
    room.set_ok("sentry", &["add", "ci", "--", command]);
    let first = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(first["items"].as_array().unwrap().len(), 2);
    assert_eq!(room.watches("sentry")[0]["sent"], 2);

    room.set_ok("sentry", &["pause", "ci"]);
    let replaced = room.set_ok("sentry", &["add", "ci", "--every", "30", "--", "true"]);
    assert_eq!(replaced["replaced"], true);
    let watches = room.watches("sentry");
    assert_eq!(watches.len(), 1);
    assert_eq!(watches[0]["state"], "on");
    assert_eq!(watches[0]["every_seconds"], 30);
    assert_eq!(watches[0]["command"], "true");
    assert_eq!(watches[0]["sent"], 2);
    let second = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(second["items"], serde_json::json!([]));

    let removed = room.set_ok("sentry", &["remove", "ci"]);
    assert_eq!(removed["forgotten"], 2);
    assert!(room.watches("sentry").is_empty());
    room.set_ok("sentry", &["add", "ci", "--", "true"]);
    assert_eq!(room.watches("sentry")[0]["sent"], 0);
    let third = room.watch_ok("sentry", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(third["items"].as_array().unwrap().len(), 2);
}

#[test]
fn list_in_an_empty_room_reports_no_reader_and_no_watches() {
    let room = Room::new();
    room.join("alpha");
    let listing = room.set_ok("alpha", &["list", "--json"]);
    assert_eq!(
        listing,
        serde_json::json!({
            "reader": {"state": "none", "since": null, "active": null, "standbys": []},
            "watches": []
        })
    );
    let text = room.set_run("alpha", &["list"]);
    assert_eq!(String::from_utf8_lossy(&text.stdout), "reader: none\n");
    assert_eq!(
        String::from_utf8_lossy(&text.stderr),
        "No watches in this room.\n"
    );
}

#[cfg(unix)]
#[test]
fn live_reader_picks_up_edits_and_honors_each_watch_schedule() {
    let room = Room::new();
    for seat in ["reader", "alpha", "beta"] {
        room.join(seat);
    }
    let mut reader = SpawnedChild::spawn(
        room.watch_cmd("reader")
            .env("TEST_PROBE_MARKER", "reader-env")
            .arg("run"),
    );
    let pid = reader.pid();
    reader.until(Duration::from_secs(5), || {
        (room.listing("reader")["reader"]["active"]["pid"] == pid).then_some(())
    });
    let fast = room.path().join("fast");
    let slow = room.path().join("slow");
    for path in [&fast, &slow] {
        std::fs::create_dir(path).unwrap();
        std::fs::write(
            path.join("input"),
            path.file_name().unwrap().as_encoded_bytes(),
        )
        .unwrap();
    }
    let probe = "read_label=$(cat input) || exit 1; date +%s >> reads; \
                 printf 'fp\\t%s %s\\n' \"$read_label\" \"$TEST_PROBE_MARKER\"";
    for (path, seat, interval) in [(&fast, "alpha", "10"), (&slow, "beta", "15")] {
        let add = room
            .watch_cmd("reader")
            .current_dir(path)
            .args([
                "add", "test", "--to", seat, "--every", interval, "--", probe,
            ])
            .output()
            .unwrap();
        assert!(add.status.success(), "{:?}", add.stderr);
    }
    reader.until(Duration::from_secs(20), || {
        let listing = room.listing("reader");
        assert_eq!(listing["reader"]["active"]["pid"], pid);
        listing["watches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|watch| watch["last_result"] == "ok, 1 item")
            .then_some(())
    });
    assert_eq!(
        room.inbox("alpha")["messages"][0]["body"],
        "test | fast reader-env"
    );
    assert_eq!(
        room.inbox("beta")["messages"][0]["body"],
        "test | slow reader-env"
    );
    let reads = |path: &Path| {
        std::fs::read_to_string(path.join("reads"))
            .unwrap()
            .lines()
            .map(|line| line.parse::<u64>().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(reads(&slow).len(), 1);
    let fast_reads = reader.until(Duration::from_secs(15), || {
        let reads = reads(&fast);
        (reads.len() >= 2).then_some(reads)
    });
    assert!(fast_reads[1].saturating_sub(fast_reads[0]) >= 10);
    assert_eq!(reads(&slow).len(), 1);
    room.set_ok("reader", &["pause", "test", "--to", "alpha"]);
    room.set_ok("reader", &["remove", "test", "--to", "beta"]);
    let fast_count = reads(&fast).len();

    // Use another watch with the minimum interval as a clock for a full pass after both edits.
    room.set_ok("reader", &["add", "control", "--every", "10", "--", "true"]);
    let control_read = || room.watches("reader")[0]["last_read"].clone();
    let first_read = reader.until(Duration::from_secs(5), || {
        let read = control_read();
        (!read.is_null()).then_some(read)
    });
    reader.until(Duration::from_secs(15), || {
        (control_read() != first_read).then_some(())
    });
    assert_eq!(reads(&fast).len(), fast_count);
    assert_eq!(reads(&slow).len(), 1);
    assert_eq!(room.inbox("alpha")["messages"].as_array().unwrap().len(), 1);
    room.set_ok("reader", &["stop"]);
    assert!(reader.wait_within(Duration::from_secs(5)).status.success());
}

#[cfg(unix)]
#[test]
fn standby_takes_over_after_kill_and_stop_exits_all_invocations() {
    let room = Room::new();
    for seat in ["alpha", "beta", "recipient"] {
        room.join(seat);
    }
    let probe = concat!(
        "printf 'same\\tfirst item\\n'\n",
        "if [ -f new-item ]; then printf 'new\\tsecond item\\n'; fi",
    );
    let add = room
        .watch_cmd("recipient")
        .current_dir(room.path())
        .args(["add", "ci", "--every", "10", "--", probe])
        .output()
        .unwrap();
    assert!(add.status.success(), "{:?}", add.stderr);
    room.set_ok("recipient", &["add", "control", "--", "true"]);
    let mut alpha = room.start_reader("alpha");
    let beta = room.start_reader("beta");
    let alpha_pid = alpha.pid();
    let beta_pid = beta.pid();
    let elected = alpha.until(Duration::from_secs(8), || {
        let listing = room.listing("recipient");
        let status = &listing["reader"];
        (status["active"].is_object() && status["standbys"][0].is_object()).then(|| status.clone())
    });
    assert_eq!(elected["state"], "live");
    let (seat, mut holder, mut standby) = if elected["active"]["pid"] == alpha_pid {
        ("alpha", alpha, beta)
    } else {
        assert_eq!(elected["active"]["pid"], beta_pid);
        ("beta", beta, alpha)
    };
    assert_eq!(elected["standbys"].as_array().unwrap().len(), 1);
    assert_eq!(elected["standbys"][0]["pid"], standby.pid());
    let standby_seat = elected["standbys"][0]["seat"].as_str().unwrap();
    let table = room.set_run("recipient", &["list"]);
    let header = String::from_utf8_lossy(&table.stdout);
    assert!(header.starts_with(&format!("reader: {seat}, pid {}", holder.pid())));
    assert!(header.contains(&format!("; standby: {standby_seat}, pid {}", standby.pid())));
    let control_read = holder.until(Duration::from_secs(5), || {
        let inbox = room.inbox("recipient");
        let messages = inbox["messages"].as_array().unwrap();
        assert!(messages.len() <= 1);
        let first = messages.first()?;
        assert_eq!(first["sender"], seat);
        let read = room.watches("recipient")[1]["last_read"].clone();
        read.is_string().then_some(read)
    });

    holder.kill().unwrap();
    assert!(!holder.wait_within(Duration::from_secs(5)).status.success());
    std::fs::write(room.path().join("new-item"), "").unwrap();
    let standby_pid = standby.pid();
    standby.until(Duration::from_secs(60), || {
        let listing = room.listing("recipient");
        let status = &listing["reader"];
        (status["active"]["pid"] == standby_pid
            && status["standbys"].as_array().unwrap().is_empty()
            && listing["watches"][0]["sent"] == 2)
            .then_some(())
    });
    assert_eq!(room.watches("recipient")[1]["last_read"], control_read);
    let inbox = room.inbox("recipient");
    let messages = inbox["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1]["sender"], standby_seat);
    assert_eq!(messages[1]["body"], "ci | second item");

    let replacement = room.start_reader(seat);
    let replacement_pid = replacement.pid();
    standby.until(Duration::from_secs(5), || {
        let listing = room.listing("recipient");
        (listing["reader"]["standbys"][0]["pid"] == replacement_pid).then_some(())
    });
    room.set_ok("recipient", &["stop"]);
    assert!(standby.wait_within(Duration::from_secs(5)).status.success());
    assert!(
        replacement
            .wait_within(Duration::from_secs(5))
            .status
            .success()
    );
    let listing = room.listing("recipient");
    assert_eq!(listing["reader"]["state"], "stopped");
    assert!(listing["reader"]["active"].is_null());
    assert!(listing["reader"]["since"].is_string());
    assert_eq!(listing["reader"]["standbys"], serde_json::json!([]));
    let table = room.set_run("recipient", &["list"]);
    assert!(String::from_utf8_lossy(&table.stdout).starts_with("reader: stopped since "));

    let mut restarted = room.start_reader(seat);
    let restarted_pid = restarted.pid();
    restarted.until(Duration::from_secs(5), || {
        let listing = room.listing("recipient");
        (listing["reader"]["active"]["pid"] == restarted_pid).then_some(())
    });
    room.set_ok("recipient", &["stop"]);
    assert!(
        restarted
            .wait_within(Duration::from_secs(5))
            .status
            .success()
    );
}

/// Run `check` on a command that must fail: exit 1, nothing on stdout, and the error document.
fn check_error(room: &Room, seat: &str, command: &str) -> Value {
    let out = room.set_run(seat, &["check", "--", command]);
    assert_eq!(out.status.code(), Some(1), "{command}");
    assert!(out.stdout.is_empty());
    serde_json::from_slice(&out.stderr).unwrap()
}

#[cfg(unix)]
#[test]
fn check_prints_parsed_items_and_the_first_real_read_sends_them_once() {
    let room = Room::new();
    room.join("sentry");
    room.join("captain");
    let runs = room.path().join("runs");
    let script = room.script(
        "probe.sh",
        &format!(
            "#!/bin/sh\necho run >> '{}'\n{}",
            runs.display(),
            concat!(
                "printf 'fp-1\\tbuild red on main\\nfp-1\\tsame item again\\n",
                "fp-2\\tlint failed\\n'\n",
            ),
        ),
    );
    let command = script.to_str().unwrap();

    let checked = room.set_ok("sentry", &["check", "--", command]);
    assert_eq!(
        checked,
        serde_json::json!({"items": [
            {"fingerprint": "fp-1", "text": "build red on main"},
            {"fingerprint": "fp-2", "text": "lint failed"},
        ]})
    );
    assert_eq!(std::fs::read_to_string(&runs).unwrap(), "run\n");
    // A dry run leaves the room as it was: no watch, no reader lease and no mail.
    assert_eq!(room.listing("sentry")["watches"], serde_json::json!([]));
    assert_eq!(room.listing("sentry")["reader"]["state"], "none");
    assert_eq!(room.inbox("captain")["messages"], serde_json::json!([]));

    room.set_ok("captain", &["add", "ci", "--every", "10", "--", command]);
    let mut reader = room.start_reader("sentry");
    let inbox = reader.until(Duration::from_secs(10), || {
        let inbox = room.inbox("captain");
        (inbox["messages"].as_array().unwrap().len() == 1).then_some(inbox)
    });
    assert_eq!(
        inbox["messages"][0]["body"],
        "ci | build red on main\nci | lint failed"
    );

    // The reader makes a second pass once the interval has passed. It mails nothing new.
    let first_read = room.watches("captain")[0]["last_read"].clone();
    reader.until(Duration::from_secs(15), || {
        (room.watches("captain")[0]["last_read"] != first_read).then_some(())
    });
    assert_eq!(
        room.inbox("captain")["messages"].as_array().unwrap().len(),
        1
    );
    let watches = room.watches("captain");
    assert_eq!(watches[0]["sent"], 2);
    assert_eq!(watches[0]["last_result"], "ok, 2 items");

    room.set_ok("captain", &["stop"]);
    assert!(reader.wait_within(Duration::from_secs(5)).status.success());
}

#[cfg(unix)]
#[test]
fn check_reports_an_empty_result_as_success() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script("quiet.sh", "#!/bin/sh\nexit 0\n");

    let out = room.set_run("sentry", &["check", "--", script.to_str().unwrap()]);
    assert!(out.status.success(), "{:?}", out.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        serde_json::json!({"items": []})
    );
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "No items: the command printed nothing.\n"
    );
}

#[cfg(unix)]
#[test]
fn check_points_at_the_line_that_breaks_the_contract() {
    let room = Room::new();
    room.join("sentry");
    let no_tab = room.script(
        "no-tab.sh",
        "#!/bin/sh\nprintf 'fp-1\\tfine\\nbare text\\n'\n",
    );
    let no_fingerprint = room.script(
        "no-fingerprint.sh",
        "#!/bin/sh\nprintf 'fp-1\\tfine\\n\\tno fingerprint\\n'\n",
    );

    assert_eq!(
        check_error(&room, "sentry", no_tab.to_str().unwrap()),
        serde_json::json!({
            "code": "probe_contract",
            "error": "probe output breaks the contract: line 2: expected <fingerprint><TAB><text>",
        })
    );
    assert_eq!(
        check_error(&room, "sentry", no_fingerprint.to_str().unwrap()),
        serde_json::json!({
            "code": "probe_contract",
            "error": concat!(
                "probe output breaks the contract: ",
                "line 2: fingerprint must be 1 to 200 characters",
            ),
        })
    );
}

#[cfg(unix)]
#[test]
fn check_reports_a_failed_run_with_its_first_diagnostic_line() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "broken.sh",
        "#!/bin/sh\necho 'upstream unavailable' >&2\nexit 3\n",
    );

    assert_eq!(
        check_error(&room, "sentry", script.to_str().unwrap()),
        serde_json::json!({"code": "probe_failed", "error": "upstream unavailable"})
    );
}

#[cfg(unix)]
#[test]
fn check_reports_a_run_past_the_reader_limit() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script("stuck.sh", "#!/bin/sh\nsleep 300\n");

    let started = Instant::now();
    let error = check_error(&room, "sentry", script.to_str().unwrap());
    assert!(
        started.elapsed() >= Duration::from_secs(59),
        "run ended too early: {:?}",
        started.elapsed()
    );
    assert_eq!(
        error,
        serde_json::json!({"code": "probe_failed", "error": "probe timed out after 60 seconds"})
    );
}

#[cfg(unix)]
#[test]
fn check_runs_the_command_from_the_callers_directory() {
    let room = Room::new();
    room.join("sentry");
    let dir = room.path().join("work");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("input"), "from-work").unwrap();

    let out = room
        .watch_cmd("sentry")
        .current_dir(&dir)
        .args(["check", "--", "printf 'fp\\t%s\\n' \"$(cat input)\""])
        .output()
        .unwrap();
    assert!(out.status.success(), "{:?}", out.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        serde_json::json!({"items": [{"fingerprint": "fp", "text": "from-work"}]})
    );
}

#[test]
fn check_requires_a_command_after_double_dash() {
    let room = Room::new();
    room.join("sentry");

    let out = room.set_run("sentry", &["check"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[cfg(unix)]
#[test]
fn check_refuses_a_seat_that_has_not_joined() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script("probe.sh", "#!/bin/sh\nprintf 'fp\\tfine\\n'\n");

    assert_eq!(
        check_error(&room, "stranger", script.to_str().unwrap()),
        serde_json::json!({
            "code": "unknown_seat",
            "error": "seat has not joined this room; run `formation mail join` first",
        })
    );
}

/// Build a probe script that steps through `actions` on successive invocations, repeating the last.
#[cfg(unix)]
fn sequence_script(room: &Room, name: &str, actions: &[&str]) -> (PathBuf, PathBuf) {
    let count_file = room.path().join(format!("{name}.count"));
    let mut cases = String::new();
    for (index, action) in actions.iter().enumerate() {
        let step = index + 1;
        let pattern = if step == actions.len() {
            "*".to_owned()
        } else {
            step.to_string()
        };
        cases.push_str(&format!("  {pattern}) {action} ;;\n"));
    }
    let body = format!(
        concat!(
            "#!/bin/sh\n",
            "count_file='{count}'\n",
            "n=0\n",
            "if [ -f \"$count_file\" ]; then n=$(cat \"$count_file\"); fi\n",
            "n=$((n + 1))\n",
            "printf '%s\\n' \"$n\" > \"$count_file\"\n",
            "case \"$n\" in\n",
            "{cases}",
            "esac\n",
        ),
        count = count_file.display(),
        cases = cases,
    );
    let script = room.script(&format!("{name}.sh"), &body);
    (script, count_file)
}

#[cfg(unix)]
fn call_count(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

#[cfg(unix)]
#[test]
fn wait_forgets_after_two_clean_omissions_and_ignores_interleaved_failures() {
    let room = Room::new();
    room.join("alpha");
    room.join("beta");
    let mode_file = room.path().join("mode");
    let script = room.script(
        "state.sh",
        &format!(
            concat!(
                "#!/bin/sh\n",
                "mode=$(cat '{mode}')\n",
                "case \"$mode\" in\n",
                "  present) printf 'fp-1\\tservice degraded\\n' ;;\n",
                "  absent) exit 0 ;;\n",
                "  fail) echo 'probe offline' >&2; exit 1 ;;\n",
                "esac\n",
            ),
            mode = mode_file.display(),
        ),
    );
    let probe = format!("ci={}", script.display());
    let other_probe = format!("other={}", script.display());

    // Initial present on alpha/ci, beta/ci, and alpha/other.
    std::fs::write(&mode_file, "present").unwrap();
    for (seat, spec) in [
        ("alpha", probe.as_str()),
        ("beta", probe.as_str()),
        ("alpha", other_probe.as_str()),
    ] {
        let out = room.watch_ok(seat, &["wait", "--probe", spec, "--timeout", "0"]);
        assert_eq!(out["items"].as_array().unwrap().len(), 1);
    }

    // Single omission followed by return stays quiet and resets the absence streak.
    std::fs::write(&mode_file, "absent").unwrap();
    let miss_one = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(miss_one["expired"], true);
    std::fs::write(&mode_file, "present").unwrap();
    let returned_early = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(returned_early["expired"], true);
    assert_eq!(returned_early["items"], serde_json::json!([]));

    // Interleaved failures after one omission never advance the threshold to two.
    std::fs::write(&mode_file, "absent").unwrap();
    room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    std::fs::write(&mode_file, "fail").unwrap();
    let first_fail = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(
        first_fail["failed"],
        serde_json::json!([{"probe": "ci", "error": "probe offline"}])
    );
    let second_fail = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(second_fail["expired"], true);
    assert_eq!(second_fail["failed"], serde_json::json!([]));
    std::fs::write(&mode_file, "present").unwrap();
    let still_remembered = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(still_remembered["expired"], true);
    assert_eq!(still_remembered["items"], serde_json::json!([]));

    // Two consecutive successful omissions (with a failure between) forget alpha/ci only.
    std::fs::write(&mode_file, "absent").unwrap();
    room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    std::fs::write(&mode_file, "fail").unwrap();
    let mid_fail = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(mid_fail["failed"].as_array().unwrap().len(), 1);
    std::fs::write(&mode_file, "absent").unwrap();
    let forgotten = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(forgotten["expired"], true);

    std::fs::write(&mode_file, "present").unwrap();
    let renotified = room.watch_ok("alpha", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(renotified["expired"], false);
    assert_eq!(renotified["items"].as_array().unwrap().len(), 1);
    assert_eq!(renotified["items"][0]["fingerprint"], "fp-1");

    // Same watch name for beta and other watch name for alpha remain untouched.
    let beta = room.watch_ok("beta", &["wait", "--probe", &probe, "--timeout", "0"]);
    assert_eq!(beta["expired"], true);
    assert_eq!(beta["items"], serde_json::json!([]));
    let other = room.watch_ok(
        "alpha",
        &["wait", "--probe", &other_probe, "--timeout", "0"],
    );
    assert_eq!(other["expired"], true);
    assert_eq!(other["items"], serde_json::json!([]));
}

#[cfg(unix)]
#[test]
fn reader_mails_returning_conditions_after_two_omissions_scoped_by_watch() {
    let room = Room::new();
    for seat in ["sentry", "alpha", "beta"] {
        room.join(seat);
    }

    let (cycle_alpha, cycle_alpha_count) = sequence_script(
        &room,
        "cycle-alpha",
        &[
            "printf 'fp-1\\talpha alert\\n'",
            "exit 0",
            "exit 0",
            "printf 'fp-1\\talpha alert\\n'",
        ],
    );
    let (cycle_beta, cycle_beta_count) =
        sequence_script(&room, "cycle-beta", &["printf 'fp-1\\tbeta alert\\n'"]);
    let (flicker, flicker_count) = sequence_script(
        &room,
        "flicker",
        &[
            "printf 'fp-f\\tflicker alert\\n'",
            "exit 0",
            "printf 'fp-f\\tflicker alert\\n'",
            "exit 0",
            "printf 'fp-f\\tflicker alert\\n'",
        ],
    );
    let (flaky, flaky_count) = sequence_script(
        &room,
        "flaky",
        &[
            "printf 'fp-k\\tflaky alert\\n'",
            "exit 0",
            "echo 'upstream down' >&2; exit 1",
            "echo 'upstream down' >&2; exit 1",
            "printf 'fp-k\\tflaky alert\\n'",
        ],
    );
    let (fail_gap, fail_gap_count) = sequence_script(
        &room,
        "fail-gap",
        &[
            "printf 'fp-g\\tgap alert\\n'",
            "exit 0",
            "echo 'blip error' >&2; exit 1",
            "exit 0",
            "printf 'fp-g\\tgap alert\\n'",
        ],
    );

    room.set_ok(
        "alpha",
        &[
            "add",
            "cycle",
            "--every",
            "10",
            "--",
            cycle_alpha.to_str().unwrap(),
        ],
    );
    room.set_ok(
        "beta",
        &[
            "add",
            "cycle",
            "--every",
            "10",
            "--",
            cycle_beta.to_str().unwrap(),
        ],
    );
    room.set_ok(
        "alpha",
        &[
            "add",
            "flicker",
            "--every",
            "10",
            "--",
            flicker.to_str().unwrap(),
        ],
    );
    room.set_ok(
        "alpha",
        &[
            "add",
            "flaky",
            "--every",
            "10",
            "--",
            flaky.to_str().unwrap(),
        ],
    );
    room.set_ok(
        "beta",
        &[
            "add",
            "fail-gap",
            "--every",
            "10",
            "--",
            fail_gap.to_str().unwrap(),
        ],
    );

    let mut reader = room.start_reader("sentry");
    reader.until(Duration::from_secs(55), || {
        let ready = call_count(&cycle_alpha_count) >= 4
            && call_count(&cycle_beta_count) >= 4
            && call_count(&flicker_count) >= 5
            && call_count(&flaky_count) >= 5
            && call_count(&fail_gap_count) >= 5;
        if !ready {
            return None;
        }
        let alpha_bodies: Vec<String> = room.inbox("alpha")["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|msg| msg["body"].as_str().unwrap().to_owned())
            .collect();
        let beta_bodies: Vec<String> = room.inbox("beta")["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|msg| msg["body"].as_str().unwrap().to_owned())
            .collect();
        let alpha_done = alpha_bodies
            .iter()
            .filter(|body| *body == "cycle | alpha alert")
            .count()
            == 2
            && alpha_bodies
                .iter()
                .any(|body| body == "flaky | read failed: upstream down");
        let beta_done = beta_bodies
            .iter()
            .filter(|body| *body == "fail-gap | gap alert")
            .count()
            == 2;
        (alpha_done && beta_done).then_some(())
    });

    room.set_ok("sentry", &["stop"]);
    assert!(reader.wait_within(Duration::from_secs(5)).status.success());

    let alpha_bodies: Vec<String> = room.inbox("alpha")["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|msg| msg["body"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        alpha_bodies
            .iter()
            .filter(|body| *body == "cycle | alpha alert")
            .count(),
        2,
        "alpha/cycle should mail twice: {alpha_bodies:?}"
    );
    assert_eq!(
        alpha_bodies
            .iter()
            .filter(|body| *body == "flicker | flicker alert")
            .count(),
        1,
        "alpha/flicker should mail once: {alpha_bodies:?}"
    );
    assert_eq!(
        alpha_bodies
            .iter()
            .filter(|body| *body == "flaky | flaky alert")
            .count(),
        1,
        "alpha/flaky should mail once despite interleaved failures: {alpha_bodies:?}"
    );
    assert_eq!(
        alpha_bodies
            .iter()
            .filter(|body| *body == "flaky | read failed: upstream down")
            .count(),
        1,
        "alpha/flaky failure should mail once: {alpha_bodies:?}"
    );

    let beta_bodies: Vec<String> = room.inbox("beta")["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|msg| msg["body"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        beta_bodies
            .iter()
            .filter(|body| *body == "cycle | beta alert")
            .count(),
        1,
        "beta/cycle must stay isolated from alpha/cycle: {beta_bodies:?}"
    );
    assert_eq!(
        beta_bodies
            .iter()
            .filter(|body| *body == "fail-gap | gap alert")
            .count(),
        2,
        "beta/fail-gap should mail twice after two clean omissions: {beta_bodies:?}"
    );
    assert_eq!(
        beta_bodies
            .iter()
            .filter(|body| *body == "fail-gap | read failed: blip error")
            .count(),
        1,
        "beta/fail-gap failure should mail once: {beta_bodies:?}"
    );
}

#[cfg(unix)]
#[test]
fn returning_condition_reuses_persisted_absence_across_reader_takeover() {
    let room = Room::new();
    for seat in ["alpha", "beta", "recipient"] {
        room.join(seat);
    }
    let (script, count_file) = sequence_script(
        &room,
        "takeover",
        &[
            "printf 'fp-1\\ttakeover alert\\n'",
            "exit 0",
            "exit 0",
            "printf 'fp-1\\ttakeover alert\\n'",
        ],
    );
    room.set_ok(
        "recipient",
        &["add", "ci", "--every", "10", "--", script.to_str().unwrap()],
    );

    let mut first = room.start_reader("alpha");
    let second = room.start_reader("beta");
    let first_pid = first.pid();
    let elected = first.until(Duration::from_secs(8), || {
        let status = room.listing("recipient")["reader"].clone();
        (status["active"].is_object() && status["standbys"][0].is_object()).then_some(status)
    });
    let (first_seat, mut holder, standby_seat, mut standby) =
        if elected["active"]["pid"] == first_pid {
            ("alpha", first, "beta", second)
        } else {
            ("beta", second, "alpha", first)
        };

    // Wait for pass 1 (present -> mailed) and pass 2 (first clean omission -> persisted).
    holder.until(Duration::from_secs(20), || {
        let watches = room.watches("recipient");
        (call_count(&count_file) >= 2 && watches[0]["last_result"] == "ok, 0 items").then_some(())
    });
    assert_eq!(room.watches("recipient")[0]["sent"], 1);
    assert_eq!(
        room.inbox("recipient")["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // Gracefully release the first reader so the standby takes over immediately.
    holder.terminate().unwrap();
    assert!(holder.wait_within(Duration::from_secs(5)).status.success());
    let standby_pid = standby.pid();
    standby.until(Duration::from_secs(5), || {
        (room.listing("recipient")["reader"]["active"]["pid"] == standby_pid).then_some(())
    });

    // Pass 3 (second clean omission) forgets fp-1; pass 4 (present) mails it again via standby.
    standby.until(Duration::from_secs(28), || {
        let inbox = room.inbox("recipient");
        (inbox["messages"].as_array().unwrap().len() == 2).then_some(())
    });
    room.set_ok("recipient", &["stop"]);
    assert!(standby.wait_within(Duration::from_secs(5)).status.success());

    let inbox = room.inbox("recipient");
    let messages = inbox["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["sender"], first_seat);
    assert_eq!(messages[0]["body"], "ci | takeover alert");
    assert_eq!(messages[1]["sender"], standby_seat);
    assert_eq!(messages[1]["body"], "ci | takeover alert");
}
