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

`mail join` is the one command every seat runs before anything else, and it is already a write. After the join commits, it creates `seats/<seat>/` with the room directory's own mode (owner only) if it is missing, and adds one field to its output:

```json
{"seat":"orbit-1-sgt","rebound":false,"replaced_session":null,"seat_dir":"/Users/op/.formation/rooms/g-6/seats/orbit-1-sgt"}
```

- **Additive.** Every existing field of the join output stays as it is.
- **Idempotent.** Joining again as the same seat from the same session changes nothing in the room and prints the same path, so a seat that lost the path after a refresh reads it by joining again.
- **A failure to create the folder fails the join** with a typed error naming the path. A seat that believes it has a folder and does not would lose the files it writes next.
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

## Build

1. **Join.** Create the folder after the join commits and add `seat_dir` to the join output.
2. **Plugin.** The briefs and the run reference define `{scratch}` as the seat folder; the usage page describes it.

## Verification

Each through the built executable, as the mail tests are today.

| Result | Check |
| --- | --- |
| A first join creates the folder and reports it | Join a new seat in a named room; the output's `seat_dir` exists, is a directory and is `<home>/rooms/<room>/seats/<seat>` |
| A second join changes nothing | Join again from the same session; same `seat_dir`, and a file written there between the joins is still present |
| The folder follows the room | Join with `--room <path>` and with `FORMATION_HOME` set; `seat_dir` is under that room directory each time |
| The folder is private | On Unix, its mode grants nothing to group or other |
| A folder that cannot be created fails the join | Put a regular file at `seats`; join exits 1 with one JSON error line naming the path |
| Existing output is unchanged | The join goldens differ only by the added field |

Not verified by tests: that every brief's use of `{scratch}` reads correctly with the new meaning. That is read once when the plugin text changes.
