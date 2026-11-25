---
title: Formation decisions
owner: codex
created_date: 2026-11-24
updated_date: 2026-11-24
status: Draft
feature: formation
doc_role: decisions
summary: Decisions governing unit staffing, records, provider integration, inbox receipts and repository architecture.
---

# Formation decisions

## Workspace lineups are composed from units

### Context

The original eight named formations bundled a whole shift into one choice. Two
workspaces on the same shift could need different levels of coverage.

### Decision

Staff the chief and aide once. Choose a unit for each workspace, then name
any reinforcement separately. The [unit definitions](2_design.md#units) and [reinforcement procedure](../../../plugin/skills/formation/references/run.md) govern staffing.

### Consequences

A captain can request another seat as the workspace load changes.
Cost: the chief has to choose and communicate a lineup for each workspace.

## One shift skill with separate provider procedures

The original skill assumed Claude desktop session tools. Operators now staff Claude and Codex seats in the same shift. Both need the same unit definitions, authority and handoff rules. Each host has different ways to start a child, address a peer and retire a session.

Keep one installable skill. Put host-specific mechanics in the Claude and Codex references and record what the host actually started. Separate provider-specific skills would duplicate the staffing rules and let them drift. Cost: every change to a shared procedure needs review against both hosts. [Skill port](https://github.com/constellation-works/formation/commit/6a01c32) · [Skill entry point](../../../plugin/skills/formation/SKILL.md)

## Formation owns mail and Scribe owns group history

A shift needs both a place to deliver work to a seat and a way to recover the context of the people and sessions involved. Extending Scribe groups into inboxes would tie message delivery to the history system. Keep seat presence and mail in the standalone Formation room. Use Scribe for group membership, handoffs and opted-in history, and Orbit for operational assignments where configured.

Cost: the chief must record the relationship among the host session address, group seat and assignment. A group lookup alone cannot identify a live recipient. [Room contract](../../usage.md#local-inbox) · [Roster procedure](../../../plugin/skills/formation/references/start.md)

## A recipient acknowledges mail after handling it

The early portable-kit proposal would mark a message read when it was fetched. In Formation, a sentry can fetch mail for its parent and the parent may still be idle. Consuming the envelope during that read would lose the captain’s pending work.

Keep reads repeatable. Let the responsible recipient acknowledge the receipt explicitly. A sender retry reuses its ID and original content and receives the original receipt. Cost: an interrupted handoff can present the same pending envelope again. Recipients must track which receipt they handled. [Portable alternative](references/portable.md#the-mailbox) · [CLI integration tests](../../../crates/formation/tests/mail.rs)

## A wake claim requires a turn in the original chat

Subagent completion is convenient evidence for a sender, but the Codex trial recorded that event while the captain remained on its setup turn. Qualify each receiving host, build and path separately. Claim idle wake only after observing a subsequent turn in that previously idle chat before user input. Cost: a cross-provider qualification needs access to the receiving chat’s turn evidence in both directions. [Two Codex parent controls](../shift-messaging/1_scope.md#local-sentry-trial-2026-11-24)

## Use the Rust CLI template from the first command

The first inbox was small enough to implement directly in a command handler. Formation adopted the shared template so new commands follow an existing path through the CLI, core app and store. Keep authorization, retries and outcomes in the core app and keep database access in the store.

Cost: even a small operation crosses several modules, and a change to its public output may require a golden update. The dependency gate checks the boundaries. [Template alignment](https://github.com/constellation-works/formation/commit/b184ee3) · [Message path](../../../ARCHITECTURE.md#follow-a-message)
