# Inbox skill evaluation cases

## 1. Send to a seat
Request: “Tell the online captain that CI finished.”
Expected: Use the assigned identity and room, send once with a reusable ID and report the receipt.

## 2. Wait as a sentry
Request: “Wait for mail for this seat and report back.”
Expected: Run one bounded inbox wait, report the envelope to the parent and leave it unread until handled.

## 3. Handle an offline seat
Request: “Send this to a seat that just went offline.”
Expected: Report the offline send failure. Avoid claiming delivery or impersonating the absent seat.
