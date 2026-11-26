---
title: "Design: formations composed from units"
owner: claude
created_date: 2026-11-16
updated_date: 2026-11-26
status: Draft
feature: formation
doc_role: design
summary: "Workspace units, staffing and authority, with a draft seat network built around watch processes and an optional dispatcher."
---

# Design: formations composed from units

The watch-based revisions to [Ranks](#ranks), [Units](#units) and [Chain of command](#chain-of-command)
are for design review. The [seat network](references/network.md#seat-network) defines the
proposed routes and the approval gate for their build.

Bearing: a draft, written on 2026-11-16 from Daniel's proposal in the lead's chat during the
second squad day. Daniel approved the names, settled the four questions under
[Decided](#decided) and approved the design the same day: "Approve and build". The team
rebuilt the skill from it on a branch. Daniel had it merged the same day, mid-formation
("merge to agent-main"), so the two orchestrators then running were sent the renamed
paths. The rules, the day, the
messages and the measures in [1_overview.md](1_overview.md) carry over unchanged; this doc
replaces its Roles, Authority columns and The formations sections. Where it leads, units
that a trigger starts and that regulate themselves, is in [3_vision.md](3_vision.md). A
proposal to run formations outside the Claude desktop app and without Orbit is in
[references/portable.md](references/portable.md); Daniel parked it on 2026-11-19.

## Why change

On 2026-11-16 Daniel opened `formation squad orbit` and an hour later asked for "a single
orchestrator for scribe workspace? opus. no need for squad formation." No spec fitted: a
squad staffs four sessions per workspace, and tree changes who Daniel talks to. The lead
staffed it by hand. Daniel, the same hour: "a better formation design would allow for
dynamic composition of various types. I think we should break it down into smaller units",
and "no more existing formations such as night-watch, tree, strike, solo etc. Just the
units, composable dynamically."

A named formation fixed two things at once: how much staff each workspace gets, and who
Daniel talks to. The first differs per workspace on the same day. The second no longer
needs to vary (see [Chain of command](#chain-of-command)).

## Ranks

A rank fixes a session's model and effort. It does not fix its duty: the captain hands out
duties as orders. Daniel: "\"implementer\" isn't just constrained to implementing. I think
we should allow for flexibility, so, for ex, if \"orc\" deems that certain work (triaging,
fixing red CI, rescuing PR, review PR or code changes, etc) requires bigger model, then orc
can delegate the work to \"implementer\"."

| Rank | Name | Model, effort | Replaces | Work | Record |
|---|---|---|---|---|---|
| chief | `0-chief` | Opus, high | lead, manager | Staffs the units, keeps the roster, handles mail, relays and the list of what waits for Daniel, is the first stop for every captain's questions and blockers, and runs the close-out. It wakes the designer for what the [designer](#the-chief-and-the-designer) owns | Its assignment in `ws_constellation` |
| designer, on demand | `0-designer` | Fable, high | the chief's design-partner duty | Writes every design doc, in whichever repository owns the feature, and is Daniel's design partner. Advises the chief on architecture and on decisions that go to Daniel with options. It carries none of the day's traffic | The design task it writes for |
| aide | `0-aide` | Haiku, high | aide | Briefs Daniel at its start with the previous session's summary and the open claims (scribe's open proposals); manages the scribe store on his instruction through the `scribe` CLI; quick questions; relaying his quoted words. Daniel, 2026-11-21: "I will be interacting with the scribe primarily to manage scribe artifacts, e.g. instruct scribe to approve certain claims, delete certain artifacts, etc. the scribe will be using cli tool to perform these operations." The CLI still refuses an agent settling a session's claim, changing an entry Daniel wrote, and any delete; until scribe changes, the aide hands him the line to type for those | Agent notes under its own name |
| captain | `<ws>-0-capt` | Opus, high | orchestrator | Commands one workspace per `orchestration-session`, owns its record, talks to Daniel | Its assignment |
| lieutenant | `<ws>-1-lt` | Opus, medium | implementer, reviewer | Whatever needs the big model: implementation outside the drains, root-cause analysis, debugging, PR rescue, review | The Orbit task it claims or is ordered onto |
| sergeant | `<ws>-1-sgt` | Sonnet, medium | resolver | Routine work: triage, re-runs, rebases, pilot warnings and holds, stuck PRs, and in a squad the mechanical red-CI fixes | A comment on each task it touches |
| sentry, on request | `<ws>-1-sentry` | Haiku, medium | watcher | Dispatches incoming items to the appropriate sergeant or lieutenant in its squad | See the [open dispatcher question](references/network.md#open-questions) |
| specialist | `1-spec-<subject>` | Sonnet, medium, unless Daniel names another | reviewer, in part | Answers asks on one subject from any captain; never acts ([Specialists](#specialists)) | None: a comment on the task an ask names |

- A `formation watch` process does the timed reading. Seats act when Daniel or an incoming
  message wakes them. Quiet workspaces can send findings to their captain, and busy workspaces
  can use a sergeant as the cheaper wake target. An assigned sentry receives the default
  watch output for its squad and dispatches it. See the [seat network](references/network.md#seat-network).
- Formation sessions use Orbit MCP when available. They select the workspace on its
  owner using the descriptor from `orbit.workspace.list`. When MCP is unavailable
  for the operation or owner, they use the Orbit CLI over SSH on that owner
  (Daniel, 2026-11-24). Both routes follow the rank’s authority. The CLI override
  applies to an individual authorized command and is recorded in Orbit’s audit trail.
  On 2026-11-23 the chief could not
  read attention counts at the start (the grant named captains and sergeants only), and
  he widened it: "I grant full operator authority and override to all chiefs and captains,
  unless specified otherwise." The chief and the captains now set the override on any
  command refused without it; what is Daniel's alone, and the chief's advising role, are
  read as specified otherwise and stand.
- The watch runs the attention and forge probes. Orbit's `ci_red` attention kind covers
  completed runs on the base branch's current head. The forge probe covers open pull
  requests as well. It reports base-branch red at once and reports a pull request after
  its check has stayed failed for 15 minutes on the same head, allowing the delivery run
  time to repair its own candidate.
- In a squad, red CI on the base branch is fixed by the unit and not left to Orbit's scheduled
  sweep (Daniel, 2026-11-19: "squad mode == high throughput mode"). The sergeant re-runs a
  flake and opens the fix when it is mechanical; a failure that needs diagnosis, or whose
  fix would touch an assertion or a guard, is the lieutenant's next order. The fixer claims
  the task first, naming the workflow, job, step, error and run, which is what the sweep
  matches before it files. The reasons are in the
  [overview](1_overview.md#authority). A red pull request stays with the run that owns it.
- The chief runs at high effort. `start_session` clamps a child's effort to its parent's, so
  a chief at medium starts every captain at medium (observed twice on the second squad day).
- Daniel talks to tier 0: the chief, the aide, the designer and every captain.

### The chief and the designer

Daniel, 2026-11-26, asked whether an Opus seat should stand in front of the chief: "many of my requests don't require fable (question, feature request, status check, even formation-complete). so things could get filtered by opus first." Most of a chief's turns that day were routine: acknowledging mail, relaying his answers, filing tasks, status checks and the fallback check. Each one paid for the whole day's context at the largest model's rate.

A filter in front of a standing Fable chief only filters mail. Whatever Daniel types in the chief's chat still wakes it. So the standing seat is the Opus chief, and Fable is kept for design, in a seat that is woken with a brief and stays small. Daniel: "I like that idea. but I think Fable should write all designs."

- The designer writes every design doc. It works from a design task, in a worktree of the repository that owns the feature, and opens the pull request there. That repository's captain lands it once Daniel has approved the design; no other session writes to the branch meanwhile.
- The chief wakes the designer for design work, for architecture advice a captain asked for, for a decision that goes to Daniel with options and one recommendation, and for a conflict between two rules. Daniel can open the designer's chat himself.
- The chief answers everything else. When it is unsure whether a question is the designer's, it asks the designer.

## Units

A unit is the staff of one workspace. Each has exactly one captain. The draft keeps the
unit names and moves each former timed reader into a watch process.

| Unit | Line-up | Fits | Attention owner |
|---|---|---|---|
| solo | captain, with a watch as needed | A quiet workspace, or one with a resident | The resident if active, else the captain |
| duo | captain and watch process | A workspace that needs watching and little clearing | The watch reads; the captain acts |
| trio | captain, sergeant and watch process | Steady traffic | The watch reads; the sergeant clears; the captain decides |
| squad | captain, lieutenant, sergeant and watch process | The most demanding work | As trio, with a lieutenant for heavy work |

- Daniel on residents: "if a resident is enabled, I won't be using \"duo\", I will be using
  \"solo\"." A unit starts its attention watch once it owns that workspace's attention.
  An active resident keeps ownership ([overview](1_overview.md), rule 6).
- The captain staffs a sentry on request and after approval from the chief or Daniel.
  A squad can also have multiple sergeants or lieutenants. A lieutenant is started when
  the first heavy piece is ordered, then kept.
- The sergeant and sentry are detached and report by message. A lieutenant stays attached:
  its turn end is the result the captain waits for. A lieutenant that writes code works in
  its own worktree on a task it has claimed with `context_files`.

## Composing

```
formation squad orbit, solo scribe      start the day
formation add duo site                  add a unit mid-day
formation add specialist standards      add a standing specialist
formation trio scribe                   change a staffed workspace's unit
formation                               chief and aide only
formation-complete                      close everything
```

The chief and the aide are staffed once, whatever the units. Daniel: "aide and lead will
always be there, just instantiated once."

Naming a staffed workspace again changes its unit: the captain starts the ranks the new
line-up adds, and takes the ones it drops through the close-out. The captain and its record
stay.

## Chain of command

Daniel: "when I engage directly with orcs, they are talking with me. Any issue that comes
up, questions, blockers, and else goes to you first." And of the lead: "advisor, manager
for the direct reports. To me, the lead will be design partner, discussing ideas etc - But
I will still directly interact with any top-levels."

```
Daniel ─┬─ 0-chief ─┬─ orbit-0-capt ──┬─ orbit-1-lt
        │           │                 ├─ orbit-1-sgt
        │           │                 └─ orbit-1-sentry
        │           └─ scribe-0-capt
        ├─ 0-designer
        └─ 0-aide
```

- The tree shows staffing. The [seat network table](references/network.md#seat-network)
  shows message routes, including peers and the aide. A sentry in the tree is optional.
- The chief sends work orders to captains, who propagate them through their units.
  Sends from the chief beyond the captains are announcements such as close-out or refresh.
  The aide exchanges requests and relays with the chief and captains.
- The designer exchanges messages with the chief. A captain's design question reaches it through the chief, which sends the question with the record it concerns.
- Lieutenants and sergeants report to their captain. A squad's appointed sentry can exchange
  dispatch messages with both ranks. Captains can reach their direct reports and peers.
- A captain that is blocked, needs a decision or has finished something it was asked for
  messages the chief. The chief answers what it can: advice, a fact from another unit's
  record, coordination between units. What only Daniel may settle the chief brings to him
  once, batched with what can wait, with the captain's options and recommendation intact.
- When Daniel is in a captain's chat, the captain answers him there. What he decides there
  that binds another unit, the captain writes on that unit's record and tells the chief.
- The chief settles no row of the Authority table. It filters and advises; it does not
  approve, merge or ship. No weaker model sits in the line: the chief is at least the
  captain's model.
- Captains are detached, so only their messages wake the chief.

## Authority

The table in the [overview](1_overview.md#authority) stands with its columns renamed: Sergeant, Captain, Daniel. A
captain with no sergeant (solo, duo) settles the sergeant's row itself. A lieutenant holds
no row: it works under the captain's orders, and its changes ship on the captain's approval.

## Delegating and reinforcing

- **By rank.** Routine work goes to the sergeant. The captain orders a lieutenant when the
  work needs the big model. A sergeant that meets such work passes it to the captain; it
  does not message the lieutenant.
- **Review.** A lieutenant never reviews a change it wrote. Review goes to Orbit's review
  crews, which also give a different provider. A second lieutenant is a reinforcement, for
  parallel heavy work or a review that cannot wait.
- **Reinforcements.** Daniel: "when things get busy, orc can request for additional units".
  A captain asks the chief for a rank, not a unit: "one more lieutenant", "a sergeant". A
  second unit in the same workspace would bring a second captain, against one attention
  owner per workspace. The 2026-11-16 design let the chief approve up to the squad
  line-up plus one more lieutenant per workspace. The watch-based design calls for
  approval from the chief or Daniel before a captain starts an additional direct
  report. The roster records the new seat. See
  [Captain routes and staffing approval](4_decisions.md#captain-routes-and-staffing-approval).
- **No cap on units.** Squad was capped at two workspaces because each orchestrator split
  Daniel's attention. The chief is now the first stop, and the close-out measure "questions
  that reached Daniel" shows when there are too many.
- Short fan-out is still a subagent, in any rank.

## Specialists

This section records the standing specialist design from 2026-11-16.
Daniel, 2026-11-16, on what session messaging allows beyond a chain of command: "it just
became a network", then "let's add standing specialists to the design." The network itself
is outlined in [references/network.md](references/network.md).

A specialist is a standing session that holds one subject's context and answers asks about
it, so that each unit need not load that context itself. Examples: `standards` (the
engineering standards, read against a diff), `release` (what a release needs before Daniel
cuts it), `security`.

- **It sits beside the chain of command, not in it.** It reports to nobody and commands
  nobody. The chief starts it, sends it the roster and detaches it.
- **Ask and answer.** A captain, the chief, or a lieutenant whose captain passed it the id
  sends one question with the record id and the answer needed. The specialist reads the
  thing itself, replies once to the asker with the answer first, and never starts a
  conversation. What must last goes on the task as a comment before the reply.
- **It never acts.** No edits, no PR comments, no task state, no sessions. One writer per
  set of files still holds.
- **Its answer is advice.** It holds no authority row, and an answer approves nothing. A
  Sonnet specialist may advise an Opus captain because it is not in the reporting line:
  nothing the captain sends upward passes through it.
- **Nobody waits on it.** Asks queue behind its current one. An asker says what it does
  without an answer by its next wake, as for any message.
- **When it earns a session.** Two or more units need the same loaded context, more than
  once in the day. A single question is a subagent.
- **Adding one is Daniel's.** He names it in his line. A captain that wants one asks the
  chief, who takes the ask to him.
- **It is replaced from its brief.** Its subject and sources are its whole state; an ask in
  hand is a comment on the task it names.

## What replaces the old formations

| Old | Now |
|---|---|
| solo | `formation` with no units: the chief and the aide |
| tree | Any units; the chief is the first stop that the manager was |
| squad | `formation squad <ws>` |
| strike | One unit per repo; the chief holds the shared contract and the task list |
| review-board | An order: the captain puts Orbit's review crews on the change, and asks a specialist for one lens |
| research | An order: a lieutenant follows `research-loop`; a second attacks the verdict |
| night-watch | Step 4 of the close-out, which already files a resident per workspace with pending tasks |
| firefight | An order from the affected workspace's captain: one lieutenant writes, everyone else holds dispatch |

## Original build

The standalone repository now ships the [Formation skill](../../../plugin/skills/formation/SKILL.md). The file layout below records the original umbrella implementation.

A change to the skill's files lands at a formation start or close,
never mid-day: running sessions hold briefs that cite them by path.

- `SKILL.md`: the grammar above, ranks, units, chain of command, start, run and close.
- `units.yaml` replaces `formations/*.yaml`: the ranks, each unit's line-up, who approves
  a reinforcement, and the night resident's defaults that night-watch held.
- `briefs/`: `captain.md` (from orchestrator), `lieutenant.md` (implementer and reviewer),
  `sergeant.md` (resolver), `sentry.md` (watcher), `specialist.md`, `aide.md`. The captain and sentry briefs
  take the unit as a parameter, since the sentry's reader differs between duo and trio.
- `1_overview.md` (first written as `1_scope.md`) keeps Roles and The formations as the
  record of the first design and points here. This doc is the design half.
- `orchestration-manager` and `docs/domain/orchestration-sessions.md` take the new names.

## Decided

Daniel, 2026-11-16, in the lead's chat. On the names (ranks, units, session names, grammar):
"name works." On the four questions the first draft left open: "Accept all four". On the
design and the branch build: "Approve and build".

1. **Review of a lieutenant's change** goes to Orbit's review crews. The squad has one
   lieutenant; the first draft had two so that one could review the other, and a second
   standing Opus session is idle most of the day.
2. **The chief approves a reinforcement** up to the squad line-up plus one more lieutenant
   per workspace. Beyond that Daniel decides.
3. **No cap on units.**
4. **Solo without a resident is allowed** for a quiet workspace, as `scribe-0-orc` ran on
   2026-11-16. The captain reads attention on its own wakes, with no timer, and asks for a
   duo when items sit unread.
5. **Standing specialists** are part of the design: "let's add standing specialists to the
   design." On their name (`1-spec-<subject>`), their model (Sonnet at medium unless he
   names another), that only he adds one, that captains and the chief ask and a lieutenant
   only with the id its captain passed, and when one earns a session: "They stand".
6. **Packaging.** The skill stayed in the Constellation umbrella while it was being proven. It is not an
   Orbit plugin: it needs the Claude desktop app's session tools, and Daniel on Orbit:
   "orbit is provider neutral, it should work in codex and claude at minimum." If the
   close-out measures show it works: "we should release it as claude plugin". That needs
   the procedure split from what is local here (his quoted preferences, `dk-server-2`,
   `ws_constellation`).

## Sources

The [umbrella record](https://github.com/constellation-works/constellation/blob/agent-main/docs/design/formation/2_design.md#task-references) identifies the work behind this design.

## Build

File one later task per item after Daniel approves the draft and settles the relevant
[open questions](references/network.md#open-questions).

1. **Roster edges.** Record ranks, reporting parents and squad membership, including approved additions, then derive the seat table from the roster.
2. **Mail routing.** Apply the approved edge rules in `formation mail send`. If Daniel chooses enforcement, add a one-line refusal naming reachable seats, and verify the chief's announcement route.
3. **Watch recipient.** Select the default addressee from the roster in sentry, sergeant, captain order. Verify that probes mail findings on change and cover deliberate process stops and failed reads.
4. **Sentry staffing and brief.** Rewrite the plugin's sentry brief and reference around the optional dispatcher, update unit defaults and role descriptions, and carry the approved dispatch rules into the brief.
5. **Chief probes.** Replace the scheduled fallback turn with change-detecting watch probes, using the receiving seat Daniel approves. Verify that an unchanged read wakes nobody.
6. **Chief and designer.** Set the chief's model to Opus in the unit defaults, add the designer's rank, brief and roster edge, and write the list of what the chief passes to the designer into the chief's brief.
