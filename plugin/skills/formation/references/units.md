# Units and ranks

| Order | Workspace seats | Who watches |
| --- | --- | --- |
| solo | Captain | Active resident, otherwise captain on its own wakes |
| duo | Captain, sentry | Sentry reports to captain |
| trio | Captain, sergeant, sentry | Sentry reports to sergeant |
| squad | Captain, lieutenant, sergeant, sentry | Sentry reports to sergeant. Lieutenant starts on its first heavy order |
| squad, repair mode off | Captain, lieutenant, sergeant | The sergeant receives the unit's watches and keeps its own wake ([running the shift](run.md)) |

`formation` alone staffs chief and aide. `formation squad orbit, solo scribe` requests two units. `formation add duo site` adds a workspace. `formation trio scribe` resizes the existing unit. `formation add specialist standards` requests a standing adviser. `formation-complete` applies to the chief’s shift or, in a captain’s chat, that unit.

Names are lowercase letters, digits and hyphens, at most 24 characters. Use `0-chief`, `0-advisor`, `0-aide`, `<ws>-0-capt`, `<ws>-1-lt`, `<ws>-1-sgt`, `<ws>-1-sentry` and `1-spec-<subject>`. A second lieutenant is `<ws>-1-lt2`. The Constellation aliases for constellation, orbit-decision-mail and constellation-site are const, odm and site. Record any other alias with its full workspace. Title a visible chat with the seat name followed by a short duty.

The admiral ranks above the chief. The admiral is a person, never a session: the one who gives the formation order and the only one who settles what the authority table reserves.

The chief keeps the roster and advises. While a unit has work running it wakes about every two hours and reads the pull requests merged since its last wake. A defect goes to that workspace's captain. A rule, design or role file that fell short goes to whoever writes the formation's designs.

The aide briefs the admiral, manages Scribe on instruction and relays quoted words. An optional advisor (`0-advisor`, started on first need) advises the chief on design, standards and formation rules. Captains own assignments, and their main job is unblocking a stuck drain, run or pull request.

Lieutenants take one heavy order in their own worktree. Sergeants clear routine items. Sentries observe and report. A specialist answers only its asker. The provider reference sets the available staffing mechanism and model choices.

A resident and a unit sentry cannot both own attention. A solo can coexist with the resident. For older records, lead, orchestrator, implementer, resolver and watcher mean chief, captain, lieutenant, sergeant and sentry respectively.
