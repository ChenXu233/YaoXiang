#!/usr/bin/env python3
"""错误码/警告码权威注册表校验（#320 M1）。

三方对齐：src/util/diagnostic/codes/*.rs ↔ locales/*.json ↔ RFC-013 码表。

检查项：
  1. 码唯一性：同一码在 codes/*.rs 不得定义两次
  2. 段位合法性：W 码不得出现在 e*.rs，E 码不得出现在 w*.rs；
     码的千位段必须与所在文件段位一致（如 e1xxx.rs 只收 E1xxx）
  3. locales 覆盖：codes/*.rs 每个码必须在全部 locales/*.json 有 key；
     locales 中孤立 key（实现无定义）同样报错
  4. RFC-013 码表一致：实现已定义的码必须出现在 RFC-013 码表；
     RFC-013 码表中未实现的码必须为预留（行说明含「预留」「未接线」「已删」）

用法：
  python scripts/check_error_codes.py            # 校验，发现问题 exit 1
  python scripts/check_error_codes.py --report   # 额外输出注册表统计
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CODES_DIR = REPO / "src" / "util" / "diagnostic" / "codes"
LOCALES_DIR = REPO / "locales"
RFC013 = REPO / "docs" / "src" / "design" / "rfc" / "accepted" / "013-error-code-specification.md"

# 人工翻译源（构建期 build.rs 门槛同步强制）；其余语言由 i18n bot 异步补齐（#325）
HARD_LOCALES = {"zh"}

CODE_RE = re.compile(r'code:\s*"([EW]\d{4})"')
RFC_TABLE_ROW_RE = re.compile(r"^\|\s*`?([EW]\d{4})`?\s*\|")
RESERVED_MARKS = ("预留", "未接线", "已删")


def load_registered_codes() -> tuple[dict[str, list[str]], list[str]]:
    """返回 (码 -> 定义文件列表, 错误列表)。

    两个注册源：
    - 编译器诊断码：src/util/diagnostic/codes/*.rs 的 code: "..." 定义
    - 运行时错误值码：src/std/result.rs 的 RUNTIME_ERROR_CODES 表（#323 M4）
    """
    defs: dict[str, list[str]] = {}
    errors: list[str] = []
    sources = sorted(CODES_DIR.glob("e*xxx.rs")) + sorted(CODES_DIR.glob("w*xxx.rs"))
    result_rs = REPO / "src" / "std" / "result.rs"
    if result_rs.exists():
        sources.append(result_rs)
    for path in sources:
        seg = path.name[1]  # e1xxx.rs -> '1'
        text = path.read_text(encoding="utf-8")
        if path == result_rs:
            # RUNTIME_ERROR_CODES 表：("E60xx", "语义") 元组形态
            for m in re.finditer(r'\("([EW]\d{4})",', text):
                defs.setdefault(m.group(1), []).append(path.name)
            continue
        for m in CODE_RE.finditer(text):
            code = m.group(1)
            defs.setdefault(code, []).append(path.name)
            if path.parent == CODES_DIR:
                # 段位合法性仅约束诊断码文件（E 码在 e 文件且千位一致；W 码只在 w 文件）
                if code.startswith("E"):
                    if not path.name.startswith("e"):
                        errors.append(f"段位错误: {code} 定义在 {path.name}（E 码不得出现在 w* 文件）")
                    elif code[1] != seg:
                        errors.append(f"段位错误: {code} 定义在 {path.name}（千位段不一致）")
                else:  # W 码
                    if not path.name.startswith("w"):
                        errors.append(f"段位错误: {code} 定义在 {path.name}（W 码不得出现在 e* 文件）")
    for code, files in defs.items():
        if len(files) > 1:
            errors.append(f"重复定义: {code} 出现于 {', '.join(files)}")
    return defs, errors


def check_locales(defs: dict[str, list[str]]) -> tuple[list[str], list[str]]:
    """返回 (errors, warnings)。

    分级（#325）：zh 是唯一人工翻译源（构建期门槛同步强制），缺失为 error；
    其余语言由 i18n bot 异步补齐，缺失仅 warning（推送→bot 落地的窗口期不阻塞 CI）。
    孤立 key（实现无定义）说明码被删但翻译残留，任何语言都是 error（bot 不会清理）。
    """
    errors: list[str] = []
    warnings: list[str] = []
    locale_files = sorted(LOCALES_DIR.glob("*.json"))
    if not locale_files:
        return [f"未找到 locales 文件: {LOCALES_DIR}"], []
    for lf in locale_files:
        lang = lf.stem
        keys = set(json.loads(lf.read_text(encoding="utf-8")).keys())
        for code in defs:
            if code not in keys:
                msg = f"locales 缺失: {code} 未在 {lf.name} 定义消息模板"
                if lang in HARD_LOCALES:
                    errors.append(msg)
                else:
                    warnings.append(msg + "（bot 将异步补齐）")
        orphans = sorted(k for k in keys if re.fullmatch(r"[EW]\d{4}", k) and k not in defs)
        for k in orphans:
            errors.append(f"locales 孤立: {k} 在 {lf.name} 有模板但实现未定义")
    return errors, warnings


def collect_rfc013_codes() -> tuple[dict[str, str], list[str]]:
    """返回 (码 -> 表行原文, 错误列表)。"""
    errors: list[str] = []
    rows: dict[str, str] = {}
    in_table = False
    for line in RFC013.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("|"):
            in_table = True
            m = RFC_TABLE_ROW_RE.match(stripped)
            if m:
                code = m.group(1)
                if code in rows:
                    errors.append(f"RFC-013 码表重复行: {code}")
                rows[code] = stripped
        elif in_table and not stripped:
            in_table = False
    return rows, errors


def check_rfc013(defs: dict[str, list[str]]) -> list[str]:
    errors: list[str] = []
    rfc_rows, rfc_errors = collect_rfc013_codes()
    errors.extend(rfc_errors)
    if not rfc_rows:
        return [f"RFC-013 未解析到码表行: {RFC013}"]
    for code in defs:
        if code not in rfc_rows:
            errors.append(f"RFC-013 缺行: {code} 已实现但码表未收录")
    for code, row in rfc_rows.items():
        if code not in defs and not any(mark in row for mark in RESERVED_MARKS):
            errors.append(f"RFC-013 未实现未标注: {code} 无实现且行内无预留/未接线/已删标注")
    return errors


def main() -> int:
    report = "--report" in sys.argv
    defs, errors = load_registered_codes()
    locale_errors, locale_warnings = check_locales(defs)
    errors += locale_errors
    errors += check_rfc013(defs)

    if report:
        by_prefix: dict[str, int] = {}
        for code in defs:
            prefix = code[:2]
            by_prefix[prefix] = by_prefix.get(prefix, 0) + 1
        print(f"注册表统计: 共 {len(defs)} 码")
        for prefix in sorted(by_prefix):
            print(f"  {prefix}xx: {by_prefix[prefix]}")

    if locale_warnings:
        print(f"\n警告（{len(locale_warnings)} 项，不阻塞——bot 异步补齐）:")
        for w in locale_warnings:
            print(f"  - {w}")

    if errors:
        print(f"\n错误码注册表校验失败（{len(errors)} 项）:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1
    print(f"错误码注册表校验通过（{len(defs)} 码，三方对齐）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
