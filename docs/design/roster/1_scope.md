---
title: Formation — Roster view
owner: claude
created_date: 2026-11-27
updated_date: 2026-11-27
status: Accepted
feature: roster
doc_role: scope
summary: Two read commands list the formations in the home and show one formation's seats beside their live state, and a recorded close tells an ended formation from a running one.
stage: shipped
next: Maintain the roster commands and skill guidance together.
---

# Formation — Roster view

The operator approved this scope on 2026-11-27 ([Decided](#decided)).

## Problem

Nobody can ask Formation who is in a formation, or which formations exist.

| Observed | Cause |
| --- | --- |
| 2026-11-27: the day's roster was written six times, as comments on the chief's record: at the start, then for each seat or unit that was added | The roster is prose on a record. `formation roster check` validates a draft and stores nothing, so no command prints the line-up |
| The same day one seat joined the room without being on any roster, and a staffed sergeant never joined. Neither was visible from one place | `formation mail who` lists the seats that joined, with no rank, workspace or parent. The comment lists the seats that were planned. Nothing compares the two |
| The home directory held three rooms, one of them closed that morning. Nothing on disk told it from the running one | Close-out stops the watches and closes the notes group. The room itself records no end |

The [delegated-authority scope](../delegated-authority/1_scope.md#the-roster-in-the-room) already puts each seat's rank, workspace and parent in the room with `roster apply`. This scope adds the way to read it.

## Goal

- One command lists every formation in the home and says whether each is still open.
- One command shows a formation: each seat with its rank, workspace, model, record and parent, beside whether it is joined and reachable now.
- The planned seats and the joined seats are compared in that view.
- A formation's end is recorded in its room.

Out of scope: editing a roster from the view, a history of roster changes, a room that lives outside the home, and a pane or page. `--json` is what a pane would read later.

## Shape

```sh
formation roster list [--status active|closed] [--json]
formation roster show [<room>] [--json]
formation roster close [--room <room>]
```

The argument is a room name, as in every other Formation command. A room is named after the day's group, and the help says so. The CLI keeps one noun.

### `roster list`

```
ROOM  STATUS  OPENED            CLOSED            SEATS     WORKSPACES                LAST SEEN
g-12  active  2026-11-27 15:26  —                 6/8 held  orbit, formation, scribe  now
g-11  closed  2026-11-27 06:59  2026-11-27 15:15  0/5       orbit                     15:11Z
g-10  active  2026-11-25 16:09  —                 0/4       orbit, scribe             2 days ago
```

- **Every room under the home, newest first.** `--status` keeps one status.
- **`SEATS` is held seats over planned seats.** A room with no applied roster shows its joined seats and `no roster`.
- **`LAST SEEN` is the latest presence of any seat.** It is what shows that a room nobody closed is old, as in the third row.
- **It reads and writes nothing.** It opens each room read-only and creates no file. A room it cannot read is named on stderr and the rest are still listed.

### `roster show`

```
room g-12, active, roster applied 15:26Z by 0-chief
admiral
├─ 0-chief            chief     —          opus high     <record>  held
│  ├─ orbit-0-capt    captain   orbit      opus high     <record>  held
│  │  ├─ orbit-1-sgt  sergeant  orbit      sonnet medium —         not joined
│  │  └─ orbit-1-sentry sentry  orbit      haiku medium  —         seen 20:32Z, 2 unread
│  └─ scribe-0-capt   captain   scribe     opus high     <record>  held
├─ 0-advisor          advisor   —          fable high    <record>  held
└─ 0-aide             aide      —          haiku high    —         seen 17:29Z
not on the roster: <seat> (joined 20:12Z)
```

- **Drawn by reporting line,** from each seat's `reports_to`. A seat with none hangs from the admiral.
- **The last column is live:** `held`, or when the seat was last seen and how much mail no hook has delivered to it. It is the same presence and the same count `mail who` reports.
- **Two mismatches are named.** A planned seat that has not joined reads `not joined`. A joined seat that no roster names is listed under the tree.
- **With no argument** it shows the room the calling session is joined to, and is refused with `room_required` outside one.
- **A closed room** shows the roster as it stood at close, with each seat's last presence.
- **A room with no applied roster** prints its joined seats in one flat list and says no roster is applied.

### What the room holds

| Added | Where | Why |
| --- | --- | --- |
| `model`, `effort` and `record` on a seat, all optional | The roster format, version 2 | They are in the chief's hand-written roster today and nowhere else |
| When the roster was applied and by which seat | The room, written by `roster apply` | The first line of `show` and the `OPENED` column |
| When the room was closed and by which seat | The room, written by `roster close` | The status |

Version 2 is not in any release, so these fields join it without a third version.

### Status and close

A room is `active` until `formation roster close` records its end, and `closed` after.

- **Close is a marker.** It stops no watch and removes nothing. Mail in a closed room is unchanged.
- **A closed room refuses `roster apply`** with `room_closed`, so an ended formation's roster is not rewritten.
- **Closing twice changes nothing** and reports the first close.
- **Rooms that exist today have no marker** and list as `active`. The chief or the admiral closes each old one once.

### Skill changes

| File | Change |
| --- | --- |
| [Roster checks](../../../plugin/skills/formation/references/roster.md) | Gains `list`, `show` and `close` |
| [Startup](../../../plugin/skills/formation/references/start.md) | The chief writes one roster file, applies it, and names the room on its record. It no longer writes the roster as a comment. A seat or unit added later is an edit to the file and a second apply |
| [Close-out](../../../plugin/skills/formation/references/close-out.md) | The last step runs `formation roster close` beside the group close |

### Compatibility

- `roster check` and `roster apply` keep their behaviour. `mail who` is unchanged.
- An older room file opens: the new values are added by append-only schema steps and are empty until an apply or a close writes them.
- A room addressed by a path outside the home works with `show` and `close` and is absent from `list`.

### Passed over

| Alternative | Why not |
| --- | --- |
| One `show` for the current room only | It cannot say which formations exist, or whether yesterday's was closed |
| Derive the status from presence | A formation whose seats are idle overnight would read as ended, and an ended one with a lingering session as running |
| Read the notes store's list of groups | That store is optional, and it knows members but no rank, parent or mail state |
| A pane beside the chat | The command serves the admiral and every seat now. A pane can read its JSON later |

## Decided

The operator, 2026-11-27, approved the scope as drafted. The approval reached him as a design request through the chief and was given in the chief's chat. Two points were open questions in the draft and were approved with the recommendation:

1. **No third status for an abandoned room.** Two statuses and the `LAST SEEN` column show it, and a threshold for "abandoned" would be a guess.
2. **A closed room does not refuse mail.** A seat finishing its handoff after the close still needs its inbox. Close is a marker, and only `roster apply` is refused.

## Build

1. **`roster show`.** The three seat fields, the applied stamp, and the command with its two mismatches. It follows the task that adds `roster apply`.
2. **`roster close` and `roster list`.** The close marker, the refusal of a later apply, and the listing.
3. **The skill.** The roster reference, the startup step and the close-out step.

## Verification

| Result | Check |
| --- | --- |
| The roster is readable | After `roster apply`, `roster show` prints every seat under its parent with rank, workspace, model, effort and record; `--json` carries the same fields |
| Planned and joined are compared | A planned seat that has not joined reads `not joined`; a joined seat that no roster names is listed under the tree |
| Live state matches mail | For each seat, `held` and the unread count in `roster show` equal those in `mail who` at the same moment |
| A room with no roster still shows | `roster show` on a room with joined seats and no applied roster lists them and says no roster is applied |
| Formations are listed | With one closed and one open room in the home, `roster list` shows both, `--status closed` shows one, and the home's files are unchanged afterwards |
| The end is recorded | After `roster close`, the room lists as `closed` with the time, a second close changes nothing, and `roster apply` is refused with `room_closed` |
| The roster is written once | On the first formation day after the skill change, the chief's record names the room and carries no roster comment |

Not verified by a check: that a seat marked `held` is doing its job. Presence says a session renewed its seat within the last minute and a half, and nothing more.
