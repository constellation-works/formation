---
title: Formation — Shift messaging
owner: codex
last_updated: 2026-11-24
status: Draft
feature: formation
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

## Verification

1. The offline example validates on Linux and macOS. A missing seat, duplicate captain or unknown route endpoint fails before any host is contacted.
2. First live slice: bind two test seats to headless hosts, approve one route, deliver once, restart the relay and replay the same ID. Verify one recipient turn, a stable receipt and no implied reverse route.
3. For each desktop adapter, use a disposable existing chat. Deliver while idle and while busy; verify the exact background chat gets one new turn, the active turn finishes uninterrupted, and sender identity appears in its transcript. Repeat in both directions with a real chief/captain pair. Store the host versions and receipts.
4. Revoke a queued grant, expire a shift, restart after host submission and stop a recipient manually. Verify no unauthorized wake or blind retry and an accurate sender-visible result. If only notification works, report notification in the capability matrix.
5. Run the first operator trial with a CI alert and a linked reply. Success means Daniel sees the full exchange in the two original chats without forwarding a message or setting up a chat polling loop.

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
