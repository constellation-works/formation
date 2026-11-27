---
title: Formation — Mail log and status
owner: claude
created_date: 2026-11-27
updated_date: 2026-11-27
status: Draft
feature: mail-log
doc_role: scope
summary: The admiral reads every envelope in a room with one command, replies link into threads, and one status screen shows the room, his inbox and the latest mail.
stage: designing
next: The operator reviews this scope and its two open questions.
---

# Formation — Mail log and status

Proposed on 2026-11-27 and not yet reviewed. Nothing in it is built. It builds on the admiral-seat scope, which makes a terminal call the admiral's.

## Problem

The operator cannot read what his seats said to each other without opening each chat.

| Observed | Cause |
| --- | --- |
| 2026-11-27: one room held more than 120 envelopes, and no command listed them | `mail inbox` shows the caller's unacknowledged mail and `mail sent` the caller's own sends. Nothing reads the room |
| Seats cite earlier mail in prose, by receipt number or by a made-up id | An envelope has a sender, a recipient, an id, a body and delivery times. It has no field for the mail it answers |
| Seeing a shift at a glance takes a command per subject: seats, watches, and soon the roster and the requests | Each record has its own read command and nothing puts them on one screen |

## Goal

- One command lists a room's mail, newest first, for the admiral.
- A reply is linked to what it answers, so a matter can be followed from its first message to its last.
- One command prints the state of the current formation on one screen: the room, the seats, the admiral's inbox, the open requests and the latest mail.
- Every view has a JSON form, so a pane or a window built later reads the same commands.

Out of scope: a graphical view or a tmux layout (below), searching message text, and editing or deleting mail.

## Shape

```sh
formation mail log [--seat <seat>] [--with <seat>] [--since <time>] [--limit <n>] [--threads] [--no-watch] [--full] [--json]
formation mail show <receipt> [--json]
formation mail send <seat> --re <receipt> --id <id> --body <body>
formation status [--json]
```

### `mail log`

```
TIME    FROM              TO             STATE      RE   ID                 FIRST LINE
21:42Z  0-chief           0-advisor      delivered  124  req2-settled       REQ-2 settled by the admiral in the chief's chat…
21:38Z  0-advisor         0-chief        delivered  —    req-admiral-seat   REQ design. Record: <task>, draft PR <…>
21:33Z  orbit-0-capt      0-chief        acked      —    capt-ci-red        Outcome: base CI red on <…>
```

- **Every envelope in the room, newest first,** fifty by default. `--seat` keeps mail a seat sent or received, `--with` narrows that to one other seat, `--since` takes a time.
- **One line per envelope:** when it was stored, sender, recipient, how far it got (`stored`, `delivered`, `acked`), what it replies to, its id and its first line. `--full` prints whole bodies; `mail show <receipt>` prints one.
- **It is read-only.** It marks nothing delivered or acknowledged and renews no presence.
- **The admiral reads all of it. A seat reads its own.** Called by a seat, the log holds only envelopes that seat sent or received, and `mail show` refuses any other. A seat that pulled the whole room's mail into its context would pay for text it has no use for, and a message body is untrusted text from another session.
- **Watch mail is marked.** An envelope a watch produced shows the watch's name as its sender's source, and `--no-watch` leaves those out, because probe results outnumber seat mail.

### Threads

A thread is a set of envelopes that are linked. Nothing is guessed from ids or from who wrote to whom.

| Link | Where it comes from |
| --- | --- |
| A reply | `mail send --re <receipt>` stores the receipt of the envelope it answers |
| A request | Every envelope the request commands send carries its request ([delegated authority](../delegated-authority/1_scope.md#requests)) |

- **`--re` is optional** and takes a receipt the caller sent or received. An unknown receipt, or one the caller is no party to, is refused. The admiral may reply to any.
- **`mail log --threads` prints one line per thread:** the latest activity, the seats in it, the count, the first message's first line, and how far the last envelope got. An envelope with no link is a thread of one.
- **`mail show <receipt>` prints the envelope and the rest of its thread** in order.
- **The hook names the link.** Delivered mail that answers an earlier envelope says which, so the receiving seat need not find it in prose.

### `status`

```
room g-12, active, roster applied 15:26Z
seats     6 of 8 held; not joined: orbit-1-sgt; not on the roster: <seat>
inbox     2 waiting on you: 1 request (blocking), 1 from 0-chief
requests  3 open: 2 with 0-chief, 1 with you
watches   reader live, 7 on
recent    21:42Z 0-chief → 0-advisor   REQ-2 settled by the admiral in the chief's chat…
          21:38Z 0-advisor → 0-chief   REQ design. Record: <task>, draft PR <…>
          21:33Z orbit-0-capt → 0-chief Outcome: base CI red on <…>
```

- **One screen, read-only,** for the room the caller is in: the room and its status, the seats, the caller's inbox, the open requests, the watch reader and the last few envelopes.
- **Each line comes from a command that already exists or is already designed:** the roster view, `mail who`, `mail inbox`, `request list`, `watch list` and `mail log`. `status` adds no record.
- **A part that is not there is left out.** A room with no applied roster shows its joined seats; a build with no requests shows no `requests` line.
- **A seat may call it** and sees its own inbox and its own recent mail.

### A pane or a window, later

Not designed here. Both would read `status --json` and `mail log --json`, which is why every view in this scope and the roster view has a JSON form. On the day `status` ships, a terminal pane running it on an interval is the overview, with no further work.

### Compatibility

- `mail inbox`, `mail sent`, `mail who` and `mail send` without `--re` behave as before.
- The reply link is one nullable value on an envelope, added by an append-only schema step. Older envelopes have none and list as threads of one.
- A hook's delivery text gains one line only when an envelope has a link.

### Passed over

| Alternative | Why not |
| --- | --- |
| Group mail into threads by the pair of seats | Two seats exchange mail about many matters in a day. The grouping would be one long conversation per pair, which the `--with` filter already gives |
| Read thread membership from ids | Ids are chosen by the sender for retry safety and follow no pattern Formation set |
| Let every seat read the whole log | Cost and untrusted text, as above. A seat that needs another seat's mail asks its parent |
| Build the pane first | It would need these same reads, written once inside the pane and again for the terminal |
| A separate `thread` command group | Two more nouns for what one flag and one argument cover |

## Open questions

1. **Does a seat's `mail log` hold only its own mail?** Recommended: yes. The admiral reads the room; a seat reads what it sent and received.
2. **Is the log flat by default, with `--threads` to group?** Recommended: yes. The flat log is the overview of recent mail, and it works on mail sent before replies were linked.

## Build

1. **`mail log` and `mail show`.** The read, its filters and the seat's narrower view. It follows the admiral caller.
2. **Reply links and threads.** `--re`, the link on request mail, `--threads`, the thread in `mail show`, and the hook's line.
3. **`status`.** After the roster view and the request commands, leaving out any part that is not built.
4. **The skill.** The mail reference gains `log`, `show` and `--re`; role files tell a seat to answer with `--re`.

## Verification

| Result | Check |
| --- | --- |
| The admiral reads the room | From a terminal, `mail log` lists every envelope in the room newest first, and `--seat`, `--with` and `--since` each narrow it |
| A seat reads its own | Called by a seat, `mail log` holds only envelopes it sent or received, and `mail show` on another seat's receipt is refused |
| Reading changes nothing | An envelope's delivery state and every seat's last presence are the same before and after `mail log`, `mail show` and `status` |
| A reply is linked | After `mail send --re <receipt>`, `mail log` shows the receipt in `RE`, `mail show` prints both envelopes, and the delivering hook names the receipt |
| A wrong link is refused | `--re` with an unknown receipt, or one the calling seat is no party to, stores nothing |
| Threads group | Three linked envelopes and one unlinked one list as two lines under `--threads`, the first with a count of three |
| Status is one screen | `status` prints the room, seats, inbox, requests, watches and recent mail, and each figure equals what its own command reports at the same moment |
| A pane can read it | `status --json` and `mail log --json` carry every field the text shows |

Not verified by a check: that the first line of a message says what matters. The log shows first lines because seats are told to lead with the outcome; a seat that buries it makes a poor line.
