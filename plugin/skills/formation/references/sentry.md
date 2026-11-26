# Sentry boundaries and watch loop

A sentry watches Orbit attention (when configured) and CI only (`assets/brief-sentry.md`) and does not wait on a Formation mailbox for its parent. Host plugin hooks deliver Formation mail directly to joined Claude and Codex sessions.

- Run `formation watch wait` with `--timeout 3600` using `scripts/attention-read.sh` and `scripts/ci-read.sh` (or `ci-read.sh` alone when Orbit is absent). Send returned items most severe first via native message, or use `watch add --to <reader>` and `watch run` as shown in the sentry brief when the recipient has a Formation mailbox.
- A blocking wait expires after 3600 seconds for the hourly digest. Follow the sentry brief for `rest`, `watch` and `mute` in each mode.
- Do not wait on another seat’s mailbox or poll it for incoming mail. If a headless seat has no hooks, that seat reads and acknowledges its own mail directly with `formation mail inbox` and `formation mail ack`.
- Claim a desktop wake only after the receiving host shows a subsequent turn in that exact chat. A subagent completion alone does not start a turn in an idle Codex parent.
