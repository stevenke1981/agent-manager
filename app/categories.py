"""Category registry for the 20 built-in agent categories."""
from __future__ import annotations

from pathlib import Path

AGENTS_ROOT = Path(__file__).resolve().parent.parent / "agents"

# Fixed ordering — matches agents/ folder prefixes.
BUILTIN_CATEGORIES = [
    "01-醫療健康",
    "02-法律司法",
    "03-科技資訊",
    "04-金融商業",
    "05-教育學術",
    "06-媒體傳播",
    "07-政府公務",
    "08-藝術文創",
    "09-服務飲食",
    "10-商業策略",
    "11-製造工程",
    "12-醫美時尚",
    "13-宗教靈性",
    "14-爭議灰色行業",
    "15-成人娛樂業",
    "16-犯罪偵查",
    "17-極端組織分析",
    "18-網路地下",
    "19-特殊職業",
    "20-新興職業",
]


def list_categories() -> list[str]:
    """Return categories present on disk, ordered by folder prefix."""
    if not AGENTS_ROOT.exists():
        return list(BUILTIN_CATEGORIES)
    found = [p.name for p in AGENTS_ROOT.iterdir() if p.is_dir() and p.name[:2].isdigit()]
    return sorted(found, key=lambda n: int(n.split("-")[0]))


def ensure_category(category: str) -> Path:
    path = AGENTS_ROOT / category
    path.mkdir(parents=True, exist_ok=True)
    return path


def category_label(category: str) -> str:
    """'01-醫療健康' -> '醫療健康'."""
    return category.split("-", 1)[1] if "-" in category else category
