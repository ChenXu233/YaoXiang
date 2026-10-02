#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-docs-orphan.py —— 文档结构门禁：孤儿页 / 缺失 frontmatter / 重复 H1

背景（2026-10-02 审计发现）：
  - `docs/src/reference/error-codes.md` 有 165 行真实内容，却不在任何 nav/sidebar
    项里，只被 2 处引用 → 事实上的孤儿页；而侧栏「错误码总览」指向的
    `error-code/index.md` 只有 56 行空壳，9 个 stdlib 页全指向那个空壳。
  - `error-code/index.md` 与 8 个 `E*.md` 连 frontmatter 都没有，但
    `config.js:342` 对该目录设了 `useTitleFromFrontmatter: true`。
  - `docs/src/reference/error-code/index.md` 与被删的 `error-codes.md`
    H1 都叫「错误码参考」。

本脚本检查：
  1. 每个 .md 是否可从首页经链接到达（不可达 = 孤儿）
  2. generateSidebar 使用的目录里的文件是否有 title frontmatter
  3. 站内 H1 标题是否重复
  4. 页内相对链接的目标文件是否存在（轻量死链预检，补充构建期检查）

退出码：0 = 通过；1 = 有问题
"""
import io
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DOCS = os.path.join(ROOT, 'docs', 'src')
CONFIG = os.path.join(DOCS, '.vitepress', 'config.js')

problems = []
notes = []

# generateSidebar 扫描的目录 —— 这些目录的文件标题来自 frontmatter
SIDEBAR_DIRS = [
    'tutorial/basics', 'tutorial/advanced',
    'design/rfc/accepted', 'design/rfc/review', 'design/rfc/draft',
    'design/rfc/rejected', 'design/rfc/deprecated',
    'reference/error-code', 'reference/package',
]
# 显式排除在构建之外的目录（config.js:40 srcExclude）
EXCLUDED_PREFIX = ('archive/', 'old/')


def read(p):
    with io.open(p, 'r', encoding='utf-8', errors='replace') as f:
        return f.read()



def all_md():
    out = []
    for r, dirs, files in os.walk(DOCS):
        dirs[:] = [d for d in dirs if d not in ('node_modules', '.vitepress', 'dist')]
        for fn in files:
            if fn.endswith('.md'):
                out.append(os.path.join(r, fn))
    return out


def rel(p):
    return os.path.relpath(p, DOCS).replace('\\', '/')


def is_published(p):
    """config.js:40 的 srcExclude 只匹配中文侧（英文侧已另行修复为 en/archive/**）。"""
    r = rel(p)
    return not any(r.startswith(x) for x in EXCLUDED_PREFIX)


def parse_links(text, cur_rel):
    """抽出 markdown 链接与 frontmatter 里的 link: 目标。"""
    out = []
    # 只在正文（frontmatter 之后）找 markdown 链接 —— frontmatter 里的
    # link:/actions 属于站点配置，由 VitePress 自己处理，不是页面死链
    body = text
    if text.startswith('---') and text.count('---') >= 2:
        body = text.split('---', 2)[2]
    fm_links = re.findall(r"link:\s*['\"]?(/[^'\"\s]*)['\"]?", text)
    for m in re.finditer(r'\]\(([^)#\s]+)(?:#[^)]*)?\)', body):
        out.append(m.group(1))
    return out


def resolve(p, link):
    """把一个链接目标解析成仓库内路径；解析不出来返回 None。

    必须处理三种写法，否则会把正常链接全判成死链：
      - 目录式   ./commands      -> commands.md 或 commands/index.md
      - 补扩展名 ./foo.md        -> foo.md
      - 站点根   /reference/     -> docs/src/reference/index.md
    """
    if link.startswith('/'):
        base = DOCS
        target = link.lstrip('/')
    else:
        base = os.path.dirname(p)
        target = link
    cand = os.path.normpath(os.path.join(base, target))
    for c in (cand, cand + '.md', os.path.join(cand, 'index.md'),
              os.path.join(cand, 'README.md')):
        if os.path.isfile(c):
            return c
    return None


# 显式豁免：这些页面按设计不从任何页面链接到达。
# 目录说明页（README.md）的作用是解释「这个目录收什么」，
# 内容是指向 TRACKING.md 等权威索引的指针，不重复维护列表。
ORPHAN_EXEMPT = {
    'design/rfc/accepted/README.md',
}

# 显式豁免的重复 H1：两个「包管理器」分别是面向用户的上手指南与
# 面向维护者的参考文档，标题已区分为「使用指南 / 参考文档」。
H1_EXEMPT = {
    ('包管理器', 'guide/packaging.md', 'reference/package/index.md'),
}


def main():
    if not os.path.isdir(DOCS):
        print('[ERROR] 未找到 docs/src/', file=sys.stderr)
        return 2

    files = all_md()
    zh = [p for p in files if '/en/' not in ('/' + rel(p))]
    published = [p for p in zh if is_published(p)]
    notes.append(f'中文侧 md 共 {len(zh)} 篇，其中参与构建 {len(published)} 篇')

    # ---------- 1. 孤儿页：从根可达性 ----------
    incoming = defaultdict(set)
    for p in published:
        r = rel(p)
        for link in parse_links(read(p), r):
            if link.startswith(('http://', 'https://', 'mailto:', 'javascript:')):
                continue
            cand = resolve(p, link)
            if cand and is_published(cand) and os.path.abspath(cand) != os.path.abspath(p):
                incoming[rel(cand)].add(r)

    # config.js 里显式列出的链接也算入边
    if os.path.exists(CONFIG):
        cfg = read(CONFIG)
        for p in published:
            r = rel(p)
            stem = '/' + r[:-3]
            if stem in cfg or '/' + os.path.dirname(r) + '/' in cfg:
                incoming[r].add('.vitepress/config.js')

    orphans = sorted(p for p in published
                     if not incoming.get(rel(p)) and rel(p) not in ORPHAN_EXEMPT)
    if orphans:
        problems.append(f'[孤儿] {len(orphans)} 篇文档无法从任何页面链接到达'
                        f'（豁免清单：scripts/ci/check-docs-orphan.py ORPHAN_EXEMPT）：\n'
                        + ''.join(f'         - {rel(p)}\n' for p in orphans))

    # ---------- 2. frontmatter 完整性 ----------
    no_title = []
    for d in SIDEBAR_DIRS:
        full = os.path.join(DOCS, d)
        if not os.path.isdir(full):
            continue
        for r, dirs, files in os.walk(full):
            dirs[:] = [x for x in dirs if x != 'en']
            for fn in files:
                if not fn.endswith('.md') or fn == 'README.md':
                    continue
                p = os.path.join(r, fn)
                t = read(p)
                if not t.startswith('---'):
                    no_title.append(f'{rel(p)}（无 frontmatter）')
                    continue
                fm = t.split('---', 2)[1] if t.count('---') >= 2 else ''
                if 'title' not in fm:
                    no_title.append(f'{rel(p)}（frontmatter 无 title）')
    if no_title:
        problems.append('[frontmatter] 侧栏自动生成目录中的文件缺少 title（'
                        'config.js 用 useTitleFromFrontmatter，侧栏会显示空标题）：\n'
                        + ''.join(f'         - {x}\n' for x in no_title))

    # ---------- 3. 重复 H1 ----------
    h1 = defaultdict(list)
    for p in published:
        m = re.search(r'^#\s+(.+?)\s*$', read(p), re.M)
        if m:
            h1[m.group(1).strip()].append(rel(p))
    dup = {k: v for k, v in h1.items()
           if len(v) > 1 and (k, *sorted(v)) not in H1_EXEMPT
           and tuple(sorted([k] + v)) not in H1_EXEMPT}
    if dup:
        problems.append('[H1] 站内存在重复的一级标题（易让读者以为是同一页）：\n'
                        + ''.join(f'         - 「{k}」出现在 {", ".join(v)}\n'
                                  for k, v in sorted(dup.items())))

    # ---------- 4. 页内相对链接死链预检 ----------
    dead = []
    outside = []
    for p in published:
        t = read(p)
        for link in parse_links(t, rel(p)):
            if link.startswith(('http://', 'https://', 'mailto:', 'javascript:')):
                continue
            # 纯锚点、以及明显是代码/命令片段的误抓
            if not link or link.startswith('#'):
                continue
            # RFC 里讨论历史/提案中的 URL 结构（如 006 里的 /zh/getting-started），
            # 那是方案文本不是当前链接，VitePress 也不会校验。
            if rel(p).startswith('design/rfc/') and link.startswith(('/zh', '/en/zh')):
                continue
            # `](1,2,3)` 这类是正则/表格里的误抓
            if re.fullmatch(r'[\d\s,.\-+]+', link):
                continue
            cand = resolve(p, link)
            if cand is None:
                dead.append(f'{rel(p)} -> {link}')
            elif not os.path.abspath(cand).startswith(os.path.abspath(DOCS) + os.sep):
                outside.append(f'{rel(p)} -> {link}')
    if dead:
        problems.append(f'[死链] {len(dead)} 个页内相对链接指向不存在的文件：\n'
                        + ''.join(f'         - {d}\n' for d in sorted(dead)[:30]))
    if outside:
        problems.append(
            f'[越界链接] {len(outside)} 个相对链接指向 docs/src/ 之外'
            f'（VitePress 会判为死链，请改用行内代码或站内链接）：\n'
            + ''.join(f'         - {d}\n' for d in sorted(outside)[:30]))

    out = os.path.join(ROOT, 'scripts', 'ci', '_docs-orphan-report.txt')
    with io.open(out, 'w', encoding='utf-8') as f:
        for n in notes:
            f.write(n + '\n')
        f.write('\n')
        f.write('\n'.join(problems) if problems else '无问题\n')
    print('\n'.join(notes))
    if problems:
        print(f'\n发现 {len(problems)} 类结构问题，详见 scripts/ci/_docs-orphan-report.txt')
        return 1
    print('\n文档结构检查通过。')
    return 0


if __name__ == '__main__':
    sys.exit(main())
