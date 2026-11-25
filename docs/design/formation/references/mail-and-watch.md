---
title: "Proposal: seat mail and one watch command"
owner: claude
created_date: 2026-11-25
updated_date: 2026-11-25
status: Draft
feature: formation
doc_role: reference
summary: "A proposal to carry formations across Claude and Codex with three parts: a durable mailbox for each seat that has no native channel, an address taken from the seat's name in the open scribe group, and one watch command that blocks on the mailbox and on any number of probes (CI, pull requests, attention) so that a subagent or the app holds the wait and no model polls; with what changes in the first inbox build, who holds the watch on each host, what is not verified, the order to build and the questions for Daniel"
---

# Proposal: seat mail and one watch command

Bearing: a proposal for Daniel's review, written on 2026-11-25 by the chief of that day's formation. Nothing here is approved to build. It keeps the local inbox this repository already has, removes three parts of it and adds one command. The first build and its trial are in [shift messaging](../../shift-messaging/1_scope.md). The rules a message follows are in [network.md](network.md) and stand unchanged.

## What Daniel asked for

Daniel, 2026-11-25, in the chief's chat:

- "to extend the formation to codex crews, I have started a formation codebase."
- "I think we can keep this every simple, with mailbox for each unit that requires it, and a subagent can poll the mailbox along with other things that requires watch (mostly for codex crew for our use-case - but can be used for ci/PR/etc)."
- "for the sender address, for formation, we can use active scribe group's unit name."

## What this answers

Each part below answers something seen in the formation of 2026-11-25, which ran one Codex captain among Claude sessions on one Mac.

1. **Chief to Codex captain.** The chief wrote a comment that began `FOR <seat>` on the captain's task record, and the captain found it when it next read the record. Two sessions on the same machine talked through a tracker on another machine, and the record filled with routing notes.
2. **Codex captain to chief.** Nothing woke the chief. It read the captain's record at a fallback check twice an hour, so an answer could wait 40 minutes.
3. **The sentry polls in a model.** Each read is a turn even when nothing changed, which is why a pace script rations them ([2_design.md](../2_design.md#ranks)). [network.md](network.md#what-it-limits) names what is missing: "A sentry on a timer is the bridge until a shell can wait on an attention change."

Two earlier findings bound the answer:

- **An idle Codex parent is not woken by its child.** In the [trial of 2026-11-24](../../shift-messaging/1_scope.md#local-sentry-trial-2026-11-24) a Codex child blocked in an inbox call received the mail, and the idle parent started no turn on the child's completion or on its message.
- **Hooks cost an approval per app.** Daniel parked the [portable kit](portable.md) for that reason: "too much hassle, with approval and everything." This proposal uses no hook.

## The proposal in short

| Part | What it is |
| --- | --- |
| Mailbox | One for each seat that needs it, in one room per formation. Mail waits for a reader who is away |
| Address | The seat's name in the open scribe group. The room is named by the group's id |
| Watch | One command that blocks on the mailbox and on any number of probes, and returns when something is new. A subagent or the app holds it, so no model polls |

```sh
formation mail join                                    # open my mailbox, once
formation mail send orbit-0-capt --id ask-7 --body '…' # stored, whoever is watching
formation watch --probe ci='ci-read.sh owner/repo main' --timeout 3600
formation mail ack 12                                  # after handling receipt 12
```

## Mailbox

**Which seats.** A seat needs a mailbox when one of its edges has no native channel. Claude sessions keep the app's session messages, and a Codex parent and its children keep the collaboration tools: both wake the receiver and stamp the sender, which mail does not. Mail is for an edge that crosses providers or ends at a headless process.

| Edge | Channel |
| --- | --- |
| Claude chief to Claude captain | Session message, as today |
| Codex captain to its Codex sergeant or sentry | Collaboration tools, as today |
| Claude chief to Codex captain, and back | Mail |
| Any seat to a headless worker, and back | Mail |

On 2026-11-25 that would have been two mailboxes: the chief's and the Codex captain's.

**Room.** One room per formation, at `~/.formation/rooms/<room>/`, where `<room>` is the scribe group's id. A path is still accepted, for use outside a formation. Nothing deletes a room: the chief lists unread mail in its close-out, and a cleanup verb waits until old rooms are a problem.

**Verbs.** `mail join` opens the caller's mailbox, and a seat runs it once, beside its `scribe group join`. `mail send` stores an envelope for any open mailbox, and refuses an address nobody opened with the list of open ones, so a mistyped seat is not a silent loss. `mail inbox` shows unread mail and leaves it pending. `mail ack` marks it handled. `mail who` lists every mailbox with its unread count and whether a watch holds it.

**What changes in the first build.**

| First build | Proposal | Why |
| --- | --- | --- |
| A seat is proven by a key file, named on every call | A seat is a name | The room serves cooperating sessions under one OS account, and [usage.md](../../../usage.md#local-inbox) already says any of them can read the keys. The key only prevents a slip, and taking the name from scribe prevents the same slip with no file to pass |
| `send` refuses a recipient not seen for 90 seconds | `send` stores it, and the reply says whether a watch holds that mailbox | A captain between two watches is the usual case, and its mail must wait for it. The sender still learns that nobody will be told soon |
| `mail inbox --wait` waits for mail only | `watch` waits for mail and probes | One waiting command, so a seat never runs two timers |
| The room is a directory the operator makes | The room is named by the group and made on the first `join` | No setup step per shift |
| An envelope has no time | An envelope carries when it was sent | Mail can now wait for hours, and its reader needs its age |

Idempotent sends, the 8 KiB body limit, explicit acknowledgment and the audit of join, send and ack stay as built.

## Address

The CLI finds the room and the seat in this order:

1. `--room` and `--as`, when given.
2. Otherwise from scribe. The host puts the session's id in the shell (`CLAUDE_CODE_SESSION_ID`, `CODEX_THREAD_ID`). The CLI asks `scribe group list --json` for the open group that has a seat holding that session. The group's id is the room and the seat's name is the address.
3. Otherwise it refuses, in one line that names both flags.

- **Checked on 2026-11-25:** in the chief's Claude desktop session the variable held the same id that scribe lists for the `0-chief` seat.
- **A subagent.** A Claude subagent runs inside its parent's session, so it should resolve to the parent's seat, which is the mailbox it is meant to watch. A Codex child has its own thread, so its brief passes `--as`. Neither is verified.
- **The name is a label, not proof.** Any process under the account can pass `--as`. That changes no rule: a message is a request and never approval, and Daniel's approval travels only as his quoted words ([network.md](network.md#rules), rules 1 and 2).
- **Outside a formation** there is no group, so the caller names a room and a seat itself. That is the use for a lone CI or pull-request watch.
- **Scribe stays optional.** The lookup is one function in composition. Without scribe on the path, the flags are required and everything else works.

## Watch

`formation watch` blocks until the seat has unread mail, a probe reports something new, or the time runs out. It is the shell wait that the sentry lacked.

**A probe** is any command that prints one line per item as `<fingerprint><TAB><text>`, the shape the skill's `ci-read.sh` already has. `watch` runs each probe every `--every` seconds. The binary knows nothing about GitHub or Orbit: a new thing to watch is a new script.

| Case | What `watch` does |
| --- | --- |
| A fingerprint this seat has not been handed for this probe | Returns it, and records it as handed in the room |
| A fingerprint already handed | Stays silent |
| A probe exits non-zero | Returns `read failed` with the first line of the error, once, and again only after a read has succeeded. A failed read is not all-clear |
| Unread mail | Returns the envelopes and leaves them unread until `mail ack` |
| More than 20 items | Returns 20 and says how many wait |
| The time runs out | Returns nothing, marked expired, exit 0 |
| Another watch already holds this mailbox | Refuses, and names the flag that takes over |

The handed fingerprints live in the room, so they outlast the watcher. That replaces the sentry's `cursor` and `seen` files. The pace tiers go too: a blocked process costs no turn, so the interval can stay short, and `rest` and `watch` become stopping and starting the command. Expiry gives a sentry its hourly wake for the slow digest.

## Who holds the watch

The mailbox wakes nobody by itself. Something has to sit in `watch` and tell the seat when it returns.

| Host | Holder | Status |
| --- | --- | --- |
| Codex desktop seat | A child subagent runs `watch`. The parent keeps its turn open and waits on the child | Not verified. The trial showed only that an idle parent is not woken |
| Claude desktop seat that needs only its mail | A background command in the seat's own session. The app starts a turn when the command exits | The app's own tool description says so. Not tried with a mail wait |
| Claude unit with a sentry | The sentry session runs `watch` with the unit's probes and messages its reader | The message wake is in daily use. The sentry on `watch` is new |
| Headless process | Runs `watch` in its own loop | The inbox wait is covered by the first build's tests |

A Codex captain therefore **stands watch**: it starts the child, waits on it, handles what comes back, acknowledges the mail, and starts the child again. Its turn does not end while it is on watch. One watcher carries the captain's mail and the unit's probes together, which is the subagent Daniel sketched.

## Rules this adds

1. **Native channel first.** Mail is for an edge with no channel of its own.
2. **Record first, as before.** What must be known tomorrow goes on the task. The mail carries the ask, the record and the answer needed.
3. **One watcher per mailbox.**
4. **Acknowledge after handling and before watching again.** Unacknowledged mail comes back at the next watch.
5. **Report the reader.** A sender that is told nobody is watching says so to whoever is waiting on the answer.

## Where this differs from Daniel's sketch

| His sketch | The proposal | Why |
| --- | --- | --- |
| A mailbox for each unit that requires it | For each seat with an edge that has no native channel | It gives a test for "requires", and it keeps the app's own messages where they already wake the receiver |
| A subagent polls the mailbox and other things | One command does the polling. A subagent holds it on Codex, and the app's background command can hold it on Claude | A poll made by a model costs a turn each time. A poll made by a blocked command costs none |
| The sender address is the scribe group's unit name | The same, read from scribe by session id, with `--as` for a child and for use outside a formation | The session cannot mistype its own seat |

## What is left out

- The relay, route grants, session bindings, the receipt ladder and host wake adapters that [shift messaging](../../shift-messaging/1_scope.md) proposed. No observed need asks for them yet.
- Hooks, for the reason the portable kit was parked.
- More than one machine. Every seat on 2026-11-25 ran on one Mac.
- A script or a scheduled job sending mail to a seat. The mailbox makes it possible, and nothing here designs it.

## Limits

- **A Codex seat on watch never goes idle.** Whether Daniel's typed message reaches a parent that is waiting on its child is not verified. If it does not, a Codex captain reads its mailbox at the start of each turn and the wake is lost.
- **A watcher that dies after `watch` returns loses the probe items it was handed.** Mail is safe, because it stays unread until acknowledged. The condition is still there for the next direct read.
- **A probe runs with the caller's own environment.** How its command line is passed and what the child inherits is settled against STD-05 when it is built.
- **Rooms from the first build** hold a key column the new store does not use. Whether the schema step rebuilds the table is settled when it is built. No room from the first build is in use.

## Order to build

1. **Two trials with the build already installed, before any code.** A Codex captain starts a child on `mail inbox --wait`, waits on it, and the chief sends three messages over ten minutes, with Daniel typing once into the captain's chat during a wait. A Claude chief leaves `mail inbox --wait` in a background command and goes idle, and the captain sends once. If the Codex trial fails, the proposal stops at mail read at the start of a turn.
2. **The mailbox changes:** names for seats, the scribe lookup, sends to an absent reader, the sent time, `who`.
3. **`watch`:** probes, the handed fingerprints, the one-watcher refusal.
4. **The skill:** the join line and the stand-watch loop in the captain and chief briefs by provider, the sentry brief rewritten on `watch`, and an attention probe beside `ci-read.sh`.
5. **One formation day on it,** with a Codex captain.

## How we know

Read at the close of the first formation day that uses it.

| Measure | 2026-11-25 | Target |
| --- | --- | --- |
| Routing comments on a captain's task record | Every message between the chief and the Codex captain | None |
| Longest wait for the chief to learn of the Codex captain's answer | 40 minutes | Under a minute while a watch is held |
| Sentry turns in an hour with nothing new | One per read | None |

## Questions for Daniel

1. **Drop the key files** and let a seat be a name? Recommended: yes. Keeping them means a second flag on every call for a check that does not protect.
2. **Scribe lookup inside the CLI,** or `--as` on every call? Recommended: the lookup, with `--as` as the override. The cost is that Formation reads scribe's JSON output, which then must not change shape.
3. **Fold shift messaging into this feature** and park its relay, grants and wake adapters? Recommended: yes, keeping its trial record.
4. **A Codex captain's turn stays open while it stands watch.** Recommended: accept it if trial 1 shows your typed message still reaches it.
