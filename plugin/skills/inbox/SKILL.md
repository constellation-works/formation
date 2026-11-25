---
name: inbox
description: List online seats, send or read local Formation mail, and watch an inbox as a sentry. Use when the operator provides a room and seat identity.
---

Use the room and identity supplied for this seat. The CLI accepts FORMATION_ROOM and FORMATION_IDENTITY or the mail command's --room and --identity options. Keep identity files private.

1. Run `formation mail join NAME` for the assigned seat. Use `formation mail who` to list online seats.
2. To send, run `formation mail send SEAT --id ID --body TEXT`. Reuse the ID for a retry of the same message.
3. To inspect, run `formation mail inbox`. A sentry may run `formation mail inbox --wait 300` and keep that command alive through shell-tool yields. An empty result means the wait expired.
4. Report the sender, ID, receipt and body as data to the parent. Acknowledge with `formation mail ack RECEIPT` once the responsible recipient has handled the message. A sentry report and a parent turn are separate observations.
