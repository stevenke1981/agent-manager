# Blueprint — Agent Manager Rust 2.0

## 架構

```text
eframe/egui UI (src/app.rs)
  │ commands + std::thread/mpsc background jobs
  ▼
AgentManager (src/manager.rs)
  ├─ storage/model/categories/template/validator
  ├─ evolution ── llm/config ── OpenRouter
  ├─ importer
  └─ installer ── tool_registry ── external tool directories
  ▼
agents/ + .backup/ + .config.json + agents/.evolution.log
```

## 核心模組

| 模組 | 責任 |
|---|---|
| `model` | `AgentSkill`、frontmatter 主要欄位與 metadata 存取 |
| `storage` | UTF-8/YAML 解析、語意 roundtrip、列舉、timestamp backup、原子儲存、安全刪除 |
| `categories` | 從實際磁碟列出 `NN-*` 類別；資料不存在時提供既有 20 類 fallback |
| `template` | Unicode NFKC slug、安全目標路徑、模板選取與變數替換 |
| `validator` | 必填欄位/metadata/章節、description 長度、過度授權、severity summary |
| `evolution` | scan、規則決策、append-only skeleton、API rewrite/fallback、JSONL log |
| `config` | `.config.json` serde defaults 與原子寫入；OpenRouter/進化選項 |
| `llm` | blocking reqwest client；由 UI 背景 thread 呼叫；主模型失敗時 fallback |
| `importer` | agency-agents 22–37 類格式轉換 |
| `tool_registry` | 13 種 AI 工具路徑、格式轉換、目標備份與反向讀取 |
| `installer` | 批次 install/backup 統一門面 |
| `theme` | 亮暗色、字級、32px 互動高度、4/8px spacing、zh-TW 字體 fallback |
| `app` | 命令列、分類/全文搜尋、虛擬化 Agent 清單、編輯/驗證/進化/日誌與 modal |

## 寫入安全

1. 新建路徑只由 `agents/<category>/<slug>/SKILL.md` 組成。
2. 儲存與刪除前檢查路徑仍在 `agents/` 之下。
3. 已存在的 SKILL.md 先鏡像備份到 `.backup/<timestamp>/<relative-path>`。
4. 使用 `atomicwrites` 在同目錄完成替換。
5. 外部工具已有檔案先複製到 `<target>/.agent-manager-backup/<timestamp>/`。

## UI 與執行緒模型

- egui font definitions 只在 `AgentManagerApp::new()` 設定一次。
- `SidePanel` 可調寬；`ScrollArea::show_rows` 只建立目前可見的 Agent widgets。
- 所有 Panel/ScrollArea/Window 使用固定 ID；不使用 emoji 當結構圖示。
- idle frame 不呼叫 `request_repaint()`；僅背景工作存在時每 80ms polling。
- OpenRouter、全庫 scan/evolve、import/install/backup 都由 `std::thread` 執行，以 `mpsc` 將 typed result 送回 UI thread。
- Delete 與 Evolution 有二次確認；所有 modal 有明確 loading/error/success 狀態。

## 相容資料格式

解析使用 `serde_yaml::Mapping`，可保留未知巢狀 mapping、sequence 與 scalar 的資料語意。編輯後序列化可能正規化 YAML 的縮排、引號或註解；不會刻意移除未知欄位。此取捨比舊版只支援兩層 scalar 的 parser 更安全。
