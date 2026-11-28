# Checking a shift roster

Run `formation roster check < roster.json` for a count of seats and directed routes, or add `--json` for machine-readable output. It accepts one UTF-8 JSON document up to 256 KiB. Success exits 0. Invalid documents exit 1 with a diagnostic on stderr. Invalid CLI arguments exit 2.

[The example](../examples/roster.json) contains a Claude chief and aide, a Codex captain, and a headless sentry. Version 1 requires `version`, `seats` and `routes`. Each seat has a unique case-sensitive `name`, `role`, `host`, and optional `workspace`. Captains require a workspace and each workspace can have one captain. Names and workspace IDs use ASCII letters, digits, hyphens and underscores. Hosts are `claude-desktop`, `codex-desktop` and `headless`. Roles are chief, aide, captain, lieutenant, sergeant, sentry, specialist and worker. Version 2 also accepts advisor and optional `reports_to` on each seat: a parent seat name, or absent for a seat reporting to the admiral. Parents must exist and reporting lines must be acyclic. The seat name `admiral` is reserved for the operator and refused with `reserved_seat`.

Apply a version 2 roster to an existing room with `formation roster apply --room <room> < roster.json` (add `--json` for counts). Applying replaces the previous roster atomically, without changing joined sessions, mail or watches. Version 1 is accepted by check but refused by apply. Checking stays offline and creates no room state.

Each route has a `from` and a `to` seat. Add the reverse pair explicitly when replies are wanted. Both ends must exist and differ. The checker rejects duplicate routes and unknown fields. A roster can describe a single unit or a shift. It contains no session credentials or live bindings.

## Plugin package

Run `make update-plugins` from the checkout with Cargo, Codex and Claude Code on PATH. It installs the Formation CLI into `~/.cargo/bin` and installs the [Formation skill](../plugin/skills/formation/SKILL.md) and provider hooks ([Claude hooks](../plugin/hooks/claude.json), [Codex hooks](../plugin/hooks/codex.json)) in both clients from this checkout. The skill handles shift staffing, role references, seat cards, escalation and close-out. It selects the receiving provider’s session tools for each seat. Keep the checkout in place and the binary directory on PATH. Codex asks for one review and trust approval whenever a hook file changes. Rerun `make update-plugins` after pulling changes, then open a new chat in each client: a hook change takes effect only in a new chat (an already open chat keeps using the hooks loaded when it started and can keep calling a removed hook).

For CLI-only use, run `make install`. Checking a roster works offline without a provider account, Orbit or a broker. To try the plugin in a disposable Claude Code session, launch `claude --plugin-dir ./plugin` from the checkout.

## Local inbox

Build the CLI with `make build` and use `target/debug/formation`, or use the installed binary. A room is selected by `--room <name|path>` (or `FORMATION_ROOM`), accepted before or after the subcommand. A value with a path separator or leading `.` or `~` is a directory path; any other value (1 to 96 ASCII letters, digits, hyphens or underscores) resolves to `<home>/rooms/<name>/`, where `<home>` is `FORMATION_HOME` or `~/.formation`. `mail join` creates a missing room (directory mode 0700, database mode 0600); every other command refuses a missing room with `room_not_found` and creates nothing. `mail join` also creates the seat's folder, `<room directory>/seats/<seat>/` (mode 0700), and reports its absolute path (`Seat folder:` in the readable form, `seat_dir` in JSON). A seat keeps its probe scripts and its held, mute and log files there, so clearing scratch or a checkout does not remove them. Formation never reads or deletes the folder's contents.

A join is all-or-nothing. It checks the folder first, then binds the seat and records the session pointer in one step. Except for `session_route_unrestored`, below, a join that exits nonzero leaves no new seat, no new or changed session pointer and no change to a seat's owner, route or pending mail, so retry it once the cause is gone. Joins from one session run one at a time, whatever their rooms, under a lock on `<home>/sessions/<session id>.lock`, which stays on disk. A join waits up to 15 seconds for it and then refuses, naming the holder. A session pointer that is not a private regular file of at most 4096 bytes refuses the join before the seat is bound. If a failed join cannot put the previous pointer back, it reports `session_route_unrestored` with the pointer's path and both failures; fix that path and join again. A join that is killed before it finishes can leave the pointer naming its room, and joining again repairs it. A folder that is a symlink, belongs to another user or is open to group or other is refused with `seat_dir_unavailable`. For a folder you own with a mode such as 0755, the error gives the exact path, the observed mode, the required mode 0700 and the `chmod` to run before you retry. The refusal never changes the folder or its contents.

In a Claude Code or Codex session, `formation mail join` looks up the session in open Scribe groups using `scribe group list --json`. It joins the matching seat in the room named for the group; `--room` overrides that room. Join the Scribe group first. If Scribe is unavailable or the seat is ambiguous, use `formation mail join <seat> --room <room>` instead. This lookup runs only at join, and the explicit form works without Scribe.

In a Claude Code or Codex session (`CLAUDE_CODE_SESSION_ID`, else `CODEX_THREAD_ID`), `mail join <seat>` binds that session id to the seat and records the room in `<home>/sessions/<session id>`. Later commands in that session find both the room and the seat with no flags. A seat listed by `mail who` run with `--room` and `--as` may not have this route, and `FORMATION_ROOM` stands in for a missing pointer in a command while the hook ignores it. After a join succeeds, check the route from that session with `env -u FORMATION_ROOM formation mail who`. Joining a seat already held by another session takes it over and reports `rebound: true` with `replaced_session`; the replaced session's next call without `--as` is refused with `seat_required`. Outside a session (or to select another seat explicitly), pass `--as <seat>`:

```sh
formation mail join chief --room trial
formation mail join captain --room trial
formation mail who --room trial --as chief
formation mail send captain --id check-1 --body 'CI finished' --room trial --as chief
formation mail inbox --wait 30 --room trial --as captain
formation mail ack 1 --room trial --as captain
```

### The admiral at a terminal

The operator at a terminal is the admiral. A `mail`, `request` or `grant` call with no agent session variable, no `--as` and a terminal attached runs as the `admiral` seat, so `formation mail inbox --room trial`, `send`, `ack`, `sent`, `who`, `request settle`, `request reopen`, `grant add`, `grant withhold`, `grant revoke` and `grant load` need no seat flag. For a mail call, if neither `--room` nor `FORMATION_ROOM` is set, Formation reads the rooms under the home and uses the one whose recorded status is active. Closed rooms are ignored. With no active room, or more than one, the call is refused with `room_required` and the refusal names the active rooms. `--room` takes precedence over `FORMATION_ROOM`, and both take precedence over the default. A `request` or `grant` call does not scan for the active room: without `--room`, `FORMATION_ROOM` or a joined session's room, it is refused with `room_required`. A session still uses the room from its join, and `--as <seat>` still requires a room. The active-room scan is read-only and does not create files. Formation counts a call as attached when its stdin, stdout or stderr is a terminal, so `formation mail inbox | less` qualifies. A script whose three streams are all redirected, such as one run by cron, is still refused with `seat_required`, and so is a call from an agent session that joined no seat. A call that passes `--as <seat>` behaves as before. Outside an agent session, `--as admiral` is the same as leaving `--as` out; inside one it is refused with `reserved_seat`. `watch` and `roster` commands do not take the admiral as their caller yet.

Every room has the `admiral` seat from its creation, and a room made by an earlier build gains it on its next write, such as the next `mail who`. No session joins it: `mail join admiral` is refused with `reserved_seat`, and `roster check` refuses a roster that names a seat `admiral`. The seat has no presence, so `mail who` lists it with its two counts, `held` null (`-` in the readable form) and `seen` null; `roster show` and `roster list` leave it out of their joined seats. Only the seat that holds the chief's rank in the applied roster mails the admiral. `mail send admiral` from any other seat is refused with `admiral_via_chief` and stores nothing; the other seats reach the admiral with a request. A room with no applied roster has no known chief and refuses the send with `no_roster`. When the chief passes (or opens) a request that waits on the admiral, the admiral's seat becomes its holder and receives one envelope naming the request; settling the request acknowledges that envelope. An envelope sent from the terminal has sender `admiral` and `"terminal": true` in `mail inbox --json`, and both the readable inbox and the hook delivery say it was sent from a terminal. Every mutating admiral call, plus `mail inbox` and `mail who`, records an audit event with `"caller": "admiral"` and `"terminal": true` and stable identifiers only; read-only views (`mail sent`, `request list`, `request show`, `grant list`, `grant check`) write nothing.

This is a signal, not a lock. Any process under the same OS account that drops its session variables and holds a terminal resolves as the admiral. The reserved name stops a seat from taking the name by mistake, and the audit trail records each admiral call so that the operator can check it against what they did.

Every mail command reads in one of two forms. On a terminal, `join`, `who`, `send`, `inbox`, `ack` and `sent` print readable rows or a one-line confirmation, and a refusal prints `error: <message>` on stderr. In automatic mode, when stdout is piped or redirected, `join`, `who`, `send`, `inbox` and `ack` print one JSON document on stdout, and `sent` keeps its tab-separated rows. A refusal prints one JSON line with `code` and `error` on stderr for every verb. A refusal exits 1 and leaves stdout empty in every form. Formation picks the form once per command: `--json` wins, then `FORMATION_FORMAT` (`auto`, `human` or `json`), then whether stdout is a terminal. An unset or unrecognized `FORMATION_FORMAT` means `auto`. Scripts and agent sessions that parse mail output should pass `--json`, which is accepted before or after the verb. For the five verbs other than `sent` it gives the same document as the piped form: for example, `formation mail join --json` reports `seat_dir`, and `formation mail send --json` reports `receipt`. For `sent`, it returns the `messages`, `total` and `truncated` envelope described in the `mail sent` paragraph below. The hook commands are unaffected: their output follows each host's protocol. `mail send` stores an envelope for any joined seat, present or not, and stamps it with the time it was stored. Its reply carries `held`, which is true when the recipient renewed its presence in the last 90 seconds. A waiting inbox or Stop hook renews it every 15 seconds. A name that never joined is refused with `unknown_recipient`, and the message lists up to 32 joined seats. Each inbox envelope carries `sent` and `delivered` as RFC 3339 UTC times. `sent` is null for mail stored by an earlier build, and `delivered` is null until a hook hands it over. `mail who` lists every joined seat as `name`, `undelivered`, `unacknowledged`, `held` and `seen` (an RFC 3339 UTC time, or null for a seat that has not run a command since it joined). `undelivered` counts the seat's unacknowledged envelopes that no hook has delivered. `unacknowledged` counts every envelope the seat has not acknowledged, whether or not a hook delivered it. Read `undelivered` to see whether a hooked seat is being reached: a hook delivers mail without an acknowledgment, and a hooked seat is not expected to acknowledge, so its `unacknowledged` count grows with the mail it has received. `mail inbox` lists the unacknowledged envelopes. Joining does not renew presence. A waiting inbox checks SQLite every half second inside one bounded command. An empty array means the wait expired. Each read returns up to 32 unacknowledged messages, oldest first. A recipient acknowledgment is repeatable and retains the stored envelope. A retry with the same sender, ID, recipient and body returns the original receipt, even after acknowledgment or expiry of the recipient's presence. Its `held` reflects the recipient's presence at the retry. A changed payload using the same ID fails.

`formation mail sent` lists the caller's envelopes newest first, up to 32 by default. Use `--limit <n>` to change that limit and `--json` for a document with `messages`, `total` and `truncated`; a pipe without `--json` gets tab-separated rows. Each record shows its receipt, id, recipient, sent time and state. `stored` means the envelope is queued, `delivered` means a hook handed it over, and `acked` means the recipient acknowledged it.

The room is for cooperating processes on one machine under one OS account. Any joined seat can send to any joined seat in that room. The seat name is an accident guard and not a security boundary: any process under the same OS account can pass `--as`, read the database or act as any seat, so no authority or access control depends on a seat name. The CLI requires private directories (mode 0700) and private files (mode 0600). Keep the room on a local filesystem. The roster validator is independent of room membership.

Existing rooms from earlier builds open in place and keep their seat names, messages and receipt numbers on the first database write. Early builds could create `inbox.sqlite3` with mode 0644 inside a private room. The shared SQLite driver requires an owner-only file. If the diagnostic identifies that permission issue, run `chmod 600 "<room>/inbox.sqlite3"` for your own room and retry. Newly created databases already use mode 0600.

## Hooks

The host pipes its event JSON to the command on stdin (up to 1 MiB). Use the pair for your host.

| Host | UserPromptSubmit command | Stop command |
| --- | --- | --- |
| Claude Code | `formation hook --provider claude --event user-prompt` | `formation hook --provider claude --event stop --hold 30` |
| Codex | `formation hook --provider codex --event user-prompt` | `formation hook --provider codex --event stop` |

The hook looks up the mailbox from the event's `session_id`. Unjoined sessions and events carrying `agent_id` exit silently. UserPromptSubmit marks the binding busy and adds unread mail to the current turn. Claude Stop uses `asyncRewake` with `--hold 3300` and a host timeout of 3600 seconds. A positive-duration Claude Stop acquires one per-seat waiter lease in the room (45-second TTL, renewed every 15 seconds); a concurrent or later Stop seeing a live lease exits silently without replacing the active waiter or consuming waiting mail. When mail arrives the live waiter emits the envelope on stderr, releases its lease and exits 2. Codex Stop marks the binding idle, then checks for mail without waiting. If mail is waiting, it continues the turn with a block decision and marks the binding busy. The shipped plugin gives Codex Stop and both prompt hooks 60 seconds to run and exits 0 silently when `formation` is absent from PATH. The CLI default hold is zero seconds. An expired hold releases its lease and exits 0 with empty output. Errors exit 1 with one diagnostic line on stderr.

After storing new mail, `mail send` and the watch reader can queue a wake for an idle Codex seat with a current hook binding. The sender needs `codex` on PATH and access to the recipient's local Codex queue under the same OS account. The command uses plain `codex queue` and submits one fixed line stating that Formation queued it and the operator did not type it. The envelope stays in the room until the receiving hook reads it. If queue submission fails or the seat has no observed provider state, mail remains available at its next normal hook. Formation limits queue attempts to one per observed idle period. If Codex Desktop is closed or the target chat is unloaded, a successful queue submission may wait until the app reopens and loads that chat.

Hooks deliver whole envelopes in receipt order within the host's character budget (9,000 for Claude, 8,000 for Codex). If mail remains, the last line gives the waiting count. Use `mail inbox` to read an envelope too large for the hook. Hook delivery leaves the envelope in the inbox until `mail ack`. The delivered time records the command's handoff and remains set if the host fails to consume the output.

## Watching

Add probes to the room's watch set and run a shared reader.

```sh
formation watch add ci --every 300 -- plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main
formation watch add release --to captain -- ./scripts/release-read.sh
formation watch run
```

A watch is keyed by its recipient and name, so two seats can each have a `ci`. `--to` names a joined seat and defaults to the caller. Names use 1 to 32 lowercase ASCII letters, digits or hyphens. The interval defaults to 120 seconds and must be at least 10. One word after `--` is shell text as written. Several words are quoted so each stays one argument. `add` replaces an existing watch, switches it on and keeps its remembered fingerprints. `remove` forgets them.

```sh
formation watch pause ci
formation watch resume ci
formation watch remove release --to captain
formation watch list
formation watch list --json
formation watch stop
```

`watch run` keeps one active lease per room. Extra invocations stand by. The reader renews its lease every 15 seconds and a standby can take over when a heartbeat is 45 seconds old. Changes to the set take effect on the next pass. A watch runs through `sh -c` in the directory it was added from, using the active reader's environment. Due probes run concurrently and each has a 60-second limit. `watch stop` marks the room stopped and ends the reader and its standbys. A later `watch run` clears the mark.

`list` prints the reader state line followed by a table of watches:

```text
reader: orbit-0-capt, pid 4711, last beat 2026-11-26T04:00:00Z; standby: scribe-0-capt, pid 4820, last beat 2026-11-26T04:00:02Z
NAME       TO             EVERY (s)  STATE   LAST READ             RESULT       SENT  ADDED BY
attention  orbit-0-capt         120  on      2026-11-26T03:59:30Z  ok, 3 items     5  orbit-0-capt
ci         orbit-0-capt         300  on      2026-11-26T03:58:10Z  read failed     0  orbit-0-capt
release    scribe-0-capt         60  paused  2026-11-26T02:00:00Z  ok, 0 items     1  0-chief
```

`list --json` exposes the same fields with `reader.state` set to `live`, `none` (printed as `reader: none` or `reader: none since <time>`) or `stopped` (`reader: stopped since <time>`). Timestamps are RFC 3339 UTC. `list` works against older rooms without upgrading them.

If the room has at least one `on` watch, `reader.state` is `none` and `watch stop` was not run, the next hook event for each seat that receives or added an `on` watch delivers a one-time notice: `watch: nothing has read this room's <n> watches since <time>. Start a reader: formation watch run`. Restart a reader from a live seat's shell to clear the gap.

A probe prints one `<fingerprint><TAB><text>` per item. Fingerprints use 1 to 200 characters with no tab. Text past 400 characters is cut to end in `…`. The reader mails new items as `<probe> | <text>` (up to 20 lines per envelope) under the heading `Watch <name>, added by <seat>: probe output, not a seat's request.`, marking it as probe data rather than a seat's request. Fingerprints omitted from two consecutive complete successful reads are forgotten so a returning condition is mailed again.

If a probe exits non-zero, exceeds the 60-second limit, emits a malformed line, or lacks a tool or token in the active reader's environment (even if `watch check` passed in the caller's shell), `list` records `read failed` in `RESULT`. The reader mails `<probe> | read failed: <first diagnostic line>` once until a successful read recovers; failed reads do not forget remembered fingerprints. The room stores handover state per recipient and watch, so a standby reuses it when it takes over.

### Checking a probe

`watch check -- <command>` runs a probe once from your current directory under the reader's 60-second limit and prints its items as `{"items":[{"fingerprint":...,"text":...}]}`. It changes nothing: no watch is added, the reader's lease is not taken, no fingerprint is remembered and no mail is sent, so the first real read still sends every item. It needs a joined seat, as `list` does. A malformed line, a failed run and a run past the limit exit 1 with a JSON error on stderr. The error gives the reason: the line number for a malformed line, and the first diagnostic line for a failed run. An empty result exits 0 with `{"items":[]}`.

```sh
formation watch check -- plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main
```

### Blocking wait

`watch wait` runs caller-supplied probes from your current directory and returns new items or failures as JSON. It uses the calling seat's handover state, returns at most 20 items per call and reports any remainder. It polls every 120 seconds by default. After `--timeout` seconds (default 300) with nothing to report, it returns empty lists with `"expired":true`. Use `--timeout 0` for a single pass.

```sh
formation watch wait \
  --probe "ci=plugin/skills/formation/scripts/ci-read.sh owner/repo agent-main" \
  --timeout 300 --room trial --as sentry
```

## Delegated authority grants

Apply a version 2 roster to the room before managing authority grants; every `grant` command in a room with no applied roster refuses with `no_roster`.

Formation ships a built-in catalog of twelve decision classes: `repair` (sergeant, or the workspace captain where there is no sergeant), `ship` and `land` (captain), `staffing` (chief), `design`, `breaking`, `direction`, `release` and `operator` (admiral), `override` (nobody), and `credentials` and `grant` (admiral; never granted).

```sh
formation grant add --to chief --class direction --workspace all \
  --limit "a setting inside one workspace that a later change can undo" \
  --for day --source "advisor chat 2026-11-27 12:00"
formation grant withhold --to captain --class land --workspace site \
  --for day --source "advisor chat 2026-11-27 12:30"
formation grant revoke 1
formation grant list
formation grant list --seat chief --json
formation grant check --seat chief --class direction
```

`add` and `withhold` accept `--to` (a rank name or a seat in the applied roster), `--class`, `--workspace` (a workspace identifier or `all`, defaulting to `all`), optional `--limit`, `--for day` (default `day`; standing grants come from a grants file), and `--source` (required from a joined seat; optional from a terminal, where it defaults to `admiral`). `add` and `withhold` refuse `grant` and `credentials` with `class_not_grantable` and unknown classes with `unknown_class`.

`grant list` without `--seat` prints active room grants and withholds; `--seat <seat>` prints what that seat settles now and any withheld entries, naming whether each line came from the `catalog`, a `grant` or a `withhold` (add `--json` for machine-readable output). Precedence follows seat over rank, a named workspace over `all`, room over house file over catalog, later over earlier, and a withhold over a grant entered at the same time. `grant check --seat <seat> --class <class> [--workspace <workspace>]` exits 0 with the covering JSON entry when the seat may settle that class in the workspace (defaulting to the seat's workspace, or `all`) and exits 1 with `not_authorized` otherwise.

`add`, `withhold`, `revoke` and `load` accept either a joined seat (from the session binding or `--as`) or the admiral from a terminal with no agent session and no `--as`, and record the caller as the entering (or revoking) seat. A grant entered by the admiral from a terminal records `admiral` as its entering seat and default source, is never marked `self-entered`, and marks its notification envelopes `terminal: true`. A non-terminal call with no seat, a session that has not joined, or a seat that has not joined is refused with `seat_required` and changes nothing; a script with no terminal is a missing identity, not an admiral. A grant entered by a joined seat whose holder is the caller's own seat or rank is marked `self-entered` in `grant list`. Every `add`, `withhold` and `revoke` mails one envelope to each joined seat whose effective authority changed and none to seats whose effective authority did not change. Audit events record only stable identifiers (never limit text or source).

## Delegated authority requests

Apply a version 2 roster before using `formation request`. Every request command returns
`no_roster` in an existing room without one. Writes act as the session's joined seat or
the joined seat selected with `--as`, except for `carry` and for `settle` and `reopen`
called by the admiral from a terminal. Use `--room` to select a room explicitly. Add
`--json` to any request command for machine-readable output and stable refusal codes.

```sh
formation request open --class design --record '<task or pull request>' \
  --ask 'Should we share the cache?' \
  --option 'a: Share the cache' --option 'b: Keep separate caches' --recommend b --blocking
formation request advise <request> --recommend a --note 'Both jobs use the same key.'
formation request settle <request> --option b --note 'Keep the jobs independent.'
formation request settle <request> --option a --admiral --source 'chief chat, 10:30'
formation request reopen <request> --admiral
formation request carry --from yesterday
formation request pass <request> --recommend b
formation request list --waiting admiral
formation request list --settled-by chief --since 2026-01-01T00:00:00Z --json
formation request show <request>
```

`open` records a one-line ask, a record pointer, and two to four uniquely labeled options.
`--recommend` names one of the labels. `--blocking` records that the asker's work is waiting.
The ID includes the originating room's name. The applied roster supplies the asker's
workspace (`all` for a seat with no workspace) and parent. `--workspace <name>` names the
workspace the request concerns instead: `all`, or a workspace of any seat in the applied
roster, so an advisor or aide with no workspace can ask about one. An unknown name is refused
with `unknown_workspace`, leaving the request and mail unchanged. `--settle` and a later
`settle` check grants against that workspace. The holder and settlement authority are
unchanged. The holder receives one envelope as ordinary room mail. With `--settle <label>`,
the asker instead uses their own effective authority to open and settle in one step.

The chief holds every request a captain, advisor or aide opens. An explicit `reports_to`
is honored when it names a chief seat; with none, the sole chief in the applied roster
holds it. `open` is refused, leaving the request and mail unchanged, with `chief_required`
when the roster has no chief, `chief_ambiguous` when it has several and the asker names
none, and `request_parent_not_chief` when the asker's parent is not a chief seat. Other
seats (a sergeant, a lieutenant, a worker) keep their `reports_to` as holder, and a chief's
request with no parent waits on the admiral. `--settle` needs no holder and is not routed.

`advise` adds a recommendation and explanation without changing the state or holder.
`settle` requires a covering entry for the calling seat, class and request workspace.
`not_authorized` names the seats that hold the class in scope, or its default holder if no
seat holds it. The record preserves the chosen option, settling seat, grant ID and limit at
settlement. A catalog entry has a null grant ID in JSON. The asker receives one envelope.
From a terminal with no agent session and no `--as`, `request settle <request> --option <label>`
records the admiral's answer directly with no grant and no `--source`; `show` names `admiral`
as settler and records that the call came from a terminal (`"terminal": true` in JSON), the
asker receives one terminal-marked envelope, and any pending envelope for the request in the
admiral's mailbox is acknowledged. From a joined seat, `--admiral --source <text>` records
an answer the admiral gave in a chat, naming the entering seat and source, and also
acknowledges the request's envelope in the admiral's mailbox. `reopen` overturns a settlement,
returns the request to the admiral's seat, and mails the asker and the settling seat. From a
terminal it needs no `--admiral`; a joined seat must pass `--admiral`. A non-terminal call
with no seat or an unjoined session is refused with `seat_required`, leaving the request and
its mail unchanged. `reopen` changes the record only and does not undo external actions.
Overturned settlements remain visible in `show` and `list --settled-by`.

`carry --from <room>` copies open requests into the current room with their IDs and history. It skips existing IDs and settled requests. Holders missing from the new roster map to its chief, while requests held by `admiral` stay with the admiral.

`pass` requires the current holder and sends the request to that seat's parent. A pass by the
chief (no parent) sets the admiral's seat as holder and stores one envelope in the admiral's
mailbox naming the request. Use `--recommend <label>` to add advice
while passing. `--note` is optional with a recommendation. Seat-entered request notifications use normal room mail. If a recipient seat has yet to join, the change is refused and can be retried after it joins. Admiral notifications are stored for the named seats, including seats that have not joined. Requests waiting on the admiral appear in both `request list --waiting admiral` and the admiral's `mail inbox`, and settling clears both.

`list` shows all requests newest first by default. Use `--waiting <seat>` for that seat's
open requests or `--waiting admiral` for the admiral's queue. Use `--settled-by <seat>`
for that seat's decisions. `--since` includes activity at or after an RFC 3339 timestamp.
With `--settled-by`, it compares the settlement time. `show` includes the original ask,
options and recommendation, the advice history, and any settlement.

### Viewing a roster

Run `formation roster show [<room>] [--json]` to compare planned seats with joined
seats. Without a room it uses the calling session's joined room. The tree shows
reporting lines, rank, workspace, optional model, effort and record, and live
presence and undelivered mail. Unjoined planned seats and joined seats absent
from the roster are identified. The command does not renew presence. A room
without an applied roster lists its joined seats. Version 2 roster seats may
include the optional string fields `model`, `effort` and `record`.

Run `formation roster list [--status active|closed] [--json]` to see rooms under
the Formation home, newest first. The opening time comes from the roster
application or the first recorded join. Rooms with neither timestamp appear
last. The list reports held planned seats over the planned count, or the
joined count when there is no roster. It also shows roster workspaces and the
latest presence across all joined seats. The command leaves the home
untouched and reports each unreadable room on stderr.

Run `formation roster close [--room <room>] [--json]` from a joined session to
record its seat and the time of closure. The first close is reported on
subsequent calls. A closed room shows each seat's last presence, and
`roster apply` returns `room_closed`. Mail and watches continue to work. Room
paths outside the home work with `show` and `close`.
