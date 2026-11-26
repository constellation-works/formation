# Claude staffing and lifecycle

Use this procedure in a Claude Desktop Code chat that exposes the session tools. In a headless or CLI session inspect the available tools. A plugin install alone does not provide desktop session management.

| Rank | Claude default |
| --- | --- |
| Chief | Current Fable or Opus, high effort |
| Aide | Haiku, high |
| Captain | Opus, high |
| Lieutenant | Opus, medium |
| Sergeant or specialist | Sonnet, medium |
| Sentry | Haiku, medium |

A named Sonnet in a duo selects a sergeant who watches on its own wakes. Keep Haiku for a Claude sentry. Apply the operator’s actual model and effort choices to other seats. Check `get_session` on self before staffing. A child cannot exceed its parent’s model or starting effort. Ask for the required parent capacity if unavailable. Confirm the child’s effective settings. Where supported, `set_session_effort` changes effort for its next turn.

On a requested formation in Constellation, turn off the Claude Docs connector for formation sessions before spawning if the app offers `set_session_connector_enabled`. Inherit the permission mode. Use `start_session` with fresh context, name, model, effort, filled background and prompt, on the operator’s own initiating turn. Use `set_session_title` for readable titles. Shared-checkout seats use no worktree.

Implementation lieutenants use their own persistent worktrees. A peer-initiated turn may require proposing the start for the operator or waiting for their typed turn.

Send the roster before `detach_session`. Detach the aide, captains, specialists, sergeants and sentries after briefing. Keep a lieutenant attached to its captain for turn-end results. Use native session messages by roster ID for same-host routes; an idle, detached Claude session starts a turn on a native peer message.

For shared watches and cross-provider routes, join Formation mail once (`formation mail join`). `UserPromptSubmit` adds unread mail to a typed turn, and `Stop` uses `asyncRewake` (3300s hold, 3600s host timeout, one live waiter per seat) to wake the session if mail arrives while waiting. After the hold expires, mail waits for the next hook or [manual wake](mail.md#wake-an-unheld-claude-seat).

Each attention-owning captain registers `attention` and `ci` with `formation watch add` and starts `formation watch run` in a background shell; extra captains join as standbys. Only one reader holds the 45s lease (15s heartbeat); `formation watch list [--json]` shows the active reader, standbys (`reader.state`: `live`, `none` or `stopped`), and per-watch `last_result` (`ok, ...` or `read failed`).

A background `formation watch run` process lives only as long as that session’s background shell survives. If every reader exits without `formation watch stop`, the next hook delivers a one-time `watch: nothing has read...` notice to affected seats; restart `formation watch run` in a live seat’s background shell. Keep `formation watch wait` only for a standalone blocking wait (confirm background exit notification before relying on it).

Inspect context with `get_usage` if available. The chief and any captain in active operator conversation compact. For a run-output-heavy session with a confirmed handoff, `clear_session` with self is its last act. The parent sends the restart line identifying the handoff. Verify `formation watch list` and re-arm a reader if `none`, then re-establish the group seat before proceeding.

If the app refuses the clear because of live work, queued input, pinning or permissions, honor that refusal. The operator can type /clear or /scribe continue with the handoff ID where applicable.

Before archiving a replaced captain, have it detach lieutenants it started. At authorized shift close, issue `formation watch stop` before archiving roster sessions so readers exit cleanly, read session records and leave the aide to the operator. If pinned, on screen or busy, report the session. A visible chief or captain can ask the operator to archive it when self-archive is refused.
