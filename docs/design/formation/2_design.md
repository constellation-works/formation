---
title: "Design: formations composed from units"
owner: claude
created_date: 2026-11-16
updated_date: 2026-11-25
status: Accepted
feature: formation
doc_role: design
summary: "Formations composed from units: seven ranks that fix a session's model and not its duty, four units (solo, duo, trio, squad) that Daniel composes per workspace in one line, a chief and an aide staffed once, standing specialists any captain can ask, the chain of command, how a captain hands out work and asks for reinforcements, what replaces the eight named formations, and what Daniel decided"
---

# Design: formations composed from units

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
| chief | `0-chief` | Fable or Opus, high | lead, manager | Staffs the units, keeps the roster, is the first stop for every captain's questions and blockers, and is Daniel's design partner | Its assignment in `ws_constellation` |
| aide | `0-aide` | Haiku, high | aide | Briefs Daniel at its start with the previous session's summary and the open claims (scribe's open proposals); manages the scribe store on his instruction through the `scribe` CLI; quick questions; relaying his quoted words. Daniel, 2026-11-21: "I will be interacting with the scribe primarily to manage scribe artifacts, e.g. instruct scribe to approve certain claims, delete certain artifacts, etc. the scribe will be using cli tool to perform these operations." The CLI still refuses an agent settling a session's claim, changing an entry Daniel wrote, and any delete; until scribe changes, the aide hands him the line to type for those | Agent notes under its own name |
| captain | `<ws>-0-capt` | Opus, high | orchestrator | Commands one workspace per `orchestration-session`, owns its record, talks to Daniel | Its assignment |
| lieutenant | `<ws>-1-lt` | Opus, medium | implementer, reviewer | Whatever needs the big model: implementation outside the drains, root-cause analysis, debugging, PR rescue, review | The Orbit task it claims or is ordered onto |
| sergeant | `<ws>-1-sgt` | Sonnet, medium | resolver | Routine work: triage, re-runs, rebases, pilot warnings and holds, stuck PRs, and in a squad the mechanical red-CI fixes | A comment on each task it touches |
| sentry | `<ws>-1-sentry` | Haiku, medium | watcher | The read of attention and of red CI on the base branch and open PRs, paced by load (every 2 to 30 minutes); reports changes only, never acts | None: one line per incident on the captain's assignment |
| specialist | `1-spec-<subject>` | Sonnet, medium, unless Daniel names another | reviewer, in part | Answers asks on one subject from any captain; never acts ([Specialists](#specialists)) | None: a comment on the task an ask names |

- The sentry keeps a duty name because it is the one session whose job never changes.
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
- The sentry also reads the forge (`scripts/ci-read.sh` in the skill). Orbit's `ci_red`
  attention kind covers only finished runs on the base branch's current head, and no pull
  request. Daniel (2026-11-19): "sentry should also be checking for red CI on agent-main
  branch and PR's". Base-branch red is reported at once. A pull request is reported once
  its check has stayed failed for 15 minutes on the same head, so that a delivery run
  repairing its own pull request costs the sergeant no turn.
- The sentry's sleep follows the load. It began as a fixed five minutes, which cost a turn
  every five minutes in a workspace with nothing running and was slow when a drain was
  failing. Daniel (2026-11-24): "we need to allow sentry to dynamically determine their
  sleep duration depending on load. currently its every 5 min. And also have them go idle
  when no work is expected". After each wake a script (`scripts/sentry-pace.sh`) chooses:
  2 minutes after a wake that reported something, 5 while an Orbit run holds a task in
  progress or a run's pull request moved in the last hour, 15 when nothing is in flight,
  and 30 after an hour of nothing, which is idle. Hands-on work does not count, neither a
  task nor a pull request: on the first evening each workspace had a hands-on task, and
  both sentries stayed at five minutes. Daniel (2026-11-24): "hands-on PRs shouldn't count
  as load either - but up to the judgement of the capt". So the captain can send
  `watch <minutes>` for one it wants watched. A script chooses and not the sentry, so the pace is the same for
  every sentry and a Haiku session does no arithmetic. Idle keeps a 30-minute read and
  does not stop the timer: on the day this was adopted a drain had been running for
  hours without its captain knowing, and a stopped sentry would not have seen it either.
  A reader that knows work is starting sends `watch`. `rest` pauses the sentry outright,
  with no timer, and the captain sends it whenever no work is expected. Daniel (2026-11-24):
  "captains should be pause sentries when there is no work expected". The cost is the one
  named above: a paused sentry sees nothing, so a drain started unannounced goes unwatched
  until someone sends `watch`. The chief passes a drain Daniel mentions to its captain. A failed probe counts as work in flight, since a failed read is not all-clear.

  Since this change, the sentry waits in `formation watch` (`scripts/attention-read.sh` and `scripts/ci-read.sh`, `--timeout 3600`), and the tiers above are history.
- In a squad, red CI on the base branch is fixed by the unit and not left to Orbit's scheduled
  sweep (Daniel, 2026-11-19: "squad mode == high throughput mode"). The sergeant re-runs a
  flake and opens the fix when it is mechanical; a failure that needs diagnosis, or whose
  fix would touch an assertion or a guard, is the lieutenant's next order. The fixer claims
  the task first, naming the workflow, job, step, error and run, which is what the sweep
  matches before it files. The reasons are in the
  [overview](1_overview.md#authority). A red pull request stays with the run that owns it.
- The chief runs at high effort. `start_session` clamps a child's effort to its parent's, so
  a chief at medium starts every captain at medium (observed twice on the second squad day).
- Daniel talks to tier 0: the chief, the aide and every captain.

## Units

A unit is the staff of one workspace. Each has exactly one captain.

| Unit | Line-up | Fits | Attention owner |
|---|---|---|---|
| solo | captain | A quiet workspace, or one with a resident | The resident if one is active, else the captain on its own wakes |
| duo | captain, sentry | A workspace that needs watching and little clearing | The sentry reads; the captain acts |
| trio | captain, sergeant, sentry | Steady traffic | The sentry reads; the sergeant clears; the captain decides |
| squad | captain, lieutenant, sergeant, sentry | The most demanding work | As trio, with a lieutenant for heavy work |

- Daniel on duo: "I am planning to have the watcher perform the loop check, and notifies
  orc." In a duo the sentry reports to the captain, under the same filter the watcher brief
  has today: a fingerprint once, a critical item at once, the rest in an hourly digest.
- Daniel on residents: "if a resident is enabled, I won't be using \"duo\", I will be using
  \"solo\"." Any unit with a sentry starts only after the workspace's resident is paused or
  past its deadline ([overview](1_overview.md), rule 6).
- The captain starts its own subordinates: the sergeant, then the sentry, so the sentry's
  first report has a reader. A lieutenant is started when the first heavy piece is ordered,
  then kept.
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
        └─ 0-aide

orbit-1-sentry → orbit-1-sgt → orbit-0-capt → 0-chief → Daniel   (by message, upward)
orbit-0-capt ↔ scribe-0-capt                                     (cross-workspace asks)
any captain ↔ 1-spec-<subject>                                   (one ask, one answer)
Daniel ↔ any captain                                             (directly, in its chat)
```

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
  second unit in the same workspace would bring a second captain and a second sentry,
  against one attention owner per workspace. The chief approves up to the squad line-up plus
  one more lieutenant per workspace, records it on the roster and tells Daniel in its next
  message. Beyond that Daniel decides: adding a session is cost, which is his row.
- **No cap on units.** Squad was capped at two workspaces because each orchestrator split
  Daniel's attention. The chief is now the first stop, and the close-out measure "questions
  that reached Daniel" shows when there are too many.
- Short fan-out is still a subagent, in any rank.

## Specialists

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

## Build

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
