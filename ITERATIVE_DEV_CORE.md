# ITERATIVE_DEV_CORE — Agent Manager

> 專案迭代狀態總覽（v2.0 規則自動維護）

## 專案資訊
- 名稱：Agent Manager
- 描述：管理 `agents/` 目錄下 100+ 職業 Agent 的 Python GUI，具備模板快速建立與自我進化能力
- 目前版本：`v1.2.0`
- 啟用迭代模式：是

## 核心檔案清單
| 檔案 | 用途 |
|------|------|
| agent.md | Meta-Agent 定義（本專案的智慧 Agent） |
| plan.md | 功能規劃與里程碑 |
| blueprint.md | 系統架構藍圖 |
| knowledge_graph.md | 模組關聯圖 |
| README.md | 使用說明 |
| app/ | Python GUI 程式碼 |
| agents/ | 100+ 職業 Agent 資料庫 |

## 功能清單
### v1.0.0
- [x] Agent 瀏覽（依 20 大類 × 100+ 細項）
- [x] Agent 新建（基於模板）
- [x] Agent 編輯（YAML frontmatter + Markdown 內容）
- [x] Agent 刪除
- [x] Agent 驗證（SKILL.md 規格檢查）
- [x] 自我進化引擎（掃描 → 偵錯 → 修正 → 改進）
- [x] 模板管理（沿用 agents/ 既有 Agent 作為模板）

### v1.1.0 新增
- [x] 新類別 `21-AI生成`：6 個 AI 提示詞 / Agent 架構師範本
- [x] OpenRouter 設定（API Key、主/備援模型、自訂模型、timeout/temperature/max_tokens）
- [x] LLM Client（純 stdlib，支援模型 fallback）
- [x] 進化規則引擎（嚴重度門檻、每輪上限、dry run、骨架/API/建議三模式）
- [x] API 模式進化：LLM 依規格補齊缺失內容（不覆蓋使用者內容）
- [x] 設定對話框（GUI）＋連線測試
- [x] 進化日誌新增 action/model/reason 欄位

### v1.2.0 新增
- [x] AI 快速生成 Agent：`NewAgentDialog` 新增「AI 生成內容」按鈕，依簡短 brief 一鍵補齊 description/role/abilities/body
- [x] AI 局部修改：`AIEditDialog` 支援範圍選擇（整份 body / description / 單一章節），回傳內容可預覽後再套用
- [x] `llm_client.generate_agent_draft()` / `edit_text_with_ai()` 高階輔助
- [x] `agent_manager.create_from_ai_draft()` 落地門面（直接寫入 LLM 產生的完整 body）
- [x] 所有 AI 呼叫皆於背景執行緒執行，不凍結 UI

## 變更歷史
| 版本 | 日期 | 內容 | 影響範圍 |
|------|------|------|----------|
| v1.2.0 | 2026-04-18 | AI 生成 Agent + AI 局部編輯 | app/llm_client.py、app/agent_manager.py、app/main.py |
| v1.1.0 | 2026-04-18 | 新增 21-AI生成 類別、OpenRouter API、規則進化引擎 | agents/21-AI生成/、app/config.py、app/llm_client.py、app/evolution_rules.py、app/evolution_engine.py、app/main.py |
| v1.0.0 | 2026-04-18 | 初始建立 Agent Manager GUI + Meta-Agent | 全專案 |
