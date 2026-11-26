---
title: Formation — Shift messaging
owner: codex
created_date: 2026-11-24
updated_date: 2026-11-26
last_validated: 2026-11-25
status: Draft
feature: shift-messaging
doc_role: scope
summary: Durable seat mail between Claude and Codex sessions, handed over by a Stop hook, with one watch command for everything that is not mail.
stage: building
next: Build the Codex queue wake, which replaces the Codex Stop hold.
---

# Formation — Shift messaging

Daniel approved this shape on 2026-11-25 ([Decided](#decided)). [Built today](#built-today) is in the repository. The rest is filed as tasks tagged `shift-messaging`, with the two hook trials first. The Codex hold passed on the tested build. The Claude idle-wake trial is next.

A formation on 2026-11-26 showed that the Codex hold keeps typed messages from the seat. Daniel approved [replacing that hold with a wake](#typed-input-during-a-codex-hold) the same day, after trial 5 passed. It is not built yet.

## Problem

Daniel talks mainly to a chief and an aide, and at times to the captain of a workspace. Claude sessions reach each other through the app's session messages, and a Codex parent reaches its children through the collaboration tools. Nothing crosses the two.

The formation of 2026-11-25 ran one Codex captain among Claude sessions on one Mac, and showed three costs:

1. **Chief to Codex captain.** The chief wrote a comment that began `FOR <seat>` on the captain's task record, and the captain found it when it next read the record. Two sessions on one machine talked through a tracker on another, and the record filled with routing notes.
2. **Codex captain to chief.** Nothing woke the chief. It read the captain's record at a fallback check twice an hour, so an answer could wait 40 minutes.
3. **The sentry polls in a model.** Each read is a turn even when nothing changed, which is why a pace script rations them ([Formation design](../formation/2_design.md#ranks)).

Daniel, 2026-11-25, in the chief's chat: "I think we can keep this every simple, with mailbox for each unit that requires it, and a subagent can poll the mailbox along with other things that requires watch (mostly for codex crew for our use-case - but can be used for ci/PR/etc). for the sender address, for formation, we can use active scribe group's unit name." And, since a Codex child cannot wake its parent: "then hook is the best option?"

## Goal

A Claude chief and a Codex captain on one machine exchange an ask and its answer without Daniel carrying it, without a task record carrying it, and without either session polling in a model. A CI failure reaches a captain the same way.

Out of scope: more than one machine, enforcing who may message whom, and any provider other than Claude and Codex.

## Shape

| Part | What it is |
| --- | --- |
| Mailbox | One for each seat that needs it, in one room per formation. Mail waits for a reader who is away |
| Address | The seat's name in the open scribe group. The room is named by the group's id |
| Delivery | Two hooks shipped in the plugin hand unread mail to the seat's own session. The Stop hook also holds the wait while the seat is idle |
| Watch | One command that blocks on any number of probes (CI, pull requests, attention) and mails what is new to a seat |

### Built today

An offline roster checker and a local SQLite inbox with session-bound seats, presence, idempotent sends, a bounded inbox wait and explicit acknowledgment. The `formation hook` command delivers mail from fixture host events, and `mail sent` reports the delivery state. The plugin hook files follow in a separate change. A room admits cooperating seats under one OS account. [Inbox usage](../../usage.md#local-inbox), [hook usage](../../usage.md#hooks)

### Mailbox

A seat needs a mailbox when one of its edges has no native channel. The app's session messages and the Codex collaboration tools stay in use where they exist, because they already wake the receiver and stamp the sender.

| Edge | Channel |
| --- | --- |
| Claude chief to Claude captain | Session message, as today |
| Codex captain to its Codex sergeant | Collaboration tools, as today |
| Claude chief to Codex captain, and back | Mail |
| A sentry or a script to its captain, on either provider | Mail |

One room per formation lives at `~/.formation/rooms/<room>/`. `mail join` opens the caller's mailbox once, beside its `scribe group join`. `mail send` stores an envelope for any open mailbox and refuses an address nobody opened, with the list of open ones. `mail inbox` and `mail who` read.

### Address

`mail join` with no name asks scribe which seat the calling session holds in the open group. The group's id becomes the room and the seat's name becomes the address. Formation records the session's id beside the seat, so every later call and every hook finds the seat from the session id alone. Scribe is consulted at the join and nowhere else, and `--room` with `--as` replaces it outside a formation.

- **Checked on 2026-11-25:** in the chief's Claude desktop session, `CLAUDE_CODE_SESSION_ID` held the id that scribe lists for the `0-chief` seat. Hook input carries a `session_id` on both providers; trials 1 and 2 confirm it is the same id.
- **The name is a label, not proof.** Any process under the account can pass `--as`. A message stays a request and never approval ([session network](../formation/references/network.md#rules), rules 1 and 2).

### Delivery

The plugin ships two hooks for each provider. Each is one line that calls `formation hook`, so the logic stays in the binary and the hook file seldom changes.

| Hook | What it does |
| --- | --- |
| On the user's message | Adds the seat's unread mail to the turn |
| On stop, mail waiting | Continues the turn with the mail as its next input |
| On stop, no mail | Holds the wait. When mail arrives, the session takes a turn on it |

The hook marks what it hands over as delivered, which is the receipt the sender can read. The envelope is then in the session's transcript. A seat with no hook, such as a headless worker, still uses `mail inbox` and `mail ack`.

The hold differs by provider, because only Claude documents a wake for an idle session:

| | Claude Code | Codex |
| --- | --- | --- |
| How the hold works | The Stop hook runs in the background with `asyncRewake`. The session is idle and Daniel can type. Mail makes the hook exit with code 2, which wakes the session with the mail as a system reminder | Verified on desktop build 26.1120.41243. The Stop hook waits within the turn. Returning `decision: block` with an envelope as its reason delivered the envelope to the same chat |
| How long | The hook's `timeout`: 600 seconds by default, set per hook. Tested with `timeout: 3600`. One hold delivered mail after 33 minutes. The upper bound is unknown | Tested with `timeout: 1900` seconds. One hold delivered mail after 30 min 10.7 s. The upper bound is unknown |
| After the hold runs out | The session is idle with no watcher. Mail waits for the next turn | The same |
| Limit on continuing a turn | Eight blocks in a row, reset when a tool is called | Four deliveries observed, with tool calls between them. The cap on uninterrupted continues is unknown |

This replaces the child that polls a mailbox. On Codex the seat still stays in its turn while it waits, as it would waiting on a child, but the app runs the wait, no model step is spent, and the seat cannot forget to start it. A hook does not fire for mail inside a subagent: the hook stays silent when its input names an agent.

### Typed input during a Codex hold

Daniel approved this on 2026-11-26 after trial 5 passed. It replaces the third point under [Decided](#decided) for Codex seats. Nothing in it is built yet.

Daniel, 2026-11-26, in the chief's chat: "when I talk to codex captains directly in the chat, the messages don't seem to be going through. messages via mailbox works. why?"

**What happens.** While the Codex Stop hook waits, the seat's turn has not ended. Codex keeps a message typed into that chat as pending input for the running turn and hands it over when the hook returns. With the shipped hold of 1,800 seconds, the operator's message waits until mail arrives or the hold runs out. In the [observation below](#codex-typed-input-observation-2026-11-26), seven of fifteen typed messages waited for a hold, one of them for the full 1,800 seconds. A Claude seat is not affected, because its hold runs in the background and the session is idle.

**The hook cannot end the hold on typed input.** A reading of the Codex source (see [Evidence](#evidence)) found no signal to wait on:

- Codex waits for the Stop hook without watching for input.
- The `UserPromptSubmit` hook runs when the pending message is handed over, after the Stop hook has returned, and not when the operator submits it.
- The Stop hook's input and environment say nothing about pending input, and the session file receives the message only at the handover.

A shorter hold only moves the delay. With a hold of H seconds, a typed message waits up to H, and mail wakes an idle seat only during the first H seconds after its turn.

**Decided: wake the Codex seat, and hold nothing.** `codex queue --thread <session id> --message <text>` queues a message for an existing session. [Trial 5](#codex-queue-wake-trial-2026-11-26) verified that it starts a turn in a loaded, idle desktop chat and that the existing prompt hook delivers mail on that turn. The implementation follows.

1. **The Codex Stop hook runs with no hold.** It continues the turn with mail that is already waiting, as today, and otherwise lets the turn end. The seat is idle and the operator's message starts a turn at once.
2. **The hooks record whether the seat is in a turn.** The prompt hook marks the seat busy. The Stop hook marks it idle, and only then makes its last check for mail.
3. **`mail send` wakes an idle Codex seat.** It stores the envelope first and reads the mark second. For a busy seat it does nothing more, and the Stop hook delivers at the end of the turn. For an idle Codex seat it runs the queue command, and the prompt hook of the turn that starts adds the mail as it does today. Because each side writes before it reads, a send that crosses a turn end costs at most one wake with no mail in it, and never loses one.
4. **The wake line is fixed text.** For example: "Formation: mail is waiting for this seat. Formation queued this line. The operator did not type it." It never carries a mail body. Queued text enters the chat as user input, and a seat must not take it for the operator's words ([rule 1](../formation/references/network.md#rules)).
5. **`watch --to` needs no change,** because it sends mail. The [watch set](../watch/1_scope.md#the-reader) later superseded this form.

Costs: Formation runs another program's command line, so one of the wake adapters this scope [set aside](#set-aside) returns for one provider. The call behind `codex queue` is marked experimental in the Codex source and can change without notice. The hook file changes, so the operator reviews it in Codex once more. The Claude hold stays as it is.

**What the trial adds to the build.**

- **The wake is the plain call.** `--resume` failed with a warning about a missing daemon, while the loaded chat still picked the line up. `mail send` never passes it.
- **A chat that is not loaded is not woken.** With the app closed, and then with the app open but the chat unloaded, the line stayed queued and ran when the chat was opened. Mail for such a seat waits until the operator opens its chat, and the reply to `mail send` says only what it knows: the envelope is stored and a wake was queued.
- **A queued line during a turn costs a turn.** It ran as a separate turn 6 milliseconds after the running one ended. That is why `mail send` wakes only a seat marked idle and leaves a busy seat to its Stop hook.
- **Typed input still works during a woken turn.** The operator's line entered the running turn after 16.4 seconds and was answered in it.

**Where the wake fails** on some build, the fallback is the 1,800-second hold with the interrupt: stop the turn, then type. A held turn has no work in flight, so stopping it loses nothing, and the Stop hook of the next turn starts a new hold. A short hold would give up the mail wake, which is the reason the hold exists.

**Until it is built,** the operator stops the Codex turn and then types, or sends by mail.

### Watch

`formation watch add` stores a probe, its recipient and schedule in the room. `formation watch run` reads the set with a single leased reader and standby takeover. A probe is a command that prints one line per item as `<fingerprint><TAB><text>`, the shape the skill's `ci-read.sh` uses. The reader mails each new fingerprint. Handover state lives in the room across reader changes. A failed read is mailed once and becomes reportable again after a successful read.

The binary runs probes without a model. New sources use new scripts. A caller that needs the result in its own turn uses `formation watch wait` to block for new items. The [watch set scope](../watch/1_scope.md) describes how seats change the set while a reader runs and how to list the reader and each watch's last result.

### What changes in the first build

| First build | Proposal | Why |
| --- | --- | --- |
| A seat is proven by a key file, named on every call | A seat is a name, found from the session id | The room serves sessions under one OS account, and any of them can read the keys. The key only prevents a slip, and the session id prevents the same slip with no file to pass |
| `send` refuses a recipient not seen for 90 seconds | `send` stores it, and the reply says whether a hold is waiting on that mailbox | A captain between two turns is the usual case, and its mail must wait for it |
| A sentry child waits on the inbox for its parent | The Stop hook waits | The trial below: an idle Codex parent started no turn on its child's report |
| An envelope has no time | An envelope carries when it was sent | Mail can wait for hours, and its reader needs its age |

Idempotent sends, the body limit and the audit of join and send stay as built.

### Set aside

The first version of this scope proposed a relay, route grants, session bindings, a receipt ladder and host wake adapters. None was built and no observed need asked for them, so they were dropped from the scope. They remain in this file's history. One returns as a proposal: a wake for an idle Codex seat, under [Typed input during a Codex hold](#typed-input-during-a-codex-hold).

## Verification

Each trial is run on the exact desktop build and recorded here before the capability is claimed.

1. **Claude, idle wake.** A Stop hook with `asyncRewake` waits on the installed `mail inbox --wait`. The chief goes idle and a second seat sends. Pass: one new turn in the same chat, carrying the mail. This also settles whether the Stop event accepts `asyncRewake`. Run on 2026-11-25: it passed, with the limits recorded under [Evidence](#claude-asyncrewake-trial-2026-11-25).
2. **Codex, hold and continue.** Passed on desktop build 26.1120.41243. The same chat received three scheduled envelopes and a fourth after a half-hour hold. See the [trial record](#codex-stop-hook-trial-2026-11-25) for typed input and timeout observations.
3. **Codex, if the hold fails.** The hold passed on the tested build, so the fallback trial is unnecessary there. If another build fails the hold, try the two leads under [Evidence](#evidence) in order. If neither works, a Codex seat receives mail at its next turn only, and the scope says so.
4. **Build:** the mailbox changes, then `formation hook`, then `watch`. Then one formation day with a Codex captain, read against the table below.
5. **Codex, wake by queue.** Passed on desktop build 26.1120.50957 for the idle wake and the prompt hook. Run `codex queue --thread <its session id> --message "queue probe <nonce>"` from a shell on the same machine after the chat's turn ends without a hold. Within a minute, one turn must start in the same chat with the exact line and run the prompt hook without input from the operator. The [trial record](#codex-queue-wake-trial-2026-11-26) covers the busy-turn and `--resume` controls and the remaining cases.

| Measure | 2026-11-25 | Target |
| --- | --- | --- |
| Routing comments on a captain's task record | Every message between the chief and the Codex captain | None |
| Longest wait for the chief to learn of the Codex captain's answer | 40 minutes | Under a minute while a hold is waiting |
| Sentry turns in an hour with nothing new | One per read | None |

## Decided

Daniel, 2026-11-25, in the chief's chat, to the four questions this scope asked: "yes to all."

1. **A seat is a name.** The key files go ([decision](4_decisions.md#a-seat-is-a-name-not-a-credential)).
2. **Mail is delivered by the two hooks.** The cost is one review of the plugin's hooks in Codex, repeated whenever the hook file changes.
3. **The hold is as long as trial 2 allows on Codex,** since a seat with no hold receives mail only at its next turn. Replaced for Codex on 2026-11-26, below.
4. **The relay, grants and wake adapters are dropped** from this scope.

He also said who runs the Codex side: "I will have astra execute them and figure out if things are working for codex."

Daniel, 2026-11-26 (UTC), in the chief's chat, on the trial 5 result: "looks like we are good to go". The question put to him before the trial was whether to run it and, if it passed, replace the Codex hold with the wake.

5. **A Codex seat holds nothing and is woken by `codex queue`,** as [Typed input during a Codex hold](#typed-input-during-a-codex-hold) describes. The Claude hold stays.

## Evidence

Read from the providers' documentation on 2026-11-25 through a summarising fetcher. The live trials below record what was observed on each build.

- **Claude Code hooks.** `asyncRewake` runs a command hook in the background and wakes the session on exit code 2. A Stop hook continues the turn with `decision: block`. Hooks also run inside subagents, where the input carries `agent_id`. Added context is capped at 10,000 characters. Plugins ship hooks. Which events accept `asyncRewake` is not documented. [Hooks reference](https://code.claude.com/docs/en/hooks)
- **Codex hooks.** On by default, with Stop, UserPromptSubmit and subagent events. A Stop hook continues the turn the same way. An async hook's output waits for the next user turn, and finishing a hook starts none. Added context is capped near 2,500 tokens. Plugin hooks are reviewed and trusted once per content hash. [Hooks](https://learn.chatgpt.com/docs/hooks)
- **Codex, no idle wake.** No documented way starts a turn in an idle chat that the desktop app owns. Two leads for trial 3: a user reports that an undocumented `wake_on_agent_messages` feature flag woke an idle parent for its own children ([report](https://github.com/openai/codex/issues/61901)), and `codex queue --thread` queues text for an existing session without saying whether it starts a turn ([CLI reference](https://learn.chatgpt.com/docs/cli/reference)).
- **Codex source, read on 2026-11-26** at commit [`f79875c`](https://github.com/openai/codex/tree/f79875c) by a research agent. This is a reading of code, not documented behaviour, and the desktop app's own code is closed. A message sent during a running turn is pushed onto that turn's pending input (`codex-rs/core/src/session/turn_input.rs`). The Stop hook is awaited with no race against input, and the same turn then runs again with the pending message (`core/src/session/turn.rs`, `core/src/tasks/regular.rs`). `UserPromptSubmit` runs when pending input is drained (`core/src/hook_runtime.rs`). The Stop hook's input holds `session_id`, `turn_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`, `permission_mode`, `stop_hook_active` and `last_assistant_message` (`hooks/src/schema.rs`). `codex queue` calls an experimental `thread/queue/add` on the shared daemon, which starts a turn if the thread is loaded and idle and otherwise at the next idle (`ext/queue/src/service.rs`). `wake_on_agent_messages` is a feature flag under development, off by default. The command-line interface labels pending input "press esc to interrupt and send immediately".
- **Claude Code channels** can also push an event into an open session. They need a running server, so the hook is tried first. [Channels](https://code.claude.com/docs/en/channels)
- **Open requests:** a [desktop external-wake API](https://github.com/openai/codex/issues/61773) and [idle chats waking on agent messages](https://github.com/openai/codex/issues/63220). Neither has a maintainer reply.

### Codex queue wake trial, 2026-11-26

Trial 5 used ChatGPT desktop build 26.1120.50957 on macOS, its bundled Codex CLI
0.171.0-alpha.13.2, and Formation 0.1.0. The existing disposable “Formation test
captain” chat (`01a210e7-decf-7e03-b35a-50660ca46d82`) finished its setup turn and
reported idle. The investigator sent each queue probe from a local shell.

| Case | Observation |
| --- | --- |
| Idle wake | The plain queue call returned at 03:35:10 UTC. One turn with trigger `queue` began in the same chat at 03:35:13.640 UTC (`01a215eb-efdf-7893-8aa3-b820887296f5`). Desktop history showed `queue probe pr26-idle-3c7f1b` as a user message. The chat replied with the exact nonce while unattended. |
| Prompt hook | The chat was idle before it was temporarily bound to a disposable Formation room. Receipt 1 was stored with `held: false`. A queue line containing one nonce started a turn about nine seconds later (`01a215ef-724f-7211-a233-72e0ce5aabbb`). `UserPromptSubmit` injected the envelope and a second nonce as developer context at the start of that same turn. The sender's receipt changed from `stored` to `delivered`, and the reply included both nonces. The temporary binding was removed during the controlled pause before Stop. |
| Busy chat | A second send at 03:36:36 UTC remained in the queue while the first turn was running. The first turn completed at 03:36:53.230 UTC. A separate queue-triggered turn began at 03:36:53.236 UTC and echoed the second nonce. |
| `--resume` | The CLI accepted the submission and warned `start a local daemon or specify a server with --remote`. The explicit resume attempt failed. The already-loaded desktop chat picked up the line about two seconds after the send. |
| Keyboard input | Daniel entered `keyboard probe c84be1` during queue-triggered turn `01a215fe-13af-7dd0-8503-ea70cb0bec53`. The input interrupted the 120-second tool wait after 16.4 seconds. The transcript recorded the line as a user message in the same turn at 03:55:20.954 UTC, and the final reply echoed it about three seconds later. |
| App closed | The shell verified that the desktop and its bundled Codex processes had exited. The plain queue call succeeded at 03:56:06.797 UTC and stored message `01a215ff-0ef3-7fe1-917d-be57873c04e8`. The app relaunched about ten seconds later. The message remained queued for nearly two more minutes while the target chat was unloaded. The investigator opened the chat at 03:58:07 UTC without submitting input. The queue started turn `01a21600-e951-7101-a08a-2af2d083fc87` at 03:58:08.213 UTC. The chat echoed the nonce and the queued row cleared. |

The loaded, idle desktop wake and prompt hook both met the one-minute criterion.
The proposed `mail send` integration and its turn-end race are implementation
work beyond this trial.

### Local sentry trial, 2026-11-24

On macOS, a Claude Desktop Code chief (Claude Desktop 6.28899.6) sent a fresh message through Formation to a Codex Desktop captain (desktop build 26.1120.41243). Before the send, the captain spawned its own sentry and completed its setup turn. The host reported the captain idle. A single blocking inbox call in the child returned the chief's message ID, body and receipt 1. The child repeated all three in its final answer. The parent history recorded child completion but still showed the original setup turn as its latest turn over three minutes later.

A second control isolated explicit child-to-parent messaging. The same captain spawned another sentry and went idle. Only then did the investigator release the sentry to read the original unread envelope. The child explicitly sent the receipt, ID and body to its parent through the Codex collaboration tool and finished. At the readback 41 seconds after child completion, the captain still showed its second setup turn as the latest turn and receipt 1 remained unread.

These observations qualify the inbox and the Codex child's ability to receive. Both parent-wake attempts remained unconfirmed on this build. A live parent waiting on its child, a CLI queue send, and a follow-up typed into the parent are different experiments. The reverse direction was deferred after the operator narrowed this trial to Codex.

### Codex Stop-hook trial, 2026-11-25

A shell sender and a captain in Codex Desktop shared a throwaway Formation 0.1.0 room on macOS. Daniel confirmed, “I reviewed and trusted it and will type during the hold”.

1. **Build and configuration.** Desktop 26.1120.41243, bundled Codex CLI 0.171.0-alpha.13.2. The trusted project’s temporary `.codex/hooks.json` was:

   ```json
   {
     "description": "Temporary Formation mailbox trial for one Codex chat",
     "hooks": {
       "Stop": [
         {
           "hooks": [
             {
               "type": "command",
               "command": "/usr/bin/python3 -I /private/tmp/formation-stop-trial.0Jf8DVXH/stop.py",
               "timeout": 1900
             }
           ]
         }
       ]
     }
   }
   ```

   The command read the Stop event from stdin. For the armed captain it waited on `formation mail inbox --wait` in intervals of up to 900 seconds and returned the first unread envelope as the reason for a block. The captain acknowledged each receipt after seeing the body in this chat.

2. **Scheduled deliveries.** The shell sent `trial-1`, `trial-2` and `trial-3` at 30, 300 and 600 seconds after arming. Each arrived as a hook prompt in the same chat with the sender’s exact body. Their distinct body suffixes were `565c872788`, `43b8c70245` and `fbf0c381b0`; the captain acknowledged receipts 1, 2 and 3 respectively.
3. **Typed input.** Daniel typed “hold probe”. The second hook invocation returned receipt 2 after 240 seconds. The chat showed its envelope followed by Daniel’s message before the captain replied. Daniel reported, “It appeared immediately in the chat”. The captain received it, and the hook log shows that the mailbox receipt released the hold.
4. **Long hold.** The largest timeout tried and accepted was 1,900 seconds. A single Stop invocation held for 1,810.7 seconds across multiple CLI waits, then delivered `long-hold` with the body `Formation long-hold check afd3398470`. The captain acknowledged receipt 4. The maximum accepted timeout remains undetermined.
5. **Session identity.** All four deliveries passed the hook’s exact comparison between stdin’s `session_id` and the value read from `CODEX_THREAD_ID` in the captain’s shell.
6. **Child Stop events.** A child completed its assigned turn. Its recorded lifecycle and the trial log did not establish whether its own Stop ran or included `agent_id`. The hook filtered on the captain’s session ID before it logged events, so a child with its own ID would leave no entry there.
7. **Continue limit.** The first Stop event had `stop_hook_active: false` and the next three had `true`. The captain called tools between deliveries. No limit was reached in this sequence; the maximum without intervening tool calls remains undetermined.

hold works

### Codex typed-input observation, 2026-11-26

Three Codex captains ran in Codex Desktop during one formation, with the plugin's shipped hooks and its 1,800-second hold. The sessions report bundled Codex CLI 0.171.0-alpha.13.2, the build of the trial above, and Formation was 0.1.0. This is a reading of the three session files afterwards, by event kind and timestamp only. It was not a planned trial, and nobody noted when each message was typed.

1. **A held seat is in a turn nearly all the time.** Over about two and a half hours each session recorded three turn starts, two interrupts and no completed turn.
2. **Fifteen typed messages were recorded.** Eight were recorded within 32 seconds of other activity in the session. Six were recorded in the same second as a mail delivery, after 24, 66, 715, 923, 967 and 1,283 seconds in which the session recorded nothing. One was recorded after exactly 1,800 seconds of nothing, with no mail.
3. **A hold that runs out hands the message to the same turn.** The 1,800-second case was followed by no new turn start.
4. **The session file shows nothing while a message waits.** A file records a message when the model receives it. The quiet time before a message is therefore the longest it can have waited, and a hook cannot find a waiting message by reading the file.

`a typed message waits for the hold`

### Claude asyncRewake trial, 2026-11-25

Run in one Claude desktop Code session on macOS: Claude Code 2.1.332, desktop app 7.37095.0, formation 0.1.0. The session under trial was a formation captain. Daniel approved loading the hook and agreed to type during a wait ("Yes to both"), and approved the rerun after the first run was stopped ("Yes, rerun").

1. **Configuration: observed.** A scratch plugin, installed at local scope, shipped one Stop hook: `{"type": "command", "command": "sh stop-hook.sh", "asyncRewake": true, "timeout": 3600}`. The script joined a scratch room as `seat` and looped on `formation mail inbox --wait 60`. On mail it acknowledged each receipt, printed the envelope to stderr and exited with code 2. With no mail after 3,300 seconds it exited with code 0. A second seat sent from a detached shell script.
2. **Stop accepts `asyncRewake`: observed.** The session went idle while the hook ran, and took turns normally during the wait. Daniel typed twice during a 33-minute wait (`/compact`, then a question). The turn ran as usual and the waiting hook process survived it. Each turn that ended during a wait fired Stop again, which started a second hook process beside the first.
3. **Idle wake: observed.** Four sends, four wakes. Each gave one new turn in the same chat. The envelope arrived as a task notification titled "Stop hook feedback", with the hook's stderr inside a system reminder that begins `Stop hook blocking error from command "Stop":`. The three short waits delivered 62 or 63 seconds after the hook started, which is the sender's delay.
4. **Long hold: observed to 33 minutes. The ceiling is not determinable from this trial.** A hook process that had waited 1,982 seconds delivered a send within a second. Two other hook processes ran their full 3,300 seconds and exited on their own. `claude plugin validate` accepted `timeout` values of 86,400 and 999,999,999, but no wait longer than 55 minutes was run, so the timeout the app enforces above 3,600 seconds is unknown.
5. **Session identity: observed.** The hook's stdin carries `session_id`, and it equals `CLAUDE_CODE_SESSION_ID` in the session's shell.
6. **Subagents: observed.** Stop did not fire when a subagent finished. SubagentStop fired instead, with `agent_id` and `agent_type` in its input. Subagents the app starts on its own also fire SubagentStop, with an empty `agent_type`.

Two limits showed up that the design did not expect:

- **A plugin's scope is the project folder, not the session.** The first run was stopped because the hook also started in the other sessions open in the same folder. Each of them held a waiter on the trial seat, and one could have taken the mail. Disabling the plugin did not stop hook processes already running, and a session that had not reloaded kept running whatever script was on disk. The rerun's script exited at once unless stdin named the trial session.
- **Stop fires on every turn end, not once per idle period.** Without a guard a seat gathers one waiter per turn. The rerun kept a pid file and exited at once while its waiter was alive.

`wake works with limits:` the hook must check `session_id` against its seat, because the plugin loads in every session in the folder; it must keep one waiter per seat, because Stop fires again on each turn end during a wait; and a hold longer than 55 minutes is untested.

Read against these limits, the shipped `formation hook` finds its seat from `session_id` and exits when the session holds none, which covers the first. It keeps no single-waiter guard: a seat can hold several waiters for up to its 540-second hold, and delivery runs in one write transaction, so only one of them hands the mail over.
