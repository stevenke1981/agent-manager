"""Install agents to AI tools and backup agents from tools.

Windows-only, experimental.
"""
from __future__ import annotations

from pathlib import Path
from typing import Callable

from .categories import ensure_category
from .storage import AgentSkill, save_skill
from .tool_registry import TOOL_BY_ID


def install_agents(
    skills: list[AgentSkill],
    tool_id: str,
    *,
    target: Path | None = None,
    progress_callback: Callable[[int, int, str], None] | None = None,
) -> tuple[int, int]:
    """Install skills to a tool. Returns (success_count, fail_count)."""
    tool = TOOL_BY_ID.get(tool_id)
    if not tool:
        return 0, len(skills)

    dest = target if target is not None else tool.default_path
    success = fail = 0

    for i, skill in enumerate(skills):
        try:
            tool.install(skill, dest)
            success += 1
        except Exception:
            fail += 1
        if progress_callback:
            progress_callback(i + 1, len(skills), skill.slug)

    return success, fail


def backup_from_tool(
    tool_id: str,
    *,
    source: Path | None = None,
    skip_existing: bool = True,
    progress_callback: Callable[[int, int, str], None] | None = None,
) -> tuple[int, int]:
    """Backup skills from a tool into agent-manager. Returns (imported, skipped)."""
    tool = TOOL_BY_ID.get(tool_id)
    if not tool:
        return 0, 0

    src = source if source is not None else tool.default_path
    skills = list(tool.backup_skills(src))
    imported = skipped = 0

    for i, skill in enumerate(skills):
        try:
            ensure_category(skill.path.parent.parent.name)
            if skip_existing and skill.path.exists():
                skipped += 1
            else:
                save_skill(skill, backup=False)
                imported += 1
        except Exception:
            skipped += 1
        if progress_callback:
            progress_callback(i + 1, len(skills), skill.slug)

    return imported, skipped
