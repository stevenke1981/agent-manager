"""Create new agents from a template — any existing SKILL.md can be a template."""
from __future__ import annotations

import re
import unicodedata
from copy import deepcopy
from datetime import datetime
from pathlib import Path

from .categories import AGENTS_ROOT
from .storage import AgentSkill, load_skill, new_skill_path

VAR_RE = re.compile(r"\{\{\s*([a-zA-Z_][\w\.]*)\s*\}\}")


BLANK_TEMPLATE_BODY = """\

# {{name}} Agent

## 角色設定
{{role}}

## 核心能力
{{abilities}}

## 操作流程
1. 接收使用者輸入
2. 分析需求並回應
3. 提供具體可執行建議

## 重要聲明
本 Agent 建議僅供參考，實務應用請依專業判斷。

## 輸入範例
```
{{input_example}}
```

## 輸出範例
```
{{output_example}}
```

## 邊緣案例處理
- 輸入不清楚：要求使用者補充
- 超出專業範圍：轉介至更合適的 Agent

## 變更歷史
| 版本 | 日期 | 內容 | 影響範圍 |
|------|------|------|----------|
| v1.0.0 | {{date}} | 初始建立 | — |
"""


def _slugify(name: str) -> str:
    """Produce a safe directory slug from any name (CJK-aware: keep Chinese, ascii-ify the rest)."""
    name = name.strip().lower()
    name = unicodedata.normalize("NFKC", name)
    name = re.sub(r"[\s/\\]+", "-", name)
    name = re.sub(r"[^\w\-\u4e00-\u9fff]", "", name)
    return name or f"agent-{datetime.now().strftime('%H%M%S')}"


def substitute(text: str, variables: dict[str, str]) -> str:
    def repl(match: re.Match) -> str:
        key = match.group(1)
        return str(variables.get(key, match.group(0)))

    return VAR_RE.sub(repl, text)


def find_best_template(category: str, keyword: str = "") -> Path | None:
    """Pick a likely-similar template under the same category; fall back to category's first agent."""
    category_dir = AGENTS_ROOT / category
    if not category_dir.exists():
        return None
    candidates = sorted(category_dir.rglob("SKILL.md"))
    if not candidates:
        return None
    if keyword:
        keyword = keyword.lower()
        for c in candidates:
            if keyword in c.parent.name.lower():
                return c
    return candidates[0]


def render_from_template(
    *,
    category: str,
    name: str,
    description: str,
    role: str = "",
    abilities: str = "",
    allowed_tools: str = "Read Write",
    template_path: Path | None = None,
    input_example: str = "",
    output_example: str = "",
) -> AgentSkill:
    slug = _slugify(name)
    target = new_skill_path(category, slug)

    variables = {
        "name": name,
        "description": description,
        "category": category,
        "role": role or f"你是專業的「{name}」。",
        "abilities": abilities or "- 核心能力 1\n- 核心能力 2\n- 核心能力 3",
        "date": datetime.now().strftime("%Y-%m-%d"),
        "input_example": input_example or "請描述您的需求...",
        "output_example": output_example or "（Agent 回覆內容）",
    }

    if template_path and template_path.exists():
        base = load_skill(template_path)
        frontmatter = deepcopy(base.frontmatter)
        body = base.body
    else:
        frontmatter = {}
        body = BLANK_TEMPLATE_BODY

    frontmatter["name"] = name
    frontmatter["description"] = description
    frontmatter["license"] = frontmatter.get("license", "MIT")
    metadata = frontmatter.get("metadata") or {}
    if not isinstance(metadata, dict):
        metadata = {}
    metadata["author"] = metadata.get("author", "agent-manager")
    metadata["version"] = "1.0.0"
    metadata["category"] = category
    metadata["language"] = metadata.get("language", "zh-TW")
    frontmatter["metadata"] = metadata
    frontmatter["compatibility"] = frontmatter.get(
        "compatibility",
        "Claude Code、VS Code Copilot、GitHub Copilot 及所有相容 Agent Skills 的平台",
    )
    frontmatter["allowed-tools"] = allowed_tools

    body = substitute(body, variables)

    return AgentSkill(path=target, frontmatter=frontmatter, body=body)
