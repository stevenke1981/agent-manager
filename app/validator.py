"""Validate SKILL.md spec conformance. Reports issues with severity."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from .storage import AgentSkill

Severity = Literal["CRITICAL", "HIGH", "MEDIUM", "LOW"]

REQUIRED_FRONTMATTER = ["name", "description", "allowed-tools"]
REQUIRED_METADATA = ["version", "category"]
REQUIRED_SECTIONS = ["## 角色設定", "## 核心能力", "## 操作流程"]
RECOMMENDED_SECTIONS = ["## 輸入範例", "## 輸出範例", "## 邊緣案例處理", "## 變更歷史"]


@dataclass
class Issue:
    severity: Severity
    field: str
    message: str
    suggestion: str = ""


def validate(skill: AgentSkill) -> list[Issue]:
    issues: list[Issue] = []
    fm = skill.frontmatter or {}

    # filename / path
    if skill.path.name != "SKILL.md":
        issues.append(Issue("CRITICAL", "path", f"檔名應為 SKILL.md（目前 {skill.path.name}）"))

    # required frontmatter fields
    for key in REQUIRED_FRONTMATTER:
        if not fm.get(key):
            issues.append(Issue("CRITICAL", key, f"缺少必要 frontmatter 欄位：{key}",
                                f"請於 YAML 新增：{key}: ..."))

    metadata = fm.get("metadata") or {}
    if not isinstance(metadata, dict):
        issues.append(Issue("HIGH", "metadata", "metadata 應為字典（包含 version、category 等）"))
    else:
        for key in REQUIRED_METADATA:
            if not metadata.get(key):
                issues.append(Issue("HIGH", f"metadata.{key}",
                                    f"缺少 metadata.{key}",
                                    f'請於 metadata: 區塊新增 {key}: "..."'))

    # description length
    desc = str(fm.get("description") or "")
    if desc:
        if len(desc) < 30:
            issues.append(Issue("HIGH", "description",
                                f"description 過短（{len(desc)} 字），建議 50–300 字"))
        elif len(desc) > 500:
            issues.append(Issue("MEDIUM", "description",
                                f"description 過長（{len(desc)} 字），建議 50–300 字"))

    # required sections
    body = skill.body or ""
    for section in REQUIRED_SECTIONS:
        if section not in body:
            issues.append(Issue("HIGH", "body", f"缺少必要章節：{section}",
                                f"請新增 Markdown 章節：{section}"))

    for section in RECOMMENDED_SECTIONS:
        if section not in body:
            issues.append(Issue("LOW", "body", f"建議補齊章節：{section}"))

    # allowed-tools: warn if overly broad
    tools = str(fm.get("allowed-tools") or "")
    if tools and ("*" in tools or "all" in tools.lower()):
        issues.append(Issue("MEDIUM", "allowed-tools",
                            "allowed-tools 過度授權，建議明列工具名稱"))

    return issues


def summary(issues: list[Issue]) -> dict[str, int]:
    counts = {"CRITICAL": 0, "HIGH": 0, "MEDIUM": 0, "LOW": 0}
    for issue in issues:
        counts[issue.severity] = counts.get(issue.severity, 0) + 1
    return counts
