# Start the shift

1. Read the operator’s unit/workspace pairs and named specialists. Discover the actual owners with `orbit.workspace.list` and retain their host-qualified selectors. Read the previous formation with `scribe group list` and `scribe group show <id>`, then the handoffs and Orbit records it names. For a still-open group use seat handoffs and report the open group. With no group, read the latest closed orchestration assignment per workspace.
2. Read `orbit.attention` with `limit: 1` for counts and degraded sources, then each resident checkpoint, deadline and open warn notices. Captains read the items. Hold staffing of a unit with a sentry until its resident is paused by the operator or expired. If a resident starts during the shift, stand down the sentry before transferring attention.
3. In the host-qualified coordination workspace (Constellation uses ws_constellation), open the chief’s assignment titled `[orchestration]: formation <units> - <family>` with orchestration, formation-day and no-drain tags. File one proposed captain assignment per workspace tagged orchestration, no-drain and the workspace. Include complexity. The captain claims it with its plan and in-progress status together.

   Use the installed orchestration-session and orchestration-manager procedures for those records.
4. Where Scribe is configured, open the shift group with the operator’s actual typed line. Join once under the seat’s own name, author and rank. Carry its ID in the roster and briefs. Group membership records member context under Scribe’s rules. Session capture is a separate operator opt-in. If a quote or join is refused, report once and continue staffing. Retry opening on a valid typed turn. Repair the seat only when its recorded identity is established.

```sh
scribe group open --kind formation --project "$project" --title "$title" --related "task:$assignment" --quote "$typed_line"
scribe group join "$group" --as "$seat" --author "$model" --role "$rank"
```

5. Use the provider procedure. Brief the aide, each captain and each named specialist with background, order, durable record, previous handoff, roster routes, provider, actual model, group and close trigger. A captain briefs its own unit. Start the sergeant before the sentry. A lieutenant starts only when ordered. If an integration is absent, name the missing capability and what the seat can do.
6. Stamp the roster comment in UTC. Record the group, seat names, provider host IDs, reporting lines, models/efforts and task IDs. `scribe group show` identifies transcripts. A host session ID or collaboration ID is a separate address. Send each seat only its allowed routes, give the aide the captains’ routes and records, then apply the provider’s reporting mode. Tell the operator who actually started, what carried over and what awaits them.
