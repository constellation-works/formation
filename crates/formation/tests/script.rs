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
