#!/usr/bin/env bash
# The sentry's read of red CI on a repository's base branch and its open pull requests,
# and of open pull requests that conflict with their base.
# Read-only. Prints one line per item:
#
#   <fingerprint><TAB>ci_red | <severity> | <run or PR> | <reason> | <command>
#   <fingerprint><TAB>pr_conflict | medium | PR #<n> | <reason> | <command>
#
# Orbit's own `ci_red` attention kind covers only completed runs on the base branch's
# current head, and no pull request, so the sentry reads the forge as well.
#
# Usage: ci-read.sh [--base-only] <owner/name> <base branch> [pr grace minutes, default 15]
# --base-only reads the base branch and no pull request. Use it while repair mode is off:
# an Orbit drain started with --auto-fix repairs the pull requests its runs published, so
# a line for each of them would only cost its reader a turn.
# Exit 0 with no output means a complete read found no failures or conflicts.
# A non-zero exit means the read failed.
set -euo pipefail

usage="usage: ci-read.sh [--base-only] <owner/name> <base branch> [pr grace minutes]"
base_only=0
if [ "${1:-}" = "--base-only" ]; then
  base_only=1
  shift
fi
repo="${1:?$usage}"
base="${2:?$usage}"
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

[ "$base_only" -eq 1 ] && exit 0

# Pull requests: a check that failed on the current head and has stayed failed for the
# grace period. A delivery run's own re-run settles a flake within minutes; one that is
# still red after that waits for the sergeant. A new push changes the fingerprint.
# A conflict with the base is listed at once: nothing clears it by waiting. `mergeable`
# is UNKNOWN while the forge is still computing it, and that is not a conflict.
gh pr list -R "$repo" --state open --limit 100 \
  --json number,headRefOid,baseRefName,mergeable,statusCheckRollup \
  --jq '
    .[]
    | { n: .number, h: .headRefOid[0:9], b: .baseRefName, m: .mergeable,
        f: [ .statusCheckRollup[]?
             | select((.conclusion // .state) as $c
                      | ["FAILURE", "TIMED_OUT", "STARTUP_FAILURE", "ERROR"] | index($c))
             | select(((.completedAt // "") | if . == "" then 0 else fromdateiso8601 end)
                      < (now - '"$grace_seconds"'))
             | (.name // .context) ] }
    | ( select(.f | length > 0)
        | "pr:\(.n):\(.h)\tci_red | medium | PR #\(.n) | \(.f | unique | join(", ") | .[0:100]) failed at \(.h) | gh pr checks \(.n) -R '"$repo"'" ),
      ( select(.m == "CONFLICTING")
        | "conflict:\(.n):\(.h)\tpr_conflict | medium | PR #\(.n) | conflicts with \(.b) at \(.h) | gh pr view \(.n) -R '"$repo"' --json mergeable,headRefName,baseRefName" )'
