#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""test-gate-census: 测试三扇门普查（coding-rules 第七部分）

背景
----
WBS 0.6 的「大清理」前置：把全仓 #[test] 按三扇门（条款 / 病史 / 证明）机械分类，
产出可重跑的逐条判定表。本脚本只读不写测试——不改、不删任何测试代码。

三扇门的机器判据（与 coding-rules 第七部分对齐）
-----------------------------------------------
* 条款门（0.6-10 收紧）：测试文件的头部（前 25 行；src/**/tests*/ 下的文件连父
  mod.rs 一起看）出现**可核条目**——节号形态 §<数字>（含 RFC-<数字> §<数字>、
  SPEC §<数字>）。只写「见 RFC-011」而没有任何节号的不算：0.6-10 的 40 条分层抽样
  实测「引用了但撑不住断言」占 50%——这条实测是**放弃「凭一句引用就留」的直接原因**。
  （该抽样审计已于 0.6-19 废止并删除，结论保留、审计产物不再维护。）
* 病史门：测试函数体、文档注释或**文件头**（//! 回归背景）点名真实故障
  （issue #N、回归、缺陷、事故）——该测试钉的是一次已发生的事故。文件头半径
  的理由：项目惯例是把「回归背景：issue #N」写在文件头，逐条测试注释里不重复。
* 证明门：机器判不了（要判断「换一个正确的替代实现，它会不会红」）。故凡前两门都
  不过的，一律落「待判」，不自动判成「均不通过」；唯一例外是「体内一条断言都没有」
  （连 should_panic 也没有）——那不是判据薄弱，是根本没断言。

处置（留 / 退役候选 / 待判）
---------------------------
* 永不判退役：语料（tests/yaoxiang/**）、tests/integration/**、
  tests/formatter_integration.rs、tests/yx_runner*.rs（spec-shaped，走进程边界）、
  以及测试名含 unchanged/anchor/guard/preserv/baseline 的现状锚 —— 一律「留」。
* 病史门过 → 「留」。
* 条款门（有可核条目）→ 一律「待核」——**有引用不等于引用撑得住断言**，
  收紧后不再凭文件头一句话给「留」。
  （0.6-10 的 40 条抽样审计已于 0.6-19 废止并删除：审计池随测试退役缩小后，原样本
  38 条已不存在，--check 恒报「缺 38 条、多 38 条」；审计脚本与数据文件一并删除，
  条款引用不再有「审计确认」这一档。）
* 两门皆不过 + 断言只有弱断言（is_ok/is_some/is_err/is_none + unwrap/expect）
  → 「退役候选」（候选，不是判决；证明门仍需人工复核）。
* 两门皆不过 + 有 assert_eq!/assert_ne!/matches!/带消息的 assert → 「待判」。

用法
----
    python scripts/dev/test-gate-census.py                # 打印摘要 + 写 TSV
    python scripts/dev/test-gate-census.py --check        # 只比较已入库 TSV 是否最新
    python scripts/dev/test-gate-census.py --selftest     # 正/反例自检（不扫仓库）
    python scripts/dev/test-gate-census.py --stdout       # 只输出 TSV 到 stdout

退出码：0 = 通过；1 = --check 与已入库 TSV 不一致；3 = 环境错误（扫描根缺失）。
"""
import argparse
import io
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TSV_PATH = os.path.join(REPO_ROOT, "docs", "src", "dev", "test-gate-census.tsv")
SCAN_ROOTS = ("src", "tests", "benches", "tools")

# 条款门（0.6-10 收紧）：只看**可核条目**——节号形态 §<数字>。
# 0.6-9 的旧判据（出现 RFC-011 / language-spec / 语言规范 等字样即算）已废弃：
# 40 条分层抽样实测错引率 50%，见 docs/src/dev/test-gate-census.md。
CLAUSE_RE = re.compile(r"§\s*\d")
# 只引整篇 RFC/规范、没有任何节号（收紧后不算条款门，但也不能当「没引用」处理——
# 这类同样要人工核，落「待核」而不是「退役候选」）。
LOOSE_REF_RE = re.compile(r"(RFC[- ]?\d{3}|SPEC[- ]?[A-Z0-9]|language-spec|语言规范)")
# 病史门：点名真实故障
HISTORY_RE = re.compile(r"(issue\s*#\d+|回归|缺陷|事故)")
# 永不判退役的路径（spec-shaped，走进程边界）
NO_RETIRE_PATHS = (
    "tests/yaoxiang/",
    "tests/integration/",
    "tests/formatter_integration.rs",
    "tests/yx_runner.rs",
    "tests/yx_multifile_runner.rs",
)
NO_RETIRE_NAME_RE = re.compile(r"(unchanged|anchor|guard|preserv|baseline)", re.IGNORECASE)
# 弱断言：只证明「没 panic / 有值」，没锁定具体输出
WEAK_RE = re.compile(r"\b(is_ok|is_some|is_none|is_err|unwrap|expect)\b")
# 强断言：锁定具体值/结构/消息
STRONG_RE = re.compile(r"\b(assert_eq|assert_ne|matches)\s*!")
ASSERT_RE = re.compile(r"\b(assert|assert_eq|assert_ne|debug_assert|matches)\s*!")
# 委托断言：调用名为 assert*/check* 的助手函数（如 assert_neq_truth_table(&c)）——
# 断言在助手体内，词法上看不到 assert! 宏，但判据真实存在（实测假阳性来源）。
# 0.6-15：名字部分放开数字——[A-Za-z_]* 匹配不到 assert_slot_is_42 这类含数字的助手，
# 会把「体内有断言」的测试误判成「体内无任何断言」（实测 15 条「均不通过」里 3 条是这种）。
HELPER_RE = re.compile(r"\b(assert|check)[A-Za-z0-9_]*\s*\(")

TEST_ATTR_RE = re.compile(r"^\s*#\[test\]\s*$")
FN_RE = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
SHOULD_PANIC_RE = re.compile(r"#\[should_panic")

# Rust 字面量与注释的词法形态（0.6-13：body_span 取函数体前要先跳过它们——
# 字符串里的花括号会把配平提前拉平，函数体被截断，分箱随之出错）。
RAW_STRING_RE = re.compile(r"(b|c)?r(#*)\"")   # r"..." / r#"..."# / br"..." / cr#"..."#
PLAIN_STRING_RE = re.compile(r"(b|c)?\"")       # "..." / b"..." / c"..."
CHAR_RE = re.compile(r"(b|c)?'")                 # 'x' / '\n' / b'x'（'a 是生命周期，不是字面量）


class EnvError(RuntimeError):
    pass


def force_utf8_stdio() -> None:
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            try:
                stream.reconfigure(encoding="utf-8", errors="replace")
            except Exception:  # pragma: no cover - 平台差异
                pass


def read_text(path: str) -> str:
    with io.open(path, "r", encoding="utf-8", errors="replace") as fh:
        return fh.read()


def iter_rust_files():
    out = []
    for root in SCAN_ROOTS:
        base = os.path.join(REPO_ROOT, root)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = sorted(d for d in dirnames if d not in ("target", "node_modules", ".git"))
            for name in sorted(filenames):
                if name.endswith(".rs"):
                    out.append(os.path.join(dirpath, name))
    return sorted(out)


def rel(path: str) -> str:
    return os.path.relpath(path, REPO_ROOT).replace(os.sep, "/")


def header_text(path: str, rel_path: str) -> str:
    """文件头 25 行；src/**/tests*/ 下的文件连父 mod.rs 一起看。"""
    lines = read_text(path).splitlines()[:25]
    parts = ["\n".join(lines)]
    if rel_path.startswith("src/") and "/tests/" in rel_path:
        parent = os.path.join(os.path.dirname(path), "mod.rs")
        if os.path.isfile(parent):
            parts.append("\n".join(read_text(parent).splitlines()[:25]))
    return "\n".join(parts)


def _is_ident_char(ch: str) -> bool:
    return ch.isalnum() or ch == "_"


def mask_noncode(text: str) -> str:
    """把字符串字面量 / 字符字面量 / 行注释 / 块注释 / 原始字符串的内容换成空格。

    Rust 源码里花括号可以出现在这些非代码区域（典型如 `tokenize("{")`）。
    按字面量配平花括号会把函数体提前截断——实测 `test_prefix_info_lbrace`
    的函数体被截成两行，断言摘要取到文件余下部分，分箱因此出错（0.6-13）。

    返回值与入参等长：非代码区域的内容换成空格，换行原样保留，故行数与
    行号不变，调用方仍按行号定位。注释与字面量**内容**判据不受影响——
    掩码只用于找函数体边界，断言统计仍读原文。
    """
    out = list(text)
    n = len(text)

    def blank(start: int, end: int) -> None:
        for k in range(start, min(end, n)):
            if out[k] != "\n":
                out[k] = " "

    i = 0
    while i < n:
        ch = text[i]
        # 行注释
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
            continue
        # 块注释（Rust 允许嵌套）
        if text.startswith("/*", i):
            depth = 0
            j = i
            while j < n:
                if text.startswith("/*", j):
                    depth += 1
                    blank(j, j + 2)
                    j += 2
                    continue
                if text.startswith("*/", j):
                    depth -= 1
                    blank(j, j + 2)
                    j += 2
                    if depth == 0:
                        break
                    continue
                blank(j, j + 1)
                j += 1
            i = j
            continue
        # 原始字符串：r"..." / r#"..."# / br"..." / cr#"..."#
        m = RAW_STRING_RE.match(text, i)
        if m and (m.group(1) or i == 0 or not _is_ident_char(text[i - 1])):
            close = '"' + m.group(2)
            j = text.find(close, m.end())
            end = n if j < 0 else j + len(close)
            blank(i, end)
            i = end
            continue
        # 普通字符串："..." / b"..." / c"..."
        m = PLAIN_STRING_RE.match(text, i)
        if m:
            j = m.end()
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            blank(i, j)
            i = j
            continue
        # 字符字面量：'x' / '\n' / b'x'；'a / '_ 是生命周期，不是字面量
        m = CHAR_RE.match(text, i)
        if m:
            j = m.end()
            if j < n and text[j] == "\\":
                k = j + 1
                if text.startswith("u{", k):
                    close = text.find("}", k)
                    k = n if close < 0 else close + 1
                else:
                    k += 1
                j = k
                while j < n and text[j] != "'":
                    j += 1
                j = min(j + 1, n)
                blank(i, j)
                i = j
                continue
            nxt = text[j] if j < n else ""
            nxt2 = text[j + 1] if j + 1 < n else ""
            if nxt2 == "'":
                # 'x' —— 单字符字面量
                blank(i, j + 2)
                i = j + 2
                continue
            if nxt.isalpha() or nxt == "_":
                # 'a / '_ / 'static —— 生命周期，不是字符字面量
                blank(i, i + 1)
                i += 1
                continue
            # 其余按字符字面量扫到收尾引号（如 '{'、')'、' '）
            k = text.find("'", j)
            end = n if k < 0 else k + 1
            blank(i, end)
            i = end
            continue
        i += 1
    return "".join(out)


def body_span(masked_lines, start):
    """从 fn name( 所在行起，按花括号配平找到函数体结束行（含）。

    入参必须是 mask_noncode 处理过的行——否则字符串字面量里的花括号
    （如 `tokenize("{")`）会把配平提前拉平，函数体被截断（0.6-13 修的就是这个）。
    """
    depth = 0
    opened = False
    for idx in range(start, len(masked_lines)):
        for ch in masked_lines[idx]:
            if ch == "{":
                depth += 1
                opened = True
            elif ch == "}":
                depth -= 1
                if opened and depth == 0:
                    return start, idx
    return start, len(masked_lines) - 1


def doc_block(lines, fn_line):
    """函数上方紧邻的注释块（/// 与 //），用于病史门。"""
    out = []
    idx = fn_line - 1
    while idx >= 0:
        s = lines[idx].strip()
        if s.startswith("///") or s.startswith("//") or s.startswith("#["):
            out.append(s)
            idx -= 1
            continue
        break
    return "\n".join(reversed(out))


def match_panic_blocks(masked_body: str) -> int:
    """数出「match <expr> { … panic!(…) … }」形态的块数。

    形态定义：match 关键字后（到第一个 '{'，即扫描表达式）开出的块里出现 panic!(——
    断言写在某个臂里，词法上看不到 assert! 宏（test-specification 规则 6.1 要求写成
    assert!(matches!(...))，但存量代码有此形态，0.6-15 实测 1 条）。

    块边界用花括号配平确定；**入参必须是掩码行**——字符串与注释里的花括号不参与配平，
    注释里的 panic! 也不会被当成断言。
    """
    blocks = 0
    for m in re.finditer(r"(?<![\w.])match\b[^{};]*\{", masked_body):
        depth = 0
        end = len(masked_body)
        for i in range(m.end() - 1, len(masked_body)):
            ch = masked_body[i]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    end = i
                    break
        if re.search(r"\bpanic!\s*\(", masked_body[m.end() : end]):
            blocks += 1
    return blocks


def assertion_summary(body: str, masked_body: str = "") -> str:
    counts = []
    for label, pattern in (
        ("assert_eq", re.compile(r"\bassert_eq\s*!")),
        ("assert_ne", re.compile(r"\bassert_ne\s*!")),
        ("matches", re.compile(r"\bmatches\s*!")),
        ("assert", re.compile(r"\bassert\s*!")),
        ("is_ok", re.compile(r"\.is_ok\(\)")),
        ("is_some", re.compile(r"\.is_some\(\)")),
        ("is_none", re.compile(r"\.is_none\(\)")),
        ("is_err", re.compile(r"\.is_err\(\)")),
        ("unwrap", re.compile(r"\.unwrap\(\)")),
        ("expect", re.compile(r"\.expect\(")),
        ("helper", HELPER_RE),
    ):
        n = len(pattern.findall(body))
        if n:
            counts.append(f"{label}x{n}")
    if masked_body:
        n = match_panic_blocks(masked_body)
        if n:
            counts.append(f"match_panicx{n}")
    return " ".join(counts) if counts else "（无断言）"


def classify(rel_path, name, header, body, has_should_panic, masked_body=None):
    """返回 (门, 依据, 处置)。

    header 同时承载两种用途：条款门（可核条目 §<数字>）与病史门的**文件头半径**
    （测试文件的 //! 头常写「回归背景：issue #226」，那是整份文件的病史声明）。

    masked_body：同一函数体的**掩码行**（字符串与注释已抹掉）。match + panic! 形态
    只在掩码行上判定，避免注释里的 match/panic! 造成假阳性；缺省时退回 body。
    """
    if masked_body is None:
        masked_body = body
    clause = bool(CLAUSE_RE.search(header))
    history = bool(HISTORY_RE.search(body)) or bool(HISTORY_RE.search(header))
    if rel_path.startswith(NO_RETIRE_PATHS):
        return "证明", "spec-shaped 判据面（语料/集成/格式化/runner），走进程边界", "留"
    if NO_RETIRE_NAME_RE.search(name):
        return "证明", "现状锚（名字含 unchanged/anchor/guard/preserve/baseline）", "留"
    if history:
        if HISTORY_RE.search(body):
            return "病史", "函数体/注释点名 issue/回归/缺陷/事故", "留"
        return "病史", "文件头（//! 回归背景）点名 issue/回归/缺陷/事故", "留"
    if clause:
        return "条款", "文件头有可核条目（§<数字>），引用未经人工确认（0.6-19 起抽样审计已废止）", "待核"
    if LOOSE_REF_RE.search(header):
        return "待判", "文件头只引整篇 RFC/规范而无节号（收紧后不算条款门）——需人工核", "待核"
    has_helper = bool(HELPER_RE.search(body))
    has_match_panic = match_panic_blocks(masked_body) > 0
    no_macro = not ASSERT_RE.search(body)
    # 「均不通过」是机器的**确信判决**（体内一条断言都没有），必须判准；
    # 「退役候选」只是待人工复核的池子，池子大小不是判决。
    # 故 0.6-15 的两处放宽只改「均不通过」这一档：体内唯一的断言形态是
    # 委托助手（含数字的名字）或 match + panic! 时，改落「待判/待判」——
    # 其余分箱（弱断言池等）一律按原判据走，不因放宽而变动。
    if no_macro and not has_helper and not has_match_panic and not has_should_panic:
        return "均不通过", "体内无任何断言（含委托断言助手、match+panic 形态）、也无 should_panic——证明门机器可判", "退役候选"
    if no_macro and not has_helper and not has_should_panic and has_match_panic:
        return "待判", "两门皆不过；断言为 match + panic! 形态，机器不判其覆盖力", "待判"
    if has_helper and no_macro:
        return "待判", "两门皆不过；断言委托给 assert*/check* 助手，机器不判其覆盖力", "待判"
    if WEAK_RE.search(body) and not STRONG_RE.search(body):
        return "待判", "两门皆不过；断言只有 is_ok/is_some/unwrap 一类弱断言", "退役候选"
    return "待判", "两门皆不过；含强断言，证明门需人工复核", "待判"


def scan():
    rows = []
    for path in iter_rust_files():
        rel_path = rel(path)
        raw = read_text(path)
        lines = raw.splitlines()
        # 掩码行只用于找函数体边界；断言统计与病史门仍读原文（0.6-13）
        masked_lines = mask_noncode(raw).splitlines()
        header = header_text(path, rel_path)
        for idx, line in enumerate(lines):
            if not TEST_ATTR_RE.match(line):
                continue
            fn_idx = idx + 1
            name = None
            while fn_idx < len(lines) and fn_idx < idx + 12:
                m = FN_RE.match(lines[fn_idx])
                if m:
                    name = m.group(1)
                    break
                fn_idx += 1
            if not name:
                continue
            start, end = body_span(masked_lines, fn_idx)
            body = "\n".join(lines[start : end + 1])
            masked_body = "\n".join(masked_lines[start : end + 1])
            attr_block = "\n".join(lines[idx : fn_idx + 1])
            gate, basis, action = classify(
                rel_path,
                name,
                header,
                body + "\n" + doc_block(lines, fn_idx),
                bool(SHOULD_PANIC_RE.search(attr_block)),
                masked_body,
            )
            rows.append(
                {
                    "name": name,
                    "file": rel_path,
                    "asserts": assertion_summary(body, masked_body),
                    "gate": gate,
                    "basis": basis,
                    "action": action,
                    "line": fn_idx + 1,
                }
            )
    rows.sort(key=lambda r: (r["file"], r["line"], r["name"]))
    return rows


def render_tsv(rows):
    lines = ["测试名\t入库文件路径\t断言摘要\t门\t依据\t处置"]
    for r in rows:
        lines.append("\t".join([r["name"], r["file"], r["asserts"], r["gate"], r["basis"], r["action"]]))
    return "\n".join(lines) + "\n"


def summarize(rows):
    from collections import Counter

    gates = Counter(r["gate"] for r in rows)
    actions = Counter(r["action"] for r in rows)
    files = Counter(r["file"] for r in rows)
    out = [f"[census] 测试 {len(rows)} 条，覆盖 {len(files)} 个文件"]
    out.append("[census] 按门：" + " · ".join(f"{k} {v}" for k, v in sorted(gates.items())))
    out.append("[census] 按处置：" + " · ".join(f"{k} {v}" for k, v in sorted(actions.items())))
    return "\n".join(out)


SELFTEST_CLAUSE = "//! 条款门正例：依据 language-spec §3.2 与 RFC-029\n"


def _selftest_case(tmp, filename, header, body):
    path = os.path.join(tmp, filename)
    with io.open(path, "w", encoding="utf-8") as fh:
        fh.write(header + body)
    raw = read_text(path)
    lines = raw.splitlines()
    masked_lines = mask_noncode(raw).splitlines()
    for idx, line in enumerate(lines):
        if TEST_ATTR_RE.match(line):
            fn_idx = idx + 1
            while fn_idx < len(lines) and not FN_RE.match(lines[fn_idx]):
                fn_idx += 1
            name = FN_RE.match(lines[fn_idx]).group(1)
            start, end = body_span(masked_lines, fn_idx)
            body_text = "\n".join(lines[start : end + 1])
            masked_body = "\n".join(masked_lines[start : end + 1])
            gate, basis, action = classify(
                rel(filename),
                name,
                "\n".join(lines[:25]),
                body_text + "\n" + doc_block(lines, fn_idx),
                bool(SHOULD_PANIC_RE.search("\n".join(lines[idx : fn_idx + 1]))),
                masked_body,
            )
            return {
                "name": name,
                "gate": gate,
                "action": action,
                "basis": basis,
                "body_lines": body_text.count("\n") + 1,
                "body": body_text,
            }
    raise EnvError(f"selftest 样例没有解析出 #[test]：{filename}")


def selftest():
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        pos = _selftest_case(
            tmp, "pos.rs", SELFTEST_CLAUSE,
            "#[test]\nfn test_clause_positive() {\n    assert_eq!(1 + 1, 2);\n}\n",
        )
        neg = _selftest_case(
            tmp, "neg.rs", "//! 没有条款引用\n",
            "#[test]\nfn test_weak_negative() {\n    let r = Ok::<i32, ()>(1);\n    assert!(r.is_ok());\n}\n",
        )
        bare = _selftest_case(
            tmp, "bare.rs", "//! 依据 RFC-011（没有节号）\n",
            "#[test]\nfn test_bare_rfc_not_a_clause() {\n    assert_eq!(1 + 1, 2);\n}\n",
        )
        sectioned = _selftest_case(
            tmp, "sectioned.rs", "//! 依据 RFC-011 §4 编译期泛型\n",
            "#[test]\nfn test_sectioned_clause_is_pending() {\n    assert_eq!(1 + 1, 2);\n}\n",
        )
        empty = _selftest_case(
            tmp, "empty.rs", "//! 没有条款引用\n",
            "#[test]\nfn test_no_assertion() {\n    let _x = 1;\n}\n",
        )
        # 0.6-13：字符串 / 字符 / 原始字符串 / 注释里的花括号不得截断函数体。
        # 每个样例后面都跟一条「哨兵」测试——旧实现会在花括号上失衡，
        # 一路吞到文件尾，函数体因此被算长。
        # 第一个样例把 "}" 放在 "{" 前面：旧实现一见到 "}" 就把配平拉平、
        # 当场截断；两个花括号数量相等也照样截错（这才是「配平」这个做法的病根）。
        braces = _selftest_case(
            tmp, "braces.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_brace_in_string_not_truncated() {\n"
            '    let close = "}";\n'
            '    let open = "{";\n'
            "    assert_eq!(open.len() + close.len(), 2);\n"
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_braces() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
        unbalanced = _selftest_case(
            tmp, "unbalanced.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_unbalanced_brace_in_string_not_truncated() {\n"
            '    let tokens = tokenize("{").unwrap();\n'
            "    assert_eq!(tokens.len(), 1);\n"
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_unbalanced() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
        raw = _selftest_case(
            tmp, "raw.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_brace_in_raw_string_and_comment() {\n"
            '    let raw = r#"{"#;\n'
            "    /* } 注释里的花括号 */\n"
            '    assert_eq!(raw, "{");\n'
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_raw() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
        charlit = _selftest_case(
            tmp, "charlit.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_brace_in_char_literal() {\n"
            "    let ch = '{';\n"
            "    assert_eq!(ch, '{');\n"
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_charlit() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
        # 0.6-15：助手名含数字（assert_slot_is_42）与 match + panic! 形态
        # 都必须被认作断言——旧正则把它们当成「体内无任何断言」，会把有判据的测试
        # 误分箱成退役候选。另加两条反向对照：match 里没有 panic!、panic! 只在注释里，
        # 都**不得**被认作断言（证明放宽没有过头）。
        digit_helper = _selftest_case(
            tmp, "digit_helper.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_digit_helper_is_an_assertion() {\n"
            "    let value = 42;\n"
            "    assert_slot_is_42(&value);\n"
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_digit_helper() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
        match_panic = _selftest_case(
            tmp, "match_panic.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_match_panic_is_an_assertion() {\n"
            "    let value = 42;\n"
            "    match value {\n"
            "        42 => {}\n"
            '        _ => panic!("expected 42"),\n'
            "    }\n"
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_match_panic() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
        match_no_panic = _selftest_case(
            tmp, "match_no_panic.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_match_without_panic_is_not_an_assertion() {\n"
            "    let value = 42;\n"
            "    match value {\n"
            "        42 => {}\n"
            "        _ => {}\n"
            "    }\n"
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_match_no_panic() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
        panic_in_comment = _selftest_case(
            tmp, "panic_in_comment.rs", "//! 没有条款引用\n",
            "#[test]\n"
            "fn test_panic_in_comment_is_not_an_assertion() {\n"
            "    let value = 42;\n"
            '    // 若这里 panic!("boom")，测试就会红\n'
            "    match value {\n"
            "        42 => {}\n"
            "        _ => {}\n"
            "    }\n"
            "}\n\n"
            "#[test]\n"
            "fn test_canary_after_panic_in_comment() {\n"
            "    assert_eq!(2 + 2, 4);\n"
            "}\n",
        )
    ok = True
    if pos["gate"] != "条款" or pos["action"] != "待核":
        print(f"[selftest] FAIL 未经人工确认的条款引用应落 条款/待核，实际 {pos['gate']}/{pos['action']}")
        ok = False
    if neg["gate"] != "待判" or neg["action"] != "退役候选":
        print(f"[selftest] FAIL 弱断言反例应落 待判/退役候选，实际 {neg['gate']}/{neg['action']}")
        ok = False
    if empty["gate"] != "均不通过" or empty["action"] != "退役候选":
        print(f"[selftest] FAIL 无断言反例应落 均不通过/退役候选，实际 {empty['gate']}/{empty['action']}")
        ok = False
    if bare["gate"] == "条款":
        print(f"[selftest] FAIL 只有 RFC 编号没有节号不得过条款门，实际 {bare['gate']}/{bare['action']}")
        ok = False
    if sectioned["gate"] != "条款" or sectioned["action"] != "待核":
        print(f"[selftest] FAIL 有节号但未经人工确认应落 条款/待核，实际 {sectioned['gate']}/{sectioned['action']}")
        ok = False
    for label, case, want_lines in (
        ("字符串里的花括号", braces, 5),
        ("字符串里不成对的花括号", unbalanced, 4),
        ("原始字符串与块注释里的花括号", raw, 5),
        ("字符字面量里的花括号", charlit, 4),
    ):
        if case["body_lines"] != want_lines:
            print(
                f"[selftest] FAIL {label}不得截断函数体，实际取到 {case['body_lines']} 行"
                f"（应为 {want_lines}）"
            )
            ok = False
        if case["action"] != "待判":
            print(
                f"[selftest] FAIL 取到完整函数体后应见强断言（→待判），"
                f"实际 {case['action']}（{label}）"
            )
            ok = False
    for label, case, want_lines in (
        ("助手名含数字", digit_helper, 4),
        ("match + panic! 形态", match_panic, 7),
        ("match 里没有 panic!", match_no_panic, 7),
        ("panic! 只出现在注释里", panic_in_comment, 8),
    ):
        if case["body_lines"] != want_lines:
            print(
                f"[selftest] FAIL {label}的函数体边界取错，实际 {case['body_lines']} 行"
                f"（应为 {want_lines}）"
            )
            ok = False
    for label, case in (
        ("助手名含数字", digit_helper),
        ("match + panic! 形态", match_panic),
    ):
        if case["gate"] != "待判" or case["action"] != "待判":
            print(
                f"[selftest] FAIL {label}必须被认作断言（→待判/待判），"
                f"实际 {case['gate']}/{case['action']}"
            )
            ok = False
    for label, case in (
        ("match 里没有 panic!", match_no_panic),
        ("panic! 只出现在注释里", panic_in_comment),
    ):
        if case["gate"] != "均不通过" or case["action"] != "退役候选":
            print(
                f"[selftest] FAIL {label}不得被认作断言，"
                f"实际 {case['gate']}/{case['action']}"
            )
            ok = False
    if ok:
        print(
            "[selftest] OK：§节号正例→条款/待核、弱断言反例→待判/退役候选、"
            "无断言反例→均不通过/退役候选、裸 RFC 反例→不过条款门、"
            "字面量与注释里的花括号不截断函数体（4 例）、"
            "含数字的助手名与 match+panic 形态被认作断言（2 例，含 2 例反向对照）"
        )
    else:
        print("[selftest] FAIL：见上方逐条失败项")
    return 0 if ok else 1


def main(argv=None):
    force_utf8_stdio()
    parser = argparse.ArgumentParser(description="测试三扇门普查（coding-rules 第七部分）")
    parser.add_argument("--selftest", action="store_true", help="跑最小正/反例自检，不扫仓库")
    parser.add_argument("--check", action="store_true", help="只比较已入库 TSV 是否与当前树一致")
    parser.add_argument("--stdout", action="store_true", help="只把 TSV 打到 stdout，不写文件")
    args = parser.parse_args(argv)

    if args.selftest:
        return selftest()

    for root in SCAN_ROOTS:
        if not os.path.isdir(os.path.join(REPO_ROOT, root)):
            print(f"[error] 扫描根缺失：{root}", file=sys.stderr)
            return 3

    rows = scan()
    tsv = render_tsv(rows)
    if args.stdout:
        sys.stdout.write(tsv)
        return 0
    if args.check:
        if not os.path.isfile(TSV_PATH):
            print(f"[census] TSV 不存在：{rel(TSV_PATH)}", file=sys.stderr)
            return 1
        if read_text(TSV_PATH) != tsv:
            print("[census] TSV 与当前树不一致，请重跑生成", file=sys.stderr)
            return 1
        print(f"[census] TSV 最新：{rel(TSV_PATH)}（{len(rows)} 条）")
        return 0

    with io.open(TSV_PATH, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(tsv)
    print(summarize(rows))
    print(f"[census] 已写入 {rel(TSV_PATH)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
