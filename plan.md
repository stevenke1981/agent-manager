# Plan — Agent Manager 2.0

## Outcome

將 Python/tkinter 1.2 完整切換為可建置、可執行的 Rust eframe/egui Windows 桌面工作台，同時保留 `agents/` 37 類、306 份資料及核心行為。

## Rust 2.0 里程碑（2026-07-17）

- [x] Cargo crate、lockfile、無 `unsafe` 的模組化 source
- [x] YAML frontmatter/body parser、未知 YAML 語意 roundtrip、Unicode path
- [x] CRUD、備份、原子寫入、全文搜尋
- [x] category、Unicode slug、template generation
- [x] validation severity/summary、全庫 scan
- [x] evolution decision、append-only skeleton、OpenRouter rewrite/fallback、JSONL log
- [x] `.config.json`、OpenRouter ping/complete、AI draft/scoped edit
- [x] agency-agents importer
- [x] 13 種工具 install/backup registry，覆寫前建立可回復備份
- [x] 現代 eframe/egui 工作台、亮暗主題、虛擬化 306 筆清單、未儲存/錯誤/載入狀態
- [x] Windows Rust 啟動器與 headless `--check`
- [x] parser、validator、backup/atomic save、slug/template、evolution、path conversion 測試
- [x] 最終 fmt/check/test/clippy/release/headless/GUI smoke gate（結果見 `test.md`/`final.md`）

## Preserved boundaries

- `agents/`、`agent.md` 及既有知識文件內容不在重寫範圍內。
- `.config.json`、`.backup/`、`.codebase-memory/`、`target/`、evolution log 不提交。
- 不在 migration 驗證中對外部工具目錄做實際寫入；使用 temp/unit path 測試格式與轉換。
- 不進行 commit 或 push。

## 後續候選（非 2.0 阻擋）

- YAML lossless concrete-syntax tree，以保留註解與完全相同的引用風格。
- Markdown rendered preview/diff/backup restore UI。
- OpenRouter 成本/用量統計與取消中的請求。
- 以各工具官方版本做 live format matrix 測試。
