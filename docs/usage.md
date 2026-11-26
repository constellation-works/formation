# Checking a shift roster

Run `formation roster check < roster.json` for a count of seats and directed routes, or add `--json` for machine-readable output. It accepts one UTF-8 JSON document up to 256 KiB. Success exits 0. Invalid documents exit 1 with a diagnostic on stderr. Invalid CLI arguments exit 2.

[The example](../examples/roster.json) contains a Claude chief and aide, a Codex captain, and a headless sentry. Version 1 requires `version`, `seats` and `routes`. Each seat has a unique case-sensitive `name`, `role`, `host`, and optional `workspace`. Captains require a workspace and each workspace can have one captain. Names and workspace IDs use ASCII letters, digits, hyphens and underscores. Hosts are `claude-desktop`, `codex-desktop` and `headless`. Roles are chief, aide, captain, lieutenant, sergeant, sentry, specialist and worker.

Each route has a `from` and a `to` seat. Add the reverse pair explicitly when replies are wanted. Both ends must exist and differ. The checker rejects duplicate routes and unknown fields. A roster can describe a single unit or a shift. It contains no session credentials or live bindings.

## Plugin package

Run `make update-plugins` from the checkout with Cargo, Codex and Claude Code on PATH. It installs the Formation CLI into `~/.cargo/bin` and installs the [Formation skill](../plugin/skills/formation/SKILL.md) and provider hooks ([Claude hooks](../plugin/hooks/claude.json), [Codex hooks](../plugin/hooks/codex.json)) in both clients from this checkout. The skill handles shift staffing, role briefs, escalation and close-out. It selects the receiving provider’s session tools for each seat. Keep the checkout in place and the binary directory on PATH. Codex asks for one review and trust approval whenever a hook file changes. Rerun `make update-plugins` after pulling changes, then open a new chat in each client: a hook change takes effect only in a new chat (an already open chat keeps using the hooks loaded when it started and can keep calling a removed hook).

For CLI-only use, run `make install`. Checking a roster works offline without a provider account, Orbit or a broker. To try the plugin in a disposable Claude Code session, launch `claude --plugin-dir ./plugin` from the checkout.

## Local inbox

Build the CLI with `make build` and use `target/debug/formation`, or use the installed binary. A room is selected by `--room <name|path>` (or `FORMATION_ROOM`), accepted before or after the subcommand. A value with a path separator or leading `.` or `~` is a directory path; any other value (1 to 96 ASCII letters, digits, hyphens or underscores) resolves to `<home>/rooms/<name>/`, where `<home>` is `FORMATION_HOME` or `~/.formation`. `mail join` creates a missing room (directory mode 0700, database mode 0600); every other command refuses a missing room with `room_not_found` and creates nothing.

In a Claude Code or Codex session, `formation mail join` looks up the session in open Scribe groups using `scribe group list --json`. It joins the matching seat in the room named for the group; `--room` overrides that room. Join the Scribe group first. If Scribe is unavailable or the seat is ambiguous, use `formation mail join <seat> --room <room>` instead. This lookup runs only at join, and the explicit form works without Scribe.

In a Claude Code or Codex session (`CLAUDE_CODE_SESSION_ID`, else `CODEX_THREAD_ID`), `mail join <seat>` binds that session id to the seat and records the room in `<home>/sessions/<session id>`. Later commands in that session find both the room and the seat with no flags. Joining a seat already held by another session takes it over and reports `rebound: true` with `replaced_session`; the replaced session's next call without `--as` is refused with `seat_required`. Outside a session (or to select another seat explicitly), pass `--as <seat>`:

```sh
formation mail join chief --room trial
formation mail join captain --room trial
formation mail who --room trial --as chief
formation mail send captain --id check-1 --body 'CI finished' --room trial --as chief
formation mail inbox --wait 30 --room trial --as captain
formation mail ack 1 --room trial --as captain
```

Join, who, send, inbox and ack return JSON on stdout. Mail refusals print one JSON line with `code` and `error` on stderr and exit 1. `mail send` stores an envelope for any joined seat, present or not, and stamps it with the time it was stored. Its reply carries `held`, which is true when the recipient renewed its presence in the last 90 seconds. A waiting inbox or Stop hook renews it every 15 seconds. A name that never joined is refused with `unknown_recipient`, and the message lists up to 32 joined seats. Each inbox envelope carries `sent` and `delivered` as RFC 3339 UTC times. `sent` is null for mail stored by an earlier build, and `delivered` is null until a hook hands it over. `mail who` lists every joined seat as `name`, `unread` (a count), `held` and `seen` (an RFC 3339 UTC time, or null for a seat that has not run a command since it joined). Joining does not renew presence. A waiting inbox checks SQLite every half second inside one bounded command. An empty array means the wait expired. Each read returns up to 32 unacknowledged messages, oldest first. A recipient acknowledgment is repeatable and retains the stored envelope. A retry with the same sender, ID, recipient and body returns the original receipt, even after acknowledgment or expiry of the recipient's presence. Its `held` reflects the recipient's presence at the retry. A changed payload using the same ID fails.

`formation mail sent` lists the caller's envelopes newest first, up to 32 by default. Use `--limit <n>` to change that limit and `--json` for a document with `messages`, `total` and `truncated`. Each record shows its receipt, id, recipient, sent time and state. `stored` means the envelope is queued, `delivered` means a hook handed it over, and `acked` means the recipient acknowledged it.

The room is for cooperating processes on one machine under one OS account. Any joined seat can send to any joined seat in that room. The seat name is an accident guard and not a security boundary: any process under the same OS account can pass `--as`, read the database or act as any seat, so no authority or access control depends on a seat name. The CLI requires private directories (mode 0700) and private files (mode 0600). Keep the room on a local filesystem. The roster validator is independent of room membership.

Existing rooms from earlier builds open in place and keep their seat names, messages and receipt numbers on the first database write. Early builds could create `inbox.sqlite3` with mode 0644 inside a private room. The shared SQLite driver requires an owner-only file. If the diagnostic identifies that permission issue, run `chmod 600 "<room>/inbox.sqlite3"` for your own room and retry. Newly created databases already use mode 0600.

## Hooks

The host pipes its event JSON to the command on stdin (up to 1 MiB). Use the pair for your host.

| Host | UserPromptSubmit command | Stop command |
| --- | --- | --- |
| Claude Code | `formation hook --provider claude --event user-prompt` | `formation hook --provider claude --event stop --hold 30` |
| Codex | `formation hook --provider codex --event user-prompt` | `formation hook --provider codex --event stop --hold 30` |

The hook looks up the mailbox from the event's `session_id`. Unjoined sessions and events carrying `agent_id` exit silently. UserPromptSubmit adds context to the current turn. With mail waiting, Claude Stop emits mail on stderr and exits 2 for an `asyncRewake` hook. Codex Stop emits a block decision whose reason contains the mail. Set the host timeout longer than the Stop hold. The shipped plugin configures `--hold 540` with `timeout` 600 on Claude Stop (`asyncRewake: true`), `--hold 1800` with `timeout` 1860 on Codex Stop, `timeout` 60 on `UserPromptSubmit`, and exits 0 silently when `formation` is absent from PATH. The CLI default hold is zero seconds. An expired hold exits 0 with empty output. Errors exit 1 with one diagnostic line on stderr.

Hooks deliver whole envelopes in receipt order within the host's character budget (9,000 for Claude, 8,000 for Codex). If mail remains, the last line gives the waiting count. Use `mail inbox` to read an envelope too large for the hook. Hook delivery leaves the envelope in the inbox until `mail ack`. The delivered time records the command's handoff and remains set if the host fails to consume the output.

## Watching

`formation watch` polls one or more caller-supplied probes on an interval and passes on only what is new:

```sh
formation watch \
  --probe "ci=plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main" \
  --to captain --every 120 --room trial --as sentry
```

Each `--probe <name>=<command>` names a probe (1 to 32 lowercase ASCII letters, digits or hyphens) and a command run through `sh -c` in the caller's working directory with the caller's environment. Every probe runs immediately and then every `--every` seconds (default 120, minimum 10). A probe run that passes 60 seconds is killed with its process group and reported as a failed read, and any surviving child is terminated when `watch` exits.

Probe stdout is treated as data under a one-line-per-item contract: `<fingerprint><TAB><text>`, where `<fingerprint>` is 1 to 200 characters with no tab and `<text>` past 400 characters is cut to end in `…`. Handed fingerprints are stored in the room per `(seat, probe)` (using `--to` when given, else the caller's seat), so restarted or concurrent watchers never hand over the same fingerprint twice. A non-zero exit, killed run or malformed line is a failed read: it is reported once with its first line and stays silent until that probe completes a successful read.

- **With `--to <seat>`:** `--to` must be a joined seat. New items are mailed from the caller's seat as `<probe> | <text>` (and failed reads as `<probe> | read failed: <error>`), up to 20 lines per envelope, with handed fingerprints committed in the same transaction that stores each envelope. The command runs until `--timeout` (no default) or a termination signal (`SIGTERM` or `SIGINT`) and exits 0.
- **Without `--to`:** The command returns at the first poll with new items or failures as `{"items":[{"probe":...,"fingerprint":...,"text":...}],"failed":[{"probe":...,"error":...}],"remaining":<n>,"expired":false}`, handing over at most 20 items per call and leaving any remainder for the next call. At `--timeout` (default 300 seconds) with nothing to report, it returns empty lists with `"expired":true` and exits 0:

```sh
formation watch \
  --probe "ci=plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main" \
  --timeout 300 --room trial --as sentry
```
