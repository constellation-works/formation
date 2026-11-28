#!/usr/bin/env bash
# The sentry's read of Orbit attention for one host-qualified workspace selector.
# Read-only. Prints one line per item:
#
#   <fingerprint><TAB><kind> | <severity> | <id> | <reason> | <next command>
#
# Usage: attention-read.sh hm_<owner>/ws_<name>
#
# Both reads are pinned to the selector's owner host. This probe holds no Operator
# capability, so it covers the owner's agent-readable task kinds only. It never asks
# for the Operator-only runtime and host kinds (run_failed, run_stuck, claim_stale,
# host_unhealthy); those are read on an authorized path (references/role-sentry.md#coverage).
# Exit 0 with no output means nothing needs attention. A non-zero exit with one line
# on stderr means the read failed, was truncated or degraded, the selector names no
# owner (exit 2), or `orbit` is absent.
set -uo pipefail

if [ "$#" -ne 1 ] || [ -z "${1:-}" ]; then
  echo "usage: attention-read.sh hm_<owner>/ws_<name>" >&2
  exit 2
fi

sel="$1"

if ! command -v orbit >/dev/null 2>&1; then
  echo "attention-read.sh: orbit is not installed on PATH" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "attention-read.sh: python3 is not installed on PATH" >&2
  exit 1
fi

exec python3 - "$sel" <<'PY'
import json
import re
import subprocess
import sys

# Orbit's maximum public page size (`per_workspace_item_cap` is 200).
QUERY_LIMIT = 500

# Operator-only runtime and host sources. Orbit answers an unprivileged read of
# these with a `capability` degradation, so the probe never requests them.
OPERATOR_KINDS = [
    "run_failed",
    "run_stuck",
    "claim_stale",
    "host_unhealthy",
]

# Agent-readable kinds that the sentry's digest covers. The fast read omits them;
# the critical read returns only their critical items.
DIGEST_KINDS = [
    "friction_untriaged",
    "notice_open",
    "review_waiting",
]

# `hm_<owner>/ws_<name>`, as `orbit.workspace.list` reports a workspace.
OWNER_SELECTOR = re.compile(r"(hm_[A-Za-z0-9_-]+)/ws_[A-Za-z0-9_-]+")

SEVERITY_ORDER = {
    "critical": 0,
    "high": 1,
    "medium": 2,
    "low": 3,
}

REF_KEYS = (
    "task_id",
    "run_id",
    "pr_id",
    "pr_number",
    "friction_id",
    "notice_id",
    "resident_id",
    "job_run_id",
)


def one_line(text: str) -> str:
    return " ".join(text.split())


def fail(message: str) -> None:
    line = one_line(message) or "attention read failed"
    print(f"attention-read.sh: {line}", file=sys.stderr)
    sys.exit(1)


def extract_error(stdout: str, stderr: str, returncode: int) -> str:
    for stream in (stdout, stderr):
        text = stream.strip()
        if not text:
            continue
        try:
            payload = json.loads(text)
        except json.JSONDecodeError:
            payload = None
        if isinstance(payload, dict):
            err = payload.get("error") or payload.get("message")
            if isinstance(err, str) and err.strip():
                return one_line(err)
        for line in text.splitlines():
            stripped = line.strip()
            if stripped:
                return stripped
    return f"orbit exited with status {returncode}"


def is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def describe_degraded_entry(entry: object) -> str:
    if isinstance(entry, dict):
        kind = one_line(str(entry.get("kind") or ""))
        scope = one_line(str(entry.get("scope") or ""))
        code = one_line(str(entry.get("code") or ""))
        msg = one_line(str(entry.get("message") or ""))
        label = kind or scope or "source"
        if code:
            label = f"{label} [{code}]"
        if msg:
            return f"{label}: {msg}"
        return label
    return one_line(str(entry)) or "unknown source"


def degraded_error(degraded: list) -> str:
    count = len(degraded)
    noun = "source" if count == 1 else "sources"
    details = "; ".join(describe_degraded_entry(entry) for entry in degraded[:3])
    if count > 3:
        details = f"{details}; +{count - 3} more"
    return f"incomplete attention read ({count} degraded {noun}): {details}"


def truncation_error(counts: dict, item_count: int) -> str:
    matching = counts.get("matching")
    if is_int(matching):
        return f"truncated attention read: returned {item_count} of {matching} matching items"
    return f"truncated attention read: response reported truncated after {item_count} items"


def ensure_complete(payload: dict, items: list) -> None:
    if payload.get("truncated") is True:
        fail("truncated attention read")
    if payload.get("incomplete") is True or payload.get("complete") is False:
        fail("incomplete attention read")

    counts = payload.get("counts")
    if counts is not None:
        if not isinstance(counts, dict):
            fail("unexpected counts metadata from orbit.attention")
        if counts.get("truncated") is True:
            fail(truncation_error(counts, len(items)))
        returned = counts.get("returned")
        if is_int(returned) and returned != len(items):
            fail(
                f"incomplete attention read: counts.returned={returned} "
                f"does not match items={len(items)}"
            )
        matching = counts.get("matching")
        if is_int(matching) and matching > len(items):
            fail(
                f"truncated attention read: returned {len(items)} of {matching} matching items"
            )
    elif len(items) >= QUERY_LIMIT:
        fail(f"truncated attention read: reached limit {QUERY_LIMIT} without counts metadata")

    degraded = payload.get("degraded")
    if degraded is not None:
        if not isinstance(degraded, list):
            fail("unexpected degraded metadata from orbit.attention")
        if degraded:
            fail(degraded_error(degraded))


def owner_host(selector: str) -> str:
    match = OWNER_SELECTOR.fullmatch(selector)
    if match is None:
        print(
            "attention-read.sh: the selector must be host-qualified (hm_<owner>/ws_<name>) "
            "so both reads pin to its owner",
            file=sys.stderr,
        )
        sys.exit(2)
    return match.group(1)


def read_attention(selector: str, query: dict) -> list:
    proc = subprocess.run(
        [
            "orbit",
            "--workspace",
            selector,
            "tool",
            "run",
            "orbit.attention",
            "--input",
            json.dumps(query),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        fail(extract_error(proc.stdout, proc.stderr, proc.returncode))
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError:
        fail(extract_error("", proc.stdout or proc.stderr, proc.returncode) or "invalid JSON from orbit.attention")
    if not isinstance(payload, dict) or not isinstance(payload.get("items"), list):
        if isinstance(payload, dict) and isinstance(payload.get("error"), str):
            fail(payload["error"])
        fail("unexpected response from orbit.attention")
    items = payload["items"]
    ensure_complete(payload, items)
    return items


def clean_field(value: str) -> str:
    return one_line(value.replace("|", " "))


def item_id(item: dict) -> str:
    refs = item.get("refs")
    if isinstance(refs, dict):
        for key in REF_KEYS:
            val = refs.get(key)
            if val is not None and str(val).strip():
                if key == "pr_number":
                    return f"PR #{val}"
                return str(val).strip()
        for val in refs.values():
            if val is not None and str(val).strip():
                return str(val).strip()
    host = item.get("host")
    if isinstance(host, dict):
        name = host.get("name")
        if isinstance(name, str) and name.strip():
            return name.strip()
    fallback = item.get("id")
    if isinstance(fallback, str) and fallback.strip():
        return fallback.strip()
    return "-"


def truncate_reason(text: str, limit: int = 120) -> str:
    cleaned = clean_field(text) or "-"
    if len(cleaned) <= limit:
        return cleaned
    return cleaned[: limit - 1] + "…"


def next_command(item: dict) -> str:
    nxt = item.get("next")
    if isinstance(nxt, dict):
        for key in ("command", "tool", "summary"):
            val = nxt.get(key)
            if isinstance(val, str) and val.strip():
                return one_line(val)
    return "-"


def main() -> None:
    selector = sys.argv[1]
    owner = owner_host(selector)
    fast_items = read_attention(
        selector,
        {
            "limit": QUERY_LIMIT,
            "host": [owner],
            "exclude_kind": DIGEST_KINDS + OPERATOR_KINDS,
        },
    )
    crit_items = read_attention(
        selector,
        {
            "kind": DIGEST_KINDS,
            "severity": "critical",
            "host": [owner],
            "limit": QUERY_LIMIT,
        },
    )

    seen: set[str] = set()
    merged: list[dict] = []
    for item in fast_items + crit_items:
        if not isinstance(item, dict):
            continue
        fp = one_line(str(item.get("fingerprint") or item.get("id") or ""))
        if not fp or fp in seen:
            continue
        seen.add(fp)
        merged.append(item)

    merged.sort(
        key=lambda item: SEVERITY_ORDER.get(
            str(item.get("severity") or "").strip().lower(), 99
        )
    )

    for item in merged:
        fp = one_line(str(item.get("fingerprint") or item.get("id") or ""))
        kind = clean_field(str(item.get("kind") or "-")) or "-"
        severity = clean_field(str(item.get("severity") or "-")) or "-"
        ident = clean_field(item_id(item)) or "-"
        reason = truncate_reason(str(item.get("reason") or "-"))
        cmd = next_command(item)
        print(f"{fp}\t{kind} | {severity} | {ident} | {reason} | {cmd}")


if __name__ == "__main__":
    main()
PY
