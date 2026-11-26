---
title: Formation — Shift messaging
owner: codex
created_date: 2026-11-24
updated_date: 2026-11-26
last_validated: 2026-11-26
status: Draft
feature: shift-messaging
doc_role: scope
summary: Durable seat mail between Claude and Codex sessions, delivered by host hooks and fed by one shared watch reader.
stage: building
next: Daniel refreshes and trusts the updated Codex plugin hooks for production chats.
---

# Formation — Shift messaging

Daniel approved this shape on 2026-11-25 ([Decided](#decided)). A formation on 2026-11-26 showed that the Codex hold delayed typed input. Daniel approved the queue wake after trial 5 on the receiving desktop build. The same day he approved [a longer Claude hold](#a-claude-seat-after-its-hold) and four trials of it.

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
| Delivery | Two hooks hand unread mail to the bound session. Claude waits in the background. Formation queues a wake for an idle Codex seat |
| Watch | One command that blocks on any number of probes (CI, pull requests, attention) and mails what is new to a seat |

### Built today

An offline roster checker, a leased watch reader, and a local SQLite inbox with session-bound seats, presence, idempotent sends, a bounded inbox wait and explicit acknowledgment. The plugin hooks deliver mail and `mail sent` reports its state. A room admits cooperating seats under one OS account. [Inbox usage](../../usage.md#local-inbox), [hook usage](../../usage.md#hooks)

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
| On stop, no mail | Claude waits in the background. Codex lets the turn end and can be woken by the sender |

The hook marks what it hands over as delivered, which is the receipt the sender can read. The envelope is then in the session's transcript. A seat with no hook, such as a headless worker, still uses `mail inbox` and `mail ack`.

Claude's Stop hook uses `asyncRewake` and exits with code 2 when mail arrives, waking the session with a system reminder. When the hold expires, later mail waits for the next hook, which [a longer hold](#a-claude-seat-after-its-hold) addresses. Both providers ignore hook input naming a subagent.

### Typed input during a Codex hold

Daniel, 2026-11-26, in the chief's chat: "when I talk to codex captains directly in the chat, the messages don't seem to be going through. messages via mailbox works. why?"

The old Stop hook waited inside the turn, which kept typed input pending. Seven of fifteen typed messages waited during the observed formation, one for the full 1,800 seconds. [Typed-input observation](#codex-typed-input-observation-2026-11-26)

The new Codex Stop hook checks for mail and returns without a hold. The prompt hook marks the bound seat busy before reading mail. Stop marks it idle before its final check and marks it busy again when it hands over a continuation.

Ordinary sends and the leased watch reader commit mail before checking for a wake. A sender may claim one wake per idle period on the current Codex binding. Formation calls `codex queue --thread <session id> --message <text>` with a fixed line and a timeout. The receiving prompt hook reads the durable envelope. If a send crosses Stop and the hook delivers first, at most one queue line from that idle interval can arrive with an empty inbox. A failed submission leaves the receipt stored for the next normal hook.

The queued line explicitly attributes itself to Formation and carries no envelope content. Apply the usual approval rules to the line and the hook-delivered envelope. Daniel refreshes and trusts the plugin hooks when adopting the cutover. [Session network](../formation/references/network.md#rules), [Plugin package](../../usage.md#plugin-package)

### A Claude seat after its hold

Daniel approved the longer hold and its four trials on 2026-11-26 ([Decided](#decided), 6). The hold is not built yet and no trial has run. Items 3 and 5 below are still proposals.

**Observed on 2026-11-26 (UTC), by the chief.** A Claude sergeant ended its turn at 03:53. Its seat was last seen at 04:02, when the 540-second hold ended. Three envelopes then sat unread until the chief sent that chat a native session message at about 05:02. Seats that take turns on a timer, such as the sentry and the chief, were not affected.

**Verified in the shipped hook file.** The Claude Stop hook runs with `--hold 540` and a host timeout of 600 seconds. Nothing waits for that seat after the hold.

**The 540 seconds is a choice, not a host limit.** 600 seconds is the documented default timeout of a command hook, and no maximum is documented. [Trial 1](#claude-asyncrewake-trial-2026-11-25) ran a timeout of 3,600 seconds: one waiter delivered after 1,982 seconds and two ran their full 3,300. No hold longer than 55 minutes has been tried.

| Recipient | Today | With the longer hold |
| --- | --- | --- |
| Idle, a hold waiting | Woken within a second of the send (trial 1) | The same |
| Idle, the hold ended | Mail waits for the seat's next turn | The hold spans the shift, so this is only a waiter that died |
| Busy | The prompt hook hands the mail over, or the Stop at the turn's end finds it | The same |
| App closed, or the chat not loaded | Mail waits for the chat's next prompt | The same. No path wakes it |

1. **The hold spans the shift (decided).** The Claude hold rises to `--hold 3300` with a timeout of 3,600, the range trial 1 observed. It rises again to a shift's length if trial 6 shows the app honours a longer timeout.
2. **One waiter per seat (decided).** Stop fires at every turn end and each starts a waiter. At 540 seconds that was tolerable and at an hour it is not. The waiter takes a lease on its seat in the room, as the watch reader does, and a later Stop that finds a live lease exits at once. Trial 1 showed that an older waiter still wakes the chat after turns in between.
3. **The sender is told (proposed).** `send` already replies whether a hold is waiting. A sender whose mail to an unheld seat is still unread after ten minutes gets one line through its own hook. On 2026-11-26 nobody was told for an hour.
4. **The fallback is a native session message (in use).** A Claude desktop sender that reads `held: false` follows with one line to that chat, and the recipient's prompt hook hands over the mail. A Codex seat or a process has no such route: it tells the chief or the seat's captain, who sends the line. The plugin's mail reference carries this procedure.
5. **A wake from any process (proposed, if trial 9 passes).** Claude Code documents a messaging socket for each session that a script can post to, which starts a turn in an idle session. It would do for a Claude seat what `codex queue` does for a Codex seat, for any sender including a Codex seat and the watch reader, and it covers a dead waiter. Formation would keep what it needs to reach each chat in the room. If that includes a host token, keeping it is Daniel's decision.

| Passed over | Why |
| --- | --- |
| A scheduled wake in each seat | It fires only while the session is open and idle, and each fire is a model turn with nothing to read |
| A background shell waiter started by the seat | A new turn on its exit is documented only for `claude -p`, and the seat must start it again every turn. The Stop hook does that by itself |
| Channels | Research preview, set up from a terminal, enabled by an organisation's owner. Desktop support is not documented |
| `claude --resume <id> -p` against a chat the app has open | Nothing documents two processes on one session, and the Codex equivalent failed in trial 5 |
| Remote Control | It carries a person's messages from a phone or browser and offers a process nothing |

### Watch

`formation watch add` stores a probe, its recipient and schedule in the room. `formation watch run` reads the set with a single leased reader and standby takeover. A probe is a command that prints one line per item as `<fingerprint><TAB><text>`, the shape the skill's `ci-read.sh` uses. The reader mails each new fingerprint. Handover state lives in the room across reader changes. A failed read is mailed once and becomes reportable again after a successful read.

The binary runs probes without a model. New sources use new scripts. A caller that needs the result in its own turn uses `formation watch wait` to block for new items. The [watch set scope](../watch/1_scope.md) describes how seats change the set while a reader runs and how to list the reader and each watch's last result.

### What changes in the first build

| First build | Proposal | Why |
| --- | --- | --- |
| A seat is proven by a key file, named on every call | A seat is a name, found from the session id | The room serves sessions under one OS account, and any of them can read the keys. The key only prevents a slip, and the session id prevents the same slip with no file to pass |
| `send` refuses a recipient not seen for 90 seconds | `send` stores it, and the reply says whether a hold is waiting on that mailbox | A captain between two turns is the usual case, and its mail must wait for it |
| A sentry child waits on the inbox for its parent | Claude Stop waits in the background. Formation queues a wake for an idle Codex seat | The local sentry trial found that an idle Codex parent started no turn on its child's report |
| An envelope has no time | An envelope carries when it was sent | Mail can wait for hours, and its reader needs its age |

Idempotent sends, the body limit and the audit of join and send stay as built.

### Set aside

The relay, route grants and receipt ladder from the first scope remain in the document history. Queue wake applies to the current local Codex binding under the [delivery rules](#delivery).

## Verification

Each trial is run on the exact desktop build and recorded here before the capability is claimed.

1. **Claude, idle wake.** A Stop hook with `asyncRewake` waits on the installed `mail inbox --wait`. The chief goes idle and a second seat sends. Pass: one new turn in the same chat, carrying the mail. This also settles whether the Stop event accepts `asyncRewake`. Run on 2026-11-25: it passed, with the limits recorded under [Evidence](#claude-asyncrewake-trial-2026-11-25).
2. **Codex, hold and continue.** Passed on desktop build 26.1120.41243. The same chat received three scheduled envelopes and a fourth after a half-hour hold. See the [trial record](#codex-stop-hook-trial-2026-11-25) for typed input and timeout observations.
3. **Codex, fallback for the original hold.** Trial 2 passed on the tested build, so this fallback trial was unnecessary.
4. **Build:** the mailbox changes, then `formation hook`, then `watch`. Then one formation day with a Codex captain, read against the table below.
5. **Codex, wake by queue.** Plain queue started a new turn and ran the prompt hook within a minute in the exact disposable chat on desktop build 26.1120.50957. A busy chat queued the line for the next idle period. With Desktop closed, the line remained queued until the app reopened and loaded the target chat. [Pinned trial record](https://github.com/constellation-works/formation/blob/f390829a1764a4aa80a05b8044f0e069d7f99e03/docs/design/shift-messaging/1_scope.md#codex-queue-wake-trial-2026-11-25)

Trials 6 to 9 belong to [A Claude seat after its hold](#a-claude-seat-after-its-hold). Daniel approved them on 2026-11-26 ([Decided](#decided), 6), and none has run. They use a disposable Claude desktop chat, a scratch room and a scratch plugin as trial 1 did, and no seat of a live shift.

6. **Claude, long hold.** A timeout of 14,400 seconds, and a send 3 hours 50 minutes into the wait. Pass: one new turn in the same chat, carrying the mail. Repeated at 43,200 seconds if it passes.
7. **Claude, one waiter.** Five turns end during one wait. Pass: one waiter process is left, and a send still wakes the chat.
8. **Claude, app restart.** The app is quit and reopened during a hold. Recorded: whether the waiter is gone, and when the chat next receives mail.
9. **Claude, wake by socket.** After the hold has ended, a detached shell posts one fixed line to the chat's messaging socket. Pass: one new turn in the same chat, and the prompt hook hands over the waiting mail. Recorded: what a hook sees of the socket and its token, how the line is labelled in the chat, and the result while the chat is busy and in each permission mode.

Trial 5 exercised the provider queue and prompt hook. The built-executable integration tests exercise Formation's ordinary send, watch reader and turn-end ordering with a fake queue. The Linux validation host has no disposable Desktop chat for a live test of the integrated send.

| Measure | 2026-11-25 | Target |
| --- | --- | --- |
| Routing comments on a captain's task record | Every message between the chief and the Codex captain | None |
| Longest wait for the chief to learn of the Codex captain's answer | 40 minutes | Under a minute while a hold is waiting |
| Sentry turns in an hour with nothing new | One per read | None |

## Decided

Daniel, 2026-11-25, in the chief's chat, to the four questions this scope asked: "yes to all."

1. **A seat is a name.** The key files go ([decision](4_decisions.md#a-seat-is-a-name-not-a-credential)).
2. **Mail is delivered by the two hooks.** The cost is one review of the plugin's hooks in Codex, repeated whenever the hook file changes.
3. Daniel approved replacing the Codex hold with plain queue after trial 5 on 2026-11-26.
4. The relay and grants are outside this scope.
5. **A Codex seat holds nothing and is woken by `codex queue`,** as [Typed input during a Codex hold](#typed-input-during-a-codex-hold) describes. The Claude hold stays.
6. **The Claude hold spans the shift, with one waiter per seat,** and trials 6 to 9 run, as [A Claude seat after its hold](#a-claude-seat-after-its-hold) describes. The line to a sender and the socket wake are not decided.

He also said who runs the Codex side: "I will have astra execute them and figure out if things are working for codex."

Daniel, 2026-11-26 (UTC), in the chief's chat, on the trial 5 result: "looks like we are good to go". The question put to him before the trial was whether to run it and, if it passed, replace the Codex hold with the wake.

Daniel, 2026-11-26 (UTC), in the chief's chat, to "Build the longer Claude mail hold now and run the four tests?": "Yes to both".

## Evidence

Read from the providers' documentation on 2026-11-25 through a summarising fetcher. The live trials below record what was observed on each build.

- **Claude Code hooks.** `asyncRewake` runs a command hook in the background and wakes the session on exit code 2. A Stop hook continues the turn with `decision: block`. Hooks also run inside subagents, where the input carries `agent_id`. Added context is capped at 10,000 characters. Plugins ship hooks. Which events accept `asyncRewake` is not documented. [Hooks reference](https://code.claude.com/docs/en/hooks)
- **Codex hooks.** On by default, with Stop, UserPromptSubmit and subagent events. A Stop hook continues the turn the same way. An async hook's output waits for the next user turn, and finishing a hook starts none. Added context is capped near 2,500 tokens. Plugin hooks are reviewed and trusted once per content hash. [Hooks](https://learn.chatgpt.com/docs/hooks)
- **Codex, no idle wake.** No documented way starts a turn in an idle chat that the desktop app owns. Two leads for trial 3: a user reports that an undocumented `wake_on_agent_messages` feature flag woke an idle parent for its own children ([report](https://github.com/openai/codex/issues/61901)), and `codex queue --thread` queues text for an existing session without saying whether it starts a turn ([CLI reference](https://learn.chatgpt.com/docs/cli/reference)).
- **Codex source, read on 2026-11-26** at commit [`f79875c`](https://github.com/openai/codex/tree/f79875c) by a research agent. This is a reading of code, not documented behaviour, and the desktop app's own code is closed. A message sent during a running turn is pushed onto that turn's pending input (`codex-rs/core/src/session/turn_input.rs`). The Stop hook is awaited with no race against input, and the same turn then runs again with the pending message (`core/src/session/turn.rs`, `core/src/tasks/regular.rs`). `UserPromptSubmit` runs when pending input is drained (`core/src/hook_runtime.rs`). The Stop hook's input holds `session_id`, `turn_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`, `permission_mode`, `stop_hook_active` and `last_assistant_message` (`hooks/src/schema.rs`). `codex queue` calls an experimental `thread/queue/add` on the shared daemon, which starts a turn if the thread is loaded and idle and otherwise at the next idle (`ext/queue/src/service.rs`). `wake_on_agent_messages` is a feature flag under development, off by default. The command-line interface labels pending input "press esc to interrupt and send immediately".
- **Claude Code channels** can also push an event into an open session. They need a running server, so the hook is tried first. [Channels](https://code.claude.com/docs/en/channels)
- **Claude Code, read on 2026-11-26** through a summarising agent, for [A Claude seat after its hold](#a-claude-seat-after-its-hold). None of it is trialled.
  - A command hook's timeout defaults to 600 seconds with no stated maximum, and it is enforced on an `asyncRewake` hook. An `asyncRewake` hook wakes an idle session. [Hooks reference](https://code.claude.com/docs/en/hooks)
  - Each session binds an inbox socket. A hook or a script posts to it with `CLAUDE_CODE_MESSAGING_SOCKET` and `CLAUDE_CODE_MESSAGING_TOKEN`, and an idle receiver starts a new turn. The message counts as another session's, and a receiver in bypass mode holds it for approval. Both variables were set in a Claude desktop session's shell on 2026-11-26; their values were not read. [Cross-session messaging](https://code.claude.com/docs/en/cross-session-messaging)
  - A background command keeps running after the final response, and a local desktop session sets it no time limit. A new turn on its exit is documented only for `claude -p`. [Tools reference](https://code.claude.com/docs/en/tools-reference), [headless](https://code.claude.com/docs/en/headless)
  - Scheduled tasks fire only while the session is running and idle, at intervals of a minute or more, with no catch-up. [Scheduled tasks](https://code.claude.com/docs/en/scheduled-tasks)
  - Channels are a research preview started with `claude --channels` in a terminal, and an organisation's owner enables them. Remote Control sends a person's messages into a local session and documents no interface for a process. [Remote Control](https://code.claude.com/docs/en/remote-control)
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
Trial 5 did not exercise Formation's send integration or the turn-end race; the built-executable integration tests now cover both, as described in Verification.

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
