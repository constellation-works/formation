#!/usr/bin/env bash
# The sentry's read of red CI on a repository's base branch and its open pull requests.
# Read-only. Prints one line per red item:
#
#   <fingerprint><TAB>ci_red | <severity> | <run or PR> | <reason> | <command>
#
# Orbit's own `ci_red` attention kind covers only completed runs on the base branch's
# current head, and no pull request, so the sentry reads the forge as well.
#
# Usage: ci-read.sh <owner/name> <base branch> [pr grace minutes, default 15]
# Exit 0 with no output means nothing is red. A non-zero exit means the read failed,
# which is not all-clear.
set -euo pipefail

repo="${1:?usage: ci-read.sh <owner/name> <base branch> [pr grace minutes]}"
base="${2:?usage: ci-read.sh <owner/name> <base branch> [pr grace minutes]}"
grace_seconds=$(( ${3:-15} * 60 ))

# Base branch: the newest finished run of each workflow. A cancelled or skipped run is
# superseded, not red.
gh run list -R "$repo" --branch "$base" --limit 40 \
  --json databaseId,workflowName,status,conclusion,headSha \
  --jq '
    [.[] | select(.status == "completed" and .conclusion != "cancelled" and .conclusion != "skipped")]
    | group_by(.workflowName) | map(max_by(.databaseId)) | .[]
    | select(.conclusion != "success" and .conclusion != "neutral")
    | "ci:\(.workflowName):\(.headSha[0:9])\tci_red | high | run \(.databaseId) | \(.workflowName) \(.conclusion) on '"$base"' at \(.headSha[0:9]) | gh run view \(.databaseId) -R '"$repo"' --log-failed"'

# Pull requests: a check that failed on the current head and has stayed failed for the
# grace period. A delivery run repairs its own pull request within minutes; one that is
# still red after that has nobody working on it. A new push changes the fingerprint.
gh pr list -R "$repo" --state open --limit 100 \
  --json number,headRefOid,statusCheckRollup \
  --jq '
    .[]
    | { n: .number, h: .headRefOid[0:9],
        f: [ .statusCheckRollup[]?
             | select((.conclusion // .state) as $c
                      | ["FAILURE", "TIMED_OUT", "STARTUP_FAILURE", "ERROR"] | index($c))
             | select(((.completedAt // "") | if . == "" then 0 else fromdateiso8601 end)
                      < (now - '"$grace_seconds"'))
             | (.name // .context) ] }
    | select(.f | length > 0)
    | "pr:\(.n):\(.h)\tci_red | medium | PR #\(.n) | \(.f | unique | join(", ") | .[0:100]) failed at \(.h) | gh pr checks \(.n) -R '"$repo"'"'
