"""Minimal OpenRouter client — stdlib only (urllib).

Usage:
    from app.config import load_config
    from app.llm_client import LLMClient, LLMError

    client = LLMClient(load_config())
    reply = client.complete(system="You are ...", user="Hello")
"""
from __future__ import annotations

import json
import re
import socket
import urllib.error
import urllib.request
from dataclasses import dataclass

from .config import AppConfig


class LLMError(RuntimeError):
    pass


@dataclass
class LLMResponse:
    content: str
    model: str
    raw: dict


class LLMClient:
    def __init__(self, cfg: AppConfig) -> None:
        self.cfg = cfg

    def available(self) -> bool:
        return bool(self.cfg.openrouter_api_key.strip())

    def complete(
        self,
        *,
        system: str,
        user: str,
        model: str | None = None,
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> LLMResponse:
        if not self.available():
            raise LLMError("OpenRouter API key not configured. Open Settings to add one.")

        model_name = model or self.cfg.primary_model
        try:
            return self._call(model_name, system, user, max_tokens, temperature)
        except LLMError:
            if self.cfg.fallback_model and self.cfg.fallback_model != model_name:
                return self._call(self.cfg.fallback_model, system, user, max_tokens, temperature)
            raise

    def _call(
        self,
        model_name: str,
        system: str,
        user: str,
        max_tokens: int | None,
        temperature: float | None,
    ) -> LLMResponse:
        url = self.cfg.openrouter_base_url.rstrip("/") + "/chat/completions"
        payload = {
            "model": model_name,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            "max_tokens": max_tokens or self.cfg.max_tokens,
            "temperature": self.cfg.temperature if temperature is None else temperature,
        }
        body = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=body,
            method="POST",
            headers={
                "Authorization": f"Bearer {self.cfg.openrouter_api_key}",
                "Content-Type": "application/json",
                "HTTP-Referer": "https://github.com/luckyegg168/agent-manager",
                "X-Title": "Agent Manager",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=self.cfg.request_timeout) as resp:
                data = json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", errors="replace")[:500]
            raise LLMError(f"HTTP {e.code} from OpenRouter: {detail}") from e
        except (urllib.error.URLError, socket.timeout) as e:
            raise LLMError(f"Network error: {e}") from e
        except json.JSONDecodeError as e:
            raise LLMError(f"Invalid JSON from OpenRouter: {e}") from e

        try:
            content = data["choices"][0]["message"]["content"]
        except (KeyError, IndexError, TypeError) as e:
            raise LLMError(f"Unexpected response shape: {data}") from e
        return LLMResponse(content=content, model=model_name, raw=data)

    def ping(self) -> str:
        """Quick health check used by Settings dialog."""
        if not self.available():
            return "未設定 API Key"
        try:
            reply = self.complete(
                system="You answer with a single word: ok",
                user="ping",
                max_tokens=10,
                temperature=0.0,
            )
            return f"OK （模型：{reply.model}）"
        except LLMError as e:
            return f"失敗：{e}"


# ---------- v1.2 高階輔助：AI 生成 / AI 局部修改 ----------

GENERATE_SYSTEM_PROMPT = """\
你是 Agent Skills (SKILL.md) 規格的資深技術編輯。使用者會提供一個 Agent 的
類別、名稱與一句話描述，請你產出該 Agent 的完整內容欄位。

規則：
1. 輸出必須是**純 JSON**（不加 ```json 圍欄、不加任何說明）。
2. JSON 欄位：
   {
     "description": "50–300 字；第一句為啟動時機（何時/情境觸發此 Agent），其後說明核心價值",
     "role": "一段角色設定，語氣專業且具體，2–4 句",
     "abilities": "核心能力清單，每行以 '- ' 開頭，至少 4 項，精簡具體",
     "body": "完整 Markdown 內文，必含章節：## 角色設定、## 核心能力、## 操作流程、## 輸入範例、## 輸出範例、## 邊緣案例處理、## 變更歷史"
   }
3. body 內文須包含（順序可照規格）：
   - ## 角色設定：沿用 role 內容或再擴寫
   - ## 核心能力：沿用 abilities 列表
   - ## 操作流程：條列 4–8 步驟
   - ## 輸入範例 / ## 輸出範例：各一個實際情境
   - ## 邊緣案例處理：至少 2 個邊緣狀況的處理方式
   - ## 變更歷史：僅需初版列，格式：`| v1.0.0 | YYYY-MM-DD | 初始建立 | — |`
4. 不要輸出 frontmatter（--- 區塊），frontmatter 由程式組裝。
5. 不要輸出任何 JSON 以外的字元。
"""


EDIT_SYSTEM_PROMPT = """\
你是 SKILL.md 內容的精準編輯器。使用者會提供：(1) 目前內容；(2) 修改指令；
(3) 範圍。請依指令只修改該範圍內相應的部分，**不要重寫無關段落**。

規則：
1. 輸出「修改後的完整範圍內容」，不要加任何說明或 diff 符號。
2. 範圍為 body 時：保留所有未指定要修改的章節原文。
3. 範圍為特定章節（例如 ## 核心能力）時：只輸出該章節標題+新內文。
4. 範圍為 description 時：輸出純文字（不含 Markdown 標題），50–300 字。
5. 絕不加 ```markdown 圍欄；絕不輸出 frontmatter。
"""


def _strip_json_fence(text: str) -> str:
    cleaned = text.strip()
    cleaned = re.sub(r"^```(?:json)?\s*\n", "", cleaned)
    cleaned = re.sub(r"\n```\s*$", "", cleaned)
    return cleaned.strip()


def _strip_md_fence(text: str) -> str:
    cleaned = text.strip()
    cleaned = re.sub(r"^```(?:markdown|md)?\s*\n", "", cleaned)
    cleaned = re.sub(r"\n```\s*$", "", cleaned)
    return cleaned


def generate_agent_draft(
    client: "LLMClient",
    *,
    category: str,
    name: str,
    brief: str,
    allowed_tools: str = "Read Write",
) -> dict:
    """Ask the LLM to draft an agent. Returns dict with keys:
    description, role, abilities, body, model.
    """
    user = (
        f"類別：{category}\n"
        f"Agent 名稱：{name}\n"
        f"allowed-tools：{allowed_tools}\n"
        f"一句話描述：{brief}\n\n"
        "請依規則輸出 JSON。"
    )
    reply = client.complete(system=GENERATE_SYSTEM_PROMPT, user=user, temperature=0.6)
    text = _strip_json_fence(reply.content)
    try:
        data = json.loads(text)
    except json.JSONDecodeError as e:
        raise LLMError(f"LLM 回傳非合法 JSON：{e}；內容片段：{text[:200]}") from e
    required = ("description", "role", "abilities", "body")
    missing = [k for k in required if not str(data.get(k, "")).strip()]
    if missing:
        raise LLMError(f"LLM 回傳缺少欄位：{', '.join(missing)}")
    data["model"] = reply.model
    return data


def edit_text_with_ai(
    client: "LLMClient",
    *,
    current: str,
    instruction: str,
    scope: str,
) -> tuple[str, str]:
    """Ask the LLM to edit `current` per `instruction`. Returns (new_text, model_used).

    `scope` is a label like "body" / "description" / "## 核心能力"，純作提示用途。
    """
    user = (
        f"--- 範圍 ---\n{scope}\n"
        f"--- 修改指令 ---\n{instruction}\n"
        f"--- 目前內容 ---\n{current}\n\n"
        "請輸出修改後的完整範圍內容。"
    )
    reply = client.complete(system=EDIT_SYSTEM_PROMPT, user=user, temperature=0.3)
    return _strip_md_fence(reply.content), reply.model
