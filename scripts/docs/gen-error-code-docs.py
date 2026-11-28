#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""错误码文档生成器（中英双语）。

数据源（唯一权威）：
  1. `src/util/diagnostic/codes/*.rs` —— `define_codes!` 注册表单源宏（code / category /
     span_exempt / 构造函数名 / 参数名 / 条目上方中文注释）。
  2. `locales/<lang>.json` —— 用户可见文案的唯一权威（title / template / message / help）。
  3. `scripts/docs/error-code-strings.json` —— 页面散文（标题 / 说明 / 表头 / 章节标题 /
     提示语）的唯一权威，分 `zh` / `en` 两套；需要插值的地方用 `{name}` 占位。
  4. `src/**` —— 扫描每个码是否真有发射点（`ErrorCodeDefinition::<fn>(` 调用，
     以及 std 层 `error_new("EXXXX", …)` 字面量）。

命令行：

    python scripts/docs/gen-error-code-docs.py [--lang {zh,en}] [--out-root DIR]

  --lang zh（默认）：locale 取 `locales/zh.json`，散文取串表的 `zh` 段，
                     输出根默认 `docs/src`。
  --lang en         ：locale 取 `locales/en.json`，散文取串表的 `en` 段，
                     输出根默认 `docs/src/en`。
  --out-root DIR    ：覆盖输出根。产物路径恒为 `<DIR>/reference/…`。

输出（全部 UTF-8 无 BOM + LF）：

  <root>/reference/error-code/index.md              全量清单页
  <root>/reference/error-code/E0xxx.md … E8xxx.md  分族页
  <root>/reference/warning-code/warning-codes.md    警告码页
  <root>/reference/package/error-codes.md          包管理器错误页

本文件重跑幂等：同样输入必得同样输出。改文案请改 `locales/*.json`，改码表请改
`define_codes!` 注册表，改页面散文请改 `scripts/docs/error-code-strings.json`，然后重跑：

    python scripts/docs/gen-error-code-docs.py --lang zh
    python scripts/docs/gen-error-code-docs.py --lang en
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import OrderedDict, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CODES_DIR = ROOT / "src" / "util" / "diagnostic" / "codes"
SRC_DIR = ROOT / "src"
LOCALE_DIR = ROOT / "locales"
STRINGS_JSON = ROOT / "scripts" / "docs" / "error-code-strings.json"

# 每种语言的数据源与默认输出根（`--lang` / `--out-root` 可覆盖）
LANGS = {
    "zh": {"locale": "zh.json", "out_root": ROOT / "docs" / "src"},
    "en": {"locale": "en.json", "out_root": ROOT / "docs" / "src" / "en"},
}

# 当前语言与其散文串表（main() 里按 --lang 填好）
LANG = "zh"
STRINGS: dict = {}

# 散文串表里的占位符形如 `{name}`
PLACEHOLDER_RE = re.compile(r"\{(\w+)\}")

# 分族页元数据：文件前缀 -> (category, 串表里的标题 key, 串表里的说明 key)
FAMILIES = OrderedDict(
    [
        ("E0xxx", ("Lexer / Parser", "family.E0xxx.title", "family.E0xxx.desc")),
        ("E1xxx", ("TypeCheck", "family.E1xxx.title", "family.E1xxx.desc")),
        ("E2xxx", ("Semantic", "family.E2xxx.title", "family.E2xxx.desc")),
        ("E4xxx", ("Generic", "family.E4xxx.title", "family.E4xxx.desc")),
        ("E5xxx", ("Module", "family.E5xxx.title", "family.E5xxx.desc")),
        ("E6xxx", ("Runtime", "family.E6xxx.title", "family.E6xxx.desc")),
        ("E7xxx", ("Io", "family.E7xxx.title", "family.E7xxx.desc")),
        ("E8xxx", ("Internal", "family.E8xxx.title", "family.E8xxx.desc")),
    ]
)

# 码号体系表：族前缀 -> (类别, 注册码计数前缀)。「含义」列取自串表。
SYSTEM_ROWS = [
    ("E0xxx", "`Lexer` / `Parser`", "E0"),
    ("E1xxx", "`TypeCheck`", "E1"),
    ("E2xxx", "`Semantic`", "E2"),
    ("E3xxx", "`Codegen`", "E3"),
    ("E4xxx", "`Generic`", "E4"),
    ("E5xxx", "`Module`", "E5"),
    ("E6xxx", "`Runtime`", "E6"),
    ("E7xxx", "`Io`", "E7"),
    ("E8xxx", "`Internal`", "E8"),
    ("W1xxx", "`Warning`", "W1"),
]

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

# 词边界（#515）：`\b` 使 `type_mismatch` 不再子串命中 `return_type_mismatch(`——
# `_` 与字母交界无词边界；独立调用与 `::` 限定的边界不受影响
FN_CALL_RE_TMPL = r"(?:ErrorCodeDefinition\s*::\s*)?\b{fn}\s*\("
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


def t(key: str, **kw) -> str:
    """取当前语言的散文文案，并替换 `{name}` 占位符。

    占位符在串表里写成 `{name}`；缺 key 或缺参数一律报错（不静默产出半成品）。
    """
    try:
        text = STRINGS[LANG][key]
    except KeyError as exc:
        raise KeyError(f"散文串表缺 key: {LANG}.{key}") from exc
    missing = [m.group(1) for m in PLACEHOLDER_RE.finditer(text) if m.group(1) not in kw]
    if missing:
        raise KeyError(f"散文串表 {LANG}.{key} 缺占位符参数: {', '.join(missing)}")
    return PLACEHOLDER_RE.sub(lambda m: str(kw[m.group(1)]), text)


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


def load_locale_codes(locale_json: Path) -> tuple[dict, set[str]]:
    """返回 (code -> {title, message, template, help, ...}, 全部含 code 条目的键集合)。"""
    data = json.loads(read_text(locale_json))
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
    return t("yes") if flag else t("no")


def emission_cell(where: list[str]) -> str:
    if not where:
        return t("emit.none")
    if len(where) == 1:
        # 英文的复数：单数用独立键，避免出现「✅ 1 sites」。
        return t("emit.count_one", count=1)
    return t("emit.count", count=len(where))


def table(headers: list[str], rows: list[list[str]]) -> str:
    out = ["| " + " | ".join(headers) + " |"]
    out.append("| " + " | ".join("---" for _ in headers) + " |")
    for row in rows:
        out.append("| " + " | ".join(row) + " |")
    return "\n".join(out)


def code_title(code: str, info: dict, entry: dict | None) -> str:
    if info.get("title"):
        return info["title"]
    if entry and entry.get("src_name"):
        return entry["src_name"]
    return t("title.missing")


def build_index(entries, locale_codes, emissions, std_codes) -> str:
    reg_e = [e for e in entries if e["code"].startswith("E")]
    reg_w = [e for e in entries if e["code"].startswith("W")]
    total = len(entries) + len(std_codes)

    lines = [
        "---",
        f"title: '{t('index.frontmatter.title')}'",
        f"description: '{t('index.frontmatter.description')}'",
        "---",
        "",
        f"# {t('index.h1')}",
        "",
        t("gen.note"),
        "",
        t("index.intro"),
        "",
        t("index.h.system"),
        "",
        t("index.system.lead"),
        "",
        table(
            [
                t("index.system.hdr.family"),
                t("index.system.hdr.category"),
                t("index.system.hdr.meaning"),
                t("index.system.hdr.count"),
            ],
            [
                [
                    f"`{fam}`",
                    category,
                    t(f"index.system.meaning.{fam}"),
                    str(_count(entries, prefix)),
                ]
                for fam, category, prefix in SYSTEM_ROWS
            ],
        ),
        "",
        t("index.system.e3note"),
        "",
        t("index.h.stats"),
        "",
        t("index.stats.registry", count=len(entries), e=len(reg_e), w=len(reg_w)),
        t("index.stats.std", count=len(std_codes)),
        t("index.stats.total", count=total),
        t("index.stats.no_emit", count=sum(1 for e in entries if not emissions.get(e["code"]))),
        "",
        t("index.h.quick"),
        "",
    ]

    prefix_rows = []
    for fam, (cat, _title_key, _desc_key) in FAMILIES.items():
        page = t("index.quick.details.family", fam=fam)
        prefix_rows.append([f"`{fam}`", f"`{cat}`", t(f"family.{fam}.title"), page])
    prefix_rows.append([
        "`W1xxx`",
        "`Warning`",
        t("index.quick.meaning.W1xxx"),
        t("index.quick.details.W1xxx"),
    ])
    lines.append(
        table(
            [
                t("index.quick.hdr.family"),
                t("index.quick.hdr.category"),
                t("index.quick.hdr.meaning"),
                t("index.quick.hdr.details"),
            ],
            prefix_rows,
        )
    )

    lines += [
        "",
        t("index.quick.e3note"),
        "",
        t("index.h.all"),
        "",
        t("index.all.lead"),
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
        table(
            [
                t("index.all.hdr.code"),
                t("index.all.hdr.name"),
                t("index.all.hdr.category"),
                t("index.all.hdr.span"),
                t("index.all.hdr.template"),
                t("index.all.hdr.emit"),
            ],
            rows,
        )
    )

    lines += [
        "",
        t("index.h.std"),
        "",
        t("index.std.lead"),
        "",
    ]
    std_rows = []
    for s in std_codes:
        where = s.get("where", [])
        sample = t("emit.sep").join(f"`{w}`" for w in where[:2])
        more = t("emit.more", count=len(where)) if len(where) > 2 else ""
        std_rows.append(
            [
                f"`{s['code']}`",
                esc(s.get("title", "")),
                t("index.std.category"),
                inline(esc(s.get("template", ""))),
                f"{sample}{more}",
            ]
        )
    lines.append(
        table(
            [
                t("index.std.hdr.code"),
                t("index.std.hdr.name"),
                t("index.std.hdr.category"),
                t("index.std.hdr.template"),
                t("index.std.hdr.emit"),
            ],
            std_rows,
        )
    )

    lines += [
        "",
        t("index.h.legend"),
        "",
        t("index.legend.span"),
        t("index.legend.no_emit"),
        t("index.legend.std"),
        "",
        t("index.h.explain"),
        "",
        "```bash",
        "yx explain E1001",
        "yx explain E1001 --json",
        "```",
        "",
        t("index.h.rust"),
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
        f"title: '{t('family.frontmatter.title', fam=fam, title=title)}'",
        f"description: '{desc}'",
        "---",
        "",
        f"# {t('family.h1', fam=fam, title=title)}",
        "",
        t("gen.note"),
        "",
        desc,
        "",
        t("family.summary", count=len(fam_entries), cat=cat),
        "",
    ]
    if not fam_entries:
        lines.append(t("family.empty"))
        return "\n".join(lines)

    lines += [
        t("family.h.list"),
        "",
        table(
            [
                t("family.list.hdr.code"),
                t("family.list.hdr.name"),
                t("family.list.hdr.category"),
                t("family.list.hdr.span"),
                t("family.list.hdr.template"),
                t("family.list.hdr.emit"),
            ],
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
        t("family.h.detail"),
        "",
    ]
    for e in fam_entries:
        info = locale_codes.get(e["code"], {})
        name = code_title(e["code"], info, e)
        lines.append(t("family.detail.h3", code=e["code"], name=name))
        lines.append("")
        lines.append(t("family.detail.category", cat=e["category"]))
        lines.append(t("family.detail.span", value=yes_no(e["span_exempt"])))
        lines.append(t("family.detail.constructor", fn=e["fn"], params=", ".join(e["params"])))
        if info.get("template"):
            lines.append(t("family.detail.template", value=inline(esc(info["template"]))))
        if info.get("message") and info.get("message") != info.get("template"):
            lines.append(t("family.detail.message", value=esc(info["message"])))
        if info.get("help"):
            lines.append(t("family.detail.help", value=esc(info["help"])))
        where = emissions.get(e["code"], [])
        if where:
            sample = t("emit.sep").join(f"`{w}`" for w in where[:3])
            more = t("emit.more", count=len(where)) if len(where) > 3 else ""
            lines.append(t("family.detail.emit", sample=sample, more=more))
        else:
            lines.append(t("family.detail.no_emit"))
        if e["src_name"] and e["src_name"] != name:
            lines.append(t("family.detail.src_name", value=esc(e["src_name"])))
        lines.append("")

    return "\n".join(lines)


def collect_package_variants() -> list[dict]:
    """解析 `src/package/error.rs` 的 `PackageError` thiserror 变体。

    包管理器错误不带 `E`/`W` 码号（它们是 CLI 层错误，不是编译器诊断），
    但同样需要文档。变体名 / `#[error("…")]` 格式串 / doc 注释均取自源码；
    变体的含义说明取自散文串表（`pkg.<变体名>`）。
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
                "meaning": t(f"pkg.{name}"),
            }
        )
    # 未登记释义的变体视为生成器待补，打印出来便于发现
    missing = [v["name"] for v in out if not v["meaning"]]
    if missing:
        print(t("stats.pkg_missing", names=", ".join(missing)), file=sys.stderr)
    return out


def build_package_errors(variants: list[dict]) -> str:
    lines = [
        "---",
        f"title: '{t('pkg.frontmatter.title')}'",
        f"description: '{t('pkg.frontmatter.description')}'",
        "---",
        "",
        f"# {t('pkg.h1')}",
        "",
        t("pkg.note.quote"),
        t("pkg.note.cont"),
        "",
        t("pkg.intro.1"),
        t("pkg.intro.2"),
        t("pkg.intro.3"),
        t("pkg.intro.4", count=len(variants)),
        t("pkg.intro.5"),
        "",
        t("pkg.h.list"),
        "",
        table(
            [
                t("pkg.list.hdr.variant"),
                t("pkg.list.hdr.text"),
                t("pkg.list.hdr.meaning"),
            ],
            [
                [f"`{v['name']}`", inline(esc(v["format"])), esc(v["meaning"])]
                for v in variants
            ],
        ),
        "",
        t("pkg.h.detail"),
        "",
    ]
    for v in variants:
        lines += [
            t("pkg.detail.h3", name=v["name"]),
            "",
            t("pkg.detail.meaning", value=esc(v["meaning"])),
            t("pkg.detail.text", value=esc(v["format"])),
            t("pkg.detail.doc", value=esc(v["doc"])),
            "",
        ]

    lines += [
        t("pkg.h.fix"),
        "",
        t("pkg.fix.lead"),
        "",
        table(
            [
                t("pkg.fix.hdr.symptom"),
                t("pkg.fix.hdr.variants"),
                t("pkg.fix.hdr.action"),
            ],
            [
                [
                    t(f"pkg.fix.r{i}.symptom"),
                    t(f"pkg.fix.r{i}.variants"),
                    t(f"pkg.fix.r{i}.action"),
                ]
                for i in range(1, 8)
            ],
        ),
        "",
        t("pkg.h.faq"),
        "",
        t("pkg.faq.q1"),
        "",
        t("pkg.faq.a1.1"),
        t("pkg.faq.a1.2"),
        t("pkg.faq.a1.3"),
        "",
        t("pkg.faq.q2"),
        "",
        t("pkg.faq.a2"),
        "",
        t("pkg.faq.q3"),
        "",
        t("pkg.faq.a3"),
    ]
    return "\n".join(lines)


def build_warnings(entries, locale_codes, emissions) -> str:
    w_entries = sorted((e for e in entries if e["code"].startswith("W")), key=lambda e: e["code"])
    lines = [
        "---",
        f"title: '{t('warn.frontmatter.title')}'",
        f"description: '{t('warn.frontmatter.description')}'",
        "---",
        "",
        f"# {t('warn.h1')}",
        "",
        t("gen.note"),
        "",
        t("warn.intro", count=len(w_entries)),
        "",
        t("warn.h.config"),
        "",
        t("warn.config.lead"),
        "",
        "```toml",
        "[lint]",
        t("warn.config.toml_comment"),
        'dead-code = "warn"',
        "```",
        "",
        t("warn.config.off"),
        t("warn.config.warn"),
        t("warn.config.deny"),
        "",
        t("warn.h.list"),
        "",
        table(
            [
                t("warn.list.hdr.code"),
                t("warn.list.hdr.name"),
                t("warn.list.hdr.span"),
                t("warn.list.hdr.template"),
                t("warn.list.hdr.emit"),
            ],
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
        t("warn.h.detail"),
        "",
    ]
    for e in w_entries:
        info = locale_codes.get(e["code"], {})
        name = code_title(e["code"], info, e)
        lines.append(t("warn.detail.h3", code=e["code"], name=name))
        lines.append("")
        lines.append(t("warn.detail.category", cat=e["category"]))
        lines.append(t("warn.detail.span", value=yes_no(e["span_exempt"])))
        if info.get("template"):
            lines.append(t("warn.detail.template", value=inline(esc(info["template"]))))
        if info.get("help"):
            lines.append(t("warn.detail.help", value=esc(info["help"])))
        where = emissions.get(e["code"], [])
        if where:
            sample = t("emit.sep").join(f"`{w}`" for w in where[:3])
            more = t("emit.more", count=len(where)) if len(where) > 3 else ""
            lines.append(t("warn.detail.emit", sample=sample, more=more))
        else:
            lines.append(t("warn.detail.no_emit"))
        lines.append("")

    lines += [
        t("warn.h.levels"),
        "",
        table(
            [
                t("warn.levels.hdr.level"),
                t("warn.levels.hdr.effect"),
            ],
            [
                ["`off`", t("warn.levels.off")],
                ["`warn`", t("warn.levels.warn")],
                ["`deny`", t("warn.levels.deny")],
            ],
        ),
        "",
        t("warn.h.vserror"),
        "",
        t("warn.vserror.error"),
        t("warn.vserror.warning"),
        "",
        t("warn.h.deadcode"),
        "",
        t("warn.deadcode"),
    ]
    return "\n".join(lines)


def _display_path(path: Path) -> str:
    """输出文件清单用仓库内相对路径；根在仓库外时退回绝对路径。"""
    resolved = path.resolve()
    try:
        return resolved.relative_to(ROOT).as_posix()
    except ValueError:
        return resolved.as_posix()


def main() -> int:
    global LANG, STRINGS

    parser = argparse.ArgumentParser(
        description="生成错误码 / 警告码 / 包管理器错误的中英双语参考页。",
    )
    parser.add_argument(
        "--lang",
        choices=sorted(LANGS),
        default="zh",
        help="页面语言：zh 取 locales/zh.json 与串表 zh 段，en 取 locales/en.json 与串表 en 段。",
    )
    parser.add_argument(
        "--out-root",
        default=None,
        help="输出根目录；产物路径恒为 <DIR>/reference/…。默认按语言取 docs/src 与 docs/src/en。",
    )
    args = parser.parse_args()

    LANG = args.lang
    STRINGS = json.loads(read_text(STRINGS_JSON))
    locale_json = LOCALE_DIR / LANGS[LANG]["locale"]
    out_root = Path(args.out_root).resolve() if args.out_root else LANGS[LANG]["out_root"]
    docs_root = out_root / "reference"

    entries, failures = collect_registrations()
    locale_codes, _all = load_locale_codes(locale_json)
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
    written: list[Path] = []
    index_path = docs_root / "error-code" / "index.md"
    write_md(index_path, build_index(entries, locale_codes, emissions, std_codes))
    written.append(index_path)
    for fam, (cat, _title_key, _desc_key) in FAMILIES.items():
        fam_path = docs_root / "error-code" / f"{fam}.md"
        write_md(
            fam_path,
            build_family(
                fam,
                cat,
                t(f"family.{fam}.title"),
                t(f"family.{fam}.desc"),
                entries,
                locale_codes,
                emissions,
            ),
        )
        written.append(fam_path)
    warn_path = docs_root / "warning-code" / "warning-codes.md"
    write_md(warn_path, build_warnings(entries, locale_codes, emissions))
    written.append(warn_path)
    pkg_variants = collect_package_variants()
    pkg_path = docs_root / "package" / "error-codes.md"
    write_md(pkg_path, build_package_errors(pkg_variants))
    written.append(pkg_path)

    # ---- 自校验统计 ----
    reg_e = [e for e in entries if e["code"].startswith("E")]
    reg_w = [e for e in entries if e["code"].startswith("W")]
    no_emit = [e for e in entries if not emissions.get(e["code"])]
    missing_locale = [e["code"] for e in entries if e["code"] not in locale_codes]

    print(t("stats.title"))
    print(t("stats.entries", count=len(entries)))
    print(t("stats.e_w", e=len(reg_e), w=len(reg_w)))
    for fam in FAMILIES:
        n = sum(1 for e in entries if e["code"][0] == "E" and e["code"][1] == fam[1])
        print(t("stats.family", fam=fam.ljust(6), count=n, cat=FAMILIES[fam][0]))
    print(t("stats.family", fam="W1xxx".ljust(6), count=len(reg_w), cat="Warning"))
    print(t("stats.std", count=len(std_codes), codes=", ".join(s["code"] for s in std_codes)))
    print(t("stats.total", count=len(entries) + len(std_codes)))
    print(t("stats.emitted", count=len(entries) - len(no_emit)))
    print(t("stats.no_emit", count=len(no_emit)))
    if no_emit:
        print(t("stats.no_emit_list", codes=", ".join(e["code"] for e in no_emit)))
    print(t("stats.missing_locale", count=len(missing_locale))
          + (f" -> {', '.join(missing_locale)}" if missing_locale else ""))
    print(t("stats.unregistered", codes=", ".join(sorted(set(locale_codes) - registered))))
    print(t("stats.pkg", count=len(pkg_variants)))
    print(t("stats.dupes", value=dupes if dupes else t("stats.none")))
    print(t("stats.failures", count=len(failures)))
    for f in failures:
        print(t("stats.failure_line", file=f["file"], statics=f["statics"], line=f["line"][:90]))
    if failures:
        print(t("stats.fatal"), file=sys.stderr)
        return 1

    print(t("stats.outputs"))
    for path in written:
        print(f"  {_display_path(path)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
