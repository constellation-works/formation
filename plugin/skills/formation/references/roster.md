# Check a roster

1. Use the operator’s roster file as input to the offline checker.

   ```sh
   formation roster check --json < "$roster_file"
   ```

2. Report the counts from stdout. For example, `{"version":1,"seats":4,"routes":5}` means the draft has four seats and five directed routes. On failure, report the CLI diagnostic so the operator can correct the draft.

Keep the check offline. Joining seats and authorizing messages require a separate operator request.
