#!/usr/bin/env python3
"""找出 Rust 源里某个 fn 的结束行（忽略字符串/注释中的花括号）。"""
import sys


def scan_brace_depth_events(src):
    """返回 (位置, 该位置之后的新深度) 事件列表。"""
    events = []
    depth = 0
    i = 0
    n = len(src)
    in_str = False
    in_line = False
    in_block = False
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ''
        if in_line:
            if c == '\n':
                in_line = False
            i += 1
            continue
        if in_block:
            if c == '*' and nxt == '/':
                in_block = False
                i += 2
                continue
            i += 1
            continue
        if in_str:
            if c == '\\':
                i += 2
                continue
            if c == '"':
                in_str = False
            i += 1
            continue
        if c == '/' and nxt == '/':
            in_line = True
            i += 1
            continue
        if c == '/' and nxt == '*':
            in_block = True
            i += 1
            continue
        if c == '"':
            in_str = True
            i += 1
            continue
        if c == '{':
            depth += 1
            events.append((i, depth))
        elif c == '}':
            depth -= 1
            events.append((i, depth))
        i += 1
    return events


def depth_at(src, events, pos):
    d = 0
    for p, nd in events:
        if p >= pos:
            break
        d = nd
    return d


def main():
    path = sys.argv[1]
    needle = sys.argv[2]
    src = open(path, encoding='utf-8').read()
    events = scan_brace_depth_events(src)
    idx = src.index(needle)
    line_start = src.rfind('\n', 0, idx) + 1
    line_no = src[:line_start].count('\n') + 1
    base = depth_at(src, events, idx)
    print(f'{needle}: 起始行 {line_no}, 起始深度 {base}')

    # 找第一个 "}" 事件，位置在 needle 之后且深度回到 base
    for pos, nd in events:
        if pos > idx and nd == base:
            end_line = src[:pos].count('\n') + 1
            print(f'  结束行: {end_line}')
            lines = src.split('\n')
            for j in range(max(0, end_line - 3), min(len(lines), end_line + 2)):
                print(f'    {j + 1}| {lines[j]}')
            return
    print('  未找到结束位置')


if __name__ == '__main__':
    main()
