"""High-level CRUD orchestration, used by the GUI."""
from __future__ import annotations

from pathlib import Path

from .categories import list_categories
from .storage import AgentSkill, delete_skill, list_skills, load_skill, save_skill
from .template_engine import find_best_template, render_from_template
from .validator import Issue, summary, validate


class AgentManager:
    def categories(self) -> list[str]:
        return list_categories()

    def list(self, category: str | None = None) -> list[AgentSkill]:
        return list_skills(category)

    def open(self, path: Path) -> AgentSkill:
        return load_skill(path)

    def save(self, skill: AgentSkill) -> list[Issue]:
        save_skill(skill, backup=True)
        return validate(skill)

    def delete(self, skill: AgentSkill) -> None:
        delete_skill(skill)

    def create(
        self,
        *,
        category: str,
        name: str,
        description: str,
        role: str = "",
        abilities: str = "",
        allowed_tools: str = "Read Write",
        template_keyword: str = "",
    ) -> AgentSkill:
        template_path = find_best_template(category, keyword=template_keyword or name)
        skill = render_from_template(
            category=category,
            name=name,
            description=description,
            role=role,
            abilities=abilities,
            allowed_tools=allowed_tools,
            template_path=template_path,
        )
        save_skill(skill, backup=False)
        return skill

    # ---------- v1.2 AI 生成/編輯門面 ----------

    def create_from_ai_draft(
        self,
        *,
        category: str,
        name: str,
        allowed_tools: str,
        draft: dict,
    ) -> AgentSkill:
        """將 llm_client.generate_agent_draft() 產出的 draft 落地為 SKILL.md。

        draft 預期包含：description / role / abilities / body。
        body 會直接覆蓋模板內文（已由 LLM 依規格產出全章節）。
        """
        skill = render_from_template(
            category=category,
            name=name,
            description=str(draft.get("description", "")).strip(),
            role=str(draft.get("role", "")).strip(),
            abilities=str(draft.get("abilities", "")).strip(),
            allowed_tools=allowed_tools or "Read Write",
            template_path=None,  # AI 已產出完整 body，不套用既有模板
        )
        body = str(draft.get("body", "")).strip()
        if body:
            skill.body = body + "\n"
        save_skill(skill, backup=False)
        return skill

    def validate(self, skill: AgentSkill) -> tuple[list[Issue], dict[str, int]]:
        issues = validate(skill)
        return issues, summary(issues)
