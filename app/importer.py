"""Import agents from agency-agents-main directory into agent-manager format.

Reads .md files with (name/description/color/emoji/vibe) frontmatter and
converts them to SKILL.md files stored under AGENTS_ROOT/<category>/<slug>/.
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Callable

from .categories import AGENTS_ROOT, ensure_category
from .storage import AgentSkill, save_skill

_FRONTMATTER_RE = re.compile(r"^---\s*\n(.*?)\n---\s*\n(.*)$", re.DOTALL)

DEFAULT_SOURCE = Path.home() / "agency-agents-main"

# Maps agency-agents-main top-level folder → numbered agent-manager category
CATEGORY_MAP: dict[str, str] = {
    "academic": "22-Academic",
    "design": "23-Design",
    "engineering": "24-Engineering",
    "finance": "25-Finance",
    "game-development": "26-GameDev",
    "integrations": "27-Integrations",
    "marketing": "28-Marketing",
    "paid-media": "29-PaidMedia",
    "product": "30-Product",
    "project-management": "31-ProjectMgmt",
    "sales": "32-Sales",
    "spatial-computing": "33-SpatialComp",
    "specialized": "34-Specialized",
    "strategy": "35-Strategy",
    "support": "36-Support",
    "testing": "37-Testing",
}

_SKIP_NAMES = {
    "README.md", "CONTRIBUTING.md", "SECURITY.md",
    "CONTRIBUTING_zh-CN.md", "PULL_REQUEST_TEMPLATE.md",
    "EXECUTIVE-BRIEF.md", "QUICKSTART.md",
}


def _parse_flat_frontmatter(text: str) -> tuple[dict, str]:
    """Parse top-level flat YAML frontmatter only (no nested blocks needed for source files)."""
    m = _FRONTMATTER_RE.match(text)
    if not m:
        return {}, text
    fm: dict = {}
    for line in m.group(1).splitlines():
        if ":" in line and not line.startswith(" "):
            k, _, v = line.partition(":")
            v = v.strip().strip('"').strip("'")
            fm[k.strip()] = v
    return fm, m.group(2)


def _collect_md_files(source_root: Path) -> list[tuple[Path, str]]:
    """Return sorted list of (md_path, category_name) for all importable files."""
    results: list[tuple[Path, str]] = []
    for folder_name, category in CATEGORY_MAP.items():
        folder = source_root / folder_name
        if not folder.exists():
            continue
        for md_file in sorted(folder.rglob("*.md")):
            if md_file.name in _SKIP_NAMES:
                continue
            results.append((md_file, category))
    return results


def _to_skill(md_path: Path, category: str) -> AgentSkill:
    """Convert a source .md file to an AgentSkill ready for saving."""
    text = md_path.read_text(encoding="utf-8")
    src_fm, body = _parse_flat_frontmatter(text)

    slug = md_path.stem
    skill_path = AGENTS_ROOT / category / slug / "SKILL.md"
    cat_label = category.split("-", 1)[1] if "-" in category else category

    fm: dict = {
        "name": src_fm.get("name", slug),
        "description": src_fm.get("description", ""),
        "license": "MIT",
        "metadata": {
            "author": "agency-agents",
            "version": "1.0",
            "category": cat_label,
            "language": "en",
        },
        "compatibility": "Claude Code compatible",
        "allowed-tools": "Read Write",
    }
    for key in ("color", "emoji", "vibe"):
        if src_fm.get(key):
            fm[key] = src_fm[key]

    return AgentSkill(path=skill_path, frontmatter=fm, body=body)


def count_importable(source_root: Path) -> int:
    """Return how many .md agent files exist in source_root."""
    return len(_collect_md_files(source_root))


def import_all(
    source_root: Path,
    *,
    skip_existing: bool = True,
    progress_callback: Callable[[int, int], None] | None = None,
) -> tuple[int, int]:
    """Import all agents from source_root into AGENTS_ROOT.

    Returns (imported_count, skipped_count).
    """
    files = _collect_md_files(source_root)
    total = len(files)
    imported = skipped = 0

    for i, (md_path, category) in enumerate(files):
        try:
            skill = _to_skill(md_path, category)
            if skip_existing and skill.path.exists():
                skipped += 1
            else:
                ensure_category(category)
                save_skill(skill, backup=False)
                imported += 1
        except Exception:
            skipped += 1
        if progress_callback:
            progress_callback(i + 1, total)

    return imported, skipped
