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

Send the roster before `detach_session`. Detach the aide, captains, specialists, sergeants and sentries after briefing. Keep a lieutenant attached to its captain for turn-end results. Use native session messages by roster ID for same-host routes. An idle, detached Claude session has received a native peer message and started a turn in the tested host. Formation mail requires its own receiving watch.

A background sleep may notify a live Claude sentry on exit. Confirm that capability before promising a watch.

Inspect context with `get_usage` if available. The chief and any captain in active operator conversation compact. For a run-output-heavy session with a confirmed handoff, `clear_session` with self is its last act. The parent sends the restart line identifying the handoff. Re-establish any lost wake, scratch cursor and group seat before proceeding.

If the app refuses the clear because of live work, queued input, pinning or permissions, honor that refusal. The operator can type /clear or /scribe continue with the handoff ID where applicable.

Before archiving a replaced captain, have it detach lieutenants it started. At authorized close, archive roster sessions by ID after reading their records, leaving the aide to the operator. If pinned, on screen or busy, report the session. A visible chief or captain can ask the operator to archive it when self-archive is refused.
