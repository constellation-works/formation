# Sentry boundaries and watch loop

A sentry watches Orbit attention (when configured) and CI only (`assets/brief-sentry.md`) and does not wait on a Formation mailbox for its parent. Host plugin hooks deliver Formation mail directly to joined Claude and Codex sessions.

- Run `formation watch` with `--timeout 3600` using `scripts/attention-read.sh` and `scripts/ci-read.sh` (or `ci-read.sh` alone when Orbit is absent). Send returned items most severe first via native message, or pass `--to <reader>` when the reader has a Formation mailbox.
- Expiry (`3600` seconds) is the sentry's wake for the hourly digest. `rest` stops the running `formation watch` command, `watch` starts it, and `{scratch}/mute` drops muted lines.
- Do not wait on another seat’s mailbox or poll it for incoming mail. If a headless seat has no hooks, that seat reads and acknowledges its own mail directly with `formation mail inbox` and `formation mail ack`.
- Claim a desktop wake only after the receiving host shows a subsequent turn in that exact chat. A subagent completion alone does not start a turn in an idle Codex parent.
