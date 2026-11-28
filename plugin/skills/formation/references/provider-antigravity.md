# Antigravity staffing and lifecycle

Use this reference for a seat that runs in an Antigravity conversation (the app, the IDE or the `agy` CLI). Such a seat joins a room and takes seat mail at its turns through two hooks. No qualified path outside the conversation starts one of its turns, so an idle seat reads its mail at its next turn. [Not available](#not-available) lists what Formation does not claim for it, and [Verified and unverified](#verified-and-unverified) separates the tested mechanics from the rest.

## Staffing

- Formation has no verified way to start, detach or archive an Antigravity conversation. The admiral opens the conversation and gives it the filled [seat card](../assets/seat-card.md) as its first message.
- The roster's `host` admits only `claude-desktop`, `codex-desktop` and `headless`, and the decoder refuses any other value. An Antigravity seat cannot be rostered yet. Record it on the assignment with its room and seat name, and tell the admiral the roster has no Antigravity host.
- Record the model and effort the conversation actually runs, and the conversation's ID as the host ID.
- To load the plugin, run `agy plugin install` on the `plugin/` directory of the checkout, then confirm with `agy plugin list` that `formation` is installed. The install has not been trialled in this repository.

## Session identity

- A seat's session id comes from the first set variable of `CLAUDE_CODE_SESSION_ID`, `CODEX_THREAD_ID` and `ANTIGRAVITY_CONVERSATION_ID`. In an Antigravity conversation only the last is set.
- `formation mail join` with no name looks up the conversation in the open Scribe groups (`scribe group list --json`) and joins the matching seat in the room named for the group. Use `formation mail join <seat> --room <room>` when Scribe is unavailable or the seat is ambiguous. The explicit form works without Scribe.
- The join records the conversation ID beside the seat, so later calls and hooks find the seat from the ID alone. Joining a seat held by another session takes it over and reports `rebound: true` with `replaced_session`. That session's next call without `--as` is refused with `seat_required`.
- After a join, check the route with `env -u FORMATION_ROOM formation mail who`. Outside a conversation, pass `--as <seat>` as [seat mail](mail.md) describes.

## Hook delivery

`plugin/hooks.json` at the plugin root declares two hooks. Antigravity runs hooks synchronously, so each blocks the turn while it runs. Each has a 60-second limit.

| Antigravity event | Command | Effect |
| --- | --- | --- |
| `PreInvocation` | `formation hook --provider antigravity --event user-prompt` | Marks the seat busy. With unread mail, returns `injectSteps` with one `systemMessage` that carries the envelopes. |
| `Stop` | `formation hook --provider antigravity --event stop` | With mail waiting, returns `decision: continue` and the mail as `reason`, which keeps the turn going. With none, returns `{}`. Marks the seat idle. It never holds. |

- `PreInvocation` runs before every model call, and its `invocationNum` input counts them. Mail can therefore enter a long turn at a later call, not only at the user's message.
- Whole envelopes are handed over in receipt order within 8,000 characters. A larger envelope is read with `formation mail inbox`. Hook delivery leaves it unacknowledged until `formation mail ack`.
- A conversation with no joined seat, and a conversation with no mail, get `{}` and exit 0. A missing or non-string `conversationId` exits 1 with a `hook_input` diagnostic on stderr.
- If `formation` is not on PATH, the plugin's command prints `{}` and exits 0, because Antigravity expects a JSON object on stdout.
- The plugin's Stop hook passes no `--hold`, so a turn that ends with no mail ends at once. Antigravity's Stop input carries `continuationActive`. Formation does not read it, so a live trial must confirm that a continued turn does not loop on the same mail.

## Collaboration

- Inside one Antigravity tree, a seat starts a subagent with `invoke_subagent` and messages one with `send_message` during an active turn. The Antigravity CLI names both in its conversations skill as the tools for subagents, in place of its conversation service.
- Those calls do not reach Claude or Codex seats. For a seat on another provider, use [seat mail](mail.md): `formation mail send` to its name, and read replies with `formation mail inbox` or at the next hook.
- The Claude and Codex hooks skip events that carry `agent_id`. The Antigravity hook has no such check. Whether a subagent's hooks run under its own conversation ID (unjoined, so `{}`) or under the parent's (which would let the subagent take the parent's mail) is not verified.

## Not available

Formation Rule 1 applies: a capability is claimed only after it is qualified on the receiving host. None of these is qualified for Antigravity.

- **Idle wake.** No qualified path outside the conversation starts a turn in an idle Antigravity conversation. The `agy` binary's `ConversationService` (Connect HTTP+JSON) exposes create, get, list, update and delete methods and no queue-message method, and the conversations skill documents the same five endpoints. Formation's queued wake exists only for Codex seats. The `formation watch run` reader mails an Antigravity seat like any other and queues no wake.
- **Desktop notification.** No Antigravity notification path is verified, so the chief sends no blocking-request announcement through an Antigravity host. The next list (`formation request list --waiting admiral`) stays the route, as [role: chief](role-chief.md) describes.
- **Roster host.** See [Staffing](#staffing).

## Verified and unverified

| Mechanic | Status | Basis |
| --- | --- | --- |
| Session binding from `ANTIGRAVITY_CONVERSATION_ID`, explicit join and takeover | Verified by the binary's tests | Three hook tests and one mail test, run on this worktree |
| Unnamed join from a Scribe group | Not verified for Antigravity | The Scribe lookup test uses a Claude session id |
| Hook output shapes: `injectSteps`, `decision: continue`, `{}`, and the `hook_input` exit 1 | Verified by the binary's tests | Same tests; the shapes match the Antigravity CLI's hook contract |
| Plugin manifest and root `hooks.json` | Accepted by `agy plugin validate --strict` with no errors | Loader reports one hook group and one skill |
| Live hook delivery in an Antigravity conversation | Not trialled | Trial 10 in the shift-messaging scope |
| Continued turn after a Stop with mail | Not verified | See the `continuationActive` note under [Hook delivery](#hook-delivery) |
| Subagent use of the hooks, under its own or the parent's ID | Not verified | See [Collaboration](#collaboration) |
| `invoke_subagent` and `send_message` in a tree | Named by the Antigravity CLI's docs; not exercised by Formation | Conversations skill |
| Idle wake | Not available | No queue-message method |
| Desktop notification of a blocking request | Not available | No verified path |
| Antigravity roster host | Not available | Roster decoder refuses the value |
