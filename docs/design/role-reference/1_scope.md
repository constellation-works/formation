---
title: Formation — Role references
owner: claude
created_date: 2026-11-27
updated_date: 2026-11-27
status: Draft
feature: role-reference
doc_role: scope
summary: One Formation skill in which every role reads one reference of its own, a seat card replaces the per-role briefs, and each role's record carries the crew of the seat that holds it.
stage: idea
next: The operator answers the two open questions and approves; then build phase 1, the role files and the seat card.
---

# Formation — Role references

The operator chose the direction on 2026-11-27 ([Decided](#decided)). The layout, the seat card, the record table and the checks are proposed here and not yet reviewed.

## Problem

A seat learns its job from three places, and the skill that holds two of them exists twice.

| Place | What it holds | Size |
| --- | --- | --- |
| The brief, pasted as the seat's first message | The role's procedure with the day's values filled in | Captain 13,549 bytes, sergeant 13,474 |
| The skill, which the brief sends the seat back to by section or reference name | The rules, the authority table, records, refresh, close-out | 3,578 bytes of `SKILL.md` and 37,421 of references here |
| The operator's orchestration skills | How a captain claims and keeps its record | Outside this repository |

The operator's own coordination repository keeps a second copy of the skill, with its own briefs. That copy is the one a formation runs from today. Both copies load in every session under the name `formation`.

| Observed | Cause |
| --- | --- |
| The second copy's `SKILL.md` is 66,145 bytes and 916 lines, four times the 16,000-byte budget and nearly twice the 500-line one ([STD-11](../../standards/STD-11-agent-context/STD-11.md) §R16). A captain's brief sends it to that file for its rules | One file holds every role's rules, the reasons and the dated history, and nothing checks its size |
| The sergeant brief is 18,119 bytes in one copy and 13,474 in the other. The advisor has a brief in one copy and no entry at all in this one | Every rule change is made twice, by hand ([STD-11](../../standards/STD-11-agent-context/STD-11.md) §R2 allows one canonical directory) |
| 2026-11-27: all four records of the day (the chief's and three captains') showed a crew nobody chose. One captain ran on one model family and its record named another | The chief files each record with no crew, so the tracker draws one from a pool and redraws it when the complexity changes. [Start](../../../plugin/skills/formation/references/start.md) step 3 names the tags and the complexity but no crew, and the captain brief's claim step names the plan only. The rule exists, in a skill neither step points at |
| The same day the advisor had no record. Its state was the design task it happened to be writing, so the seat had no row beside the chief's and the captains' | The unit definitions give the advisor's record as "the design task it writes for" |
| A record filed that day with its crew set kept it: the tracker logged the crew as explicit | Setting the crew at filing is enough to stop the draw |

The first three rows are one fault: no single file is the place where a role's instructions live. The record rows are what falls through when a rule has to be remembered across files.

## Goal

- A seat reads `SKILL.md` and one reference to know its whole job. The day's values reach it on one short card.
- A rule about a role is written once, in that role's file.
- There is one Formation skill. An operator's own choices live outside it, in a file the skill reads.
- Every role's record has a title, tags and a crew that are set when it is filed and are checked by a command.
- The skill's size and links are checked in CI.

Out of scope: changing what any rank may decide, the unit line-ups, mail, watches, or the orchestration skills. Changing how the tracker pilots a record task is also out of scope.

## Shape

```
plugin/skills/formation/
  SKILL.md                    what Formation is, the rules every seat follows, the routing
  references/
    role-chief.md             one file per role
    role-advisor.md
    role-aide.md
    role-captain.md
    role-lieutenant.md
    role-sergeant.md
    role-sentry.md
    role-specialist.md
    start.md  run.md  close-out.md           procedures, as today
    authority.md  records.md  continuity.md  mail.md  roster.md  units.md
    provider-claude.md  provider-codex.md
  assets/
    units.yaml                names, default models, and each role's record
    seat-card.md              the one brief
    orbit-mcp.json
  scripts/
    attention-read.sh  ci-read.sh  record-check.sh
  evals/cases.md
```

| Part | What changes |
| --- | --- |
| Role files | New. Each takes the procedure from that role's brief and the rules the brief pointed at |
| Seat card | New. Replaces `assets/brief-*.md`, which are removed |
| `SKILL.md` | Gains one step that routes a seat to its role file, and the list of role files |
| `units.yaml` | Each role's `brief:` becomes `role:`, and `record:` becomes a title, tags and crew |
| `references/sentry.md` | Folds into `role-sentry.md`; only the sentry reads it |
| Advisor | Added as an optional seat ([open question 1](#open)) |
| `record-check.sh` | New. Compares each record with the roster |
| Skill check | New, in `make ci-fast`. Budgets, links and the role set |

### Role files

A role file is everything that role needs beyond `SKILL.md` ([STD-11](../../standards/STD-11-agent-context/STD-11.md) §R9). Every one has the same six headings, so a seat finds the same thing in the same place whichever role it holds:

```markdown
# Role: captain

Seat `<ws>-0-capt`. Started by the chief. Reports to the chief. The admiral talks to you directly.

## First turn
1. Join the group and the mail room named on your card.
2. Claim your record (see Record).
3. …

## Duties
…

## You settle
Rows two and three of the authority table: … Everything else goes up with options and one recommendation.

## Messages
Who you may message is on your card. …

## Record
…

## Refresh and close
…
```

- **Each file carries its own authority row.** A sergeant reads its row in `role-sergeant.md`. The whole table stays in [authority](../../../plugin/skills/formation/references/authority.md) for the chief and for an operator reading it.
- **A role file names a procedure only by its title.** `SKILL.md` links every reference, so nothing is reachable only through a role file (§R10). The chief reads `role-chief.md` and, when it starts a shift, `start.md`; neither needs the other to be understood.
- **What varies by unit is a table in the role file,** not a filled placeholder. The captain's staffing lines for solo, duo, trio and squad become one table in `role-captain.md`, keyed by the `unit` on its card.
- **A file over the budget splits by subject and keeps the prefix** (§R9): `role-captain.md` and `role-captain-staffing.md`. The captain's brief is 13,549 bytes before its authority row and record rule are added, so it is the one likely to split.
- **No history and no reasons.** A role file says what to do now (§R14). Reasons stay in the design docs.

### Seat card

The card is the only thing a starting session pastes. It holds the day's values and nothing a role file could hold:

```markdown
You are `orbit-0-capt`, captain of `orbit` in formation `g-12`.
Read the Formation skill, then `references/role-captain.md`, and follow its First turn.

skill:        /abs/path/to/skills/formation
house rules:  /abs/path/to/house.md        (or: none)
unit:         trio (you, a sergeant, a sentry)
group / room: g-12
record:       <task id>, crew `opus`
workspace:    hm_<owner>/ws_orbit, repo owner/name, base agent-main
reports to:   0-chief (<session id>)
may message:  0-chief, orbit-1-sgt, orbit-1-sentry, the other captains
repair mode:  on
attention:    yours (no resident)
handoff:      <two or three lines from the last group's summary>
open items:   <attention counts by kind; what waits for the admiral>
```

- **One template for every role.** A line a role does not use is left out when the card is filled.
- **The card is kept.** The seat writes it to its [seat folder](../seat-folder/1_scope.md) as `card.md` on its first turn. After a compaction or a refresh it reads the card and its role file again instead of relying on a summary of its first message.
- **`skill` is an absolute path** so a seat whose host cannot load a skill by name still reaches the files. The existing `formation_skill` value in the briefs is the same path.

### Records

The record of each role moves from prose to fields in `units.yaml`:

| Role | Title | Tags | Crew | Filed by |
| --- | --- | --- | --- | --- |
| chief | `[orchestration]: formation <units> - <family>` | `orchestration`, `formation-day`, `no-drain` | the chief's | the chief |
| advisor | `[advisor]: formation <group> - <family>` | `advisor`, `no-drain` | the advisor's | whoever starts it ([open question 2](#open)) |
| captain | `[orchestration]: <ws> - <family>` | `orchestration`, `no-drain`, the workspace | the captain's | the chief, before the start |
| lieutenant, sergeant | the task it is ordered onto or claims | that task's | its own, set when it claims | — |
| aide, sentry, specialist | none | | | |

- **The crew is the workspace's registered crew for the model the seat actually runs on.** The chief already lists the workspaces at start; it asks for their crews in the same call and writes the match on the card.
- **Filing sets `crew` and `orchestrator` together.** The start procedure's filing step names both. A record never waits for a draw.
- **The claim verifies.** The first-turn step of a role with a record is one update: status, plan, and the crew and orchestrator from the card. When the seat runs on a model other than the one its card says, it sets the crew that matches and tells its parent once.
- **No registered crew matches.** The chief tells the admiral once, leaves the field as filed and notes it on the record. It does not pick a near match.

### Record check

`scripts/record-check.sh <workspace selector> <task id>=<crew> …` reads each record and prints one line for each whose crew, orchestrator or tags differ from the roster. It is read-only, prints nothing when all match, and exits non-zero with one line on stderr when a read fails, as the two existing probes do.

The chief runs it at its first wake after staffing and before the close-out summary. Its output is in the probes' line format, so it could be registered as a room watch; that is not proposed here.

### House rules

An operator's own choices are not the product's: which model each rank runs on, a rank the operator added, a decision the operator made on a given day and the words it was made in. This skill ships in a public plugin and cannot carry a person's quoted words. That rule is §R26 of the agent-context standard from its version 4; this repository vendors version 3 and already follows it.

- **A house file holds them.** It is one document in the operator's own repository.
- **The instruction file names it.** When the instruction file of the checkout the chief runs in names a Formation house file, the chief reads it after this skill and writes its path on every seat card.
- **Order.** The admiral's current words, then the house file, then this skill's defaults. A house file changes models, names, line-ups and who the admiral has given a standing grant. Like a brief, it is a record of the admiral's choices and not a source of new authority.

The second copy of the skill is retired by placing every rule it holds: in a role file, in a shared reference, in the house file, or dropped with the reason. The placement is one table on the task that does it. Its "not yet verified" list is design material and moves to [the vision doc](../formation/3_vision.md).

### Skill check

`make ci-fast` gains one check of `plugin/skills/formation/`:

- `SKILL.md` is at most 16,000 bytes and 500 lines, and every reference at most 16,000 bytes (§R16).
- Every relative link resolves, and every file under `references/`, `scripts/` and `assets/` is linked from `SKILL.md` (§R8, §R17).
- Every role that `units.yaml` gives a session has `references/role-<role>.md`, and each role file has the six headings.
- Every role's record in `units.yaml` has a title, tags and a crew rule, or is `none`.

### Compatibility

- A formation closes at the end of its day, so no running seat holds an old brief when the change lands.
- `formation roster check`, mail and watches are unchanged.
- The orchestration skills are unchanged. A captain's claim step no longer depends on their filing table for the crew.

### Passed over

| Alternative | Why not |
| --- | --- |
| Keep the second copy as the one skill and retire the plugin's | It carries dated history and quoted words, and it is the copy that is over budget. The plugin is the one other operators and the Codex host install |
| One skill per role | Eight skills whose names and descriptions load in every session, and the rules all seats share would be copied into each |
| Keep the briefs and move only the shared rules | The briefs are where the two copies differ today, and a brief is lost to a summary at the first compaction |
| Fix the crew with one more sentence in the start step | The rule was already written once, in a file the step does not point at. The claim step and the check make it part of the path |

## Decided

The operator, 2026-11-27, in the advisor's chat:

1. **One skill, with a reference per role.** To a proposal naming this plugin as its home, the operator asked for the spec.
2. **The advisor keeps a record** in the coordination workspace, tagged `advisor`.

## Open

1. **Is the advisor a product role?** Recommended: yes, as an optional seat started on first need, with no authority row, reaching the chief only. Which model it runs on stays a house choice. The alternative keeps its role file in the house file's directory, which leaves one role outside the one skill.
2. **Who files the advisor's record?** Recommended: whoever starts it, as for a captain, so its id is on the card. An advisor the admiral opens directly, with no card, files its own on its first turn.

## Build

1. **Role files and seat card.** Write the eight role files from the briefs and the references they point at, the seat card, the `units.yaml` fields and the `SKILL.md` routing. Remove the briefs. Add the skill check and the evaluation cases.
2. **House file and the second copy.** Add the house-file step. In the operator's repository, write the house file, place every rule of the second copy, delete that copy and point its instruction file at this skill.
3. **Record check.** Add `record-check.sh` and its two uses in the chief's role file.

Phase 1 fixes the crew by itself: the filing step and the claim step both name it.

## Verification

| Result | Check |
| --- | --- |
| The skill is within budget and fully linked | The skill check passes in `make ci-fast`; it fails on a reference of 16,001 bytes, on a bundled file `SKILL.md` does not link, and on a role in `units.yaml` with no role file |
| A seat needs only its card, `SKILL.md` and its role file | Evaluation case: a session given a filled captain card states its first three actions and its authority rows, and opens no reference but `role-captain.md` |
| The chief still starts a shift | Evaluation case: "formation trio on this workspace" selects `role-chief.md` and `start.md`, files the records with a crew and fills a card per seat |
| Records carry the seat's crew | On the first formation day after phase 1, every record's history shows the crew as explicit at filing, and `record-check.sh` prints nothing |
| A wrong crew is reported | `record-check.sh` against a record whose crew was changed by hand prints one line naming the record, the crew found and the crew expected |
| No rule was lost | The placement table on the phase 2 task lists every heading of the second copy with where it went |

Not verified by a check: that a role file reads well to the model tier that uses it. Each is read once by a seat of that rank on its first formation day, and what it got wrong goes back as a change to the file. Whether a tracker's pilot redraws a crew that was set at filing was not observed: the one record filed that way left `proposed` before a pilot ran.
