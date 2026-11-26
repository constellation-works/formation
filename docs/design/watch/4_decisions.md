---
title: Formation — Watch decisions
owner: claude
created_date: 2026-11-26
updated_date: 2026-11-26
status: Draft
feature: watch
doc_role: decisions
summary: Watch listing, command retirement and the local reader trust boundary.
---

# Formation — Watch decisions

## The list prints one human form

**Recorded:** 2026-11

### Context

`STD-01@4 §R9` asks a piped command for tab-separated records with no header, and §R12 keeps everything but the payload off stdout. The [watch scope](1_scope.md#seeing-what-is-watched) approves a `watch list` that opens with the reader's state and then a headed table, because the reader line is what tells a seat that nothing is reading its watches. A pipeline that dropped it would hide the failure `list` exists to show.

### Decision

`formation watch list` prints the reader line and the headed table whether or not stdout is a terminal. Cells never truncate and carry no escape codes. `--json` carries the same fields, with the reader state as an object. An empty set prints only the reader line, and the empty-state notice goes to stderr.

### Consequences

- A script that reads the rows uses `--json`, which is the stable contract (`STD-01@4 §R10`).
- Cost: `watch list | cut -f1` does not work; the text form is for people.


## Retire the original watch flags before the first release

**Recorded:** 2026-11

**Code anchors:** `crates/formation/src/command/watch/mod.rs::Action`

The [approved blocking form](1_scope.md#the-blocking-form) moves probe arguments to `watch wait` and moves the mailed form to `watch add` and `watch run`. Formation has yet to tag its first release. Keeping the old forms with deprecation warnings until a release would keep two ways to start polling and retain an unleased mailing path. Under this exception to `STD-01@4 §R35`, the parser rejects both old forms now.

Cost: callers must migrate their commands. Existing room handover state carries over.

## Run probes within the trusted local room

**Recorded:** 2026-11

**Code anchors:** `crates/formation-core/src/app/watch/reader.rs::Reader`, `crates/formation-core/src/app/watch/poll.rs::send_changes`, `crates/formation/src/command/watch/probe.rs::run_probe`

The [approved reader](1_scope.md#the-reader) uses the reader's environment and the directory saved by the seat. Seats that can write this private room already run commands as the same OS user. The lease regulates concurrency within that group (`STD-05@1 §R5`). Probe items and first-line failures use the existing room mail and failure records. The last-read result stores a count or failure state.

The alternatives were per-command environment and directory allowlists and a redactor before writing probe output to room mail. The approved probe contract retains the reader's environment, saved directory and bounded probe text (`STD-05@1 §R6`, §R10, §R12, §R13 and §R14). This contract applies to commands supplied by cooperating seats in the private local room.

Cost: a stored probe can access the reader's working environment and include sensitive text in room mail. Only add scripts you trust to run as the reader and to emit mail-safe text.

## Pin the attention probe to its owner and its agent-readable kinds

**Recorded:** 2026-11-26

**Code anchors:** `plugin/skills/formation/scripts/attention-read.sh::owner_host`, `plugin/skills/formation/scripts/attention-read.sh::main`, `crates/formation/tests/script.rs::attention_read_pins_the_owner_so_an_unrelated_follower_is_never_read`

The attention probe ran with a workspace-only selector, so Orbit fanned each read out to every in-scope host. An unrelated offline host then degraded the read, and the probe failed closed on every snapshot. Its queries also requested the Operator-only kinds `run_failed`, `run_stuck`, `claim_stale` and `host_unhealthy`. The probe runs as a shell, which holds no Operator capability (see the [overview](../formation/1_overview.md#what-the-app-gives-and-limits)), so Orbit answers those kinds with a `capability` degradation, and the fail-closed read kept that as a permanent failure. The shell's credentials are the boundary, so the fix does not widen them.

The probe accepts only a host-qualified selector and pins both reads to its owner with the `host` filter. The fast read excludes the digest kinds and the four Operator-only kinds. The critical read requests the digest kinds with `severity: critical` only. Any other degraded entry, truncation or malformed counts still fails the read closed. The Operator-only kinds move to an authorized path: a live resident whose mandate covers the workspace, or an Operator session's own Orbit read. [The sentry reference](../../../plugin/skills/formation/references/sentry.md#coverage) states the boundary.

The alternative was to give the probe the Operator capability. That widens an unprivileged process's credentials, which the change does not do. A workspace-only selector is not kept for compatibility: it names no owner to pin, so the probe refuses it.

### Consequences

- A watch must be added with the host-qualified selector from `orbit.workspace.list`. A workspace-only selector exits 2 before any Orbit call.
- The background reader does not cover run failures, stuck runs, stale claims or host health. A resident or the operator reads them on the authorized path, and a captain reports a `capability` degradation to the operator.
- The owner-pinned reads do not see an unrelated offline host. A selected owner that is unreachable still fails the read, because the owner's degradation is in scope.
