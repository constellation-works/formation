#!/usr/bin/env bash
# Compare each Formation shift record's crew, orchestrator, and required role tags
# against the roster. Read-only. Prints one line per differing record:
#
#   <fingerprint><TAB>record_mismatch | medium | <task id> | <details> | <next command>
#
# Usage: record-check.sh <workspace selector> <task id>=[<role>:]<crew> ...
#
# Every read is pinned to the selector's owner host (`hm_<owner>/ws_<name>`).
# Each record argument accepts `<task id>=<crew>` (checks `crew` and `orchestrator`)
# or `<task id>=<role>:<crew>` (also checks literal required tags for `<role>` in
# `assets/units.yaml`, skipping `<...>` placeholders).
# Exit 0 with no output means every record matches. A non-zero exit with one line
# on stderr and nothing on stdout means invalid arguments (exit 2), `orbit` or
# `python3` is absent, or a read failed.
set -uo pipefail

if [ "$#" -lt 2 ] || [ -z "${1:-}" ]; then
  echo "usage: record-check.sh <workspace selector> <task id>=[<role>:]<crew> ..." >&2
  exit 2
fi

if ! command -v orbit >/dev/null 2>&1; then
  echo "record-check.sh: orbit is not installed on PATH" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "record-check.sh: python3 is not installed on PATH" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

exec python3 - "$script_dir" "$@" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import re
import subprocess
import sys

# `hm_<owner>/ws_<name>`, as `orbit.workspace.list` reports a workspace.
OWNER_SELECTOR = re.compile(r"(hm_[A-Za-z0-9_-]+)/ws_[A-Za-z0-9_-]+")


def one_line(text: str) -> str:
    return " ".join(text.split())


def clean_field(value: str) -> str:
    return one_line(value.replace("|", " "))


def fail(message: str, code: int = 1) -> None:
    line = one_line(message) or "record check failed"
    print(f"record-check.sh: {line}", file=sys.stderr)
    sys.exit(code)


def checked_selector(selector: str) -> str:
    if OWNER_SELECTOR.fullmatch(selector) is None:
        fail(
            "the selector must be host-qualified (hm_<owner>/ws_<name>) "
            "so every read pins to its owner",
            code=2,
        )
    return selector


def strip_yaml_comment(line: str) -> str:
    in_single = False
    in_double = False
    escaped = False
    for idx, ch in enumerate(line):
        if escaped:
            escaped = False
            continue
        if ch == "\\" and in_double:
            escaped = True
            continue
        if ch == "'" and not in_double:
            in_single = not in_single
        elif ch == '"' and not in_single:
            in_double = not in_double
        elif ch == "#" and not in_single and not in_double:
            if idx == 0 or line[idx - 1].isspace():
                return line[:idx].rstrip()
    return line.rstrip()


def unquote_scalar(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        return value[1:-1]
    return value


def parse_inline_list(value: str) -> list[str]:
    inner = value.strip()[1:-1].strip()
    if not inner:
        return []
    return [unquote_scalar(part) for part in inner.split(",") if part.strip()]


def load_role_tags(units_path: Path) -> dict[str, list[str] | None]:
    if not units_path.is_file():
        fail(f"missing unit definitions at {units_path}")

    roles: dict[str, list[str] | None] = {}
    section: str | None = None
    role: str | None = None
    in_record = False
    in_record_tags = False
    record_indent = 6

    for raw in units_path.read_text(encoding="utf-8").splitlines():
        line = strip_yaml_comment(raw)
        if not line.strip():
            continue
        indent = len(line) - len(line.lstrip(" "))
        stripped = line.strip()

        if indent == 0:
            section = stripped[:-1].strip() if stripped.endswith(":") else None
            role = None
            in_record = False
            in_record_tags = False
            continue

        if section not in {"once", "ranks"}:
            continue

        if indent == 2 and stripped.endswith(":"):
            role = stripped[:-1].strip()
            roles[role] = None
            in_record = False
            in_record_tags = False
            continue

        if role is None:
            continue

        if indent == 4 and ":" in stripped:
            in_record = False
            in_record_tags = False
            key, val = stripped.split(":", 1)
            key, val = key.strip(), val.strip()
            if key == "record":
                if val == "none":
                    roles[role] = None
                elif not val:
                    roles[role] = []
                    in_record = True
            continue

        if in_record and indent >= record_indent:
            if in_record_tags and stripped.startswith("- "):
                tag = unquote_scalar(stripped[2:])
                if tag and not (tag.startswith("<") and tag.endswith(">")):
                    current = roles[role]
                    if current is not None:
                        current.append(tag)
                continue
            if ":" in stripped:
                in_record_tags = False
                key, val = stripped.split(":", 1)
                key, val = key.strip(), val.strip()
                if key == "tags":
                    if val.startswith("[") and val.endswith("]"):
                        roles[role] = [
                            tag
                            for tag in parse_inline_list(val)
                            if tag and not (tag.startswith("<") and tag.endswith(">"))
                        ]
                    elif val:
                        roles[role] = []
                    else:
                        roles[role] = []
                        in_record_tags = True

    return roles


def parse_spec(raw: str) -> tuple[str, str | None, str]:
    if "=" not in raw:
        fail(f"invalid record argument {raw!r}; expected <task id>=[<role>:]<crew>", code=2)
    task_id, rhs = raw.split("=", 1)
    if not task_id or not rhs or any(ch.isspace() for ch in task_id) or any(ch.isspace() for ch in rhs):
        fail(f"invalid record argument {raw!r}; expected <task id>=[<role>:]<crew>", code=2)
    if ":" in rhs:
        role, crew = rhs.split(":", 1)
        if not role or not crew:
            fail(f"invalid record argument {raw!r}; expected <task id>=[<role>:]<crew>", code=2)
        return task_id, role, crew
    return task_id, None, rhs


def extract_error(stdout: str, stderr: str, returncode: int) -> str:
    for stream in (stderr, stdout):
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


def read_task(selector: str, task_id: str) -> dict:
    query = {
        "id": task_id,
        "fields": ["id", "crew", "orchestrator", "tags"],
    }
    proc = subprocess.run(
        [
            "orbit",
            "--workspace",
            selector,
            "tool",
            "run",
            "orbit.task.show",
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
        fail(extract_error(proc.stdout, proc.stderr, proc.returncode) or "invalid JSON from orbit.task.show")
    if not isinstance(payload, dict):
        fail("unexpected response from orbit.task.show")
    err = payload.get("error")
    if isinstance(err, str) and err.strip():
        fail(err)
    return payload


def check_record(selector: str, task_id: str, expected_crew: str, required_tags: list[str] | None) -> str | None:
    task = read_task(selector, task_id)
    mismatches: list[str] = []

    raw_crew = task.get("crew")
    found_crew = raw_crew.strip() if isinstance(raw_crew, str) and raw_crew.strip() else "(unset)"
    if found_crew != expected_crew:
        mismatches.append(f"crew={found_crew} (expected {expected_crew})")

    raw_orch = task.get("orchestrator")
    found_orch = raw_orch.strip() if isinstance(raw_orch, str) and raw_orch.strip() else "(unset)"
    if found_orch != expected_crew:
        mismatches.append(f"orchestrator={found_orch} (expected {expected_crew})")

    if required_tags:
        raw_tags = task.get("tags")
        if raw_tags is None:
            present_tags: set[str] = set()
        elif isinstance(raw_tags, list):
            present_tags = {str(tag).strip() for tag in raw_tags if str(tag).strip()}
        else:
            fail(f"{task_id}: unexpected tags field from orbit.task.show")
        missing = [tag for tag in required_tags if tag not in present_tags]
        if missing:
            mismatches.append(f"missing tags: {', '.join(missing)}")

    if not mismatches:
        return None

    fp = f"record:{one_line(task_id)}"
    details = clean_field("; ".join(mismatches))
    cmd = f"orbit --workspace {selector} task show {one_line(task_id)}"
    return f"{fp}\trecord_mismatch | medium | {clean_field(task_id)} | {details} | {cmd}"


def main() -> None:
    script_dir = Path(sys.argv[1])
    selector = checked_selector(sys.argv[2])
    specs = [parse_spec(raw) for raw in sys.argv[3:]]

    role_tags: dict[str, list[str] | None] = {}
    if any(role is not None for _, role, _ in specs):
        role_tags = load_role_tags(script_dir.parent / "assets" / "units.yaml")
        for _, role, _ in specs:
            if role is None:
                continue
            if role not in role_tags:
                fail(f"unknown role {role!r} in assets/units.yaml", code=2)
            if role_tags[role] is None:
                fail(f"role {role!r} has record: none in assets/units.yaml", code=2)

    findings: list[str] = []
    for task_id, role, expected_crew in specs:
        required_tags = role_tags.get(role) if role is not None else None
        line = check_record(selector, task_id, expected_crew, required_tags)
        if line is not None:
            findings.append(line)

    for line in findings:
        print(line)


if __name__ == "__main__":
    main()
PY
