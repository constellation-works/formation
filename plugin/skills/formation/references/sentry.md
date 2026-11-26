# Sentry boundaries and watch loop

A sentry watches Orbit attention (when configured) and CI only (`assets/brief-sentry.md`) and does not wait on a Formation mailbox for its parent. Host plugin hooks deliver Formation mail directly to joined Claude and Codex sessions.

- The room’s shared watch set (`formation watch add`) and leased reader (`formation watch run`) are primary; standbys join through the shared room. Keep `formation watch wait --probe <name>=<command> --timeout 3600` only for a standalone blocking wait in the caller’s own turn.
- Use `formation watch check -- <command>` to dry-run a probe without affecting the room, `formation watch list [--json]` to inspect reader/standby state and watch results, `pause`/`resume` for `rest`/`watch` orders, `remove` to drop ad hoc watches, and `stop` only at deliberate shift close-out.
- Probes run in the active reader’s environment and saved `cwd`; `read failed` in `watch list` and one failure mail flag a broken probe until a clean read recovers. A `watch: nothing has read...` hook line means no reader holds the lease: run `watch list` and start `formation watch run`.
- Do not wait on another seat’s mailbox or poll it for incoming mail. If a headless seat has no hooks, that seat reads and acknowledges its own mail directly with `formation mail inbox` and `formation mail ack`.
- Claim a desktop wake only after the receiving host shows a subsequent turn in that exact chat. A subagent completion alone does not start a turn in an idle Codex parent.
