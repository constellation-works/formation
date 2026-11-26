---
title: Formation — Watch decisions
owner: claude
created_date: 2026-11-26
updated_date: 2026-11-26
status: Draft
feature: watch
doc_role: decisions
summary: Where the watch set's list output departs from the CLI standard's piped form.
---

# Formation — Watch decisions

## The list prints one human form

**Recorded:** 2026-11

### Context

`STD-01@4 §R9` asks a piped command for tab-separated records with no header, and §R12 keeps everything but the payload off stdout. The [watch scope](1_scope.md#seeing-what-is-watched) approves a `watch list` that opens with the reader's state and then a headed table, because the reader line is what tells a seat that nothing is reading its watches. A pipeline that dropped it would hide the failure `list` exists to show.

### Decision

`formation watch list` prints the reader line and the headed table whether or not stdout is a terminal. Cells never truncate and carry no escape codes. `--json` carries the same fields, with the reader state as an object. An empty set prints only the reader line, and the empty-state notice goes to stderr.

### Consequences

- A script that reads the rows uses `--json`, which is the stable contract (`STD-01@4 §R10`).
- Cost: `watch list | cut -f1` does not work; the text form is for people.
