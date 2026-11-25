# Units and ranks

| Order | Workspace seats | Default workspace attention coverage |
| --- | --- | --- |
| solo | Captain | Active resident, otherwise captain on its own wakes |
| duo | Captain, sentry | Sentry reports to captain |
| trio | Captain, sergeant, sentry | Sentry reports to sergeant |
| squad | Captain, lieutenant, sergeant, sentry | Sentry reports to sergeant. Lieutenant starts on its first heavy order |

`formation` alone staffs chief and aide. `formation squad orbit, solo scribe` requests two units. `formation add duo site` adds a workspace. `formation trio scribe` resizes the existing unit. `formation add specialist standards` requests a standing adviser. `formation-complete` applies to the chief’s shift or, in a captain’s chat, that unit.

Names are lowercase letters, digits and hyphens, at most 24 characters. Use `0-chief`, `0-aide`, `<ws>-0-capt`, `<ws>-1-lt`, `<ws>-1-sgt`, `<ws>-1-sentry` and `1-spec-<subject>`. A second lieutenant is `<ws>-1-lt2`. The Constellation aliases for constellation, orbit-decision-mail and constellation-site are const, odm and site. Record any other alias with its full workspace. Title a visible chat with the seat name followed by a short duty.

The chief keeps the roster and advises. The aide briefs the operator, manages Scribe on instruction and relays quoted words. Captains own assignments. Lieutenants take one heavy order in their own worktree. Sergeants clear routine items. Sentries observe and report. A specialist answers only its asker. The provider reference sets the available staffing mechanism and model choices.

A watch can serve a captain, lieutenant or another assigned seat. Its beneficiary and reporting route are recorded separately from the default attention reader. In a network formation, assign a receiving mailbox watcher for each direction. A resident and a unit sentry cannot both own the same workspace attention feed. A solo can coexist with the resident. For older records, lead, orchestrator, implementer, resolver and watcher mean chief, captain, lieutenant, sergeant and sentry respectively.
