# Work with seat mail

Native channels stay first: use Claude session messages between Claude sessions and Codex collaboration tools inside a Codex tree. Use Formation mail for an edge that crosses providers, or when a headless script mails a seat.

A seat with a cross-provider edge runs `formation mail join` once, beside its `scribe group join`. In a Claude Code or Codex session with an open Scribe group, `formation mail join` records the room and seat from Scribe so later calls and hooks need no flags. Pass `"$seat" --room "$room"` when Scribe is unavailable or outside a session, and `--as <seat>` when calling outside a joined session.

`UserPromptSubmit` adds unread mail to the current turn. Hook delivery marks the envelope delivered. Use `formation mail inbox` and `formation mail ack "$receipt"` for a seat with no hook, such as a headless worker, or to read and acknowledge an envelope too large for the hook budget.

| Admiral request | Command |
| --- | --- |
| Join from an open Scribe group (or pass `"$seat" --room "$room"`) | `formation mail join` |
| List every joined seat with its unread count, `held` and `seen` | `formation mail who` |
| Send the authorized text to the named seat (writes an envelope) | `formation mail send "$recipient" --id "$id" --body "$body"` |
| Inspect unread mail and leave it pending | `formation mail inbox` |
| Acknowledge handled mail (writes the receipt state) | `formation mail ack "$receipt"` |

- For a send, use the admiral’s request or standing permission covering this recipient and content. If either is unclear, ask before sending.
- Choose a fresh ID for a new message. On a retry, reuse the same ID, recipient and body. For example, `{"id":"ci-1","receipt":7}` confirms storage. If the CLI refuses an `unknown_recipient`, report the refusal and the joined seats it lists. If it reports an ID conflict, reconcile the earlier send before sending again.
- Use `held` as a recent-presence signal. A receipt confirms that mail is stored. Check `formation mail who` for the seat’s current `held` status and unread count.
- Acknowledge only after the responsible recipient has handled the message and authorized marking it handled. Reading an inbox leaves the envelope unacknowledged.

## Wake an unheld Claude seat

Claude’s background `Stop` hook waits for 55 minutes (3,300 seconds) with one live waiter per seat. After it expires, stored mail waits for the seat’s next hook or inbox read.

If a send to a Claude seat returns `"held": false`, use an authorized native Claude session message to the same open chat with the single line `Formation mail waiting`. If you are a Codex captain or a process, route the wake request to the chief or the seat’s captain, who can send that native message. For a closed or unloaded chat, ask the admiral to open it and start a turn.
