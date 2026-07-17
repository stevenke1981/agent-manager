# Test Evidence — Agent Manager 2.0

環境：Windows PowerShell；Rust 工具使用 `C:\Users\steven\.cargo\bin\cargo.exe`。

## 已執行

| 命令 | 結果 |
|---|---|
| package `python validate_agents.py` | PASS；306 agents、37 categories |
| package `CHECKSUMS.sha256` verification | PASS；319 files、0 failures |
| package `python apply_to_repo.py E:\agent-manager --dry-run` | PASS；306 source agents、無寫入 |
| post-apply SHA-256 reduction | PASS；source=306、destination=309、identical=306、destination-only=3、add/update=0 |
| `cargo check` | PASS；完整核心與 egui UI 編譯成功 |
| `cargo test --all-targets` | PASS；23 passed、0 failed（22 unit + 1 CLI integration） |
| release `agent-manager.exe --check` | PASS；exit 0；306 skills、0 load errors；CRITICAL/HIGH/MEDIUM/LOW 全為 0 |
| `cargo fmt --check` | PASS；無格式差異 |
| `cargo check` | PASS；dev profile 完成 |
| `cargo test --all-targets` | PASS；23 passed、0 failed（最終 gate 重跑） |
| `cargo clippy --all-targets -- -D warnings` | PASS；0 warnings/errors |
| `cargo build --release` | PASS；optimized release 完成 |
| `cargo test --test cli_check` | PASS；無效 YAML 會輸出 `LOAD ERROR` / `load_errors=1` 並回傳非零 |
| release GUI process smoke | PASS；隱藏啟動後 event loop 存活 3 秒，再終止本輪測試程序 |
| `git diff --check` | PASS；無 whitespace error |

## Agent content V2 同步證據

- 官方 apply 備份：`E:\agent-manager\.agent-backup\20260717-120128\agents`（本機 ignored，不提交）。
- Git Agent diff：306 個 `SKILL.md`、0 非 Skill、0 刪除；沒有新增或移除 category+slug/path identity。
- Identity invariant 為 category+slug/path，不包含 frontmatter `name`；306 個相對路徑全部保留。
- 相對 HEAD 有 185 筆 `name` 被來源正規化，分布於類別 22–34、36–37，例如 `Anthropologist` → `academic-anthropologist`。
- 來源 README/UPGRADE_REPORT 明定保留 category+slug 並重建 V2 內容；`validate_agents.py` 強制 `name == slug`。因此 185 筆變更屬預期來源資料，維持 validator PASS 與 306/306 SHA-256 equality。
- UI 直接使用 frontmatter `name`，所以部分 title-case 舊顯示名稱會改成 slug；此可見變化已接受為 V2 契約的一部分。
- `AGENTS_INDEX.md` SHA-256：`2B43D6E70A59A8946C1C619A487649593003D0EAA2034BFB4CCFA19379CF7326`，同步前後一致。
- `KNOWLEDGE_GRAPH.md` SHA-256：`3718210B65E70EB2496A0EB42840C36B3C7E7A851555493FB653E76A447C4AD1`，同步前後一致。
- `README.md` SHA-256：`EAE9E09C00FF8119BDCC6AE70B4426454A3FA5B1D55CA5E692AA810D17E0CF27`，同步前後一致。
- 306 個更新檔的 private-key/token signature 與私人絕對路徑掃描：0 matches。
- Source package directory、zip、`.agent-backup/` 均由 root-only `.gitignore` 規則排除。

## 聚焦測試涵蓋

- 複雜/未知 YAML mapping、sequence 與 Markdown body parser roundtrip。
- 儲存的 timestamp backup、原子替換與無暫存殘留。
- validation severity 與 summary。
- Unicode/NFKC slug 與 blank template substitutions。
- evolution 規則優先順序。
- Cursor project path 與 Copilot CLI `.agent.md` 路徑轉換。
- Copilot CLI tools sequence 寫入，以及既有外部目標檔備份。
- AI JSON/Markdown fence cleanup。
- 路徑 traversal / 非標準刪除形狀拒絕，完整 Agent 目錄備份後刪除。
- checked list 的無效 YAML diagnostics，不再靜默丟棄。
- API 修復需通過驗證才寫入；fallback 從磁碟原始版本重建。
- Aider/Windsurf marker upsert 冪等性與高頻備份路徑唯一性。
- 巢狀同名匯入來源產生穩定且不碰撞的 slug。
- Delete target snapshot 與 AI path/revision 綁定。
- mutating background task 會鎖定 editor/save/new/delete/selection；唯讀 task 不鎖定。
- evolution dry run 保持 Agent 原檔不變且不建立 `.evolution.log`。
- Aider/Windsurf 50-skill batch 每 marker 僅一份、最多一次備份，重裝不新增備份。

## 未執行的外部寫入

- 未使用真實 OpenRouter Key；API 連線需由使用者在 Settings 輸入 Key 後測試。
- 未安裝到使用者實際 AI 工具目錄；避免 migration 驗證污染外部設定。格式、路徑與批次門面由 unit tests 與編譯 gate 覆蓋。
