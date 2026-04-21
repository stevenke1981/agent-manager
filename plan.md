# Plan — Agent Manager

## 目標
打造一個 **Python GUI 應用**，管理 `agents/` 下 100+ 職業 Agent（SKILL.md 規格），具備：
1. 依需求快速建立 Agent（模板化）
2. 瀏覽、編輯、刪除
3. 規格驗證
4. 自我進化：找出問題 → 糾正 → 改進

## 里程碑

### v1.0.0（已完成 — 2026-04-18）
- [x] `agent.md` meta-agent 定義
- [x] 核心模組：storage、categories、validator、template_engine、evolution_engine、agent_manager
- [x] tkinter GUI：分類樹、編輯器、進化面板
- [x] 啟動腳本 `start_gui.bat`
- [x] 文件：README、blueprint、knowledge_graph

### v1.1.0（已完成 — 2026-04-18）
- [x] 新類別 `21-AI生成`：6 個 AI 提示詞工程師 / Agent 架構師範本
  - image-prompt-engineer（圖片生成）
  - video-prompt-engineer（影片生成）
  - audio-prompt-engineer（音訊/音樂）
  - 3d-prompt-engineer（3D 模型）
  - llm-prompt-engineer（LLM 提示詞）
  - agent-architect（Agent 系統設計）
- [x] OpenRouter API 整合（config + client + Settings dialog）
- [x] 進化規則引擎（嚴重度門檻、dry run、每輪上限、三模式）
- [x] GUI 設定對話框 + 測試連線

### v1.2.0（已完成 — 2026-04-18）
- [x] **AI 生成 Agent**：`NewAgentDialog` 新增「AI 生成內容」按鈕，依使用者類別 + 名稱 + 一句話需求，呼叫 OpenRouter API 生成 description（含啟動時機）、角色設定、核心能力與完整 body；使用者可在 GUI 預覽/編輯後再建立
- [x] **AI 局部編輯**：工具列新增「AI 局部修改」→ `AIEditDialog`
  - 範圍可選：整份 body / description / `## 角色設定` / `## 核心能力` / `## 操作流程` / `## 輸入範例` / `## 輸出範例` / `## 邊緣案例處理`
  - 生成後先顯示預覽，可再編輯後按「套用」寫入（自動備份）
- [x] `llm_client.generate_agent_draft()`、`edit_text_with_ai()` 兩支高階輔助
- [x] `agent_manager.create_from_ai_draft()` 落地門面
- [x] 所有 API 呼叫走背景執行緒，UI 不凍結

### v1.3.0（規劃）
- [ ] 搜尋列（跨全部 Agent 全文搜尋）
- [ ] Markdown 即時預覽面板
- [ ] 批次匯出 JSON / 壓縮包
- [ ] `agents/AGENTS_INDEX.md` 自動同步
- [ ] 多語系支援（英文 UI）
- [ ] 版本差異視覺化（diff view）
- [ ] 回滾 UI（從 `.backup/` 還原）
- [ ] 進化規則 per-category 客製化
- [ ] API 用量 / 成本追蹤
- [ ] AI 編輯支援自訂章節（非預設 `##` 標題）

## 風險與對策
| 風險 | 對策 |
|------|------|
| YAML 自行解析失準 | 後續可換 PyYAML；目前 subset 足夠 |
| 中文字符在 Windows cmd 亂碼 | GUI 本身 OK；只影響 stdout 檢查輸出 |
| 大量自動修復覆蓋使用者內容 | 所有修改前自動備份至 `.backup/` |

## 變更歷史
| 版本 | 日期 | 內容 | 影響範圍 |
|------|------|------|----------|
| v1.2.0 | 2026-04-18 | AI 生成 Agent + AI 局部編輯 | app/llm_client、app/agent_manager、app/main |
| v1.1.0 | 2026-04-18 | 加入 21-AI生成 類別與 OpenRouter API 規劃 | agents/21-AI生成、app/config、app/llm_client、app/evolution_rules |
| v1.0.0 | 2026-04-18 | 初始規劃 | — |
