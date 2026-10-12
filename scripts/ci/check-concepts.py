#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-concepts.py —— 禁令一（不得生造）门禁：平行表示 / 消歧别名 / 同义词表 / 调用点不足

规则本体：docs/src/dev/coding-rules.md 第一部分·禁令一（判据 A/B/C/D）
相似度阈值：RFC-039 决议登记 D43（判据 A 用 Jaccard ≥ 0.5）
任务来源：docs/src/dev/architecture/09-execution-wbs.md §P0 任务 0.2.1
引入阶段：P0（report-only，默认退出码恒 0）；P9 统一转硬（CI 改为 --strict）。

判据实现：
  A. 职责重叠   全仓 pub enum 变体名两两比对，分两层报告：
                - 违规层：Jaccard ≥ 0.5（D43）且双向皆无 From/TryFrom 转换；
                - 疑似层：变体名重合 ≥ 半数（overlap = |∩|/min(|A|,|B|) ≥ 0.5，
                  禁令一 A 的原文表述）但未落入违规层。「职责重叠」一半由人裁决
                  （08 §已知局限：门禁只报疑似）。
                另检「别名再导出」：pub use 把被检出平行枚举以另一路径/名字再导出
                （WBS 0.2.1 验收要求 ir::Type 别名一并计入）。
  B. 调用点不足  pub struct/enum 的名字在定义文件以外的文件中出现 < 2 个文件
                （以「使用文件数」近似调用点；启发式，report-only，人裁决）。
  C. 消歧别名    use ... as ... 把判据 A 检出的平行概念改名引入 → 违规；
                其余 use-as 重命名仅计数（08 机器清单：统计数量，豁免附
                // reason: 注释）。
  D. 同义词表    match 臂含 ≥2 个字符串字面量分支（"a" | "b" =>），
                即手写字符串匹配弥合差异；按所在函数聚合报告。

豁免：命中行或其上一行含 `// reason:` 注释 → 豁免（数量单列，
见 coding-rules 第五部分）。

已知局限（与 08 §已知局限一致）：
  - 纯文本扫描，不展开宏；宏生成的枚举/impl 会漏检。
  - 标识符索引含注释与字符串字面量中的 token（判据 B 因此偏宽松）。
  - 多行 match 臂最多回溯 12 行拼接。

退出码：0 = report-only（默认）；--strict 且违规层命中 > 0 → 1（P9 启用）。
"""
import io
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(ROOT, 'src')

# D43：禁令一 A 判据的相似度阈值（Jaccard）
JACCARD_VIOLATION = 0.5
# 禁令一 A 原文表述「变体名重合 ≥ 半数」：|∩| / min(|A|,|B|)
OVERLAP_SUSPECT = 0.5
# 疑似层的最小枚举规模：1 变体枚举重合恒为 1，纯噪声
MIN_VARIANTS = 2
# 判据 D：单个 match 臂至少含几个字符串字面量分支才算「同义词表」
SYNONYM_MIN_STRINGS = 2
# 多行 match 臂拼接的最大回溯行数
SYNONYM_MAX_LOOKBACK = 12

# 汇总层用的运算符枚举名集合（仅用于把检出结果归类成「套」，不参与检测）
OPERATOR_ENUM_NAMES = {'BinOp', 'UnOp', 'BinaryOp', 'UnaryOp', 'CompareOp'}
# 汇总层用的类型表示枚举名集合（同上）
TYPE_ENUM_NAMES = {'Type', 'MonoType'}

ENUM_RE = re.compile(r'^\s*pub(?:\(crate\))?\s+enum\s+([A-Za-z_]\w*)')
STRUCT_RE = re.compile(r'^\s*pub(?:\(crate\))?\s+struct\s+([A-Za-z_]\w*)')
USE_RE = re.compile(r'^\s*(pub\s+)?use\s')
ALIAS_RE = re.compile(r'([A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)\s+as\s+([A-Za-z_]\w*)')
CONV_RE = re.compile(r'\b(From|TryFrom)\s*<([^>]*)>\s*for\s+([A-Za-z_][\w:]*)')
STR_LIT_RE = re.compile(r'"(?:[^"\\]|\\.)*"')
FN_RE = re.compile(r'\bfn\s+([A-Za-z_]\w*)')
IDENT_RE = re.compile(r'[A-Za-z_]\w*')


def external_crates():
    """Cargo.toml 依赖表（D0：依赖表的权威源是 Cargo.toml）→ crate 名集合。

    use 路径首段命中依赖名 → 外部 crate 重命名（如 lsp_types::X as Y），
    不属于判据 C 的「区分仓库内同语义项」。std/core/alloc 同理。
    """
    names = {'std', 'core', 'alloc'}
    try:
        text = read(os.path.join(ROOT, 'Cargo.toml'))
    except OSError:
        return names
    in_deps = False
    for line in text.split('\n'):
        s = line.strip()
        if s.startswith('['):
            header = s.strip('[]').strip()
            in_deps = header == 'dependencies' or header.endswith('.dependencies')
            continue
        if in_deps:
            m = re.match(r'^([A-Za-z0-9_-]+)\s*=', s)
            if m:
                names.add(m.group(1).replace('-', '_'))
    return names


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
    return os.path.relpath(p, ROOT).replace('\\', '/')


def has_reason(lines, idx):
    """命中行或上一行带 // reason: 注释 → 豁免（coding-rules 第五部分）。"""
    for j in (idx, idx - 1):
        if 0 <= j < len(lines) and '// reason:' in lines[j]:
            return True
    return False


def parse_enums(path, lines):
    """提取 pub enum / pub(crate) enum 的 (名字, 行号, 变体名列表)。"""
    out = []
    i = 0
    n = len(lines)
    while i < n:
        m = ENUM_RE.match(lines[i])
        if not m:
            i += 1
            continue
        name = m.group(1)
        base_indent = len(lines[i]) - len(lines[i].lstrip())
        j = i
        while j < n and '{' not in lines[j]:
            j += 1
        if j >= n:
            i += 1
            continue
        variants = []
        after = lines[j].split('{', 1)[1]
        if '}' in after:
            # 单行枚举：pub enum X { A, B, C }
            inner = after.split('}', 1)[0]
            for tok in inner.split(','):
                tok = tok.strip()
                if tok and tok[0].isupper():
                    variants.append(re.match(r'([A-Za-z_]\w*)', tok).group(1))
            out.append({'name': name, 'line': i + 1, 'variants': variants, 'file': path})
            i = j + 1
            continue
        variant_indent = None
        k = j + 1
        while k < n:
            line = lines[k]
            stripped = line.strip()
            if not stripped or stripped.startswith('//') or stripped.startswith('#'):
                k += 1
                continue
            indent = len(line) - len(line.lstrip())
            if stripped.startswith('}') and indent <= base_indent:
                break
            # 变体行：首字母大写且与首个变体同缩进（结构体变体的字段更深一层）
            if stripped[0].isupper():
                if variant_indent is None:
                    variant_indent = indent
                if indent == variant_indent:
                    variants.append(re.match(r'([A-Za-z_]\w*)', stripped).group(1))
            k += 1
        out.append({'name': name, 'line': i + 1, 'variants': variants, 'file': path})
        i = max(k, j + 1)
    return out


def iter_use_statements(lines):
    """产出 (起始行号, 完整语句文本)，拼接多行 use。"""
    i = 0
    n = len(lines)
    while i < n:
        if USE_RE.match(lines[i]):
            start = i
            stmt = [lines[i]]
            while not lines[i].rstrip().endswith(';') and i + 1 < n:
                i += 1
                stmt.append(lines[i])
            yield start, '\n'.join(stmt)
        i += 1


def pub_use_items(stmt):
    """从 pub use 语句提取 (item 末段名, alias 或 None)。只展开一层花括号。"""
    body = re.sub(r'^\s*pub\s+use\s+', '', stmt).rstrip().rstrip(';')
    if '{' in body and '}' in body:
        inner = body[body.index('{') + 1: body.rindex('}')]
        parts = [p.strip() for p in inner.split(',') if p.strip()]
    else:
        parts = [body.strip()]
    items = []
    for p in parts:
        m = re.match(r'^([A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)\s+as\s+([A-Za-z_]\w*)$', p)
        if m:
            items.append((m.group(1).split('::')[-1], m.group(2)))
            continue
        m = re.match(r'^([A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)$', p)
        if m:
            items.append((p.split('::')[-1], None))
    return items


def find_conversions(text):
    """impl From<X> for Y / impl TryFrom<X> for Y → [(trait, X 末段, Y 末段)]。"""
    out = []
    for m in CONV_RE.finditer(text):
        srcs = re.findall(r'[A-Za-z_]\w*', m.group(2))
        if srcs:
            out.append((m.group(1), srcs[-1], m.group(3).split('::')[-1]))
    return out


def enclosing_fn(lines, idx):
    for j in range(idx, max(-1, idx - 400), -1):
        m = FN_RE.search(lines[j])
        if m:
            return m.group(1)
    return '<unknown>'


def find_synonym_arms(lines):
    """match 臂含 ≥2 个字符串字面量分支 → [(起始行, 结束行, 字面量数)]。"""
    hits = []
    pending = []
    for idx, line in enumerate(lines):
        if '=>' in line:
            text = ' '.join(t for _, t in pending) + ' ' + line
            start = pending[0][0] if pending else idx
            pending = []
            head = text.split('=>', 1)[0]
            lits = STR_LIT_RE.findall(head)
            if len(lits) >= SYNONYM_MIN_STRINGS and '|' in head:
                hits.append((start, idx, len(lits)))
        elif '|' in line and STR_LIT_RE.search(line):
            pending.append((idx, line))
            if len(pending) > SYNONYM_MAX_LOOKBACK:
                pending = pending[-SYNONYM_MAX_LOOKBACK:]
        elif line.strip() and not line.strip().startswith('//'):
            pending = []
    return hits


def main():
    strict = '--strict' in sys.argv[1:]

    files = sorted(rs_files())
    lines_of = {}
    text_of = {}
    for f in files:
        text_of[f] = read(f)
        lines_of[f] = text_of[f].split('\n')

    # ---------- 收集事实 ----------
    enums = []
    structs = []  # (name, file, line_idx)
    conversions = []  # (trait, src, tgt)
    alias_map = {}  # 再导出别名 → 原名（pub use X as Y）
    use_stmts = []  # (file, start_idx, stmt, is_pub)
    for f in files:
        lines = lines_of[f]
        enums.extend(parse_enums(f, lines))
        for idx, line in enumerate(lines):
            m = STRUCT_RE.match(line)
            if m:
                structs.append((m.group(1), f, idx))
        conversions.extend(find_conversions(text_of[f]))
        for start, stmt in iter_use_statements(lines):
            is_pub = bool(re.match(r'^\s*pub\s+use\s', stmt))
            use_stmts.append((f, start, stmt, is_pub))
            if is_pub:
                for item, alias in pub_use_items(stmt):
                    if alias:
                        alias_map[alias] = item

    def norm(name):
        seen = set()
        while name in alias_map and name not in seen:
            seen.add(name)
            name = alias_map[name]
        return name

    def has_conversion(a, b):
        for _trait, src, tgt in conversions:
            if {norm(src), norm(tgt)} == {a, b}:
                return True
        return False

    # ---------- 判据 A：平行枚举 ----------
    pairs = []  # (tier, a, b, jaccard, overlap, inter, has_conv)
    for i in range(len(enums)):
        for j in range(i + 1, len(enums)):
            a, b = enums[i], enums[j]
            if len(a['variants']) < MIN_VARIANTS or len(b['variants']) < MIN_VARIANTS:
                continue
            va, vb = set(a['variants']), set(b['variants'])
            inter = len(va & vb)
            if inter == 0:
                continue
            jac = inter / len(va | vb)
            ov = inter / min(len(va), len(vb))
            # 疑似层要求至少 2 个变体名重合：单名巧合（2 变体枚举撞 1 个名）是纯噪声
            if jac < JACCARD_VIOLATION and not (ov >= OVERLAP_SUSPECT and inter >= 2):
                continue
            conv = has_conversion(a['name'], b['name'])
            tier = 'violation' if (jac >= JACCARD_VIOLATION and not conv) else 'suspect'
            pairs.append((tier, a, b, jac, ov, inter, conv))

    parallel_names = set()
    for _tier, a, b, _jac, _ov, _inter, _conv in pairs:
        parallel_names.add(a['name'])
        parallel_names.add(b['name'])

    # 判据 A 补充：别名再导出（把平行概念以另一路径/名字再导出）
    # 豁免桶文件（mod.rs / lib.rs / main.rs）中的同名再导出——那是 Rust 惯用的
    # barrel 模式，不是别名；改名再导出（X as Y）与非桶文件中的再导出才计入。
    BARREL_FILES = {'mod.rs', 'lib.rs', 'main.rs'}
    reexports = []  # (file, line, item, alias)
    for f, start, stmt, is_pub in use_stmts:
        if not is_pub:
            continue
        is_barrel = os.path.basename(f) in BARREL_FILES
        for item, alias in pub_use_items(stmt):
            if item in parallel_names:
                if is_barrel and alias is None:
                    continue
                reexports.append((f, start + 1, item, alias))

    # ---------- 判据 C：消歧别名 ----------
    ext_crates = external_crates()
    c_violations = []  # (file, line, item, alias)
    c_total = 0
    c_exempt = 0
    c_external = 0
    for f, start, stmt, _is_pub in use_stmts:
        lines = lines_of[f]
        # 花括号列表内的别名会丢失 use 路径前缀，先从语句头取首段
        head = re.match(r'^\s*(?:pub\s+)?use\s+([A-Za-z_]\w*)', stmt)
        stmt_first = head.group(1) if head else ''
        for m in ALIAS_RE.finditer(stmt):
            path = m.group(1)
            item = path.split('::')[-1]
            alias = m.group(2)
            if alias == '_' or alias == item:
                continue
            c_total += 1
            first_seg = path.split('::', 1)[0]
            if first_seg in ext_crates or stmt_first in ext_crates:
                # 对外部 crate 类型的重命名（Position as LspPosition 等），仅计数
                c_external += 1
                continue
            if has_reason(lines, start):
                c_exempt += 1
                continue
            if norm(item) in parallel_names:
                c_violations.append((f, start + 1, item, alias))

    # ---------- 判据 D：同义词表 ----------
    d_hits = []  # (file, fn_name, start_line, end_line, n_literals)
    for f in files:
        lines = lines_of[f]
        for start, end, nlits in find_synonym_arms(lines):
            if has_reason(lines, end):
                continue
            d_hits.append((f, enclosing_fn(lines, start), start + 1, end + 1, nlits))

    # ---------- 判据 B：调用点不足（使用文件数 < 2） ----------
    ident_files = defaultdict(set)
    for f in files:
        for tok in set(IDENT_RE.findall(text_of[f])):
            ident_files[tok].add(f)
    b_suspects = []  # (kind, name, file, line, n_usage_files)
    b_exempt = 0
    for e in enums:
        usage = len(ident_files.get(e['name'], set()) - {e['file']})
        if usage < 2:
            if has_reason(lines_of[e['file']], e['line'] - 1):
                b_exempt += 1
            else:
                b_suspects.append(('enum', e['name'], e['file'], e['line'], usage))
    for name, f, idx in structs:
        usage = len(ident_files.get(name, set()) - {f})
        if usage < 2:
            if has_reason(lines_of[f], idx):
                b_exempt += 1
            else:
                b_suspects.append(('struct', name, f, idx + 1, usage))

    # ---------- 报告 ----------
    w = print
    w('check-concepts: 禁令一（不得生造）门禁报告%s' % (' [STRICT]' if strict else '（report-only）'))
    w('扫描范围: %s（%d 个 .rs）' % (rel(SRC) + '/', len(files)))
    w('')

    n_viol = 0

    w('== 判据 A：平行枚举（变体名重合）==')
    a_viol = [p for p in pairs if p[0] == 'violation']
    a_susp = [p for p in pairs if p[0] == 'suspect']
    if a_viol:
        w('违规（Jaccard ≥ %.1f 且无 From/TryFrom，D43）：' % JACCARD_VIOLATION)
        for _t, a, b, jac, ov, inter, _c in a_viol:
            w('  %s（%s:%d）× %s（%s:%d）  J=%.2f overlap=%.2f 变体 %d/%d 重合 %d'
              % (a['name'], rel(a['file']), a['line'], b['name'], rel(b['file']), b['line'],
                 jac, ov, len(a['variants']), len(b['variants']), inter))
    else:
        w('违规：无')
    if a_susp:
        w('疑似（重合 ≥ 半数但未达 D43 阈值或已有转换，人裁决）：')
        for _t, a, b, jac, ov, inter, conv in a_susp:
            note = '有 From/TryFrom' if conv else '无 From/TryFrom'
            w('  %s（%s:%d）× %s（%s:%d）  J=%.2f overlap=%.2f 重合 %d，%s'
              % (a['name'], rel(a['file']), a['line'], b['name'], rel(b['file']), b['line'],
                 jac, ov, inter, note))
    else:
        w('疑似：无')
    if reexports:
        w('别名再导出（平行表示的别名路径，一并计入）：')
        for f, line, item, alias in reexports:
            if alias:
                w('  %s:%d  pub use …::%s as %s（%s 是 %s 的别名）' % (rel(f), line, item, alias, alias, item))
            else:
                w('  %s:%d  pub use …::%s（同名单路径再导出）' % (rel(f), line, item))
    n_viol += len(a_viol) + len(reexports)
    w('')

    w('== 判据 C：消歧别名（use … as … 区分同语义项）==')
    if c_violations:
        w('违规（被改名项属于判据 A 检出的平行概念）：')
        for f, line, item, alias in c_violations:
            w('  %s:%d  %s as %s' % (rel(f), line, item, alias))
    else:
        w('违规：无')
    w('use-as 重命名合计 %d 处（外部 crate 改名 %d 处仅计数；豁免 %d 处）。' % (c_total, c_external, c_exempt))
    n_viol += len(c_violations)
    w('')

    w('== 判据 D：同义词表（字符串字面量分支弥合）==')
    if d_hits:
        by_fn = defaultdict(list)
        for f, fn, s, e, nlits in d_hits:
            by_fn[(f, fn)].append((s, e, nlits))
        for (f, fn), arms in sorted(by_fn.items()):
            w('  %s  %s()：%d 个字符串别名臂' % (rel(f), fn, len(arms)))
            for s, e, nlits in arms:
                w('    L%d-%d（%d 个字面量）' % (s, e, nlits))
    else:
        w('无')
    n_viol += len(d_hits)
    w('')

    w('== 判据 B：调用点不足（pub 项的使用文件数 < 2，启发式，人裁决）==')
    if b_suspects:
        for kind, name, f, line, usage in sorted(b_suspects, key=lambda x: (x[4], x[2], x[3])):
            w('  %s %s（%s:%d）  定义外使用文件数 = %d' % (kind, name, rel(f), line, usage))
    else:
        w('无')
    w('豁免（// reason:）：%d 处' % b_exempt)
    w('')

    # ---------- 汇总 ----------
    # 并查集聚类被检出的枚举，按定义文件去重计「套」
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(x, y):
        parent[find(x)] = find(y)

    for _t, a, b, _j, _o, _i, _c in pairs:
        union(id(a), id(b))
    clusters = defaultdict(list)
    for e in enums:
        if id(e) in parent:
            clusters[find(id(e))].append(e)

    op_files = set()
    type_files = set()
    for members in clusters.values():
        names = {m['name'] for m in members}
        if names & OPERATOR_ENUM_NAMES:
            op_files.update(m['file'] for m in members if m['name'] in OPERATOR_ENUM_NAMES)
        if names & TYPE_ENUM_NAMES:
            type_files.update(m['file'] for m in members if m['name'] in TYPE_ENUM_NAMES)

    w('== 汇总 ==')
    w('检出平行枚举簇 %d 个：' % len(clusters))
    for members in clusters.values():
        desc = '，'.join('%s(%s:%d)' % (m['name'], rel(m['file']), m['line']) for m in members)
        w('  - %s' % desc)
    w('运算符枚举：%d 套（按定义文件计：%s）'
      % (len(op_files), '、'.join(sorted(rel(f) for f in op_files)) if op_files else '无'))
    w('平行类型表示：%d 套（按定义文件计：%s）；别名再导出 %d 处'
      % (len(type_files), '、'.join(sorted(rel(f) for f in type_files)) if type_files else '无',
         len(reexports)))
    w('违规层命中合计：%d（A 平行枚举 %d + A 别名再导出 %d + C 消歧别名 %d + D 同义词表臂 %d）'
      % (n_viol, len(a_viol), len(reexports), len(c_violations), len(d_hits)))
    if strict:
        w('模式：--strict')
    else:
        w('模式：report-only（P0 引入，退出码恒 0；P9 转硬后改用 --strict，违规层命中将使 CI 失败）')

    if strict and n_viol > 0:
        return 1
    return 0


if __name__ == '__main__':
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    sys.exit(main())
