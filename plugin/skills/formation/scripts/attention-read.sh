#!/usr/bin/env bash
# The sentry's read of Orbit attention for one workspace selector.
# Read-only. Prints one line per item:
#
#   <fingerprint><TAB><kind> | <severity> | <id> | <reason> | <next command>
#
# Usage: attention-read.sh <workspace selector>
# Exit 0 with no output means nothing needs attention. A non-zero exit with one
# line on stderr means the read failed or `orbit` is absent.
set -uo pipefail

if [ "$#" -ne 1 ] || [ -z "${1:-}" ]; then
  echo "usage: attention-read.sh <workspace selector>" >&2
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
import subprocess
import sys

EXCLUDED_KINDS = [
    "friction_untriaged",
    "host_unhealthy",
    "notice_open",
    "review_waiting",
]

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
    return payload["items"]


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
    fast_items = read_attention(
        selector,
        {"limit": 20, "exclude_kind": EXCLUDED_KINDS},
    )
    crit_items = read_attention(
        selector,
        {"kind": EXCLUDED_KINDS, "severity": "critical", "limit": 20},
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
