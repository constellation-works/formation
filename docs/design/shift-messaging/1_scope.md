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
next: Qualify parent wake independently of inbox delivery on each desktop host.
---

# Formation — Shift messaging

## Problem

Daniel talks mainly to a chief and an aide, and occasionally directly to the captain of a workspace. Each chat carries context worth preserving. Today a sentry can notice failed CI and leave a task comment while the captain’s idle desktop chat waits for its next prompt. Daniel becomes the messenger exactly when an exception needs a decision.

## Goal

A headless sentry reports a failed check to its captain. The Codex captain can ask a Claude chief for a decision and receive the answer in the same conversation. Daniel authorizes those directed routes for one shift. Each receiving session retains its original identity, history and tool permissions. A fresh alert can wake an idle recipient without a chat polling for mail.

The first implementation provides an offline roster checker and a local SQLite inbox with seat credentials, presence, idempotent sends, bounded sentry waits and explicit acknowledgments. A room admits cooperating seats under one OS account. Directed route grants, binding to a provider session and host-managed wake remain proposals. Existing Formation role briefs still govern the team's work. Orbit may supply a task link or CI event through an adapter. Formation can operate independently of an Orbit installation.

## Shape

**Seats and routes.** A shift gives names such as chief, aide, orbit-captain and orbit-sentry a common lifetime. The chief and aide are normally desktop sessions. Each workspace has one captain, who can also be a direct contact. Supporting roles may be headless. A roster proposes directed pairs. An authenticated human approves the final seat-to-session bindings, permitted routes and expiry. Rebinding a seat to a new session invalidates grants to the old binding. The aide can relay an attributed instruction; quoting Daniel in a message gives it no additional authority.

**Future plugin and host boundary.** Claude and Codex load a thin plugin over the same Rust core. The proposed local relay persists messages, checks grants, and pushes a notification to the receiver’s authenticated adapter. Only the owning host can start a turn, queue a follow-up or confirm an existing turn consumed a message. The desktop surface stays open to Daniel while workers can use host-controlled CLI/SDK sessions. The relay never edits provider transcript files. A receiver without a verified wake capability offers an inbox or notification and reports that limitation to the sender.

**Delivery example.** The sentry submits a check URL and a unique message ID on its approved route. The relay records the envelope before acknowledging acceptance. A busy captain queues the alert behind its active turn. When ready, the adapter checks the binding and grant again and submits with agent authorship. The chief receives an escalation only on a separately approved captain-to-chief route. A reply names the original message and requires a valid return route.

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

## Sentries and delivery into a chat

The [portable-kit proposal](../formation/references/portable.md) suggests two ideas for the next trial. A script can recognize repeat observations before a model spends a turn, and an adapter can offer pending mail when the recipient already has a turn. Both use the existing Formation room, seat credentials and receipts.

**Watch one CI check first.** Give the captain a sentry seat and one assigned watch. The [CI reader](../../../plugin/skills/formation/scripts/ci-read.sh) already emits a fingerprint and a report for each red item. A script can compare observations and send the first failure with its evidence link. A subagent can use the same seat when the captain also needs someone to interpret a failure. Only one watcher owns a given watch at a time.

Keep the watch fingerprint separate from the mail ID. Repeated observations of the same failure produce one report. Capture the report body and a fresh mail ID before sending, then reuse both if the send outcome is uncertain or the recipient is offline. Mark the change reported after Formation returns its receipt. If the source confirms recovery and the same check later fails again, send a fresh report. A failed or partial source read leaves the last known state intact. A burst of related failures can share one envelope with their evidence links.

**Offer pending mail at the next supported opportunity.** Qualify the receiving adapter for each host and build. Start with an operator-initiated turn, then try delivery during an active turn, at turn end and after the chat is idle. Enable each behavior only after observing it in the intended chat. Where idle wake is unavailable, mail stays pending for the recipient’s next turn.

A hook must establish that it is running for the bound recipient before reading mail. A child hook sharing the project directory cannot assume its parent’s seat. An empty inbox produces no injected text. For a busy inbox, offer a bounded batch with sender, ID, receipt and body, and indicate when more is pending. Repeated hook calls within one turn should offer a receipt once. Reading or presenting mail leaves it pending until the responsible recipient explicitly acknowledges handling it. An interrupted turn can therefore recover it.

## Verification

1. Validate the offline roster on Linux and macOS. Missing seats, duplicate captains and unknown route endpoints fail before contacting a host.
2. Exercise the watcher with a first CI failure, repeated identical observations, a failed source read, a confirmed recovery and a recurrence. Expect two failure envelopes, one for each occurrence. Interrupt the send after storage but before the watcher records success. Restart it and verify the retry returns the same receipt. Repeat with the recipient initially offline.
3. Bind two headless test seats and approve one route. Deliver once, restart the relay and replay the same ID. Check the recipient turn, stable receipt and absence of an implied reverse route. Revoke a queued grant and expire a shift before delivery. If the host stops after submission, reconcile its receipt before any retry.
4. In disposable desktop chats, send a unique marker at each of the four opportunities above, first Claude chief to Codex captain, then in reverse. Record the host build, chat ID, mail receipt and receiving turn ID. A next-prompt delivery succeeds when that prompt’s turn contains the marker. Idle wake requires a new turn in the already-idle chat before any human input. A busy chat must finish its current work uninterrupted.
5. Exercise the hook with an empty inbox, an oversized backlog, repeated calls in one turn and a call from a child. Check silence, bounded output, per-turn deduplication and seat isolation respectively. Interrupt the parent turn before acknowledgment and verify its mail is still readable.
6. In the operator trial, a real CI failure reaches the captain and a linked reply returns to the chief. Record which delivery opportunity each host supported and whether Daniel had to prompt either chat.

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
