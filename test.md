# Test Evidence — Agent Manager 2.0

環境：Windows PowerShell；Rust 工具使用 `C:\Users\steven\.cargo\bin\cargo.exe`。

## 已執行

| 命令 | 結果 |
|---|---|
| `cargo check` | PASS；完整核心與 egui UI 編譯成功 |
| `cargo test --all-targets` | PASS；23 passed、0 failed（22 unit + 1 CLI integration） |
| `cargo run -- --check` | 預期 exit 1；讀取 306 skills、0 load errors；CRITICAL=14、HIGH=838、MEDIUM=0、LOW=721 |
| `cargo fmt --check` | PASS；無格式差異 |
| `cargo check` | PASS；dev profile 完成 |
| `cargo test --all-targets` | PASS；23 passed、0 failed（最終 gate 重跑） |
| `cargo clippy --all-targets -- -D warnings` | PASS；0 warnings/errors |
| `cargo build --release` | PASS；optimized release 完成 |
| `cargo test --test cli_check` | PASS；無效 YAML 會輸出 `LOAD ERROR` / `load_errors=1` 並回傳非零 |
| release GUI process smoke | PASS；隱藏啟動後 event loop 存活 3 秒，再終止本輪測試程序 |
| `git diff --check` | PASS；無 whitespace error |

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
