//! Integration tests for `formation watch` through the built executable.
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

use cw_testkit::TempRoot;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

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

struct SpawnedChild(Option<Child>);

impl SpawnedChild {
    fn spawn(cmd: &mut Command) -> Self {
        let child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        Self(Some(child))
    }

    fn wait_within(mut self, timeout: Duration) -> Output {
        let mut child = self.0.take().unwrap();
        let start = Instant::now();
        loop {
            if child.try_wait().unwrap().is_some() {
                return child.wait_with_output().unwrap();
            }
            if start.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                panic!("child did not exit within {timeout:?}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for SpawnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
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

    let first = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
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

    let second = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
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

    let first_fail = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
    assert_eq!(first_fail["expired"], false);
    assert_eq!(first_fail["items"], serde_json::json!([]));
    assert_eq!(
        first_fail["failed"],
        serde_json::json!([{"probe": "ci", "error": "upstream down"}])
    );

    let repeat_fail = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
    assert_eq!(repeat_fail["expired"], true);
    assert_eq!(repeat_fail["items"], serde_json::json!([]));
    assert_eq!(repeat_fail["failed"], serde_json::json!([]));

    std::fs::write(&mode_file, "ok").unwrap();
    let recovered = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
    assert_eq!(recovered["expired"], true);
    assert_eq!(recovered["failed"], serde_json::json!([]));

    std::fs::write(&mode_file, "fail").unwrap();
    let fail_again = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
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

    let out = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
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
fn to_mails_lines_and_chunks_25_items_into_20_and_5() {
    let room = Room::new();
    room.join("sentry");
    room.join("captain");
    let script = room.script(
        "twenty-five.sh",
        "#!/bin/sh\nidx=1\nwhile [ \"$idx\" -le 25 ]; do\n  printf 'fp-%02d\\titem %02d\\n' \"$idx\" \"$idx\"\n  idx=$((idx + 1))\ndone\n",
    );
    let probe = format!("ci={}", script.display());

    room.watch_ok(
        "sentry",
        &["--probe", &probe, "--to", "captain", "--timeout", "0"],
    );

    let inbox = room.inbox("captain");
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
fn two_watchers_hand_over_each_fingerprint_once_between_them() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "race.sh",
        "#!/bin/sh\nidx=1\nwhile [ \"$idx\" -le 25 ]; do\n  printf 'fp-%02d\\titem %02d\\n' \"$idx\" \"$idx\"\n  idx=$((idx + 1))\ndone\n",
    );
    let probe = format!("ci={}", script.display());

    let mut cmd_a = room.watch_cmd("sentry");
    cmd_a.args(["--probe", &probe, "--timeout", "0"]);
    let mut cmd_b = room.watch_cmd("sentry");
    cmd_b.args(["--probe", &probe, "--timeout", "0"]);

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
fn without_to_returns_20_with_remaining_5_then_other_5() {
    let room = Room::new();
    room.join("sentry");
    let script = room.script(
        "batch.sh",
        "#!/bin/sh\nidx=1\nwhile [ \"$idx\" -le 25 ]; do\n  printf 'fp-%02d\\titem %02d\\n' \"$idx\" \"$idx\"\n  idx=$((idx + 1))\ndone\n",
    );
    let probe = format!("ci={}", script.display());

    let first = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
    assert_eq!(first["expired"], false);
    assert_eq!(first["remaining"], 5);
    let first_items = first["items"].as_array().unwrap();
    assert_eq!(first_items.len(), 20);
    assert_eq!(first_items[0]["fingerprint"], "fp-01");
    assert_eq!(first_items[19]["fingerprint"], "fp-20");

    let second = room.watch_ok("sentry", &["--probe", &probe, "--timeout", "0"]);
    assert_eq!(second["expired"], false);
    assert_eq!(second["remaining"], 0);
    let second_items = second["items"].as_array().unwrap();
    assert_eq!(second_items.len(), 5);
    assert_eq!(second_items[0]["fingerprint"], "fp-21");
    assert_eq!(second_items[4]["fingerprint"], "fp-25");
}

#[cfg(unix)]
#[test]
fn probe_past_60_seconds_is_killed_and_leaves_no_child() {
    let room = Room::new();
    room.join("sentry");
    let pid_file = room.path().join("pids");
    let script = room.script(
        "slow.sh",
        &format!(
            "#!/bin/sh\nsleep 300 </dev/null >/dev/null 2>&1 &\nprintf '%s\\n%s\\n' \"$!\" \"$$\" > '{}'\nwait\n",
            pid_file.display()
        ),
    );
    let probe = format!("slow={}", script.display());

    let start = Instant::now();
    let mut cmd = room.watch_cmd("sentry");
    cmd.args(["--probe", &probe, "--timeout", "120"]);
    let out = SpawnedChild::spawn(&mut cmd).wait_within(Duration::from_secs(90));
    let elapsed = start.elapsed();

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        elapsed >= Duration::from_secs(59),
        "probe ended too early: {elapsed:?}"
    );
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["expired"], false);
    assert_eq!(body["items"], serde_json::json!([]));
    let failed = body["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["probe"], "slow");

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
