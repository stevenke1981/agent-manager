# Agent Manager 2.0

Agent Manager 是以 Rust、eframe/egui 實作的 Windows 桌面工作台，用來管理本 repo `agents/` 中 37 類、306 份 Agent Skills。2.0 不需要 Python，保留原有 CRUD、驗證、自我進化、OpenRouter、匯入與多工具安裝/備份能力。

## 功能

- 解析與編輯 `SKILL.md` YAML frontmatter 和 Markdown body，保留未知巢狀欄位與清單語意。
- 新增、搜尋、開啟、儲存、驗證、刪除；覆寫與刪除前備份到 `.backup/<timestamp>/`。
- 原子寫入，避免中途中斷留下半份檔案；Windows 與 Unicode/zh-TW 路徑可用。
- CRITICAL / HIGH / MEDIUM / LOW 驗證與 306 筆全庫掃描。
- 規則式進化：安全的 append-only 骨架補齊、OpenRouter API 修復、dry run、門檻與每輪上限、JSONL 日誌。
- OpenRouter ping/complete、AI Agent 草稿、AI body/description/章節局部修改；網路與大量工作皆在背景執行。
- 匯入 `agency-agents-main` 的 22–37 類資料。
- 安裝/備份 Claude Code、Copilot VS Code、Copilot CLI/Desktop、Antigravity、Gemini CLI、OpenCode、Cursor、Aider、Windsurf、OpenClaw、Hermes、Qwen CLI、Kimi CLI。
- 亮暗主題、可調整側欄、全文搜尋、可見計數、虛擬化清單、未儲存提示與描述性錯誤。

## 需求與啟動

- Windows 10/11
- Rust stable（本專案使用 Rust 2024 edition）

雙擊 `start_gui.bat`，或在 PowerShell 執行：

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo run --release
```

第一次會下載 crate 並編譯。後續可直接執行：

```powershell
.\target\release\agent-manager.exe
```

## Headless 檢查

不開 GUI、實際讀取全部 agents：

```powershell
cargo run -- --check
```

輸出會包含讀取成功的 SKILL.md 數量與有驗證問題的 Agent 數量。既有資料不會因檢查而被修改。

## 資料安全

- `.config.json` 可能包含 OpenRouter API Key，已由 `.gitignore` 排除。
- Agent 儲存與刪除備份位於 `.backup/`；對外工具既有目標檔備份位於目標根目錄的 `.agent-manager-backup/`。
- `agents/.evolution.log` 是本機 JSONL 執行紀錄，不提交 git。
- API、掃描、進化、匯入及安裝均顯示忙碌/完成/錯誤狀態；只有工作進行中才排程 repaint。

## 專案結構

```text
Cargo.toml / Cargo.lock
src/
  main.rs           GUI 與 --check 入口
  app.rs            eframe/egui 工作台
  model.rs          AgentSkill 資料模型
  storage.rs        YAML 解析、備份與原子寫入
  validator.rs      規格驗證與嚴重度
  template.rs       slug 與模板建立
  evolution.rs      掃描、規則、修復與 JSONL 日誌
  config.rs         .config.json
  llm.rs            OpenRouter 與 AI 輔助
  importer.rs       agency-agents 匯入
  tool_registry.rs  13 種工具格式/路徑轉換
  installer.rs      安裝與反向備份門面
  theme.rs          集中式 egui 設計 token
agents/              既有 Agent Skills 資料（不由 2.0 migration 修改）
```

開發與驗證命令請見 [test.md](test.md)，架構細節見 [blueprint.md](blueprint.md)，遷移結果與限制見 [final.md](final.md)。
