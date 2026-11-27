#!/usr/bin/env python3
"""Check the Formation skill budgets, links, role files and units.yaml schema."""

import argparse
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SKILL = ROOT / "plugin" / "skills" / "formation"

SKILL_MAX_BYTES = 16_000
SKILL_MAX_LINES = 500
REFERENCE_MAX_BYTES = 16_000
BUNDLED_DIRS = ("references", "scripts", "assets")
REQUIRED_ROLE_HEADINGS = (
    "First turn",
    "Duties",
    "You settle",
    "Messages",
    "Record",
    "Refresh and close",
)


def without_code(text):
    return re.sub(r"(?ms)^ {0,3}(```+|~~~+)[^\n]*\n.*?^ {0,3}\1[ \t]*$", "", text)


def heading_anchors(text):
    found = set()
    for line in re.findall(r"^ {0,3}#{1,6}[ \t]+(.+?)[ \t]*$", without_code(text), re.M):
        heading = re.sub(r"\[([^\]]+)\]\([^)]+\)|<[^>]+>", r"\1", line)
        slug = re.sub(r"[^\w -]", "", heading.strip().lower()).replace(" ", "-")
        candidate = slug
        suffix = 1
        while candidate in found:
            candidate = f"{slug}-{suffix}"
            suffix += 1
        found.add(candidate)
    return found


def level2_headings(text):
    return [
        match.strip()
        for match in re.findall(r"^ {0,3}##[ \t]+(.+?)[ \t]*$", without_code(text), re.M)
    ]


def local_links(text):
    cleaned = without_code(text)
    inline = re.findall(r"\]\(\s*(<[^>]+>|[^\s)]+)", cleaned)
    references = re.findall(r"^ {0,3}\[[^\]]+\]:\s*(<[^>]+>|\S+)", cleaned, re.M)
    seen = set()
    for link in inline + references:
        if link in seen:
            continue
        seen.add(link)
        target = urlsplit(link.strip("<>"))
        if not target.scheme and not target.netloc:
            yield link, unquote(target.path), unquote(target.fragment)


def strip_yaml_comment(line):
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


def unquote_scalar(value):
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        return value[1:-1]
    return value


def parse_inline_list(value):
    inner = value.strip()[1:-1].strip()
    if not inner:
        return []
    return [unquote_scalar(part) for part in inner.split(",") if part.strip()]


def parse_units_roles(path):
    """Parse role entries under `once:` and `ranks:` in assets/units.yaml."""
    roles = {}
    section = None
    role = None
    in_record = False
    in_record_tags = False
    record_indent = None

    for raw in path.read_text(encoding="utf-8").splitlines():
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
            roles[role] = {"section": section}
            in_record = False
            in_record_tags = False
            record_indent = None
            continue

        if role is None:
            continue

        if indent == 4 and ":" in stripped:
            in_record = False
            in_record_tags = False
            key, val = stripped.split(":", 1)
            key, val = key.strip(), val.strip()
            if key == "role":
                roles[role]["role"] = unquote_scalar(val)
            elif key == "record":
                if val:
                    roles[role]["record"] = unquote_scalar(val)
                else:
                    roles[role]["record"] = {}
                    in_record = True
                    record_indent = 6
            continue

        if in_record and indent >= record_indent:
            if in_record_tags and stripped.startswith("- "):
                roles[role]["record"]["tags"].append(unquote_scalar(stripped[2:]))
                continue
            if ":" in stripped:
                in_record_tags = False
                key, val = stripped.split(":", 1)
                key, val = key.strip(), val.strip()
                if key == "tags":
                    if val.startswith("[") and val.endswith("]"):
                        roles[role]["record"]["tags"] = parse_inline_list(val)
                    elif val:
                        roles[role]["record"]["tags"] = unquote_scalar(val)
                    else:
                        roles[role]["record"]["tags"] = []
                        in_record_tags = True
                else:
                    roles[role]["record"][key] = unquote_scalar(val)

    return roles


def check_skill(skill_dir):
    errors = []
    skill_dir = skill_dir.resolve()
    skill_md = skill_dir / "SKILL.md"
    if not skill_md.is_file():
        return [f"{skill_md}: missing SKILL.md"], {}

    skill_raw = skill_md.read_bytes()
    skill_text = skill_raw.decode("utf-8")
    skill_lines = len(skill_text.splitlines())
    if len(skill_raw) > SKILL_MAX_BYTES:
        errors.append(f"SKILL.md: {len(skill_raw)} bytes exceeds {SKILL_MAX_BYTES}-byte budget")
    if skill_lines > SKILL_MAX_LINES:
        errors.append(f"SKILL.md: {skill_lines} lines exceeds {SKILL_MAX_LINES}-line budget")

    ref_dir = skill_dir / "references"
    ref_files = sorted(
        path for path in ref_dir.rglob("*") if path.is_file() and not path.name.startswith(".")
    ) if ref_dir.is_dir() else []
    for ref in ref_files:
        rel = ref.relative_to(skill_dir).as_posix()
        size = ref.stat().st_size
        if size > REFERENCE_MAX_BYTES:
            errors.append(f"{rel}: {size} bytes exceeds {REFERENCE_MAX_BYTES}-byte budget")

    linked_from_skill = set()
    for link, rel_path, fragment in local_links(skill_text):
        target = (skill_md.parent / rel_path).resolve() if rel_path else skill_md
        try:
            rel_target = target.relative_to(skill_dir).as_posix()
        except ValueError:
            errors.append(f"SKILL.md: link escapes skill directory: {link}")
            continue
        if not target.exists():
            errors.append(f"SKILL.md: broken link: {link}")
            continue
        if rel_path:
            linked_from_skill.add(rel_target)
        if fragment and target.suffix.lower() == ".md":
            if fragment not in heading_anchors(target.read_text(encoding="utf-8")):
                errors.append(f"SKILL.md: link has no heading or anchor: {link}")

    for ref in ref_files:
        if ref.suffix.lower() != ".md":
            continue
        rel_ref = ref.relative_to(skill_dir).as_posix()
        ref_text = ref.read_text(encoding="utf-8")
        for link, rel_path, fragment in local_links(ref_text):
            target = (ref.parent / rel_path).resolve() if rel_path else ref
            try:
                target.relative_to(skill_dir)
            except ValueError:
                errors.append(f"{rel_ref}: link escapes skill directory: {link}")
                continue
            if not target.exists():
                errors.append(f"{rel_ref}: broken link: {link}")
                continue
            if fragment and target.suffix.lower() == ".md":
                if fragment not in heading_anchors(target.read_text(encoding="utf-8")):
                    errors.append(f"{rel_ref}: link has no heading or anchor: {link}")

    bundled_files = []
    for subdir in BUNDLED_DIRS:
        base = skill_dir / subdir
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if path.is_file() and not path.name.startswith("."):
                rel = path.relative_to(skill_dir).as_posix()
                bundled_files.append(rel)
                if rel not in linked_from_skill:
                    errors.append(f"{rel}: not linked from SKILL.md")

    units_path = skill_dir / "assets" / "units.yaml"
    if not units_path.is_file():
        errors.append("assets/units.yaml: missing unit definitions")
        return errors, {}

    roles = parse_units_roles(units_path)
    if not roles:
        errors.append("assets/units.yaml: no roles found under once or ranks")

    for role_name, info in roles.items():
        expected_rel = f"references/role-{role_name}.md"
        role_rel = info.get("role")
        if not role_rel:
            errors.append(f"{expected_rel}: missing role file for role '{role_name}' in assets/units.yaml")
        else:
            role_file = (skill_dir / role_rel).resolve()
            if role_rel != expected_rel:
                errors.append(f"{role_rel}: role '{role_name}' in assets/units.yaml must name {expected_rel}")
            if not role_file.is_file():
                errors.append(f"{role_rel}: missing role file for role '{role_name}' in assets/units.yaml")
            else:
                headings = level2_headings(role_file.read_text(encoding="utf-8"))
                missing = [h for h in REQUIRED_ROLE_HEADINGS if h not in headings]
                if missing:
                    for heading in missing:
                        errors.append(f"{role_rel}: missing required heading '{heading}'")
                else:
                    positions = [headings.index(h) for h in REQUIRED_ROLE_HEADINGS]
                    if positions != sorted(positions):
                        expected_order = ", ".join(REQUIRED_ROLE_HEADINGS)
                        errors.append(f"{role_rel}: required headings out of order (expected: {expected_order})")

        record = info.get("record")
        if record == "none":
            pass
        elif isinstance(record, dict):
            title = record.get("title")
            tags = record.get("tags")
            crew = record.get("crew")
            has_tags = (isinstance(tags, list) and len(tags) > 0) or (isinstance(tags, str) and bool(tags.strip()))
            if not (isinstance(title, str) and title.strip()) or not has_tags or not (isinstance(crew, str) and crew.strip()):
                errors.append(f"assets/units.yaml: role '{role_name}' record must define non-empty title, tags and crew")
        else:
            errors.append(f"assets/units.yaml: role '{role_name}' record must be 'none' or define title, tags and crew")

    stats = {
        "skill_bytes": len(skill_raw),
        "skill_lines": skill_lines,
        "bundled": len(bundled_files),
        "roles": len(roles),
    }
    return errors, stats


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "skill_dir",
        nargs="?",
        default=str(DEFAULT_SKILL),
        help="path to the skill directory to validate",
    )
    args = parser.parse_args()
    errors, stats = check_skill(Path(args.skill_dir))
    if errors:
        raise SystemExit("\n".join(errors))
    print(
        f"skill check: SKILL.md {stats['skill_bytes']} bytes / {stats['skill_lines']} lines, "
        f"{stats['bundled']} bundled files linked, {stats['roles']} roles checked"
    )


if __name__ == "__main__":
    main()
