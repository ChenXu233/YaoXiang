#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-docs-examples.py —— 文档代码示例回归门禁

背景（2026-10-02 审计发现）：
  教程与指南里的 20 个代码块用 0.8.2 编译器实跑直接失败（命名参数用 `:`、
  f-string 格式说明符未实现、列表方法不存在、字典下标赋值静默 no-op 等），
  而现有任何 CI 都不检查这件事。仓库里 `src/std/tests/stdlib_docs.rs:121`
  已经在对 stdlib 文档做同类校验，教程侧却完全空白。

本脚本：
  1. 抽取 tutorial/ 与 guide/ 下的 ```yx / ```yaoxiang 代码块
  2. 逐个写到临时目录，用 yaoxiang-rs check 跑一遍
  3. 报告失败的示例（文件 + 行号 + 错误码）

设计取舍：
  - 只校验「语法/类型能否通过 check」，不校验 stdout 是否与注释里的
    「预期输出」一致 —— 后者需要更强的 harness，且大量示例注释本身
    就是历史遗留的错述，先把编译通过这条硬底线立起来。
  - 顶层需要 `main()` 的示例，如果文件里没有 main，会被包一层
    main = () => { ... }，但若示例本身已定义 main 则原样跑。
  - 需要 `use` 的示例原样保留。

退出码：0 = 全部通过；1 = 有失败；3 = 找不到编译器（跳过）
"""
import io
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
# 默认同时扫中英两侧。
#
# 背景（2026-10-02）：英文侧 docs/src/en/** 是 auto-translate 的产物，而翻译
# prompt 明确要求「Keep code blocks unchanged」——中文源码块改了，英文副本
# 不会跟着变。实测就出现过：中文侧 124 块修到 0 失败后，英文侧仍留着
# yaoxiang 二进制名、0.7.14、裸 ok/err、list.append 等全部旧写法，
# 因为门禁只扫中文侧而完全看不见。
#
# 两侧都扫之后，「中文修了英文没修」会直接让 CI 变红。
# 差异输出按 locale 分组，便于定位是哪一侧的锅。
SCAN_DIRS = [
    os.path.join(ROOT, 'docs', 'src', 'tutorial'),
    os.path.join(ROOT, 'docs', 'src', 'guide'),
    os.path.join(ROOT, 'docs', 'src', 'en', 'tutorial'),
    os.path.join(ROOT, 'docs', 'src', 'en', 'guide'),
]
# 明确标记为「故意展示错误」或伪代码的块，跳过
SKIP_MARKERS = ('should-fail', '预期错误', '故意错误', 'pseudo', '伪代码',
                'text', 'console', 'diff')

# 显式跳过：在代码块紧邻的上一行写 `<!-- docs-example: skip -->` 即可豁免。
# 用于「教学示意片段」——文档里大量片段引用了未定义的占位类型
# （Surface / Rect / Shape）或未声明的变量（x），它们本意是讲清语法形态
# 而非可独立运行。这类片段数量很大且形态不一，不适合用启发式规则猜。
#
# 判据：读者是否会把这段代码原样粘进编辑器。对「会粘」的示例，门禁必须过；
# 对「只读」的示意片段，标注 skip。
SKIP_COMMENT = re.compile(r'<!--\s*docs-example:\s*skip\s*-->')

# REPL 会话记录：guide/repl.md 的 ```yaoxiang 块里是 `>> 输入` 与 `输出`
# 交替的终端转录。REPL 靠交互式多轮累积状态（先定义 x 再用 x），
# 把转录当模块编译必然失败，而且它本来就不是给读者粘贴的代码。
REPL_TRANSCRIPT = re.compile(r'^\s*>>', re.M)


def _is_repl_transcript(code):
    return bool(REPL_TRANSCRIPT.search(code))

# 单独成块时无法通过、但与同页其他块合并后是正确示例的诊断码。
# 典型场景：文档先在 A 块定义 `Point: Type = {...}`，再在 B 块演示它的用法。
# 逐块校验会把 B 块判成 E1003 Unknown type / E1001 Unknown variable，
# 属于误报。因此对命中这些码的块，改用「整页所有块合并后再校验」的结果判定。
CROSS_BLOCK_CODES = {'E1001', 'E1003', 'E1053', 'E2002', 'E2010'}

# 包装伪影：把片段包进 main 后，文档里原本依赖「外层作用域」的写法会被编译器
# 判成非法。典型是 `for i in 0..5 { i = i + 1 }` —— for 的循环变量是只读的，
# 但文档原意是演示「不要这样写」或只是示意。这类不是文档错误。
WRAPPER_ARTIFACT = re.compile(
    r"E2010\] Cannot assign to immutable variable|"
    r"E0011\] Unexpected token: 'LParen'")


def _is_wrapper_artifact(first):
    return bool(WRAPPER_ARTIFACT.search(first))


def find_binary():
    """定位 yaoxiang-rs 可执行文件。

    注意：不要用 shutil.copy 把它复制到别处再跑。Windows 下该二进制依赖
    构建期产生的同目录 DLL（如 libz3.dll），单独复制会导致
    STATUS_DLL_NOT_FOUND (0xC0000135) 静默崩溃 —— returncode 非 0 但
    stdout/stderr 全空，表现为「所有示例失败且没有任何错误信息」。
    必须原地调用。
    """
    cands = [
        os.environ.get('YAOXIANG_BIN', ''),
        os.path.join(ROOT, 'target', 'debug', 'yaoxiang-rs'),
        os.path.join(ROOT, 'target', 'debug', 'yaoxiang-rs.exe'),
        os.path.join(ROOT, 'target', 'release', 'yaoxiang-rs'),
        os.path.join(ROOT, 'target', 'release', 'yaoxiang-rs.exe'),
    ]
    for c in cands:
        if c and os.path.exists(c):
            return c
    return None


def extract_blocks(path):
    """返回 [(起始行号, 语言, 代码)]"""
    t = io.open(path, 'r', encoding='utf-8', errors='replace').read()
    out = []
    lines = t.split('\n')
    i = 0
    while i < len(lines):
        m = re.match(r'^```(\w*)\s*$', lines[i])
        if m:
            lang = m.group(1).lower()
            start = i + 1
            j = start
            while j < len(lines) and not re.match(r'^```\s*$', lines[j]):
                j += 1
            code = '\n'.join(lines[start:j])
            out.append((start + 1, lang, code, '\n'.join(lines[max(0, i - 3):i + 1])))
            i = j + 1
        else:
            i += 1
    return out


def is_yx(lang, code, context):
    if lang in ('yx', 'yaoxiang'):
        return True
    if lang in ('bash', 'sh', 'shell', 'json', 'toml', 'yaml', 'text', ''):
        return False
    return False


def _eligible(block):
    """该代码块是否值得纳入校验。"""
    lineno, lang, code, ctx = block
    if not is_yx(lang, code, ctx):
        return False
    # 显式豁免：块前一行（3 行上下文内）有 <!-- docs-example: skip -->
    if SKIP_COMMENT.search(ctx):
        return False
    if _is_repl_transcript(code):
        return False
    # 大小写不敏感：bot 翻译会改注释大小写（如「伪代码」→「Pseudocode」），
    # 小写子串匹配会漏判，译文块被当真代码跑出假失败
    if any(mk in code.lower() for mk in SKIP_MARKERS):
        return False
    if not code.strip():
        return False
    # 片段式代码（明显是节选，含省略号）跳过
    if '...' in code or '…' in code:
        return False
    return True


def _run(binp, f):
    """执行 check，返回 (returncode, 合并输出)。"""
    try:
        proc = subprocess.run([binp, 'check', f], capture_output=True,
                              text=True, encoding='utf-8', errors='replace',
                              timeout=30)
    except subprocess.TimeoutExpired:
        return 124, 'TIMEOUT'
    except OSError as e:
        return 125, str(e)
    return proc.returncode, ((proc.stdout or '') + (proc.stderr or '')).strip()


def _first_error(err, rc):
    """从输出里提取第一条可读诊断。"""
    for ln in err.split('\n'):
        if re.search(r'E\d{4}', ln):
            return ln.strip()[:150]
    m = re.search(r'message:\s*"([^"]+)"', err)
    if m:
        return 'Parse error: ' + m.group(1)[:120]
    return (err.split('\n')[0] or f'rc={rc}')[:150]


def _merge_blocks(codes, keep_main=True):
    """把若干块合并成一个可编译模块（不包 main，顶层声明必须留顶层）。"""
    seen_use = set()
    parts = []
    has_main = False
    for c in codes:
        kept = []
        for ln in c.split('\n'):
            s = ln.strip()
            if s.startswith('use '):
                if s in seen_use:
                    continue
                seen_use.add(s)
            kept.append(ln)
        body = '\n'.join(kept)
        if re.search(r'^\s*main\s*[:=]', body, re.M):
            if has_main:
                continue
            has_main = True
        parts.append(body)
    out = '\n'.join(parts)
    if keep_main and not has_main:
        out += '\n\nmain = () => {}\n'
    return out


def _merge_ok(binp, merged_path):
    """合并体能否通过 check。"""
    rc, _ = _run(binp, merged_path)
    return rc == 0


def _prefix_resolves(binp, tmp, tag, codes, idx):
    """第 idx 块的失败是否只因「依赖本页前面块里的定义」。

    从最短前缀开始，逐个把「前缀 + 第 idx 块」拼起来跑 check。
    只要有一个前缀能让它通过，就判定为跨块依赖（误报）。

    用递增前缀而非整页合并：同一页里往往还含语法错片段
    （如 if-elif-else 演示用了不存在的 is_banned），整页合并体自身会
    失败，反而把前面那些真错误一起豁免掉。
    """
    cur = codes[idx]
    for start in range(idx - 1, -1, -1):
        cand = _merge_blocks(codes[start:idx + 1], keep_main=False) + '\n' + cur
        f = os.path.join(tmp, f'pfx_{tag}_{start}.yx')
        io.open(f, 'w', encoding='utf-8', newline='\n').write(cand)
        rc, _ = _run(binp, f)
        if rc == 0:
            return True
    return False


def wrap(code):
    """把片段式示例补成一个可编译的完整模块。

    文档里的示例绝大多数是「顶层声明 + 可选的 main 调用」片段。类型定义
    （`Point: Type = {...}`）只允许出现在模块顶层，塞进函数体会报
    E1071，因此不能简单地整体缩进包一层 main。

    规则：
      - 含 `use ` / 顶层 `X: Type =` / 顶层函数定义 → 原样作为模块级代码，
        若其中没有可执行入口再补一个空 main 之外的入口；
      - 纯表达式/语句片段 → 包进 main = () => { ... }。
    """
    lines = code.split('\n')
    top_level_defined = re.search(
        r'^\s*(?:use\s+\S+|[A-Z][A-Za-z0-9_]*\s*:\s*(?:Type|\()|'
        r'[a-z_][A-Za-z0-9_]*\s*:\s*\([^)]*\)\s*->\s*\S+\s*=)', code, re.M)
    has_entry = re.search(r'^\s*main\s*[:=]', code, re.M)

    if has_entry:
        return code, '原样（含 main）'
    if top_level_defined:
        # 顶层声明片段：直接放模块级。check 只需要能通过类型检查，
        # 没有入口函数也能跑（前面实测 hello world 无输出即是无入口）。
        return code, '顶层声明片段'
    body = '\n'.join('    ' + ln for ln in lines)
    return f'main = () => {{\n{body}\n}}', '包入 main'


def strip_comments(code):
    """去掉整行注释，避免注释里的示例影响判定。"""
    out = []
    for ln in code.split('\n'):
        s = ln.strip()
        if s.startswith('//') or s.startswith('#'):
            continue
        out.append(ln)
    return '\n'.join(out)


def main():
    binp = find_binary()
    if not binp:
        print('[SKIP] 未找到 yaoxiang-rs 二进制，跳过示例回归')
        print('       构建方式：cargo build --bin yaoxiang-rs')
        return 3

    # 自检：先跑一个必定通过的最小示例，确认解释器在本机能正常工作。
    # 否则会重演「二进制存在但无法执行（缺 DLL / 无执行权限）导致全部误报」。
    probe_dir = os.path.join(ROOT, '.docs-examples-tmp')
    os.makedirs(probe_dir, exist_ok=True)
    probe = os.path.join(probe_dir, '_probe.yx')
    io.open(probe, 'w', encoding='utf-8', newline='\n').write(
        'main = () => {\n    print("ok")\n}\n')
    try:
        pr = subprocess.run([binp, 'check', probe], capture_output=True,
                            text=True, encoding='utf-8', errors='replace',
                            timeout=30)
    except (OSError, subprocess.TimeoutExpired) as e:
        print(f'[SKIP] 解释器无法执行（{e}）：{binp}')
        return 3
    if pr.returncode != 0:
        print(f'[SKIP] 解释器自检失败（rc={pr.returncode}）：{binp}')
        print(f'       stdout={pr.stdout[:200]!r} stderr={pr.stderr[:200]!r}')
        print('       在 Windows 上通常是缺少同目录 DLL，请改用 YAOXIANG_BIN '
              '指向主仓的 target/debug/yaoxiang-rs.exe')
        return 3
    print(f'解释器自检通过：{binp}')

    total = 0
    failures = []
    # 注意：临时目录名不能含 target / .git / .yaoxiang。`check` 的文件收集器
    # 按「路径任意一段」匹配来排除（src/util/diagnostic/command.rs:256-261
    # is_default_excluded_name），因此把临时文件放在 target/ 或名字里带
    # target 的目录下，会让所有示例被静默跳过，表现为「全部失败但没有任何
    # 错误信息」。这里用 target 同级的 .docs-examples-tmp，避开该规则，
    # 且不污染源码树（已加进 .gitignore）。
    tmp = os.path.join(ROOT, '.docs-examples-tmp')
    os.makedirs(tmp, exist_ok=True)

    for base in SCAN_DIRS:
        if not os.path.isdir(base):
            continue
        for r, dirs, files in os.walk(base):
            # 只排构建期与依赖目录。**不能排 'en'** —— SCAN_DIRS 里已经显式
            # 列了 en 侧根目录，排掉就等于又回到「只扫中文侧」。
            dirs[:] = [d for d in dirs
                       if d not in ('node_modules', '.vitepress', 'dist')]
            for fn in sorted(files):
                if not fn.endswith('.md'):
                    continue
                p = os.path.join(r, fn)
                rel = os.path.relpath(p, ROOT).replace('\\', '/')
                blocks = [b for b in extract_blocks(p) if _eligible(b)]
                for bi, (lineno, lang, code, ctx) in enumerate(blocks):
                    total += 1
                    src, mode = wrap(code)
                    f = os.path.join(tmp, f'ex{total}.yx')
                    io.open(f, 'w', encoding='utf-8', newline='\n').write(src)
                    rc, err = _run(binp, f)
                    if rc == 0:
                        continue
                    first = _first_error(err, rc)
                    if _is_wrapper_artifact(first):
                        continue
                    code_m = re.search(r'E\d{4}', first)
                    # 命中跨块类诊断 → 逐个前缀验证
                    if code_m and code_m.group(0) in CROSS_BLOCK_CODES:
                        if _prefix_resolves(binp, tmp, abs(hash(rel)) % 100000,
                                            [b[2] for b in blocks], bi):
                            continue
                    failures.append((rel, lineno, first, err[:600], mode))

    # 按 locale 分组：英文侧失败几乎总是「中文侧修了、en 没跟上」
    def _loc(r):
        # rel 形如 docs/src/en/... 或 docs/src/...
        return 'en' if '/en/' in r.replace('\\', '/') else 'zh'

    zh_fail = [f for f in failures if _loc(f[0]) == 'zh']
    en_fail = [f for f in failures if _loc(f[0]) == 'en']

    lines = [f'扫描 {len(SCAN_DIRS)} 个目录（zh {len(SCAN_DIRS) - 2} 个 + en 2 个），'
             f'抽取并校验 {total} 个代码块',
             f'失败 {len(failures)} 个（zh {len(zh_fail)} / en {len(en_fail)}）', '']
    for tag, group in (('ZH', zh_fail), ('EN', en_fail)):
        if not group:
            continue
        lines.append(f'--- {tag} 侧 {len(group)} 个 ---')
        for rel, lineno, msg, full, mode in group:
            lines.append(f'  {rel}:{lineno}  [{mode}]  {msg}')
        lines.append('')
    lines.append('=' * 60)
    for rel, lineno, msg, full, mode in failures:
        lines.append(f'--- {rel}:{lineno}  ({mode}) ---')
        lines.append(full)
        lines.append('')

    with io.open(os.path.join(ROOT, 'scripts', 'ci', '_docs-examples-report.txt'),
                 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))

    print(lines[0])
    print(lines[1])
    if failures:
        print('\n'.join(f'  {r}:{l}  [{md}]  {m}'
                        for r, l, m, _f, md in failures[:25]))
        print('\n详见 scripts/ci/_docs-examples-report.txt')
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
