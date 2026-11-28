---
title: Formation — Roster view decisions
owner: claude
created_date: 2026-11-27
updated_date: 2026-11-27
status: Draft
feature: roster
doc_role: decisions
summary: The roster close syntax, list output and command layout.
---

# Formation — Roster view decisions

## Closing a roster records a marker

**Code anchors:** `crates/formation/src/command/roster/close.rs::Close`

The [approved close syntax](1_scope.md#status-and-close) runs from a joined
session without a confirmation flag. This is a narrow exception to
`STD-01@4 §R5` because the stamp is permanent and prevents future roster
applies. Close keeps room records, mail and watches available, and a repeated
call reports the first stamp.

Cost: A joined seat can close a room in one command. Requiring `--confirm`
would add a separate acknowledgment before the marker is recorded.

## The list prints a headed table

**Code anchors:** `crates/formation/src/output/roster.rs::write_list`

The [approved list view](1_scope.md#roster-list) uses column headings so a
reader can distinguish opening time, closure and presence. The headings also
appear when piped, a deviation from `STD-01@4 §R9`. Scripts can use the JSON
array. Empty matches use the regular list empty state.

Cost: Piped text contains the header and space-aligned columns. The
headerless tab-separated alternative would match the standard's pipe default.

## The roster moves to a command directory with its fourth verb

**Code anchors:** `crates/formation/src/command/roster/mod.rs::Action`

The Orbit delivery pipeline commits the candidate as one change. The move to
one file per roster verb accompanies the new verbs, a sequencing deviation
from the move-only precursor in `STD-02@10 §R49`.

Cost: The mechanical move and behavior changes share a review. A standalone
move would need a separate delivery.
