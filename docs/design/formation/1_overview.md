---
title: Formation overview
owner: codex
created_date: 2026-11-15
updated_date: 2026-11-24
status: Draft
feature: formation
doc_role: overview
stage: building
next: Qualify delivery to the original desktop chat on each receiving host.
summary: How an operator staffs a shift across Claude and Codex, and what the local inbox delivers today.
---

# Formation overview

Formation lets an operator run several workspaces through a small set of familiar chats. The chief coordinates the shift, the aide keeps the operator briefed, and a captain owns each workspace. A captain can work with headless agents while remaining directly available to the operator.

For example, the skill order `formation squad orbit, solo scribe` asks for one chief and aide, a staffed Orbit unit and one Scribe captain. The chief records who actually started. The Orbit sentry reports a red check to the sergeant, who handles routine repairs and takes decisions outside its mandate to the captain. The Scribe captain can cover a quiet workspace on its own wakes. The [unit definitions](2_design.md#units) give the staffing and attention owner for each lineup.

## What ships

- **One plugin skill for Claude and Codex.** It supplies the shift procedure, role briefs and provider-specific instructions for staffing, reporting, handoff and close. The receiving host supplies the session tools. [Packaged skill](../../../plugin/skills/formation/SKILL.md)
- **A Rust CLI with two jobs.** `formation roster check` validates a proposed lineup and directed routes offline. `formation mail` lets cooperating local seats join a room, list online peers, send, read, wait and acknowledge. [CLI usage](../../usage.md)
- **Integrations for a Constellation shift.** Orbit holds assignments, decisions and attention. Scribe holds group membership and opted-in session history. An operator can use the local inbox independently of either integration. [Shift startup](../../../plugin/skills/formation/references/start.md)

## Delivery today

A Claude Desktop Code chief sent a message through the local inbox and a Codex Desktop sentry read it. The two attempts to start a new turn in the idle Codex captain produced no observed parent turn. The reverse cross-provider direction is still untested. The [shift-messaging design](../shift-messaging/1_scope.md#local-sentry-trial-2026-11-24) owns the trial record and acceptance criteria.

The [current design](2_design.md) describes how a shift runs. The [vision](3_vision.md) describes possible extensions. The [decisions](4_decisions.md) explain why the project took this shape. The original eight-formation design and its evolution remain in the [merged design snapshot](https://github.com/constellation-works/formation/tree/520d0d03181bfb68ea15347093f2a4b831c6c298/docs/design/formation).
