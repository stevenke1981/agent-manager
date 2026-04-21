# Blueprint — Agent Manager 系統架構

## 分層

```
┌──────────────────────────────────────────────┐
│          GUI Layer（tkinter）                │
│   app/main.py                                │
│   ├─ AgentManagerApp（主視窗）              │
│   ├─ NewAgentDialog（建立對話框）           │
│   └─ LogDialog（進化日誌）                  │
└──────────────────┬───────────────────────────┘
                   │
┌──────────────────▼───────────────────────────┐
│          Orchestration Layer                 │
│   app/agent_manager.py — AgentManager CRUD   │
└──────────────────┬───────────────────────────┘
                   │
    ┌──────────────┼──────────────┬─────────────────┐
    ▼              ▼              ▼                 ▼
 template      validator      evolution         storage
_engine.py    .py            _engine.py         .py
    │              │              │                 │
    └──────────────┴──────────────┴──────┬──────────┘
                                         ▼
                              ┌──────────────────────┐
                              │   agents/ 檔案系統   │
                              │   + .backup/         │
                              │   + .evolution.log   │
                              └──────────────────────┘
```

## 模組職責

| 模組 | 職責 | 關鍵介面 |
|------|------|---------|
| `categories` | 21 大類別註冊（v1.1 新增 AI 生成類） | `list_categories()`、`ensure_category()` |
| `storage` | SKILL.md 讀寫（含自製 YAML 子集） | `load_skill()`、`save_skill()`、`list_skills()`、`delete_skill()` |
| `validator` | 規格檢查、問題分級 | `validate()`、`summary()` |
| `template_engine` | 模板挑選 + 變數替換 | `find_best_template()`、`render_from_template()` |
| `evolution_engine` | 掃描 → 規則決策 → 修復 → 記錄 | `scan_all()`、`evolve_once(cfg)`、`auto_fix_skeleton()`、`auto_fix_api()`、`read_log()` |
| `agent_manager` | GUI 呼叫的統一門面 | `AgentManager.create/list/open/save/delete/validate` |
| `config` <!-- v1.1 新增 --> | 持久化設定（OpenRouter、進化參數） | `AppConfig`、`load_config()`、`save_config()`、`all_models()` |
| `llm_client` <!-- v1.1 新增；v1.2 擴充 --> | OpenRouter HTTP client（純 stdlib）＋ AI 生成/編輯輔助 | `LLMClient.complete()`、`LLMClient.ping()`、`generate_agent_draft()`、`edit_text_with_ai()` |
| `evolution_rules` <!-- v1.1 新增 --> | 規則決策：skeleton / api / suggest / skip | `decide(issues, cfg)` |
| `main` | tkinter 介面 | `main()`、`SettingsDialog`（v1.1） |

## 資料流

### 建立 Agent
```
使用者 → NewAgentDialog → AgentManager.create()
     → template_engine.find_best_template(category, keyword)
     → template_engine.render_from_template(...)
     → storage.save_skill(skill, backup=False)
     → GUI 更新樹 & 選取新項目
```

### AI 生成 Agent（v1.2 新增）
```
NewAgentDialog「AI 生成內容」按鈕
  → 收集 category / name / allowed-tools / brief
  → 背景執行緒：llm_client.generate_agent_draft(client, ...)
      → LLMClient.complete(system=GENERATE_SYSTEM_PROMPT, user=…) → JSON
  → 回填 description/role/abilities；暫存完整 body
  → 使用者修改後按「建立」:
      → AgentManager.create_from_ai_draft(category, name, draft)
          → render_from_template(template_path=None, ...) 組 frontmatter
          → 覆蓋 body 為 AI 完整版
          → storage.save_skill(skill, backup=False)
```

### AI 局部編輯（v1.2 新增）
```
編輯器「AI 局部修改」按鈕 → AIEditDialog
  → 使用者選範圍（body / description / ## 某章節）+ 輸入指令
  → 擷取當前範圍文字：
      body       → skill.body
      description → frontmatter["description"]
      某 ## 章節  → 從 `## 標題` 到下一個 `## ` 前的片段
  → 背景執行緒：edit_text_with_ai(client, current, instruction, scope)
      → LLMClient.complete(system=EDIT_SYSTEM_PROMPT, user=…)
  → 預覽區顯示結果（使用者可再編輯）
  → 「套用」按鈕 → 依 scope 寫回 skill.body / description / 章節片段
  → AgentManager.save(skill) → 自動備份
```

### 自我進化（v1.1 — 規則引擎）
```
工具列「執行自我進化」
  → load_config() → AppConfig
  → evolution_engine.evolve_once(cfg)
  → for each SKILL.md（到 max_agents_per_run 為止）:
      - validate() → issues
      - decide(issues, cfg) → RuleDecision(skeleton | api | suggest | skip)
      - if mode == "skeleton": auto_fix_skeleton()（僅追加骨架）
      - if mode == "api":      auto_fix_api()（LLM 重寫 → 驗證 → 備份寫入）
      - if mode == "suggest":  僅記錄建議
      - 失敗自動 fallback 至 skeleton
      - 記錄 action/model/mode_reason 至 .evolution.log
  → GUI 更新進化面板 + 狀態列（各 action 計數）
```

### OpenRouter API 呼叫（v1.1）
```
設定對話框（測試連線 / 儲存）
  → load_config() → AppConfig
  → LLMClient(cfg).ping() / complete()
  → POST {base_url}/chat/completions
      headers: Authorization Bearer, HTTP-Referer, X-Title
      body:    {model, messages, max_tokens, temperature}
  → 主模型失敗 → 自動 fallback 至 fallback_model
  → 回傳 LLMResponse(content, model, raw)
```

### 規則優先順序（evolution_rules.decide）
1. 無問題 → `skip`
2. 最高嚴重度 < 門檻 → `suggest`
3. API 模式開啟 + Key 存在 → `api`
4. 自動套用開啟 → `skeleton`
5. 其他 → `suggest`

## 檔案慣例
- Agent 放置於：`agents/<NN-類別名>/<slug>/SKILL.md`
- 備份：`.backup/<YYYYMMDD-HHMMSS>/<slug>.SKILL.md`
- 日誌：`agents/.evolution.log`（JSON Lines）
- 設定：`.config.json`（v1.1 新增；含 OpenRouter API Key，請加入 `.gitignore`）

## 設計取捨
- **不用 PyYAML**：zero external deps；YAML 子集足夠 SKILL.md frontmatter。
- **不做 Markdown rendering**：編輯器用純 Text widget；預覽留待 v1.1。
- **自我進化只「補骨架」**：絕不覆蓋使用者實質內容（SKELETON_SECTIONS 僅在章節不存在時追加）。

## 變更歷史
| 版本 | 日期 | 內容 | 影響範圍 |
|------|------|------|----------|
| v1.2.0 | 2026-04-18 | 擴充 llm_client（AI 生成/AI 編輯輔助）、agent_manager 新增 create_from_ai_draft、main 新增 AIEditDialog 與 NewAgentDialog 的 AI 按鈕 | app/llm_client.py、app/agent_manager.py、app/main.py |
| v1.1.0 | 2026-04-18 | 新增 config / llm_client / evolution_rules 三模組；evolution_engine 升級為規則引擎 | app/ 核心模組 |
| v1.0.0 | 2026-04-18 | 初始架構 | 全專案 |
