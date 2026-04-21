"""Agent Manager GUI — tkinter.

Run:
    python -m app.main
or via start_gui.bat
"""
from __future__ import annotations

import sys
import threading
import tkinter as tk
from pathlib import Path
from tkinter import messagebox, ttk

if __package__ is None or __package__ == "":
    # Allow `python app/main.py` direct run.
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    from app.agent_manager import AgentManager  # type: ignore
    from app.categories import category_label
    from app.config import DEFAULT_MODELS, all_models, load_config, save_config
    from app.evolution_engine import evolve_once, read_log, scan_all
    from app.importer import DEFAULT_SOURCE, count_importable, import_all
    from app.installer import backup_from_tool, install_agents
    from app.llm_client import LLMClient, LLMError, edit_text_with_ai, generate_agent_draft
    from app.storage import AgentSkill
    from app.tool_registry import TOOLS, ToolDef
else:
    from .agent_manager import AgentManager
    from .categories import category_label
    from .config import DEFAULT_MODELS, all_models, load_config, save_config
    from .evolution_engine import evolve_once, read_log, scan_all
    from .importer import DEFAULT_SOURCE, count_importable, import_all
    from .installer import backup_from_tool, install_agents
    from .llm_client import LLMClient, LLMError, edit_text_with_ai, generate_agent_draft
    from .storage import AgentSkill
    from .tool_registry import TOOLS, ToolDef


APP_TITLE = "Agent Manager  ·  300+ AI Agent 管理平台"


class AgentManagerApp(tk.Tk):
    def __init__(self) -> None:
        super().__init__()
        self.title(APP_TITLE)
        self.geometry("1280x820")
        self.minsize(1024, 640)

        self.manager = AgentManager()
        self.current_skill: AgentSkill | None = None
        self._search_after_id: str | None = None

        self._build_style()
        self._build_toolbar()
        self._build_searchbar()
        self._build_main()
        self._build_statusbar()

        self.bind_all("<Control-s>", lambda e: self.on_save() or "break")
        self.bind_all("<Control-n>", lambda e: self.on_new_agent() or "break")
        self.bind_all("<Control-f>", lambda e: self._focus_search() or "break")

        self.after(100, self.refresh_tree)

    # ---------- style ----------
    def _build_style(self) -> None:
        style = ttk.Style(self)
        try:
            style.theme_use("clam")
        except tk.TclError:
            pass
        style.configure("Toolbar.TButton", padding=(8, 5))
        style.configure("Treeview", rowheight=26, background="#ffffff", fieldbackground="#ffffff")
        style.configure("Treeview.Heading", font=("", 9, "bold"))

    # ---------- toolbar ----------
    def _build_toolbar(self) -> None:
        bar = ttk.Frame(self, padding=(8, 6))
        bar.pack(side=tk.TOP, fill=tk.X)

        def add(text: str, cmd) -> None:
            ttk.Button(bar, text=text, style="Toolbar.TButton", command=cmd).pack(side=tk.LEFT, padx=4)

        add("➕ 新增", self.on_new_agent)
        add("📋 副本", self.on_duplicate)
        add("🗑 刪除", self.on_delete)
        ttk.Separator(bar, orient="vertical").pack(side=tk.LEFT, fill=tk.Y, padx=8)
        add("💾 儲存", self.on_save)
        add("✔ 驗證", self.on_validate)
        add("🤖 AI 修改", self.on_ai_edit)
        ttk.Separator(bar, orient="vertical").pack(side=tk.LEFT, fill=tk.Y, padx=8)
        add("🔍 掃描", self.on_scan_all)
        add("⚡ 進化", self.on_evolve)
        add("📜 日誌", self.on_view_log)
        ttk.Separator(bar, orient="vertical").pack(side=tk.LEFT, fill=tk.Y, padx=8)
        add("📥 匯入", self.on_import_agents)
        add("🔧 安裝", self.on_installer)
        ttk.Separator(bar, orient="vertical").pack(side=tk.LEFT, fill=tk.Y, padx=8)
        add("⚙ 設定", self.on_open_settings)
        add("🔄 重新整理", self.refresh_tree)

    # ---------- main split ----------
    def _build_main(self) -> None:
        paned = ttk.PanedWindow(self, orient=tk.HORIZONTAL)
        paned.pack(fill=tk.BOTH, expand=True, padx=8, pady=(0, 4))

        # Left — category tree
        left = ttk.Frame(paned)
        ttk.Label(left, text="Agents（依類別）", padding=(4, 4)).pack(anchor=tk.W)
        self.tree = ttk.Treeview(left, show="tree", selectmode="browse")
        tree_sb = ttk.Scrollbar(left, orient=tk.VERTICAL, command=self.tree.yview)
        self.tree.configure(yscrollcommand=tree_sb.set)
        self.tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        tree_sb.pack(side=tk.RIGHT, fill=tk.Y)
        self.tree.bind("<<TreeviewSelect>>", self.on_tree_select)
        self.tree.tag_configure("category", font=("", 9, "bold"))
        self.tree.tag_configure("agent_even", background="#f5f7ff")
        self.tree.tag_configure("agent_odd", background="#ffffff")
        paned.add(left, weight=1)

        # Right — notebook (editor + evolution)
        right = ttk.Notebook(paned)
        self._build_editor_tab(right)
        self._build_evolution_tab(right)
        paned.add(right, weight=3)

    def _build_editor_tab(self, nb: ttk.Notebook) -> None:
        frame = ttk.Frame(nb, padding=8)
        nb.add(frame, text="編輯器")

        form = ttk.LabelFrame(frame, text="Frontmatter", padding=8)
        form.pack(fill=tk.X)

        self.var_name = tk.StringVar()
        self.var_category = tk.StringVar()
        self.var_version = tk.StringVar()
        self.var_tools = tk.StringVar()
        self.var_description = tk.StringVar()

        self._form_row(form, 0, "name", self.var_name)
        self._form_row(form, 1, "category", self.var_category, readonly=False)
        self._form_row(form, 2, "metadata.version", self.var_version)
        self._form_row(form, 3, "allowed-tools", self.var_tools)

        ttk.Label(form, text="description").grid(row=4, column=0, sticky=tk.NW, padx=4, pady=4)
        self.txt_description = tk.Text(form, height=4, wrap=tk.WORD)
        self.txt_description.grid(row=4, column=1, sticky=tk.EW, padx=4, pady=4)
        form.columnconfigure(1, weight=1)

        body_frame = ttk.LabelFrame(frame, text="Markdown 內容", padding=8)
        body_frame.pack(fill=tk.BOTH, expand=True, pady=(8, 0))

        self.txt_body = tk.Text(body_frame, wrap=tk.NONE, undo=True, font=("Consolas", 10))
        yscroll = ttk.Scrollbar(body_frame, orient=tk.VERTICAL, command=self.txt_body.yview)
        xscroll = ttk.Scrollbar(body_frame, orient=tk.HORIZONTAL, command=self.txt_body.xview)
        self.txt_body.configure(yscrollcommand=yscroll.set, xscrollcommand=xscroll.set)
        yscroll.pack(side=tk.RIGHT, fill=tk.Y)
        xscroll.pack(side=tk.BOTTOM, fill=tk.X)
        self.txt_body.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        # Validation summary below editor
        self.validation_label = ttk.Label(frame, text="", foreground="#555")
        self.validation_label.pack(anchor=tk.W, pady=(6, 0))

    def _form_row(self, parent: ttk.Frame, row: int, label: str, var: tk.StringVar, *, readonly: bool = False) -> None:
        ttk.Label(parent, text=label).grid(row=row, column=0, sticky=tk.W, padx=4, pady=4)
        entry = ttk.Entry(parent, textvariable=var)
        if readonly:
            entry.state(["readonly"])
        entry.grid(row=row, column=1, sticky=tk.EW, padx=4, pady=4)

    def _build_evolution_tab(self, nb: ttk.Notebook) -> None:
        frame = ttk.Frame(nb, padding=8)
        nb.add(frame, text="自我進化")

        controls = ttk.Frame(frame)
        controls.pack(fill=tk.X)
        ttk.Button(controls, text="掃描全部", command=self.on_scan_all).pack(side=tk.LEFT, padx=4)
        ttk.Button(controls, text="一鍵進化（自動修復）", command=self.on_evolve).pack(side=tk.LEFT, padx=4)
        ttk.Button(controls, text="查看日誌", command=self.on_view_log).pack(side=tk.LEFT, padx=4)

        cols = ("severity", "field", "message", "path")
        evo_outer = ttk.Frame(frame)
        evo_outer.pack(fill=tk.BOTH, expand=True, pady=(8, 0))
        self.evo_tree = ttk.Treeview(evo_outer, columns=cols, show="headings", height=20)
        for col, title, width in [
            ("severity", "嚴重度", 90),
            ("field", "欄位", 130),
            ("message", "問題", 460),
            ("path", "檔案", 400),
        ]:
            self.evo_tree.heading(col, text=title)
            self.evo_tree.column(col, width=width, anchor=tk.W)
        evo_scroll = ttk.Scrollbar(evo_outer, orient=tk.VERTICAL, command=self.evo_tree.yview)
        self.evo_tree.configure(yscrollcommand=evo_scroll.set)
        self.evo_tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        evo_scroll.pack(side=tk.RIGHT, fill=tk.Y)

        self.evo_tree.tag_configure("CRITICAL", background="#fde2e2")
        self.evo_tree.tag_configure("HIGH", background="#fff3cd")
        self.evo_tree.tag_configure("MEDIUM", background="#e7f1ff")
        self.evo_tree.tag_configure("LOW", background="#f3f3f3")

    # ---------- statusbar ----------
    def _build_statusbar(self) -> None:
        self.status = tk.StringVar(value="就緒")
        ttk.Separator(self).pack(fill=tk.X)
        ttk.Label(self, textvariable=self.status, anchor=tk.W, padding=(8, 4)).pack(fill=tk.X)

    # ---------- search ----------
    def _build_searchbar(self) -> None:
        bar = ttk.Frame(self, padding=(8, 2))
        bar.pack(side=tk.TOP, fill=tk.X)
        ttk.Label(bar, text="🔍").pack(side=tk.LEFT, padx=(0, 4))
        self._search_entry = ttk.Entry(bar, width=30)
        self._search_entry.pack(side=tk.LEFT, fill=tk.X, expand=True)
        self._search_entry.bind("<KeyRelease>", self._on_search_changed)
        ttk.Button(bar, text="✕", width=3, command=self._clear_search).pack(side=tk.LEFT, padx=(4, 0))
        ttk.Separator(bar, orient="vertical").pack(side=tk.LEFT, fill=tk.Y, padx=6)
        self._search_count_label = ttk.Label(bar, text="", foreground="#888")
        self._search_count_label.pack(side=tk.LEFT)

    def _on_search_changed(self, _event=None) -> None:
        if self._search_after_id:
            self.after_cancel(self._search_after_id)
        self._search_after_id = self.after(300, self._apply_search_filter)

    def _apply_search_filter(self) -> None:
        self._search_after_id = None
        q = self._search_entry.get().strip()
        self.refresh_tree(search=q)

    def _clear_search(self) -> None:
        self._search_entry.delete(0, tk.END)
        self._apply_search_filter()

    def _focus_search(self) -> None:
        self._search_entry.focus_set()
        self._search_entry.select_range(0, tk.END)

    # ---------- tree ----------
    def refresh_tree(self, search: str = "") -> None:
        self.tree.delete(*self.tree.get_children())
        q = search.strip().lower()
        total = 0
        matched = 0
        for category in self.manager.categories():
            label_base = f"{category.split('-', 1)[0]}　{category_label(category)}"
            agents_in_cat = []
            for skill in self.manager.list(category):
                total += 1
                if q:
                    fm = skill.frontmatter or {}
                    desc = str(fm.get("description", ""))
                    if (q not in skill.slug.lower()
                            and q not in (skill.name or "").lower()
                            and q not in desc.lower()):
                        continue
                agents_in_cat.append(skill)
                matched += 1
            if q and not agents_in_cat:
                continue
            cat_label = f"{label_base} ({len(agents_in_cat)})"
            cat_id = self.tree.insert(
                "", tk.END, text=cat_label, open=bool(q),
                values=(category,), tags=("category",),
            )
            for i, skill in enumerate(agents_in_cat):
                display = f"{skill.slug}  ·  {skill.name}"
                row_tag = "agent_even" if i % 2 == 0 else "agent_odd"
                self.tree.insert(
                    cat_id, tk.END, text=display,
                    values=(str(skill.path),), tags=("agent", row_tag),
                )
        if q:
            if hasattr(self, "_search_count_label"):
                self._search_count_label.configure(text=f"符合 {matched} 個")
            self.status.set(f"搜尋「{q}」：找到 {matched} / {total} 個 Agent")
        else:
            if hasattr(self, "_search_count_label"):
                self._search_count_label.configure(text="")
            self.status.set(f"已載入 {total} 個 Agent")

    def _iter_agents(self):
        for cat in self.tree.get_children():
            for agent in self.tree.get_children(cat):
                yield agent

    def on_tree_select(self, _event=None) -> None:
        selected = self.tree.selection()
        if not selected:
            return
        item = selected[0]
        tags = self.tree.item(item, "tags")
        if "agent" not in tags:
            return
        path_str = self.tree.item(item, "values")[0]
        try:
            skill = self.manager.open(Path(path_str))
            self._load_into_editor(skill)
        except Exception as e:
            messagebox.showerror("錯誤", f"無法開啟 Agent：{e}")

    def _load_into_editor(self, skill: AgentSkill) -> None:
        self.current_skill = skill
        fm = skill.frontmatter or {}
        metadata = fm.get("metadata") or {}
        self.var_name.set(str(fm.get("name", "")))
        self.var_category.set(str(metadata.get("category", skill.path.parent.parent.name)))
        self.var_version.set(str(metadata.get("version", "1.0.0")))
        self.var_tools.set(str(fm.get("allowed-tools", "")))
        self.txt_description.delete("1.0", tk.END)
        self.txt_description.insert("1.0", str(fm.get("description", "")))
        self.txt_body.delete("1.0", tk.END)
        self.txt_body.insert("1.0", skill.body or "")
        self._update_validation_label(skill)
        self.status.set(f"已開啟 {skill.path}")

    def _update_validation_label(self, skill: AgentSkill) -> None:
        issues, counts = self.manager.validate(skill)
        if not issues:
            self.validation_label.configure(text="✓ 驗證通過", foreground="#1a7f37")
        else:
            text = " / ".join(f"{k}:{v}" for k, v in counts.items() if v)
            self.validation_label.configure(text=f"⚠ 驗證問題 — {text}", foreground="#b54708")

    def _apply_editor_to_skill(self) -> AgentSkill | None:
        if not self.current_skill:
            return None
        skill = self.current_skill
        fm = skill.frontmatter or {}
        metadata = fm.get("metadata") or {}
        if not isinstance(metadata, dict):
            metadata = {}
        fm["name"] = self.var_name.get().strip()
        fm["description"] = self.txt_description.get("1.0", tk.END).strip()
        fm["allowed-tools"] = self.var_tools.get().strip()
        metadata["category"] = self.var_category.get().strip()
        metadata["version"] = self.var_version.get().strip() or "1.0.0"
        fm["metadata"] = metadata
        skill.frontmatter = fm
        skill.body = self.txt_body.get("1.0", tk.END).rstrip() + "\n"
        return skill

    # ---------- actions ----------
    def on_new_agent(self) -> None:
        NewAgentDialog(self, self._after_create)

    def _after_create(self, skill: AgentSkill | None) -> None:
        if not skill:
            return
        self.refresh_tree()
        self.status.set(f"已建立 {skill.path}")
        # select newly created
        for agent_item in self._iter_agents():
            if str(skill.path) == self.tree.item(agent_item, "values")[0]:
                self.tree.see(agent_item)
                self.tree.selection_set(agent_item)
                break

    def on_duplicate(self) -> None:
        if not self.current_skill:
            messagebox.showinfo("提示", "請先選取一個 Agent")
            return
        src = self.current_skill
        NewAgentDialog(
            self,
            self._after_create,
            preset_category=src.category or src.path.parent.parent.name,
            preset_name=f"{src.name}-copy",
            preset_description=src.description,
            preset_template=src.slug,
        )

    def on_delete(self) -> None:
        if not self.current_skill:
            return
        if not messagebox.askyesno("確認刪除", f"要刪除 {self.current_skill.path.parent.name} 嗎？（自動備份至 .backup/）"):
            return
        try:
            self.manager.delete(self.current_skill)
            self.current_skill = None
            self.refresh_tree()
        except Exception as e:
            messagebox.showerror("刪除失敗", str(e))

    def on_save(self) -> None:
        skill = self._apply_editor_to_skill()
        if not skill:
            messagebox.showinfo("提示", "無開啟中的 Agent")
            return
        try:
            issues = self.manager.save(skill)
            self._update_validation_label(skill)
            self.status.set(f"已儲存：{skill.path}（{len(issues)} 項驗證待處理）")
        except OSError as e:
            messagebox.showerror("儲存失敗", str(e))

    def on_validate(self) -> None:
        skill = self._apply_editor_to_skill()
        if not skill:
            return
        self._update_validation_label(skill)

    def on_scan_all(self) -> None:
        self.status.set("掃描中...")
        def worker():
            results = scan_all()
            self.after(0, lambda: self._after_scan(results))
        threading.Thread(target=worker, daemon=True).start()

    def _after_scan(self, results) -> None:
        self._populate_evo_tree(results)
        self.status.set(f"掃描完成：{len(results)} 個 Agent 有問題")

    def _populate_evo_tree(self, results) -> None:
        self.evo_tree.delete(*self.evo_tree.get_children())
        for r in results:
            for issue in r.issues:
                sev = issue["severity"]
                self.evo_tree.insert(
                    "", tk.END,
                    values=(sev, issue["field"], issue["message"], r.skill_path),
                    tags=(sev,),
                )

    def on_evolve(self) -> None:
        cfg = load_config()
        mode = "API 重寫" if cfg.evolution_use_api and cfg.openrouter_api_key else "骨架補齊"
        msg = (
            f"進化模式：{mode}\n"
            f"最低嚴重度門檻：{cfg.evolution_min_severity}\n"
            f"本輪上限：{cfg.evolution_max_agents_per_run} 個 Agent\n"
            f"Dry run：{'是' if cfg.evolution_dry_run else '否'}\n\n"
            "繼續？（修改前會自動備份）"
        )
        if not messagebox.askyesno("執行自我進化", msg):
            return
        self.status.set("進化中...")
        self.update_idletasks()

        def worker() -> None:
            records = evolve_once(cfg=cfg)
            self.after(0, lambda: self._after_evolve(records))

        threading.Thread(target=worker, daemon=True).start()

    def _after_evolve(self, records) -> None:
        fixed_count = sum(len(r.fixed_issues) for r in records)
        actions = {}
        for r in records:
            actions[r.action] = actions.get(r.action, 0) + 1
        self.refresh_tree()
        self.on_scan_all()
        breakdown = "、".join(f"{k}:{v}" for k, v in actions.items())
        self.status.set(
            f"進化完成：處理 {len(records)} 個 Agent（{breakdown}），修復 {fixed_count} 項問題"
        )
        messagebox.showinfo(
            "完成",
            f"已處理 {len(records)} 個 Agent（{breakdown}），修復 {fixed_count} 項問題。\n"
            "日誌：agents/.evolution.log",
        )

    def on_view_log(self) -> None:
        LogDialog(self, read_log(limit=200))

    def on_open_settings(self) -> None:
        SettingsDialog(self)

    def on_import_agents(self) -> None:
        ImportDialog(self, on_done=self.refresh_tree)

    def on_installer(self) -> None:
        InstallerDialog(self, manager=self.manager, on_done=self.refresh_tree)

    def on_ai_edit(self) -> None:
        if not self.current_skill:
            messagebox.showinfo("提示", "請先選取一個 Agent")
            return
        cfg = load_config()
        if not cfg.openrouter_api_key.strip():
            messagebox.showwarning(
                "需要 API Key",
                "AI 局部修改需先到「設定（OpenRouter）」填入 API Key。",
            )
            return
        self._apply_editor_to_skill()  # flush editor → skill
        AIEditDialog(self, cfg=cfg, on_applied=self._after_ai_edit)

    def _after_ai_edit(self, changed: bool) -> None:
        if not changed or not self.current_skill:
            return
        self._load_into_editor(self.current_skill)
        self.status.set("AI 局部修改已套用（已備份）")


class NewAgentDialog(tk.Toplevel):
    def __init__(
        self,
        parent: AgentManagerApp,
        on_done,
        *,
        preset_category: str = "",
        preset_name: str = "",
        preset_description: str = "",
        preset_template: str = "",
    ) -> None:
        super().__init__(parent)
        self.title("新增 Agent")
        self.geometry("640x520")
        self.transient(parent)
        self.grab_set()
        self.manager = parent.manager
        self.on_done = on_done

        _outer = ttk.Frame(self)
        _outer.pack(fill=tk.BOTH, expand=True)
        _canvas = tk.Canvas(_outer, borderwidth=0, highlightthickness=0)
        _vsb = ttk.Scrollbar(_outer, orient=tk.VERTICAL, command=_canvas.yview)
        _canvas.configure(yscrollcommand=_vsb.set)
        _vsb.pack(side=tk.RIGHT, fill=tk.Y)
        _canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        frm = ttk.Frame(_canvas, padding=12)
        _fid = _canvas.create_window((0, 0), window=frm, anchor=tk.NW)
        frm.bind("<Configure>", lambda e: _canvas.configure(scrollregion=_canvas.bbox("all")))
        _canvas.bind("<Configure>", lambda e: _canvas.itemconfig(_fid, width=e.width))
        _canvas.bind("<MouseWheel>", lambda e: _canvas.yview_scroll(int(-1 * (e.delta / 120)), "units"))

        ttk.Label(frm, text="類別").grid(row=0, column=0, sticky=tk.W, pady=4)
        self.cat_var = tk.StringVar(value=preset_category)
        categories = self.manager.categories()
        self.cat_combo = ttk.Combobox(frm, textvariable=self.cat_var, values=categories, state="readonly")
        self.cat_combo.grid(row=0, column=1, sticky=tk.EW, pady=4)
        if preset_category and preset_category in categories:
            self.cat_combo.set(preset_category)
        elif categories:
            self.cat_combo.set(categories[0])

        ttk.Label(frm, text="Agent 名稱").grid(row=1, column=0, sticky=tk.W, pady=4)
        self.name_var = tk.StringVar(value=preset_name)
        ttk.Entry(frm, textvariable=self.name_var).grid(row=1, column=1, sticky=tk.EW, pady=4)

        ttk.Label(frm, text="allowed-tools").grid(row=2, column=0, sticky=tk.W, pady=4)
        self.tools_var = tk.StringVar(value="Read Write")
        ttk.Entry(frm, textvariable=self.tools_var).grid(row=2, column=1, sticky=tk.EW, pady=4)

        ttk.Label(frm, text="模板關鍵字（可留空）").grid(row=3, column=0, sticky=tk.W, pady=4)
        self.tpl_var = tk.StringVar(value=preset_template)
        ttk.Entry(frm, textvariable=self.tpl_var).grid(row=3, column=1, sticky=tk.EW, pady=4)

        ttk.Label(frm, text="description（50–300 字，含啟動時機）").grid(row=4, column=0, sticky=tk.NW, pady=4)
        self.desc = tk.Text(frm, height=5, wrap=tk.WORD)
        self.desc.grid(row=4, column=1, sticky=tk.EW, pady=4)
        if preset_description:
            self.desc.insert("1.0", preset_description)

        ttk.Label(frm, text="角色設定（可選）").grid(row=5, column=0, sticky=tk.NW, pady=4)
        self.role = tk.Text(frm, height=4, wrap=tk.WORD)
        self.role.grid(row=5, column=1, sticky=tk.EW, pady=4)

        ttk.Label(frm, text="核心能力（每行一項）").grid(row=6, column=0, sticky=tk.NW, pady=4)
        self.abilities = tk.Text(frm, height=5, wrap=tk.WORD)
        self.abilities.grid(row=6, column=1, sticky=tk.EW, pady=4)

        ttk.Label(frm, text="AI 提示（一句話描述需求，給 API 生成用）").grid(
            row=7, column=0, sticky=tk.NW, pady=4
        )
        self.ai_brief = tk.Text(frm, height=3, wrap=tk.WORD)
        self.ai_brief.grid(row=7, column=1, sticky=tk.EW, pady=4)

        self.ai_status = ttk.Label(frm, text="", foreground="#555")
        self.ai_status.grid(row=8, column=0, columnspan=2, sticky=tk.W, pady=(4, 0))

        frm.columnconfigure(1, weight=1)

        btns = ttk.Frame(self, padding=(12, 0, 12, 12))
        btns.pack(fill=tk.X)
        ttk.Button(btns, text="取消", command=self._cancel).pack(side=tk.RIGHT, padx=4)
        ttk.Button(btns, text="建立", command=self._submit).pack(side=tk.RIGHT, padx=4)
        ttk.Button(btns, text="AI 生成內容", command=self._on_ai_generate).pack(side=tk.LEFT, padx=4)
        self.ai_body: str | None = None  # AI 回傳的完整 body（若有）

    def _cancel(self) -> None:
        self.on_done(None)
        self.destroy()

    def _submit(self) -> None:
        name = self.name_var.get().strip()
        category = self.cat_var.get().strip()
        desc = self.desc.get("1.0", tk.END).strip()
        if not name or not category or not desc:
            messagebox.showerror("錯誤", "類別、名稱、description 為必填")
            return
        abilities_raw = self.abilities.get("1.0", tk.END).strip()
        abilities = "\n".join(
            line if line.startswith("- ") else f"- {line}"
            for line in abilities_raw.splitlines()
            if line.strip()
        )

        if self.ai_body:
            # 使用 AI 產出的完整 body（含所有章節）
            skill = self.manager.create_from_ai_draft(
                category=category,
                name=name,
                allowed_tools=self.tools_var.get().strip() or "Read Write",
                draft={
                    "description": desc,
                    "role": self.role.get("1.0", tk.END).strip(),
                    "abilities": abilities,
                    "body": self.ai_body,
                },
            )
        else:
            skill = self.manager.create(
                category=category,
                name=name,
                description=desc,
                role=self.role.get("1.0", tk.END).strip(),
                abilities=abilities,
                allowed_tools=self.tools_var.get().strip() or "Read Write",
                template_keyword=self.tpl_var.get().strip(),
            )
        self.on_done(skill)
        self.destroy()

    def _on_ai_generate(self) -> None:
        cfg = load_config()
        if not cfg.openrouter_api_key.strip():
            messagebox.showwarning(
                "需要 API Key",
                "請先到「設定（OpenRouter）」填入 API Key。",
            )
            return
        category = self.cat_var.get().strip()
        name = self.name_var.get().strip()
        brief = self.ai_brief.get("1.0", tk.END).strip()
        if not category or not name or not brief:
            messagebox.showerror("錯誤", "類別、名稱、AI 提示為必填")
            return

        self.ai_status.configure(text="AI 生成中...（呼叫 OpenRouter）", foreground="#555")
        self.update_idletasks()

        def worker() -> None:
            try:
                client = LLMClient(cfg)
                draft = generate_agent_draft(
                    client,
                    category=category,
                    name=name,
                    brief=brief,
                    allowed_tools=self.tools_var.get().strip() or "Read Write",
                )
                self.after(0, lambda: self._fill_from_draft(draft))
            except LLMError as e:
                self.after(0, lambda: self.ai_status.configure(
                    text=f"失敗：{e}", foreground="#b54708"
                ))

        threading.Thread(target=worker, daemon=True).start()

    def _fill_from_draft(self, draft: dict) -> None:
        self.desc.delete("1.0", tk.END)
        self.desc.insert("1.0", str(draft.get("description", "")))
        self.role.delete("1.0", tk.END)
        self.role.insert("1.0", str(draft.get("role", "")))
        self.abilities.delete("1.0", tk.END)
        self.abilities.insert("1.0", str(draft.get("abilities", "")))
        self.ai_body = str(draft.get("body", "")).strip() or None
        self.ai_status.configure(
            text=f"✓ 已由 {draft.get('model', '')} 生成草稿；可修改後按「建立」",
            foreground="#1a7f37",
        )


class LogDialog(tk.Toplevel):
    def __init__(self, parent: tk.Tk, records: list[dict]) -> None:
        super().__init__(parent)
        self.title("進化日誌（最近 200 筆）")
        self.geometry("1120x600")
        cols = ("ts", "action", "model", "fixed", "remaining", "reason", "path")
        tree_frame = ttk.Frame(self)
        tree_frame.pack(fill=tk.BOTH, expand=True, padx=8, pady=8)
        tree = ttk.Treeview(tree_frame, columns=cols, show="headings")
        for col, title, width in [
            ("ts", "時間", 150),
            ("action", "動作", 90),
            ("model", "模型", 180),
            ("fixed", "已修", 60),
            ("remaining", "剩餘", 60),
            ("reason", "決策原因", 200),
            ("path", "檔案", 360),
        ]:
            tree.heading(col, text=title)
            tree.column(col, width=width, anchor=tk.W)
        for r in records:
            tree.insert(
                "", tk.END,
                values=(
                    r.get("timestamp", ""),
                    r.get("action", ""),
                    r.get("model", ""),
                    len(r.get("fixed_issues", [])),
                    len(r.get("remaining_issues", [])),
                    r.get("mode_reason", ""),
                    r.get("skill_path", ""),
                ),
            )
        log_scroll = ttk.Scrollbar(tree_frame, orient=tk.VERTICAL, command=tree.yview)
        tree.configure(yscrollcommand=log_scroll.set)
        tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        log_scroll.pack(side=tk.RIGHT, fill=tk.Y)
        ttk.Button(self, text="關閉", command=self.destroy).pack(pady=(0, 8))


class SettingsDialog(tk.Toplevel):
    """OpenRouter 與進化規則設定。"""

    def __init__(self, parent: tk.Tk) -> None:
        super().__init__(parent)
        self.title("設定 — OpenRouter / 進化規則")
        self.geometry("640x620")
        self.transient(parent)
        self.grab_set()
        self.cfg = load_config()

        nb = ttk.Notebook(self)
        nb.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        self._build_api_tab(nb)
        self._build_evolution_tab(nb)

        btns = ttk.Frame(self)
        btns.pack(fill=tk.X, padx=10, pady=(0, 10))
        ttk.Button(btns, text="測試連線", command=self._on_test).pack(side=tk.LEFT)
        ttk.Button(btns, text="取消", command=self._on_close).pack(side=tk.RIGHT, padx=(4, 0))
        ttk.Button(btns, text="儲存", command=self._on_save).pack(side=tk.RIGHT)

        self.status_lbl = ttk.Label(self, text="", foreground="#555")
        self.status_lbl.pack(fill=tk.X, padx=10, pady=(0, 10))
        self.protocol("WM_DELETE_WINDOW", self._on_close)

    def _build_api_tab(self, nb: ttk.Notebook) -> None:
        frm = ttk.Frame(nb, padding=12)
        nb.add(frm, text="OpenRouter")

        self.var_key = tk.StringVar(value=self.cfg.openrouter_api_key)
        self.var_base = tk.StringVar(value=self.cfg.openrouter_base_url)
        self.var_primary = tk.StringVar(value=self.cfg.primary_model)
        self.var_fallback = tk.StringVar(value=self.cfg.fallback_model)
        self.var_timeout = tk.IntVar(value=self.cfg.request_timeout)
        self.var_max_tokens = tk.IntVar(value=self.cfg.max_tokens)
        self.var_temp = tk.DoubleVar(value=self.cfg.temperature)

        rows = [
            ("API Key", self.var_key, {"show": "*"}),
            ("Base URL", self.var_base, {}),
            ("Timeout (sec)", self.var_timeout, {}),
            ("Max tokens", self.var_max_tokens, {}),
            ("Temperature (0.0–1.0)", self.var_temp, {}),
        ]
        for row, (label, var, kwargs) in enumerate(rows):
            ttk.Label(frm, text=label).grid(row=row, column=0, sticky=tk.W, pady=4)
            ttk.Entry(frm, textvariable=var, **kwargs).grid(row=row, column=1, sticky=tk.EW, pady=4)

        ttk.Label(frm, text="主模型").grid(row=5, column=0, sticky=tk.W, pady=4)
        models = all_models(self.cfg)
        ttk.Combobox(frm, textvariable=self.var_primary, values=models).grid(
            row=5, column=1, sticky=tk.EW, pady=4
        )
        ttk.Label(frm, text="備援模型").grid(row=6, column=0, sticky=tk.W, pady=4)
        ttk.Combobox(frm, textvariable=self.var_fallback, values=models).grid(
            row=6, column=1, sticky=tk.EW, pady=4
        )

        ttk.Label(frm, text="自訂模型（每行一個 slug，例如 anthropic/claude-opus-4.7）").grid(
            row=7, column=0, columnspan=2, sticky=tk.W, pady=(12, 2)
        )
        self.txt_custom = tk.Text(frm, height=6, wrap=tk.NONE)
        self.txt_custom.grid(row=8, column=0, columnspan=2, sticky=tk.EW, pady=4)
        if self.cfg.custom_models:
            self.txt_custom.insert("1.0", "\n".join(self.cfg.custom_models))

        frm.columnconfigure(1, weight=1)

    def _build_evolution_tab(self, nb: ttk.Notebook) -> None:
        frm = ttk.Frame(nb, padding=12)
        nb.add(frm, text="進化規則")

        self.var_use_api = tk.BooleanVar(value=self.cfg.evolution_use_api)
        self.var_auto = tk.BooleanVar(value=self.cfg.evolution_auto_apply)
        self.var_valid = tk.BooleanVar(value=self.cfg.evolution_require_validation)
        self.var_dry = tk.BooleanVar(value=self.cfg.evolution_dry_run)
        self.var_min_sev = tk.StringVar(value=self.cfg.evolution_min_severity)
        self.var_max_agents = tk.IntVar(value=self.cfg.evolution_max_agents_per_run)

        ttk.Checkbutton(
            frm, text="使用 OpenRouter API 重寫（需已填入 API Key）",
            variable=self.var_use_api,
        ).grid(row=0, column=0, columnspan=2, sticky=tk.W, pady=4)
        ttk.Checkbutton(frm, text="自動套用修復（未啟用則僅提出建議）",
                        variable=self.var_auto).grid(row=1, column=0, columnspan=2, sticky=tk.W, pady=4)
        ttk.Checkbutton(frm, text="修復後仍要通過驗證（未通過會記錄剩餘問題）",
                        variable=self.var_valid).grid(row=2, column=0, columnspan=2, sticky=tk.W, pady=4)
        ttk.Checkbutton(frm, text="Dry run（API 模式不寫入檔案）",
                        variable=self.var_dry).grid(row=3, column=0, columnspan=2, sticky=tk.W, pady=4)

        ttk.Label(frm, text="最低嚴重度門檻").grid(row=4, column=0, sticky=tk.W, pady=4)
        ttk.Combobox(
            frm, textvariable=self.var_min_sev,
            values=["CRITICAL", "HIGH", "MEDIUM", "LOW"],
            state="readonly",
        ).grid(row=4, column=1, sticky=tk.EW, pady=4)

        ttk.Label(frm, text="本輪最多處理 Agent 數").grid(row=5, column=0, sticky=tk.W, pady=4)
        ttk.Spinbox(frm, from_=1, to=500, textvariable=self.var_max_agents).grid(
            row=5, column=1, sticky=tk.EW, pady=4
        )

        ttk.Label(
            frm,
            text=(
                "規則優先順序：\n"
                "1. 無問題 → 跳過\n"
                "2. 問題低於門檻 → 僅建議\n"
                "3. 已啟用 API + 有 Key → API 重寫\n"
                "4. 自動套用 → 骨架補齊\n"
                "5. 其他 → 僅建議"
            ),
            foreground="#666",
            justify=tk.LEFT,
        ).grid(row=6, column=0, columnspan=2, sticky=tk.W, pady=(16, 4))

        frm.columnconfigure(1, weight=1)

    def _collect(self):
        self.cfg.openrouter_api_key = self.var_key.get().strip()
        self.cfg.openrouter_base_url = self.var_base.get().strip() or "https://openrouter.ai/api/v1"
        self.cfg.primary_model = self.var_primary.get().strip()
        self.cfg.fallback_model = self.var_fallback.get().strip()
        try:
            self.cfg.request_timeout = int(self.var_timeout.get())
        except (ValueError, tk.TclError):
            self.cfg.request_timeout = 60
        try:
            self.cfg.max_tokens = int(self.var_max_tokens.get())
        except (ValueError, tk.TclError):
            self.cfg.max_tokens = 2048
        try:
            self.cfg.temperature = float(self.var_temp.get())
        except (ValueError, tk.TclError):
            self.cfg.temperature = 0.4
        custom_raw = self.txt_custom.get("1.0", tk.END).strip().splitlines()
        self.cfg.custom_models = [m.strip() for m in custom_raw if m.strip()]
        self.cfg.evolution_use_api = bool(self.var_use_api.get())
        self.cfg.evolution_auto_apply = bool(self.var_auto.get())
        self.cfg.evolution_require_validation = bool(self.var_valid.get())
        self.cfg.evolution_dry_run = bool(self.var_dry.get())
        self.cfg.evolution_min_severity = self.var_min_sev.get().strip().upper() or "HIGH"
        try:
            self.cfg.evolution_max_agents_per_run = max(1, int(self.var_max_agents.get()))
        except (ValueError, tk.TclError):
            self.cfg.evolution_max_agents_per_run = 20

    def _on_save(self) -> None:
        self._collect()
        save_config(self.cfg)
        self.status_lbl.configure(text="✓ 已儲存", foreground="#1a7f37")
        self._pending_destroy = self.after(700, self.destroy)

    def _on_close(self) -> None:
        if hasattr(self, "_pending_destroy") and self._pending_destroy:
            try:
                self.after_cancel(self._pending_destroy)
            except Exception:
                pass
        self.destroy()

    def _on_test(self) -> None:
        self._collect()
        self.status_lbl.configure(text="測試中...", foreground="#555")
        self.update_idletasks()

        def worker() -> None:
            client = LLMClient(self.cfg)
            msg = client.ping()
            self.after(0, lambda: self.status_lbl.configure(
                text=msg,
                foreground="#1a7f37" if msg.startswith("OK") else "#b54708",
            ))

        threading.Thread(target=worker, daemon=True).start()


class AIEditDialog(tk.Toplevel):
    """AI 局部修改：依指令 + 範圍 重寫 description / body / 特定章節。"""

    SCOPE_LABELS = [
        ("整份 body（所有章節）", "body"),
        ("description（frontmatter）", "description"),
        ("## 角色設定", "## 角色設定"),
        ("## 核心能力", "## 核心能力"),
        ("## 操作流程", "## 操作流程"),
        ("## 輸入範例", "## 輸入範例"),
        ("## 輸出範例", "## 輸出範例"),
        ("## 邊緣案例處理", "## 邊緣案例處理"),
    ]

    def __init__(self, parent: "AgentManagerApp", *, cfg, on_applied) -> None:
        super().__init__(parent)
        self.title("AI 局部修改")
        self.geometry("820x640")
        self.transient(parent)
        self.grab_set()
        self.app = parent
        self.cfg = cfg
        self.on_applied = on_applied
        self.skill = parent.current_skill
        self._new_content: str | None = None
        self._scope_value: str = "body"

        frm = ttk.Frame(self, padding=12)
        frm.pack(fill=tk.BOTH, expand=True)

        ttk.Label(frm, text=f"Agent：{self.skill.path.parent.name}").pack(anchor=tk.W)

        top = ttk.Frame(frm)
        top.pack(fill=tk.X, pady=(8, 4))
        ttk.Label(top, text="範圍").pack(side=tk.LEFT)
        self.scope_var = tk.StringVar(value=self.SCOPE_LABELS[0][0])
        ttk.Combobox(
            top,
            textvariable=self.scope_var,
            values=[label for label, _ in self.SCOPE_LABELS],
            state="readonly",
            width=32,
        ).pack(side=tk.LEFT, padx=6)

        ttk.Label(frm, text="修改指令（具體越好，例如「為 ## 操作流程 新增風險評估步驟」）").pack(
            anchor=tk.W, pady=(8, 2)
        )
        self.txt_instr = tk.Text(frm, height=4, wrap=tk.WORD)
        self.txt_instr.pack(fill=tk.X)

        ttk.Label(frm, text="AI 輸出預覽（可再編輯後套用）").pack(anchor=tk.W, pady=(8, 2))
        preview_frm = ttk.Frame(frm)
        preview_frm.pack(fill=tk.BOTH, expand=True)
        self.txt_preview = tk.Text(preview_frm, wrap=tk.WORD, font=("Consolas", 10))
        preview_scroll = ttk.Scrollbar(preview_frm, orient=tk.VERTICAL, command=self.txt_preview.yview)
        self.txt_preview.configure(yscrollcommand=preview_scroll.set)
        self.txt_preview.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        preview_scroll.pack(side=tk.RIGHT, fill=tk.Y)

        self.status = ttk.Label(frm, text="", foreground="#555")
        self.status.pack(anchor=tk.W, pady=(4, 0))

        btns = ttk.Frame(self, padding=(12, 0, 12, 12))
        btns.pack(fill=tk.X)
        ttk.Button(btns, text="關閉", command=self._close).pack(side=tk.RIGHT, padx=4)
        ttk.Button(btns, text="套用到檔案（自動備份）", command=self._apply).pack(
            side=tk.RIGHT, padx=4
        )
        ttk.Button(btns, text="生成", command=self._generate).pack(side=tk.LEFT, padx=4)
        self.protocol("WM_DELETE_WINDOW", self._on_close)

    def _current_scope(self) -> tuple[str, str]:
        label = self.scope_var.get()
        for lab, val in self.SCOPE_LABELS:
            if lab == label:
                return lab, val
        return self.SCOPE_LABELS[0]

    def _extract_scope_content(self, scope_val: str) -> str:
        if scope_val == "body":
            return self.skill.body or ""
        if scope_val == "description":
            return str((self.skill.frontmatter or {}).get("description", ""))
        # 某個 `## 標題` 章節 → 擷取該章節至下一個 `## ` 為止
        body = self.skill.body or ""
        marker = scope_val + "\n"
        idx = body.find(scope_val)
        if idx < 0:
            return f"{scope_val}\n（目前檔案無此章節，AI 會新增）"
        tail = body[idx:]
        next_idx = tail.find("\n## ", len(scope_val))
        if next_idx < 0:
            return tail.strip()
        return tail[:next_idx].strip()

    def _generate(self) -> None:
        instruction = self.txt_instr.get("1.0", tk.END).strip()
        if not instruction:
            messagebox.showerror("錯誤", "請輸入修改指令")
            return
        label, scope_val = self._current_scope()
        self._scope_value = scope_val
        current = self._extract_scope_content(scope_val)
        self.status.configure(text=f"AI 生成中...（範圍：{label}）", foreground="#555")
        self.update_idletasks()

        def worker() -> None:
            try:
                client = LLMClient(self.cfg)
                new_text, model = edit_text_with_ai(
                    client,
                    current=current,
                    instruction=instruction,
                    scope=label,
                )
                self.after(0, lambda: self._show_result(new_text, model))
            except LLMError as e:
                self.after(0, lambda: self.status.configure(
                    text=f"失敗：{e}", foreground="#b54708"
                ))

        threading.Thread(target=worker, daemon=True).start()

    def _show_result(self, new_text: str, model: str) -> None:
        self.txt_preview.delete("1.0", tk.END)
        self.txt_preview.insert("1.0", new_text)
        self._new_content = new_text
        self.status.configure(
            text=f"✓ {model} 已生成；可直接編輯預覽再按「套用」",
            foreground="#1a7f37",
        )

    def _apply(self) -> None:
        preview = self.txt_preview.get("1.0", tk.END).strip()
        if not preview:
            messagebox.showinfo("提示", "預覽為空，請先按「生成」")
            return
        _, scope_val = self._current_scope()
        skill = self.skill
        if scope_val == "description":
            skill.frontmatter = dict(skill.frontmatter or {})
            skill.frontmatter["description"] = preview
        elif scope_val == "body":
            skill.body = preview + "\n"
        else:
            # 章節替換
            body = skill.body or ""
            idx = body.find(scope_val)
            if idx < 0:
                skill.body = body.rstrip() + "\n\n" + preview + "\n"
            else:
                tail = body[idx:]
                next_idx = tail.find("\n## ", len(scope_val))
                if next_idx < 0:
                    skill.body = body[:idx] + preview + "\n"
                else:
                    skill.body = body[:idx] + preview + "\n" + tail[next_idx + 1 :]
        self.app.manager.save(skill)
        self.status.configure(text="✓ 已套用並備份至 .backup/", foreground="#1a7f37")
        self.on_applied(True)
        self._pending_destroy = self.after(600, self.destroy)

    def _close(self) -> None:
        self.on_applied(False)
        self._on_close()

    def _on_close(self) -> None:
        if hasattr(self, "_pending_destroy") and self._pending_destroy:
            try:
                self.after_cancel(self._pending_destroy)
            except Exception:
                pass
        self.destroy()


class ImportDialog(tk.Toplevel):
    """匯入 agency-agents-main 目錄中的 agents。"""

    def __init__(self, parent: tk.Tk, *, on_done: callable) -> None:
        super().__init__(parent)
        self.title("匯入 Agency Agents")
        self.geometry("600x320")
        self.resizable(False, False)
        self.transient(parent)
        self.grab_set()
        self._on_done = on_done

        frm = ttk.Frame(self, padding=16)
        frm.pack(fill=tk.BOTH, expand=True)

        ttk.Label(frm, text="來源目錄：").grid(row=0, column=0, sticky=tk.W, pady=6)
        self.var_src = tk.StringVar(value=str(DEFAULT_SOURCE))
        ttk.Entry(frm, textvariable=self.var_src, width=42).grid(row=0, column=1, sticky=tk.EW, padx=4)
        ttk.Button(frm, text="瀏覽…", command=self._browse).grid(row=0, column=2, padx=4)

        self.var_skip = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            frm, text="跳過已存在的 Agent（不覆蓋）", variable=self.var_skip,
        ).grid(row=1, column=0, columnspan=3, sticky=tk.W, pady=6)

        self.lbl_count = ttk.Label(frm, text="", foreground="#555")
        self.lbl_count.grid(row=2, column=0, columnspan=3, sticky=tk.W, pady=2)

        self.progress = ttk.Progressbar(frm, mode="determinate", length=400)
        self.progress.grid(row=3, column=0, columnspan=3, sticky=tk.EW, pady=8)

        self.lbl_status = ttk.Label(frm, text="就緒", foreground="#555")
        self.lbl_status.grid(row=4, column=0, columnspan=3, sticky=tk.W)

        frm.columnconfigure(1, weight=1)

        btns = ttk.Frame(self)
        btns.pack(fill=tk.X, padx=16, pady=(0, 12))
        self.btn_start = ttk.Button(btns, text="開始匯入", command=self._start)
        self.btn_start.pack(side=tk.LEFT)
        ttk.Button(btns, text="關閉", command=self._close).pack(side=tk.RIGHT)

        self._refresh_count()

    def _browse(self) -> None:
        from tkinter import filedialog
        path = filedialog.askdirectory(title="選擇 agency-agents-main 目錄")
        if path:
            self.var_src.set(path)
            self._refresh_count()

    def _refresh_count(self) -> None:
        from pathlib import Path
        src = Path(self.var_src.get().strip())
        if src.exists():
            n = count_importable(src)
            self.lbl_count.configure(text=f"可匯入 Agent 數：{n}")
        else:
            self.lbl_count.configure(text="⚠ 目錄不存在")

    def _start(self) -> None:
        from pathlib import Path
        src = Path(self.var_src.get().strip())
        if not src.exists():
            from tkinter import messagebox as mb
            mb.showerror("錯誤", "來源目錄不存在", parent=self)
            return
        self.btn_start.configure(state="disabled")
        self.progress["value"] = 0

        def _progress(current: int, total: int) -> None:
            self.after(0, lambda c=current, t=total: self._update_import_progress(c, t))

        def worker() -> None:
            imported, skipped = import_all(
                src,
                skip_existing=bool(self.var_skip.get()),
                progress_callback=_progress,
            )
            self.after(0, lambda: self._done(imported, skipped))

        threading.Thread(target=worker, daemon=True).start()

    def _update_import_progress(self, current: int, total: int) -> None:
        self.progress["maximum"] = total
        self.progress["value"] = current
        self.lbl_status.configure(text=f"處理中 {current}/{total}…")

    def _done(self, imported: int, skipped: int) -> None:
        self.lbl_status.configure(
            text=f"✓ 完成：新增 {imported} 個，跳過 {skipped} 個",
            foreground="#1a7f37",
        )
        self.btn_start.configure(state="normal")
        self._on_done()

    def _close(self) -> None:
        self.destroy()


class InstallerDialog(tk.Toplevel):
    """安裝選取的 Agents 到 AI 工具 / 從工具備份（Windows 實驗功能）。"""

    def __init__(self, parent: tk.Tk, *, manager: AgentManager, on_done: callable) -> None:
        super().__init__(parent)
        self.title("安裝/備份工具 🔧  [Windows 實驗]")
        self.geometry("1060x680")
        self.minsize(900, 580)
        self.transient(parent)
        self.grab_set()
        self._manager = manager
        self._on_done = on_done
        self._tool_vars: dict[str, tk.BooleanVar] = {}
        self._tool_paths: dict[str, tk.StringVar] = {}

        self._build()
        self._load_agents()

    # ── layout ────────────────────────────────────────────

    def _build(self) -> None:
        paned = ttk.PanedWindow(self, orient=tk.HORIZONTAL)
        paned.pack(fill=tk.BOTH, expand=True, padx=8, pady=8)

        # Left — agent list
        left = ttk.LabelFrame(paned, text="選擇 Agents", padding=6)
        paned.add(left, weight=2)

        search_frm = ttk.Frame(left)
        search_frm.pack(fill=tk.X, pady=(0, 4))
        self._search_var = tk.StringVar()
        self._search_var.trace_add("write", lambda *_: self._filter_agents())
        ttk.Entry(search_frm, textvariable=self._search_var,
                  width=28).pack(side=tk.LEFT, fill=tk.X, expand=True)
        ttk.Button(search_frm, text="全選", width=5,
                   command=self._select_all).pack(side=tk.LEFT, padx=2)
        ttk.Button(search_frm, text="清除", width=5,
                   command=self._deselect_all).pack(side=tk.LEFT)

        cols = ("category", "name")
        self._agent_tree = ttk.Treeview(left, columns=cols, show="headings",
                                        selectmode="extended", height=28)
        self._agent_tree.heading("category", text="類別", anchor=tk.W)
        self._agent_tree.heading("name", text="Agent 名稱", anchor=tk.W)
        self._agent_tree.column("category", width=120)
        self._agent_tree.column("name", width=200)
        sb = ttk.Scrollbar(left, orient=tk.VERTICAL, command=self._agent_tree.yview)
        self._agent_tree.configure(yscrollcommand=sb.set)
        self._agent_tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        sb.pack(side=tk.RIGHT, fill=tk.Y)

        # Right — notebook Install / Backup
        right = ttk.Frame(paned)
        paned.add(right, weight=3)

        nb = ttk.Notebook(right)
        nb.pack(fill=tk.BOTH, expand=True)
        self._build_install_tab(nb)
        self._build_backup_tab(nb)

        # Bottom status
        self._status = tk.StringVar(value="就緒")
        self._progress = ttk.Progressbar(right, mode="determinate", length=600)
        self._progress.pack(fill=tk.X, padx=4, pady=(4, 0))
        ttk.Label(right, textvariable=self._status, anchor=tk.W,
                  foreground="#555").pack(fill=tk.X, padx=4)
        ttk.Button(right, text="關閉", command=self.destroy).pack(
            side=tk.RIGHT, padx=8, pady=6)

    # ── Install tab ────────────────────────────────────────

    def _build_install_tab(self, nb: ttk.Notebook) -> None:
        outer = ttk.Frame(nb, padding=6)
        nb.add(outer, text="安裝到工具")

        ttk.Label(outer, text="選擇目標工具（可多選）：", foreground="#333").pack(anchor=tk.W)

        canvas = tk.Canvas(outer, borderwidth=0, highlightthickness=0, height=360)
        vsb = ttk.Scrollbar(outer, orient=tk.VERTICAL, command=canvas.yview)
        canvas.configure(yscrollcommand=vsb.set)
        vsb.pack(side=tk.RIGHT, fill=tk.Y)
        canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        inner = ttk.Frame(canvas)
        canvas.create_window((0, 0), window=inner, anchor=tk.NW)
        inner.bind("<Configure>",
                   lambda e: canvas.configure(scrollregion=canvas.bbox("all")))

        for tool in TOOLS:
            self._add_tool_row(inner, tool, mode="install")

        hint = ttk.Label(outer,
            text="★ 專案範圍工具需指定目標專案目錄\n"
                 "⚠ Aider/Windsurf 採「附加」模式（多次安裝會累積，請先刪除舊檔）",
            foreground="#888", justify=tk.LEFT)
        hint.pack(anchor=tk.W, pady=(4, 0))

        btn_frm = ttk.Frame(outer)
        btn_frm.pack(fill=tk.X, pady=4)
        ttk.Button(btn_frm, text="▶ 安裝選取的 Agents",
                   command=self._run_install).pack(side=tk.LEFT)

    # ── Backup tab ─────────────────────────────────────────

    def _build_backup_tab(self, nb: ttk.Notebook) -> None:
        outer = ttk.Frame(nb, padding=6)
        nb.add(outer, text="從工具備份")

        ttk.Label(outer, text="選擇來源工具（備份到此 App agents/ 目錄）：",
                  foreground="#333").pack(anchor=tk.W)

        canvas = tk.Canvas(outer, borderwidth=0, highlightthickness=0, height=360)
        vsb = ttk.Scrollbar(outer, orient=tk.VERTICAL, command=canvas.yview)
        canvas.configure(yscrollcommand=vsb.set)
        vsb.pack(side=tk.RIGHT, fill=tk.Y)
        canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        inner = ttk.Frame(canvas)
        canvas.create_window((0, 0), window=inner, anchor=tk.NW)
        inner.bind("<Configure>",
                   lambda e: canvas.configure(scrollregion=canvas.bbox("all")))

        for tool in TOOLS:
            self._add_tool_row(inner, tool, mode="backup")

        self._skip_existing = tk.BooleanVar(value=True)
        ttk.Checkbutton(outer, text="跳過已存在的 Agent",
                        variable=self._skip_existing).pack(anchor=tk.W, pady=4)
        ttk.Button(outer, text="▶ 備份到此 App",
                   command=self._run_backup).pack(anchor=tk.W)

    # ── tool rows ──────────────────────────────────────────

    def _add_tool_row(self, parent: ttk.Frame, tool: ToolDef, *, mode: str) -> None:
        row = ttk.Frame(parent, padding=(0, 2))
        row.pack(fill=tk.X, pady=1)

        key = f"{mode}_{tool.id}"
        var = self._tool_vars.setdefault(key, tk.BooleanVar(value=False))
        ttk.Checkbutton(row, variable=var, width=2).pack(side=tk.LEFT)

        ttk.Label(row, text=tool.name, width=16, anchor=tk.W,
                  font=("", 9, "bold")).pack(side=tk.LEFT)

        path_key = f"{mode}_{tool.id}_path"
        path_var = self._tool_paths.setdefault(
            path_key, tk.StringVar(value=str(tool.default_path))
        )
        path_entry = ttk.Entry(row, textvariable=path_var, width=38)
        path_entry.pack(side=tk.LEFT, padx=(4, 2))

        def _browse(pv=path_var):
            from tkinter import filedialog
            d = filedialog.askdirectory(title="選擇目錄")
            if d:
                pv.set(d)

        ttk.Button(row, text="…", width=3, command=_browse).pack(side=tk.LEFT)

        label_text = "★ 專案" if tool.project_scoped else ""
        ttk.Label(row, text=label_text, foreground="#b54708",
                  width=5).pack(side=tk.LEFT, padx=2)
        ttk.Label(row, text=tool.description,
                  foreground="#777").pack(side=tk.LEFT)

    # ── agent list ─────────────────────────────────────────

    def _load_agents(self) -> None:
        self._all_agents: list[AgentSkill] = []
        for cat in self._manager.categories():
            for skill in self._manager.list(cat):
                self._all_agents.append(skill)
        self._filter_agents()

    def _filter_agents(self) -> None:
        q = self._search_var.get().lower()
        self._agent_tree.delete(*self._agent_tree.get_children())
        for skill in self._all_agents:
            cat = skill.path.parent.parent.name
            name = skill.name or skill.slug
            if q and q not in name.lower() and q not in cat.lower():
                continue
            self._agent_tree.insert("", tk.END, iid=str(skill.path),
                                    values=(cat, name))

    def _select_all(self) -> None:
        self._agent_tree.selection_set(self._agent_tree.get_children())

    def _deselect_all(self) -> None:
        self._agent_tree.selection_remove(self._agent_tree.get_children())

    def _selected_skills(self) -> list[AgentSkill]:
        paths = {str(s.path): s for s in self._all_agents}
        return [paths[iid] for iid in self._agent_tree.selection()
                if iid in paths]

    # ── install ────────────────────────────────────────────

    def _run_install(self) -> None:
        skills = self._selected_skills()
        if not skills:
            messagebox.showwarning("提示", "請先在左側選取至少一個 Agent", parent=self)
            return

        targets = [
            (t, self._tool_paths.get(f"install_{t.id}_path"))
            for t in TOOLS
            if self._tool_vars.get(f"install_{t.id}", tk.BooleanVar()).get()
        ]
        if not targets:
            messagebox.showwarning("提示", "請勾選至少一個目標工具", parent=self)
            return

        total_ops = len(skills) * len(targets)
        done = [0]
        results: list[str] = []

        def _prog(i, n, slug, tool_name=""):
            done[0] += 1
            d, s, tn = done[0], slug, tool_name
            self.after(0, lambda: self._update_install_progress(d, total_ops, s, tn))

        def worker() -> None:
            for tool, path_var in targets:
                target_path = Path(path_var.get().strip()) if path_var else tool.default_path
                ok, fail = install_agents(
                    skills, tool.id, target=target_path,
                    progress_callback=lambda i, n, s, tn=tool.name: _prog(i, n, s, tn),
                )
                results.append(f"{tool.name}: 成功 {ok}，失敗 {fail}")
            self.after(0, lambda: self._install_done(results))

        threading.Thread(target=worker, daemon=True).start()

    def _install_done(self, results: list[str]) -> None:
        msg = "\n".join(results)
        self._status.set("安裝完成 ✓")
        messagebox.showinfo("安裝完成", msg, parent=self)
        self._on_done()

    def _update_install_progress(self, done_count: int, total_ops: int, slug: str, tool_name: str) -> None:
        self._progress["maximum"] = total_ops
        self._progress["value"] = done_count
        self._status.set(f"安裝中… {done_count}/{total_ops}  {slug} → {tool_name}")

    # ── backup ─────────────────────────────────────────────

    def _run_backup(self) -> None:
        sources = [
            (t, self._tool_paths.get(f"backup_{t.id}_path"))
            for t in TOOLS
            if self._tool_vars.get(f"backup_{t.id}", tk.BooleanVar()).get()
        ]
        if not sources:
            messagebox.showwarning("提示", "請勾選至少一個來源工具", parent=self)
            return

        results: list[str] = []
        total_tools = len(sources)

        def _prog(i, n, slug):
            s = slug
            self.after(0, lambda s=s: self._status.set(f"備份中… {s}"))

        def worker() -> None:
            for idx, (tool, path_var) in enumerate(sources):
                src = Path(path_var.get().strip()) if path_var else tool.default_path
                self.after(0, lambda t=tool, ix=idx: self._status.set(
                    f"備份 {t.name}… ({ix+1}/{total_tools})"
                ))
                ok, skip = backup_from_tool(
                    tool.id, source=src,
                    skip_existing=self._skip_existing.get(),
                    progress_callback=_prog,
                )
                results.append(f"{tool.name}: 新增 {ok}，跳過 {skip}")
            self.after(0, lambda: self._backup_done(results))

        threading.Thread(target=worker, daemon=True).start()

    def _backup_done(self, results: list[str]) -> None:
        msg = "\n".join(results)
        self._status.set("備份完成 ✓")
        messagebox.showinfo("備份完成", msg, parent=self)
        self._on_done()


def main() -> None:
    app = AgentManagerApp()
    app.mainloop()


if __name__ == "__main__":
    main()
