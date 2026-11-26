---
title: Formation — Watch set
owner: claude
created_date: 2026-11-26
updated_date: 2026-11-26
status: Draft
feature: watch
doc_role: scope
summary: A watch set kept in the room that seats change while it runs, one reader with standbys, and a command that lists what is being watched.
stage: idea
next: Daniel reviews the shape and answers the three questions.
---

# Formation — Watch set

This is a proposal. Nothing in it is built, and it waits for Daniel ([Questions for Daniel](#questions-for-daniel)).

## Problem

`formation watch` already runs any command as a probe and mails a seat what is new ([shift messaging](../shift-messaging/1_scope.md#watch), [usage](../../usage.md#watching)). The plugin ships two probes: Orbit attention for a workspace, and red CI on a repository's base branch and open pull requests. A seat can write a script and watch it today.

Daniel, 2026-11-26 (UTC), in the designer's chat: "formation watch direly needs dynamic watch capability. right now, it can only watch mailbox, correct? for the squad to be effective, they would able to watch change detections in orbit.attention, red CI checks of PR's in selected repo, etc. it should be dynamic enough such that agent can write up a script and watch over a set of things."

The probes he asks for exist, and he could not tell. The formation of that day showed why:

| Observed on 2026-11-26 | Cause |
| --- | --- |
| Daniel took `watch` for a mailbox reader | No command shows what a room watches. The room stores what was sent and never which watches exist |
| One watch process was running for three captains, started by a captain for itself | The plugin tells only a sentry to start one, and no sentry had joined the room. Nobody could see the gap |
| The one running watch was started with `--timeout 3600` | When it ends, nothing restarts it and no seat is told |

Reading the code adds four limits. No incident has shown them yet.

1. **The probes are fixed at launch.** They are arguments of the process, so adding or dropping one means stopping and restarting it.
2. **One interval and one recipient** serve every probe of a process.
3. **A fingerprint is remembered for the life of the room.** Orbit hashes an attention item's condition state into its fingerprint. A condition that clears and returns in the same state is never sent again.
4. **A trial run consumes what it finds.** Running a new probe once without `--to` hands its items over, so the real watch stays silent about them.

## Goal

- A seat adds, changes or drops a watch with one command while every other watch keeps running.
- One command shows what the room watches, when each watch was last read, and whether anything is reading.
- A reader that stops is noticed by the seats that depend on it within one of their turns.

Out of scope: more than one machine, rules on who may add a watch (a room admits cooperating seats under one OS account), a model inside the watch, and new probes. A new thing to watch stays a new script.

## Shape

| Part | What it is |
| --- | --- |
| Watch set | The room's list of watches. Each is a name, a command, a recipient and an interval |
| Reader | One process that runs whichever watches are due and mails what is new. A second one waits as standby |
| List | One command that prints the set, each watch's last read and the reader's state |
| Check | A dry run of a probe that hands nothing over |

### The watch set

The set lives in the room's store beside the mailboxes. A watch is identified by its recipient and its name, so two captains can each have a `ci`.

| Field | Meaning |
| --- | --- |
| name | 1 to 32 lowercase letters, digits or hyphens, as today |
| to | The joined seat that is mailed. Defaults to the seat that adds the watch |
| command | Run through `sh -c`, in the working directory the watch was added from |
| every | Seconds between reads. Default 120, minimum 10 |
| state | `on` or `paused` |
| added by, added at | The seat that set it, and when |
| last read, last result | Written by the reader after each read |

```sh
formation watch add attention -- "$formation_skill/scripts/attention-read.sh" <workspace selector>
formation watch add ci --every 300 -- "$formation_skill/scripts/ci-read.sh" <owner/name> <base branch>
formation watch add release --to <seat> --every 60 -- ./scripts/release-read.sh
formation watch pause ci
formation watch resume ci
formation watch remove release --to <seat>
```

`add` with a name that exists replaces that watch and keeps its remembered fingerprints. `remove` forgets them. A change takes effect at the reader's next pass, at most ten seconds later, and nothing restarts.

### The reader

`formation watch run` is the only process that executes probes for a room.

- **It runs what is due.** Each pass it reads the set, runs every `on` watch whose interval has passed, and mails new items to that watch's recipient. Probes run side by side with the 60-second limit they have today.
- **It holds a lease.** The reader records its seat, process id and a beat every 15 seconds. A second `formation watch run` finds a live lease and waits as a standby.
- **A standby takes over.** When the beat is older than 45 seconds, a standby takes the lease in one transaction and reads on. The remembered fingerprints are in the room, so the change of reader sends nothing twice.
- **It stops on purpose.** `formation watch stop` leaves a mark in the room. The reader and its standbys exit at their next beat, and `run` clears the mark.

Each captain starts `formation watch run` in the background at the start of a shift. The first one reads and the others stand by, so the set is read for as long as any one of them is alive.

A probe runs in the reader's environment, which may be another seat's shell. A probe that needs a tool or a token the reader lacks fails there, and that shows as `read failed` in the list and in the recipient's mail.

### Seeing what is watched

`formation watch list` prints the reader's state and one row per watch. `--json` gives the same as a document.

```
reader: orbit-0-capt, pid 4711, last beat 4 s ago; standby: scribe-0-capt
NAME       TO                EVERY  STATE   LAST READ  RESULT       SENT  ADDED BY
attention  orbit-0-capt      120 s  on      38 s ago   ok, 3 items  5     orbit-0-capt
ci         orbit-0-capt      300 s  on      2 min ago  ok, 0 items  2     orbit-0-capt
ci         scribe-0-capt     300 s  on      2 min ago  read failed  0     scribe-0-capt
release    formation-0-capt  60 s   paused  2 h ago    ok, 0 items  1     0-chief
```

With no live lease the first line reads `reader: none since <time>`.

### A reader that stops

A reader can end without a `stop`: its session closes, it is killed, or the machine sleeps. The hooks already run for every joined seat at each prompt and each stop. When the set has an `on` watch, no lease is live and no stop mark is set, the hook hands one line to each seat that receives or added an `on` watch, by the path mail takes:

```
watch: nothing has read this room's 4 watches since 03:40Z. Start a reader: formation watch run
```

Each seat gets the line once per stoppage. A seat that neither receives nor added a watch gets nothing.

### Writing a probe

The contract stays: one line per item as `<fingerprint><TAB><text>`, exit 0 with no output for nothing to report, and a non-zero exit for a failed read. One rule is added under [A condition that returns](#a-condition-that-returns): a probe prints every item of the condition it reports, on every read.

`formation watch check -- <command>` runs the command once under the reader's limits and prints the parsed items, or the line that broke the contract. It hands nothing over and mails nobody, so a seat can test a script before adding it.

### A condition that returns

Today a fingerprint that was sent once is never sent again. The proposal: a fingerprint missing from two successful reads in a row is forgotten, and if the item comes back it is mailed as new. A failed read counts for nothing, and an item that flickers for a single read stays quiet.

The cost falls on probes that list only part of a condition, because an unlisted item looks cleared. The shipped attention probe reads the first 20 items, so it changes to read them all.

### Watch mail is data

A watch's mail carries text from outside the formation: a check's name, an attention item's reason. In a public repository the author of a pull request chooses the check names. The hooks hand all mail today under one heading that calls it a request from another session. Mail from a watch gets its own heading, which names the watch and the seat that added it and says the lines are probe output and no seat's request.

### The blocking form

`formation watch --probe <name>=<command>` without `--to` blocks until something is new and returns it to its caller. That remains as `formation watch wait`, for a caller outside a formation or one that wants the result in its own turn. The `--to` form is replaced by the set and the reader. Formation has no tagged release, so the old spelling is removed and not kept beside the new one.

### The plugin

- **Start.** A captain that owns its workspace's attention adds the unit's two watches and starts a reader. The chief adds its own probes when the [seat network](../formation/references/network.md#watch-and-dispatch) design settles their recipient.
- **Any seat, any time.** The skill says a seat may add a watch for something it waits on and removes it when it is done.
- **Orders.** A sentry's `rest` and `watch` orders become `pause` and `resume` on the unit's watches.
- **Close.** The close-out runs `formation watch stop`.

### Passed over

| Alternative | Why not |
| --- | --- |
| Keep one process per seat and let it reread its probes | Each seat must keep its own process alive, and each death is silent. It fixes the first limit only |
| The hook restarts a dead reader itself | A process spawned from a hook has the app's environment, and nothing yet shows that `gh` and `orbit` probes run there or that the process outlives the hook. [Question 2](#questions-for-daniel) |
| An OS service per host | Installing one is the operator's act on each machine. Standbys cover a reader's death without it |
| A file that declares the set, loaded at start | A captain's set is two `add` lines in its brief. It returns when a set outgrows that |
| Mute for single items in the set | Fingerprints already stop repeats, and `pause` covers a whole watch. The sentry keeps its mute file |
| Mail when an item clears | It doubles the mail, and the seat working the item sees it clear. It returns if a seat is seen working an item that had already cleared |

## Questions for Daniel

1. Is this the shape: the set in the room, one reader with standbys, and `watch list`? Recommended: yes.
2. When no reader is left, should the hooks tell the affected seats, or start a reader themselves? Recommended: tell them in the first build, and trial a hook-started reader afterwards.
3. Should a cleared fingerprint be forgotten after two clean reads, so that a returning condition is mailed again? Recommended: yes, with the attention probe changed to read every item.

## Build

One task per item, filed after Daniel answers.

1. **The set.** The store's watch table and `add`, `remove`, `pause`, `resume`, `list`.
2. **The reader.** `run`, `stop`, the lease, standby and per-watch intervals. The blocking form moves to `wait`.
3. **Check.** `watch check`.
4. **Hooks.** The stopped-reader line, and the heading for watch mail.
5. **Returning conditions,** if question 3 is yes: forgetting, and the attention probe reads every item.
6. **Plugin.** Start, the briefs, close-out and the usage page.

## Verification

Each through the built executable, as the watch tests are today.

| Result | Check |
| --- | --- |
| A watch added while the reader runs is read with no restart | Add one with a probe that prints an item. Its mail arrives within the interval plus ten seconds, from the same reader process |
| A removed or paused watch is not read | Its probe leaves a file on each run. No new file appears after the change |
| One reader at a time, with takeover | Start two. One holds the lease. Kill it, and the other is reading within 60 seconds with nothing mailed twice |
| A deliberate stop is silent | After `stop`, both exit and no seat receives the stopped-reader line |
| A dead reader is noticed once | Kill every reader. The next hook event of a seat with an `on` watch carries the line, and its second does not. A seat with no watch gets nothing |
| `check` hands nothing over | Run `check`, then the reader. The reader still mails the items |
| A returning condition is mailed again | Present, absent for two reads, present: mailed twice. Absent for one read: mailed once. Failed reads between do not count |
| `list` reads true | A golden for the table and one for `--json`, with a live reader, a standby and no reader |

Then one formation day: every captain's watches appear in `watch list` at the start and at the close, and the day's stopped-reader lines are counted.

Unresolved risks:

- **One environment for all probes.** A probe that passes `check` in its author's shell can fail in the reader's. The failure is visible, and the fix is the seat's own reader or a probe that finds its tools itself.
- **A stored command runs later in another process.** Any process that can write the room can have the reader run a command. Today's room has the same boundary: every seat already runs a shell under the one OS account.
- **A reader in a session's background shell lives as long as that shell.** How long each provider keeps one alive after a turn is not measured. The standbys and the stopped-reader line are the cover, and the formation day measures how often they are needed.
