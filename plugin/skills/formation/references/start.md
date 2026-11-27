# Start the shift

1. Read the admiral’s unit/workspace pairs and named specialists. Discover the actual owners with `orbit.workspace.list` and retain their host-qualified selectors. Read the previous formation with `scribe group list` and `scribe group show <id>`, then the handoffs and Orbit records it names. For a still-open group use seat handoffs and report the open group. With no group, read the latest closed orchestration assignment per workspace.
2. Read `orbit.attention` with `limit: 1` for counts and degraded sources, then each resident checkpoint, deadline and open warn notices. Captains read the items. Hold staffing of a unit with a sentry until its resident is paused by the admiral or expired. If a resident starts during the shift, stand down the sentry before transferring attention.
3. In the host-qualified coordination workspace (Constellation uses ws_constellation), open the chief’s assignment titled `[orchestration]: formation <units> - <family>` with orchestration, formation-day and no-drain tags. File one proposed captain assignment per workspace tagged orchestration, no-drain and the workspace. Include complexity. The captain claims it with its plan and in-progress status together.

   Use the installed orchestration-session and orchestration-manager procedures for those records.
4. Where Scribe is configured, open the shift group with the admiral’s actual typed line. Join once under the seat’s own name, author and rank. Carry its ID in the roster and briefs. Group membership records member context under Scribe’s rules. Session capture is a separate admiral opt-in. If a quote or join is refused, report once and continue staffing. Retry opening on a valid typed turn. Repair the seat only when its recorded identity is established.

```sh
scribe group open --kind formation --project "$project" --title "$title" --related "task:$assignment" --quote "$typed_line"
scribe group join "$group" --as "$seat" --author "$model" --role "$rank"
```

   Every captain, plus any seat with a cross-provider edge or registering, receiving or reading watches (including `0-chief`, `0-aide` when relaying across providers, and shared specialists), joins Formation mail once beside `scribe group join`. Same-provider seat messages keep native channels.

```sh
formation mail join
```

   Pass `"$seat" --room "$group"` if Scribe is unavailable.

   A join is complete only when it exits 0 and the seat’s own session can then reach its room. After the join, run this check in the assigned session, with no `--room` or `--as`, and send it with the join line in each seat’s brief.

```sh
env -u FORMATION_ROOM formation mail who
```

   - Both must hold: the join exited 0 and the check lists the seat. After a failed join, a passing check shows only an earlier route.
   - A failed join or any refusal of the check, including `room_required` and `seat_required`, is a failed or partial join: report a startup failure and repair it before the seat takes work.
   - Keep `env -u FORMATION_ROOM`. An inherited room override stands in for a missing session pointer in `mail who`, and the hook ignores it.
   - A seat listed with flags, with that variable set or from another session proves nothing: it can be registered with no route.
   - A join that exits nonzero changes nothing, so retry it once the cause is fixed. For `seat_dir_unavailable` on a legacy seat folder, set it to mode 0700 as the error says. `session_route_unrestored` is the one exception: the session pointer it names may have changed, so fix that path and join again.

5. Use the provider procedure. Brief the aide, each captain and each named specialist with background, order, durable record, previous handoff, roster routes, provider, actual model, group, mail join line, `{ws_selector}`, `{repo}`, `{base_branch}`, attention ownership and close trigger.
   - A captain owning its workspace’s attention registers `attention` (when Orbit is configured) and `ci` in the room watch set and starts `formation watch run` in the background; other captains join `formation watch run` as standbys through the shared room.
   - A captain in a resident-owned workspace avoids duplicate attention monitoring and registers no `attention` watch while the resident owns attention.
   - Keep the chief’s fallback probes unconfigured until the seat-network design settles their recipient (`0-aide` vs `0-chief`).
   - A captain briefs its own unit, starting the sergeant before the sentry; a lieutenant starts only when ordered. Name any missing integration and what the seat can still do.
6. Stamp the roster comment in UTC with group, seat names, host IDs, reporting lines, models/efforts and task IDs. Verify starting watch coverage with `formation watch list` (or `--json`), record each captain’s watches and reader state on the chief’s assignment for close-out comparison, apply the provider’s reporting mode, and tell the admiral who started, what carried over and what awaits them.
