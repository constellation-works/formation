---
title: Formation — Shift messaging
owner: codex
created_date: 2026-11-24
updated_date: 2026-11-24
status: Draft
feature: shift-messaging
doc_role: scope
summary: A standalone plugin for directed, operator-approved messages between desktop and headless sessions.
stage: building
next: Exchange a message and reply between Claude and Codex seats with assigned mailbox sentries.
---

# Formation — Shift messaging

## Problem

Daniel talks mainly to a chief and an aide, and occasionally directly to the captain of a workspace. Each chat carries context worth preserving. An order can reach a captain’s inbox while the captain is working or its desktop chat is idle. Daniel becomes the messenger exactly when an exception needs a decision.

## Goal

The first milestone is a **network formation**, a shift whose seats run on different providers. On one machine, a Claude chief sends an order to a Codex captain. A sentry watches the captain’s mailbox and reports the order to the captain. A reply follows an authorized return route to the chief's mailbox and its assigned watcher. Each side keeps its existing session, and neither principal has to poll while working. Whether an idle principal starts a turn is a separate result for each receiving host.

The first implementation provides an offline roster checker and a local SQLite inbox with seat credentials, presence, idempotent sends, bounded sentry waits and explicit acknowledgments. A room admits cooperating seats under one OS account. Directed route grants, binding to a provider session and host-managed wake remain proposals. Existing Formation role briefs still govern the team's work. Orbit may supply a task link or CI event through an adapter. Formation can operate independently of an Orbit installation.

## Shape

**Seats and routes.** A shift gives names such as chief, aide, orbit-captain and orbit-sentry a common lifetime. The chief and aide are normally desktop sessions. Each workspace has one captain, who can also be a direct contact. Supporting roles may be headless. A roster proposes directed pairs. An authenticated human approves the final seat-to-session bindings, permitted routes and expiry. Rebinding a seat to a new session invalidates grants to the old binding. The aide can relay an attributed instruction; quoting Daniel in a message gives it no additional authority.

**Future plugin and host boundary.** Claude and Codex load a thin plugin over the same Rust core. The proposed local relay persists messages, checks grants, and pushes a notification to the receiver’s authenticated adapter. Only the owning host can start a turn, queue a follow-up or confirm an existing turn consumed a message. The desktop surface stays open to Daniel while workers can use host-controlled CLI/SDK sessions. The relay never edits provider transcript files. A receiver without a verified wake capability offers an inbox or notification and reports that limitation to the sender.

**Delivery example.** The chief sends an order with a fresh message ID. Its receipt identifies the stored envelope. The captain’s sentry reports the sender, ID, receipt and body through the assigned route. The captain handles the order, acknowledges it and sends a linked reply with its own ID. If the host cannot bring the report into the chat, the envelope stays pending for the captain’s next turn.

| Receipt | What the sender can infer |
| --- | --- |
| Accepted | The envelope is durable in the relay. |
| Queued | The host has queued it behind current work. |
| Started | The owning host provided evidence of a turn for this message. |
| Acknowledged | The recipient explicitly confirmed handling. |
| Expired or revoked | The pending message can no longer start a turn. |
| Outcome unknown | Submission may have succeeded. Reconcile the host receipt before retrying. |

**Storage and failure handling.** Start with one local relay per operator and transactional SQLite storage for shift bindings, grants, envelopes and receipts. Key sends by shift, sender binding and caller-generated message ID. Replays with the same body return the stored receipt; a conflicting body is an error. Preserve per-recipient order across relay restarts. Dispatch rechecks expiry and revocation, including queued work. The owning adapter must serialize admission with a human stop or grant change. If it cannot do so, expose its weaker capability and keep autonomous wake disabled. After a crash between host submission and receipt storage, leave the outcome unknown until the adapter can reconcile it. Keep message bodies local and out of routine logs. Transport authentication establishes the sending seat; fields supplied by a sender cannot confer another seat’s identity.

**Candidate interfaces.** An operator binds a seat, grants a route and ends a shift. An agent can send on its own approved route, inspect a receipt, or acknowledge delivery to its own seat. The adapter offers a capability handshake for the exact host, session and build. Task administration, filesystem grants and model credentials remain with their current owners. Cross-machine transport and message retention after shift close need separate decisions before wider rollout.

## Sentry watches

Give repeated checking to a sentry when a busy seat needs to concentrate on its order or when a network formation needs a receiver on another provider. A captain can assign watches for itself and its lieutenants. A chief or another captain can have its own mailbox watched on its receiving host. The sentry observes and reports. The named recipient owns the decision and any acknowledgment.

Each assignment names the source, what change matters, the seat that needs the result, the authorized reporting route, the cadence or bounded wait, and when to stop. For example, a lieutenant can ask its captain for a review-queue watch until a decision arrives. The sentry may report directly to the lieutenant on an assigned route, or its parent can relay. A mailbox, CI check, review queue, job or workspace attention feed can all be watched independently. Only one watcher owns a given source and scope. Resting a CI watch leaves a live mailbox watch running.

For mail, keep the recipient’s inbox identity scoped to that watch. Report each newly observed receipt once during the watch and leave it pending until the recipient handles it. For other sources, report meaningful changes and keep the cursor on a failed read. The [portable-kit proposal](../formation/references/portable.md) describes fingerprinting repeat observations. CI is one available source for that technique.

## Verification

1. Start with a cross-provider order and reply. Record both seats, provider builds, watchers, permission to send each way and where the watchers will report. Use the existing local room and authorized seats. Each receiving sentry begins its bounded wait before the sender submits a fresh marker.
2. For each leg record the sender’s receipt, the envelope the watcher read and what reached the intended conversation. Verify handling and acknowledgment by the named recipient. Retry the same send and check for the original receipt and a single envelope. An offline new recipient or unavailable reporting route is a recorded delivery gap.
3. Repeat with the receiving principal busy and then idle, in both directions. Record the receiving chat and turn IDs and any human prompt. A child report or inbox receipt proves only that step. Idle wake succeeds when the host starts a new turn in the original idle chat before human input. Where the host offers mail only at the next turn, qualify that behavior explicitly.
4. Give a busy lieutenant a second, independent watch while the cross-provider mailbox watch is active. Deliver its result to the assigned reader, stop that watch and verify that a subsequent mailbox message still reaches its watcher. CI, a review queue or a job can supply this second signal.
5. For future host adapters, test empty and bounded inbox delivery, interruptions before acknowledgment, grant revocation, expiry and recipient isolation. Reconcile uncertain host submission before retrying.

## Integration evidence and open questions

Claude Code channels can push events into an open Claude Code session. Codex app-server supports starting turns in sessions managed by its client. Each native desktop receiver still needs independent verification for existing user-owned chats, busy queuing, sender provenance and revocation. Registration alone proves only that a host can load the plugin.

- [Claude Code channels](https://code.claude.com/docs/en/channels)
- [Codex app-server](https://developers.openai.com/blog/codex-as-a-platform)
- [Desktop external-wake request](https://github.com/openai/codex/issues/61773)
- [Formation's native Codex chat request](https://github.com/openai/codex/issues/63220)

### Local sentry trial, 2026-11-24

On macOS, a Claude Desktop Code chief (Claude Desktop 6.28899.6) sent a fresh message through Formation to a Codex Desktop captain (desktop build 26.1120.41243). Before the send, the captain spawned its own sentry and completed its setup turn. The host reported the captain idle. A single blocking inbox call in the child returned the chief's message ID, body and receipt 1. The child repeated all three in its final answer. The parent history recorded child completion but still showed the original setup turn as its latest turn over three minutes later.

A second control isolated explicit child-to-parent messaging. The same captain spawned another sentry and went idle. Only then did the investigator release the sentry to read the original unread envelope. The child explicitly sent the receipt, ID and body to its parent through the Codex collaboration tool and finished. At the readback 41 seconds after child completion, the captain still showed its second setup turn as the latest turn and receipt 1 remained unread.

These observations qualify the inbox and the Codex child's ability to receive. Both parent-wake attempts remained unconfirmed on this build. A live parent waiting on its child, a CLI queue send, and a follow-up typed into the parent are different experiments. The reverse direction was deferred after the operator narrowed this trial to Codex.
