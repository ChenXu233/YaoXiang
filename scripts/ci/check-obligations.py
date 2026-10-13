#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-obligations.py —— 义务字段静态门禁（WBS 9.2 / 02-stage-contract §2）

9.2 的原始判据是「字段出现 ≥2 次（定义 + 写入）但无第三文件读取 → 失败」。
本脚本在它之上补齐三条硬判据——2026-10-13 用户裁决（机制甲）明确要求：
交付标记只是声明，「标记 ↔ 该字段在消费端真被读到」必须做成硬判据，
否则标记与真实读取一旦漂移，运行期（assert_drained）看不见。

判据（任一违反 → 退出码 1）：

  1. 清单对账：三处逐字一致（义务条数以 02 §义务字段表表头为准）
       a) src/driver/obligations.rs 的 INVENTORY（字段名 / 消费者阶段 / 红名单）
       b) src/frontend/core/typecheck/types.rs 的 TypeCheckResult 字段定义
       c) docs/src/dev/architecture/02-stage-contract.md §义务字段表
     反向 containment（F2）：TypeCheckResult 的字段集必须恰好等于「INVENTORY ∪
     NON_OBLIGATION_FIELDS（显式登记，含理由）」——两端都不许有未登记字段，
     并通过「字段数 − 义务数 − 非义务数 = 0」的差值显式输出。
     F1 锚点校验：登记理由与证据指针里的 file:line 必须可复核（文件存在、
     行号在册、锚点后紧跟的符号必须真的出现在被锚文件里）——防「指针悄悄
     指到不存在的符号」这种纯文本锚点的陈旧形态。
     F4 表格锚点：02 §义务字段表的「定义行」列必须等于 types.rs 的字段声明行
     （表注约定），防两组行号各自为政。
  2. 读出证据：非红名单义务必须在声明的消费者文件里有真读取
       （成员访问 .字段名；注释、测试代码、字符串不算）
       —— 这是「定义 + 写入 + 真消费」的 9.2 判据强化版
  3. 交付标记：非红名单义务必须在 src/driver/arms.rs 的交付点被标记
       （deliver(Obligation::X) 或 deliver_all(IR_GENERATION_DELIVERED)）；
       红名单义务反之不得被标记（标记了说明红名单已陈旧）
  4. 红名单陈旧检测：红名单义务必须在 typecheck 树 + 本清单之外零真读取；
       每条必须带 WBS 任务 id 与「消费者为零」的证据指针
       （唯一允许的清单外读取点是 obligations.rs 自己的空义务探针 is_produced）
  5. 反例自检（--selftest）：自带七类夹具（幽灵义务 / 正例·红名单为空 / 清单滞后 /
     结构体滞后 / 红名单陈旧 / 未登记字段 / 锚点失效）各按预期红绿

退出码：0 = 门禁通过（红名单条目按登记原样输出，不静默）；
        1 = 违反判据；2 = 脚本自身无法完成对账

用法：
  python scripts/ci/check-obligations.py                 # 对当前仓库跑门禁
  python scripts/ci/check-obligations.py --selftest      # 只跑夹具自检（六类）
  python scripts/ci/check-obligations.py --root <dir>    # 对指定仓库根跑（夹具）
  python scripts/ci/check-obligations.py --verbose
"""
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# 三处对账的路径（相对仓库根）
MANIFEST = 'src/driver/obligations.rs'
TYPES = 'src/frontend/core/typecheck/types.rs'
CONTRACT = 'docs/src/dev/architecture/02-stage-contract.md'
DELIVERY_SITE = 'src/driver/arms.rs'
TYPECHECK_TREE = 'src/frontend/core/typecheck'

# 消费者阶段 → 声明式消费者文件（判据 2 的扫描面）
CONSUMER_FILES = {
    'ProofExecution': ['src/driver/arms.rs', 'src/frontend/proof_execution.rs'],
    'IrGeneration': ['src/middle/core/ir_gen.rs'],
    'Monomorphization': ['src/driver/arms.rs', 'src/driver/helpers.rs'],
}

# 交付阶段全集（INVENTORY 的 consumer 必须落在这里之一）
DELIVERY_STAGES = ('ProofExecution', 'IrGeneration', 'Monomorphization')


def read(path):
    with io.open(path, 'r', encoding='utf-8', errors='replace') as f:
        return f.read()


def find(root, rel):
    """定位仓库根下的文件：支持夹具用的 .txt 变体（避免被 cargo/rustfmt 当目标）。"""
    for cand in (rel, rel + '.txt'):
        p = os.path.join(root, cand)
        if os.path.exists(p):
            return p
    return None


def is_char_literal(text, i):
    """判断 text[i] 的撇号是字符字面量（'x' / '\\n'）还是 Rust 生命周期（&'static）。

    生命周期没有右撇号，若当字符字面量处理会把后面整段代码吞进字符串态——
    这正是本判据早期版本漏判 escaped_refs 一类「读取在文件后段」的原因。
    """
    n = len(text)
    j = i + 1
    if j >= n:
        return False
    if text[j] == chr(92):          # '\n' 这类转义
        return j + 2 < n and text[j + 2] == "'"
    return j + 1 < n and text[j + 1] == "'"


def strip_comments(text):
    """去掉 // 行注释与 /* */ 块注释（字符串字面量内部不误伤）。"""
    out = []
    i, n = 0, len(text)
    state = 'code'   # code | line | block | str | char
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ''
        if state == 'code':
            if c == '/' and nxt == '/':
                state = 'line'
                i += 2
                continue
            if c == '/' and nxt == '*':
                state = 'block'
                i += 2
                continue
            if c == '"':
                state = 'str'
            elif c == "'" and is_char_literal(text, i):
                state = 'char'
            out.append(c)
            i += 1
            continue
        if state == 'line':
            if c == chr(10):
                state = 'code'
                out.append(c)
            i += 1
            continue
        if state == 'block':
            if c == '*' and nxt == '/':
                state = 'code'
                i += 2
                continue
            if c == chr(10):
                out.append(c)   # 保行号
            i += 1
            continue
        if state == 'str':
            if c == chr(92):
                i += 2
                continue
            if c == '"':
                state = 'code'
            i += 1
            continue
        if state == 'char':
            if c == chr(92):
                i += 2
                continue
            if c == "'":
                state = 'code'
            i += 1
            continue
    return ''.join(out)


def cut_test_sections(text):
    """截断到 #[cfg(test)] / mod tests 之前（测试代码不算消费证据）。"""
    cut = -1
    for marker in ('#[cfg(test)]', 'mod tests'):
        idx = text.find(marker)
        if idx != -1 and (cut == -1 or idx < cut):
            cut = idx
    return text if cut == -1 else text[:cut]


def parse_manifest(text):
    """解析 obligations.rs 的 INVENTORY 表。"""
    start = text.find('const INVENTORY')
    if start == -1:
        return None
    end = text.find(chr(10) + '];', start)
    if end == -1:
        return None
    block = text[start:end]
    entries = []
    for chunk in block.split('ObligationSpec {')[1:]:
        obligation = re.search(r'obligation:\s*Obligation::(\w+)', chunk)
        field = re.search(r'field:\s*"([^"]+)"', chunk)
        consumer = re.search(r'consumer:\s*Stage::(\w+)', chunk)
        if not (obligation and field and consumer):
            continue
        red = re.search(
            r'red_list:\s*Some\(RedListEntry\s*\{\s*wbs:\s*"([^"]+)",\s*evidence:\s*"([^"]+)",',
            chunk, re.S)
        entries.append({
            'variant': obligation.group(1),
            'field': field.group(1),
            'consumer': consumer.group(1),
            'wbs': red.group(1) if red else None,
            'evidence': red.group(2) if red else None,
        })
    return entries


def parse_non_obligation_fields(text):
    """解析 obligations.rs 的 NON_OBLIGATION_FIELDS 登记（字段名 + 理由）。

    判据 1 的第三处：与 INVENTORY 一起闭合 TypeCheckResult 的字段普查。
    """
    start = text.find('pub const NON_OBLIGATION_FIELDS')
    if start == -1:
        return None
    end = text.find(chr(10) + '];', start)
    if end == -1:
        return None
    block = text[start:end]
    entries = []
    for chunk in block.split('NonObligationField {')[1:]:
        field = re.search(r'field:\s*"([^"]+)"', chunk)
        reason = re.search(r'reason:\s*"([^"]+)"', chunk)
        if field:
            entries.append({
                'field': field.group(1),
                'reason': reason.group(1) if reason else '',
            })
    return entries


ANCHOR_RE = re.compile(
    r'([A-Za-z0-9_./-]+\.(?:rs|py|md|json|toml|yx)):(\d+)(?:-(\d+))?'
    r'(?:[ \t]+([A-Za-z_][A-Za-z0-9_.:]*))?')
CONT_ANCHOR_RE = re.compile(r'(?<![\w./:-]):(\d+)(?:-(\d+))?(?![\d-])')
FILE_MENTION_RE = re.compile(r'([A-Za-z0-9_./-]+\.(?:rs|py|md|json|toml|yx))')


def resolve_anchor_path(root, rel):
    """把锚点里的路径解析到仓库内：先原样、再 src/ 下、最后按唯一基名。"""
    parts = [os.path.join(root, rel), os.path.join(root, 'src', rel)]
    for cand in parts:
        for probe in (cand, cand + '.txt'):
            if os.path.isfile(probe):
                return probe
    base = os.path.basename(rel)
    found = []
    src_root = os.path.join(root, 'src')
    for dirpath, dirnames, filenames in os.walk(src_root):
        dirnames[:] = [d for d in dirnames if d not in ('.git', 'target')]
        if base in filenames:
            found.append(os.path.join(dirpath, base))
            if len(found) > 1:
                return None
        elif base + '.txt' in filenames:
            found.append(os.path.join(dirpath, base + '.txt'))
            if len(found) > 1:
                return None
    return found[0] if found else None


def verify_anchors(root, text, label):
    """F1：登记/证据里的 file:line 锚点必须可复核——文件存在、行号在册、
    锚点后紧跟的符号（若有）必须在被锚文件里出现。"""
    violations = []
    matched_spans = []
    anchors = []
    for m in ANCHOR_RE.finditer(text):
        anchors.append((m.group(1), int(m.group(2)),
                        int(m.group(3)) if m.group(3) else None, m.group(4)))
        matched_spans.append((m.start(), m.end()))
    for m in CONT_ANCHOR_RE.finditer(text):
        if any(lo <= m.start() < hi for lo, hi in matched_spans):
            continue
        mentions = FILE_MENTION_RE.findall(text[:m.start()])
        if not mentions:
            continue
        anchors.append((mentions[-1], int(m.group(1)),
                        int(m.group(2)) if m.group(2) else None, None))
    for rel, line, end_line, symbol in anchors:
        where = '%s: %s:%d' % (label, rel, line)
        path = resolve_anchor_path(root, rel)
        if path is None:
            violations.append('%s 的锚点文件无法定位（%s）' % (where, rel))
            continue
        try:
            body = io.open(path, encoding='utf-8', errors='replace').read()
        except IOError:
            violations.append('%s 的锚点文件读不出（%s）' % (where, path))
            continue
        total = body.count(chr(10)) + 1
        if line < 1 or line > total:
            violations.append('%s 行号越界（该文件共 %d 行）' % (where, total))
            continue
        if end_line is not None and (end_line < line or end_line > total):
            violations.append('%s 区间终点越界（该文件共 %d 行）' % (where, total))
            continue
        if symbol:
            tail = re.split(r'[.:]', symbol)[-1]
            if tail and tail not in body:
                violations.append('%s 的锚点符号 %s 不在该文件里（F1 符号锚）'
                                  % (where, symbol))
    return violations


def parse_ir_generation_delivered(text):
    """解析 IR_GENERATION_DELIVERED 常量里的义务变体名。"""
    m = re.search(r'pub const IR_GENERATION_DELIVERED[^=]*=\s*&\[(.*?)\];', text, re.S)
    if not m:
        return None
    return set(re.findall(r'Obligation::(\w+)', m.group(1)))


def parse_struct_fields(text, struct_name):
    """解析某个 struct 体里的字段名集合。"""
    m = re.search(r'pub struct ' + re.escape(struct_name) + r'\s*\{(.*?)\n\}', text, re.S)
    if not m:
        return None
    fields = set()
    for line in m.group(1).splitlines():
        fm = re.match(r'\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_][a-z0-9_]*)\s*:', line)
        if fm:
            fields.add(fm.group(1))
    return fields


def parse_doc_table(text):
    """解析 02-stage-contract §义务字段表：字段名 → 定义行 + 表头声明的条数。

    表头（### 义务字段：N 个…）自己声明了条数——把它与 INVENTORY 对账，
    这样「三处一起漏改」也会红（只对字段名集合做不到这一点）。
    定义行按表注约定取**字段声明行**（含行号时一并返回，供 F4 对账）。
    """
    bt = chr(96)
    m = re.search(r'^### 义务字段：\s*(\d+)\s*个.*?$(.*?)^###', text, re.S | re.M)
    if not m:
        return None, None
    declared = int(m.group(1))
    rows = {}
    for line in m.group(2).splitlines():
        rm = re.match(r'\|\s*' + bt + r'([a-z_][a-z0-9_]*)' + bt + r'\s*\|\s*' +
                      bt + r'(\d+)' + bt + r'\s*\|', line)
        if rm:
            rows[rm.group(1)] = int(rm.group(2))
    return rows, declared


def parse_decl_lines(text):
    """types.rs 的字段声明行：字段名 → 行号（F4 的基准）。"""
    decls = {}
    for idx, line in enumerate(text.splitlines()):
        dm = re.match(r'\s*pub(?:\(super\))?\s+([a-z_][a-z0-9_]*)\s*:', line)
        if dm:
            decls.setdefault(dm.group(1), idx + 1)
    return decls


def reads_field(text, field):
    """真读取判定：成员访问 .字段名（注释与测试代码不算）。返回首个命中行号，0 = 无。"""
    cut = cut_test_sections(text)
    stripped_lines = strip_comments(cut).splitlines()
    original_lines = cut.splitlines()
    pattern = re.compile(r'[.]\s*' + re.escape(field) + r'(?![a-zA-Z0-9_])')
    for idx, line in enumerate(stripped_lines):
        original = original_lines[idx] if idx < len(original_lines) else line
        if pattern.search(original):
            return idx + 1
    return 0


def collect_src_reads(root, field):
    """全 src/ 扫描该字段的真读取（排除 typecheck 树与义务清单自身）。"""
    hits = []
    src = os.path.join(root, 'src')
    for base, dirs, files in os.walk(src):
        dirs[:] = [d for d in dirs if d not in ('target', '.git')]
        for fn in files:
            if not fn.endswith(('.rs', '.rs.txt')):
                continue
            full = os.path.join(base, fn)
            rel = os.path.relpath(full, root).replace('\\', '/')
            if rel.endswith('.txt'):
                rel = rel[:-4]
            if rel == MANIFEST or rel.startswith(TYPECHECK_TREE + '/'):
                continue
            if 'test' in rel.lower():
                continue
            try:
                text = read(full)
            except OSError:
                continue
            line = reads_field(text, field)
            if line:
                hits.append('%s:%d' % (rel, line))
    return hits


def parse_delivery_marks(text):
    """解析交付点上的标记：deliver(Obligation::X)。"""
    return set(re.findall(r'deliver\(\s*Obligation::(\w+)\s*\)', text))


def check(root, verbose=False):
    """对仓库根 root 跑全部判据。返回 (violations, notes, summary)。"""
    violations = []
    notes = []
    manifest_path = find(root, MANIFEST)
    types_path = find(root, TYPES)
    contract_path = find(root, CONTRACT)
    arms_path = find(root, DELIVERY_SITE)
    for label, p in ((MANIFEST, manifest_path), (TYPES, types_path),
                     (CONTRACT, contract_path), (DELIVERY_SITE, arms_path)):
        if p is None:
            return (['无法完成对账：缺文件 %s' % label], notes, None)

    manifest_text = read(manifest_path)
    entries = parse_manifest(manifest_text)
    if not entries:
        return (['无法解析 %s 的 INVENTORY 表' % MANIFEST], notes, None)

    # 判据 1：三处对账
    fields = [e['field'] for e in entries]
    if len(fields) != len(set(fields)):
        violations.append('INVENTORY 有重复字段名：%s' % fields)
    if not entries:
        violations.append('INVENTORY 为空')

    struct_decls = parse_decl_lines(read(types_path))
    struct_fields = parse_struct_fields(read(types_path), 'TypeCheckResult')
    if struct_fields is None:
        violations.append('无法解析 %s 的 TypeCheckResult 结构体' % TYPES)
        struct_fields = set()

    non_obligation = parse_non_obligation_fields(manifest_text)
    if non_obligation is None:
        violations.append('无法解析 %s 的 NON_OBLIGATION_FIELDS（判据 1 的第三处）' % MANIFEST)
        non_obligation = []
    non_obligation_fields = {e['field'] for e in non_obligation}

    doc_rows, declared = parse_doc_table(read(contract_path))
    doc_fields = sorted(doc_rows) if doc_rows else None
    if doc_fields is None:
        violations.append('无法解析 %s 的 §义务字段表（含表头条数声明）' % CONTRACT)
        doc_fields, declared = [], None
    if declared is not None and declared != len(entries):
        violations.append('02 §义务字段表表头声明 %d 条，INVENTORY 有 %d 条——三处一起漏改'
                          % (declared, len(entries)))

    for e in entries:
        f = e['field']
        if f not in struct_fields:
            violations.append('字段 %s 不在 TypeCheckResult（%s）里' % (f, TYPES))
        if e['consumer'] not in DELIVERY_STAGES:
            violations.append('字段 %s 的消费者阶段 %s 不是交付阶段之一 %s'
                              % (f, e['consumer'], list(DELIVERY_STAGES)))

    # F1（门禁加固）：登记与证据里的锚点必须可复核
    for entry in non_obligation:
        violations.extend(verify_anchors(
            root, entry['reason'], '非义务登记 %s 的理由' % entry['field']))
    for e in entries:
        if e['evidence']:
            violations.extend(verify_anchors(
                root, e['evidence'], '义务 %s 的证据指针' % e['field']))

    # F4（表格锚点）：02 §义务字段表的「定义行」必须等于类型声明行
    if doc_rows:
        for field, doc_line in sorted(doc_rows.items()):
            actual = struct_decls.get(field)
            if actual is None:
                continue
            if actual != doc_line:
                violations.append('02 §义务字段表 %s 的定义行写 %d，实际声明在 %s:%d'
                                  '（表注约定：字段声明行）'
                                  % (field, doc_line, TYPES, actual))

    # 判据 1 的反向 containment：结构体字段必须全部登记（义务清单或非义务登记）
    inventory_fields = set(fields)
    for f in sorted(struct_fields - inventory_fields - non_obligation_fields):
        violations.append('TypeCheckResult 字段 %s 未登记：既不在义务清单（INVENTORY）也不在'
                          '非义务字段登记（NON_OBLIGATION_FIELDS，须含理由）' % f)
    for entry in non_obligation:
        f = entry['field']
        if f not in struct_fields:
            violations.append('非义务登记 %s 不在 TypeCheckResult 里（登记已陈旧）' % f)
        if f in inventory_fields:
            violations.append('字段 %s 同时登记为义务与非义务——两表必须互补' % f)
        if len(entry['reason']) < 12:
            violations.append('非义务登记 %s 的理由过短/为空（必须写实测消费面）: %r'
                              % (f, entry['reason']))
    missing_in_doc = [f for f in fields if f not in doc_fields]
    extra_in_doc = [f for f in doc_fields if f not in fields]
    if missing_in_doc:
        violations.append('02 §义务字段表缺 %d 条：%s' % (len(missing_in_doc), ', '.join(missing_in_doc)))
    if extra_in_doc:
        violations.append('02 §义务字段表多出 INVENTORY 之外 %d 条：%s'
                          % (len(extra_in_doc), ', '.join(extra_in_doc)))

    # 判据 3：交付标记集合
    arms_text = read(arms_path)
    marks = parse_delivery_marks(arms_text)
    if 'deliver_all(IR_GENERATION_DELIVERED)' in arms_text:
        cons = parse_ir_generation_delivered(manifest_text)
        if cons is None:
            violations.append('无法解析 IR_GENERATION_DELIVERED（%s）' % MANIFEST)
        else:
            marks |= cons
    expected_marks = {e['variant'] for e in entries if not e['wbs']}
    red_variants = {e['variant'] for e in entries if e['wbs']}
    for v in sorted(expected_marks - marks):
        violations.append('义务 %s 没有交付标记（%s 的交付点缺 deliver）' % (v, DELIVERY_SITE))
    for v in sorted(marks & red_variants):
        violations.append('红名单义务 %s 被标记交付了——红名单已陈旧，须清账' % v)
    unknown = marks - {e['variant'] for e in entries}
    if unknown:
        violations.append('交付点标记了 INVENTORY 之外的义务：%s' % ', '.join(sorted(unknown)))

    # 判据 2：读出证据（非红名单）+ 判据 4：红名单零读取
    for e in entries:
        f = e['field']
        if e['wbs']:
            if not re.match(r'^\d+[.]\d+([.]\d+)?$', e['wbs']):
                violations.append('红名单 %s 的 WBS id 形态非法：%r' % (f, e['wbs']))
            if not e['evidence'] or len(e['evidence']) < 20:
                violations.append('红名单 %s 缺消费者为零的证据指针' % f)
            hits = collect_src_reads(root, f)
            if hits:
                violations.append('红名单 %s 在 typecheck 树外出现真读取（陈旧）：%s'
                                  % (f, ', '.join(hits[:3])))
            notes.append('红名单 %-24s WBS %-7s 树外真读取 0' % (f, e['wbs']))
            continue
        candidates = [find(root, c) for c in CONSUMER_FILES.get(e['consumer'], [])]
        candidates = [c for c in candidates if c]
        if not candidates:
            violations.append('义务 %s 的消费者阶段 %s 没有登记消费者文件' % (f, e['consumer']))
            continue
        hit = None
        for c in candidates:
            line = reads_field(read(c), f)
            if line:
                hit = '%s:%d' % (os.path.relpath(c, root).replace('\\', '/'), line)
                break
        if hit is None:
            violations.append('义务 %s（消费者阶段 %s）在声明消费者文件里没有真读取——'
                              '标记与读取漂移或字段零消费' % (f, e['consumer']))
        elif verbose:
            notes.append('读出证据 %-24s %s' % (f, hit))

    summary = {
        'entries': len(entries),
        'to_deliver': len(expected_marks),
        'marked': len(expected_marks & marks),
        'red_list': [(e['field'], e['wbs']) for e in entries if e['wbs']],
        'non_obligation': len(non_obligation),
        'struct_total': len(struct_fields),
        'residual': sorted(struct_fields - set(fields) - non_obligation_fields),
    }
    return violations, notes, summary


def run_root(root, verbose=False):
    violations, notes, summary = check(root, verbose)
    if verbose:
        for n in notes:
            print('  - %s' % n)
    if summary is None:
        print('义务字段门禁：无法完成对账')
        for v in violations:
            print('  ! %s' % v)
        return 2, violations
    if violations:
        print('义务字段门禁：FAIL —— %d 条违反判据' % len(violations))
        for v in violations:
            print('  x %s' % v)
        return 1, violations
    if summary['red_list']:
        print('义务字段门禁：PASS（%d 条义务：%d 条须交付且全部有交付标记；红名单 %d 条按登记保留；'
              '非义务登记 %d 条）'
              % (summary['entries'], summary['to_deliver'], len(summary['red_list']),
                 summary['non_obligation']))
        for field, wbs in summary['red_list']:
            print('  红名单（不静默）：%s —— 清账任务 WBS %s' % (field, wbs))
    else:
        print('义务字段门禁：PASS（%d 条义务：%d 条须交付且全部有交付标记；'
              '红名单 0 条——已清账归零；非义务登记 %d 条）'
              % (summary['entries'], summary['to_deliver'], summary['non_obligation']))
        print('  红名单为空：字段普查由「义务清单 ∪ 非义务登记」闭合（反向 containment 判据）')
    print('  字段普查差值（F2）：TypeCheckResult %d 字段 - 义务 %d - 非义务登记 %d = %d%s'
          % (summary['struct_total'], summary['entries'], summary['non_obligation'],
             len(summary['residual']),
             '' if not summary['residual']
             else '（未登记：%s）' % ', '.join(summary['residual'])))
    return 0, []


def selftest():
    """判据 5：自带反例/正例夹具——探测器不得空转。

    四种夹具各钉一类事实，缺一即探测器有盲区：
      negative            幽灵义务（定义 + 写入但零读取、未登记红名单）→ 必红
      positive            正例（红名单为空 + 有交付标记 + 真实读取）→ 必绿
      stale-manifest      代码删了清单没同步（结构体已无该字段，INVENTORY 仍登记）→ 必红
      stale-struct        清单删了代码还在（INVENTORY 少登记，结构体与文档表仍在）→ 必红
      stale-red-list      红名单字段长出消费者（陈旧登记）→ 必红
      unregistered-field  结构体新增字段既不在义务清单也不在非义务登记 → 必红
      stale-anchor        登记理由里的 file:line 锚点失效（符号不存在 / 行号越界）→ 必红
    """
    base = os.path.join(ROOT, 'tests', 'baselines', 'obligations-gate')
    fixtures = (
        ('negative', True),
        ('positive', False),
        ('stale-manifest', True),
        ('stale-struct', True),
        ('stale-red-list', True),
        ('unregistered-field', True),
        ('stale-anchor', True),
    )
    ok = True
    for name, want_red in fixtures:
        root = os.path.join(base, name)
        if not os.path.isdir(root):
            print('自检：缺夹具目录 %s' % root)
            ok = False
            continue
        rc, _ = run_root(root)
        is_red = rc == 1
        verdict = 'PASS' if is_red == want_red else 'FAIL'
        if verdict == 'FAIL':
            ok = False
        print('自检夹具 %-18s 期望 %s，实际 %s -> %s'
              % (name, '红' if want_red else '绿', '红' if is_red else '绿', verdict))
    if not ok:
        print('自检失败：探测器判据不可信（反例未红 或 正例未绿）')
        return 1
    print('自检通过：七类夹具（幽灵义务 / 正例·红名单为空 / 清单滞后 / 结构体滞后 / '
          '红名单陈旧 / 未登记字段 / 锚点失效）行为符合预期')
    return 0


def main():
    args = sys.argv[1:]
    verbose = '--verbose' in args
    if '--selftest' in args:
        return selftest()
    root = ROOT
    if '--root' in args:
        root = os.path.abspath(args[args.index('--root') + 1])
    rc, _ = run_root(root, verbose)
    return rc


if __name__ == '__main__':
    sys.exit(main())
