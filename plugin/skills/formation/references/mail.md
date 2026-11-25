# Work with seat mail

Use the assigned room directory and identity file through `FORMATION_ROOM` and `FORMATION_IDENTITY`. Run only the operation requested. Mail commands update local room state, including presence on a read.

| Operator request | Command |
| --- | --- |
| Join the assigned seat (writes the binding and may create a credential file) | `formation mail join "$seat"` |
| List seats currently online | `formation mail who` |
| Send the authorized text to the named seat (writes an envelope) | `formation mail send "$recipient" --id "$id" --body "$body"` |
| Inspect unread mail and leave it pending | `formation mail inbox` |
| Acknowledge handled mail (writes the receipt state) | `formation mail ack "$receipt"` |

- For a send, use the operator’s request or standing permission covering this recipient and content. If either is unclear, ask before sending.
- Choose a fresh ID for a new message. On a retry, reuse the same ID, recipient and body. For example, `{"id":"ci-1","receipt":7}` confirms storage. If the CLI refuses an offline recipient, report the refusal. If it reports an ID conflict, reconcile the earlier send before sending again.
- Acknowledge only after the responsible recipient has handled the message and authorized marking it handled. Reading an inbox leaves the envelope available to a sentry or human.
