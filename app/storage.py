"""Read/write SKILL.md files with YAML frontmatter — no external deps."""
from __future__ import annotations

import os
import re
import shutil
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

from .categories import AGENTS_ROOT

FRONTMATTER_RE = re.compile(r"^---\s*\n(.*?)\n---\s*\n(.*)$", re.DOTALL)


@dataclass
class AgentSkill:
    path: Path
    frontmatter: dict = field(default_factory=dict)
    body: str = ""

    @property
    def name(self) -> str:
        return str(self.frontmatter.get("name", self.path.parent.name))

    @property
    def description(self) -> str:
        return str(self.frontmatter.get("description", ""))

    @property
    def category(self) -> str:
        meta = self.frontmatter.get("metadata", {}) or {}
        return str(meta.get("category", ""))

    @property
    def slug(self) -> str:
        return self.path.parent.name


# ---------- tiny YAML (subset sufficient for SKILL.md frontmatter) ----------

def _parse_scalar(value: str):
    value = value.strip()
    if not value:
        return ""
    if value.startswith('"') and value.endswith('"'):
        return value[1:-1]
    if value.startswith("'") and value.endswith("'"):
        return value[1:-1]
    if value.lower() in ("true", "false"):
        return value.lower() == "true"
    if value.lower() in ("null", "~"):
        return None
    return value


def _parse_yaml(text: str) -> dict:
    """Parse a very small subset of YAML: top-level keys, nested 'metadata:' block, scalar values."""
    result: dict = {}
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.rstrip()
        if not stripped or stripped.lstrip().startswith("#"):
            i += 1
            continue
        if not line.startswith(" "):
            if ":" not in line:
                i += 1
                continue
            key, _, rest = line.partition(":")
            key = key.strip()
            rest = rest.strip()
            if not rest:
                # nested block
                block: dict = {}
                i += 1
                while i < len(lines) and (lines[i].startswith("  ") or not lines[i].strip()):
                    sub = lines[i]
                    if not sub.strip():
                        i += 1
                        continue
                    if ":" in sub:
                        sk, _, sv = sub.strip().partition(":")
                        block[sk.strip()] = _parse_scalar(sv)
                    i += 1
                result[key] = block
            else:
                result[key] = _parse_scalar(rest)
                i += 1
        else:
            i += 1
    return result


def _dump_scalar(value) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    s = str(value)
    # quote if contains colon, leading special char, or newline
    if any(ch in s for ch in [":", "#", "\n"]) or s.strip() != s:
        return '"' + s.replace('"', '\\"') + '"'
    return s


def _dump_yaml(data: dict) -> str:
    out: list[str] = []
    for key, value in data.items():
        if isinstance(value, dict):
            out.append(f"{key}:")
            for sk, sv in value.items():
                out.append(f"  {sk}: {_dump_scalar(sv)}")
        else:
            out.append(f"{key}: {_dump_scalar(value)}")
    return "\n".join(out)


# ---------- file ops ----------

def load_skill(path: Path) -> AgentSkill:
    text = path.read_text(encoding="utf-8")
    m = FRONTMATTER_RE.match(text)
    if not m:
        return AgentSkill(path=path, frontmatter={}, body=text)
    fm = _parse_yaml(m.group(1))
    return AgentSkill(path=path, frontmatter=fm, body=m.group(2))


def save_skill(skill: AgentSkill, *, backup: bool = True) -> None:
    skill.path.parent.mkdir(parents=True, exist_ok=True)
    if backup and skill.path.exists():
        backup_dir = AGENTS_ROOT.parent / ".backup" / datetime.now().strftime("%Y%m%d-%H%M%S")
        backup_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(skill.path, backup_dir / f"{skill.path.parent.name}.SKILL.md")
    content = "---\n" + _dump_yaml(skill.frontmatter) + "\n---\n" + skill.body
    tmp = skill.path.with_suffix(".tmp")
    try:
        tmp.write_text(content, encoding="utf-8")
        os.replace(tmp, skill.path)
    finally:
        if tmp.exists():
            tmp.unlink(missing_ok=True)


def list_skills(category: str | None = None) -> list[AgentSkill]:
    root = AGENTS_ROOT if category is None else AGENTS_ROOT / category
    skills: list[AgentSkill] = []
    if not root.exists():
        return skills
    for skill_path in root.rglob("SKILL.md"):
        try:
            skills.append(load_skill(skill_path))
        except Exception:
            continue
    skills.sort(key=lambda s: s.path)
    return skills


def delete_skill(skill: AgentSkill) -> None:
    backup_dir = AGENTS_ROOT.parent / ".backup" / datetime.now().strftime("%Y%m%d-%H%M%S")
    backup_dir.mkdir(parents=True, exist_ok=True)
    if skill.path.exists():
        shutil.copy2(skill.path, backup_dir / f"{skill.path.parent.name}.SKILL.md")
    agent_dir = skill.path.parent
    if agent_dir.exists():
        shutil.rmtree(agent_dir)


def new_skill_path(category: str, slug: str) -> Path:
    return AGENTS_ROOT / category / slug / "SKILL.md"
