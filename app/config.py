"""Persistent configuration for Agent Manager.

Stored at <repo>/.config.json — includes OpenRouter API key/models
and evolution preferences. Secret values are kept local to the repo
and should be gitignored by the user (.config.json is already ignored
by convention; add to .gitignore if needed).
"""
from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field, fields
from pathlib import Path

CONFIG_PATH = Path(__file__).resolve().parent.parent / ".config.json"

# Popular OpenRouter model identifiers as of 2026-04.
DEFAULT_MODELS = [
    "anthropic/claude-opus-4.7",
    "anthropic/claude-sonnet-4.6",
    "anthropic/claude-haiku-4.5",
    "openai/gpt-5",
    "openai/gpt-5-mini",
    "google/gemini-2.5-pro",
    "google/gemini-2.5-flash",
    "meta-llama/llama-4-maverick",
    "qwen/qwen3-max",
    "deepseek/deepseek-v3.1",
    "x-ai/grok-4",
    "mistralai/mistral-large-2",
]


@dataclass
class AppConfig:
    # OpenRouter
    openrouter_api_key: str = ""
    openrouter_base_url: str = "https://openrouter.ai/api/v1"
    primary_model: str = "anthropic/claude-haiku-4.5"
    fallback_model: str = "openai/gpt-5-mini"
    request_timeout: int = 60
    max_tokens: int = 2048
    temperature: float = 0.4

    # Evolution
    evolution_use_api: bool = False           # off by default — safe
    evolution_min_severity: str = "HIGH"      # CRITICAL | HIGH | MEDIUM | LOW
    evolution_auto_apply: bool = True
    evolution_require_validation: bool = True
    evolution_max_agents_per_run: int = 20
    evolution_dry_run: bool = False

    # Custom models list (merged with DEFAULT_MODELS in the GUI combobox)
    custom_models: list[str] = field(default_factory=list)


def load_config() -> AppConfig:
    if not CONFIG_PATH.exists():
        return AppConfig()
    try:
        data = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return AppConfig()
    return _from_dict(data)


def save_config(cfg: AppConfig) -> None:
    CONFIG_PATH.write_text(
        json.dumps(asdict(cfg), ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def _from_dict(data: dict) -> AppConfig:
    known = {f.name for f in fields(AppConfig)}
    cleaned = {k: v for k, v in data.items() if k in known}
    return AppConfig(**cleaned)


def all_models(cfg: AppConfig) -> list[str]:
    """Default + user-added, de-duped, preserving order."""
    seen: set[str] = set()
    merged: list[str] = []
    for m in DEFAULT_MODELS + list(cfg.custom_models):
        if m and m not in seen:
            seen.add(m)
            merged.append(m)
    return merged
