# Checking a shift roster

Run `formation roster check < roster.json` for a count of seats and directed routes, or add `--json` for machine-readable output. It accepts one UTF-8 JSON document up to 256 KiB. Success exits 0. Invalid documents exit 1 with a diagnostic on stderr. Invalid CLI arguments exit 2.

[The example](../examples/roster.json) contains a Claude chief and aide, a Codex captain, and a headless sentry. Version 1 requires `version`, `seats` and `routes`. Each seat has a unique case-sensitive `name`, `role`, `host`, and optional `workspace`. Captains require a workspace and each workspace can have one captain. Names and workspace IDs use ASCII letters, digits, hyphens and underscores. Hosts are `claude-desktop`, `codex-desktop` and `headless`. Roles are chief, aide, captain, lieutenant, sergeant, sentry, specialist and worker.

Each route has a `from` and a `to` seat. Add the reverse pair explicitly when replies are wanted. Both ends must exist and differ. The checker rejects duplicate routes and unknown fields. A roster can describe a single unit or a shift. It contains no session credentials or live bindings.

## Plugin package

Run `make update-plugins` from the checkout with Cargo, Codex and Claude Code on PATH. It installs the Formation CLI into `~/.cargo/bin` and installs the [Formation skill](../plugin/skills/formation/SKILL.md) in both clients from this checkout. Keep the checkout in place and the binary directory on PATH. Rerun the target after pulling changes, then open a new session in each client.

For CLI-only use, run `make install`. Checking a roster works offline without a provider account, Orbit or a broker. To try the plugin in a disposable Claude Code session, launch `claude --plugin-dir ./plugin` from the checkout.

## Local inbox

Build the CLI with `make build` and use `target/debug/formation`, or use the installed binary. Create one local, private directory per shift and give each seat its own identity file. In this example the operator runs both seats from one shell:

```sh
export FORMATION_ROOM="$(mktemp -d)"
export FORMATION_IDENTITY="$FORMATION_ROOM/chief.key"
formation mail join chief
FORMATION_IDENTITY="$FORMATION_ROOM/captain.key" formation mail join captain
formation mail who
formation mail send captain --id check-1 --body 'CI finished'
FORMATION_IDENTITY="$FORMATION_ROOM/captain.key" formation mail inbox --wait 30
FORMATION_IDENTITY="$FORMATION_ROOM/captain.key" formation mail ack 1
```

All mail commands return JSON. A seat is online for 90 seconds after activity. A waiting inbox renews that seat every 15 seconds and checks SQLite every half second inside one bounded command. An empty array means the wait expired. Each read returns up to 32 unread messages, oldest first. A recipient acknowledgment is repeatable and retains the stored envelope. A retry with the same sender, ID, recipient and body returns the original receipt, even after acknowledgment or expiry of the recipient's presence. A changed payload using the same ID fails.

The room is for cooperating processes on one machine under one OS account. Any joined seat can send to any online seat in that room. The identity file selects the seat and prevents accidental reads or acknowledgments for a different seat. Processes with direct filesystem access under the same account can access the database and credentials. The CLI requires a private room and creates seat files with owner-only access. Keep the room on a local filesystem. The roster validator and Scribe groups are independent of room membership.

Existing rooms keep their seat keys, messages and receipt numbers. The first database write upgrades an unversioned inbox in place. Early builds could create `inbox.sqlite3` with mode 0644 inside a private room. The shared SQLite driver requires an owner-only file. If the diagnostic identifies that permission issue, run `chmod 600 "$FORMATION_ROOM/inbox.sqlite3"` for your own room and retry. Newly created databases already use mode 0600.
