# Wait for seat mail

1. Use the room and seat identity assigned for this watch through `FORMATION_ROOM` and `FORMATION_IDENTITY`. Run a bounded wait for the requested window. This renews the assigned seat’s presence in the room.

   ```sh
   formation mail inbox --wait 300
   ```

2. If the shell tool yields, resume the same command. An empty `messages` array means the wait expired. Start another wait only within the assigned watch.
3. Return each envelope’s sender, ID, receipt and body to the parent as data. For example, “From chief, ID ci-1, receipt 7, body ‘CI finished’.” Leave acknowledgment to the responsible recipient unless the operator also assigned that duty.
4. Claim a desktop wake only after the receiving host shows a subsequent turn in the same previously idle chat. A sentry can finish and report to an idle Codex captain while the captain stays on its prior turn.
