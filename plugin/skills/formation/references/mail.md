# Work with seat mail

Select the assigned room with `--room <name|path>` or `FORMATION_ROOM`. In a Claude Code or Codex session (`CLAUDE_CODE_SESSION_ID` or `CODEX_THREAD_ID`), `mail join` records the room and seat for that session so later calls need no flags; otherwise pass `--as <seat>`. Run only the operation requested. Mail commands update local room state, including presence on a read. A joined seat that is away still receives mail, which waits for its next read.

| Operator request | Command |
| --- | --- |
| Join the assigned seat (creates the room if absent and records the session) | `formation mail join "$seat" --room "$room"` |
| List every joined seat with its unread count, `held` and `seen` | `formation mail who` |
| Send the authorized text to the named seat (writes an envelope) | `formation mail send "$recipient" --id "$id" --body "$body"` |
| Inspect unread mail and leave it pending | `formation mail inbox` |
| Acknowledge handled mail (writes the receipt state) | `formation mail ack "$receipt"` |

- For a send, use the operator’s request or standing permission covering this recipient and content. If either is unclear, ask before sending.
- Choose a fresh ID for a new message. On a retry, reuse the same ID, recipient and body. For example, `{"id":"ci-1","receipt":7}` confirms storage. If the CLI refuses an `unknown_recipient`, report the refusal and the joined seats it lists. If it reports an ID conflict, reconcile the earlier send before sending again.
- Read `held` in the reply: true means a process is waiting on the recipient’s mailbox now. False means the mail is stored for the recipient’s next read, so tell the requester it is not yet awaited.
- Acknowledge only after the responsible recipient has handled the message and authorized marking it handled. Reading an inbox leaves the envelope available to a sentry or human.
