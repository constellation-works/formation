---
title: Formation — Shift messaging
owner: codex
created_date: 2026-11-24
updated_date: 2026-11-25
status: Draft
feature: shift-messaging
doc_role: scope
summary: Durable seat mail between Claude and Codex sessions, handed over by a Stop hook, with one watch command for everything that is not mail.
stage: building
next: Trial the Stop hook on Codex and Claude desktop, then build the mailbox changes.
---

# Formation — Shift messaging

Daniel approved this shape on 2026-11-25 ([Decided](#decided)). [Built today](#built-today) is in the repository. The rest is filed as tasks tagged `shift-messaging`, with the two hook trials first.

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

An offline roster checker, and a local SQLite inbox with a key file per seat, presence, idempotent sends, a bounded inbox wait and explicit acknowledgment ([usage](../../usage.md#local-inbox)). A room admits cooperating seats under one OS account.

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
| How the hold works | The Stop hook runs in the background with `asyncRewake`. The session is idle and Daniel can type. Mail makes the hook exit with code 2, which wakes the session with the mail as a system reminder | The Stop hook blocks, and returns `decision: block` with the mail as the reason, which continues the turn. The turn has not ended while it holds |
| How long | The hook's `timeout`: 600 seconds by default, set per hook, maximum not documented | The same |
| After the hold runs out | The session is idle with no watcher. Mail waits for the next turn | The same |
| Limit on continuing a turn | Eight blocks in a row, reset when a tool is called | Not documented |

This replaces the child that polls a mailbox. On Codex the seat still stays in its turn while it waits, as it would waiting on a child, but the app runs the wait, no model step is spent, and the seat cannot forget to start it. A hook does not fire for mail inside a subagent: the hook stays silent when its input names an agent.

### Watch

`formation watch --to <seat>` is for everything that is not mail. A probe is any command that prints one line per item as `<fingerprint><TAB><text>`, the shape the skill's `ci-read.sh` already has. `watch` runs each probe on an interval and mails the seat every fingerprint it has not sent before. The fingerprints live in the room, so two watchers never send the same item, and a restarted one sends nothing twice. A probe that fails is mailed once as `read failed`, and again only after a read has succeeded.

`watch` needs no model. A captain starts it as a background command, or a sentry subagent runs it where the read needs judgment before it is passed on. The binary knows nothing about GitHub or Orbit: a new thing to watch is a new script. Without `--to`, `watch` returns the new items to its caller, for use outside a formation.

### What changes in the first build

| First build | Proposal | Why |
| --- | --- | --- |
| A seat is proven by a key file, named on every call | A seat is a name, found from the session id | The room serves sessions under one OS account, and any of them can read the keys. The key only prevents a slip, and the session id prevents the same slip with no file to pass |
| `send` refuses a recipient not seen for 90 seconds | `send` stores it, and the reply says whether a hold is waiting on that mailbox | A captain between two turns is the usual case, and its mail must wait for it |
| A sentry child waits on the inbox for its parent | The Stop hook waits | The trial below: an idle Codex parent started no turn on its child's report |
| An envelope has no time | An envelope carries when it was sent | Mail can wait for hours, and its reader needs its age |

Idempotent sends, the body limit and the audit of join and send stay as built.

### Set aside

The first version of this scope proposed a relay, route grants, session bindings, a receipt ladder and host wake adapters. None was built and no observed need asks for them, so they are dropped from the scope. They remain in this file's history.

## Verification

Each trial is run on the exact desktop build and recorded here before the capability is claimed.

1. **Claude, idle wake.** A Stop hook with `asyncRewake` waits on the installed `mail inbox --wait`. The chief goes idle and a second seat sends. Pass: one new turn in the same chat, carrying the mail. This also settles whether the Stop event accepts `asyncRewake`.
2. **Codex, hold and continue.** The same with a blocking Stop hook in a Codex desktop captain. Pass: the turn continues with the mail. Record what happens when Daniel types during the hold, and the longest `timeout` the app accepts.
3. **Codex, if the hold fails.** Try the two undocumented or untested wakes named under [Evidence](#evidence), in that order. If neither works, a Codex seat receives mail at its next turn only, and the scope says so.
4. **Build:** the mailbox changes, then `formation hook`, then `watch`. Then one formation day with a Codex captain, read against the table below.

| Measure | 2026-11-25 | Target |
| --- | --- | --- |
| Routing comments on a captain's task record | Every message between the chief and the Codex captain | None |
| Longest wait for the chief to learn of the Codex captain's answer | 40 minutes | Under a minute while a hold is waiting |
| Sentry turns in an hour with nothing new | One per read | None |

## Decided

Daniel, 2026-11-25, in the chief's chat, to the four questions this scope asked: "yes to all."

1. **A seat is a name.** The key files go ([decision](4_decisions.md#a-seat-is-a-name-not-a-credential)).
2. **Mail is delivered by the two hooks.** The cost is one review of the plugin's hooks in Codex, repeated whenever the hook file changes.
3. **The hold is as long as trial 2 allows on Codex,** since a seat with no hold receives mail only at its next turn.
4. **The relay, grants and wake adapters are dropped** from this scope.

He also said who runs the Codex side: "I will have astra execute them and figure out if things are working for codex."

## Evidence

Read from the providers' documentation on 2026-11-25 through a summarising fetcher. Nothing in this list has been run.

- **Claude Code hooks.** `asyncRewake` runs a command hook in the background and wakes the session on exit code 2. A Stop hook continues the turn with `decision: block`. Hooks also run inside subagents, where the input carries `agent_id`. Added context is capped at 10,000 characters. Plugins ship hooks. Which events accept `asyncRewake` is not documented. [Hooks reference](https://code.claude.com/docs/en/hooks)
- **Codex hooks.** On by default, with Stop, UserPromptSubmit and subagent events. A Stop hook continues the turn the same way. An async hook's output waits for the next user turn, and finishing a hook starts none. Added context is capped near 2,500 tokens. Plugin hooks are reviewed and trusted once per content hash. [Hooks](https://learn.chatgpt.com/docs/hooks)
- **Codex, no idle wake.** No documented way starts a turn in an idle chat that the desktop app owns. Two leads for trial 3: a user reports that an undocumented `wake_on_agent_messages` feature flag woke an idle parent for its own children ([report](https://github.com/openai/codex/issues/61901)), and `codex queue --thread` queues text for an existing session without saying whether it starts a turn ([CLI reference](https://learn.chatgpt.com/docs/cli/reference)).
- **Claude Code channels** can also push an event into an open session. They need a running server, so the hook is tried first. [Channels](https://code.claude.com/docs/en/channels)
- **Open requests:** a [desktop external-wake API](https://github.com/openai/codex/issues/61773) and [idle chats waking on agent messages](https://github.com/openai/codex/issues/63220). Neither has a maintainer reply.

### Local sentry trial, 2026-11-24

On macOS, a Claude Desktop Code chief (Claude Desktop 6.28899.6) sent a fresh message through Formation to a Codex Desktop captain (desktop build 26.1120.41243). Before the send, the captain spawned its own sentry and completed its setup turn. The host reported the captain idle. A single blocking inbox call in the child returned the chief's message ID, body and receipt 1. The child repeated all three in its final answer. The parent history recorded child completion but still showed the original setup turn as its latest turn over three minutes later.

A second control isolated explicit child-to-parent messaging. The same captain spawned another sentry and went idle. Only then did the investigator release the sentry to read the original unread envelope. The child explicitly sent the receipt, ID and body to its parent through the Codex collaboration tool and finished. At the readback 41 seconds after child completion, the captain still showed its second setup turn as the latest turn and receipt 1 remained unread.

These observations qualify the inbox and the Codex child's ability to receive. Both parent-wake attempts remained unconfirmed on this build. A live parent waiting on its child, a CLI queue send, and a follow-up typed into the parent are different experiments. The reverse direction was deferred after the operator narrowed this trial to Codex.
