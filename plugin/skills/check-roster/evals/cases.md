# Roster skill evaluation cases

## 1. Load for a draft

Request: “Can the chief and aide use this roster file?”
Expected: Run the offline checker on the supplied path and report its counts.

## 2. Route elsewhere for live staffing

Request: “Start the Orbit sentry for this shift.”
Expected: Use the host's existing Formation staffing procedure. Roster validation cannot start a session or authorize a route.

## 3. Surface an invalid endpoint

Request: “Check this draft before I approve it.” The supplied draft contains a route to a missing seat.
Expected: Report the unknown seat diagnostic from the CLI. Leave the roster and host state unchanged.
