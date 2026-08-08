import re, glob

def parse_args(s, i):
    """s[i] == '('; return (args_list, index_after_closing_paren)"""
    depth = 0
    cur = ''
    args = []
    j = i
    while j < len(s):
        c = s[j]
        if c == '(':
            depth += 1
            if depth > 1:
                cur += c
        elif c == ')':
            depth -= 1
            if depth == 0:
                if cur.strip():
                    args.append(cur.strip())
                return args, j + 1
            cur += c
        elif c == ',' and depth == 1:
            args.append(cur.strip())
            cur = ''
        else:
            cur += c
        j += 1
    raise ValueError("unbalanced")

total = 0
for path in glob.glob('src/std/*.rs'):
    s = open(path, encoding='utf-8').read()
    out = []
    i = 0
    n = 0
    while i < len(s):
        m = re.match(r'NativeExport::(new|constant)\(', s[i:])
        if m:
            kind = m.group(1)
            args, end = parse_args(s, i + len(m.group(0)) - 1)
            # find trailing comma and optional cast
            j = end
            while j < len(s) and s[j] in ' \t\n\r':
                j += 1
            trailing = ''
            if j < len(s) and s[j] == ',':
                trailing = ','
                j += 1
            if kind == 'new':
                # 4 args: name, native, sig, handler — handler may be "x as NativeHandler"
                assert len(args) == 4, (path, args)
                name, native, sig, handler = args
                handler = handler.replace(' as NativeHandler', '')
                line = f'export!({name}, {native}, {sig}, {handler}){trailing}'
            else:
                assert len(args) == 3, (path, args)
                name, native, sig = args
                line = f'export!({name}, {native}, {sig}){trailing}'
            out.append(line)
            i = j
            n += 1
            continue
        out.append(s[i])
        i += 1
    if n:
        open(path, 'w', encoding='utf-8').write(''.join(out))
        total += n
        print(f"{path}: {n} exports macro-ized")
print("TOTAL:", total)
