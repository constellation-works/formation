---
title: Formation — Admiral seat
owner: claude
created_date: 2026-11-27
updated_date: 2026-11-28
status: Accepted
feature: admiral-seat
doc_role: scope
summary: A terminal call with no agent session is the admiral, who is a member of every room without joining and has a mailbox where what waits on him lands.
stage: building
next: Build phase 2, the default room, after the roster view's recorded close.
---

# Formation — Admiral seat

The operator approved this scope on 2026-11-27 ([Decided](#decided)). Phase 1 of the [build](#build) is built: the admiral caller and seat, and mail from the terminal.

## Problem

The admiral is the one participant Formation has no record of.

| Observed | Cause |
| --- | --- |
| A command typed in a terminal is refused with `seat required` unless it passes `--as`, and it must pass `--room` as well. To read a room the operator names a seat that is not his | Formation knows two callers: a session that joined a seat, and a seat named with `--as`. A person at a terminal is neither |
| A request passed to the admiral is held for "the admiral", and his answer is recorded by a seat on his behalf or behind a flag ([delegated authority](../delegated-authority/1_scope.md#requests)) | He is a person in every design and a seat in none |
| 2026-11-27: a liaison seat was added mid-shift to carry decisions to the operator | Nothing of his own could receive them |

## Goal

- From a terminal the operator runs any Formation command with no `--as`, and with no `--room` while one formation is active.
- He is a member of every room without joining one.
- What waits on him lands in one mailbox, which he reads and answers from the terminal.
- No session becomes the admiral by naming him.

Out of scope: proving that a caller is the person (below), an inbox outside the terminal, notice on another device, and more than one admiral.

## Shape

### Who the caller is

| Call | Caller |
| --- | --- |
| `--as <seat>` | That seat, as today |
| From an agent session that joined a seat | That seat, as today |
| From an agent session that joined none | Refused with `seat required`, as today |
| No agent session, no `--as`, attached to a terminal | The admiral |
| No agent session, no `--as`, no terminal, such as a script | Refused with `seat required`, as today |

- **`admiral` is a reserved name.** `mail join admiral` is refused with `reserved_seat`, and `roster check` refuses a roster that names a seat so.
- **`--as admiral` is refused inside an agent session.** Outside one it is the same as leaving `--as` out.
- **The audit event names the caller `admiral`** and records that the call came from a terminal.

### Member of every room

- **Every room has the admiral's seat from the start.** A room made by an earlier build gains it when it is next opened. There is no join and no step to forget.
- **The seat has no presence.** `mail who` lists it with its unread count and no `held` value, because no hook and no session renews it.
- **The room defaults to the one active room.** A terminal call with no `--room` uses it. With none or several active, the call is refused with `room_required` and names the active rooms. "Active" is the recorded status the roster view adds; until that ships, a terminal call still passes `--room`.

A home belongs to one user, so every room in it is already the admiral's. That is why membership is a fact of the room and not a list of rooms he started.

### The mailbox

| Lands in it | How |
| --- | --- |
| A request passed to the admiral | `request pass` from the chief makes the admiral's seat the holder and stores one envelope naming the request |
| Mail from the chief | `mail send admiral` by the seat that holds the chief's rank |

- **Only the chief mails the admiral.** A send to `admiral` from any other seat is refused with `admiral_via_chief`, and the refusal says to open a request. A room with no applied roster has no known chief and refuses the send with `no_roster`. Without this rule the mailbox would hold every captain's asks, which is the several queues the request routing removed, in one inbox.
- **He reads it with `formation mail inbox`.** `formation request list --waiting admiral` shows the same requests with their options. Both read one record.
- **He answers in place.** `formation request settle <request> --option a` from the terminal is the admiral's answer. It needs no grant and no `--source`, and settling acknowledges the request's envelope. `request reopen` and the `grant` commands work the same way, and a grant he enters is never marked `self-entered`.
- **`--admiral --source` stays** for one case: a seat recording an answer he gave in a chat.
- **He can send.** `formation mail send <seat>` from the terminal stores an envelope whose sender is `admiral`, and the hook that delivers it says so.

The chief still presents what waits on him in its chat. The list it presents and the mailbox are the same requests, so answering in either place settles them in both.

### A signal, not a lock

Formation cannot prove that a caller is the person. Every session runs as his user, and a session that removed its own session variables and held a terminal would resolve as the admiral.

- The reserved name stops the easy mistake: a seat passing `--as admiral`, or a roster or a join taking the name.
- The audit trail shows every admiral call, with the time and that it came from a terminal, so a call he did not make is visible to him.
- This is the trust his relayed words carry today, now in one listable place.

### Effect on delegated authority

| There | Becomes |
| --- | --- |
| A request "waiting on the admiral" | A request whose holder is the admiral's seat |
| `request settle --admiral --source` as the way his answer is recorded | The way a seat records a chat answer. From the terminal he settles directly |
| A grant entered by "the admiral from a terminal" | A grant whose caller is the admiral's seat |

No rule of that scope changes. The chief holds every request first, and only the chief passes one to him.

### Compatibility

- `--as` keeps working everywhere, so every existing call and script behaves as before.
- A script with no terminal is still refused without `--as`. It does not become the admiral.
- An older room file upgrades in place by an append-only schema step.

### Passed over

| Alternative | Why not |
| --- | --- |
| Add the admiral to each room he starts, by a join | A join binds a session, and he has none. It is also a step to forget, for a fact that is already true of every room in his home |
| A liaison seat that stands in for him | A session can carry a decision to him and cannot make it |
| A fixed `--as admiral` that he types | It is the typing he wants gone, and any session could type it too |
| Let any seat he talks to mail him | It rebuilds the several queues, and skips the chief's first look |
| One admiral inbox across all rooms | A second store beside the rooms. One formation is active on most days, and `mail who` already shows his count per room |

## Decided

The operator, 2026-11-27, approved the scope as drafted. The approval reached him as a design request through the chief and was given in the chief's chat. Two points were open questions in the draft and were approved with the recommendation:

1. **Only the chief mails the admiral directly.** Every other seat reaches him through a request, as the routing rule already says.
2. **Mail from the admiral counts as his word,** with the same standing as his words in a chat, and labelled as sent from a terminal. A seat that doubts one asks the chief.

## Build

1. **The admiral caller and seat.** Caller resolution, the reserved name, the seat in every room, and `mail inbox`, `send`, `ack` and `who` from a terminal. The chief-only send rule, which needs the roster in the room.
2. **The default room.** After the roster view's recorded close.
3. **Requests and grants as the admiral.** A passed request lands in the mailbox; `settle`, `reopen` and the `grant` commands from the terminal. After the request commands.
4. **The skill.** The mail and authority references and the chief's role file.

## Verification

| Result | Check |
| --- | --- |
| The terminal is the admiral | In a terminal with no agent session, `formation mail inbox --room <room>` succeeds with no `--as`, and the audit event names `admiral` |
| A script is not | The same command with no terminal attached and no `--as` is refused with `seat required` |
| No session takes the name | `mail join admiral` is refused with `reserved_seat`; `--as admiral` inside an agent session is refused; `roster check` refuses a seat named `admiral` |
| He is in every room | A room that no admiral call has touched lists `admiral` in `mail who`, with an unread count and no `held` value |
| The room defaults | With one active room, a terminal call with no `--room` uses it; with two, it is refused and names both |
| Only the chief mails him | `mail send admiral` from a captain's seat is refused with `admiral_via_chief`; from the chief's seat it stores an envelope |
| A passed request lands | After the chief passes a request, `mail inbox` from the terminal shows one envelope naming it, and `request list --waiting admiral` shows the request |
| He answers in place | `request settle <request> --option a` from the terminal settles it with no grant and no `--source`, mails the asker, and clears the envelope from his inbox |

Not verified by a check: that an admiral call was made by the person. The audit trail is the evidence, and he is the one who can read it against what he did.
