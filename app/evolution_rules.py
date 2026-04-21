"""Evolution condition rules — determines whether/how to evolve an agent."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable

from .config import AppConfig
from .validator import Issue

SEVERITY_ORDER = {"LOW": 0, "MEDIUM": 1, "HIGH": 2, "CRITICAL": 3}


@dataclass
class RuleDecision:
    should_evolve: bool
    mode: str              # "skeleton" | "api" | "suggest" | "skip"
    reason: str


def decide(issues: list[Issue], cfg: AppConfig) -> RuleDecision:
    """Decide how to handle one agent given its validator issues + config.

    Rules (in order):
      1. No issues → skip
      2. Max severity < min threshold → suggest only
      3. API mode enabled + key set → api (LLM-assisted rewrite)
      4. Auto apply enabled → skeleton (safe defaults)
      5. Otherwise → suggest only
    """
    if not issues:
        return RuleDecision(False, "skip", "no issues")

    max_sev = max(SEVERITY_ORDER.get(i.severity, 0) for i in issues)
    threshold = SEVERITY_ORDER.get(cfg.evolution_min_severity.upper(), 2)
    if max_sev < threshold:
        return RuleDecision(
            True, "suggest",
            f"max severity below threshold ({cfg.evolution_min_severity})",
        )

    if cfg.evolution_use_api and cfg.openrouter_api_key.strip():
        return RuleDecision(True, "api", "api mode enabled")

    if cfg.evolution_auto_apply:
        return RuleDecision(True, "skeleton", "auto-apply skeleton fix")

    return RuleDecision(True, "suggest", "auto-apply disabled")


def cap_agents(decisions: Iterable[tuple], cfg: AppConfig) -> list:
    """Limit number of agents processed per run."""
    decisions = list(decisions)
    limit = max(1, cfg.evolution_max_agents_per_run)
    return decisions[:limit]


API_SYSTEM_PROMPT = """\
你是 Agent Skills (SKILL.md) 規格的資深技術編輯。使用者會給你一份 SKILL.md
的 frontmatter 與 body、以及驗證器回報的問題清單。請依規格修復問題。

規則：
1. 輸出必須是完整的 SKILL.md，從 `---` 開頭、`---` 結束 frontmatter，接著 Markdown body。
2. 保留使用者既有內容，只補齊缺失欄位/章節；不要縮短或重寫使用者實質內容。
3. frontmatter 必含：name、description、license、metadata (author/version/category/language)、
   compatibility、allowed-tools。description 50–300 字且含啟動時機。
4. body 必含章節：## 角色設定、## 核心能力、## 操作流程；建議章節 ## 輸入範例、
   ## 輸出範例、## 邊緣案例處理、## 變更歷史。
5. 不新增不必要的章節或裝飾文字。
6. 不輸出解說、不加程式碼圍欄、只輸出 SKILL.md 本身。
"""


def api_user_prompt(frontmatter_text: str, body: str, issues: list[Issue]) -> str:
    bullet = "\n".join(f"- [{i.severity}] {i.field}: {i.message}" for i in issues)
    return f"""以下是目前的 SKILL.md：

--- frontmatter ---
{frontmatter_text}
--- body ---
{body}
--- 問題清單 ---
{bullet}

請輸出修復後的完整 SKILL.md（從 --- 開頭）。
"""
