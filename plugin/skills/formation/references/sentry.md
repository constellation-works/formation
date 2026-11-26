# Sentry boundaries and watch loop

A sentry watches Orbit attention (when configured) and CI only (`assets/brief-sentry.md`) and does not wait on a Formation mailbox for its parent. Host plugin hooks deliver Formation mail directly to joined Claude and Codex sessions.

- The room’s shared watch set (`formation watch add`) and leased reader (`formation watch run`) are primary; standbys join through the shared room. Keep `formation watch wait --probe <name>=<command> --timeout 3600` only for a standalone blocking wait in the caller’s own turn.
- Use `formation watch check -- <command>` to dry-run a probe without affecting the room, `formation watch list [--json]` to inspect reader/standby state and watch results, `pause`/`resume` for `rest`/`watch` orders, `remove` to drop ad hoc watches, and `stop` only at deliberate shift close-out.
- Probes run in the active reader’s environment and saved `cwd`; `read failed` in `watch list` and one failure mail flag a broken probe until a clean read recovers. A `watch: nothing has read...` hook line means no reader holds the lease: run `watch list` and start `formation watch run`.
- Do not wait on another seat’s mailbox or poll it for incoming mail. If a headless seat has no hooks, that seat reads and acknowledges its own mail directly with `formation mail inbox` and `formation mail ack`.
- Claim a desktop wake only after the receiving host shows a subsequent turn in that exact chat. A subagent completion alone does not start a turn in an idle Codex parent.

## Coverage

The attention probe (`scripts/attention-read.sh`) holds no Operator capability. It accepts only a host-qualified selector, `hm_<owner>/ws_<name>` from `orbit.workspace.list`, and pins both of its reads to the owner host. A selector without an owner is refused before any read.

The probe covers every attention kind except the four Operator-only kinds: `run_failed`, `run_stuck`, `claim_stale` and `host_unhealthy`. The fast read omits the digest kinds (`friction_untriaged`, `notice_open`, `review_waiting`). The critical read returns only critical digest items. The sentry's hourly digest reads the digest kinds in full.

Orbit answers an unprivileged read of the Operator-only kinds with a `capability` degradation, so the probe never requests them. Read them on an authorized path:

- a live resident whose mandate covers the workspace, which reads them read-only; or
- an Operator session's own Orbit read, `orbit.attention` with `"host": ["hm_<owner>"]` and `"kind": ["run_failed", "run_stuck", "claim_stale", "host_unhealthy"]`.

A `capability` degradation on a path means that path lacks the Operator capability. Report it to the admiral and do not treat the empty result as clear.

Any other degraded entry, a truncated page or malformed counts fails the probe's read closed. The reader keeps every remembered fingerprint, reports the failure once and resumes when a complete read returns.
