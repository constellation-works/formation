# Checking a shift roster

Run `formation roster check < roster.json` for a count of seats and directed routes, or add `--json` for machine-readable output. It accepts one UTF-8 JSON document up to 256 KiB. Success exits 0. Invalid documents exit 1 with a diagnostic on stderr. Invalid CLI arguments exit 2.

[The example](../examples/roster.json) contains a Claude chief and aide, a Codex captain, and a headless sentry. Version 1 requires `version`, `seats` and `routes`. Each seat has a unique case-sensitive `name`, `role`, `host`, and optional `workspace`. Captains require a workspace and each workspace can have one captain. Names and workspace IDs use ASCII letters, digits, hyphens and underscores. Hosts are `claude-desktop`, `codex-desktop` and `headless`. Roles are chief, aide, captain, lieutenant, sergeant, sentry, specialist and worker.

Each route has a `from` and a `to` seat. Add the reverse pair explicitly when replies are wanted. Both ends must exist and differ. The checker rejects duplicate routes and unknown fields. A roster can describe a single unit or a shift. It contains no session credentials or live bindings.

## Plugin package

Run `make update-plugins` from the checkout with Cargo, Codex and Claude Code on PATH. It installs the Formation CLI into `~/.cargo/bin` and installs the [Formation skill](../plugin/skills/formation/SKILL.md) and provider hooks ([Claude hooks](../plugin/hooks/claude.json), [Codex hooks](../plugin/hooks/codex.json)) in both clients from this checkout. The skill handles shift staffing, role briefs, escalation and close-out. It selects the receiving provider’s session tools for each seat. Keep the checkout in place and the binary directory on PATH. Codex asks for one review and trust approval whenever a hook file changes. Rerun `make update-plugins` after pulling changes, then open a new chat in each client: a hook change takes effect only in a new chat (an already open chat keeps using the hooks loaded when it started and can keep calling a removed hook).

For CLI-only use, run `make install`. Checking a roster works offline without a provider account, Orbit or a broker. To try the plugin in a disposable Claude Code session, launch `claude --plugin-dir ./plugin` from the checkout.

## Local inbox

Build the CLI with `make build` and use `target/debug/formation`, or use the installed binary. A room is selected by `--room <name|path>` (or `FORMATION_ROOM`), accepted before or after the subcommand. A value with a path separator or leading `.` or `~` is a directory path; any other value (1 to 96 ASCII letters, digits, hyphens or underscores) resolves to `<home>/rooms/<name>/`, where `<home>` is `FORMATION_HOME` or `~/.formation`. `mail join` creates a missing room (directory mode 0700, database mode 0600); every other command refuses a missing room with `room_not_found` and creates nothing. `mail join` also creates the seat's folder, `<room directory>/seats/<seat>/` (mode 0700), and prints its absolute path as `seat_dir`. A seat keeps its probe scripts and its held, mute and log files there, so clearing scratch or a checkout does not remove them. Formation never reads or deletes the folder's contents.

A join is all-or-nothing. It checks the folder first, then binds the seat and records the session pointer in one step. Except for `session_route_unrestored`, below, a join that exits nonzero leaves no new seat, no new or changed session pointer and no change to a seat's owner, route or pending mail, so retry it once the cause is gone. Joins from one session run one at a time, whatever their rooms, under a lock on `<home>/sessions/<session id>.lock`, which stays on disk. A join waits up to 15 seconds for it and then refuses, naming the holder. A session pointer that is not a private regular file of at most 4096 bytes refuses the join before the seat is bound. If a failed join cannot put the previous pointer back, it reports `session_route_unrestored` with the pointer's path and both failures; fix that path and join again. A join that is killed before it finishes can leave the pointer naming its room, and joining again repairs it. A folder that is a symlink, belongs to another user or is open to group or other is refused with `seat_dir_unavailable`. For a folder you own with a mode such as 0755, the error gives the exact path, the observed mode, the required mode 0700 and the `chmod` to run before you retry. The refusal never changes the folder or its contents.

In a Claude Code or Codex session, `formation mail join` looks up the session in open Scribe groups using `scribe group list --json`. It joins the matching seat in the room named for the group; `--room` overrides that room. Join the Scribe group first. If Scribe is unavailable or the seat is ambiguous, use `formation mail join <seat> --room <room>` instead. This lookup runs only at join, and the explicit form works without Scribe.

In a Claude Code or Codex session (`CLAUDE_CODE_SESSION_ID`, else `CODEX_THREAD_ID`), `mail join <seat>` binds that session id to the seat and records the room in `<home>/sessions/<session id>`. Later commands in that session find both the room and the seat with no flags. A seat listed by `mail who` run with `--room` and `--as` may not have this route, and `FORMATION_ROOM` stands in for a missing pointer in a command while the hook ignores it. After a join succeeds, check the route from that session with `env -u FORMATION_ROOM formation mail who`. Joining a seat already held by another session takes it over and reports `rebound: true` with `replaced_session`; the replaced session's next call without `--as` is refused with `seat_required`. Outside a session (or to select another seat explicitly), pass `--as <seat>`:

```sh
formation mail join chief --room trial
formation mail join captain --room trial
formation mail who --room trial --as chief
formation mail send captain --id check-1 --body 'CI finished' --room trial --as chief
formation mail inbox --wait 30 --room trial --as captain
formation mail ack 1 --room trial --as captain
```

Join, who, send, inbox and ack return JSON on stdout. Mail refusals print one JSON line with `code` and `error` on stderr and exit 1. `mail send` stores an envelope for any joined seat, present or not, and stamps it with the time it was stored. Its reply carries `held`, which is true when the recipient renewed its presence in the last 90 seconds. A waiting inbox or Stop hook renews it every 15 seconds. A name that never joined is refused with `unknown_recipient`, and the message lists up to 32 joined seats. Each inbox envelope carries `sent` and `delivered` as RFC 3339 UTC times. `sent` is null for mail stored by an earlier build, and `delivered` is null until a hook hands it over. `mail who` lists every joined seat as `name`, `undelivered`, `unacknowledged`, `held` and `seen` (an RFC 3339 UTC time, or null for a seat that has not run a command since it joined). `undelivered` counts the seat's unacknowledged envelopes that no hook has delivered. `unacknowledged` counts every envelope the seat has not acknowledged, whether or not a hook delivered it. Read `undelivered` to see whether a hooked seat is being reached: a hook delivers mail without an acknowledgment, and a hooked seat is not expected to acknowledge, so its `unacknowledged` count grows with the mail it has received. `mail inbox` lists the unacknowledged envelopes. Joining does not renew presence. A waiting inbox checks SQLite every half second inside one bounded command. An empty array means the wait expired. Each read returns up to 32 unacknowledged messages, oldest first. A recipient acknowledgment is repeatable and retains the stored envelope. A retry with the same sender, ID, recipient and body returns the original receipt, even after acknowledgment or expiry of the recipient's presence. Its `held` reflects the recipient's presence at the retry. A changed payload using the same ID fails.

`formation mail sent` lists the caller's envelopes newest first, up to 32 by default. Use `--limit <n>` to change that limit and `--json` for a document with `messages`, `total` and `truncated`. Each record shows its receipt, id, recipient, sent time and state. `stored` means the envelope is queued, `delivered` means a hook handed it over, and `acked` means the recipient acknowledged it.

The room is for cooperating processes on one machine under one OS account. Any joined seat can send to any joined seat in that room. The seat name is an accident guard and not a security boundary: any process under the same OS account can pass `--as`, read the database or act as any seat, so no authority or access control depends on a seat name. The CLI requires private directories (mode 0700) and private files (mode 0600). Keep the room on a local filesystem. The roster validator is independent of room membership.

Existing rooms from earlier builds open in place and keep their seat names, messages and receipt numbers on the first database write. Early builds could create `inbox.sqlite3` with mode 0644 inside a private room. The shared SQLite driver requires an owner-only file. If the diagnostic identifies that permission issue, run `chmod 600 "<room>/inbox.sqlite3"` for your own room and retry. Newly created databases already use mode 0600.

## Hooks

The host pipes its event JSON to the command on stdin (up to 1 MiB). Use the pair for your host.

| Host | UserPromptSubmit command | Stop command |
| --- | --- | --- |
| Claude Code | `formation hook --provider claude --event user-prompt` | `formation hook --provider claude --event stop --hold 30` |
| Codex | `formation hook --provider codex --event user-prompt` | `formation hook --provider codex --event stop` |

The hook looks up the mailbox from the event's `session_id`. Unjoined sessions and events carrying `agent_id` exit silently. UserPromptSubmit marks the binding busy and adds unread mail to the current turn. Claude Stop uses `asyncRewake` with `--hold 3300` and a host timeout of 3600 seconds. A positive-duration Claude Stop acquires one per-seat waiter lease in the room (45-second TTL, renewed every 15 seconds); a concurrent or later Stop seeing a live lease exits silently without replacing the active waiter or consuming waiting mail. When mail arrives the live waiter emits the envelope on stderr, releases its lease and exits 2. Codex Stop marks the binding idle, then checks for mail without waiting. If mail is waiting, it continues the turn with a block decision and marks the binding busy. The shipped plugin gives Codex Stop and both prompt hooks 60 seconds to run and exits 0 silently when `formation` is absent from PATH. The CLI default hold is zero seconds. An expired hold releases its lease and exits 0 with empty output. Errors exit 1 with one diagnostic line on stderr.

After storing new mail, `mail send` and the watch reader can queue a wake for an idle Codex seat with a current hook binding. The sender needs `codex` on PATH and access to the recipient's local Codex queue under the same OS account. The command uses plain `codex queue` and submits one fixed line stating that Formation queued it and the operator did not type it. The envelope stays in the room until the receiving hook reads it. If queue submission fails or the seat has no observed provider state, mail remains available at its next normal hook. Formation limits queue attempts to one per observed idle period. If Codex Desktop is closed or the target chat is unloaded, a successful queue submission may wait until the app reopens and loads that chat.

Hooks deliver whole envelopes in receipt order within the host's character budget (9,000 for Claude, 8,000 for Codex). If mail remains, the last line gives the waiting count. Use `mail inbox` to read an envelope too large for the hook. Hook delivery leaves the envelope in the inbox until `mail ack`. The delivered time records the command's handoff and remains set if the host fails to consume the output.

## Watching

Add probes to the room's watch set and run a shared reader.

```sh
formation watch add ci --every 300 -- plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main
formation watch add release --to captain -- ./scripts/release-read.sh
formation watch run
```

A watch is keyed by its recipient and name, so two seats can each have a `ci`. `--to` names a joined seat and defaults to the caller. Names use 1 to 32 lowercase ASCII letters, digits or hyphens. The interval defaults to 120 seconds and must be at least 10. One word after `--` is shell text as written. Several words are quoted so each stays one argument. `add` replaces an existing watch, switches it on and keeps its remembered fingerprints. `remove` forgets them.

```sh
formation watch pause ci
formation watch resume ci
formation watch remove release --to captain
formation watch list
formation watch list --json
formation watch stop
```

`watch run` keeps one active lease per room. Extra invocations stand by. The reader renews its lease every 15 seconds and a standby can take over when a heartbeat is 45 seconds old. Changes to the set take effect on the next pass. A watch runs through `sh -c` in the directory it was added from, using the active reader's environment. Due probes run concurrently and each has a 60-second limit. `watch stop` marks the room stopped and ends the reader and its standbys. A later `watch run` clears the mark.

`list` prints the reader state line followed by a table of watches:

```text
reader: orbit-0-capt, pid 4711, last beat 2026-11-26T04:00:00Z; standby: scribe-0-capt, pid 4820, last beat 2026-11-26T04:00:02Z
NAME       TO             EVERY (s)  STATE   LAST READ             RESULT       SENT  ADDED BY
attention  orbit-0-capt         120  on      2026-11-26T03:59:30Z  ok, 3 items     5  orbit-0-capt
ci         orbit-0-capt         300  on      2026-11-26T03:58:10Z  read failed     0  orbit-0-capt
release    scribe-0-capt         60  paused  2026-11-26T02:00:00Z  ok, 0 items     1  0-chief
```

`list --json` exposes the same fields with `reader.state` set to `live`, `none` (printed as `reader: none` or `reader: none since <time>`) or `stopped` (`reader: stopped since <time>`). Timestamps are RFC 3339 UTC. `list` works against older rooms without upgrading them.

If the room has at least one `on` watch, `reader.state` is `none` and `watch stop` was not run, the next hook event for each seat that receives or added an `on` watch delivers a one-time notice: `watch: nothing has read this room's <n> watches since <time>. Start a reader: formation watch run`. Restart a reader from a live seat's shell to clear the gap.

A probe prints one `<fingerprint><TAB><text>` per item. Fingerprints use 1 to 200 characters with no tab. Text past 400 characters is cut to end in `…`. The reader mails new items as `<probe> | <text>` (up to 20 lines per envelope) under the heading `Watch <name>, added by <seat>: probe output, not a seat's request.`, marking it as probe data rather than a seat's request. Fingerprints omitted from two consecutive complete successful reads are forgotten so a returning condition is mailed again.

If a probe exits non-zero, exceeds the 60-second limit, emits a malformed line, or lacks a tool or token in the active reader's environment (even if `watch check` passed in the caller's shell), `list` records `read failed` in `RESULT`. The reader mails `<probe> | read failed: <first diagnostic line>` once until a successful read recovers; failed reads do not forget remembered fingerprints. The room stores handover state per recipient and watch, so a standby reuses it when it takes over.

### Checking a probe

`watch check -- <command>` runs a probe once from your current directory under the reader's 60-second limit and prints its items as `{"items":[{"fingerprint":...,"text":...}]}`. It changes nothing: no watch is added, the reader's lease is not taken, no fingerprint is remembered and no mail is sent, so the first real read still sends every item. It needs a joined seat, as `list` does. A malformed line, a failed run and a run past the limit exit 1 with a JSON error on stderr. The error gives the reason: the line number for a malformed line, and the first diagnostic line for a failed run. An empty result exits 0 with `{"items":[]}`.

```sh
formation watch check -- plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main
```

### Blocking wait

`watch wait` runs caller-supplied probes from your current directory and returns new items or failures as JSON. It uses the calling seat's handover state, returns at most 20 items per call and reports any remainder. It polls every 120 seconds by default. After `--timeout` seconds (default 300) with nothing to report, it returns empty lists with `"expired":true`. Use `--timeout 0` for a single pass.

```sh
formation watch wait \
  --probe "ci=plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main" \
  --timeout 300 --room trial --as sentry
```
