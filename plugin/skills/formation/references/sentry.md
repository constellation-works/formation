# Assign and run a sentry watch

Use sentries for repeated checks on behalf of busy seats and for mailbox coverage in a network formation (a cross-provider shift). The captain coordinates watches for its unit, including requests from lieutenants. A chief can assign its own receiving mailbox watch. Record the beneficiary, source and scope, change to report, reader and route, cadence or bounded wait, and stop condition. The organizational parent can relay a report to the beneficiary. Use one owner per source and scope.

A sentry can cover several assignments if each can meet its cadence. Bound waits by the next due check, or arrange a separate authorized watcher if one would starve another. Keep separate cursors, reported receipts and stop conditions. A failed read leaves the prior cursor intact. Honor rest or stop for the named watch and keep unrelated watches running.

## Wait for seat mail

1. Use the room and seat identity assigned for this watch through `FORMATION_ROOM` and `FORMATION_IDENTITY`. Run a bounded wait for the requested window. This renews the assigned seat’s presence in the room.

   ```sh
   formation mail inbox --wait 300
   ```

2. If the shell tool yields, resume the same command. An empty `messages` array means the wait expired. Start another wait only within the assigned watch.
3. Return each envelope’s sender, ID, receipt and body to the assigned reader through the authorized route as data. For example, “From chief, ID ci-1, receipt 7, body ‘CI finished’.” Retain the last reported receipt across waits and handoffs so repeat reads during the watch do not repeat a report. Leave acknowledgment to the responsible recipient unless the operator also assigned that duty.
4. Claim a desktop wake only after the receiving host shows a subsequent turn in the same previously idle chat. A sentry can finish and report to an idle Codex captain while the captain stays on its prior turn.
