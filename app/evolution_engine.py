"""Self-evolution engine: scan → decide → fix (skeleton or API) → log.

v1.1 adds rule-based decision + optional OpenRouter-assisted rewrite.
"""
from __future__ import annotations

import json
import re
from dataclasses import asdict, dataclass
from datetime import datetime

from .categories import AGENTS_ROOT
from .config import AppConfig, load_config
from .evolution_rules import API_SYSTEM_PROMPT, RuleDecision, api_user_prompt, decide
from .llm_client import LLMClient, LLMError
from .storage import FRONTMATTER_RE, AgentSkill, _dump_yaml, _parse_yaml, list_skills, save_skill
from .validator import Issue, validate

EVOLUTION_LOG = AGENTS_ROOT / ".evolution.log"
SKELETON_SECTIONS: dict[str, str] = {
    "## 角色設定": "你是專業的 Agent，請依據使用者需求提供協助。",
    "## 核心能力": "- 核心能力 1\n- 核心能力 2\n- 核心能力 3",
    "## 操作流程": "1. 接收輸入\n2. 分析需求\n3. 回應建議",
    "## 輸入範例": "```\n請描述您的需求...\n```",
    "## 輸出範例": "```\n（Agent 回覆內容）\n```",
    "## 邊緣案例處理": "- 輸入不清：要求補充\n- 超出範圍：轉介",
    "## 變更歷史": (
        "| 版本 | 日期 | 內容 | 影響範圍 |\n"
        "|------|------|------|----------|\n"
        f"| v1.0.0 | {datetime.now().strftime('%Y-%m-%d')} | 初始建立 | — |"
    ),
}


@dataclass
class ScanResult:
    skill_path: str
    issues: list[dict]


@dataclass
class EvolutionRecord:
    timestamp: str
    skill_path: str
    action: str               # "skeleton" | "api" | "suggest" | "skip"
    mode_reason: str
    model: str
    fixed_issues: list[dict]
    remaining_issues: list[dict]
    error: str = ""


# -------- scan --------

def scan_all() -> list[ScanResult]:
    results: list[ScanResult] = []
    for skill in list_skills():
        issues = validate(skill)
        if issues:
            results.append(
                ScanResult(
                    skill_path=str(skill.path),
                    issues=[asdict(i) for i in issues],
                )
            )
    return results


# -------- fixers --------

def auto_fix_skeleton(skill: AgentSkill) -> tuple[list[Issue], list[Issue]]:
    """Append-only skeleton fix — never overwrites user content."""
    issues_before = validate(skill)
    fm = skill.frontmatter = dict(skill.frontmatter or {})
    body = skill.body or ""

    fm.setdefault("name", skill.path.parent.name)
    fm.setdefault(
        "description",
        f"{skill.path.parent.name} Agent — 請補充描述（50–300 字），包含啟動時機說明。",
    )
    fm.setdefault("license", "MIT")
    fm.setdefault("allowed-tools", "Read Write")
    fm.setdefault(
        "compatibility",
        "Claude Code、VS Code Copilot、GitHub Copilot 及所有相容 Agent Skills 的平台",
    )
    metadata = fm.get("metadata") or {}
    if not isinstance(metadata, dict):
        metadata = {}
    metadata.setdefault("author", "agent-manager")
    metadata.setdefault("version", "1.0.0")
    category_from_path = skill.path.parent.parent.name if skill.path.parent.parent else ""
    metadata.setdefault("category", category_from_path)
    metadata.setdefault("language", "zh-TW")
    fm["metadata"] = metadata

    for section, template in SKELETON_SECTIONS.items():
        if section not in body:
            body += f"\n\n{section}\n{template}\n"

    skill.body = body
    save_skill(skill, backup=True)
    issues_after = validate(skill)
    fixed = [i for i in issues_before if i not in issues_after]
    return fixed, issues_after


def auto_fix_api(
    skill: AgentSkill,
    issues: list[Issue],
    client: LLMClient,
    cfg: AppConfig,
) -> tuple[list[Issue], list[Issue], str]:
    """Ask the LLM to rewrite the SKILL.md. Returns (fixed, remaining, model_used)."""
    frontmatter_text = _dump_yaml(skill.frontmatter or {})
    reply = client.complete(
        system=API_SYSTEM_PROMPT,
        user=api_user_prompt(frontmatter_text, skill.body or "", issues),
    )
    new_fm, new_body = _parse_full_skill(reply.content)
    if not new_fm:
        raise LLMError("LLM 回傳格式不含 frontmatter，放棄套用")

    issues_before = validate(skill)
    skill.frontmatter = new_fm
    skill.body = new_body

    if cfg.evolution_dry_run:
        # do not persist
        issues_after_pseudo = validate(skill)
        fixed = [i for i in issues_before if i not in issues_after_pseudo]
        return fixed, issues_after_pseudo, reply.model

    save_skill(skill, backup=True)
    issues_after = validate(skill)

    if cfg.evolution_require_validation and issues_after:
        # Still log, but caller can inspect remaining issues.
        pass

    fixed = [i for i in issues_before if i not in issues_after]
    return fixed, issues_after, reply.model


def _parse_full_skill(text: str) -> tuple[dict, str]:
    """Extract frontmatter + body from an LLM-generated SKILL.md.

    Tolerates code fences accidentally wrapping the output.
    """
    cleaned = text.strip()
    cleaned = re.sub(r"^```(?:markdown|md)?\s*\n", "", cleaned)
    cleaned = re.sub(r"\n```\s*$", "", cleaned)
    match = FRONTMATTER_RE.match(cleaned)
    if not match:
        return {}, ""
    return _parse_yaml(match.group(1)), match.group(2)


# -------- main loop --------

def evolve_once(
    *,
    cfg: AppConfig | None = None,
    auto_apply: bool | None = None,
) -> list[EvolutionRecord]:
    """Run one evolution pass across all agents, applying rule decisions."""
    cfg = cfg or load_config()
    if auto_apply is not None:
        cfg.evolution_auto_apply = auto_apply

    client = LLMClient(cfg) if cfg.evolution_use_api else None
    records: list[EvolutionRecord] = []
    processed = 0

    for skill in list_skills():
        if processed >= cfg.evolution_max_agents_per_run:
            break
        issues = validate(skill)
        decision = decide(issues, cfg)
        if not decision.should_evolve:
            continue

        record = _apply_decision(skill, issues, decision, client, cfg)
        records.append(record)
        processed += 1

    _append_log(records)
    return records


def _apply_decision(
    skill: AgentSkill,
    issues: list[Issue],
    decision: RuleDecision,
    client: LLMClient | None,
    cfg: AppConfig,
) -> EvolutionRecord:
    now = datetime.now().isoformat(timespec="seconds")
    path = str(skill.path)

    if decision.mode == "skeleton":
        fixed, remaining = auto_fix_skeleton(skill)
        return EvolutionRecord(
            timestamp=now, skill_path=path, action="skeleton",
            mode_reason=decision.reason, model="",
            fixed_issues=[asdict(i) for i in fixed],
            remaining_issues=[asdict(i) for i in remaining],
        )

    if decision.mode == "api" and client is not None:
        try:
            fixed, remaining, model = auto_fix_api(skill, issues, client, cfg)
            return EvolutionRecord(
                timestamp=now, skill_path=path, action="api",
                mode_reason=decision.reason, model=model,
                fixed_issues=[asdict(i) for i in fixed],
                remaining_issues=[asdict(i) for i in remaining],
            )
        except LLMError as exc:
            # fall back to skeleton so we still make progress
            fixed, remaining = auto_fix_skeleton(skill)
            return EvolutionRecord(
                timestamp=now, skill_path=path, action="skeleton",
                mode_reason=f"api failed, fell back: {exc}", model="",
                fixed_issues=[asdict(i) for i in fixed],
                remaining_issues=[asdict(i) for i in remaining],
                error=str(exc),
            )

    # suggest only
    return EvolutionRecord(
        timestamp=now, skill_path=path, action="suggest",
        mode_reason=decision.reason, model="",
        fixed_issues=[],
        remaining_issues=[asdict(i) for i in issues],
    )


# -------- log --------

def _append_log(records: list[EvolutionRecord]) -> None:
    if not records:
        return
    AGENTS_ROOT.mkdir(parents=True, exist_ok=True)
    with EVOLUTION_LOG.open("a", encoding="utf-8") as f:
        for r in records:
            f.write(json.dumps(asdict(r), ensure_ascii=False) + "\n")


def read_log(limit: int = 50) -> list[dict]:
    if not EVOLUTION_LOG.exists():
        return []
    lines = EVOLUTION_LOG.read_text(encoding="utf-8").splitlines()
    return [json.loads(ln) for ln in lines[-limit:] if ln.strip()]
