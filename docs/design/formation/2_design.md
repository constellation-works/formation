---
title: How Formation runs a shift
owner: codex
created_date: 2026-11-16
updated_date: 2026-11-24
status: Draft
feature: formation
doc_role: design
summary: Workspace staffing, authority, provider session mechanics, local mail and shift continuity.
---

# How Formation runs a shift

## Units

The operator chooses a lineup per workspace and can resize it during a shift. The captain and its assignment carry over. Staffing requests are interpreted by the skill. The CLI accepts a JSON roster for offline validation. [Unit definitions](../../../plugin/skills/formation/assets/units.yaml) · [Example roster](../../../examples/roster.json)

| Unit | Seats under the captain | Who watches attention |
| --- | --- | --- |
| Solo | None | An active resident, otherwise the captain on its own wakes |
| Duo | Sentry | Sentry reports to captain |
| Trio | Sergeant and sentry | Sentry reports to sergeant |
| Squad | Sergeant, sentry and a lieutenant on the first heavy order | Sentry reports to sergeant |

## Chain of command

The chief owns the roster and coordinates captains. The aide briefs the operator and can relay their attributed instructions. A captain owns one workspace assignment, briefs its unit and may land reviewed work within its shipping mandate. The sergeant clears routine findings. The lieutenant takes one heavy order at a time in its own worktree. The sentry observes, deduplicates and reports. Only one seat or resident owns a workspace's attention watch at a time. [Authority](../../../plugin/skills/formation/references/authority.md) · [Running a shift](../../../plugin/skills/formation/references/run.md)

A relay carries the source of an order. The recipient checks the operator’s direction against the assignment when authority matters. The chief advises on shipping decisions and can approve staffing within the standing reinforcement limit of a squad plus one additional lieutenant per workspace. The operator decides beyond that limit and retains decisions about major scope, security, release and grants.

### Specialists

Only the operator adds a standing specialist. A specialist answers a recurring subject question from the chief, a captain or a lieutenant given its address for that order. It sends its answer and evidence back to the asker, who owns the action. [Specialist brief](../../../plugin/skills/formation/assets/brief-specialist.md)

## Staffing on each provider

Claude Desktop Code can start and address seats when its session tools are available. The skill briefs seats before detaching them and keeps a lieutenant attached for its captain to receive the result. Model and effort choices respect the host's parent limits. [Claude procedure](../../../plugin/skills/formation/references/provider-claude.md)

Codex uses the operator’s existing desktop chats for the chief, aide and captains when assigned there. A captain can spawn a collaboration subagent for a bounded order or an inbox watch. The roster records the actual host address, parent, model and effort. Upon child completion, the parent can consume the result in an active turn. Reaching an idle parent has a separate evidence requirement. [Codex procedure](../../../plugin/skills/formation/references/provider-codex.md) · [Host observations](references/network.md)

## Local mail

A shift room is a private directory on one machine. Joining associates a seat name with a local identity file. A sender names the recipient, supplies a message ID and receives a durable receipt. A new send requires an online recipient. Retrying an accepted send with the same sender, ID, recipient and body returns the original receipt even after the recipient goes offline. Reusing the ID with different content fails. [Send implementation](../../../crates/formation-core/src/app/mail/send.rs)

A sentry can wait on the assigned seat's inbox in one bounded command. The command polls SQLite locally and renews presence while it waits. An empty result means the wait expired. Reading leaves the envelope pending, so a sentry can report it and the responsible recipient can acknowledge it after handling. The CLI handles presentation, the core app checks identity and replay, and the store owns SQLite transactions and migrations. [Inbox behavior](../../usage.md#local-inbox) · [Architecture](../../../ARCHITECTURE.md)

## Shift records and handoff

At startup the chief reads the previous handoff, checks resident coverage and records the actual roster on the shift assignment. Each captain claims its workspace assignment and records outcomes there. In Constellation, seats use Orbit MCP first and fall back to the owner’s CLI over SSH when the operation or owner is unavailable through MCP. Scribe links members and handoffs where configured. Capture requires the operator’s opt-in. [Startup](../../../plugin/skills/formation/references/start.md) · [Records](../../../plugin/skills/formation/references/records.md)

A replacement captain reads the assignment in observe-only mode until its predecessor stands down. The new captain checks its routes and resumes the watch once. At close the captain stops the watch and records unfinished orders, running work and the next actions. The chief reconciles the units. The operator closes the aide and decides whether to resume or start a resident. [Continuity](../../../plugin/skills/formation/references/continuity.md) · [Close-out](../../../plugin/skills/formation/references/close-out.md)

## Concerns & Honest Limitations

- The local room trusts cooperating processes under one OS account. Any joined seat can send to an online seat. Directed routes in the roster express intended communication. Enforcement, expiry and binding an inbox to a provider chat remain proposed.
- Native session messaging, a local inbox read and starting a new desktop turn have different observable outcomes. The [trial record](../shift-messaging/1_scope.md#local-sentry-trial-2026-11-24) identifies the host builds, successful child reads and missing parent turn.
- A quiet solo unit can leave new work waiting until its captain’s next wake. The operator can add a sentry or arrange resident coverage when that delay matters.
