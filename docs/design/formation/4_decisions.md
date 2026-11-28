---
title: Formation decisions
owner: codex
created_date: 2026-11-24
updated_date: 2026-11-26
status: Draft
feature: formation
doc_role: decisions
summary: Standing Formation decisions and the dated directions for the watch-based draft.
---

# Formation decisions

## Workspace lineups are composed from units

### Context

The original eight named formations bundled a whole shift into one choice. Two
workspaces on the same shift could need different levels of coverage.

### Decision

Staff the chief and aide once. Choose a unit for each workspace, then name
any reinforcement separately. The [approved design](2_design.md#units)
defines the units and [reinforcement authority](2_design.md#delegating-and-reinforcing).

### Consequences

A captain can request another seat as the workspace load changes.
Cost: the chief has to choose and communicate a lineup for each workspace.

## Watch takes over timed reading

### Context

The standing reader woke a model on each interval, including intervals with no change. Probes can read the same sources from a process.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat: "yes you are right. anything being dealt by agents can be programmatically watched." A `formation watch` process does the timed reading. Sessions handle the resulting mail.

### Consequences

Cost: someone must configure probes and keep the watch process running for the workspace it covers.

## Chief work orders go to captains

### Context

Direct orders to every rank would give workers two sources of direction. Captains already own their squads' work.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat: "agreed. just captains will be fine, and they propagate orders downwards as needed." The chief sends work orders to captains. Its sends to other ranks beyond the chief–aide exchange are one-way announcements, such as close-out or refresh.

### Consequences

Cost: an order takes a captain hop before it reaches the worker.

## Lieutenants report to their captain

### Context

A sergeant could handle a lieutenant's reporting as part of routine triage, but that would put the sergeant in the lieutenant's reporting line.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat, answered "yes" when asked whether lieutenants should report to their captain. Each lieutenant reports to its own captain.

### Consequences

Cost: the captain directly handles lieutenant reports and any decisions those reports require.

## Aide routes are two-way with chief and captains

### Context

The aide can relay one way, or it can receive questions and answers from the top-level seats it supports.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat: "yes, bidirectional between aide and captains" and "I think just captains and chief will do for aide". The aide exchanges messages with the chief and captains and relays Daniel's quoted direction or asks on his behalf.

### Consequences

Cost: messages between the aide and a squad's other seats take a captain hop. The aide's role is relay and assistance.

## Use a lower-cost wake target in busy workspaces

### Context

Routing every watch finding to the captain spends the larger model's turn and loads its longer context. A sergeant can handle routine findings.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat: "great idea" in response to using a smaller model as the wake target in a busy workspace and the captain in a quiet one. The [seat network](references/network.md#seat-network) shows the recipient order when a squad has a sentry.

### Consequences

Cost: the squad must staff and route to the sergeant for the lower-cost path, and decisions that need the captain still require an escalation.

## Staff the sentry as a dispatcher on request

### Context

A squad with several workers can benefit from human-language routing. A default timed reader would duplicate work the watch process already performs.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat: "there could be multiple sergeants or lieutenants. So, instead, captains and chief could request for aide unit, who will watch the mail box and direct the message to the appropriate unit within the squad." On its name: "then let's call it sentry. different role, different names". The sentry is an optional squad dispatcher staffed on request. The watch process owns timed probe reads.

### Consequences

Cost: a dispatched item can take an additional model turn. The detailed dispatch and return rules are [open](references/network.md#open-questions).

## Captain routes and staffing approval

### Context

Keeping every captain exchange through the chief adds a relay. Direct edges let a captain coordinate within its own squad and address its peers.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat: "Captains have bidirectional channels between their direct reports (could spawn sentry, lieutenant, or sergeant, upon approval granted from the chief or me), reporters, and other captains." A captain can start those direct reports once the chief or Daniel approves the staffing.

### Consequences

Cost: the roster has to reflect approved additions so peers and reports can resolve the intended seats. The form of a cross-workspace ask is [open](references/network.md#open-questions).

## Move the chief's scheduled fallback into probes

### Context

The twice-hourly fallback used one full chief turn per check. A watch probe can surface changes from the underlying records.

### Decision

Daniel, 2026-11-26 (UTC), in the chief's chat: "sounds good." He agreed to move the scheduled fallback check into probes that send mail on change.

### Consequences

Cost: the probes must cover the fallback's checks and expose failed reads. Their receiving seat is [open](references/network.md#open-questions).

## Keep JSON as the piped form of mail output

### Context

STD-01@4 §R9 says the piped form SHOULD be one tab-separated line per record. Mail's `join`, `who`, `send`, `inbox` and `ack` have printed one JSON document when piped since they shipped, and seats, briefs and hook launchers parse it (receipt IDs, `held`, message bodies and `seat_dir`). Mail also had no readable form for a terminal.

### Decision

The operator, in the task that added mail output modes, asked for human and JSON modes with the existing piped JSON kept as the compatibility form. Those five verbs keep one JSON document as their piped form and add a readable form on a terminal. `mail sent` keeps its tab-separated piped form. The mode resolves once: `--json`, then `FORMATION_FORMAT` (`auto`, `human`, `json`; unknown values mean `auto`), then terminal detection. This deviates from STD-01@4 §R9 only in the piped default of those five verbs.

### Consequences

Cost: a pipeline that wants fields from `who` or `inbox` reads JSON rather than `cut -f`. Revisit with a breaking-change notice under STD-01@4 §R35 if the piped default ever moves to tab-separated rows.
