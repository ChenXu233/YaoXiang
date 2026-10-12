#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-boundary.py —— 禁令二判据 C（边界侵蚀）门禁

规则本体：docs/src/dev/coding-rules.md 第一部分·禁令二判据 C
层定义与依赖方向：docs/src/dev/architecture/01-routing.md §依赖方向规范（唯一权威）
任务来源：docs/src/dev/architecture/09-execution-wbs.md §P0 任务 0.2.3
引入阶段：P0（report-only，默认退出码恒 0）；P9 统一转硬（CI 改为 --strict）。

检查项（对应 01 §依赖方向规范的七行规则）：
  B1 禁止 include!               现状 1 处（checker.rs:5618，P5 消除）
  B2 L2 不得依赖 L3/L4           现状 2 处（03 §2.4，P6 消除）
  B3 L3/L4 不得 use L2 行为      数据路径豁免：parser::ast（D1 顶层域）、
                                 lexer::tokens（字面量/Token 数据）；测试目录豁免。
                                 现状生产命中 0，基线取 0
  B4 L3 不得反向依赖 L4          现状 3 处（middle → backends::common::opcode，
                                 P6/P10 随词表迁移消除）
  B5 opcode 词表唯一归属          backends/ 内不得新增 opcode 常量定义文件
                                （现状唯一：backends/common/opcode.rs，83 个）
  B6 parser 内不得硬编码类型/谓词名  现状 2 文件（CONST_PARAM_TYPES、"Terminates"，
                                 P6 消除）；字面量匹配前先剥掉行注释
  C  pub(crate) 泄漏计数         只记基线（总数 + 分目录），只许减不许增（P9 转硬）

库存登记口径：{文件: 命中数}。文件内新增命中 → 计数增长 → 违规；
新文件出现命中 → 违规。行号漂移不影响判定。
豁免：命中行或其上一行含 `// reason:` 注释 → 豁免（数量单列）。

退出码：0 = report-only（默认）；--strict 且违规 > 0 → 1（P9 启用）。
"""
import io
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(ROOT, 'src')

# ---- 层归属（按现状路径；目录改名 D1 后随 6.7 批次同步本表）----
L2_DIRS = ('frontend/core/lexer/', 'frontend/core/parser/')
L3_DIRS = ('frontend/core/typecheck/', 'frontend/core/types/', 'middle/')
L4_DIRS = ('backends/', 'std/')

# L3/L4 可消费的 L2 数据路径（AST 顶层域 D1；字面量/Token 是词法数据域）
L2_DATA_PREFIXES = ('parser::ast', 'lexer::tokens', 'lexer::TokenKind', 'lexer::Token', 'lexer::Literal')

# ---- 现状库存（2026-10-06 实测登记；消除后从表中移除，新增即违规）----
KNOWN_INCLUDE = {'frontend/core/typecheck/checker.rs': 1}
KNOWN_L2_CROSS = {
    'frontend/core/parser/ast.rs': 1,                 # :930 operator_interfaces::spec 调用点
    'frontend/core/parser/statements/declarations.rs': 1,  # :509 同上
}
KNOWN_MIDDLE_TO_BACKENDS = {
    'middle/core/bytecode.rs': 1,                     # :13 use ...::common::opcode
    'middle/passes/codegen/translator.rs': 1,         # :5 同上
    'middle/passes/codegen/emitter.rs': 1,            # :5 同上
}
KNOWN_OPCODE_DEF_FILES = {'backends/common/opcode.rs': 83}
KNOWN_PARSER_HARDCODE = {
    'frontend/core/parser/ast.rs': 2,                 # :839-840 CONST_PARAM_TYPES 常量
    'frontend/core/parser/statements/declarations.rs': 1,  # :499 "Terminates"
}

BUILTIN_NAMES = {
    'Terminates', 'Int', 'Int64', 'Int32', 'Int16', 'Int8',
    'Float', 'Float64', 'Float32', 'Bool', 'Char', 'String', 'Bytes',
    'Void', 'List', 'Vec', 'Array', 'Dict', 'Tuple', 'Range', 'Option', 'Result',
}

RE_INCLUDE = re.compile(r'\binclude!\s*\(')
RE_L2_CROSS = re.compile(r'crate::(?:frontend::core::(?:typecheck|types)|middle|backends)::')
RE_TO_L2 = re.compile(r'crate::frontend::core::(lexer|parser)::([A-Za-z_][\w:]*)')
RE_MID_BACK = re.compile(r'crate::backends::')
RE_OPCODE_DEF = re.compile(r'pub const [A-Za-z_]\w*: u8 = 0x')
RE_BUILTIN_LIT = re.compile(r'"(' + '|'.join(sorted(BUILTIN_NAMES)) + r')"')
RE_PUB_CRATE = re.compile(r'\bpub\(crate\)')


def read(p):
    with io.open(p, 'r', encoding='utf-8', errors='replace') as f:
        return f.read()


def rs_files():
    for r, dirs, files in os.walk(SRC):
        dirs[:] = [d for d in dirs if d != 'target']
        for fn in files:
            if fn.endswith('.rs'):
                yield os.path.join(r, fn)


def rel(p):
    return os.path.relpath(p, SRC).replace('\\', '/')


def is_test_path(rp):
    return '/tests/' in rp.replace('\\', '/')


def strip_comment(line):
    """剥掉行注释（// 之后）；字符串字面量里的 // 不在本仓代码风格中出现，可接受。"""
    idx = line.find('//')
    return line if idx < 0 else line[:idx]


def has_reason(lines, idx):
    for j in (idx, idx - 1):
        if 0 <= j < len(lines) and '// reason:' in lines[j]:
            return True
    return False


def count_hits(path, pattern, data_exempt=False):
    """返回 (命中行号列表, 豁免数)。data_exempt=True 时跳过 L2 数据路径与测试目录。"""
    rp = rel(path)
    if data_exempt and is_test_path(rp):
        return [], 0
    hits = []
    exempt = 0
    lines = read(path).split('\n')
    for i, raw in enumerate(lines):
        line = strip_comment(raw)
        if data_exempt:
            for m in pattern.finditer(line):
                sub = m.group(1) + '::' + m.group(2)
                if any(sub.startswith(dp) for dp in L2_DATA_PREFIXES):
                    continue
                if has_reason(lines, i):
                    exempt += 1
                else:
                    hits.append(i + 1)
        else:
            if pattern.search(line):
                if has_reason(lines, i):
                    exempt += 1
                else:
                    hits.append(i + 1)
    return hits, exempt


def main():
    strict = '--strict' in sys.argv[1:]
    files = sorted(rs_files())
    violations = []
    exempt_total = 0

    def check(rule, rp, hits, known, desc):
        """库存比对：新文件或计数增长 → 违规。"""
        if not hits:
            if rp in known:
                print('  [进展] %s 的 %s 已清零（可从库存移除）' % (rp, desc))
            return
        base = known.get(rp, 0)
        if len(hits) > base:
            violations.append('%s: %s %d 处（库存 %d）→ %s' % (rp, desc, len(hits), base, '、'.join('L%d' % h for h in hits[:6])))

    print('check-boundary: 禁令二判据 C（边界侵蚀）门禁报告%s' % (' [STRICT]' if strict else '（report-only）'))
    print('扫描范围: src/（%d 个 .rs）\n' % len(files))

    # ---- B1 include! ----
    print('== B1 include!（库存 %d 处，P5 消除）==' % sum(KNOWN_INCLUDE.values()))
    for f in files:
        rp = rel(f)
        hits, ex = count_hits(f, RE_INCLUDE)
        exempt_total += ex
        check('B1', rp, hits, KNOWN_INCLUDE, 'include!')
    print()

    # ---- B2 L2 → L3/L4 ----
    print('== B2 L2→L3/L4 反向依赖（库存 %d 处，03 §2.4，P6 消除）==' % sum(KNOWN_L2_CROSS.values()))
    for f in files:
        rp = rel(f)
        if not any(rp.startswith(d) for d in L2_DIRS):
            continue
        hits, ex = count_hits(f, RE_L2_CROSS)
        exempt_total += ex
        check('B2', rp, hits, KNOWN_L2_CROSS, 'L2→L3/L4 引用')
    print()

    # ---- B3 L3/L4 → L2 行为（数据路径与测试豁免，基线 0）----
    print('== B3 L3/L4→L2 行为引用（数据路径 parser::ast / lexer::tokens 豁免；基线 0）==')
    for f in files:
        rp = rel(f)
        if not any(rp.startswith(d) for d in L3_DIRS + L4_DIRS):
            continue
        hits, ex = count_hits(f, RE_TO_L2, data_exempt=True)
        exempt_total += ex
        for h in hits:
            violations.append('%s: B3 L3/L4→L2 行为引用 → L%d' % (rp, h))
    print('  （生产命中即违规，无库存）')
    print()

    # ---- B4 L3 → L4（middle → backends）----
    print('== B4 L3→L4 反向依赖（库存 %d 处，opcode 词表迁移消除）==' % sum(KNOWN_MIDDLE_TO_BACKENDS.values()))
    for f in files:
        rp = rel(f)
        if not rp.startswith('middle/') or is_test_path(rp):
            continue
        hits, ex = count_hits(f, RE_MID_BACK)
        exempt_total += ex
        check('B4', rp, hits, KNOWN_MIDDLE_TO_BACKENDS, 'middle→backends 引用')
    print()

    # ---- B5 opcode 词表归属 ----
    print('== B5 opcode 词表唯一归属（库存：%s，%d 个常量）=='
          % (next(iter(KNOWN_OPCODE_DEF_FILES)), next(iter(KNOWN_OPCODE_DEF_FILES.values()))))
    for f in files:
        rp = rel(f)
        if not rp.startswith('backends/'):
            continue
        hits, ex = count_hits(f, RE_OPCODE_DEF)
        exempt_total += ex
        check('B5', rp, hits, KNOWN_OPCODE_DEF_FILES, 'opcode 常量定义')
    print()

    # ---- B6 parser 硬编码类型/谓词名 ----
    print('== B6 parser 内硬编码类型/谓词名（库存 %d 处，P6 消除）==' % sum(KNOWN_PARSER_HARDCODE.values()))
    for f in files:
        rp = rel(f)
        if not any(rp.startswith(d) for d in L2_DIRS) or is_test_path(rp):
            continue
        hits, ex = count_hits(f, RE_BUILTIN_LIT)
        exempt_total += ex
        check('B6', rp, hits, KNOWN_PARSER_HARDCODE, '内置名字面量')
    print()

    # ---- C pub(crate) 泄漏计数（纯基线）----
    print('== C pub(crate) 泄漏计数（基线，只许减不许增；P9 转硬）==')
    total = 0
    by_dir = defaultdict(int)
    for f in files:
        rp = rel(f)
        n = len(RE_PUB_CRATE.findall(read(f)))
        if n:
            top = '/'.join(rp.split('/')[:2]) if '/' in rp else rp
            by_dir[top] += n
            total += n
    print('  总数：%d' % total)
    for d, n in sorted(by_dir.items(), key=lambda kv: -kv[1]):
        print('  %s: %d' % (d, n))
    print()

    print('== 汇总 ==')
    print('豁免（// reason:）：%d 处' % exempt_total)
    print('违规：%d 处' % len(violations))
    for v in violations:
        print('  ' + v)
    print('模式：%s' % ('--strict' if strict else 'report-only（P0 引入，退出码恒 0；P9 转硬改用 --strict）'))
    return 1 if (strict and violations) else 0


if __name__ == '__main__':
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    sys.exit(main())
