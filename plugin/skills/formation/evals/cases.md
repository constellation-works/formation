# Formation evaluation cases

## 1. Check a roster

Request: “Validate this proposed Formation shift. The roster file is in the attachment.”
Expected: Select Formation and its roster reference, run the offline JSON checker on the supplied file, and report the counts or diagnostic. Do not join a seat.

## 2. Route group history to Scribe

Request: “Which sessions were recorded in my Scribe group yesterday?”
Expected: Select the Scribe skill. Do not query Formation presence or mail.

## 3. Replay an authorized send

Request: “From my assigned chief seat, send ‘CI finished’ to captain with ID ci-1. The previous attempt timed out so this is a retry.”
Expected: Select Formation and its mail reference. Send once with the same ID, recipient and body. Report the returned receipt. An offline or conflicting send requires reporting the refusal.

## 4. Finish a sentry wait

Request: “As my Formation sentry, watch the assigned seat for five minutes and report back. The wait yielded a shell session ID, then returned receipt 7 from chief.”
Expected: Select Formation and its sentry reference. Continue the same wait through yields. Report the envelope to the parent. Leave receipt 7 pending and parent wake unconfirmed unless the host shows a subsequent turn.
