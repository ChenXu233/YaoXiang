#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-fanout.py —— 禁令三（该重构不补丁）门禁：同义重复 / 扇出 / 入口接线

规则本体：docs/src/dev/coding-rules.md 第一部分·禁令三（判据 A/B/C）
入口清单权威：RFC-039 §1.1（5 个手工接线入口的 11 处不一致）
任务来源：docs/src/dev/architecture/09-execution-wbs.md §P0 任务 0.2.2
引入阶段：P0（report-only，默认退出码恒 0）；P9 统一转硬（CI 改为 --strict）。

判据实现：
  C. 入口接线（主检查）   函数体内直接调用 ≥2 个不同阶段原语（词法/语法/类型
                         检查/证明/死代码/单态化/IR/代码生成/执行）即判定为
                         「手工接线的入口」。现状库存登记在 KNOWN_WIRING_SITES
                         （2026-10-06 实测，对应 RFC-039 §1.1 的 5 入口 +
                         LSP/eval/embedded 等），P4 统一 Driver 后清零。
                         新增未登记的接线点 → 违规（验收：故意新增第 6 个
                         入口式接线必须红）。测试目录豁免。
  A. 同义重复（报告）     同名函数定义出现在 ≥2 个生产文件中（启发式；
                         变体后缀规避如 name_used_as_type_in 由 review 兜底，
                         已登记于 01 路由表 C）。常见助手名按停用表过滤。
  B. 扇出（报告/可选 diff） 默认打印概念族触点文件数基线；
                         --diff <git-ref> 模式下，改动文件集与任一概念族
                         交集 ≥3 → 疑似「一次修改同步改 ≥3 处同义映射」。

退出码：0 = report-only（默认）；--strict 且违规 > 0 → 1（P9 启用）。
"""
import io
import os
import re
import subprocess
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(ROOT, 'src')

# 阶段原语标记（按现状代码的调用形态标定；P4 统一 Driver 后本表随库存一起删除）
STAGE_MARKERS = {
    'LEX': re.compile(r'\btokenize\s*\(|Lexer::new|\brun_lexing\b'),
    'PARSE': re.compile(r'parser::parse\(|\bparse_module\(|\brun_parsing\b|\bparse_file\(|Parser::new|\bparse\(&'),
    'TC': re.compile(r'check_module|TypeChecker::new|\brun_typecheck\b|check_source_in_project|\bcheck_project\b|check_single_file'),
    'PROOF': re.compile(r'run_proof_execution|proof_execution'),
    'DEADCODE': re.compile(r'dead_code|DeadCode'),
    'MONO': re.compile(r'[Mm]onomorph'),
    'IRGEN': re.compile(r'AstToIrGenerator|run_ir_generation|generate_module_ir|generate_ir|ir_gen::'),
    'CODEGEN': re.compile(r'CodegenContext|BytecodeModule::from|BytecodeFile|Translator::new'),
    'EXEC': re.compile(r'Interpreter::new|execute_module|\.execute\('),
}

# 现状接线点库存（2026-10-06 实测登记；P4 统一 Driver 后清零）
# 形态：{(文件, 函数名): 阶段集合}；未登记的接线点 = 违规
KNOWN_WIRING_SITES = {
    # RFC-039 §1.1 五入口及其阶段包装方法（P4 统一 Driver 时清零）
    ('lib.rs', 'eval_code'): {'LEX', 'PARSE'},
    ('lib.rs', 'run_with_source_name'): {'CODEGEN', 'EXEC'},
    ('lib.rs', 'run_project'): {'CODEGEN', 'EXEC'},
    ('frontend/pipeline.rs', 'run'): {'LEX', 'PARSE', 'TC', 'PROOF', 'IRGEN'},
    ('frontend/pipeline.rs', 'run_typecheck'): {'TC', 'DEADCODE'},
    ('frontend/pipeline.rs', 'run_ir_generation'): {'IRGEN', 'MONO'},
    # 2026-10-06 P3：证明执行机制共享化，从 pipeline.rs 移入 proof_execution.rs
    #（orchestrator 四入口同调，02 §漏洞修复——库存随之迁移而非新增）
    ('frontend/proof_execution.rs', 'execute_single_proof_fn'): {'CODEGEN', 'EXEC', 'IRGEN'},
    ('frontend/validate.rs', 'validate_source'): {'LEX', 'PARSE', 'TC'},
    # 2026-10-06 P3：四入口补齐 proof_execution 消费点（PROOF 阶段入列）
    ('frontend/module/orchestrator.rs', 'compile_project'): {'PARSE', 'TC', 'PROOF', 'IRGEN'},
    ('frontend/module/orchestrator.rs', 'check_project'): {'PARSE', 'TC', 'DEADCODE', 'PROOF'},
    ('frontend/module/orchestrator.rs', 'check_source_in_project'): {'LEX', 'PARSE', 'TC', 'PROOF'},
    ('frontend/module/orchestrator.rs', 'collect_project_refs'): {'LEX', 'PARSE', 'DEADCODE'},
    ('frontend/module/orchestrator.rs', 'compile_embedded_module'): {'PARSE', 'TC', 'PROOF', 'IRGEN'},
    ('frontend/module/orchestrator.rs', 'parse_file'): {'LEX', 'PARSE'},
    # LSP 手工阶段序列（P4 的 4.2.6 消除）
    ('lsp/server.rs', 'update_semantic_db'): {'LEX', 'PARSE', 'TC'},
    ('lsp/handlers/completion.rs', 'document_symbol_items'): {'LEX', 'PARSE'},
    ('lsp/handlers/diagnostics.rs', 'run_diagnostics'): {'LEX', 'PARSE', 'TC'},
    # REPL 与嵌入 std（归 P4 ProgramKind / 06 D 节裁决）
    ('repl/eval.rs', 'evaluate'): {'CODEGEN', 'EXEC'},
    ('std/yx_sources.rs', 'embedded_std_module_info'): {'LEX', 'PARSE', 'TC'},
}

# 判据 A 停用表：通用助手名不参与同名重复报告
A_STOPLIST = {
    'new', 'default', 'from', 'fmt', 'clone', 'drop', 'eq', 'hash', 'build',
    'run', 'get', 'set', 'len', 'name', 'span', 'check', 'parse', 'test',
    'main', 'init', 'push', 'pop', 'insert', 'remove', 'contains', 'iter',
    'next', 'reset', 'clear', 'is_empty', 'to_string', 'as_str', 'ok', 'err',
}

# 判据 B 概念族（标识符词边界；文件集运行时计算）
CONCEPT_FAMILIES = {
    'BinOp 家族': re.compile(r'\bBinOp\b'),
    'UnOp 家族': re.compile(r'\bUnOp\b'),
    'MonoType': re.compile(r'\bMonoType\b'),
    'ConstValue': re.compile(r'\bConstValue\b'),
    'Literal': re.compile(r'\bLiteral\b'),
    'bytecode 运算符（BinaryOp/UnaryOp/CompareOp）': re.compile(r'\b(BinaryOp|UnaryOp|CompareOp)\b'),
}
B_FANOUT_MIN = 3

FN_RE = re.compile(r'\bfn\s+([A-Za-z_]\w*)')
CHAR_LIT_RE = re.compile(r"^'(\\\\.|[^\\\\'])" + "'")


def sanitize(line):
    """剥掉行注释与字符串/字符/原始字符串字面量（花括号可能藏在这些地方，
    orchestrator.rs:1312 的注释 `{x}` 曾把函数边界检测整段吞掉）。
    局限：跨行块注释 /* */ 不处理（本仓以 // 文档注释为主）。"""
    out = []
    i = 0
    n = len(line)
    while i < n:
        c = line[i]
        if c == '/' and i + 1 < n and line[i + 1] == '/':
            break
        if c == 'r' and i + 1 < n and line[i + 1] == '"':
            i += 2
            while i < n and line[i] != '"':
                i += 1
            i += 1
            continue
        if c == 'r' and i + 2 < n and line[i + 1] == '#' and line[i + 2] == '"':
            i += 3
            while i + 1 < n and not (line[i] == '"' and line[i + 1] == '#'):
                i += 1
            i += 2
            continue
        if c == '"':
            i += 1
            while i < n and line[i] != '"':
                i += 2 if line[i] == '\\' else 1
            i += 1
            continue
        if c == "'":
            m = CHAR_LIT_RE.match(line[i:])
            if m:
                i += len(m.group(0))
                continue
            # 生命周期 'a：保留（不影响花括号计数）
            out.append(c)
            i += 1
            continue
        out.append(c)
        i += 1
    return ''.join(out)


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


def fn_ranges(lines):
    """产出 (函数名, 起始行, 结束行)。签名可跨行；以首个 { 起计深度；
    注释/字符串先经 sanitize 剥离。"""
    out = []
    name = None
    start = 0
    depth = 0
    opened = False
    for i, raw in enumerate(lines):
        line = sanitize(raw)
        if name is None:
            m = FN_RE.search(line)
            if not m:
                continue
            name = m.group(1)
            start = i
            depth = 0
            opened = False
        depth += line.count('{') - line.count('}')
        if '{' in line:
            opened = True
        if opened and depth <= 0:
            out.append((name, start, i))
            name = None
    return out


def wiring_sites():
    """函数内 ≥2 个不同阶段标记 → [(file, line, fn, stages)]。测试目录豁免。"""
    sites = []
    for f in rs_files():
        rp = rel(f)
        if is_test_path(rp):
            continue
        lines = read(f).split('\n')
        for name, s, e in fn_ranges(lines):
            body = '\n'.join(sanitize(x) for x in lines[s:e + 1])
            stages = sorted(k for k, rx in STAGE_MARKERS.items() if rx.search(body))
            if len(stages) >= 2:
                sites.append((rp, s + 1, name, stages))
    return sites


def main():
    strict = '--strict' in sys.argv[1:]
    diff_ref = None
    for i, a in enumerate(sys.argv[1:]):
        if a == '--diff' and i + 2 <= len(sys.argv[1:]):
            diff_ref = sys.argv[i + 2]

    files = sorted(rs_files())
    violations = []

    print('check-fanout: 禁令三（该重构不补丁）门禁报告%s' % (' [STRICT]' if strict else '（report-only）'))
    print('扫描范围: src/（%d 个 .rs）\n' % len(files))

    # ---- C 入口接线 ----
    sites = wiring_sites()
    print('== C 入口接线点（函数内跨 ≥2 阶段手工接线；库存 %d 处，P4 清零）==' % len(KNOWN_WIRING_SITES))
    for rp, line, name, stages in sites:
        key = (rp, name)
        if key in KNOWN_WIRING_SITES:
            print('  [库存] %s:%d %s  [%s]' % (rp, line, name, '/'.join(stages)))
        else:
            violations.append('%s:%d %s  新增入口接线 [%s]' % (rp, line, name, '/'.join(stages)))
            print('  [违规] %s:%d %s  [%s]' % (rp, line, name, '/'.join(stages)))
    for (rp, name) in KNOWN_WIRING_SITES:
        if not any((s[0], s[2]) == (rp, name) for s in sites):
            print('  [进展] %s %s 已消失（可从库存移除）' % (rp, name))
    print()

    # ---- A 同名函数多定义 ----
    print('== A 同名函数多定义（生产文件，启发式，人裁决）==')
    fn_defs = defaultdict(set)  # name -> files
    for f in files:
        rp = rel(f)
        if is_test_path(rp):
            continue
        for m in re.finditer(r'^\s*(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_]\w*)', read(f), re.M):
            fn_defs[m.group(1)].add(rp)
    dups = {n: fs for n, fs in fn_defs.items() if len(fs) >= 2 and n not in A_STOPLIST and len(n) >= 4}
    for n, fs in sorted(dups.items()):
        print('  %s: %d 个文件 → %s' % (n, len(fs), '、'.join(sorted(fs))))
    print('  （合计 %d 个同名定义；同名 ≠ 必违规，语义重复由 review 裁决）' % len(dups))
    print()

    # ---- B 扇出 ----
    print('== B 概念族触点基线（扇出事实；--diff 模式下 ≥%d 个同族文件被改 → 疑似扇出）==' % B_FANOUT_MIN)
    family_files = {}
    for fam, rx in CONCEPT_FAMILIES.items():
        s = set()
        for f in files:
            if rx.search(read(f)):
                s.add(rel(f))
        family_files[fam] = s
        print('  %s: %d 个文件' % (fam, len(s)))
    if diff_ref:
        changed = subprocess.run(
            ['git', 'diff', '--name-only', diff_ref, '--', 'src/'],
            cwd=ROOT, capture_output=True, text=True,
        ).stdout.split()
        changed = {c.replace('\\', '/').removeprefix('src/') for c in changed}
        print('\n  --diff %s：改动 %d 个 src 文件' % (diff_ref, len(changed)))
        for fam, fs in family_files.items():
            inter = changed & fs
            if len(inter) >= B_FANOUT_MIN:
                msg = '扇出疑似：%s 命中 %d 个改动文件（%s）' % (fam, len(inter), '、'.join(sorted(inter)[:6]))
                violations.append(msg)
                print('  [违规] ' + msg)
    print()

    print('== 汇总 ==')
    print('接线点实测 %d 处（库存 %d）' % (len(sites), len(KNOWN_WIRING_SITES)))
    print('违规：%d 处' % len(violations))
    for v in violations:
        print('  ' + v)
    print('模式：%s' % ('--strict' if strict else 'report-only（P0 引入，退出码恒 0；P9 转硬改用 --strict）'))
    return 1 if (strict and violations) else 0


if __name__ == '__main__':
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    sys.exit(main())
