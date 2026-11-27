---
title: Formation — Seat folder
owner: claude
created_date: 2026-11-27
updated_date: 2026-11-27
status: Draft
feature: seat-folder
doc_role: scope
summary: Each seat gets a folder inside its room for the files it keeps between turns, created and reported by `mail join`.
stage: building
next: Build the folder creation and the `seat_dir` field in `mail join`, then point the briefs at it.
---

# Formation — Seat folder

Daniel chose the location on 2026-11-27 ([Decided](#decided)). Creating and reporting the folder from `mail join` is proposed here and not yet reviewed.

## Problem

A seat keeps a few files between turns: the probe scripts its watches run, the asks it is holding for one batched message, its mute list and a log. The briefs call the place `{scratch}` and say nothing about where it is. In practice the captain filled it with a folder under the checkout's scratch directory, `tmp/<group>/<seat>/`.

That directory is declared scratch: the umbrella ignores it and anyone may clear it. These files are not scratch. A watch stores the command line of its probe, and the reader runs that script every interval for as long as the room has a reader.

| Observed | Cause |
| --- | --- |
| 2026-11-27: the operator cleared the scratch directory during a shift. A sergeant's six watches lost their probe scripts, and its held and mute files were gone. It rebuilt them from its own transcript | Live seat state was stored where scratch is stored |
| 2026-11-21: a captain's context was cleared. Its wake loop and the script it ran were gone with the old session scratchpad, and the captain re-created them from its handoff | The same, with the session scratchpad as the place |

Formation already keeps a directory per room, `<home>/rooms/<room>/`, holding the inbox and the watch set. The probes that the watch set names lived outside it.

## Goal

- A seat's files are in a place that clearing scratch, cleaning a checkout or refreshing the session does not touch.
- Every seat finds that place the same way, whichever provider it runs on and however the room was selected.
- The files live exactly as long as the room's mail and watches do.

Out of scope: a schema for what a seat stores, a command to list or clean seat folders, more than one machine, and moving any of this into the room's database. The files stay plain files the seat writes with its own tools.

## Shape

| Part | What it is |
| --- | --- |
| Seat folder | `<room directory>/seats/<seat>/`. For a named room that is `<home>/rooms/<room>/seats/<seat>/` |
| Creation | `formation mail join` creates it |
| Report | `mail join` prints its absolute path as `seat_dir` |
| Ownership | The seat writes there. Formation never reads, lists or deletes what is inside |

### Where it is

The folder is a child of the room directory that composition already resolves, so it follows every way a room can be selected: a name under `<home>/rooms/`, a path given to `--room`, `FORMATION_ROOM`, or the room recorded for the session. `<home>` is `FORMATION_HOME`, else `~/.formation`.

The folder is named for the seat. A seat name is already validated as an identifier at join, so it is one path component and cannot leave `seats/`. Join refuses a name that would, before it creates anything.

`seats/` is reserved in a room directory. Nothing else in Formation writes under it.

### Creation and report

`mail join` is the one command every seat runs before anything else, and it is already a write. Before it binds the seat, it creates `seats/<seat>/` with the room directory's own mode (owner only) if it is missing, and adds one field to its output:

```json
{"seat":"orbit-1-sgt","rebound":false,"replaced_session":null,"seat_dir":"/Users/op/.formation/rooms/g-6/seats/orbit-1-sgt"}
```

- **Additive.** Every existing field of the join output stays as it is.
- **Idempotent.** Joining again as the same seat from the same session changes nothing in the room and prints the same path, so a seat that lost the path after a refresh reads it by joining again.
- **A failure to create the folder fails the join** with `seat_dir_unavailable` and the path. A seat that believes it has a folder and does not would lose the files it writes next.
- **A failed join changes nothing.** The folder is checked before the seat is bound, and the binding and the session pointer (`<home>/sessions/<session id>`, written when the join runs in a provider session) commit together. A join that exits nonzero, at the folder, the pointer or the commit, leaves no new seat, no new or changed pointer and, for a seat that already existed, the same owner, route and pending mail; a pointer that cannot be restored, below, is the one exception. A seat is never listed or sendable while its caller has no route. Joining again once the cause is removed is the whole recovery. If the binding cannot commit after the pointer was written, the previous pointer is put back.
- **One join per session at a time.** A join from a session takes an advisory lock on `<home>/sessions/<session id>.lock` before it reads the previous pointer, and holds it until the join has committed or the pointer is back. A failing join in one room therefore cannot restore a stale pointer over a later join of the same session in another. The lock is taken before the room database's write lock. The wait is bounded at 15 seconds, longer than the database's, and a timeout names the holder. The lock file is never deleted.
- **A route that cannot be protected or restored is reported.** A previous pointer that is not a private regular file within the size cap cannot be put back, so the join refuses before it binds. If restoring fails after the join failed, the join reports `session_route_unrestored` with the pointer's path and both failures. A process killed between the pointer write and the commit restores nothing; joining again repairs the pointer.
- **The folder is checked as the room directory is.** Join refuses a `seats/` or seat folder that is a symlink, belongs to another user or is open to group or other, with `seat_dir_unavailable`. It never changes an existing folder or its contents.
- **A legacy folder is told how to be fixed.** A folder the caller owns with a group or other bit set, such as mode 0755 from before folders were private, is refused with the exact path, the observed mode, the required mode 0700 and the instruction to set mode 0700 and join again. Formation does not change the mode itself: the owner decides whether the contents were meant to be open.
- **No new command.** A read-only command may not create the folder, and a seat needs the path once, at the moment it joins.

### What the seat does with it

The briefs' `{scratch}` becomes the seat folder: probe scripts, `held`, `mute`, `seen` and `log` go there. A watch still stores an absolute command path, so a probe kept in the seat folder is added as `formation watch add <name> -- "<seat_dir>/<probe>.sh"`.

Real scratch stays where it was: logs of a one-off command, JSON inputs, patches. Those may be cleared at any time.

### Lifetime

Nothing in Formation deletes a room directory today, and this adds no deletion. A seat folder is removed when the operator removes the room. A session that replaces a seat joins under the same seat name and finds its predecessor's files.

### Compatibility

- A room created before this change gets each seat's folder at that seat's next join.
- A seat that kept files elsewhere moves them and re-adds its watches with the new paths. `watch add` with the same name and recipient replaces the watch and keeps its remembered fingerprints.

### Passed over

| Alternative | Why not |
| --- | --- |
| A fixed path spelled out in the briefs | It is wrong when the room is a path, comes from `FORMATION_ROOM` or the session file, or `FORMATION_HOME` is set. Composition already knows the answer |
| A new `formation seat` noun with a command that prints the path | The command would have to create the folder to be useful, and a read-only command never writes. `mail join` is already the write every seat makes |
| A gitignored folder in the checkout | A checkout clean or a second checkout of the same repository loses or splits it, and it has a different lifetime from the room |
| Store held and mute lines in the room's database | It needs a schema and commands for files a seat edits freely. It returns if two seats are seen needing to read one seat's list |

## Decided

Daniel, 2026-11-27 (UTC), in the chief's chat, to a proposal of `~/.formation/rooms/<group>/seats/<seat>/` against a gitignored folder in the checkout: "go with ~/.formation".

1. **Seat files live in the room directory,** under `seats/<seat>/`.

Open, proposed above: `mail join` creates the folder and reports it as `seat_dir`.

Corrected 2026-11-27. The first build bound the seat before it checked the folder, so a refused join left a visible seat with no session pointer. A pre-created 0755 folder at a Codex captain's join produced that state: the seat answered to `--room` and `--as` while its own session had no route and its hook stayed silent. Join now checks the folder first and commits the binding and the pointer together.

Review of that correction found two gaps, closed the same day. A join whose commit failed restored the pointer it had read earlier over a successful join of the same session in another room; the session lock closes that. A route check run with `FORMATION_ROOM` set passed for a session with no pointer; the startup check now unsets it.

## Build

1. **Join.** Create the folder before the join commits, bind the seat and write the session pointer in one step, and add `seat_dir` to the join output.
2. **Plugin.** The briefs and the run reference define `{scratch}` as the seat folder; the usage page describes it.

## Verification

Each through the built executable, as the mail tests are today, except the two rows marked as core tests.

| Result | Check |
| --- | --- |
| A first join creates the folder and reports it | Join a new seat in a named room; the output's `seat_dir` exists, is a directory and is `<home>/rooms/<room>/seats/<seat>` |
| A second join changes nothing | Join again from the same session; same `seat_dir`, and a file written there between the joins is still present |
| The folder follows the room | Join with `--room <path>` and with `FORMATION_HOME` set; `seat_dir` is under that room directory each time |
| The folder is private | On Unix, its mode grants nothing to group or other |
| A folder that cannot be created fails the join | Put a regular file at `seats`; join exits 1 with one `seat_dir_unavailable` JSON error line naming the path |
| A legacy folder refuses with its fix | Pre-create the folder at 0755 with a file in it; join exits 1 with `seat_dir_unavailable` naming the path, `0755`, `0700` and the `chmod`; the seat is not listed, no pointer exists, and the folder's mode and contents are unchanged. Set it to 0700 and join again: the join succeeds |
| A failed pointer write leaves no seat | Join once from the session elsewhere, make `<home>/sessions/` read-only and join a new seat; join exits 1, the pointer still names the earlier room, `mail who` does not list the seat and a send to it is refused. Restore the mode and join again: the seat and the pointer exist |
| An unreadable pointer refuses first | Put a directory where the session's pointer file goes; join exits 1 and `mail who` does not list the seat. Remove it and join again |
| A failed commit puts the route back | A deferred foreign key fails the commit after the pointer is written; the pointer is absent or still names the earlier room, and no seat appears |
| Concurrent joins keep the winner's route | Hold one room's write lock, start a join there that will fail to commit, then join another room from the same session. After release the first join fails, the second succeeds and the pointer names the second room |
| A restore failure is visible (core test) | Make the pointer unremovable and abandon the join's claim; the error is `session_route_unrestored` and carries the pointer's path and the join's failure |
| An unfinished claim restores (core test) | Drop a claim that recorded a new room; the earlier pointer is back |
| The startup check needs the pointer | With the pointer moved aside, `mail who` succeeds with `FORMATION_ROOM` set and `env -u FORMATION_ROOM formation mail who` refuses with `room_required` |
| A failed rebind keeps the owner | Make the second session's join fail at the pointer write or at a 0755 folder; the first session still owns the seat, both sessions' pointers are unchanged and the owner's pending mail reads from it. After the fix the rebind reports `replaced_session` |
| A retried join gives a working hook | After a refused join and a corrected retry in a Codex session, a hook for that session delivers queued mail with no `--room` or `--as` |
| Headless join is unchanged | A join with no session id succeeds and writes no pointer |
| Unsafe folders stay refused | A symlinked folder and a folder open to group (0770) each refuse with `seat_dir_unavailable` and change nothing |
| Existing output is unchanged | The first-join test asserts `seat`, `rebound` and `replaced_session` beside the new field. No golden captures join output |

Not verified by tests: the 15-second lock timeout, which rests on the shared kit's lock; a join killed between the pointer write and the commit; the refusal of a foreign-owned folder, which rests on the shared kit's directory check and needs a second user; a non-UTF-8 room path, which join refuses because `seat_dir` is a JSON string; and that every brief's use of `{scratch}` reads correctly with the new meaning. That is read once when the plugin text changes.
