<div align="center">
  <h1>Formation</h1>
  <p>Run a shift of Claude and Codex sessions by rank and workspace.</p>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version 0.1.0"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
  <a href="https://github.com/constellation-works/formation/actions/workflows/ci.yml"><img src="https://github.com/constellation-works/formation/actions/workflows/ci.yml/badge.svg?branch=agent-main" alt="CI"></a>
</div>

Formation staffs a working day of AI coding sessions: one chief, one captain per workspace, and the cheaper ranks under each captain, all with fixed reporting lines. It ships as a Claude Code and Codex plugin, along with a `formation` CLI that gives the sessions a private local mailbox and a shared watch on CI and pending work.

**Work in progress, not ready for use.** Formation is pre-1.0, runs on macOS and Linux, and its roles, commands and room format still change without notice.

## Why

- **You stop being the router.** Each session knows who it reports to and what it may settle alone, so questions go up the chain instead of to you.
- **Cost follows the work.** Ranks fix each session's model and effort: Opus commands, Sonnet triages, Haiku watches.
- **Mail that arrives.** Seats send durable messages in a local room, and hooks hand them to the receiving chat on its next turn, or wake it.
- **One reader watches for everyone.** Probes for CI and pending work run under a single lease per room and mail only what changed.
- **The day ends cleanly.** Close-out writes a handoff, a workspace with pending work can pass to an Orbit resident overnight, and the next day starts fresh.

## How it works

```text
You:      formation squad orbit, solo scribe
chief:    Files the shift record, starts the aide and one captain per workspace,
          and sends each its brief.
captain:  orbit-0-capt starts a lieutenant, a sergeant and a sentry, then
          registers the attention and ci watches and runs the room's reader.
sentry:   Watches orbit: a red check on a pull request is mailed to the sergeant.
sergeant: Fixes it, or asks the captain when it needs a decision.
You:      formation-complete
chief:    Closes each unit, stops the reader and hands the day off.
```

Units are solo (a captain), duo (plus a sentry), trio (plus a sergeant) and squad (plus a lieutenant). The [units and ranks](plugin/skills/formation/references/units.md) reference has the full line-ups and models.

## Quick start

**You need:** Rust at or above the `rust-version` in [Cargo.toml](Cargo.toml), `jq`, and Claude Code or Codex on PATH, on macOS or Linux.

```sh
git clone https://github.com/constellation-works/formation && cd formation
make update-plugins
```

1. `make update-plugins` installs the `formation` CLI into `~/.cargo/bin` and the plugin (the skill and its hooks) into both clients. Keep the checkout where it is.
2. Open a new chat, because hooks load only when a chat starts. Codex asks you to review and trust the hooks once.
3. Check the example roster offline: `formation roster check --json < examples/roster.json` prints `{"version":1,"seats":4,"routes":5}`.
4. Try the mailbox from a shell: `formation mail join chief --room trial`, then `formation mail who --room trial --as chief`.
5. In the Claude desktop app's Code tab, open a session with `formation solo <workspace>` to staff your first shift.

<details>
<summary>CLI only, or a disposable plugin session</summary>

`make install` installs only the CLI. Roster checks and the mailbox work offline, with no provider account, Orbit or broker. To try the plugin without installing it, run `claude --plugin-dir ./plugin` from the checkout.
</details>

## Features

- **Staffing skill.** Parses a formation order, fills each rank's brief, records the roster and runs the daily start, changes, reinforcements and close-out, for both Claude and Codex hosts.
- **Roster check.** `formation roster check` validates names, ownership and the direction of each route. A passing check grants nothing and starts nothing.
- **Seat mail.** `formation mail` handles join, who, send, inbox, ack and sent in a private SQLite room, with idempotent sends and per-seat presence.
- **Delivery hooks.** `formation hook` adds unread mail to the next turn. A Claude Stop hook holds a waiter that wakes the chat when mail arrives, and an idle Codex seat can be woken through its queue.
- **Watches.** `formation watch` adds, checks, pauses and lists probes, and runs one shared reader per room with standby takeover. The plugin ships attention and CI probes.
- **Repair mode.** A squad can take over red and conflicting pull requests. When a drain already runs with auto-fix, the squad drops its sentry and its sergeant keeps the watch.

## Learn more

- [Usage](docs/usage.md): roster format, the plugin install, the mailbox, hooks and watches, command by command.
- [Formation skill](plugin/skills/formation/SKILL.md): how the chief staffs and runs a shift.
- [Formation design](docs/design/formation/2_design.md): ranks, units and authority. The [overview](docs/design/formation/1_overview.md) holds the problem and the measures.
- [Shift messaging](docs/design/shift-messaging/1_scope.md) and [watch set](docs/design/watch/1_scope.md): delivery and watch scope, and what has been verified.
- [Feature board](docs/design/FEATURES.md): what is being built and what comes next.
- [Architecture](ARCHITECTURE.md): crate boundaries and the dependency gate.
- [Changelog](CHANGELOG.md): release history.

## Contributing

Read [development](docs/development.md) for setup and the gates that must pass before handoff. Changes integrate on `agent-main`.

## License

[MIT](LICENSE).
