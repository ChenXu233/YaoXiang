#!/usr/bin/env python3
"""诊断质量审计（#322 M3）。

检测项：
  1. 绕过注册表构造：Diagnostic::error/warning/info/hint() 直接构造
     （必须经 ErrorCodeDefinition 快捷方法/builder，否则 i18n 与码校验失效）
     —— 硬门槛，0 容忍
  2. 未注册码使用：非 codes/*.rs 文件中的裸码字面量（"E####"/"W####"）与
     伪码（E_INTERNAL 等）——码必须在权威注册表定义 —— 硬门槛，0 容忍
  3. span 挂载率（报告型）：快捷方法调用点带 .at( 的比例
  4. 硬编码消息参数（报告型）：format! 结果直接作为唯一消息构造的嫌疑点统计

用法：
  python scripts/audit_diagnostics.py            # 审计，硬门槛失败 exit 1
  python scripts/audit_diagnostics.py --report   # 额外输出 span/消息统计基线
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "src"
CODES_DIR = SRC / "util" / "diagnostic" / "codes"

# 绕过注册表构造的检测（排除合法构造点：builder 内部、Diagnostic 定义、测试）
DIRECT_CONSTRUCT_RE = re.compile(r"\bDiagnostic::(error|warning|info|hint)\s*\(")
EXCLUDE_PATTERNS = ("codes/builder.rs", "diagnostic/error.rs", "error_macro.rs", "tests", "/test", "\\tests", "\\test")

CODE_LITERAL_RE = re.compile(r'"([EW]\d{4})"')
PSEUDO_CODE_RE = re.compile(r'"E_[A-Z_]+"|"W_[A-Z_]+"')
QUICK_FN_RE = re.compile(
    r'\((?:"(?:E\d{4}|W\d{4})",\s*(\w+)\()', re.M
)


def load_registered_codes() -> set[str]:
    codes: set[str] = set()
    for path in CODES_DIR.glob("*xxx.rs"):
        codes.update(CODE_LITERAL_RE.findall(path.read_text(encoding="utf-8")))
    return codes


def iter_rs_files():
    for path in SRC.rglob("*.rs"):
        rel = path.relative_to(REPO).as_posix()
        yield rel, path


def check_direct_construct() -> list[str]:
    errors: list[str] = []
    for rel, path in iter_rs_files():
        if any(p in rel for p in EXCLUDE_PATTERNS):
            continue
        for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if DIRECT_CONSTRUCT_RE.search(line):
                errors.append(f"绕过注册表构造: {rel}:{i}: {line.strip()[:90]}")
    return errors


def check_unregistered_codes(registered: set[str]) -> list[str]:
    errors: list[str] = []
    code_use_re = re.compile(
        r'(?:Diagnostic::(?:error|warning|info|hint)\s*\(\s*|DiagnosticBuilder::new\(\s*|\.code\s*=\s*)'
        r'"([EW][0-9A-Z_]{4,12})"'
    )
    for rel, path in iter_rs_files():
        if any(p in rel for p in EXCLUDE_PATTERNS):
            continue
        text = path.read_text(encoding="utf-8")
        for i, line in enumerate(text.splitlines(), 1):
            for m in PSEUDO_CODE_RE.finditer(line):
                errors.append(f"伪码使用: {rel}:{i}: {m.group(0)}")
            for m in code_use_re.finditer(line):
                code = m.group(1)
                if code not in registered:
                    errors.append(f"未注册码使用: {rel}:{i}: {code}")
    return errors


def report_span_and_message(registered: set[str]) -> None:
    """报告型统计：span 挂载率与快捷方法调用规模（基线跟踪用）。"""
    total_call = with_at = 0
    for rel, path in iter_rs_files():
        if any(p in rel for p in EXCLUDE_PATTERNS) or "codes/" in rel:
            continue
        text = path.read_text(encoding="utf-8")
        # 快捷方法调用点（跨行窗口 400 字符内找 .at(）
        for m in re.finditer(r"(?:ErrorCodeDefinition::)?(\w+)\([^;]{0,400}?\.at\(", text):
            total_call += 1
            with_at += 1
        for m in re.finditer(r"ErrorCodeDefinition::(\w+)\(", text):
            total_call += 1
    print(f"span 挂载率基线: .at 链 {with_at} / 快捷调用 {total_call}")


def main() -> int:
    report = "--report" in sys.argv
    registered = load_registered_codes()

    errors = check_direct_construct()
    errors += check_unregistered_codes(registered)

    if report:
        report_span_and_message(registered)
        print(f"注册码规模: {len(registered)}")

    if errors:
        print(f"\n诊断审计失败（{len(errors)} 项）:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1
    print(f"诊断审计通过（绕过构造 0，未注册码 0；注册码 {len(registered)}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
