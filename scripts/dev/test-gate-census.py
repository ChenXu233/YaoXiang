#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""test-gate-census: 测试三扇门普查（coding-rules 第七部分）

背景
----
WBS 0.6 的「大清理」前置：把全仓 #[test] 按三扇门（条款 / 病史 / 证明）机械分类，
产出可重跑的逐条判定表。本脚本只读不写测试——不改、不删任何测试代码。

三扇门的机器判据（与 coding-rules 第七部分对齐）
-----------------------------------------------
* 条款门：测试文件的头部（前 25 行；src/**/tests*/ 下的文件连父 mod.rs 一起看）
  引用 SPEC/RFC/§/语言规范 —— 该测试对得上一条「条款」。
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
* 条款门或病史门过 → 「留」。
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

# 条款门：文件头引用 SPEC/RFC/§/语言规范
CLAUSE_RE = re.compile(r"(§\s*\d|RFC[- ]?\d{3}|SPEC[- ]?[A-Z0-9]|language-spec|语言规范)")
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
HELPER_RE = re.compile(r"\b(assert|check)[A-Za-z_]*\s*\(")

TEST_ATTR_RE = re.compile(r"^\s*#\[test\]\s*$")
FN_RE = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
SHOULD_PANIC_RE = re.compile(r"#\[should_panic")


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


def body_span(lines, start):
    """从 fn name( 所在行起，按花括号配平找到函数体结束行（含）。"""
    depth = 0
    opened = False
    for idx in range(start, len(lines)):
        for ch in lines[idx]:
            if ch == "{":
                depth += 1
                opened = True
            elif ch == "}":
                depth -= 1
                if opened and depth == 0:
                    return start, idx
    return start, len(lines) - 1


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


def assertion_summary(body: str) -> str:
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
        ("helper", re.compile(r"\b(assert|check)[A-Za-z_]*\s*\(")),
    ):
        n = len(pattern.findall(body))
        if n:
            counts.append(f"{label}x{n}")
    return " ".join(counts) if counts else "（无断言）"


def classify(rel_path, name, header, body, has_should_panic):
    """返回 (门, 依据, 处置)。

    header 同时承载两种用途：条款门（SPEC/RFC/§ 引用）与病史门的**文件头半径**
    （测试文件的 //! 头常写「回归背景：issue #226」，那是整份文件的病史声明）。
    """
    clause = bool(CLAUSE_RE.search(header))
    history = bool(HISTORY_RE.search(body))
    if rel_path.startswith(NO_RETIRE_PATHS):
        return "证明", "spec-shaped 判据面（语料/集成/格式化/runner），走进程边界", "留"
    if NO_RETIRE_NAME_RE.search(name):
        return "证明", "现状锚（名字含 unchanged/anchor/guard/preserve/baseline）", "留"
    if clause and history:
        return "条款", "文件头引用 §/RFC/SPEC + 函数注释点名故障", "留"
    if clause:
        return "条款", "文件头引用 §/RFC/SPEC/语言规范", "留"
    if history:
        return "病史", "函数体/注释点名 issue/回归/缺陷/事故", "留"
    if HISTORY_RE.search(header):
        return "病史", "文件头（//! 回归背景）点名 issue/回归/缺陷/事故", "留"
    if not ASSERT_RE.search(body) and not HELPER_RE.search(body) and not has_should_panic:
        return "均不通过", "体内无任何断言（含委托断言助手）、也无 should_panic——证明门机器可判", "退役候选"
    if HELPER_RE.search(body) and not ASSERT_RE.search(body):
        return "待判", "两门皆不过；断言委托给 assert*/check* 助手，机器不判其覆盖力", "待判"
    if WEAK_RE.search(body) and not STRONG_RE.search(body):
        return "待判", "两门皆不过；断言只有 is_ok/is_some/unwrap 一类弱断言", "退役候选"
    return "待判", "两门皆不过；含强断言，证明门需人工复核", "待判"


def scan():
    rows = []
    for path in iter_rust_files():
        rel_path = rel(path)
        lines = read_text(path).splitlines()
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
            start, end = body_span(lines, fn_idx)
            body = "\n".join(lines[start : end + 1])
            attr_block = "\n".join(lines[idx : fn_idx + 1])
            gate, basis, action = classify(
                rel_path,
                name,
                header,
                body + "\n" + doc_block(lines, fn_idx),
                bool(SHOULD_PANIC_RE.search(attr_block)),
            )
            rows.append(
                {
                    "name": name,
                    "file": rel_path,
                    "asserts": assertion_summary(body),
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
    lines = read_text(path).splitlines()
    for idx, line in enumerate(lines):
        if TEST_ATTR_RE.match(line):
            fn_idx = idx + 1
            while fn_idx < len(lines) and not FN_RE.match(lines[fn_idx]):
                fn_idx += 1
            name = FN_RE.match(lines[fn_idx]).group(1)
            start, end = body_span(lines, fn_idx)
            body_text = "\n".join(lines[start : end + 1])
            gate, basis, action = classify(
                rel(filename),
                name,
                "\n".join(lines[:25]),
                body_text + "\n" + doc_block(lines, fn_idx),
                bool(SHOULD_PANIC_RE.search("\n".join(lines[idx : fn_idx + 1]))),
            )
            return {"name": name, "gate": gate, "action": action, "basis": basis}
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
        empty = _selftest_case(
            tmp, "empty.rs", "//! 没有条款引用\n",
            "#[test]\nfn test_no_assertion() {\n    let _x = 1;\n}\n",
        )
    ok = True
    if pos["gate"] != "条款" or pos["action"] != "留":
        print(f"[selftest] FAIL 正例应落 条款/留，实际 {pos['gate']}/{pos['action']}")
        ok = False
    if neg["gate"] != "待判" or neg["action"] != "退役候选":
        print(f"[selftest] FAIL 弱断言反例应落 待判/退役候选，实际 {neg['gate']}/{neg['action']}")
        ok = False
    if empty["gate"] != "均不通过" or empty["action"] != "退役候选":
        print(f"[selftest] FAIL 无断言反例应落 均不通过/退役候选，实际 {empty['gate']}/{empty['action']}")
        ok = False
    print("[selftest] OK：正例（条款门→留）、弱断言反例（待判→退役候选）、无断言反例（均不通过→退役候选）")
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
