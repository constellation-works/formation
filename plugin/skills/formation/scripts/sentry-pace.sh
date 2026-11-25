#!/usr/bin/env bash
# The sentry's pace: how long to sleep after a wake, chosen from how much work is in flight.
# Read-only apart from two small state files in <state dir>. Prints one line:
#
#   <tier> <sleep seconds> in_run=<n|?> recent_prs=<n|?> quiet_min=<n> [entered|left]
#
#   busy    this wake sent its reader something new
#   active  work is in flight: a task an Orbit run holds in progress, or a run's pull
#           request (head branch `orbit/…`) updated in the last hour; also when a probe
#           failed, since a failed read is not all-clear. A task in progress with no run,
#           and a pull request somebody opened by hand, are hands-on work: the owner
#           watches them, so they are not load unless the captain sends `watch`.
#   quiet   nothing in flight, for less than the idle-after window
#   idle    nothing in flight for the whole idle-after window; `entered` marks the wake
#           that goes idle and `left` the wake that leaves it
#
# Usage: sentry-pace.sh <state dir> <workspace selector> <owner/name> <items sent this wake> [model id]
#        sentry-pace.sh --watch <state dir> [minutes]
#                                             work is expected: pace as active for that
#                                             many minutes (default: the quiet interval),
#                                             then by what the probes find
#        sentry-pace.sh --rest  <state dir>   nothing is expected: the sentry pauses, with
#                                             no timer, until `--watch`
#
# The workspace selector must be host-qualified (`hm_…/ws_<name>`): from the Mac a bare name
# reads a stale local store and counts no task, so it is treated as a failed probe.
#
# Minutes per tier come from the environment, with these defaults (units.yaml repeats them):
#   PACE_BUSY=2  PACE_ACTIVE=5  PACE_QUIET=15  PACE_IDLE=30  PACE_IDLE_AFTER=60
# PACE_IN_RUN supplies the caller’s Orbit task count or ? for a failed read.
# PACE_RUN_BRANCH (default `orbit/`) is the head-branch prefix of a run's pull request.
set -uo pipefail

usage() { sed -n '2,/^set -uo/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'; }

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  --watch)
    dir="${2:?usage: sentry-pace.sh --watch <state dir> [minutes]}"
    mins="${3:-}"
    case "$mins" in ''|*[!0-9]*) mins="" ;; esac
    mkdir -p "$dir" && touch "$dir/flight" && printf '%s\n' "$mins" > "$dir/watch" \
      && echo "watch: pacing as active${mins:+ for $mins minutes}"
    exit ;;
  --rest)
    dir="${2:?usage: sentry-pace.sh --rest <state dir>}"
    mkdir -p "$dir" && rm -f "$dir/watch" && touch -t 200001010000 "$dir/flight" \
      && printf 'idle\n' > "$dir/tier" && echo "rest: paused until watch"
    exit ;;
esac

dir="${1:?usage: sentry-pace.sh <state dir> <workspace selector> <owner/name> <items sent> [model id]}"
sel="${2:?missing workspace selector}"
repo="${3:?missing owner/name}"
sent="${4:?missing items sent this wake}"
model="${5:-}"

busy=$(( ${PACE_BUSY:-2} * 60 ))
active=$(( ${PACE_ACTIVE:-5} * 60 ))
quiet=$(( ${PACE_QUIET:-15} * 60 ))
idle=$(( ${PACE_IDLE:-30} * 60 ))
idle_after=${PACE_IDLE_AFTER:-60}

mkdir -p "$dir"
flight="$dir/flight"      # its modification time is the last moment work was in flight
tier_file="$dir/tier"     # the tier the previous wake chose
watch="$dir/watch"        # touched by --watch: the reader expects work
mtime() { if [[ "$(uname -s)" == Darwin ]]; then stat -f %m "$1"; else stat -c %Y "$1"; fi; }
now="$(date +%s)"

# Probe 1: tasks an Orbit run holds in progress (`job_run_id` set). A drain's work counts;
# a task somebody works by hand does not.
case "${PACE_IN_RUN+x}:$sel" in
  x:*)
    in_progress="$PACE_IN_RUN"
    case "$in_progress" in ''|*[!0-9]*) in_progress="?" ;; esac ;;
  :hm_*/*)
    if [ -z "$model" ]; then echo "supply the model id for the Orbit task probe" >&2; exit 2; fi
    in_progress="$(orbit --workspace "$sel" tool run orbit.task.list --model "$model" \
        --input '{"status":["in-progress"],"fields":["id","job_run_id"],"limit":100}' 2>/dev/null \
      | python3 -c 'import json,sys; print(sum(1 for t in json.load(sys.stdin)["tasks"] if t.get("job_run_id")))' 2>/dev/null)" \
      || in_progress="?" ;;
  *)
    echo "sentry-pace.sh: '$sel' is not host-qualified (hm_…/ws_<name>); task probe skipped" >&2
    in_progress="?" ;;
esac
[ -n "$in_progress" ] || in_progress="?"

# Probe 2: a run's open pull requests touched in the last hour. An old open pull request
# is not work in flight, and neither is one somebody opened by hand.
run_branch="${PACE_RUN_BRANCH:-orbit/}"
recent_prs="$(gh pr list -R "$repo" --state open --limit 100 --json updatedAt,headRefName 2>/dev/null \
    | RUN_BRANCH="$run_branch" python3 -c '
import json, os, sys, time, calendar
prefix = os.environ["RUN_BRANCH"]
cut = time.time() - 3600
stamp = lambda s: calendar.timegm(time.strptime(s, "%Y-%m-%dT%H:%M:%SZ"))
print(sum(1 for p in json.load(sys.stdin)
          if p["headRefName"].startswith(prefix) and stamp(p["updatedAt"]) > cut))' 2>/dev/null)" \
  || recent_prs="?"
[ -n "$recent_prs" ] || recent_prs="?"

in_flight=0
if [ "$in_progress" = "?" ] || [ "$recent_prs" = "?" ]; then
  in_flight=1
elif [ "$in_progress" -gt 0 ] || [ "$recent_prs" -gt 0 ]; then
  in_flight=1
elif [ -e "$watch" ]; then
  watch_min="$(cat "$watch" 2>/dev/null)"
  case "$watch_min" in ''|*[!0-9]*) watch_for="$quiet" ;; *) watch_for=$(( watch_min * 60 )) ;; esac
  if [ $(( now - $(mtime "$watch") )) -lt "$watch_for" ]; then in_flight=1; fi
fi

# The first wake starts the quiet clock; work in flight restarts it.
if [ "$in_flight" = 1 ] || [ ! -e "$flight" ]; then touch "$flight"; fi

last="$(mtime "$flight")"
quiet_min=$(( (now - last) / 60 ))

if [ "$sent" -gt 0 ] 2>/dev/null; then
  tier=busy; secs=$busy
elif [ "$in_flight" = 1 ]; then
  tier=active; secs=$active
elif [ "$quiet_min" -lt "$idle_after" ]; then
  tier=quiet; secs=$quiet
else
  tier=idle; secs=$idle
fi

prev="$(cat "$tier_file" 2>/dev/null || true)"
mark=""
if [ "$tier" = idle ] && [ "$prev" != idle ]; then mark=" entered"; fi
if [ "$tier" != idle ] && [ "$prev" = idle ]; then mark=" left"; fi
printf '%s\n' "$tier" > "$tier_file"

printf '%s %s in_run=%s recent_prs=%s quiet_min=%s%s\n' \
  "$tier" "$secs" "$in_progress" "$recent_prs" "$quiet_min" "$mark"
