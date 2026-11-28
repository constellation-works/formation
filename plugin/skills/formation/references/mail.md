# Work with seat mail

Native channels stay first: use Claude session messages between Claude sessions and Codex collaboration tools inside a Codex tree. Use Formation mail for an edge that crosses providers, or when a headless script mails a seat.

A seat with a cross-provider edge runs `formation mail join` once, beside its `scribe group join`. In a Claude Code or Codex session with an open Scribe group, `formation mail join` records the room and seat from Scribe so later calls and hooks need no flags. Pass `"$seat" --room "$room"` when Scribe is unavailable or outside a session, and `--as <seat>` when calling outside a joined session.

`UserPromptSubmit` adds undelivered mail to the current turn. Hook delivery marks the envelope delivered. Use `formation mail inbox` and `formation mail ack "$receipt"` for a seat with no hook, such as a headless worker, or to read and acknowledge an envelope too large for the hook budget.

| Seat action | Command |
| --- | --- |
| Join from an open Scribe group (or pass `"$seat" --room "$room"`) | `formation mail join` |
| List every joined seat with its `undelivered` and `unacknowledged` counts, `held` and `seen` | `formation mail who` |
| Send the authorized text to the named seat (writes an envelope) | `formation mail send "$recipient" --id "$id" --body "$body"` |
| Answer an envelope you sent or received, linking the reply to its receipt (writes an envelope) | `formation mail send "$recipient" --re "$receipt" --id "$id" --body "$body"` |
| Inspect unacknowledged mail and leave it pending | `formation mail inbox` |
| Read recent room mail, newest first; a seat sees only its own (reads only) | `formation mail log` |
| Read one envelope and the rest of its thread; a seat is refused another seat's receipt (reads only) | `formation mail show "$receipt"` |
| Acknowledge handled mail (writes the receipt state) | `formation mail ack "$receipt"` |

- Mail commands print readable text on a terminal. A command run through a tool or script is piped: in automatic mode, `join`, `who`, `send`, `inbox`, `ack`, `log` and `show` print JSON there, and `sent` prints tab-separated rows.
- Add `--json` (or set `FORMATION_FORMAT=json`) to a call whose output you parse, such as the `receipt` from a send, the `held` and counts from `who`, or the `seat_dir` from `join`. `sent --json` returns its `messages`, `total` and `truncated` envelope. A refusal is one JSON line on stderr with `code` and `error` in that form, and `error: <message>` otherwise.
- For a send, use the admiral’s request or standing permission covering this recipient and content. If either is unclear, ask before sending.
- Choose a fresh ID for a new message. On a retry, reuse the same ID, recipient, body and reply receipt. For example, `formation mail send --json` returns `{"id":"ci-1","receipt":7,"held":false}`, which confirms storage. If the CLI refuses an `unknown_recipient`, report the refusal and the joined seats it lists. If it reports an ID conflict, reconcile the earlier send before sending again.
- Use `held` as a recent-presence signal. A receipt confirms that mail is stored. Check `formation mail who` for the seat’s current `held` status and its two counts.
- `mail who` counts two things per seat. `undelivered` is mail no hook has delivered and the seat has not acknowledged. `unacknowledged` is every envelope the seat has not acknowledged, delivered or not.
- Read `undelivered` to tell whether a hooked seat is being reached: 0 means every envelope reached its turn, and a count that stays above 0 means the hook is not delivering.
- A hooked seat is not expected to acknowledge, so its `unacknowledged` count only grows and says nothing about whether it received its mail. A seat with no hook has nothing delivered, so both counts match.
- Acknowledge only after the responsible recipient has handled the message and authorized marking it handled. Reading an inbox leaves the envelope unacknowledged.

## The admiral's seat

A mail call from a terminal with no agent session and no `--as` is the operator, the admiral.
If neither `--room` nor `FORMATION_ROOM` is set, Formation uses the one active room in the home.
A closed room is ignored. With no active room, or more than one, the command refuses with
`room_required`; the refusal names the active rooms.

`--room` and `FORMATION_ROOM` take precedence over this default. A session keeps the room from its
join, and `--as <seat>` still requires a room.
The active-room scan reads rooms without creating files.

Every room already has the reserved `admiral` seat; no session joins it. It has no presence, so `mail who` lists it with its counts and `held` null.

- Never pass `--as admiral` and never join `admiral`: inside an agent session both are refused with `reserved_seat`, and a roster that names the seat fails `roster check`.
- Only the seat holding the chief's rank in the applied roster runs `formation mail send admiral`. Any other seat is refused with `admiral_via_chief`: open a request for the chief with `formation request open` instead. A room with no applied roster refuses the send with `no_roster`.
- Mail with sender `admiral` and `"terminal": true`, which the hook labels as sent from a terminal, carries the operator's word with the same standing as their words in a chat. If one seems doubtful, ask the chief.
- The terminal is a signal, not proof of the person: a seat that relays an admiral answer still names its source.

## Answer and read the room

To answer an envelope you sent or received, send with `--re <receipt>`: `formation mail send <seat> --id <id> --body <body> --re <receipt>`. The receiving hook names the link, so the body does not restate the receipt. Lead the body with the outcome, because `mail log` shows each envelope's first line. The admiral can reply to any receipt in the room; a seat can reply only to receipts it sent or received.

Use `formation mail log` to read recent room mail. Its `RE` column names the answered receipt or the request that produced the envelope. `--threads` gives one row per thread, newest activity first, and `formation mail show <receipt>` prints the complete visible thread in receipt order. The admiral sees the whole room. A seat sees only the envelopes it sent or received, and `mail show` refuses any other receipt.

Log defaults to the newest 50 envelopes or threads. Use `--seat`, `--with`, `--since` (RFC 3339), `--limit`, `--no-watch` and `--full` to control the view. Both read commands support `--json` and leave delivery and presence unchanged.

## Wake an unheld Claude seat

Claude’s background `Stop` hook waits for 55 minutes (3,300 seconds) with one live waiter per seat. After it expires, stored mail waits for the seat’s next hook or inbox read.

If a send to a Claude seat returns `"held": false`, use an authorized native Claude session message to the same open chat with the single line `Formation mail waiting`. If you are a Codex captain or a process, route the wake request to the chief or the seat’s captain, who can send that native message. For a closed or unloaded chat, ask the admiral to open it and start a turn.
