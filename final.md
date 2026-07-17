# Final — Agent Manager Rust 2.0

## 完成內容

Python/tkinter runtime 已由 Rust 2024 + eframe/egui 應用取代。`agents/` 的 306 個既有 category+slug/path identity 已安全更新為 Agent content V2；實際 headless check 成功讀取 37 類、306 份 SKILL.md，且所有 severity 為 0。Rust source 分為資料、儲存、驗證、模板、進化、設定/LLM、匯入、工具 registry/installer、主題與 UI 模組。

UI 提供頂部命令列、類別/全文搜尋側欄、可見計數與虛擬化列表、frontmatter/Markdown editor、Save/Validate/Create/Delete、Scan/Evolution/Log、Settings、Import、Install/Backup、AI Generate/Edit、亮暗主題、dirty/loading/error/empty 狀態。

## 相容性微調

- 舊 parser 只理解 top-level + 一層 metadata scalar；2.0 改用完整 YAML value tree，未知 mapping/list/scalar 不會被丟棄。
- YAML 儲存保持資料語意，但可能正規化註解、縮排或引用字元；這是目前唯一已知的文字層 roundtrip 差異。
- Aider/Windsurf 使用穩定 marker block 做冪等 batch upsert；整批只讀取與原子寫入 consolidated file 一次，重複安裝不累加內容或備份。
- Evolution dry run 現在也適用 skeleton 與進化 log；不修改 Agent，也不建立或追加 `.evolution.log`。
- CLI `--check` 新增為 headless-safe 真實資料 gate；load error 或任何 CRITICAL/HIGH 會回傳 exit 1，且不修改任何 Agent。
- 儲存層集中驗證 category/slug 與精確 `agents/<category>/<slug>/SKILL.md` 形狀；刪除前完整備份 Agent 目錄。
- GUI 在 dirty 狀態阻止切換、新增、匯入、進化與刪除；刪除固定確認時的 path，AI 預覽固定 path/revision，過期結果不可套用。
- Mutating background task 執行期間鎖定 editor、Save/Delete/New/selection；完成時重新載入磁碟版本，path/revision 衝突會禁止儲存。
- 匯入預覽改為 source 變更時背景掃描，避免 UI frame 執行 WalkDir。

## Agent identity 與名稱正規化

Agent identity invariant 是 `(category, slug)`，等價於相對路徑 `category/slug/SKILL.md`；frontmatter `name` 不構成 identity。V2 來源 README 與 UPGRADE_REPORT 明定保留 category+slug 並重建一致內容，而 `validate_agents.py` 強制 `name == slug`。

因此相對 HEAD 的 185 筆 `name` 正規化是預期來源內容，涵蓋類別 22–34、36–37，例如 `Anthropologist` → `academic-anthropologist`。這些值不得改回，否則會破壞來源 validator 與 306/306 SHA-256 equality。UI 使用 frontmatter `name` 作為顯示名稱，因此部分 title-case 舊名會可見地改為 slug。

## 驗證

來源包 validator 通過 306 Agents / 37 categories，319 筆 checksum 全部吻合。官方 apply 先建立 `E:\agent-manager\.agent-backup\20260717-120128\agents`，再覆寫 306 個同路徑 Skill；同步後來源與目的的 306 筆 SHA-256 全部相等，三份 destination-only 文件仍存在且 hash 不變。

Rust gate 全部通過：`cargo fmt --check`、`cargo check`、`cargo test --all-targets`（23 passed）、strict Clippy（`-D warnings`）、`cargo build --release` 與 `git diff --check`。release `--check` 讀取 306 份 SKILL.md、0 load errors、所有 severity 為 0 且 exit 0。精確證據見 [test.md](test.md)。

## 已知限制

- OpenRouter 需要使用者在 Settings 輸入自己的 Key；migration 過程未使用真實憑證做 live API 呼叫。
- 外部 AI 工具安裝在本輪只做 unit/path/format 驗證，未寫入使用者實際工具目錄。
- GUI 自動化未涵蓋人工視覺驗收；會以 Windows process 短暫啟動 smoke 確認 event loop 可存活。
- V2 source pack、zip 與本機 rollback backup 不提交；它們由 root-only `.gitignore` 規則排除。

## Self-improvement

此 migration 確認的 durable workflow 已由 repo 的 `plan.md`、`blueprint.md`、`test.md` 與本檔承接；未擴張全域 AGENTS.md，也未建立新的窄用途 Skill。
