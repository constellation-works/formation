# Check a roster

1. Use the admiral’s roster file as input to the offline checker.

   ```sh
   formation roster check --json < "$roster_file"
   ```

2. Report the counts from stdout. For example, `{"version":1,"seats":4,"routes":5}` means the draft has four seats and five directed routes. On failure, report the CLI diagnostic so the admiral can correct the draft.

Keep the check offline. Joining seats and authorizing messages require a separate admiral request.

## Apply a roster

Version 2 adds the `advisor` role and optional `reports_to` seat name. Omit
`reports_to` for a seat reporting to the admiral; parents must exist and the
reporting lines must not cycle. Version 1 remains valid for offline checking.

When applying the roster is authorized, run:

```sh
formation roster apply --room "$room" --json < "$roster_file"
```

Apply requires version 2 and an existing room. It replaces the prior roster,
reports seat and route counts, and leaves session bindings, mail and watches intact.
It grants no message routes or authority.

## View a room

Use `formation roster show [<room>] [--json]` to see reporting lines, seat
metadata and live presence with undelivered mail. Omit the room to use the
calling session's joined room. Planned seats that have not joined and joined
seats absent from the roster are identified. This read does not renew presence.
Version 2 seats accept optional `model`, `effort` and `record` strings.
