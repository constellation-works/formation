# Sentry mail boundaries

A sentry watches Orbit attention and CI only (`assets/brief-sentry.md`) and does not wait on a Formation mailbox for its parent. Host plugin hooks deliver Formation mail directly to joined Claude and Codex sessions.

- Do not wait on another seat’s mailbox or poll it for incoming mail. If a headless seat has no hooks, that seat reads and acknowledges its own mail directly with `formation mail inbox` and `formation mail ack`.
- Claim a desktop wake only after the receiving host shows a subsequent turn in that exact chat. A subagent completion alone does not start a turn in an idle Codex parent.
