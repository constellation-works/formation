---
title: Formation — Delegated authority
owner: claude
created_date: 2026-11-27
updated_date: 2026-11-27
status: Accepted
feature: delegated-authority
doc_role: scope
summary: Authority becomes a record the admiral changes with one command, every ask above a captain's row is a request that goes to the chief first, and the admiral reads one list of what no seat may settle.
stage: building
next: Build phase 3, the standing grants file and grant load.
---

# Formation — Delegated authority

The operator approved this scope on 2026-11-27 ([Decided](#decided)). Phases 0, 1, 2 and 4 are built: the authority and request commands, the public skill's request rule, and the notice rule in the chief and provider references. Phase 3, standing grants, is next and the only phase left before `shipped`.

The notice rule is written as not available on both Claude and Codex: no run has exercised a host notification from a chief's session, so each provider reference sends the chief to its next turn's list.

## Problem

Every decision above a captain's row reaches the admiral, and it reaches him in whichever chat the asking seat happens to be.

| Observed | Cause |
| --- | --- |
| 2026-11-27: the operator answered asks in the chief's chat, the advisor's chat and a captain's chat. One design was approved twice, once in each of two chats. A liaison seat was added mid-shift to carry decisions to him | A captain may ask the admiral directly whenever he is in its chat, and so may the advisor. No seat is the one place an ask waits |
| The same day the chief settled nothing above a captain's row. Two wording fixes to shared standards that changed no version, and a workspace's validation setting, each waited for the admiral | The chief holds no row. [Authority](../../../plugin/skills/formation/references/authority.md) says it advises and coordinates and does not approve, merge or ship |
| The operator's copy of the skill carries thirteen dated directions as paragraphs under its authority table: who may use a tracker override, who starts drains, one captain's completion flag, what a sergeant may rewrite | A grant is prose. No command reads it, the public plugin cannot carry it, and a seat learns its reach by reading the whole section |
| The chief keeps the list of what waits for the admiral by hand, as lines in comments on its own record, rewritten at each wake | An ask is free text in a mail body. Formation has no record of one |

## Goal

- An ask above a captain's row waits in one place, with the chief, and the admiral reads one list: what no seat may settle.
- The admiral widens or narrows what a rank or a seat may settle with one command, for a day or as a standing choice, and takes it back the same way.
- A decision made on the admiral's behalf names the grant it relied on, and the admiral can list every such decision and overturn one.
- Authority is Formation's own record. It reads nothing from a tracker and changes nothing a tracker or a host permits.

Out of scope: stopping a session from acting outside its row (Formation records who may decide; a host's permissions and a tracker's capabilities decide what a call can do), the unit line-ups, mail delivery, and what any rank settles by default today.

## Shape

Three records, kept together because each needs the other two: a request has a class, the class and the grants say who settles it, and the settlement names the grant.

| Record | Holds | Lives in |
| --- | --- | --- |
| Class | A named kind of decision and who settles it by default | A catalog file in the skill; a house file may add classes |
| Grant | A rank or seat, a class, a workspace, a limit, a lifetime | The room for a day grant; the house file for a standing one |
| Request | One ask: its class, options, recommendation, who holds it now, and how it was settled | The room |

### Classes

The catalog names the rows of today's authority table so that a grant can point at one. The default holders are today's.

| Class | Covers | Default holder |
| --- | --- | --- |
| `repair` | Retry a check, repair a session-owned pull request or, in repair mode, a red or conflicting one, clear an unambiguous pilot finding, file a fix or a friction | Sergeant; the captain where there is none |
| `ship` | Approve and ship a fix, an approved design, a non-breaking refactor, an improvement or a small scoped feature | Captain |
| `land` | Land a reviewed pull request | Captain |
| `staffing` | One reinforcement, up to a squad plus one lieutenant per workspace | Chief |
| `design` | Approve a design or a major feature | Admiral |
| `breaking` | A breaking change | Admiral |
| `direction` | Product scope, architecture, security, material cost, an irreversible action | Admiral |
| `release` | A release, a tag, publishing | Admiral |
| `operator` | A decision that needs the operator's identity, a routine change, a plugin install, starting a drain | Admiral |
| `override` | Put a tracker's operator override in front of one refused command, for an act in the seat's own rows | Nobody |
| `credentials` | Credentials and sign-ins | Admiral; never granted |
| `grant` | Giving or taking back authority | Admiral; never granted |

- **Two classes cannot be granted.** Either would let a seat widen its own reach. `formation grant add` refuses them, and a house file cannot change that.
- **A house file may add a class** for a decision this catalog does not name, such as a wording fix to documents several repositories share. It may not remove one.
- **A class is coarse on purpose.** A grant narrows it with a limit in words (below), so the catalog does not grow a sub-class for every case.

### Grants

```sh
formation grant add --to chief --class direction --workspace all \
    --limit "a setting inside one workspace that a later change can undo" \
    --for day --source "<chat and time the admiral said it>"
formation grant withhold --to captain --class land --workspace site --for day --source "<…>"
formation grant revoke <grant>
formation grant list [--seat <seat>] [--json]
formation grant check --seat <seat> --class <class> [--workspace <workspace>]
```

| Field | Meaning |
| --- | --- |
| `--to` | A rank or one seat. Any rank can hold a grant, the advisor and the aide included |
| `--class`, `--workspace` | What it covers. `all` means every workspace in the room |
| `--limit` | A condition in words. The holder judges whether a request meets it, and the settlement shows the limit beside the choice |
| `--for` | `day` ends with the room. A standing grant is not added by command (below) |
| `--source` | Where the admiral said it. The words themselves stay in the private store or the house file, never in this skill |

- **`withhold` narrows a default.** It is how a direction that takes a default away is recorded, such as one that leaves landing in a workspace to the admiral for a day: the same record as a grant, with the opposite effect.
- **Who enters a grant.** The admiral from a terminal, or the seat in whose chat he said it. A grant entered by the seat that gains from it is marked `self-entered` in every list.
- **A change is mailed.** `add`, `withhold` and `revoke` send one envelope to each joined seat whose reach changed, so no seat learns of it from a relay.
- **Standing grants are a file.** The [house file](../role-reference/1_scope.md#house-rules) names a grants file in the operator's own repository, in the grant's own fields. The chief loads it at start with `formation grant load`. A day grant becomes standing only by a change to that file, which the admiral approves as a diff. At close the chief lists the day's grants once and asks which to keep.

### Effective authority

`formation grant list --seat <seat>` prints what that seat settles now and where each line came from.

| When two entries disagree | The one that applies |
| --- | --- |
| One names the seat, one its rank | The seat |
| One names the workspace, one says `all` | The workspace |
| Room, house file, catalog | The room, then the house file, then the catalog |
| Same on all three | The later one; a `withhold` over a grant entered at the same time |

The seat card's line for what a seat settles is filled from this list, and each role file's "You settle" section names the command instead of repeating a row.

### The roster in the room

A grant to a rank, and the path a request takes, need each seat's rank, workspace and parent. Today the room knows only seat names, and the roster is a comment on the chief's record.

- `formation roster apply --room <room> < roster.json` records each seat's role, workspace and `reports_to` in the room. `roster check` stays offline and unchanged.
- The roster format gains `reports_to` on a seat and the `advisor` role, as version 2. A version 1 file still checks.
- A seat's identity is the session that joined it, as for mail today.

### Requests

A request starts at a seat the admiral talks to: a captain, the advisor or the aide. Those are the chats where asks wait on him today. A sergeant's or lieutenant's ask goes to its captain as it does now, and becomes a request only if the captain cannot settle it.

```sh
formation request open --class design --record <task or pull request> \
    --ask "<one sentence>" --option "a: …" --option "b: …" --recommend a [--blocking]
formation request advise <request> --recommend b --note "<why>"
formation request settle <request> --option a [--note "<…>"]
formation request pass <request> [--recommend a --note "<…>"]
formation request settle <request> --option a --admiral --source "<chat and time>"
formation request reopen <request>
formation request list [--waiting <seat>|admiral] [--settled-by <seat>] [--since <time>] [--json]
formation request show <request>
```

| Command | Effect | Refused when |
| --- | --- | --- |
| `open` | Stores the request and mails the asker's parent, which holds it. `--settle <option>` opens and settles in one step, for a seat using a grant on its own question | The room has no roster, or the class is unknown |
| `advise` | Adds a recommendation. Changes nothing else | — |
| `settle` | Records the choice and the grant that covers it, and mails the asker | No entry covers this seat, class and workspace. The refusal names who holds the class |
| `pass` | Moves the request to the holder's parent. From the chief it moves to the admiral | The caller does not hold it |
| `settle --admiral` | Records the admiral's answer with where he gave it. Run by him, or by the seat he told | — |
| `reopen` | Marks a settlement overturned and mails the asker and the seat that settled it | The caller is a seat acting without `--admiral` |

- **The asker reads the state, not the mail.** A message from another session stays a request and never an approval. A request in state `settled` is the approval: it names either the admiral's answer and its source, or a seat and a grant that `grant check` confirms.
- **A grant permits and never obliges.** A holder that is unsure passes the request up with what it found.
- **`--blocking`** means the asker's work has stopped. Everything else waits for the next batch.
- **A request's id names its room,** so one still open at close keeps its id when `formation request carry --from <room>` copies it into the next day's room.

### Routing

| Seat | What it does with an ask |
| --- | --- |
| Captain, advisor, aide | Settles what its own entries cover. Opens a request for the rest and names the request in its turn end. It does not end a turn with a question for the admiral |
| Chief | Holds every request first. Settles what its entries cover, citing the grant. For a design, an architecture question or a conflict between units, asks the advisor to `advise` when one is staffed. Passes the rest to the admiral with one recommendation |
| Advisor, specialist | Advises on a request it is asked about. Settles only under a grant of its own |
| Admiral | Answers in any chat. The seat he tells records the answer on the request |

The admiral may still decide anything in any chat, at any time. What changes is that no seat waits on him in its own chat: the wait is on the request, and the request is with the chief.

### What the admiral reads

- **One list.** `formation request list --waiting admiral` is everything that waits on him. The chief presents it at each wake and at the end of any turn he is in, each item with its options and one recommendation. The hand-kept lines on the chief's record go.
- **One log.** The same turn lists what was settled under a grant since the last time: the request, the choice, the seat and the grant. He overturns one with `reopen`. Formation undoes nothing; what was already done is undone by a new order.
- **Notice.** A `--blocking` request that reaches the admiral is announced at once through the host's notification where the chief's host has one. Everything else waits for the list.

A grant suits a class whose decisions a later change can reverse, because the log is read after the fact. `release` and the irreversible half of `direction` are poor candidates, and the catalog says so beside them.

### Formation authority and a tracker's roles

| | Formation authority | A tracker's role or capability |
| --- | --- | --- |
| Answers | Which seat may decide this kind of question | What a call may do to the tracker's records |
| Held by | A rank or a seat, per workspace, for a day or standing | A session's connection and its configuration |
| Changed by | The admiral's grant, mid-shift | Configuration the operator edits |
| Enforced | By Formation on its own records; elsewhere by the seat keeping to its entries | By the tracker refusing the call |

- A call the tracker accepts is not thereby authorized, as today. A grant does not widen what the tracker accepts.
- A seat that settles a request and is then refused by the tracker reports the refusal. The `override` class is the only entry that speaks of a tracker privilege, and it is granted like any other.
- A tracker's own decision requests stay where they are. A Formation request about the same question points at it with `--record`, and the seat that settles one answers the other when its tracker capability allows.

### Skill changes

| File | Change |
| --- | --- |
| [Authority](../../../plugin/skills/formation/references/authority.md) | The table is keyed by class. The chief's line becomes: it holds `staffing` and what it is granted |
| Role files and seat card ([role references](../role-reference/1_scope.md)) | "You settle" names `grant list`. The captain, advisor and aide files gain the request rule; the chief file gains triage, the list and the log |
| [Records](../../../plugin/skills/formation/references/records.md) | Inside a formation an open item is a request. The decision tag is unchanged |
| Start and close-out | Start applies the roster, loads standing grants and carries open requests. Close lists the day's grants |
| Evaluation cases | A captain with an ask above its row; a chief with and without a covering grant |

### Compatibility

- A room with no applied roster refuses `grant` and `request` with `no_roster`. Mail, watches and hooks are unchanged.
- The new tables are added by append-only schema steps, and an older room file upgrades in place.
- A day's formation closes with its room, so no running seat holds a card from before the change.

### Passed over

| Alternative | Why not |
| --- | --- |
| Keep adding dated paragraphs to the skill | It is the present state: thirteen of them, readable only whole, absent from the public plugin |
| Map ranks onto a tracker's roles | A tracker's role says what a call may do in one workspace. It has no word for approving a design, a reinforcement or a shared document, and where every session holds the same capability it tells the ranks apart not at all |
| A liaison seat that carries asks to the admiral | It moves the queue to another chat and settles nothing |
| Give the chief the admiral's whole reach | Releases and irreversible choices would be settled before he reads the log |
| Keep asks as open items in a notes store | The store is optional, knows no class or grant, and cannot refuse a settlement nobody was granted |
| Requests from every rank | A sergeant's routine asks to its captain are frequent and never reach the admiral. The form would cost more than it returns there |

## Decided

The operator, 2026-11-27, in the advisor's chat, approved the scope as drafted. Four points were open questions in the draft and were approved with the recommendation:

1. **What the chief holds on the first day.** `staffing` as today, `direction` limited to a setting inside one workspace that a later change can undo, `operator` limited to answering a tracker decision whose recommended option can be undone, and one house class for wording fixes to shared documents that change no version. `design`, `breaking` and `release` stay with the admiral. The log then shows which approvals he gives unchanged, and those are the next grants.
2. **A day grant a seat records takes effect at once,** marked `self-entered` and shown in the next log. A standing grant takes effect only through the approved change to the grants file.
3. **A captain does not put a question to the admiral in its own chat.** It opens a request and says so. He may answer there anyway, and the captain records it.
4. **A `--blocking` request is announced through the host's notification** where the chief's host has one. Nothing else is.

## Build

0. **The rule, before any code.** The routing table above and a hand-kept grants table go into the operator's copy of the skill, so the next formation day already sends asks to the chief. The commands then replace the hand-keeping.
1. **Classes, grants and the roster in the room.** The catalog file, roster version 2 and `roster apply`, the `grant` commands, the effective list, and the mail on a change. The authority reference keyed by class.
2. **Requests.** The store, the commands and their refusals, mail on each change, `carry`. The request rule in the role files, the records reference, start and close-out.
3. **Standing grants.** The grants file and `grant load`. In the operator's repository, the thirteen dated directions move into that file.
4. **Notice.** The blocking announcement, per provider.

## Verification

| Result | Check |
| --- | --- |
| A grant changes what a seat settles | After `grant add --to chief --class direction`, `grant check` for the chief's seat exits zero; after `revoke` it is refused |
| The two fixed classes stay with the admiral | `grant add --class grant` and `--class credentials` are refused, with the house file loaded or not |
| An uncovered seat cannot settle | `request settle` by a chief with no covering entry is refused and names the holder; the request's state is unchanged |
| A captain's ask goes to the chief | `request open` from a captain seat stores the chief as holder and mails it, whatever the class |
| The admiral reads one list | With three open requests of which the chief can settle two, `request list --waiting admiral` shows one after the chief has acted |
| Decisions on his behalf are listable | `request list --settled-by <chief seat>` shows each settlement with its grant and limit; `reopen` marks one overturned and mails both seats |
| Seats learn of a change | A joined captain receives one envelope when a grant or a withhold changes its reach |
| An open request survives the close | After `request carry`, the request has the same id and holder in the new room |
| Fewer chats wait on the admiral | On the first formation day after phase 0, count the chats in which a seat ended a turn with a question for him. The day this was written the count was three |

Not verified by a check: that a holder judges a limit as the admiral meant it. The log shows the limit beside each choice so that he can see where it was read too widely. Formation also cannot tell the admiral from a session, because every seat runs as his user. A grant is a record with an audit trail and a `self-entered` mark, and carries the same trust as his relayed words do today.
