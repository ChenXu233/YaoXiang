#!/usr/bin/env python3
"""从 generate_expr_ir_inner 抽取一个 match 臂到独立方法。

用法:
    python tools/extract_arm.py <Expr::Name> --dry-run
    python tools/extract_arm.py <Expr::Name> --apply

设计要点（吸取 span 迁移的教训）:
- 逐行扫描 + 花括号配平，不用正则整段匹配
- 参数名与类型从 AST 定义（src/frontend/core/parser/ast.rs）解析，
  不做猜测；解析失败即报错退出而不是猜一个
- 默认 dry-run，--apply 才写盘
"""
import re
import sys

SRC = 'src/middle/core/ir_gen.rs'
AST = 'src/frontend/core/parser/ast.rs'
ARM_RE = re.compile(r'^            (Expr::[A-Za-z]+)\s*(\(|\{|\s*=>)')


def find_fn(lines, name='generate_expr_ir_inner'):
    start = None
    for i, l in enumerate(lines):
        if l.strip().startswith(f'fn {name}('):
            start = i
            break
    if start is None:
        raise SystemExit(f'找不到函数 {name}')
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
    found = []
    for k in range(start, end + 1):
        m = ARM_RE.match(lines[k])
        if m and '=>' in lines[k]:
            found.append((m.group(1), k))
    out = []
    for i, (name, k) in enumerate(found):
        nxt = found[i + 1][1] if i + 1 < len(found) else end
        last = None
        for j in range(nxt - 1, k, -1):
            if lines[j].strip() == '}':
                last = j
                break
        if last is None:
            raise SystemExit(f'找不到 {name} 的臂尾')
        out.append((name, k, last))
    return out


def parse_ast_variants():
    """解析 ast.rs 的 Expr 枚举 → {变体名: [(字段名, 类型)]}，unit 变体为空列表。"""
    src = open(AST, encoding='utf-8').read()
    m = re.search(r'pub enum Expr \{(.*?)\n\}', src, re.S)
    if not m:
        raise SystemExit('找不到 ast.rs 的 Expr 枚举')
    body = m.group(1)
    variants = {}
    # 逐个变体：名字 后跟 (tuple 字段) 或 { 结构字段 } 或 ,
    for vm in re.finditer(r'^\s{4}([A-Z][A-Za-z0-9_]*)\s*(\(|\{)?', body, re.M):
        name, opener = vm.group(1), vm.group(2)
        if opener is None:
            variants[name] = []
            continue
        # 从 opener 位置起做括号配平
        i = vm.end() - 1
        depth = 0
        fields_txt = []
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
            fields_txt.append(c)
            i += 1
        txt = ''.join(fields_txt)
        if opener == '(':
            fields = []
            depth = 0
            cur = ''
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
            # 结构体字段：name: Type,（跳过注释行与属性）
            fields = []
            for line in txt.split('\n'):
                line = line.strip()
                if not line or line.startswith('//') or line.startswith('#['):
                    continue
                fm = re.match(r'^([a-z_][A-Za-z0-9_]*)\s*:\s*(.+?),?$', line)
                if fm:
                    fields.append((fm.group(1), fm.group(2).rstrip(',')))
            variants[name] = fields
    return variants


def rust_type(ast_ty):
    """AST 字段类型 → 生成器方法参数类型。"""
    t = ast_ty.strip()
    if t.startswith('Box<') and t.endswith('>'):
        inner = t[4:-1]
        return f'&{inner}'
    if t.startswith('Vec<') or t.startswith('Option<') or t == 'String' or t == 'Span':
        return f'&{t}'
    return f'&{t}'


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
        raise SystemExit(f'{target}: 匹配到 {len(matches)} 个臂，需要恰好 1 个')

    name, a_start, a_end = matches[0]
    variants = parse_ast_variants()
    short = name.split('::')[1]
    if short not in variants:
        raise SystemExit(f'ast.rs 中找不到变体 {short}')
    fields = variants[short]

    snake = re.sub(r'(?<!^)(?=[A-Z])', '_', short).lower()
    mname = f'generate_{snake}_expr_ir'

    body = lines[a_start + 1:a_end]
    indents = [len(l) - len(l.lstrip()) for l in body if l.strip()]
    base = min(indents) if indents else 16
    # 去臂体缩进后，重新缩进到方法体层级（8 空格），保证 cargo fmt 无异议
    body_dedented = [
        ('        ' + l[base:]) if l.strip() else '' for l in body
    ]
    body_txt = '\n'.join(body)

    # 参数、绑定模式、调用实参
    params, binds, args = [], [], []
    if fields and fields[0][0] == '__tuple__':
        for fty in (f[1] for f in fields):
            params.append(rust_type(fty))
        # 绑定名从臂头解析（源码里已有名字）
        mh = re.match(r'^\s*Expr::[A-Za-z]+\((.*)\)\s*=>', lines[a_start])
        if not mh:
            raise SystemExit('无法解析 tuple 臂的绑定')
        names = [n.strip() for n in mh.group(1).split(',')]
        for n, pty in zip(names, params):
            binds.append((n, pty))
            args.append('_' if n == '_' else n)
    elif fields:
        mh = re.match(r'^\s*Expr::[A-Za-z]+\s*\{\s*(.*?)\s*\}\s*=>', lines[a_start])
        if not mh:
            raise SystemExit('无法解析 struct 臂的绑定')
        names = [n.strip() for n in mh.group(1).split(',')]
        byname = {n: t for n, t in fields}
        for n in names:
            base_n = n.split(':')[0].strip()
            if base_n == '..':
                continue
            pty = rust_type(byname[base_n])
            binds.append((base_n, pty))
            args.append(base_n)
    else:
        binds = []

    # 逐参数判定是否需要传 result_reg/instructions/constants
    extras = []
    if re.search(r'\bresult_reg\b', body_txt):
        extras.append('result_reg: usize')
    if re.search(r'\binstructions\b', body_txt):
        extras.append('instructions: &mut Vec<Instruction>')
    if re.search(r'\bconstants\b', body_txt):
        extras.append('constants: &mut Vec<ConstValue>')

    # 只保留臂体真正用到的绑定参数
    used = [b for b in binds if b[0] != '_' and re.search(rf'\b{re.escape(b[0])}\b', body_txt)]
    unused = [b for b in binds if b[0] != '_' and b not in used]

    sig_params = [f'{n}: {t}' for n, t in used] + extras
    new_method = ['    fn ' + mname + '(', '        &mut self,']
    new_method += [f'        {p},' for p in sig_params]
    # 若臂体无尾表达式（以 ; 结尾），补 Ok(()) 作为方法返回值
    tail = ''
    for l in reversed(body_dedented):
        if l.strip():
            tail = l.rstrip()
            break
    if not tail or tail.endswith(';') or tail.endswith('{'):
        tail_line = ['        Ok(())']
    else:
        tail_line = []
    new_method += ['    ) -> Result<(), Diagnostic> {'] + body_dedented + tail_line + ['    }', '']

    call_args = [b[0] for b in used] + [
        e.split(':')[0] for e in extras
    ]
    call = f'self.{mname}({", ".join(call_args)})?;'

    # 委派臂：只绑定用到的名字，其余用 ..
    if fields and fields[0][0] == '__tuple__':
        # tuple 变体必须完整列出字段：用 _ 占位未使用者也一样
        used_names = {b[0] for b in used}
        pat_binds = ', '.join(
            ('_' if n == '_' else n) if n in used_names else '_'
            for n in [b[0] for b in binds]
        )
        deleg_head = f'            {name}({pat_binds}) => {{'
    elif fields:
        if unused:
            keep = ', '.join(b[0] for b in used)
            pat = f'{{ {keep}, .. }}' if keep else '{ .. }'
            deleg_head = f'            {name} {pat} => {{'
        else:
            deleg_head = f'            {name} => {{' if not used else f'            {name} {{ {", ".join(b[0] for b in used)} }} => {{'
    else:
        deleg_head = f'            {name} => {{'
    deleg = [deleg_head, f'                {call}', '            }']

    out = lines[:a_start] + deleg + lines[a_end + 1:]
    s2, _ = find_fn(out)
    out = out[:s2] + new_method + out[s2:]

    if not apply:
        print(f'[dry-run] {name} -> {mname}')
        print(f'  臂 {a_start+1}-{a_end+1} ({a_end-a_start+1} 行) -> 委派 3 行')
        print(f'  签名: ({", ".join(sig_params) or "无额外参数"})')
        print(f'  未用到而丢弃的绑定: {[b[0] for b in unused] or "无"}')
        print(f'  委派臂: {deleg_head.strip()}')
        print(f'  调用:   {call}')
        print(f'  新方法 {len(new_method)} 行')
        return

    open(SRC, 'w', encoding='utf-8').write('\n'.join(out) + ('\n' if trailing_nl else ''))
    print(f'[applied] {name} -> {mname} ({a_end-a_start+1} 行 -> 委派 3 行)')


if __name__ == '__main__':
    main()
