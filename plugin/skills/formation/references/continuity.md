# Refresh and replace a seat

Keep the chief’s open items, full options and recommendations on its assignment. Preserve a chat that holds the admiral’s conversation through compaction. A working captain or subordinate nearing 75% context, or likely to cross it before the next check, checkpoints early. Use the receiving provider’s supported mechanisms for context inspection and continuation.

A continuing handoff includes current orders and owners, verified state, next action starting with the watch, pending admiral decisions, group and seat, task, card path (`<seat_dir>/card.md`) and route IDs. Write a Scribe handoff in continue mode when available, without inventing a quote from the admiral. Send the handoff ID to the parent and record it on the assignment. If a write is refused, put the full handoff on the task and point to it. Stop the sentry watch before a refresh.

On resumption, read `<seat_dir>/card.md`, the role file it names and the handoff by ID, reconcile orders against the live records, confirm the group seat and reporting routes, and re-arm the watch once. Preserve the existing captain and assignment. Verify attachment and capture from the host. If Scribe lost the seat or refused an admiral-only continuation, report the refusal and give the admiral the supported continuation action.

Replace a captain only when refresh fails or the admiral orders replacement. Get their agreement and quote it on the record. The incoming captain reads the assignment and latest seat handoff in observe-only mode. Move the Scribe seat with `scribe group join <group> --as <seat> --replace` under the approved takeover. The outgoing captain stops its watch, redirects its reports and records STOOD DOWN.

The new one records TAKEOVER, verifies the routes and restarts the watch. The chief updates the roster and fallback check. Ensure attached workers survive the old session’s retirement. A captain replaces its own lieutenant.
