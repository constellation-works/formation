//! End-to-end tests for the repository gates.
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        reason = "integration tests assert fixture setup"
    )
)]
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn copy_script(name: &str, dest_root: &Path) -> PathBuf {
    let src = repo_root().join("scripts").join(name);
    let dst = dest_root.join("scripts").join(name);
    fs::create_dir_all(dst.parent().unwrap()).unwrap();
    fs::copy(&src, &dst).unwrap();
    fs::set_permissions(&dst, fs::Permissions::from_mode(0o755)).unwrap();
    dst
}

#[test]
fn deny_uses_a_writable_cache_when_the_cargo_home_is_unusable() {
    // make runs in the kit's temp root and isolated environment.
    let temp = cw_testkit::TempRoot::new("FORMATION").unwrap();
    let root = temp.path();
    for name in ["Makefile", "deny.toml"] {
        fs::copy(repo_root().join(name), root.join(name)).unwrap();
    }
    let cargo = root.join("cargo");
    fs::write(
        &cargo,
        r#"#!/bin/sh
set -eu
test "$1" = deny
if [ "$2" = --version ]; then exit 0; fi
test "$2" = check
test "$3" = --config
grep -Fx 'db-path = "target/cargo-deny/advisory-dbs"' "$4"
grep -Fx 'yanked = "deny"' "$4"
grep -Fx '[licenses]' "$4"
"#,
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
    let cargo_home = root.join("cargo-home");
    fs::write(&cargo_home, b"not a directory").unwrap();
    let out = temp
        .command("make")
        .args(["deny", "CARGO=./cargo"])
        .env("CARGO_HOME", cargo_home)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn check_test_modules_fails_when_parent_lacks_mod_tests() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let script = copy_script("check-test-modules.sh", root);

    // Parent module omits `mod tests;` even though tests/mod.rs declares its child.
    fs::create_dir_all(root.join("crates/demo/src/sub/tests")).unwrap();
    fs::write(root.join("crates/demo/src/sub/mod.rs"), "// no mod tests\n").unwrap();
    fs::write(root.join("crates/demo/src/sub/tests/mod.rs"), "mod unit;\n").unwrap();
    fs::write(
        root.join("crates/demo/src/sub/tests/unit.rs"),
        "#[test] fn t() {}\n",
    )
    .unwrap();

    let out = Command::new(&script).output().unwrap();
    assert_ne!(
        out.status.code(),
        Some(0),
        "expected failure when parent lacks `mod tests;`, got stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("crates/demo/src/sub/tests"),
        "expected error to name the tests directory, got: {stderr}"
    );
}

#[test]
fn check_dependency_direction_catches_grouped_std_fs_import() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let script = copy_script("check-dependency-direction.sh", root);

    let core = root.join("crates/formation-core/src");
    fs::create_dir_all(&core).unwrap();
    fs::write(core.join("lib.rs"), "use std::{fmt, fs};\n").unwrap();

    let out = Command::new(&script).output().unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("the core reaches files only through formation-store")
    );
}

#[test]
fn attention_read_formats_items_and_reports_a_failed_read_in_one_line() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let mode_file = dir.path().join("mode");
    fs::write(&mode_file, "ok").unwrap();

    let orbit = bin.join("orbit");
    let mode_display = mode_file.display();
    let fixture = format!(
        r#"#!/usr/bin/env python3
import json
import sys
from pathlib import Path

mode = Path("{mode_display}").read_text().strip()
if mode == "fail":
    sys.stderr.write("owner unreachable\nsecond diagnostic line\n")
    sys.exit(1)

args = sys.argv[1:]
assert args[:5] == ["--workspace", "hm_host/ws_demo", "tool", "run", "orbit.attention"], args
assert args[5] == "--input", args
query = json.loads(args[6])

if query.get("severity") == "critical":
    items = [
        {{
            "id": "att_host_1",
            "fingerprint": "fp_crit_1",
            "kind": "host_unhealthy",
            "severity": "critical",
            "refs": {{}},
            "host": {{"name": "build-box"}},
            "reason": "build-box unreachable",
            "next": {{"command": "orbit host show build-box"}},
        }},
    ]
else:
    items = [
        {{
            "id": "att_warn_1",
            "fingerprint": "fp_med_1",
            "kind": "pilot_warning",
            "severity": "medium",
            "refs": {{"task_id": "TASK-12"}},
            "reason": "criterion lacks an observable check",
            "next": {{"command": "orbit --workspace ws_demo task repilot TASK-12"}},
        }},
        {{
            "id": "att_ci_1",
            "fingerprint": "fp_high_1",
            "kind": "ci_red",
            "severity": "high",
            "refs": {{"run_id": "98765"}},
            "reason": "ci failure with a very long reason " + ("x" * 120),
            "next": {{"command": "orbit tool run github.run.view --input '{{\"run\":\"98765\"}}'"}},
        }},
    ]

print(json.dumps({{"items": items}}))
"#
    );
    fs::write(&orbit, fixture).unwrap();
    fs::set_permissions(&orbit, fs::Permissions::from_mode(0o755)).unwrap();

    let path_env = match std::env::var_os("PATH") {
        Some(existing) => {
            let mut joined = std::ffi::OsString::from(bin.as_os_str());
            joined.push(":");
            joined.push(existing);
            joined
        }
        None => bin.as_os_str().to_os_string(),
    };
    let script = repo_root().join("plugin/skills/formation/scripts/attention-read.sh");

    let ok_out = Command::new(&script)
        .arg("hm_host/ws_demo")
        .env("PATH", &path_env)
        .output()
        .unwrap();
    assert!(
        ok_out.status.success(),
        "{}",
        String::from_utf8_lossy(&ok_out.stderr)
    );
    let stdout = String::from_utf8(ok_out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got: {stdout}");
    for line in &lines {
        let (fingerprint, report) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("expected tab in line: {line}"));
        assert!(!fingerprint.is_empty(), "empty fingerprint in line: {line}");
        let parts: Vec<&str> = report.split(" | ").collect();
        assert_eq!(parts.len(), 5, "expected 5 report parts in line: {line}");
        assert!(parts.iter().all(|part| !part.is_empty()));
        assert!(
            parts[3].chars().count() <= 120,
            "reason exceeded 120 chars: {}",
            parts[3]
        );
    }
    assert!(lines[0].starts_with("fp_crit_1\thost_unhealthy | critical | build-box | "));
    assert!(lines[1].starts_with("fp_high_1\tci_red | high | 98765 | "));
    assert!(lines[2].starts_with("fp_med_1\tpilot_warning | medium | TASK-12 | "));

    fs::write(&mode_file, "fail").unwrap();
    let fail_out = Command::new(&script)
        .arg("hm_host/ws_demo")
        .env("PATH", &path_env)
        .output()
        .unwrap();
    assert!(!fail_out.status.success());
    assert!(fail_out.stdout.is_empty());
    let fail_err = String::from_utf8(fail_out.stderr).unwrap();
    assert_eq!(
        fail_err.lines().count(),
        1,
        "expected one stderr line, got: {fail_err}"
    );
    assert!(fail_err.contains("owner unreachable"));
}

fn prepend_path(bin: &Path) -> std::ffi::OsString {
    match std::env::var_os("PATH") {
        Some(existing) => {
            let mut joined = std::ffi::OsString::from(bin.as_os_str());
            joined.push(":");
            joined.push(existing);
            joined
        }
        None => bin.as_os_str().to_os_string(),
    }
}

#[test]
fn attention_read_returns_more_than_20_items_preserving_filters_and_severity_order() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let queries_file = dir.path().join("queries.jsonl");

    let orbit = bin.join("orbit");
    let fixture = format!(
        r#"#!/usr/bin/env python3
import json
import sys
from pathlib import Path

args = sys.argv[1:]
assert args[:5] == ["--workspace", "hm_host/ws_demo", "tool", "run", "orbit.attention"], args
assert args[5] == "--input", args
query = json.loads(args[6])
with Path("{queries}").open("a") as fh:
    fh.write(json.dumps(query) + "\n")

if query.get("severity") == "critical":
    items = [
        {{
            "id": "att_crit_1",
            "fingerprint": "fp_crit_1",
            "kind": "host_unhealthy",
            "severity": "critical",
            "refs": {{}},
            "host": {{"name": "build-box"}},
            "reason": "build-box unreachable",
            "next": {{"command": "orbit host show build-box"}},
        }},
        {{
            "id": "att_crit_2",
            "fingerprint": "fp_crit_2",
            "kind": "review_waiting",
            "severity": "critical",
            "refs": {{"task_id": "TASK-99"}},
            "reason": "critical review waiting",
            "next": {{"command": "orbit task show TASK-99"}},
        }},
        {{
            "id": "att_dup",
            "fingerprint": "fp_item_01",
            "kind": "notice_open",
            "severity": "critical",
            "refs": {{"notice_id": "N-1"}},
            "reason": "duplicate fingerprint should be deduplicated",
            "next": {{"command": "orbit notice list"}},
        }},
    ]
else:
    severities = ["low", "medium", "high", "critical"]
    items = []
    for idx in range(1, 26):
        sev = severities[idx % len(severities)]
        items.append({{
            "id": f"att_{{idx:02d}}",
            "fingerprint": f"fp_item_{{idx:02d}}",
            "kind": "blocked",
            "severity": sev,
            "refs": {{"task_id": f"TASK-{{idx:02d}}"}},
            "reason": f"item {{idx:02d}} waiting",
            "next": {{"command": f"orbit task show TASK-{{idx:02d}}"}},
        }})

print(json.dumps({{
    "items": items,
    "counts": {{
        "matching": len(items),
        "returned": len(items),
        "truncated": False,
    }},
    "degraded": [],
}}))
"#,
        queries = queries_file.display(),
    );
    fs::write(&orbit, fixture).unwrap();
    fs::set_permissions(&orbit, fs::Permissions::from_mode(0o755)).unwrap();

    let script = repo_root().join("plugin/skills/formation/scripts/attention-read.sh");
    let out = Command::new(&script)
        .arg("hm_host/ws_demo")
        .env("PATH", prepend_path(&bin))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let queries: Vec<serde_json::Value> = fs::read_to_string(&queries_file)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0]["limit"], 500);
    assert_eq!(
        queries[0]["exclude_kind"],
        serde_json::json!([
            "friction_untriaged",
            "host_unhealthy",
            "notice_open",
            "review_waiting",
        ])
    );
    assert_eq!(queries[1]["limit"], 500);
    assert_eq!(queries[1]["severity"], "critical");
    assert_eq!(queries[1]["kind"], queries[0]["exclude_kind"]);

    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 27, "expected 25 + 2 unique items: {stdout}");

    let rank = |sev: &str| match sev {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        other => panic!("unexpected severity {other}"),
    };
    let severities: Vec<u8> = lines
        .iter()
        .map(|line| {
            let (_, body) = line.split_once('\t').unwrap();
            let parts: Vec<&str> = body.split(" | ").collect();
            assert_eq!(parts.len(), 5);
            rank(parts[1])
        })
        .collect();
    assert!(
        severities.windows(2).all(|pair| pair[0] <= pair[1]),
        "items not sorted by severity: {severities:?}"
    );
}

#[test]
fn attention_read_rejects_partial_or_truncated_snapshots_and_preserves_watch_state() {
    let temp = cw_testkit::TempRoot::new("FORMATION").unwrap();
    let bin = temp.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let mode_file = temp.path().join("mode");
    let orbit = bin.join("orbit");
    let fixture = format!(
        r#"#!/usr/bin/env python3
import json
import sys
from pathlib import Path

mode = Path("{mode}").read_text().strip()
args = sys.argv[1:]
query = json.loads(args[6])
is_crit = query.get("severity") == "critical"

def make_items(count):
    return [
        {{
            "id": f"att_{{idx:02d}}",
            "fingerprint": f"fp_item_{{idx:02d}}",
            "kind": "blocked",
            "severity": "high",
            "refs": {{"task_id": f"TASK-{{idx:02d}}"}},
            "reason": f"blocked item {{idx:02d}}",
            "next": {{"command": f"orbit task show TASK-{{idx:02d}}"}},
        }}
        for idx in range(1, count + 1)
    ]

if is_crit:
    if mode == "degraded_crit":
        print(json.dumps({{
            "items": [],
            "counts": {{"matching": 0, "returned": 0, "truncated": False}},
            "degraded": [{{
                "scope": "host:build-box",
                "kind": "host_unhealthy",
                "code": "unreachable",
                "message": "host unreachable",
            }}],
        }}))
    elif mode == "truncated_crit":
        print(json.dumps({{
            "items": [],
            "counts": {{"matching": 2, "returned": 0, "truncated": True}},
            "degraded": [],
        }}))
    else:
        print(json.dumps({{
            "items": [],
            "counts": {{"matching": 0, "returned": 0, "truncated": False}},
            "degraded": [],
        }}))
    sys.exit(0)

if mode == "complete_25":
    items = make_items(25)
    print(json.dumps({{
        "items": items,
        "counts": {{"matching": 25, "returned": 25, "truncated": False}},
        "degraded": [],
    }}))
elif mode == "complete_24":
    items = make_items(24)
    print(json.dumps({{
        "items": items,
        "counts": {{"matching": 24, "returned": 24, "truncated": False}},
        "degraded": [],
    }}))
elif mode == "truncated_flag":
    items = make_items(10)
    print(json.dumps({{
        "items": items,
        "counts": {{"matching": 25, "returned": 10, "truncated": True}},
        "degraded": [],
    }}))
elif mode == "matching_gt_returned":
    items = make_items(10)
    print(json.dumps({{
        "items": items,
        "counts": {{"matching": 25, "returned": 10, "truncated": False}},
        "degraded": [],
    }}))
elif mode == "returned_mismatch":
    items = make_items(2)
    print(json.dumps({{
        "items": items,
        "counts": {{"matching": 2, "returned": 3, "truncated": False}},
        "degraded": [],
    }}))
elif mode == "degraded_fast":
    print(json.dumps({{
        "items": [],
        "counts": {{"matching": 0, "returned": 0, "truncated": False}},
        "degraded": [{{
            "scope": "workspace:ws_demo",
            "kind": "ci_red",
            "code": "capped",
            "message": "per-workspace item cap reached",
        }}],
    }}))
else:
    sys.stderr.write(f"unknown mode {{mode}}\n")
    sys.exit(2)
"#,
        mode = mode_file.display(),
    );
    fs::write(&orbit, fixture).unwrap();
    fs::set_permissions(&orbit, fs::Permissions::from_mode(0o755)).unwrap();

    let path_env = prepend_path(&bin);
    let script = repo_root().join("plugin/skills/formation/scripts/attention-read.sh");

    for (mode, expected) in [
        ("truncated_flag", "truncated"),
        ("matching_gt_returned", "truncated"),
        ("returned_mismatch", "incomplete"),
        ("degraded_fast", "degraded"),
        ("degraded_crit", "degraded"),
        ("truncated_crit", "truncated"),
    ] {
        fs::write(&mode_file, mode).unwrap();
        let out = Command::new(&script)
            .arg("hm_host/ws_demo")
            .env("PATH", &path_env)
            .output()
            .unwrap();
        assert!(!out.status.success(), "{mode} unexpectedly succeeded");
        assert!(out.stdout.is_empty(), "{mode} wrote stdout");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(stderr.lines().count(), 1, "{mode}: {stderr}");
        assert!(stderr.contains(expected), "{mode}: {stderr}");
    }

    // End-to-end: truncated/degraded responses must not mark unlisted items cleared.
    let join = temp
        .command(env!("CARGO_BIN_EXE_formation"))
        .args(["mail", "join", "sentry", "--room"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(join.status.success());

    let probe = format!("attention={} hm_host/ws_demo", script.display());
    let wait_once = || -> serde_json::Value {
        let out = temp
            .command(env!("CARGO_BIN_EXE_formation"))
            .env("PATH", &path_env)
            .args(["watch", "--room"])
            .arg(temp.path())
            .args([
                "--as",
                "sentry",
                "wait",
                "--probe",
                &probe,
                "--timeout",
                "0",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    };

    fs::write(&mode_file, "complete_25").unwrap();
    let first = wait_once();
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_eq!(first["remaining"], 5);
    let second = wait_once();
    assert_eq!(second["items"].as_array().unwrap().len(), 5);
    assert_eq!(second["remaining"], 0);

    // Two truncated reads and one degraded read omit items 11..=25 but must fail closed.
    fs::write(&mode_file, "truncated_flag").unwrap();
    let fail_one = wait_once();
    assert_eq!(fail_one["failed"].as_array().unwrap().len(), 1);
    assert_eq!(fail_one["items"], serde_json::json!([]));
    let fail_two = wait_once();
    assert_eq!(fail_two["expired"], true);
    assert_eq!(fail_two["failed"], serde_json::json!([]));

    fs::write(&mode_file, "degraded_fast").unwrap();
    let fail_three = wait_once();
    assert_eq!(fail_three["expired"], true);
    assert_eq!(fail_three["failed"], serde_json::json!([]));

    // Returning to the complete 25-item snapshot must not re-report any unlisted item.
    fs::write(&mode_file, "complete_25").unwrap();
    let recovered = wait_once();
    assert_eq!(recovered["expired"], true);
    assert_eq!(recovered["items"], serde_json::json!([]));

    // Only two consecutive complete reads omitting item 25 forget item 25.
    fs::write(&mode_file, "complete_24").unwrap();
    assert_eq!(wait_once()["expired"], true);
    assert_eq!(wait_once()["expired"], true);

    fs::write(&mode_file, "complete_25").unwrap();
    let returned = wait_once();
    assert_eq!(returned["expired"], false);
    let items = returned["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["fingerprint"], "fp_item_25");
}
