# Checking a shift roster

Run `formation roster check < roster.json` for a count of seats and directed routes, or add `--json` for machine-readable output. It accepts one UTF-8 JSON document up to 256 KiB. Success exits 0. Invalid documents exit 1 with a diagnostic on stderr. Invalid CLI arguments exit 2.

[The example](../examples/roster.json) contains a Claude chief and aide, a Codex captain, and a headless sentry. Version 1 requires `version`, `seats` and `routes`. Each seat has a unique case-sensitive `name`, `role`, `host`, and optional `workspace`. Captains require a workspace and each workspace can have one captain. Names and workspace IDs use ASCII letters, digits, hyphens and underscores. Hosts are `claude-desktop`, `codex-desktop` and `headless`. Roles are chief, aide, captain, lieutenant, sergeant, sentry, specialist and worker.

Each route has a `from` and a `to` seat. Add the reverse pair explicitly when replies are wanted. Both ends must exist and differ. The checker rejects duplicate routes and unknown fields. A roster can describe a single unit or a shift. It contains no session credentials or live bindings.

## Plugin package

Build the binary with `cargo install --locked --path crates/formation` to put it on PATH. The local Claude and Codex marketplace manifests point to the same [plugin directory](../plugin/) and [check-roster skill](../plugin/skills/check-roster/SKILL.md). For a disposable Claude Code session, launch `claude --plugin-dir ./plugin` from the checkout. For Codex, run `codex plugin marketplace add "$PWD"` from the repo root, followed by `codex plugin add formation@formation-local`. The skill invokes the installed binary. Checking a roster works without either host, a provider account, Orbit, a broker, or network access at runtime.
