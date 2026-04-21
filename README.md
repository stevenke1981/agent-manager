# Agent Manager

管理 `agents/` 目錄下 **100+ 職業 Agent**（AgentSkills SKILL.md 規格）的 Python GUI 應用。支援**模板快速建立**、**編輯驗證**，以及**自我進化**（掃描 → 找出問題 → 自動糾正 → 改進）。

## 特性
- **分類樹瀏覽** — 21 大類、106+ Agent 一覽（v1.1 新增 AI 生成類）
- **模板建立** — 自動挑選相近 Agent 作為模板，變數替換一鍵產出
- **規格驗證** — 檢查 frontmatter 必填欄位、必要章節、description 長度等
- **規則式自我進化（v1.1）** — 骨架補齊 / OpenRouter API 重寫 / 僅建議 三模式；可設嚴重度門檻、每輪上限、dry run
- **OpenRouter 設定（v1.1）** — 主/備援模型、自訂模型清單、連線測試
- **AI 快速生成 Agent（v1.2）** — 一句話 brief → 自動填入 description/role/abilities/body，使用者可編輯後建立
- **AI 局部編輯（v1.2）** — 選擇範圍（整份 body / description / 單一章節）+ 指令 → 預覽修改 → 套用（自動備份）
- **零外部依賴** — 只用 Python 標準庫（含 tkinter + urllib）

## 快速開始

### 需求
- Python 3.10+（含 tkinter，Windows/macOS 預設內建）

### 啟動 GUI
Windows：
```cmd
start_gui.bat
```

跨平台：
```bash
python -m app.main
```

## 目錄結構
```
agent-manager/
├─ agent.md                   # Meta-Agent 定義
├─ ITERATIVE_DEV_CORE.md      # 迭代狀態總覽
├─ plan.md                    # 規劃與里程碑
├─ blueprint.md               # 架構藍圖
├─ knowledge_graph.md         # 模組關聯圖
├─ README.md                  # 本檔
├─ requirements.txt           # (空 — 無外部依賴)
├─ start_gui.bat              # Windows 啟動腳本
├─ app/
│  ├─ main.py                 # tkinter 介面入口（v1.1 含 SettingsDialog）
│  ├─ agent_manager.py        # CRUD 門面
│  ├─ categories.py           # 類別註冊
│  ├─ storage.py              # SKILL.md 讀寫 + YAML 子集
│  ├─ validator.py            # 規格驗證 + 問題分級
│  ├─ template_engine.py      # 模板 + 變數替換
│  ├─ evolution_engine.py     # 自我進化循環（v1.1 規則引擎）
│  ├─ evolution_rules.py      # v1.1 規則決策
│  ├─ config.py               # v1.1 AppConfig 持久化
│  └─ llm_client.py           # v1.1 OpenRouter HTTP client
├─ .config.json               # v1.1 設定檔（含 API Key，請 gitignore）
└─ agents/                    # 106+ 職業 Agent（SKILL.md）
   ├─ 01-醫療健康/ ~ 20-新興職業/   # v1.0 既有
   └─ 21-AI生成/                  # v1.1 新增（圖/影/音/3D/LLM/Agent）
```

## 主要功能

### 建立新 Agent
1. 工具列「**新增 Agent**」
2. 選擇類別、輸入名稱、description（50–300 字、含啟動時機）
3. 可選填模板關鍵字（若留空，自動挑該類別第一個 Agent 當模板）
4. 按「建立」→ 自動寫入 `agents/<category>/<slug>/SKILL.md` 並聚焦

### AI 生成新 Agent（v1.2）
1. 工具列「**新增 Agent**」開啟對話框
2. 填入類別、名稱、allowed-tools、**AI 提示（一句話描述需求）**
3. 按「**AI 生成內容**」→ OpenRouter 會自動填入：
   - description（50–300 字、含啟動時機）
   - 角色設定
   - 核心能力（條列）
   - 完整 body（含所有建議章節）
4. 使用者可再編輯任何欄位後按「建立」

### AI 局部修改（v1.2）
1. 左側選定 Agent；工具列「**AI 局部修改**」
2. 選擇範圍（整份 body / description / 單一章節）
3. 輸入指令（例如「為 ## 操作流程 增加風險評估步驟」）
4. 按「生成」→ 預覽 AI 輸出；可直接編輯
5. 按「套用」→ 寫入檔案並備份至 `.backup/`

### 編輯 Agent
- 左側樹狀點選 Agent → 右側顯示 frontmatter 表單 + Markdown 內文
- 修改後按「**儲存**」（自動備份至 `.backup/<timestamp>/`）
- 按「**驗證**」查看當前規格符合度

### 自我進化（v1.1 規則引擎）
- 工具列「**掃描全部**」→ 進化面板顯示所有問題（依嚴重度高亮）
- 工具列「**執行自我進化**」→ 依設定套用規則：
  1. 無問題 → 跳過
  2. 最高嚴重度 < 門檻 → 僅建議
  3. 已啟用 API + 有 Key → **API 重寫**（OpenRouter LLM 依規格補齊）
  4. 自動套用開啟 → **骨架補齊**（僅追加，不覆寫）
  5. 其他 → 僅建議
- 修改前自動備份至 `.backup/<timestamp>/`
- 寫入 `agents/.evolution.log`（JSON Lines，含 action/model/reason）
- 工具列「**查看進化日誌**」→ 檢視最近 200 筆紀錄

### OpenRouter 設定（v1.1）
- 工具列「**設定（OpenRouter）**」打開對話框
- **OpenRouter 分頁**：
  - API Key、Base URL、Timeout、max_tokens、temperature
  - 主模型 / 備援模型（可從下拉選單或自訂）
  - 自訂模型清單（每行一個 slug，例 `anthropic/claude-opus-4.7`）
  - **測試連線**：發送最小請求驗證 Key 與模型可用
- **進化規則分頁**：
  - 使用 API 重寫（需 Key）
  - 自動套用修復 / 僅建議
  - 修復後仍要通過驗證
  - **Dry run**（API 模式不寫入檔案）
  - 最低嚴重度門檻：CRITICAL / HIGH / MEDIUM / LOW
  - 本輪最多處理 Agent 數（預設 20）

## 規格要求（SKILL.md）

### 必要 frontmatter 欄位
- `name`、`description`、`allowed-tools`
- `metadata.version`、`metadata.category`

### 必要章節
- `## 角色設定`、`## 核心能力`、`## 操作流程`

### 建議章節
- `## 重要聲明`、`## 輸入範例`、`## 輸出範例`、`## 邊緣案例處理`、`## 變更歷史`

## 安全機制
- 所有寫入前自動備份至 `.backup/<YYYYMMDD-HHMMSS>/`
- 刪除操作同樣先備份
- 自我進化僅「追加骨架」，不覆蓋既有內容

## 進階使用

### 作為 Python API
```python
from app.agent_manager import AgentManager

m = AgentManager()
skill = m.create(
    category="20-新興職業",
    name="drone-flight-instructor",
    description="無人機飛行教練 Agent ...",
    role="你是資深無人機教練",
    abilities="- 合法飛行\n- 考照準備\n- 安全守則",
)
```

### 批次掃描
```python
from app.evolution_engine import scan_all, evolve_once

results = scan_all()
print(f"{len(results)} agents have issues")

records = evolve_once(auto_apply=True)
print(f"Processed {len(records)} agents")
```

## 變更歷史
| 版本 | 日期 | 內容 | 影響範圍 |
|------|------|------|----------|
| v1.2.0 | 2026-04-18 | AI 快速生成 Agent + AI 局部編輯（NewAgentDialog AI 按鈕、AIEditDialog、llm_client 新增 generate_agent_draft / edit_text_with_ai、agent_manager 新增 create_from_ai_draft） | app/llm_client.py、app/agent_manager.py、app/main.py |
| v1.1.0 | 2026-04-18 | 新增 21-AI生成 類別、OpenRouter API、規則式進化引擎、GUI 設定對話框 | agents/21-AI生成/、app/config.py、app/llm_client.py、app/evolution_rules.py、app/evolution_engine.py、app/main.py |
| v1.0.0 | 2026-04-18 | 初始版本 | 全專案 |
