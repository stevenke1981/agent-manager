# Plan — Agent Manager 2.0

## Outcome

將 Python/tkinter 1.2 完整切換為可建置、可執行的 Rust eframe/egui Windows 桌面工作台，並整合經完整驗證的 Agent content V2（37 類、306 個既有 category+slug/path identity）。

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

## Agent content V2 整合里程碑（2026-07-17）

- [x] 驗證來源包 306 Agents / 37 categories 與 319 筆 checksum manifest
- [x] 以官方 `apply_to_repo.py` dry-run 後建立完整本機備份，再覆寫 306 個同路徑 `SKILL.md`
- [x] 不新增、不刪除 category+slug/path identity；來源 306 與目的 306 同路徑 SHA-256 全部相等
- [x] 接受來源定義的 185 筆 frontmatter `name` 正規化（類別 22–34、36–37）；例如 `Anthropologist` → `academic-anthropologist`
- [x] 保留且不修改 destination-only `AGENTS_INDEX.md`、`KNOWLEDGE_GRAPH.md`、`README.md`
- [x] Rust release `--check`：306 skills、0 load errors、所有 severity 皆為 0、exit 0

## Preserved boundaries

- Agent identity invariant 是 `(category, slug)`，等價於 `category/slug/SKILL.md` 相對路徑；frontmatter `name` 不屬於 identity，V2 validator 另行要求 `name == slug`。
- `agents/` 的 306 個既有 category+slug/path identity 已更新為官方 Agent content V2；不得以 mirror 流程刪除目的端額外知識文件。
- V2 的 185 筆 `name` 正規化符合來源 README、UPGRADE_REPORT 與 validator 契約；UI 顯示名稱會由部分 title-case 舊名改為 slug，這是預期可見變更，不應回復舊名。
- `.config.json`、`.backup/`、`.codebase-memory/`、`target/`、evolution log 不提交。
- V2 source directory/zip 與 `.agent-backup/` 僅保留於本機並由 root-only ignore 規則排除。
- 不在 migration 驗證中對外部工具目錄做實際寫入；使用 temp/unit path 測試格式與轉換。
- 內容整合工作包不自行 commit 或 push；由 root agent 完成獨立驗證後統一發布到 `master`。

## 後續候選（非 2.0 阻擋）

- YAML lossless concrete-syntax tree，以保留註解與完全相同的引用風格。
- Markdown rendered preview/diff/backup restore UI。
- OpenRouter 成本/用量統計與取消中的請求。
- 以各工具官方版本做 live format matrix 測試。
