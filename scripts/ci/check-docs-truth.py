#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-docs-truth.py —— 文档与源码事实对账门禁

背景（2026-10-02 审计发现）：
  现有 `check-docs-freshness.sh` 用 git mtime 判定时效，2026-09-25 一次批量格式化
  就把 20+ 篇过期文档的 mtime 全部刷新，STALE 归零；`check_tracking.py` 的 RFC 状态
  由目录反查，frontmatter 与目录不一致永远查不出。结果是「文档与源码矛盾」这一类
  最严重的问题没有任何自动化能发现。
  本脚本改为对账「事实」，而不是「时间」。

检查项：
  1. 标准库：src/std/ 实际模块 vs 文档声明的模块（幽灵模块检测）
  2. 版本号：Cargo.toml version vs 文档中出现的硬编码版本串
  3. 命令：src/main.rs 子命令表 vs 文档中出现的 yx 子命令
  4. 关键字：src/frontend/core/lexer/state.rs vs language-spec/syntax.md 关键字表

退出码：0 = 全部一致；1 = 存在差异（CI 应失败）；2 = 脚本自身无法完成对账
"""
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DOCS = os.path.join(ROOT, 'docs', 'src')
problems = []
notes = []


def read(path):
    with io.open(path, 'r', encoding='utf-8', errors='replace') as f:
        return f.read()


def walk_md(base, skip_dirs=('node_modules', '.vitepress', 'dist')):
    out = []
    for r, dirs, files in os.walk(base):
        dirs[:] = [d for d in dirs if d not in skip_dirs]
        for fn in files:
            if fn.endswith('.md'):
                out.append(os.path.join(r, fn))
    return out


def zh_docs():
    """只取中文侧（排除 en/）。"""
    return [p for p in walk_md(DOCS) if os.sep + 'en' + os.sep not in p]


# ---------------------------------------------------------------- 1. 标准库模块
def check_stdlib_modules():
    std_dir = os.path.join(ROOT, 'src', 'std')
    if not os.path.isdir(std_dir):
        notes.append('跳过标准库对账：未找到 src/std/')
        return

    src_modules = set()
    for fn in os.listdir(std_dir):
        if fn.endswith(('.rs', '.yx')) and fn != 'mod.rs':
            name = fn.rsplit('.', 1)[0]
            if name not in ('tests',):
                src_modules.add(name)

    doc_dir = os.path.join(DOCS, 'reference', 'stdlib')
    doc_modules = set()
    if os.path.isdir(doc_dir):
        for fn in os.listdir(doc_dir):
            if fn.endswith('.md') and fn != 'index.md':
                doc_modules.add(fn[:-3])

    # language-spec/stdlib.md 里的幽灵模块
    spec = os.path.join(DOCS, 'reference', 'language-spec', 'stdlib.md')
    if os.path.exists(spec):
        t = read(spec)
        for m in sorted(set(re.findall(r'std\.([a-z_][a-z0-9_]*)', t))):
            if m not in src_modules:
                problems.append(f'[标准库] language-spec/stdlib.md 提到 std.{m}，'
                                f'但 src/std/ 下无对应实现')

    undocumented = sorted(src_modules - doc_modules)
    if undocumented:
        notes.append(f'[标准库] 已实现但无文档页：{", ".join(undocumented)}'
                     f'（若确为内部模块请忽略）')
    notes.append(f'标准库：源码 {len(src_modules)} 模块 / 文档 {len(doc_modules)} 页')


# ---------------------------------------------------------------- 2. 版本号
def _version_is_incidental(line, rel):
    """判断一行里的版本号是否与「本项目自身版本」无关。

    门禁要抓的是「文档说项目是 0.7.14，实际已到 0.8.2」这类漂移。
    但文档里大量版本号是别的东西：依赖约束（`version = "1.0"`）、
    第三方库版本、语法演进的历史里程碑、示例数据。它们不是漂移。

    只在明确指向本项目时才判为问题。
    """
    low = line.lower()
    # 明确的第三方/依赖语境
    if any(k in low for k in ('version =', 'version="', 'version = "',
                              'dependencies', 'serde', 'tokio', 'ureq',
                              'rustls', 'cargo add', 'registry')):
        return True
    # 语义化约束记号
    if re.search(r'[~^><=]\s*\d', line):
        return True
    # 表格里成组的其他版本（同一行出现两个及以上版本号 = 多方对比）
    if len(re.findall(r'\d+\.\d+\.\d+', line)) >= 2:
        return True
    # 明显是历史/规划语境的措辞
    if any(k in line for k in ('历史', '曾经', '当年', '最初', '早期版本',
                               '发布于', 'v1.0 目标', '里程碑', '截至')):
        return True
    # 指向其他项目的版本（Rust / Z3 / Node 等）
    if re.search(r'(rust|z3|node|npm|llvm|libuv|toolchain\s+v)\s*[\d.]+', low):
        return True
    # 只有当这行明确谈论「本项目/编译器/语言/CLI」的版本时才判为漂移
    if re.search(r'(yaoxiang|爻象|编译器|CLI|工具链|本体)\s*'
                 r'(的)?\s*版本|v?\d+\.\d+\.\d+\s*(版|发布)', line, re.I):
        return False
    return True


def check_versions():
    cargo = os.path.join(ROOT, 'Cargo.toml')
    if not os.path.exists(cargo):
        return
    m = re.search(r'^version\s*=\s*"([^"]+)"', read(cargo), re.M)
    if not m:
        return
    cur = m.group(1)

    known = {cur}
    for p in zh_docs():
        rel = os.path.relpath(p, DOCS).replace('\\', '/')
        # archive/ 已排除发布；blog/ 是历史评论；explanation//rfc//dev/ 是
        # 理念、治理与贡献者文档，其中的版本号是有意记录的历史里程碑
        # （如 RFC-010b 的「0.7.12 起」）。
        if rel.startswith(('archive/', 'blog/', 'explanation/', 'rfc/', 'dev/')):
            continue
        t = read(p)
        for ln_no, line in enumerate(t.split('\n'), 1):
            for vm in re.finditer(r'\bv?(\d+\.\d+\.\d+)\b', line):
                ver = vm.group(1)
                if ver in known or not (ver.startswith('0.') or ver.startswith('1.')):
                    continue
                if _version_is_incidental(line, rel):
                    continue
                problems.append(f'[版本] {rel}:{ln_no} 出现硬编码版本 {ver}，'
                                f'当前 Cargo.toml 是 {cur}（请改为变量注入或更新）')

    # 版本自述行占位门禁：语言规范索引页声明「与 Cargo.toml 的 version 同步」，
    # 该行必须用 <!-- yx-version --> 占位（VitePress 构建期注入）。写死版本串
    # 与当前一致时上面的扫描抓不到，下次 bump 必漂移——2026-10-05 实证：
    # 0.8.2→0.8.3 bump 后全体进 main 的 PR 被判红。
    spec_index = os.path.join(DOCS, 'reference', 'language-spec', 'index.md')
    if os.path.exists(spec_index) and '<!-- yx-version -->' not in read(spec_index):
        problems.append('[版本] reference/language-spec/index.md 缺少 <!-- yx-version --> '
                        '占位——编译器版本行必须走构建期注入，不得写死版本串')
    notes.append(f'版本对账：当前 {cur}（archive/blog/explanation/rfc/dev 及依赖/历史语境不参与）')


# ---------------------------------------------------------------- 3. 子命令
def check_cli():
    main_rs = os.path.join(ROOT, 'src', 'main.rs')
    if not os.path.exists(main_rs):
        return
    txt = read(main_rs)
    # clap 的子命令通常写作 `enum Commands { Xxx(Args), ... }`
    m = re.search(r'enum\s+Commands?\s*\{(.*?)\n\}', txt, re.S)
    if not m:
        notes.append('跳过 CLI 对账：未在 src/main.rs 找到子命令枚举')
        return
    subs = set(re.findall(r'^\s*([A-Z][A-Za-z]*)\s*[({,]', m.group(1), re.M))
    subs = {s.lower() for s in subs if s not in ('Help',)}
    if not subs:
        return
    notes.append(f'CLI 对账：源码子命令 {sorted(subs)}')

    # 文档里出现但源码没有的 yx 子命令
    # 注意：`yx` 既是 CLI 名，也是爻象源文件的扩展名。
    # 文档里 `yx use foo` / `yx main` 这类多半是「.yx 文件中的代码」被正则误抓，
    # 因此只在明确带子命令上下文（行首 yx 或 $ yx）时判定，避免误报。
    known_verbs = subs | {'help', 'version'}
    for p in zh_docs():
        rel = os.path.relpath(p, DOCS).replace('\\', '/')
        if rel.startswith('archive/'):
            continue
        t = read(p)
        for cm in re.finditer(r'(?:^|[$#>]\s*|\bcargo\s+)yx\s+([a-z][a-z-]{1,20})\b',
                              t, re.M):
            verb = cm.group(1)
            if verb in known_verbs:
                continue
            if verb in ('toolchain', 'install', 'uninstall', 'default', 'list',
                        'update', 'publish', 'cache', 'clean', 'outdated',
                        'dump', 'eval', 'explain', 'lsp', 'workspace', 'rm'):
                continue
            # 代码标识符（use/main/pub/base 等语言构造）一律跳过；
            # `yx self` 之类是 tools/yx 工具链的子命令，不在主 CLI 里
            if verb in ('use', 'main', 'pub', 'base', 'let', 'fn', 'type',
                        'return', 'import', 'from', 'as', 'test', 'check',
                        'self'):
                continue
            line_no = t[:cm.start()].count('\n') + 1
            problems.append(f'[CLI] {rel}:{line_no} 出现 `yx {verb}`，'
                            f'但 src/main.rs 无此子命令')


# ---------------------------------------------------------------- 4. 关键字
def check_keywords():
    state = os.path.join(ROOT, 'src', 'frontend', 'core', 'lexer', 'state.rs')
    spec = os.path.join(DOCS, 'reference', 'language-spec', 'syntax.md')
    if not (os.path.exists(state) and os.path.exists(spec)):
        return
    txt = read(state)
    m = re.search(r'pub use ([A-Za-z_,\s:]+);', txt)
    if not m:
        return
    kws = {k.strip() for k in m.group(1).split(',') if k.strip()}
    src_listed = {k for k in kws if k in {
        'spawn', 'ref', 'mut', 'if', 'else', 'match', 'while', 'for', 'in',
        'return', 'break', 'continue', 'as', 'unsafe', 'and', 'or', 'pub',
        'type', 'let', 'fn', 'use'}}

    t = read(spec)
    # 只取关键字表格区域
    tbl = re.findall(r'^\|\s*`([a-z]+)`\s*\|', t, re.M)
    tbl_set = set(tbl)
    documented = src_listed & tbl_set
    absent = sorted(k for k in src_listed if k not in tbl_set)

    if absent:
        problems.append(f'[关键字] lexer 中存在但 syntax.md 关键字表未列：{absent}')
    notes.append(f'关键字：lexer {len(src_listed)} 个 / 文档表命中 {len(documented)} 个')


# ---------------------------------------------------------------- main
def main():
    if not os.path.isdir(DOCS):
        print('[ERROR] 未找到 docs/src/', file=sys.stderr)
        return 2
    check_stdlib_modules()
    check_versions()
    check_cli()
    check_keywords()

    out = io.open(os.path.join(ROOT, 'scripts', 'ci', '_docs-truth-report.txt'),
                  'w', encoding='utf-8')
    out.write('=== 对账备注 ===\n')
    for n in notes:
        out.write(f'  {n}\n')
    out.write('\n=== 发现的问题 ===\n')
    if problems:
        for p in problems:
            out.write(f'  {p}\n')
    else:
        out.write('  无\n')
    out.close()

    for n in notes:
        print(n)
    if problems:
        print(f'\n发现 {len(problems)} 个问题，详见 scripts/ci/_docs-truth-report.txt')
        return 1
    print('\n文档与源码事实一致。')
    return 0


if __name__ == '__main__':
    sys.exit(main())
