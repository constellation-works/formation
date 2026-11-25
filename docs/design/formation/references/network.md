---
title: Session messaging evidence
owner: codex
created_date: 2026-11-16
updated_date: 2026-11-24
status: Draft
feature: formation
doc_role: reference
summary: Observed session behavior by transport and receiving host, with the limits of each observation.
---

# Session messaging evidence

| Sending path | What was observed | What remains open |
| --- | --- | --- |
| Claude native peer message to a detached Claude Desktop Code seat | A peer message addressed by host session ID started a turn in the idle recipient during the original squad trials. | Whether a message arriving through the Formation inbox starts a Claude turn. |
| Claude Desktop Code chief to Formation inbox on macOS | A Codex Desktop sentry waiting on the captain’s seat returned the chief’s message ID, body and receipt. | Starting a new turn in the idle Codex captain. |
| Codex sentry to its own captain | The host recorded child completion. A second sentry explicitly messaged the parent. In both controls the latest observed parent turn was its setup turn. | Wake on another build or through a different authorized host mechanism. |
| Codex chief or captain to a Claude Desktop recipient through Formation | The reverse cross-provider trial was deferred. | Both receipt in the target seat and an independently observed turn in the existing Claude chat. |

The [local trial record](../../shift-messaging/1_scope.md#local-sentry-trial-2026-11-24) gives the builds, observation windows and receipt. The [original Claude network notes](https://github.com/constellation-works/formation/blob/520d0d03181bfb68ea15347093f2a4b831c6c298/docs/design/formation/references/network.md) preserve the early squad observations. The packaged [Claude](../../../../plugin/skills/formation/references/provider-claude.md) and [Codex](../../../../plugin/skills/formation/references/provider-codex.md) procedures describe how a seat reports on each host.
