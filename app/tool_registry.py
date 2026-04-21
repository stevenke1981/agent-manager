"""Tool registry — Windows-only, experimental.

Defines install paths and format converters for each supported AI tool.
Each ToolDef handles:
  install()       → write AgentSkill to tool's format
  backup_skills() → read tool's format back as AgentSkill iterator
"""
from __future__ import annotations

import os
import re
from dataclasses import dataclass
from datetime import date
from pathlib import Path
from typing import Iterator

from .storage import AgentSkill

# ──────────────────────────────────────────────────────────
# Platform paths
# ──────────────────────────────────────────────────────────

USERPROFILE = Path(os.environ.get("USERPROFILE", Path.home()))
APPDATA     = Path(os.environ.get("APPDATA",     USERPROFILE / "AppData" / "Roaming"))

# ──────────────────────────────────────────────────────────
# Helpers
# ──────────────────────────────────────────────────────────

_FRONTMATTER_RE = re.compile(r"^---\s*\n(.*?)\n---\s*\n(.*)$", re.DOTALL)

_COLOR_HEX: dict[str, str] = {
    "red": "#FF4444",    "blue": "#4488FF",  "green": "#44FF88",
    "cyan": "#00FFFF",   "purple": "#AA44FF","orange": "#FF8844",
    "yellow": "#FFFF44", "pink": "#FF88CC",  "white": "#FFFFFF",
    "black": "#222222",  "gray": "#888888",  "grey": "#888888",
    "teal": "#00AAAA",   "lime": "#88FF00",  "coral": "#FF6644",
    "gold": "#FFD700",   "silver": "#C0C0C0",
}


def _to_hex(color: str) -> str:
    if color.startswith("#"):
        return color
    return _COLOR_HEX.get(color.lower(), "#888888")


def _parse_fm(text: str) -> tuple[dict, str]:
    m = _FRONTMATTER_RE.match(text)
    if not m:
        return {}, text
    fm: dict = {}
    for line in m.group(1).splitlines():
        if ":" in line and not line.startswith(" "):
            k, _, v = line.partition(":")
            fm[k.strip()] = v.strip().strip('"').strip("'")
    return fm, m.group(2)


def _write_md(path: Path, fm: dict, body: str) -> None:
    lines = ["---"]
    for k, v in fm.items():
        s = str(v)
        if any(c in s for c in (':', '#', '\n')) or s.strip() != s:
            s = '"' + s.replace('"', '\\"') + '"'
        lines.append(f"{k}: {s}")
    lines.append("---")
    path.write_text("\n".join(lines) + "\n" + body, encoding="utf-8")


def _skill_name(skill: AgentSkill) -> str:
    return str(skill.frontmatter.get("name", skill.slug))


def _skill_desc(skill: AgentSkill) -> str:
    return str(skill.frontmatter.get("description", ""))


def _wrap_skill(path: Path, fm: dict, body: str, author: str, category: str) -> AgentSkill:
    return AgentSkill(
        path=path,
        frontmatter={
            "name": fm.get("name", path.parent.name),
            "description": fm.get("description", ""),
            "license": "MIT",
            "metadata": {
                "author": author,
                "version": "1.0",
                "category": category,
                "language": "en",
            },
            "compatibility": "Claude Code compatible",
            "allowed-tools": "Read Write",
            **{k: fm[k] for k in ("color", "emoji", "vibe") if k in fm},
        },
        body=body,
    )


# ──────────────────────────────────────────────────────────
# Base class
# ──────────────────────────────────────────────────────────

@dataclass
class ToolDef:
    id: str
    name: str
    description: str          # short path hint shown in UI
    default_path: Path        # default install / backup path
    project_scoped: bool = False  # True → user must supply a target project dir

    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        raise NotImplementedError

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        return iter([])


# ──────────────────────────────────────────────────────────
# Claude Code
# ──────────────────────────────────────────────────────────

class _ClaudeCodeTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        target.mkdir(parents=True, exist_ok=True)
        fm = {"name": _skill_name(skill), "description": _skill_desc(skill)}
        for k in ("color", "emoji", "vibe"):
            if skill.frontmatter.get(k):
                fm[k] = skill.frontmatter[k]
        out = target / f"{skill.slug}.md"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        for md in sorted(source.glob("*.md")):
            try:
                fm, body = _parse_fm(md.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "38-ClaudeCode" / md.stem / "SKILL.md",
                    fm, body, "claude-code-backup", "ClaudeCode",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# GitHub Copilot — VS Code Agent Mode (~/.github/agents/*.md)
# ──────────────────────────────────────────────────────────

class _CopilotVSCodeTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        target.mkdir(parents=True, exist_ok=True)
        fm = {"name": _skill_name(skill), "description": _skill_desc(skill)}
        for k in ("color", "emoji"):
            if skill.frontmatter.get(k):
                fm[k] = skill.frontmatter[k]
        out = target / f"{skill.slug}.md"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        for md in sorted(source.glob("*.md")):
            try:
                fm, body = _parse_fm(md.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "39-CopilotVSCode" / md.stem / "SKILL.md",
                    fm, body, "copilot-vscode-backup", "CopilotVSCode",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# GitHub Copilot — CLI / Desktop (~/.copilot/agents/*.agent.md)
# Frontmatter: name, description, tools (JSON array), optional model
# ──────────────────────────────────────────────────────────

_DEFAULT_COPILOT_TOOLS = '["edit", "create_file", "read_file", "run_command"]'


class _CopilotCLITool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        target.mkdir(parents=True, exist_ok=True)
        fm: dict = {
            "name": _skill_name(skill),
            "description": _skill_desc(skill),
            "tools": _DEFAULT_COPILOT_TOOLS,
        }
        out = target / f"{skill.slug}.agent.md"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        for md in sorted(source.glob("*.agent.md")):
            try:
                fm, body = _parse_fm(md.read_text(encoding="utf-8"))
                # strip .agent suffix to get clean slug
                slug = md.name.removesuffix(".agent.md")
                yield _wrap_skill(
                    AGENTS_ROOT / "50-CopilotCLI" / slug / "SKILL.md",
                    fm, body, "copilot-cli-backup", "CopilotCLI",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Antigravity
# ──────────────────────────────────────────────────────────

class _AntigravityTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        slug = f"agency-{skill.slug}"
        d = target / slug
        d.mkdir(parents=True, exist_ok=True)
        fm = {
            "name": slug,
            "description": _skill_desc(skill),
            "risk": "low",
            "source": "community",
            "date_added": str(date.today()),
        }
        out = d / "SKILL.md"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        if not source.exists():
            return
        for d in sorted(source.iterdir()):
            sm = d / "SKILL.md"
            if not sm.exists():
                continue
            try:
                fm, body = _parse_fm(sm.read_text(encoding="utf-8"))
                slug = str(fm.get("name", d.name)).removeprefix("agency-")
                yield _wrap_skill(
                    AGENTS_ROOT / "40-Antigravity" / slug / "SKILL.md",
                    fm, body, "antigravity-backup", "Antigravity",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Gemini CLI
# ──────────────────────────────────────────────────────────

_GEMINI_EXT = """{
  "name": "agency-agents",
  "version": "1.0.0",
  "description": "Agency Agents — installed via Agent Manager",
  "contextFileName": "SKILL.md"
}
"""


class _GeminiCLITool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        # target = ~/.gemini/extensions/agency-agents/
        d = target / "skills" / skill.slug
        d.mkdir(parents=True, exist_ok=True)
        fm = {"name": _skill_name(skill), "description": _skill_desc(skill)}
        out = d / "SKILL.md"
        _write_md(out, fm, skill.body)
        ext_json = target / "gemini-extension.json"
        if not ext_json.exists():
            ext_json.write_text(_GEMINI_EXT, encoding="utf-8")
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        skills_dir = source / "skills"
        if not skills_dir.exists():
            return
        for d in sorted(skills_dir.iterdir()):
            sm = d / "SKILL.md"
            if not sm.exists():
                continue
            try:
                fm, body = _parse_fm(sm.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "41-GeminiCLI" / d.name / "SKILL.md",
                    fm, body, "gemini-backup", "GeminiCLI",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# OpenCode (global: ~/.config/opencode/agents/)
# ──────────────────────────────────────────────────────────

class _OpenCodeTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        target.mkdir(parents=True, exist_ok=True)
        color_raw = str(skill.frontmatter.get("color", "gray"))
        fm = {
            "name": _skill_name(skill),
            "description": _skill_desc(skill),
            "mode": "subagent",
            "color": _to_hex(color_raw),
        }
        out = target / f"{skill.slug}.md"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        if not source.exists():
            return
        for md in sorted(source.glob("*.md")):
            try:
                fm, body = _parse_fm(md.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "42-OpenCode" / md.stem / "SKILL.md",
                    fm, body, "opencode-backup", "OpenCode",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Cursor (.mdc, project-scoped → <project>/.cursor/rules/)
# ──────────────────────────────────────────────────────────

class _CursorTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        rules = target / ".cursor" / "rules"
        rules.mkdir(parents=True, exist_ok=True)
        fm = {
            "description": _skill_desc(skill),
            "globs": "",
            "alwaysApply": "false",
        }
        out = rules / f"{skill.slug}.mdc"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        rules = source / ".cursor" / "rules"
        if not rules.exists():
            return
        for mdc in sorted(rules.glob("*.mdc")):
            try:
                fm, body = _parse_fm(mdc.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "43-Cursor" / mdc.stem / "SKILL.md",
                    {"name": mdc.stem, **fm}, body, "cursor-backup", "Cursor",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Aider (CONVENTIONS.md — consolidated, project-scoped)
# ──────────────────────────────────────────────────────────

class _AiderTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        conv = target / "CONVENTIONS.md"
        with conv.open("a", encoding="utf-8") as f:
            f.write(f"\n\n---\n\n## {_skill_name(skill)}\n\n{skill.body.strip()}\n")
        return [conv]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        conv = source / "CONVENTIONS.md"
        if not conv.exists():
            return
        body = conv.read_text(encoding="utf-8")
        yield AgentSkill(
            path=AGENTS_ROOT / "44-Aider" / "conventions" / "SKILL.md",
            frontmatter={
                "name": "Aider CONVENTIONS",
                "description": "Backed up from CONVENTIONS.md",
                "license": "MIT",
                "metadata": {"author": "aider-backup", "version": "1.0",
                             "category": "Aider", "language": "en"},
                "compatibility": "Aider compatible",
                "allowed-tools": "Read Write",
            },
            body=body,
        )


# ──────────────────────────────────────────────────────────
# Windsurf (.windsurfrules — consolidated, project-scoped)
# ──────────────────────────────────────────────────────────

class _WindsurfTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        rules = target / ".windsurfrules"
        with rules.open("a", encoding="utf-8") as f:
            f.write(f"\n\n---\n\n## {_skill_name(skill)}\n\n{skill.body.strip()}\n")
        return [rules]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        rules = source / ".windsurfrules"
        if not rules.exists():
            return
        body = rules.read_text(encoding="utf-8")
        yield AgentSkill(
            path=AGENTS_ROOT / "45-Windsurf" / "windsurfrules" / "SKILL.md",
            frontmatter={
                "name": "Windsurf Rules",
                "description": "Backed up from .windsurfrules",
                "license": "MIT",
                "metadata": {"author": "windsurf-backup", "version": "1.0",
                             "category": "Windsurf", "language": "en"},
                "compatibility": "Windsurf compatible",
                "allowed-tools": "Read Write",
            },
            body=body,
        )


# ──────────────────────────────────────────────────────────
# OpenClaw
# ──────────────────────────────────────────────────────────

class _OpenClawTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        ws = target / skill.slug
        ws.mkdir(parents=True, exist_ok=True)
        soul = ws / "SOUL.md"
        soul.write_text(
            f"---\nname: {_skill_name(skill)}\nversion: 1.0\n---\n\n{skill.body}",
            encoding="utf-8",
        )
        (ws / "IDENTITY.md").write_text(
            f"# {_skill_name(skill)}\n\n{_skill_desc(skill)}\n", encoding="utf-8"
        )
        return [soul]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        if not source.exists():
            return
        for ws in sorted(source.iterdir()):
            sm = ws / "SOUL.md"
            if not sm.exists():
                continue
            try:
                fm, body = _parse_fm(sm.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "46-OpenClaw" / ws.name / "SKILL.md",
                    {"name": fm.get("name", ws.name), **fm}, body,
                    "openclaw-backup", "OpenClaw",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Hermes Agent
# ──────────────────────────────────────────────────────────

class _HermesTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        d = target / skill.slug
        d.mkdir(parents=True, exist_ok=True)
        fm = {
            "name": _skill_name(skill),
            "description": _skill_desc(skill),
            "source": "agent-manager",
        }
        out = d / "SKILL.md"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        if not source.exists():
            return
        for d in sorted(source.iterdir()):
            sm = d / "SKILL.md"
            if not sm.exists():
                continue
            try:
                fm, body = _parse_fm(sm.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "47-Hermes" / d.name / "SKILL.md",
                    fm, body, "hermes-backup", "Hermes",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Qwen CLI (.qwen/agents/, project-scoped)
# ──────────────────────────────────────────────────────────

class _QwenTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        agents = target / ".qwen" / "agents"
        agents.mkdir(parents=True, exist_ok=True)
        fm = {"name": _skill_name(skill), "description": _skill_desc(skill)}
        out = agents / f"{skill.slug}.md"
        _write_md(out, fm, skill.body)
        return [out]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        agents = source / ".qwen" / "agents"
        if not agents.exists():
            return
        for md in sorted(agents.glob("*.md")):
            try:
                fm, body = _parse_fm(md.read_text(encoding="utf-8"))
                yield _wrap_skill(
                    AGENTS_ROOT / "48-QwenCLI" / md.stem / "SKILL.md",
                    fm, body, "qwen-backup", "QwenCLI",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Kimi CLI
# ──────────────────────────────────────────────────────────

_KIMI_YAML = """\
version: 1
agent:
  name: {slug}
  extend: default
  system_prompt_path: ./system.md
  tools:
    - "kimi_cli.tools.shell:Shell"
    - "kimi_cli.tools.file:ReadFile"
    - "kimi_cli.tools.file:WriteFile"
"""


class _KimiTool(ToolDef):
    def install(self, skill: AgentSkill, target: Path) -> list[Path]:
        d = target / skill.slug
        d.mkdir(parents=True, exist_ok=True)
        sys_md = d / "system.md"
        sys_md.write_text(skill.body, encoding="utf-8")
        yaml_path = d / "agent.yaml"
        yaml_path.write_text(_KIMI_YAML.format(slug=skill.slug), encoding="utf-8")
        return [sys_md, yaml_path]

    def backup_skills(self, source: Path) -> Iterator[AgentSkill]:
        from .categories import AGENTS_ROOT
        if not source.exists():
            return
        for d in sorted(source.iterdir()):
            sm = d / "system.md"
            if not sm.exists():
                continue
            try:
                body = sm.read_text(encoding="utf-8")
                name = d.name
                yp = d / "agent.yaml"
                if yp.exists():
                    for line in yp.read_text(encoding="utf-8").splitlines():
                        if line.strip().startswith("name:"):
                            name = line.split(":", 1)[1].strip()
                            break
                yield _wrap_skill(
                    AGENTS_ROOT / "49-KimiCLI" / d.name / "SKILL.md",
                    {"name": name}, body, "kimi-backup", "KimiCLI",
                )
            except Exception:
                continue


# ──────────────────────────────────────────────────────────
# Registry
# ──────────────────────────────────────────────────────────

TOOLS: list[ToolDef] = [
    _ClaudeCodeTool(
        id="claude-code", name="Claude Code",
        description="~/.claude/agents/",
        default_path=USERPROFILE / ".claude" / "agents",
    ),
    _CopilotVSCodeTool(
        id="copilot-vscode", name="Copilot (VS Code)",
        description="~/.github/agents/*.md",
        default_path=USERPROFILE / ".github" / "agents",
    ),
    _CopilotCLITool(
        id="copilot-cli", name="Copilot (CLI/Desktop)",
        description="~/.copilot/agents/*.agent.md",
        default_path=USERPROFILE / ".copilot" / "agents",
    ),
    _AntigravityTool(
        id="antigravity", name="Antigravity",
        description="~/.gemini/antigravity/skills/",
        default_path=USERPROFILE / ".gemini" / "antigravity" / "skills",
    ),
    _GeminiCLITool(
        id="gemini-cli", name="Gemini CLI",
        description="~/.gemini/extensions/agency-agents/",
        default_path=USERPROFILE / ".gemini" / "extensions" / "agency-agents",
    ),
    _OpenCodeTool(
        id="opencode", name="OpenCode",
        description="~/.config/opencode/agents/",
        default_path=USERPROFILE / ".config" / "opencode" / "agents",
    ),
    _CursorTool(
        id="cursor", name="Cursor",
        description="<專案>/.cursor/rules/",
        default_path=USERPROFILE / "Documents",
        project_scoped=True,
    ),
    _AiderTool(
        id="aider", name="Aider",
        description="<專案>/CONVENTIONS.md（合併）",
        default_path=USERPROFILE / "Documents",
        project_scoped=True,
    ),
    _WindsurfTool(
        id="windsurf", name="Windsurf",
        description="<專案>/.windsurfrules（合併）",
        default_path=USERPROFILE / "Documents",
        project_scoped=True,
    ),
    _OpenClawTool(
        id="openclaw", name="OpenClaw",
        description="~/.openclaw/agency-agents/",
        default_path=USERPROFILE / ".openclaw" / "agency-agents",
    ),
    _HermesTool(
        id="hermes", name="Hermes Agent",
        description="~/.hermes/skills/",
        default_path=USERPROFILE / ".hermes" / "skills",
    ),
    _QwenTool(
        id="qwen", name="Qwen CLI",
        description="<專案>/.qwen/agents/",
        default_path=USERPROFILE / "Documents",
        project_scoped=True,
    ),
    _KimiTool(
        id="kimi", name="Kimi CLI",
        description="~/.config/kimi/agents/",
        default_path=USERPROFILE / ".config" / "kimi" / "agents",
    ),
]

TOOL_BY_ID: dict[str, ToolDef] = {t.id: t for t in TOOLS}
