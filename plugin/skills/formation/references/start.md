# Start the shift

1. Read the admiral’s unit/workspace pairs and named specialists, and establish context:
   - When the instruction file of the checkout the chief runs in names a Formation house file, read it after this skill (order: the admiral’s current words, then the house file, then this skill’s defaults; it grants no new authority).
   - Discover the actual owners and their registered crews with `orbit.workspace.list` and retain their host-qualified selectors.
   - Read the previous formation with `scribe group list` and `scribe group show <id>`, then the handoffs and Orbit records it names; for a still-open group use seat handoffs and report it, and with no group read the latest closed orchestration assignment per workspace.
2. Read `orbit.attention` with `limit: 1` for counts and degraded sources, then each resident checkpoint, deadline and open warn notices. Captains read the items. Hold staffing of a unit with a sentry until its resident is paused by the admiral or expired. If a resident starts during the shift, stand down the sentry before transferring attention.
3. In the host-qualified coordination workspace (Constellation uses `ws_constellation`), file each shift record with `crew` and `orchestrator` set together to the workspace’s registered crew matching the model that seat actually runs on (from step 1):
   - File the chief’s assignment via `orbit.task.add` with title `[orchestration]: formation <units> - <family>`, description, `acceptance_criteria`, complexity, tags `orchestration` and `formation-day`, and explicit `crew` and `orchestrator`; then claim the returned task ID in one `orbit.task.update` carrying `status: "in-progress"`, `plan`, `crew` and `orchestrator`.
   - File one proposed captain assignment per workspace via `orbit.task.add` titled `[orchestration]: <ws> - <family>` with description, `acceptance_criteria`, complexity, tags `orchestration` and `<ws>`, and explicit `crew` and `orchestrator`. Write the record ID and crew on the captain’s seat card; the captain claims it on its first turn in one `orbit.task.update` setting `status: "in-progress"`, `plan`, `crew` and `orchestrator` from the card.
   - If starting an advisor, file its proposed assignment via `orbit.task.add` titled `[advisor]: formation <group> - <family>` with description, `acceptance_criteria`, complexity, tag `advisor`, and explicit `crew` and `orchestrator`, and write the record ID and crew on its card; the advisor claims it on its first turn in one `orbit.task.update` setting `status: "in-progress"`, `plan`, `crew` and `orchestrator` from the card.
   - If no registered crew matches a seat’s model, tell the admiral once, leave the field as filed and note it on the record; never pick a near match.

   Use the installed `orchestration-session` and `orchestration-manager` procedures for those records.
4. Where Scribe is configured, open the shift group with the admiral’s actual typed line. Join once under the seat’s own name, author and rank. Carry its ID as the roster's room and in the seat cards. Group membership records member context under Scribe’s rules. Session capture is a separate admiral opt-in. If a quote or join is refused, report once and continue staffing. Retry opening on a valid typed turn. Repair the seat only when its recorded identity is established.

```sh
scribe group open --kind formation --project "$project" --title "$title" --related "task:$assignment" --quote "$typed_line"
scribe group join "$group" --as "$seat" --author "$model" --role "$rank"
```

   Every captain, plus any seat with a cross-provider edge or registering, receiving or reading watches (including `0-chief`, `0-aide` when relaying across providers, `0-advisor` and shared specialists), joins Formation mail once beside `scribe group join` so each seat gets its seat folder (`seat_dir`) and local mailbox. Same-provider seat messages keep native channels.

```sh
formation mail join --json
```

   Pass `"$seat" --room "$group" --json` if Scribe is unavailable. Read `seat_dir` from that JSON to save the seat card; a terminal shows a `Seat folder:` line instead.

   After `formation mail join --json` succeeds, run this route check in the assigned session. Include the join command and the check in each seat’s startup instructions.

```sh
env -u FORMATION_ROOM formation mail who
```

   - The check must list the assigned seat. Clearing `FORMATION_ROOM` makes `mail who` use the session route that the hook reads.
   - If the join or route check fails, report a startup failure and repair it before the seat takes work. A passing check after a failed join can reflect an earlier route.
   - For `seat_dir_unavailable` on a legacy seat folder, follow the mode `0700` correction and retry. For `session_route_unrestored`, repair the named session pointer and join again.

5. Before briefing any seat, write and apply one version 2 roster for the full lineup:
   - In `roster.json`, include each seat's `name`, `role`, `host` (`claude-desktop`,
     `codex-desktop` or `headless`), `workspace`, `reports_to` parent, `model`, `effort` and
     `record` ID (`null` if it has no record), plus `routes` as `from`/`to` seat pairs (`[]` if none).
   - Apply it with `formation roster apply --room "$group" --json < roster.json`. This stores
     the roster in the room; a check alone does not apply it or grant authority.
   - If a seat or unit is added later, edit this file and apply the full roster again.
6. When the house file names a grants file, load it now that the roster is applied: `formation grant load --room "$group" < <grants file>`.
   - The load stores the file's standing grants and withholds as house entries and adds the file's classes. It replaces the entries an earlier load stored, leaves the room's day grants alone, and changes nothing when the file is already loaded.
   - A standing grant changes only through that file, which the admiral approves as a diff.
   - A file cannot grant `grant` or `credentials`, remove or change a class, or name an unknown version. Report a refusal by its code and continue without the file.
7. Carry open requests from the previous formation into this room: `formation request carry --from <previous group id>`. It copies each open request with its id and history, skips settled ones, and maps a holder missing from the new roster to the chief. Then `formation request list --waiting admiral` gives the queue to report in step 10.
8. The chief runs `formation grant list` and records each active grant or withhold on the shift assignment, including its class, effect, target, workspace, limit, `entered_by` and `source`. `entered_by` says who entered it; `source` says where and when the admiral gave it. For each seat, fill the card's `settles` line from `formation grant list --seat <seat>`. After grant-change mail, refresh the affected seat's list before briefing it.
9. Use the provider procedure. Fill one seat card (`assets/seat-card.md`) for the aide, each captain, any advisor and each named specialist with the absolute `skill` path, `house rules` path (or `none`), role file, unit, group/room, record ID and crew, workspace selector, repo and base branch, parent, allowed recipients, repair mode, attention ownership, effective `settles` list from the command above, handoff summary and open-item counts.
   - A captain owning its workspace’s attention registers `attention` (when Orbit is configured) and `ci` in the room watch set and starts `formation watch run` in the background; other captains join `formation watch run` as standbys through the shared room.
   - A captain in a resident-owned workspace avoids duplicate attention monitoring and registers no `attention` watch while the resident owns attention.
   - Keep the chief’s fallback probes unconfigured until the seat-network design settles their recipient (`0-aide` vs `0-chief`).
   - A captain fills seat cards for its own unit (carrying its card’s `house rules` path or `none`), starting the sergeant before the sentry; a lieutenant starts only when ordered. Name any missing integration and what the seat can still do.
10. Record room `$group` on the chief’s assignment. Keep the full roster in the room; stamp the roster comment in UTC with the group, seat names, host IDs, reporting lines, models/efforts and task IDs.
    Verify watch coverage with `formation watch list` (or `--json`); record each captain’s watches and reader state on the chief’s assignment for close-out. Apply the provider’s reporting mode and tell the admiral who started, what carried over (including open requests), and what awaits them.
