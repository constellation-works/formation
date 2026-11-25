---
title: "Proposal: a portable formation kit"
owner: claude
created_date: 2026-11-19
updated_date: 2026-11-25
status: Draft
feature: formation
doc_role: reference
summary: "A proposal to take formations to any agent app, at work included, starting small: v1 is a sentry, a file mailbox per agent and hooks that hand a main agent its mail; with what each provider's hooks allow, what is not verified, the order to build, alert email triage as the first use, and the full team (ranks and units on the same mailbox) left for later"
---

# Proposal: a portable formation kit

Bearing: parked. Daniel, 2026-11-19, after reading it: "let's forget it. too much hassle,
with approval and everything." Nothing here is to be built; it stays as a record of the
idea. As written it was a proposal, nothing approved to build. Each step under
[Order to build](#order-to-build) needs Daniel's yes. The formation design is
[2_design.md](../2_design.md); the network it runs on today is [network.md](network.md).

## What Daniel asked for

Daniel, 2026-11-19, in the chief's chat:

- "I am thinking about creating a custom connector to enable this formation feature to
  different providers. I can only use this with claude code".
- "I think this formation feature belongs in desktop apps, i.e. the entire team in one
  place, where user interacts with the team. orbit is designed for automated delivery - you
  file tasks and have the drain take care of things; resident orchestrator as well.
  Formation is more for hands-on approach by picking the right team to effectively deal
  with whatever goal."
- "I want something light-weight, not a full framework, something I can use at work."
- On the first draft of this doc: "for v1, I think just sentry is enough. just mail box for
  each sentry subagents and the main agents. With a hook for the main agent that notifies
  it when its mail box has contents."

The chief had first suggested a session host inside Orbit. That is withdrawn: a host is a
framework, it puts the team outside the app Daniel works in, and it needs Orbit, which is
not at work.

## v1: a sentry, mailboxes and a hook

A **main agent** is a session Daniel works in. A **sentry** watches something for it (CI,
a review queue, a tracker) and reports what is new. v1 has no ranks, no units and no
roster.

```
formation-kit/
  fm             one script, standard library only
  hooks/         one hook file per provider, each calling fm
  watch/         the project's watch commands
```

`fm` keeps everything as plain files in `.formation/` in the project, ignored by git.
There is no service and no network, and it works on one machine.

### The mailbox

One mailbox per agent: each main agent and each sentry. A message carries the sender, the
time and the text. Reading a message marks it read, so it is delivered once.

| Verb | What it does |
|---|---|
| `fm send <to> "<text>"` | Puts a message in a mailbox |
| `fm read <me>` | Prints the unread messages and marks them read. Prints nothing when there are none |
| `fm wait <me> [seconds]` | Returns when mail arrives or the time runs out |
| `fm sentry --to <box> --every <seconds>` | The sentry loop: run the watch commands, send only lines whose fingerprint is new |

A main agent's mailbox is named by its session id, which the hook gets from the app and
tells the agent once. A sentry's mailbox is named by the main agent that starts it.

### The hooks

The hook is how a main agent learns of mail without checking for it. It runs `fm read`,
and an empty mailbox prints nothing, so it costs no tokens.

| When | What the hook does |
|---|---|
| The user sends a message | Adds the unread mail to the turn |
| After a tool call | Adds the unread mail to the turn |
| The agent is about to stop | If mail is waiting, holds the stop and hands the mail over as the next input |
| The agent is idle | Wakes it with the mail, where the app allows it |

What each provider's hooks allow, from their documentation on 2026-11-19. Nothing here
has been tested.

| Provider | On user message | After a tool | Hold the stop | Wake when idle |
|---|---|---|---|---|
| Claude Code | yes | yes | yes | yes: a background hook that wakes the session when it exits (`asyncRewake`) |
| Codex CLI and app | yes | yes | yes | none documented |
| Gemini CLI | yes | yes | yes | none documented |
| Cursor | no | yes | yes, five times in a row at most | none documented |
| Copilot CLI | no | yes | yes, eight times in a row at most | none documented |

Where nothing wakes an idle agent, mail waits for Daniel's next message. When he wants a
main agent to stand watch, he says so, and it stays in its turn by calling `fm wait`.

### The sentry

Two kinds, with the same mailbox, so either fits.

| Kind | What it is | Costs | Use it when |
|---|---|---|---|
| Script | `fm sentry` running in the background, with no model | Nothing | The read needs no judgment: a failed check, a new review request |
| Subagent | A subagent on the small model that runs the watch commands on a timer and writes each report itself | One small-model step per wake | The read needs judgment, such as saying why a build failed |

A watch command is any script that prints `<fingerprint><tab><line>`, the shape
`scripts/ci-read.sh` already has. A sentry reads its own mailbox at each wake; the main
agent sends it there a `mute` line or a change of what to watch.

### Where v1 differs from Daniel's sketch

| His sketch | The proposal | Why |
|---|---|---|
| A custom connector | A script and hooks. A connector is a wrapper over the same files, built only for an app with neither a shell nor hooks | Every coding agent runs a project script. A connector must be installed, and often approved, per app |
| Sentry subagents | A script sentry by default, a subagent where the read needs judgment | A read that only compares fingerprints needs no model. Codex and Gemini document subagents that report when done, not ones that keep running |
| One hook that notifies | Up to four hook points, by provider | Only Claude Code documents a way to wake an idle session. The others can only deliver mail while a turn runs or ends |

### Not verified

- **`asyncRewake`.** The Claude Code docs describe it and say it wakes an idle session.
  They do not say which hook events accept it.
- **Which agent a hook fired in.** Claude Code hooks also fire inside subagents. The
  hook must stay silent there. Telling the two apart by the `agent_id` field is inferred
  from the docs, not stated.
- **Session ids** in the other providers' hook input.
- **Cursor**: its docs do not say which hooks work in the CLI and which only in the IDE.
- **Limits.** Claude Code caps what a hook adds at 10,000 characters, so `fm read` hands
  over a bounded amount and says how much more waits.
- **Trust.** Every provider asks the user to trust a project's hooks. At work the hook
  file can sit at user level, outside the repository.

### Order to build

1. **The mailbox and the Claude Code hooks.** Send a message by hand and see it arrive in
   three cases: mid-turn, at the stop, and when idle. This settles the first two items
   under [Not verified](#not-verified).
2. **The script sentry**, with one real watch command.
3. **The subagent sentry**, its mailbox and `mute`.
4. **The hook file for the provider at work.**

## First use: alert email triage

Daniel, 2026-11-19: "I think this will be useful for automating alert email triage. I get
so many of these, and currently I have to manually determine whether something should be
taken action." And: "I want to start a single main agent session just for triage, and as
alerts come in, cheap sentry agent checks for duplication/already-resolved/cataloging/
initial assessment based on knowledgebase."

One main agent, and a sentry in three stages, each handing on only what it cannot settle.

| Stage | Who | Settles | Hands on |
|---|---|---|---|
| 1. Fetch and match | A script, no model | Fingerprints each alert, counts an exact repeat, pairs a "resolved" mail with its alert, applies the mute rules | Fingerprints it has not seen, or that reopened |
| 2. Assess | The small model, one fresh call per batch | Near-duplicates, the knowledge-base entry that applies, a first assessment, `now` or `digest` | Alerts that may need action, to the main agent's mailbox |
| 3. Decide | The main agent, with Daniel | What to do, and whether this alert is noise from now on | A decision written back to the catalog |

- **The catalog** is the centre, more than the mailbox: one entry per fingerprint with
  first and last seen, the count, the state (open, resolved, muted), the assessment and
  Daniel's decision. Duplicate and already-resolved are lookups in it, and it becomes
  part of the knowledge base.
- **Decisions move work down.** When Daniel rules that an alert is noise, that becomes a
  mute rule, and stage 1 handles the next one with no model. The share that reaches stage
  2 should fall over time, and that share is the measure.
- **Stage 2 never closes or mutes.** It only sorts. An alert it is unsure of goes to the
  main agent, and so does anything marked critical by its sender. Only a decision at
  stage 3 creates a mute rule.
- **Batches, not single mails.** One outage sends hundreds of mails. Stage 1 groups what
  arrives within a short window, and a burst reaches the main agent as one message.
- **A digest** lists what was dropped or held, so that a wrong drop can be seen.

This differs from "sentry subagents" in one place. Stage 2 is a fresh call of the small
model for each batch, started by the script, not a subagent that lives all day. A
long-lived subagent's context grows with every alert, and if the main agent has to start
it, each batch costs a turn on the big model. Where the script may not call a model, a
background subagent that waits on its own mailbox takes stage 2.

Not known, and they decide how stage 1 is built:

1. What sends the alerts, and can a script read them: the mail itself (IMAP, an API, a
   rule that files them) or the alerting system's own API, which also knows what resolved?
2. May a script at work call the model without a session open?
3. Where is the knowledge base, and can it be read as files?

## Later: the full team

The same mailbox carries the rest of a formation, if v1 earns it. Nothing below is part of
v1.

- **Ranks and units.** The skill's rules, ranks, units, authority table and briefs are
  text and travel as they are. `fm open "<line>"` writes a roster, and `fm join <seat>`
  prints a seat's brief. Model names become tiers (big, mid, small) mapped per provider.
- **Starting sessions.** The user opens them: `fm open` prints, per seat, the model and
  the one line to type. A squad costs five sessions to open by hand.
- **Records.** One markdown file per seat replaces the Orbit task; `fm note` adds a dated
  line. The rule that a session can be replaced from its record holds unchanged.
- **Left out for good:** starting, re-modelling and archiving sessions from a script,
  enforcing who may message whom, and more than one machine.
- **Home.** The formation at home keeps Orbit for records and attention. If the script
  sentry works, it could replace the Haiku sentry session there.

## Questions for Daniel

1. Which app and provider is it at work? Step 4 is built for that one, and the table
   above says what its hooks allow.
2. At work, may you add a hook file at user level, and may a session run a script from
   the project?
3. Is a script sentry acceptable as the default, with a subagent only where the read
   needs judgment?
4. Where should the kit live so that it can go to work: a public repository, or a folder
   you copy?
