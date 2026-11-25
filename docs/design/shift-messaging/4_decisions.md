---
title: Formation — Shift messaging decisions
owner: claude
created_date: 2026-11-25
updated_date: 2026-11-25
status: Draft
feature: shift-messaging
doc_role: decisions
summary: Standing decisions for seat mail, starting with why a seat is a name and not a credential.
---

# Formation — Shift messaging decisions

## A seat is a name, not a credential

**Recorded:** 2026-11 · Daniel, in the chief's chat, to "Drop the key files and let a seat be a name?": "yes to all."

### Context

The first build proved a seat with a key file that the caller named on every command. A room serves cooperating sessions under one OS account, and each of them can read every key, so the key never stopped a session from acting as another seat. It only caught a slip.

### Decision

A seat is a name. A hook takes it from the session id the host puts in the hook's input. A command takes it from the session recorded at `mail join`, or from `--as`. Nothing in Formation grants or checks authority by seat name, and no later feature may: a sender's name on an envelope is a claim, and a message stays a request. This is how the feature meets STD-05 §R1 and §R5: the name is documented as an accident guard, and no control is relaxed because of it.

The alternative was to keep a key per seat. It would have to stay private to one session to mean anything, and one OS account cannot give it that.

### Consequences

- A seat that needs proof of who wrote to it gets that from the record the mail points at, as [the session network](../formation/references/network.md#rules) already requires.
- Cost: any process under the account can send as any seat and read any mailbox, and Formation cannot tell. A room must never be shared across accounts or machines on this design.

## Remove `--identity` and `FORMATION_IDENTITY` without a deprecation window

**Recorded:** 2026-11 · Chief's call on Daniel's approved design dropping key files; deviates from `STD-01@4 §R35`.

### Context

`STD-01@4 §R35` keeps a removed flag or configuration key accepted for one release as an ignored no-op that warns by name, and records the removal in the changelog. Dropping key files retires both `--identity` and `FORMATION_IDENTITY`. Formation has no release: no tag, no GitHub release, and `CHANGELOG.md` holds an empty Unreleased section. Its only callers are this repository's own skill references, which change in the same commit.

### Decision

Remove `--identity` and `FORMATION_IDENTITY` outright with no no-op release window. Record the removal in the pull request description rather than editing `CHANGELOG.md`, which `STD-09 §R20` compiles at release time from merged work.

The alternative was to keep both spellings accepted as warning no-ops until a first release. That would keep dead credential-file surface before any release existed.

### Consequences

- No deprecated key-file flag or environment variable remains in the CLI, core composition, store or skill references.
- Cost: afterwards `--identity` is rejected as a usage error by the argument parser (exit 2), and any `FORMATION_IDENTITY` left in an environment is ignored.
