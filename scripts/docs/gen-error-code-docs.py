#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""错误码文档生成器（批次 B：错误码文档归一）

数据源（唯一权威）：
  1. `src/util/diagnostic/codes/*.rs` —— `define_codes!` 注册表单源宏（code / category /
     span_exempt / 构造函数名 / 参数名 / 条目上方中文注释）。
  2. `locales/zh.json` —— 用户可见文案的唯一权威（title / template / help）。
  3. `src/**` —— 扫描每个码是否真有发射点（`ErrorCodeDefinition::<fn>(` 调用，
     以及 std 层 `error_new("EXXXX", …)` 字面量）。

输出（全部 UTF-8 无 BOM + LF）：
  docs/src/reference/error-code/index.md              全量清单页
  docs/src/reference/error-code/E0xxx.md … E8xxx.md  分族页
  docs/src/reference/warning-code/warning-codes.md    警告码页

本文件重跑幂等：同样输入必得同样输出。改文案请改 `locales/*.json`，改码表请改
`define_codes!` 注册表，然后重跑：

    python scripts/docs/gen-error-code-docs.py
"""

from __future__ import annotations

import json
import os
import re
import sys
from collections import OrderedDict, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CODES_DIR = ROOT / "src" / "util" / "diagnostic" / "codes"
SRC_DIR = ROOT / "src"
ZH_JSON = ROOT / "locales" / "zh.json"
DOCS = ROOT / "docs" / "src" / "reference"

# 分族页元数据：文件前缀 -> (category, 页面标题, 页面说明)
FAMILIES = OrderedDict(
    [
        ("E0xxx", ("Lexer / Parser", "词法与语法分析错误", "词法分析器（Lexer）与语法分析器（Parser）阶段产生的错误。")),
        ("E1xxx", ("TypeCheck", "类型检查错误", "类型检查阶段产生的错误，涵盖类型匹配、模式匹配、泛型实例化、接口约束与 `?` 错误传播等。")),
        ("E2xxx", ("Semantic", "语义分析错误", "语义分析阶段产生的错误，涵盖作用域、变量生命周期、所有权与函数签名解析等。")),
        ("E4xxx", ("Generic", "泛型与特质错误", "泛型约束、特质系统与常量求值相关的错误。")),
        ("E5xxx", ("Module", "模块与导入错误", "模块系统与导入解析相关的错误。")),
        ("E6xxx", ("Runtime", "运行时错误", "程序运行期（VM / std 原生函数）产生的错误。")),
        ("E7xxx", ("Io", "I/O 与系统错误", "I/O 操作与系统调用失败产生的错误。")),
        ("E8xxx", ("Internal", "内部编译器错误", "编译器内部错误，通常表示编译器自身的缺陷。遇到此类错误请提交 issue。")),
    ]
)

# `define_codes!` 条目：("EXXXX", Category, span_exempt, fn_name(args) => .chain...)
ENTRY_RE = re.compile(
    r'\(\s*"(?P<code>[EW]\d{4})"\s*,\s*'
    r"(?P<cat>[A-Za-z_][A-Za-z0-9_]*)\s*,\s*"
    r"(?P<exempt>true|false)\s*,\s*"
    r"(?P<fn>[A-Za-z_][A-Za-z0-9_]*)\s*"
    r"\((?P<params>[^)]*)\)\s*=>\s*"
    r"(?P<chain>.*?)\)\s*,?\s*$"
)
DEFINE_RE = re.compile(r"define_codes!\s*\(\s*(?P<statics>[A-Z0-9_]+)\s*,\s*\{(?P<body>.*?)\}\s*\)", re.S)

# 条目上方的注释（源码里的中文名权威源）。实际代码用 `//`，宏 doc 示例用 `///`。
COMMENT_RE = re.compile(r"^\s*//[/!]?\s*(?P<text>.*?)\s*$")
CODE_PREFIX_RE = re.compile(r"^(?P<code>[EW]\d{4})\s*(?P<name>.*)$")

# 发射点扫描
FN_CALL_RE_TMPL = r"(?:ErrorCodeDefinition\s*::\s*)?{fn}\s*\("
ERROR_NEW_RE = re.compile(r'error_new\s*\(\s*"(?P<code>[EW]\d{{4}})"')
FIND_CODE_RE = re.compile(r'ErrorCodeDefinition\s*::\s*find\s*\(\s*"(?P<code>[EW]\d{{4}})"')
CODE_LITERAL_RE = re.compile(r'"(?P<code>[EW]\d{4})"')
CODE_KEY_RE = re.compile(r"[EW]\d{4}")
# 裸码号字面量（字符串化发射点 / find 查询 / std 层 error_new）
BARE_CODE_LITERAL_RE = re.compile(r'"(?P<code>[EW]\d{4})"')

# 扫描 src/ 时跳过的目录
SKIP_DIRS = {"target", ".git", "node_modules"}

# 包管理器错误：thiserror 变体（`src/package/error.rs`），没有 E/W 码号
PKG_ERROR_RS = ROOT / "src" / "package" / "error.rs"
PKG_VARIANT_RE = re.compile(
    r"(?P<doc>(?:\s*///[^\n]*\n)+)"       # 上方 doc 注释（可多行）
    r'\s*#\[error\((?P<fmt>"(?:[^"\\]|\\.)*")\)\]\s*\n'  # #[error("…")]
    r"\s*(?P<name>[A-Z][A-Za-z0-9]*)"  # 变体名
    r"\s*(?:\(|\{|,|$)",                # 带 payload / 无 payload
    re.M,
)

# `PackageError` 变体的中文说明。源码 doc 注释是英文（且部分含 RFC 编号），
# 用户可见文案只能给英文 `Display`，因此这里补一层中文释义作为文档正文。
PKG_VARIANT_ZH = {
    "ProjectExists": "目标目录已存在（`yx init` 不会覆盖）",
    "NotProject": "当前目录不是 YaoXiang 项目：找不到 `yaoxiang.toml`",
    "DependencyNotFound": "依赖未在 `yaoxiang.toml` 中声明",
    "DependencyAlreadyExists": "依赖已在 `yaoxiang.toml` 中声明",
    "DependencyInstallFailed": "一个或多个依赖安装失败",
    "InvalidManifest": "`yaoxiang.toml` 格式非法或内容不完整",
    "Cache": "全局包缓存读写失败（RFC-014 Phase 3）",
    "NotWorkspace": "当前不在工作区内：任一 `yaoxiang.toml` 都缺 `[workspace]` 段（RFC-014c）",
    "MemberMissing": "工作区成员的清单文件缺失（RFC-014c）",
    "MemberInvalid": "工作区成员的清单无法解析或不合规（RFC-014c）",
    "NestedWorkspace": "嵌套工作区不被支持：成员自带 `[workspace]` 段（RFC-014c）",
    "PackageTooLarge": "源码包超过 20 MiB 上限（RFC-014a）",
    "ChecksumMismatch": "校验和不符：下载内容与锁文件记录不一致（RFC-014a）",
    "InvalidPackage": "包归档非法：缺清单、路径逃逸、条目类型不合法等（RFC-014a）",
    "Network": "网络请求失败（GitHub 适配器，RFC-014a Phase 4）",
    "RateLimited": "命中 API 速率限制（RFC-014a decision 6）",
    "RegistryDeferred": "官方注册表尚未开放：改用 `publish --github` 或 `--dry-run`（RFC-014a decision 1）",
    "VersionAlreadyExists": "该版本的 Release 已存在（RFC-014a 发布校验）",
    "AuthFailed": "认证失败：凭据缺失、无效或被拒（RFC-014a）",
    "PublishTarget": "发布目标仓库无法解析",
    "TestsFailed": "发布前置测试未通过（RFC-014a 发布校验第 3 步）",
    "Io": "底层 I/O 失败：磁盘空间、权限、文件占用等",
    "Toml": "TOML 序列化 / 反序列化失败（锁文件与清单均适用）",
}


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_md(path: Path, body: str) -> None:
    """UTF-8 无 BOM + LF 换行写盘。"""
    path.parent.mkdir(parents=True, exist_ok=True)
    text = body.replace("\r\n", "\n").replace("\r", "\n")
    if not text.endswith("\n"):
        text += "\n"
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)


def rust_sources() -> list[Path]:
    out = []
    for dirpath, dirnames, filenames in os.walk(SRC_DIR):
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
        for name in sorted(filenames):
            if name.endswith((".rs", ".yx")):
                out.append(Path(dirpath) / name)
    return out


def is_test_path(path: Path) -> bool:
    parts = path.relative_to(SRC_DIR).parts
    if "tests" in parts[:-1]:
        return True
    stem = path.stem
    return stem == "tests" or stem.endswith("_test") or stem.endswith("_tests")


def collect_registrations() -> tuple[list[dict], list[dict]]:
    """解析 `define_codes!` 注册表。返回 (按文件顺序的条目, 解析失败行)。"""
    entries: list[dict] = []
    failures: list[dict] = []
    for rs in sorted(CODES_DIR.glob("*.rs")):
        if rs.name in ("mod.rs", "builder.rs", "tests.rs"):
            continue
        text = read_text(rs)
        for m in DEFINE_RE.finditer(text):
            body = m.group("body")
            statics = m.group("statics")
            body_lines = body.split("\n")
            for idx, line in enumerate(body_lines):
                stripped = line.strip()
                if not stripped or not stripped.startswith('("'):
                    continue
                em = ENTRY_RE.match(stripped)
                if not em:
                    failures.append({"file": rs.name, "statics": statics, "line": stripped})
                    continue
                params_raw = em.group("params").strip()
                params = [p.strip() for p in params_raw.split(",") if p.strip()]
                param_names = [p.split(":", 1)[0].strip() for p in params]
                entries.append(
                    {
                        "code": em.group("code"),
                        "category": em.group("cat"),
                        "span_exempt": em.group("exempt") == "true",
                        "fn": em.group("fn"),
                        "params": param_names,
                        "chain": em.group("chain").strip(),
                        "file": rs.name,
                        "statics": statics,
                        "src_name": _source_name(body_lines, idx),
                    }
                )
    return entries, failures


def _source_name(body_lines: list[str], idx: int) -> str:
    """取条目正上方那段连续注释里第一行以码号开头的文本作为中文名。

    源码注释块里首行即 `// EXXXX 中文名`；后续行是补充说明（RFC 编号、豁免理由等），
    不并入名称。找不到时返回空串。
    """
    j = idx - 1
    while j >= 0:
        cm = COMMENT_RE.match(body_lines[j])
        if not cm or not cm.group("text"):
            break
        pm = CODE_PREFIX_RE.match(cm.group("text"))
        if pm:
            return pm.group("name").strip()
        j -= 1
    return ""


def load_locale_codes() -> tuple[dict, set[str]]:
    """返回 (code -> {title, message, template, help, ...}, 全部含 code 条目的键集合)。"""
    data = json.loads(read_text(ZH_JSON))
    codes: dict[str, dict] = {}
    for key, value in data.items():
        if not isinstance(value, dict) or not CODE_KEY_RE.fullmatch(key):
            continue
        codes[key] = value
    return codes, set(codes)


def scan_emissions(entries: list[dict]) -> tuple[dict[str, list[str]], dict[str, list[str]]]:
    """扫描 src/ 统计每个码的发射点。

    返回 (code -> 发射点列表)。列表元素为 `相对 src 的路径:行号`，测试文件不计。
    """
    fn_to_codes: dict[str, list[str]] = defaultdict(list)
    for e in entries:
        fn_to_codes[e["fn"]].append(e["code"])
    fn_patterns = [
        (re.compile(FN_CALL_RE_TMPL.format(fn=re.escape(fn))), codes)
        for fn, codes in sorted(fn_to_codes.items())
    ]
    hits: dict[str, list[str]] = defaultdict(list)
    for path in rust_sources():
        # 注册表自身（`define_codes!` 条目）不是发射点；测试文件也不计
        if is_test_path(path) or CODES_DIR in path.parents:
            continue
        rel = path.relative_to(ROOT).as_posix()
        try:
            lines = read_text(path).split("\n")
        except UnicodeDecodeError:
            continue
        for lineno, line in enumerate(lines, 1):
            where = f"{rel}:{lineno}"
            for rx, codes in fn_patterns:
                if rx.search(line):
                    for code in codes:
                        hits[code].append(where)
            for m in ERROR_NEW_RE.finditer(line):
                hits[m.group("code")].append(where)
            for m in FIND_CODE_RE.finditer(line):
                hits[m.group("code")].append(where)
    return hits


def scan_str_emissions(entries: list[dict]) -> dict[str, list[str]]:
    """扫描「码号字符串字面量」形式的间接发射点。

    有些诊断不走 `ErrorCodeDefinition::<fn>()` 快捷方法，而是把码号当字符串传下去，
    最后在别处 `ErrorCodeDefinition::find(&code)` 查回来（典型：死代码告警先收集
    `DeadCodeWarning { code }`，再统一转 Diagnostic）。只查函数调用会漏掉这类，
    因此再扫一遍裸的码号字面量。

    只统计真正把码号当值的语句：排除注释行、doc 注释、以及 `Expectation::…(["E…"])`
    这类测试断言（测试文件已整体排除）。
    """
    hits: dict[str, list[str]] = defaultdict(list)
    for path in rust_sources():
        if is_test_path(path) or CODES_DIR in path.parents:
            continue
        rel = path.relative_to(ROOT).as_posix()
        try:
            lines = read_text(path).split("\n")
        except UnicodeDecodeError:
            continue
        for lineno, line in enumerate(lines, 1):
            code_part = line.split("//", 1)[0]
            if not code_part.strip():
                continue
            for m in BARE_CODE_LITERAL_RE.finditer(code_part):
                hits[m.group("code")].append(f"{rel}:{lineno}")
    return hits


def find_std_layer_codes(registered: set[str], locale_codes: dict, str_hits: dict) -> list[dict]:
    """locales 里有、注册表里没有、且在非测试代码里以码号字面量出现的码。

    这些码由标准库原生函数在运行期构造（`error_new("EXXXX", …)` /
    `result.error("EXXXX", …)`），不经过 `define_codes!` 注册表。
    """
    out = []
    for code, info in sorted(locale_codes.items()):
        if code in registered or not str_hits.get(code):
            continue
        out.append({"code": code, "where": str_hits[code], **info})
    return out


def esc(text: str) -> str:
    """转义表格单元里的 `|` 与换行。

    还必须转义 Vue 的插值定界符：VitePress 用 Vue 编译 Markdown，文本里
    出现 `{{` / `}}` 会被当成 JS 表达式解析，构建期直接报
    "[vite:vue] Error parsing JavaScript expression"。真实案例是 E1101 的
    帮助文案 `Iface: (Self: Type) -> Type = {{ ... }}` —— 那是爻象的接口
    声明语法，不是 Vue 模板。转成 HTML 实体后渲染结果完全一致。
    """
    out = (text.replace("|", "\\|")
               .replace("\n", " ")
               .replace("{{", "&#123;&#123;")
               .replace("}}", "&#125;&#125;")
               .strip())
    return out


def inline(text: str) -> str:
    """把模板渲染成行内代码；反引号需转义，Vue 定界符同样要转义。"""
    if not text:
        return "—"
    body = (text.replace("`", "\\`")
                .replace("{{", "&#123;&#123;")
                .replace("}}", "&#125;&#125;"))
    return "`" + body + "`"


def yes_no(flag: bool) -> str:
    return "是" if flag else "否"


def emission_cell(where: list[str]) -> str:
    if not where:
        return "⚠ 暂未发射"
    return f"✅ {len(where)} 处"


def table(headers: list[str], rows: list[list[str]]) -> str:
    out = ["| " + " | ".join(headers) + " |"]
    out.append("| " + " | ".join("---" for _ in headers) + " |")
    for row in rows:
        out.append("| " + " | ".join(row) + " |")
    return "\n".join(out)


GEN_NOTE = (
    # 注意：这里刻意用行内代码而不是 markdown 链接。VitePress 的
    # ignoreDeadLinks: false 会对站内/跨根链接做构建期校验，而
    # scripts/ 在 docs/src/ 之外，任何指向它的相对链接都会让构建失败。
    " 本页由 `scripts/docs/gen-error-code-docs.py` "
    "从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。"
    "改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 "
    "`python scripts/docs/gen-error-code-docs.py`。"
)


def code_title(code: str, info: dict, entry: dict | None) -> str:
    if info.get("title"):
        return info["title"]
    if entry and entry.get("src_name"):
        return entry["src_name"]
    return "（locales 无条目）"


def build_index(entries, locale_codes, emissions, std_codes) -> str:
    reg_e = [e for e in entries if e["code"].startswith("E")]
    reg_w = [e for e in entries if e["code"].startswith("W")]
    total = len(entries) + len(std_codes)

    lines = [
        "---",
        "title: '错误码'",
        "description: 'YaoXiang 编译器与标准库的全部诊断码索引'",
        "---",
        "",
        "# 错误码参考",
        "",
        GEN_NOTE,
        "",
        "YaoXiang 的每个诊断都带一个稳定码号（如 `E1001`），便于检索、"
        "在 issue 中引用、以及在 CI 里按码过滤。诊断的用户可见文案（标题 / 模板 / 帮助）"
        "由 `locales/*.json` 提供，可用 `yx explain <码号>` 在终端查看。",
        "",
        "## 码号体系",
        "",
        "码号由前缀（族）与四位序号组成，族即编译器阶段：",
        "",
        table(
            ["族前缀", "类别", "含义", "注册码数"],
            [
                ["`E0xxx`", "`Lexer` / `Parser`", "词法与语法分析错误", str(_count(entries, "E0"))],
                ["`E1xxx`", "`TypeCheck`", "类型检查错误", str(_count(entries, "E1"))],
                ["`E2xxx`", "`Semantic`", "语义分析错误", str(_count(entries, "E2"))],
                ["`E3xxx`", "`Codegen`", "代码生成错误（含 IR 层一致性检查）", str(_count(entries, "E3"))],
                ["`E4xxx`", "`Generic`", "泛型、特质与常量求值错误", str(_count(entries, "E4"))],
                ["`E5xxx`", "`Module`", "模块与导入错误", str(_count(entries, "E5"))],
                ["`E6xxx`", "`Runtime`", "运行时错误（VM 与 std 原生函数）", str(_count(entries, "E6"))],
                ["`E7xxx`", "`Io`", "I/O 与系统错误", str(_count(entries, "E7"))],
                ["`E8xxx`", "`Internal`", "内部编译器错误（编译器自身的缺陷）", str(_count(entries, "E8"))],
                ["`W1xxx`", "`Warning`", "警告（不阻止编译）", str(_count(entries, "W1"))],
            ],
        ),
        "",
        "`E3xxx` 族此前未在任何文档中列出，现已并入本页；其码在 IR 生成期与入口点检查时产生。",
        "",
        "## 统计",
        "",
        f"- `define_codes!` 注册表：**{len(entries)}** 项（{len(reg_e)} 个 `E` 码 + {len(reg_w)} 个 `W` 码）。",
        f"- std 层运行时码：**{len(std_codes)}** 个（不在 `define_codes!` 注册表内，见下节）。",
        f"- 合计 **{total}** 个诊断码。",
        f"- 其中 **{sum(1 for e in entries if not emissions.get(e['code']))}** 个注册码当前"
        "在非测试代码里没有发射点，标注为「暂未发射」——它们的码号仍然保留，"
        "但用户暂时无法触发。",
        "",
        "## 速查：按码号前缀检索",
        "",
    ]

    prefix_rows = []
    for fam, (cat, title, _desc) in FAMILIES.items():
        page = f"分族页：[`{fam}`](./{fam}.md)"
        prefix_rows.append([f"`{fam}`", f"`{cat}`", title, page])
    prefix_rows.append(["`W1xxx`", "`Warning`", "警告", "[`W1xxx`](../warning-code/warning-codes.md)"])
    lines.append(table(["族前缀", "类别", "含义", "详情"], prefix_rows))
    lines += [
        "",
        "`E3xxx` 族暂无独立分族页，全部码列在下方全量清单中。",
        "",
        "## 全量码清单",
        "",
        "「模板」列取自 `locales/zh.json`，即诊断正文里替换参数后的形态；"
        "「发射点」指该码在非测试代码中的实际调用处数量。",
        "",
    ]

    rows = []
    for e in entries:
        info = locale_codes.get(e["code"], {})
        rows.append(
            [
                f"`{e['code']}`",
                esc(code_title(e["code"], info, e)),
                f"`{e['category']}`",
                yes_no(e["span_exempt"]),
                inline(esc(info.get("template", ""))),
                emission_cell(emissions.get(e["code"], [])),
            ]
        )
    lines.append(
        table(["码号", "中文名", "类别", "span 豁免", "模板", "发射点"], rows)
    )

    lines += [
        "",
        "## std 层运行时码",
        "",
        "下列码**不在 `define_codes!` 注册表内**：它们由标准库原生函数在运行期经 "
        "`error_new(\"码号\", …)` 构造，因此不出现在 `ErrorCodeDefinition::find()` 的查询结果里，"
        "但 `locales/*.json` 有对应文案，用户终端上仍可能看到。",
        "",
    ]
    std_rows = []
    for s in std_codes:
        where = s.get("where", [])
        sample = "、".join(f"`{w}`" for w in where[:2])
        more = f" 等 {len(where)} 处" if len(where) > 2 else ""
        std_rows.append(
            [
                f"`{s['code']}`",
                esc(s.get("title", "")),
                "std 运行时",
                inline(esc(s.get("template", ""))),
                f"{sample}{more}",
            ]
        )
    lines.append(table(["码号", "中文名", "类别", "模板", "发射点"], std_rows))

    lines += [
        "",
        "## 标记说明",
        "",
        "- **span 豁免**：为 `true` 时该诊断允许没有源码位置（内部错误、运行期诊断等）。"
        "非豁免码在既无显式 `.at()` 又无 walk 上下文时会被拒绝"
        "（debug 构建 panic，release 落 `E8001`）。",
        "- **发射点 ⚠ 暂未发射**：该码在注册表中保留，但当前非测试代码里没有调用处，"
        "用户无法触发。码号不回收，避免旧产物里的诊断被复用成别的含义。",
        "- **std 层运行时码**：见上一节，不参与注册表统计。",
        "",
        "## 查看单个码",
        "",
        "```bash",
        "yx explain E1001",
        "yx explain E1001 --json",
        "```",
        "",
        "## 在 Rust 代码中查询",
        "",
        "```rust",
        "use yaoxiang::util::diagnostic::{ErrorCodeDefinition, I18nRegistry};",
        "",
        "let i18n = I18nRegistry::default();",
        "",
        "if let Some(code) = ErrorCodeDefinition::find(\"E1001\") {",
        "    println!(\"Title: {}\", i18n.get_title(&code));",
        "    if let Some(help) = i18n.get_help(&code) {",
        "        println!(\"Help: {}\", help);",
        "    }",
        "}",
        "```",
    ]
    return "\n".join(lines)


def _count(entries: list[dict], prefix: str) -> int:
    return sum(1 for e in entries if e["code"].startswith(prefix))


def build_family(fam: str, cat: str, title: str, desc: str, entries, locale_codes, emissions) -> str:
    # 族归属按码的第二位数字（E0xxx…E8xxx），码的数值顺序即文档顺序
    fam_entries = sorted(
        (e for e in entries if e["code"][0] == "E" and e["code"][1] == fam[1]),
        key=lambda e: e["code"],
    )

    lines = [
        "---",
        f"title: '{fam}：{title}'",
        f"description: '{desc}'",
        "---",
        "",
        f"# {fam}：{title}",
        "",
        GEN_NOTE,
        "",
        desc,
        "",
        f"本族共 **{len(fam_entries)}** 个码，全部在 `define_codes!` 注册表中，"
        f"类别为 `{cat}`。完整索引见[错误码首页](./index.md)。",
        "",
    ]
    if not fam_entries:
        lines.append("::: warning 暂缺\n本族在注册表中没有条目。\n:::\n")
        return "\n".join(lines)

    lines += [
        "## 码一览",
        "",
        table(
            ["码号", "中文名", "类别", "span 豁免", "模板", "发射点"],
            [
                [
                    f"`{e['code']}`",
                    esc(code_title(e["code"], locale_codes.get(e["code"], {}), e)),
                    f"`{e['category']}`",
                    yes_no(e["span_exempt"]),
                    inline(esc(locale_codes.get(e["code"], {}).get("template", ""))),
                    emission_cell(emissions.get(e["code"], [])),
                ]
                for e in fam_entries
            ],
        ),
        "",
        "## 逐码说明",
        "",
    ]
    for e in fam_entries:
        info = locale_codes.get(e["code"], {})
        name = code_title(e["code"], info, e)
        lines.append(f"### {e['code']}：{name}")
        lines.append("")
        lines.append(f"- **类别**：`{e['category']}`")
        lines.append(f"- **span 豁免**：{'是' if e['span_exempt'] else '否'}")
        lines.append(f"- **构造函数**：`ErrorCodeDefinition::{e['fn']}({', '.join(e['params'])})`")
        if info.get("template"):
            lines.append(f"- **模板**：{inline(esc(info['template']))}")
        if info.get("message") and info.get("message") != info.get("template"):
            lines.append(f"- **消息**：{esc(info['message'])}")
        if info.get("help"):
            lines.append(f"- **帮助**：{esc(info['help'])}")
        where = emissions.get(e["code"], [])
        if where:
            sample = "、".join(f"`{w}`" for w in where[:3])
            more = f" 等 {len(where)} 处" if len(where) > 3 else ""
            lines.append(f"- **发射点**：{sample}{more}")
        else:
            lines.append("- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）")
        if e["src_name"] and e["src_name"] != name:
            lines.append(f"- **源码注释名**：{esc(e['src_name'])}")
        lines.append("")

    return "\n".join(lines)


def collect_package_variants() -> list[dict]:
    """解析 `src/package/error.rs` 的 `PackageError` thiserror 变体。

    包管理器错误不带 `E`/`W` 码号（它们是 CLI 层错误，不是编译器诊断），
    但同样需要文档。变体名 / `#[error("…")]` 格式串 / doc 注释均取自源码。
    """
    text = read_text(PKG_ERROR_RS)
    body = text.split("pub enum PackageError {", 1)[1].rsplit("\n}", 1)[0]
    out = []
    for m in PKG_VARIANT_RE.finditer(body):
        doc_lines = [
            ln.strip().lstrip("/").strip()
            for ln in m.group("doc").split("\n")
            if ln.strip()
        ]
        name = m.group("name")
        out.append(
            {
                "name": name,
                "format": json.loads(m.group("fmt")),
                "doc": " ".join(doc_lines),
                "zh": PKG_VARIANT_ZH.get(name, ""),
            }
        )
    # 未登记中文释义的变体视为生成器待补，打印出来便于发现
    missing_zh = [v["name"] for v in out if not v["zh"]]
    if missing_zh:
        print(f"  ! PackageError 变体缺中文释义: {', '.join(missing_zh)}", file=sys.stderr)
    return out


def build_package_errors(variants: list[dict]) -> str:
    lines = [
        "---",
        "title: '包管理器错误'",
        "description: '包管理器错误的类型与处理方式'",
        "---",
        "",
        "# 包管理器错误",
        "",
        "> 本页由 `scripts/docs/gen-error-code-docs.py` "
        "从 `src/package/error.rs` 的 `PackageError` 枚举生成，请勿手工编辑。",
        "改文案请改该文件里的 `#[error(\"…\")]` 格式串，然后重跑 "
        "`python scripts/docs/gen-error-code-docs.py`。",
        "",
        "包管理器（`yx install` / `add` / `rm` / `update` / `publish` / `cache` / `workspace`）的错误",
        "**不带 `E` 码号**：它们是 `thiserror` 定义的 Rust 枚举 `PackageError`，",
        "在 CLI 层以 `Error: <格式串渲染结果>` 的形式打印，不进入编译器的诊断渲染管线。",
        f"当前共 **{len(variants)}** 个变体，定义在 `src/package/error.rs`。",
        "编译器诊断码请见[错误码参考](../error-code/index.md)。",
        "",
        "## 变体一览",
        "",
        table(
            ["变体", "错误文案（`Display`，英文即实际输出）", "含义"],
            [
                [f"`{v['name']}`", inline(esc(v["format"])), esc(v["zh"])]
                for v in variants
            ],
        ),
        "",
        "## 逐变体说明",
        "",
    ]
    for v in variants:
        lines += [
            f"### `{v['name']}`",
            "",
            f"- **含义**：{esc(v['zh'])}",
            f"- **文案**：`{esc(v['format'])}`",
            f"- **源码 doc**：`{esc(v['doc'])}`",
            "",
        ]

    lines += [
        "## 处理方式",
        "",
        "按族归类排查：",
        "",
        table(
            ["症状", "相关变体", "处理方式"],
            [
                [
                    "找不到 `yaoxiang.toml`",
                    "`NotProject` / `NotWorkspace`",
                    "在项目根目录内执行命令；工作区场景确认任一 `yaoxiang.toml` 含 `[workspace]` 段",
                ],
                [
                    "清单 / 锁文件解析失败",
                    "`InvalidManifest` / `Toml`",
                    "校验 `yaoxiang.toml` 与 `yaoxiang.lock` 的 TOML 语法",
                ],
                [
                    "依赖增删异常",
                    "`DependencyNotFound` / `DependencyAlreadyExists` / `DependencyInstallFailed`",
                    "用 `yx list` 查看现有依赖；已存在则先 `yx rm` 再重装",
                ],
                [
                    "工作区成员异常",
                    "`MemberMissing` / `MemberInvalid` / `NestedWorkspace`",
                    "核对 `[workspace] members` 路径；嵌套工作区不被支持，需拍平结构",
                ],
                [
                    "发布 / 网络类",
                    "`Network` / `RateLimited` / `AuthFailed` / `RegistryDeferred` / `VersionAlreadyExists` / `PublishTarget` / `TestsFailed`",
                    "官方注册表尚未开放，发布请显式加 `--github` 或 `--dry-run`；限流待重试；凭据问题先检查 token",
                ],
                [
                    "包内容 / 缓存",
                    "`PackageTooLarge` / `InvalidPackage` / `ChecksumMismatch` / `Cache`",
                    "源码包上限 20 MiB；校验和不符说明下载损坏，清理缓存后重试",
                ],
                [
                    "底层 I/O",
                    "`Io`",
                    "检查磁盘空间、文件权限、文件是否被占用",
                ],
            ],
        ),
        "",
        "## 常见问题",
        "",
        "### Q：安装依赖失败怎么办？",
        "",
        "1. 检查网络连接",
        "2. 确认依赖名称与版本约束正确",
        "3. 尝试 `yx update` 刷新索引",
        "",
        "### Q：vendor 目录损坏怎么办？",
        "",
        "清理缓存后重新安装：`yx cache clean`，再 `yx install`。",
        "",
        "### Q：发布时提示官方注册表未开放？",
        "",
        "这是预期行为（RFC-014a decision 1）。改用 `yx publish --github` 发布到 GitHub Release，"
        "或加 `--dry-run` 先做本地校验。",
    ]
    return "\n".join(lines)


def build_warnings(entries, locale_codes, emissions) -> str:
    w_entries = sorted((e for e in entries if e["code"].startswith("W")), key=lambda e: e["code"])
    lines = [
        "---",
        "title: '警告码'",
        "description: '编译器警告码及说明'",
        "---",
        "",
        "# 警告码",
        "",
        GEN_NOTE,
        "",
        "警告码以 `W` 开头，不会阻止编译，但提示代码中可能存在的问题。"
        f"当前注册表共 **{len(w_entries)}** 个警告码，完整索引见[错误码首页](../error-code/index.md)。",
        "",
        "## 配置",
        "",
        "可以通过 `yaoxiang.toml` 配置死代码警告的行为：",
        "",
        "```toml",
        "[lint]",
        '# 死代码警告级别：off | warn | deny',
        'dead-code = "warn"',
        "```",
        "",
        "- `off`：禁用警告",
        "- `warn`：显示警告（默认）",
        "- `deny`：将警告视为错误，阻止编译",
        "",
        "## 码一览",
        "",
        table(
            ["码号", "中文名", "span 豁免", "模板", "发射点"],
            [
                [
                    f"`{e['code']}`",
                    esc(code_title(e["code"], locale_codes.get(e["code"], {}), e)),
                    yes_no(e["span_exempt"]),
                    inline(esc(locale_codes.get(e["code"], {}).get("template", ""))),
                    emission_cell(emissions.get(e["code"], [])),
                ]
                for e in w_entries
            ],
        ),
        "",
        "## 逐码说明",
        "",
    ]
    for e in w_entries:
        info = locale_codes.get(e["code"], {})
        name = code_title(e["code"], info, e)
        lines.append(f"### {e['code']}：{name}")
        lines.append("")
        lines.append(f"- **类别**：`{e['category']}`")
        lines.append(f"- **span 豁免**：{'是' if e['span_exempt'] else '否'}")
        if info.get("template"):
            lines.append(f"- **模板**：{inline(esc(info['template']))}")
        if info.get("help"):
            lines.append(f"- **帮助**：{esc(info['help'])}")
        where = emissions.get(e["code"], [])
        if where:
            sample = "、".join(f"`{w}`" for w in where[:3])
            more = f" 等 {len(where)} 处" if len(where) > 3 else ""
            lines.append(f"- **发射点**：{sample}{more}")
        else:
            lines.append("- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处）")
        lines.append("")

    lines += [
        "## 警告级别",
        "",
        table(
            ["级别", "效果"],
            [
                ["`off`", "完全禁用此类警告"],
                ["`warn`", "显示警告但继续编译（默认）"],
                ["`deny`", "将警告视为错误，阻止编译"],
            ],
        ),
        "",
        "## 与错误码的区别",
        "",
        "- **错误（`E` 前缀）**：阻止编译，必须修复。",
        "- **警告（`W` 前缀）**：提示潜在问题，可选择修复。",
        "",
        "## 死代码告警的判定口径",
        "",
        "`W1001`–`W1005` 报的是**未使用的私有**（非 `pub`）函数 / 类型 / 变量 / 方法。"
        "`pub` 声明是对外接口，编译器无从得知外部是否有消费者，因此永不触发这些告警"
        "（#321 定案 B）。若看到「未使用」的告警，去掉对应的 `pub` 即可消除；"
        "反过来，需要被外部调用的声明请保留 `pub`。",
    ]
    return "\n".join(lines)


def main() -> int:
    entries, failures = collect_registrations()
    locale_codes, _all = load_locale_codes()
    emissions = scan_emissions(entries)
    str_hits = scan_str_emissions(entries)
    # 合并两类发射点：构造函数直调 + 码号字符串间接发射
    for code, where in str_hits.items():
        emissions[code] = sorted(set(emissions.get(code, [])) | set(where))

    seen = defaultdict(list)
    for e in entries:
        seen[e["code"]].append(e["file"])
    dupes = {c: f for c, f in seen.items() if len(f) > 1}

    registered = set(seen)
    std_codes = find_std_layer_codes(registered, locale_codes, str_hits)

    # ---- 输出 ----
    write_md(DOCS / "error-code" / "index.md", build_index(entries, locale_codes, emissions, std_codes))
    for fam, (cat, title, desc) in FAMILIES.items():
        write_md(
            DOCS / "error-code" / f"{fam}.md",
            build_family(fam, cat, title, desc, entries, locale_codes, emissions),
        )
    write_md(
        DOCS / "warning-code" / "warning-codes.md",
        build_warnings(entries, locale_codes, emissions),
    )
    pkg_variants = collect_package_variants()
    write_md(DOCS / "package" / "error-codes.md", build_package_errors(pkg_variants))

    # ---- 自校验统计 ----
    reg_e = [e for e in entries if e["code"].startswith("E")]
    reg_w = [e for e in entries if e["code"].startswith("W")]
    no_emit = [e for e in entries if not emissions.get(e["code"])]
    missing_locale = [e["code"] for e in entries if e["code"] not in locale_codes]

    print("=== 错误码文档生成统计 ===")
    print(f"define_codes! 注册条目           : {len(entries)}")
    print(f"  其中 E 码 / W 码               : {len(reg_e)} / {len(reg_w)}")
    for fam in FAMILIES:
        n = sum(1 for e in entries if e["code"][0] == "E" and e["code"][1] == fam[1])
        print(f"    {fam:<6}: {n} 个  ({FAMILIES[fam][0]})")
    print(f"    {'W1xxx':<6}: {len(reg_w)} 个  (Warning)")
    print(f"std 层运行时码（error_new 发射）  : {len(std_codes)}  -> {', '.join(s['code'] for s in std_codes)}")
    print(f"诊断码合计                      : {len(entries) + len(std_codes)}")
    print(f"有发射点的注册码                : {len(entries) - len(no_emit)}")
    print(f"暂未发射的注册码                : {len(no_emit)}")
    if no_emit:
        print(f"  -> {', '.join(e['code'] for e in no_emit)}")
    print(f"locales/zh.json 无条目的注册码  : {len(missing_locale)}"
          + (f" -> {', '.join(missing_locale)}" if missing_locale else ""))
    print(f"locales 有条目但未注册的码（全部）: {', '.join(sorted(set(locale_codes) - registered))}")
    print(f"PackageError 变体（无码号）       : {len(pkg_variants)}")
    print(f"重复注册的码                    : {dupes if dupes else '无'}")
    print(f"注册表解析失败行                : {len(failures)}")
    for f in failures:
        print(f"  ! {f['file']} {f['statics']}: {f['line'][:90]}")
    if failures:
        print("!! 存在解析失败行，注册表可能已变更，请更新本生成器。", file=sys.stderr)
        return 1

    print("输出文件:")
    for rel in [
        "docs/src/reference/error-code/index.md",
        *[f"docs/src/reference/error-code/{f}.md" for f in FAMILIES],
        "docs/src/reference/warning-code/warning-codes.md",
        "docs/src/reference/package/error-codes.md",
    ]:
        print(f"  {rel}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
