# Checking a shift roster

Run `formation roster check < roster.json` for a count of seats and directed routes, or add `--json` for machine-readable output. It accepts one UTF-8 JSON document up to 256 KiB. Success exits 0. Invalid documents exit 1 with a diagnostic on stderr. Invalid CLI arguments exit 2.

[The example](../examples/roster.json) contains a Claude chief and aide, a Codex captain, and a headless sentry. Version 1 requires `version`, `seats` and `routes`. Each seat has a unique case-sensitive `name`, `role`, `host`, and optional `workspace`. Captains require a workspace and each workspace can have one captain. Names and workspace IDs use ASCII letters, digits, hyphens and underscores. Hosts are `claude-desktop`, `codex-desktop` and `headless`. Roles are chief, aide, captain, lieutenant, sergeant, sentry, specialist and worker.

Each route has a `from` and a `to` seat. Add the reverse pair explicitly when replies are wanted. Both ends must exist and differ. The checker rejects duplicate routes and unknown fields. A roster can describe a single unit or a shift. It contains no session credentials or live bindings.

## Plugin package

Run `make update-plugins` from the checkout with Cargo, Codex and Claude Code on PATH. It installs the Formation CLI into `~/.cargo/bin` and installs the [Formation skill](../plugin/skills/formation/SKILL.md) in both clients from this checkout. The skill handles shift staffing, role briefs, escalation and close-out. It selects the receiving provider’s session tools for each seat. Keep the checkout in place and the binary directory on PATH. Rerun the target after pulling changes, then open a new session in each client.

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

All mail commands return JSON on stdout, and command refusals print one JSON line with `code` and `error` on stderr and exit 1. `mail send` stores an envelope for any joined seat, present or not, and stamps it with the time it was stored. Its reply carries `held`: true when the recipient renewed its presence in the last 90 seconds, which a waiting inbox does every 15 seconds, and false when nobody is waiting, so the mail waits for the recipient's next read. A name that never joined is refused with `unknown_recipient`, and the message lists up to 32 joined seats. Each envelope in an inbox read carries `sent`, an RFC 3339 UTC time, or null for an envelope stored by an earlier build. `mail who` lists every joined seat as `name`, `unread` (a count), `held` and `seen` (an RFC 3339 UTC time, or null for a seat that has not run a command since it joined). Joining does not renew presence. A waiting inbox checks SQLite every half second inside one bounded command. An empty array means the wait expired. Each read returns up to 32 unread messages, oldest first. A recipient acknowledgment is repeatable and retains the stored envelope. A retry with the same sender, ID, recipient and body returns the original receipt, even after acknowledgment or expiry of the recipient's presence; its `held` reflects the recipient's presence at the retry. A changed payload using the same ID fails.

The room is for cooperating processes on one machine under one OS account. Any joined seat can send to any joined seat in that room. The seat name is an accident guard and not a security boundary: any process under the same OS account can pass `--as`, read the database or act as any seat, so no authority or access control depends on a seat name. The CLI requires private directories (mode 0700) and private files (mode 0600). Keep the room on a local filesystem. The roster validator is independent of room membership.

Existing rooms from earlier builds open in place and keep their seat names, messages and receipt numbers on the first database write. Early builds could create `inbox.sqlite3` with mode 0644 inside a private room. The shared SQLite driver requires an owner-only file. If the diagnostic identifies that permission issue, run `chmod 600 "<room>/inbox.sqlite3"` for your own room and retry. Newly created databases already use mode 0600.
