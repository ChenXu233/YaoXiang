#!/usr/bin/env python3
"""从 generate_expr_ir_inner 抽取一个 match 臂到独立方法。

用法:
    python tools/extract_arm.py <Expr::Name> --dry-run
    python tools/extract_arm.py <Expr::Name> --apply

设计要点（吸取 span 迁移的教训）:
- 逐行扫描 + 括号配平，不用正则整段匹配
- 参数名与类型从 AST 定义（src/frontend/core/parser/ast.rs）解析，不猜测；
  解析失败即报错退出
- 默认 dry-run，--apply 才写盘
"""
import re
import sys

SRC = 'src/middle/core/ir_gen.rs'
AST = 'src/frontend/core/parser/ast.rs'

# 顶层臂：恰好 12 空格缩进以 Expr:: 开头。模式本身可能跨多行
# （如 "Expr::BinOp {\\n op,\\n ...\\n } =>"），因此只锚定起始行，
# 由 arms() 向后扫描配平的 "=> {" 终止符确定模式范围。
ARM_HEAD_RE = re.compile(r'^            (Expr::[A-Za-z]+)')

FN_NAME = 'generate_expr_ir_inner'
INDENT_ARM = ' ' * 12
INDENT_BODY = ' ' * 8


def find_fn(lines, name=FN_NAME):
    start = None
    for i, l in enumerate(lines):
        if l.strip().startswith('fn ' + name + '('):
            start = i
            break
    if start is None:
        raise SystemExit('找不到函数 ' + name)
    depth = 0
    started = False
    for k in range(start, len(lines)):
        depth += lines[k].count('{') - lines[k].count('}')
        if '{' in lines[k]:
            started = True
        if started and depth <= 0:
            return start, k
    raise SystemExit('函数花括号不配平')


def arms(lines, start, end):
    """返回 [(arm_name, head_idx, arrow_idx, arm_end_idx)]。

    head_idx   : "Expr::X" 起始行
    arrow_idx  : 模式结束、"=> {" 所在行（臂体首行 = arrow_idx + 1）
    arm_end_idx: 与臂头同缩进的孤 "}" 行
    """
    heads = []
    for k in range(start, end + 1):
        m = ARM_HEAD_RE.match(lines[k])
        if m:
            heads.append((m.group(1), k))

    out = []
    for i, (name, head) in enumerate(heads):
        limit = heads[i + 1][1] if i + 1 < len(heads) else end + 1
        # 向后扫描 "=> {"：模式内 ()/{} 需要配平
        depth = 0
        arrow = None
        for k in range(head, limit):
            line = lines[k]
            # 只统计 "=>" 之前的模式部分（否则会把臂体的 { 也计进来）
            before = line.split('=>')[0] if '=>' in line else line
            depth += before.count('(') - before.count(')')
            depth += before.count('{') - before.count('}')
            if depth == 0 and re.search(r'=>\s*\{\s*$', line):
                arrow = k
                break
        if arrow is None:
            raise SystemExit(
                '找不到 %s 的 "=> {" 终止符（%d 行起）' % (name, head + 1))
        # 臂尾：从下一个臂头往回找同缩进的孤 "}"
        last = None
        for j in range(limit - 1, arrow, -1):
            if lines[j].rstrip() == INDENT_ARM + '}':
                last = j
                break
        if last is None:
            raise SystemExit('找不到 %s 的臂尾' % name)
        out.append((name, head, arrow, last))
    return out


def parse_ast_variants():
    """解析 ast.rs 的 Expr 枚举 → {变体名: [(字段名, 类型)]}；unit 变体为 []。"""
    src = open(AST, encoding='utf-8').read()
    m = re.search(r'pub enum Expr \{(.*?)\n\}', src, re.S)
    if not m:
        raise SystemExit('找不到 ast.rs 的 Expr 枚举')
    body = m.group(1)

    # 先剔除行注释：注释里的尖括号（如 "// 过滤条件 if x > 0"）
    # 会破坏后续的括号配平与字段解析
    body = re.sub(r'//[^\n]*', '', body)

    variants = {}
    head_re = re.compile(r'^\s{4}([A-Z][A-Za-z0-9_]*)\s*(\(|\{)?', re.M)
    for vm in head_re.finditer(body):
        name, opener = vm.group(1), vm.group(2)
        if opener is None:
            variants[name] = []
            continue
        # 从 opener 起做括号配平，取出字段文本
        i = vm.end() - 1
        depth = 0
        buf = []
        while i < len(body):
            c = body[i]
            if c in '({<[':
                depth += 1
                if depth == 1:
                    i += 1
                    continue
            elif c in ')}>]':
                depth -= 1
                if depth == 0:
                    break
            buf.append(c)
            i += 1
        txt = ''.join(buf)

        if opener == '(':
            fields, depth, cur = [], 0, ''
            for c in txt:
                if c in '(<[':
                    depth += 1
                elif c in ')>]':
                    depth -= 1
                if c == ',' and depth == 0:
                    fields.append(cur.strip())
                    cur = ''
                else:
                    cur += c
            if cur.strip():
                fields.append(cur.strip())
            variants[name] = [('__tuple__', f) for f in fields]
        else:
            fields = []
            for line in txt.split('\n'):
                line = line.strip()
                if not line or line.startswith('//') or line.startswith('#['):
                    continue
                # 去掉行内注释，否则会污染字段名与类型
                line = line.split('//')[0].strip()
                if not line:
                    continue
                fm = re.match(r'^([a-z_][A-Za-z0-9_]*)\s*:\s*(.+?),?$', line)
                if fm:
                    fields.append((fm.group(1), fm.group(2).rstrip(',')))
            variants[name] = fields
    return variants


def rust_type(ast_ty):
    """AST 字段类型 → 生成器方法参数类型（一律取引用）。"""
    t = ast_ty.strip()
    if t.startswith('Box<') and t.endswith('>'):
        return '&' + t[4:-1]
    return '&' + t


def to_snake(name):
    return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    target = sys.argv[1]
    apply = '--apply' in sys.argv

    src = open(SRC, encoding='utf-8').read()
    trailing_nl = src.endswith('\n')
    lines = src.split('\n')
    if trailing_nl:
        lines = lines[:-1]

    start, end = find_fn(lines)
    found = arms(lines, start, end)
    matches = [a for a in found if a[0] == target]
    if len(matches) != 1:
        raise SystemExit('%s: 匹配到 %d 个臂，需要恰好 1 个' % (target, len(matches)))

    name, a_head, a_arrow, a_end = matches[0]
    short = name.split('::')[1]
    variants = parse_ast_variants()
    if short not in variants:
        raise SystemExit('ast.rs 中找不到变体 ' + short)
    fields = variants[short]

    mname = 'generate_' + to_snake(short) + '_expr_ir'

    # 臂体：arrow 行之后到臂尾之前
    body = lines[a_arrow + 1:a_end]
    indents = [len(l) - len(l.lstrip()) for l in body if l.strip()]
    base = min(indents) if indents else len(INDENT_ARM) + 4
    body_out = [(INDENT_BODY + l[base:]) if l.strip() else '' for l in body]
    body_txt = '\n'.join(body_out)

    # 从臂头到 arrow 行拼出模式文本（丢掉 "=>" 及其后的臂体内容）
    pat_txt = ' '.join(l.strip() for l in lines[a_head:a_arrow + 1])
    if '=>' in pat_txt:
        # 丢弃 "=>" 之后的内容，但保留 => 本身供下方正则锚定
        head_part, _ = pat_txt.split('=>', 1)
        pat_txt = head_part.strip() + ' =>'

    binds = []   # [(绑定名, 参数类型)]
    if fields and fields[0][0] == '__tuple__':
        mh = re.match(r'^Expr::[A-Za-z]+\((.*)\)\s*=>', pat_txt)
        if not mh:
            raise SystemExit('无法解析 tuple 臂的绑定: ' + pat_txt)
        names = [n.strip() for n in mh.group(1).split(',') if n.strip()]
        tys = [rust_type(f[1]) for f in fields]
        if len(names) != len(tys):
            raise SystemExit('tuple 绑定数(%d) 与 AST 字段数(%d) 不符'
                             % (len(names), len(tys)))
        binds = list(zip(names, tys))
    elif fields:
        mh = re.match(r'^Expr::[A-Za-z]+\s*\{(.*?)[,]?\s*\}\s*=>', pat_txt)
        if not mh:
            raise SystemExit('无法解析 struct 臂的绑定: ' + pat_txt)
        names = [n.strip() for n in mh.group(1).split(',') if n.strip()]
        byname = dict(fields)
        for raw in names:
            if raw == '..':
                continue
            # 支持重命名绑定 `field: bind`（AST 字段名是冒号左侧）
            if ':' in raw:
                fname, bind = [x.strip() for x in raw.split(':', 1)]
            else:
                fname, bind = raw, raw
            if fname not in byname:
                raise SystemExit('AST 中 %s 无字段 %s' % (short, fname))
            binds.append((bind, rust_type(byname[fname])))

    # 只在臂体真正用到时才传这些公共参数
    extras = []
    if re.search(r'\bresult_reg\b', body_txt):
        extras.append(('result_reg', 'usize'))
    if re.search(r'\binstructions\b', body_txt):
        extras.append(('instructions', '&mut Vec<Instruction>'))
    if re.search(r'\bconstants\b', body_txt):
        extras.append(('constants', '&mut Vec<ConstValue>'))

    used = [(n, t) for n, t in binds
            if n != '_' and re.search(r'\b' + re.escape(n) + r'\b', body_txt)]
    unused = [(n, t) for n, t in binds if n != '_' and (n, t) not in used]

    sig_params = ['%s: %s' % (n, t) for n, t in used] + \
                 ['%s: %s' % (n, t) for n, t in extras]

    # 尾表达式处理：臂体的类型本来是 ()，外提为方法后需要 Ok(())。
    # 规则要稳健：无论臂体以 `;`、`}`（if/else 块）还是 `})` 结尾，
    # 补一行 Ok(()) 都能成立；只有当臂体已经以 return/Ok(()) 结束时
    # 才不能补（否则是 unreachable code，clippy -D warnings 会拦）。
    last_stmt = ''
    for l in reversed(body_out):
        if l.strip():
            last_stmt = l.strip()
            break
    diverges = (last_stmt.startswith('return')
                or last_stmt.startswith('Ok(()')
                or last_stmt.startswith('Err('))
    tail_line = [] if diverges else ['        Ok(())']

    new_method = ['    fn ' + mname + '(', '        &mut self,']
    new_method += ['        ' + p + ',' for p in sig_params]
    new_method += ['    ) -> Result<(), Diagnostic> {']
    new_method += body_out + tail_line + ['    }', '']

    call_args = [n for n, _ in used] + [n for n, _ in extras]
    call = 'self.%s(%s)?;' % (mname, ', '.join(call_args))

    # 委派臂：绑定用到的名字，其余用 _
    if fields and fields[0][0] == '__tuple__':
        used_names = {n for n, _ in used}
        pat = ', '.join(n if n in used_names else '_' for n, _ in binds)
        deleg_head = INDENT_ARM + name + '(' + pat + ') => {'
    elif fields:
        keep = ', '.join(n for n, _ in used)
        if unused:
            pat = '{ ' + keep + ', .. }' if keep else '{ .. }'
        else:
            pat = '{ ' + keep + ' }' if keep else '{ .. }'
        deleg_head = INDENT_ARM + name + ' ' + pat + ' => {'
    else:
        deleg_head = INDENT_ARM + name + ' => {'
    deleg = [deleg_head, INDENT_ARM + '    ' + call, INDENT_ARM + '}']

    out = lines[:a_head] + deleg + lines[a_end + 1:]
    s2, _ = find_fn(out)
    out = out[:s2] + new_method + out[s2:]

    if not apply:
        print('[dry-run] %s -> %s' % (name, mname))
        print('  臂 %d-%d (%d 行) -> 委派 3 行'
              % (a_head + 1, a_end + 1, a_end - a_head + 1))
        print('  签名: (%s)' % (', '.join(sig_params) or '无额外参数'))
        print('  丢弃的未用绑定: %s' % ([n for n, _ in unused] or '无'))
        print('  委派臂: %s' % deleg_head.strip())
        print('  调用:   %s' % call)
        print('  新方法 %d 行' % len(new_method))
        return

    open(SRC, 'w', encoding='utf-8').write(
        '\n'.join(out) + ('\n' if trailing_nl else ''))
    print('[applied] %s -> %s (%d 行 -> 委派 3 行)'
          % (name, mname, a_end - a_head + 1))


if __name__ == '__main__':
    main()
