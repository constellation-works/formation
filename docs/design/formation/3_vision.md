---
title: "Vision: a self-regulating formation"
owner: claude
created_date: 2026-11-16
updated_date: 2026-11-28
status: Accepted
feature: formation
doc_role: vision
summary: "Where formations lead, as a proposal: the incident, not the unit, is what a trigger creates; each incident carries a goal, a budget and a deadline; a response ladder tries the cheapest responder first and a unit is its top rung; one regulator leases capacity against demand; and the system learns by moving each class of incident down the ladder — set against Daniel's immune-system picture, with where the two differ"
---

# Vision: a self-regulating formation

Bearing: a direction, not a commitment. Daniel accepted it as the direction on 2026-11-16
("This direction"). Nothing here is approved to build; each step under
[Order to build](#order-to-build) needs its own yes. The committed
design is [2_design.md](2_design.md); the network it runs on is
[references/network.md](references/network.md). This doc is Claude's proposal in answer to
Daniel's picture below, and it differs from that picture in five places, each marked.

## Daniel's picture

Daniel, 2026-11-16: "think of how immune system work. small pieces, working in tandem, to
achieve a goal in dynamic way. upon certain triggers, it spawns certain units. when
insufficient, they call for backup, more units arrive. When work tapers off, they reduce in
count. the optimizing parameters should be cost, time, and effectiveness. Not really sure
how they will optimize overtime on their own; but perhaps the only thing that matter is
being able to learn from failures and reflections (like creating guardrails and new
rules?), like creating anti-bodies." And: "upon configurable triggers, it will spawn a
single unit. This unit, then, resolves whatever issue or goal, all the while
self-regulating itself."

## What the proposal keeps and what it changes

It keeps the goal: a system that responds to events without Daniel typing the line, scales
to the load, shrinks when the load goes, and gets better from its failures.

It changes the mechanism, because our parts are not like immune cells. A cell is nearly
free and the body makes millions; redundancy and waste are how that system works. An Opus
session is the most expensive thing we run. A design that answers every trigger by
starting a unit spends the most where the least thought has happened.

| Daniel's picture | The proposal | Why |
|---|---|---|
| A trigger starts a unit | A trigger opens an **incident**; a [ladder](#the-response-ladder) tries the cheapest responder first, and a unit is the top rung | Most triggers are known, or are noise |
| Each unit regulates itself | Units report demand; **one regulator** grants and withdraws capacity | A unit sees only its own queue. Each will ask for more, and nobody trades one off against another |
| Cost, time and effectiveness are optimized | They are **terms on each incident**: a budget, a deadline, an exit condition | A target nobody set cannot be missed |
| Units reduce in count | **Turns** are reduced, and which model takes them | An idle session costs nothing. Cost is turns times the model's price |
| Antibodies are guardrails and rules | An antibody is a **cheaper way to handle the same thing next time**; a rule is its weakest form | It makes learning measurable |

The immune system still fits, read at a different level. Its innate response is fast,
generic and cheap; its adaptive response is slow, specific and costly, and leaves a memory
that makes the second exposure cheap. That is the ladder.

## The incident

What a trigger creates is a record, not a session. An incident is an Orbit record with:

- a **fingerprint** and a **class** (attention items already carry fingerprints);
- a **goal**, stated as an exit condition that can be checked;
- a **budget** and a **deadline**, set by its class and severity;
- one **owner** at any time, and the rung it is on;
- its **trail**: who held it, what each tried, what it cost.

Capacity is assigned to incidents. A second trigger with the same fingerprint attaches to
the open incident instead of opening another, which is the rule "one owner per problem"
made mechanical.

Triggers are configuration: an attention kind over a threshold, a red base branch, a
resident's notice, a tool any agent can call, a step in an Orbit workflow, Daniel's line.
All of them do one thing: open or attach to an incident.

## The response ladder

Every incident starts at the lowest rung that has an answer for its class and moves up
only when that rung fails or its share of the budget is spent.

| Rung | Responder | Cost | Used when |
|---|---|---|---|
| 0 | A filter | None | The class is known noise: drop it, with the condition that lifts the filter |
| 1 | A handler: a script, a check, an Orbit routine | Machine time | The class has a deterministic fix |
| 2 | A playbook, followed by the cheapest rank that can | A few Haiku or Sonnet turns | The class has been solved before and the steps are written |
| 3 | Triage by a sergeant | Sonnet turns | The class is new: find the cause, or say it needs more |
| 4 | A unit: a captain, with lieutenants as it needs | Opus turns | It needs judgment, a conversation, or work across files |
| 5 | Daniel | His attention | It is in his rows, or the budget is spent with the goal unmet |

Today nearly everything enters at rung 3 or 4, because rungs 1 and 2 hold little: the
runbooks, a few routines, the sergeant's four settled actions.

## The contract

Cost, time and effectiveness are what an incident is opened with, and each has a defined
breach:

- **Effectiveness** is the exit condition. An incident closes only when it is checked,
  and one that reopens counts against the rung that closed it.
- **Time** is the deadline. When it is at risk, the incident moves up a rung early.
- **Cost** is the budget. When it is spent with the goal unmet, the incident goes to
  rung 5 with what was tried and the options. Nothing spends past its budget on its own
  authority.

## The regulator

One regulator holds the whole formation's budget and decides capacity. A unit does not
staff itself.

- **Units report demand, not requests.** A captain keeps its queue on its record in a
  form the regulator reads: open incidents by class, the age of the oldest, the wakes that
  ended with no action. The chief plays the regulator while it is a session; the aim is
  that Orbit computes it.
- **Capacity is leased.** A rank is granted to an incident or a queue for a period, and
  the lease ends unless demand renews it. Shrinking is the default and needs no decision;
  only growing does.
- **Turns are what is regulated.** A sentry reads every five minutes while incidents are
  open and backs off as the workspace goes quiet. A message goes to the cheapest rank that
  can act on it. A captain that is woken with nothing to decide is the signal that a rung
  below it is passing too much up.
- **Span of control.** A captain commands at most five sessions. Past that the incident is
  split, each part with its own owner, as incident command does in the field.
- **Brakes.** One incident per fingerprint, a cap on live units, a spawn rate, and a stop
  Daniel can pull that holds every trigger.

## Learning is demotion

The system has learned when the same class of incident is closed on a lower rung than
last time.

1. Every incident that reaches rung 3 or above closes with an **after-action** on its
   record: the cause, what closed it, what it cost, and one proposed way to handle its
   class lower down.
2. That proposal is the antibody. A class solved once at rung 4 gets a **playbook**, so a
   sergeant closes it next time. A playbook that has run cleanly several times becomes a
   **handler**, so no model does. A class that is noise gets a **filter**.
3. A rung that fails is corrected the same way: when a playbook does not close its
   incident, the incident moves up, the playbook is marked stale, and the after-action
   fixes or retires it.
4. A **rule** in a brief or a standard is for what cannot be handled lower: a mistake in
   judgment, not a recurring event. It cites its incident, and Daniel admits the ones
   that change who decides what.

This answers "how they will optimize overtime on their own" without tuning anything: the
mix of rungs shifts downward, and the cost of each class falls with it. It also bounds the
rules. A playbook or a handler that never runs costs nothing; only rules crowd a brief,
and they are the last resort.

## What keeps it safe

- **Authority does not grow with the system.** A responder on any rung holds the rows its
  rank holds today. Automation changes who starts work, never who may approve it.
- **Triggers and budgets are Daniel's.** A new trigger starts work and a budget spends
  money. Proposing either is the system's; admitting it is his.
- **A false positive costs one filter, once.** On 2026-11-16 a critical item known to be
  false sat at the top of two workspaces all day. Under the ladder its first incident ends
  at rung 0 for every later one.
- **The response cannot feed itself.** An incident opened by a responder's own action
  attaches to the incident that responder holds.

## Where it runs

In Orbit. The desktop app cannot be woken from outside a session, and it stops when the
Mac sleeps ([network](references/network.md#what-it-limits)). Orbit runs around the clock
for every provider, and it already has most of this for planned work: tasks, a pilot that
checks them before they run, crews by model, drains that take them, routines that fire on
state, frictions that record failures. What it lacks is the same loop for operations:
things that break in the running system, which today wait for an orchestrator or for
Daniel.

So the proposal is not a second system beside Orbit. It is an incident class in Orbit,
with the ladder as its routing. A formation unit is one responder, for rung 4 by day. A
resident is the same rung at night. Ranks, rows, records and the close-out are shared.

## How we know

Per incident class, over time:

- the share of incidents closed on each rung;
- cost and time per incident;
- the rate at which closed incidents reopen;
- how many reached Daniel that were not in his rows.

The first should move down, the second and third should fall, and the fourth should reach
zero.

## Order to build

Each step is useful alone, and none needs the next.

1. **After-action at every close**, with the rung recorded by hand. This is the data
   everything else is judged on.
2. **A playbook shelf**, keyed by fingerprint class. The sergeant's brief says: look
   there first. This is rung 2, and it needs no new machinery.
3. **Demand on the record and an adaptive sentry.** Captains keep their queue in one
   form; the sentry's interval follows it.
4. **Leases.** A granted rank expires unless renewed.
5. **The incident record in Orbit**, with a budget, a deadline and an exit condition, and
   one trigger feeding it.
6. **Routing by rung in Orbit**, and the regulator computed instead of played by the
   chief.

## Open questions

- What sets a class's budget and deadline at first, before there is history?
- Who writes a playbook: the unit that solved the incident, or a specialist that reads
  after-actions across units?
- How many clean runs make a playbook safe to turn into a handler, and who checks the
  handler?
- Is an incident a new Orbit record, or a task type with these fields?
- What is the exit condition for an incident that is a question and not a fault?

## Not yet verified

Operational assumptions carried forward from skill trials that remain unverified against recorded runs in this repository (the earlier skill section marks detached native-message delivery and a solo captain's own attention reads as verified, so this list omits them):

- **Unverified — remaining CLI executor actions across owner, follower, and desktop routes:** Owner-route task approval, ship, run reads, requeue, and attention were observed, and routine enablement is documented as something an agent session cannot do; which remaining CLI executor actions honor operator capability on the owner and through a follower or desktop route (especially landing, drain control, remaining recovery actions, and routine control) is unverified.
- **Unverified — sergeant native source repair and refresh:** Native source repair and refresh performed by a sergeant for a red or conflicting pull request (distinct from an earlier orchestrator source-repair run) is unverified.
- **Unverified — sentry attention-wait grant and waking kinds:** An unprivileged attention probe holds no operator capability and a captain's federated attempt was refused; whether a sentry can mint and use an attention-wait grant, and exactly which attention kinds wake it, is unverified.
- **Unverified — sergeant heartbeat hold and intervening mail:** Wait durations above 55 minutes have not been trialled; whether a sergeant's periodic heartbeat maintains the hold over a full night and whether intervening mail wakes it is unverified.
- **Unverified — chief periodic merged-PR review and routing across a full shift:** Whether the chief's periodic merged-pull-request review and routing work through a full shift is unverified in recorded shift runs.
- **Unverified — attached polling waking a parent at every child turn end:** Provider-specific evidence differs and remains incomplete: on Codex Desktop (build 26.1120.41243), a completed child and explicit child collaboration message did not start a turn in an idle parent; the formation overview states that an idle parent is woken once none of its children is still working, but whether an attached poller wakes its parent at every child turn end is unverified in a recorded run.
- **Unverified — hourly digest for older review-waiting items and duo sentry mail turn cost:** Empty sentry turns during a five-hour run were not counted; whether hourly digest rules for older review-waiting items leave a usable captain wake, and whether direct duo sentry mail has tolerable turn cost, is unverified.
- **Unverified — read-only lieutenant worktrees and component-local staffing:** Project-root restrictions for some starts are known; whether a read-only lieutenant can use an isolated worktree, and whether component-local staffing loads the expected project instructions, starts subordinates there, and addresses lifecycle controls across projects, is unverified.
- **Unverified — self-clear restoration and chief fallback timer across compaction:** Self-clear itself was demonstrated while the first handoff recovery attempt failed; whether self-clear restores the handoff and Scribe seat, and whether the chief's fallback timer survives compaction, is unverified.
- **Unverified — Scribe group close after opener clear:** Whether Scribe group close works from the original opener after it clears, and how the older model-backed close, shell issues, or substitute approval flow apply to the current Scribe command, is unverified.
- **Unverified — typed-turn exclusion and group-open related-task syntax:** Whether peer messages and scheduled prompts stay excluded from the operator's typed-turn record, and whether Scribe group-open related-task syntax works on a live store (no verified live use is recorded in this repository), is unverified.
