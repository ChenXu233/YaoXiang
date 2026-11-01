#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-docs-refs.py —— 「文档引用的路径必须任何人都能解析」门禁

背景（2026-10-21 审计发现）：
  AGENTS.md 元规则 ① 要求文档里的任何人都能独立解析；第 10 条④ 要求引用
  只能是「仓库内路径 / revision / run id / issue 号」。但「仓库内路径」这
  四个字当时没有区分「**入库**路径（git 跟踪）」与「**工作区内、被
  .gitignore 排除**的路径」——被忽略的批次树目录字面在仓库内，clone 之后
  却根本不存在。于是 09 / 09a / CHANGELOG / 分支维护指南里积累了 24 处
  只有本机才有的「证据出处」，外部贡献者一条数字都复核不了；其中一处
  （t7 收口轮）把旧路径改指向另一个本地路径，等于把违规下移一层而不是消除。

判据（机械、可复算）：
  1. 取 git 跟踪的全部文本文件（md / rs / py / sh / toml / yml / json / ps1）。
  2. 抽取反引号内联代码片段与 markdown 链接目标中的「路径状」token：含 "/"
     且不是 URL、不是命令行参数。
  2b. #521（2026-11-01）：.md 文件另做**裸文本**抽取——反引号/链接目标掩码后
     的自由文本里同样路径状的 token（证据出处的主体面是文档；源码/脚本内的
     裸文本引用面另行登记）。抽取面放宽，判定与豁免完全复用第 3–5 步。
  3. 用 `git check-ignore -v` 判定该 token 是否落在 .gitignore 覆盖范围内。
  4. 命中、且该 token 在被忽略目录下**有具体子路径** → 违规（这条引用在
     别的机器上打不开）。
  5. 占位符形态（如 `target/yx-scratch/<batch>/tree`）**不**算违规：那是规则
     陈述，描述「工具把产物放哪」，不是「拿它当证据出处」；同理裸目录名
     `target/` 也不算。

用法：
  python scripts/ci/check-docs-refs.py            # 检查
  python scripts/ci/check-docs-refs.py --selftest  # 自测（判据能红能绿）

退出码：0 = 无违规；1 = 有违规（CI 应失败）；2 = 脚本自身无法完成判定。
"""
import io
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
REPORT = os.path.join(ROOT, 'scripts', 'ci', '_docs-refs-report.txt')

TEXT_EXT = {'.md', '.rs', '.py', '.sh', '.bash', '.toml', '.yml', '.yaml', '.json', '.ps1'}

# 豁免表：被 .gitignore 排除、但文档里出现的是「**系统/工具的运行时目录**」——
# 即规格正文规定工具写在哪、或本地构建产物的位置，而不是「拿它当证据出处」。
# 每条必须写明理由；新增条目同样要写。**工作区里的证据/归档目录不在此表内**
# （target/yx-scratch/、target/wbs39-evidence/ 等一律按违规处理）。
ALLOWED_PREFIXES = {
    'cache/': '包管理器缓存目录（RFC-014 规定工具写入的位置）——规格，不是证据引用',
    'target/yxpkg/': '打包产物目录（构建输出）——规格，不是证据引用',
    'target/distrib/': '分发产物目录（构建输出）——规格，不是证据引用',
    'target/debug/': 'cargo 构建产物目录（本地可执行文件位置）——不是证据引用',
    'target/release/': 'cargo release 构建产物目录（本地可执行文件位置）——不是证据引用',
    'target/clippy/': 'pre-commit clippy 钩子的独立缓存目录（.pre-commit-config.yaml 规定的工具写入位置）——规格，不是证据引用',
    'target/bench-': 'bench 历史脚本的本地输出文件（运行时产物）——不是证据引用',
    'target/shootout': 'bench 基准结果输出文件（脚本运行时产物）——不是证据引用',
    'target/hist-local': 'bench 历史本地库目录（运行时产物）——不是证据引用',
    'target/test-budget/': '测试预算通道的本地输出目录（运行时产物）——不是证据引用',
    'target/…': '省略号形态的构建产物目录说明——不是证据引用',
    '.claude/': '本地 agent 工作台目录（不入库；文档讲的是本地用法与历史记录）——不是证据引用',
    'docs/public/wasm/': 'VitePress playground 的发布产物目录（构建输出 / 发布目标）——规格，不是证据引用',
    'yaoxiang/cache/': 'RFC-014/029a 包管理器缓存布局示例（规格陈述）——不是证据引用',
    'cargo/registry/cache/': 'RFC-005（已否决）中的 cargo 缓存布局示例（规格陈述）——不是证据引用',
    'dist/': 'RFC-015/037 分发产物目录布局示例（规格陈述）——不是证据引用',
    'node_modules/': 'RFC-015 配置示例中的 Node 依赖目录（规格陈述）——不是证据引用',
    'pkg/msi': 'RFC-037 打包布局示例（规格陈述）——不是证据引用',
    'mylib/target/yxpkg/': '打包指南示例的产物路径（规格陈述）——不是证据引用',
}

# 内联代码片段 + markdown 链接目标
INLINE_CODE = re.compile(r'`([^`\n]+)`')
MD_LINK = re.compile(r'\]\(([^)\s]+)\)')

# 裸文本路径状 token（#521：仅对 .md 启用——证据出处的主体面是文档；
# 源码/脚本里的裸文本引用面另行登记）。只认正斜杠（文档与 .gitignore
# 规则的通行写法）；判定仍由 classify() 的 check-ignore + 豁免表兜底，
# 抽取面放宽不改变红绿判据。
BARE_TEXT = re.compile(r'[A-Za-z0-9_][A-Za-z0-9_./-]*[/][A-Za-z0-9_./-]*')

# 明显不是「路径」的 token
SKIP_PREFIX = ('http://', 'https://', 'mailto:', 'ftp://', '//')
SKIP_EXACT = ('-', '--help', '--version')


def run(cmd, stdin=None):
    # 字节输入：Windows 上 text=True 会把 '\n' 转成 '\r\n'，git check-ignore --stdin
    # 只按 '\n' 切行，尾部 '\r' 会被当成路径的一部分（2026-10-21 实测）。
    data = stdin.encode('utf-8') if isinstance(stdin, str) else stdin
    proc = subprocess.run(cmd, cwd=ROOT, input=data, capture_output=True)
    proc.out_text = proc.stdout.decode('utf-8', 'replace')
    proc.err_text = proc.stderr.decode('utf-8', 'replace')
    return proc


def tracked_text_files():
    out = run(['git', 'ls-files', '-z'])
    if out.returncode != 0:
        raise RuntimeError('git ls-files 失败: ' + out.err_text.strip())
    files = []
    for rel in out.out_text.split('\0'):
        if not rel:
            continue
        if os.path.splitext(rel)[1].lower() not in TEXT_EXT:
            continue
        files.append(rel)
    return files


def read_text(rel):
    p = os.path.join(ROOT, rel)
    try:
        with io.open(p, 'r', encoding='utf-8') as f:
            return f.read()
    except (UnicodeDecodeError, OSError):
        return None


def _common_token_filters(span):
    """裸文本与内联代码共用的 token 过滤（与 path_like_tokens 的既有判据同源）。"""
    if not span or ' ' in span or '\t' in span:
        return None
    if span.startswith(SKIP_PREFIX) or span in SKIP_EXACT:
        return None
    if '/' not in span:
        return None
    if span.startswith('-'):
        return None
    # 越界 / 绝对路径不是「仓库内引用」，且会让 git check-ignore 整批报
    # "fatal: '../' is outside repository"（2026-10-21 实测）——直接剔除。
    if span.startswith('/') or re.match(r'^[A-Za-z]:[\\/]', span):
        return None
    # 以 ~ 开头的是**用户主目录**路径（~/.yaoxiang/、~/.cargo/ …）——那是「讲本机
    # 环境配置」的内容，不是仓库内引用（AGENTS 元规则 ① 的例外），一律不判。
    if span.startswith('~'):
        return None
    if '..' in span.split('/'):
        return None
    if '://' in span:
        return None
    # 只取形如 a/b 的前缀（去掉 :行号 / :列 之类后缀）
    token = span.split(':')[0] if re.match(r'^[A-Za-z0-9_./-]+:[0-9]', span) else span
    token = token.rstrip('.,;:)\'"')
    if '/' not in token:
        return None
    return token


def path_like_tokens(text):
    """抽出路径状 token -> [(行号, 列号, token)]（反引号内联代码 + 链接目标）"""
    hits = []
    for regex in (INLINE_CODE, MD_LINK):
        for m in regex.finditer(text):
            token = _common_token_filters(m.group(1).strip())
            if token is None:
                continue
            line = text.count('\n', 0, m.start()) + 1
            col = m.start(1) - (text.rfind('\n', 0, m.start()) + 1) + 1
            hits.append((line, col, token))
    return hits


def bare_text_tokens(text):
    """裸文本路径状 token -> [(行号, 列号, token)]（#521，仅 .md 调用）。

    先把反引号片段、链接目标、带 scheme 的 URL 与 ~ 主目录路径掩掉（前两处
    由 path_like_tokens 负责；后两处的 token 会被 BARE_TEXT 从中段咬出
    「github.com/…」「yaoxiang/…」这类尾巴，须整体排除），再在剩余文本上找
    路径状连续段。判定与豁免全部复用 classify()——抽取面放宽不改变红绿判据。
    """
    masked = INLINE_CODE.sub(lambda m: '`' * len(m.group(0)), text)
    masked = MD_LINK.sub(lambda m: '(' + ' ' * max(0, len(m.group(0)) - 2) + ')', masked)
    masked = re.sub(r'\b(?:https?|ftp)://\S+', lambda m: ' ' * len(m.group(0)), masked)
    masked = re.sub(r'(?<![A-Za-z0-9_])~[/\\][^\s]*', lambda m: ' ' * len(m.group(0)), masked)
    hits = []
    for m in BARE_TEXT.finditer(masked):
        token = _common_token_filters(m.group(0))
        if token is None:
            continue
        line = text.count('\n', 0, m.start()) + 1
        col = m.start() - (text.rfind('\n', 0, m.start()) + 1) + 1
        hits.append((line, col, token))
    return hits


def _unquote(path):
    """git 在输出里会给含特殊字符的路径加 C 风格引号——解回来。"""
    if len(path) >= 2 and path.startswith('"') and path.endswith('"'):
        body = path[1:-1]
        out = []
        i = 0
        while i < len(body):
            ch = body[i]
            if ch == '\\' and i + 1 < len(body):
                nxt = body[i + 1]
                mapping = {'n': '\n', 't': '\t', 'r': '\r', '"': '"', '\\': '\\'}
                if nxt in mapping:
                    out.append(mapping[nxt])
                    i += 2
                    continue
                if nxt.isdigit():
                    out.append(chr(int(body[i + 1:i + 4], 8)))
                    i += 4
                    continue
            out.append(ch)
            i += 1
        return ''.join(out)
    return path


def classify(candidates):
    """候选 token -> [(rel, line, col, token, pattern)]（被 .gitignore 命中且有具体子路径）"""
    exemptions = []
    if not candidates:
        return [], exemptions
    uniq = sorted({tok for (_rel, _l, _c, tok) in candidates})
    # 调用方式：**argv 分批**（不用 --stdin）。原因（2026-10-21 实测）：
    #   - Windows 版 git 2.55 的 --stdin 不按 '\n' 切行（多行输入被当成一个路径）；
    #   - Linux 版 git 的 --stdin 按 '\n' 切行，吃 NUL 分隔会直接报错。
    # 两边的 --stdin 语义不一致 ⇒ 改用两边都一致的 argv 形式，分批以避开命令行长度上限。
    # 判定要**两平台一致**（2026-10-21 实测踩坑）：
    #   - 同一 token，Linux 报真实规则、Windows 对「以 / 结尾的目录形态路径」报一条**空 pattern**
    #     （.gitignore 的空行），只认非空 pattern 的话 Windows 会漏判 → CI 红、本地绿。
    # 对策：每个 token 查三种形态（带斜杠 / 去斜杠 / 去斜杠 + 探针子项），只认**非空** pattern；
    # 目录形态靠「探针子项」命中（规则写的是 docs/public/wasm/* 这类子项通配）。
    ignored = {}
    variants = []
    for tok in uniq:
        stripped = tok.rstrip('/')
        for v in (tok, stripped, stripped + '/x'):
            if v and v not in variants:
                variants.append(v)
    CHUNK = 150
    for start in range(0, len(variants), CHUNK):
        chunk = variants[start:start + CHUNK]
        proc = run(['git', 'check-ignore', '-v', '--'] + chunk)
        if proc.returncode not in (0, 1):
            # git check-ignore 失败（非「无匹配」）→ 判据无法完成，必须显式失败，
            # 不能当成「没有违规」静默放行（2026-10-21 实测踩过：rc=128 时输出为空）。
            raise RuntimeError('git check-ignore 失败 rc=%d: %s' % (proc.returncode, proc.err_text.strip()[:400]))
        for line in proc.out_text.splitlines():
            if '\t' not in line:
                continue
            meta, path = line.split('\t', 1)
            parts = meta.split(':', 2)
            if len(parts) < 3:
                continue
            pattern = parts[2]
            if not pattern.strip():
                continue          # 空 pattern = .gitignore 空行，不是真规则
            ignored.setdefault(_unquote(path), pattern)
    violations = []
    for (rel, line, col, tok) in candidates:
        stripped = tok.rstrip('/')
        pattern = None
        for v in (tok, stripped, stripped + '/x'):
            if v and v in ignored:
                pattern = ignored[v]
                break
        if pattern is None:
            continue
        allowed = next((why for pre, why in ALLOWED_PREFIXES.items() if tok.startswith(pre)), None)
        if allowed is not None:
            exemptions.append((rel, line, col, tok, allowed))
            continue
        if not pattern.strip():
            # 空 pattern = .gitignore 里的空行；实测 git check-ignore 对「以 / 结尾的路径」
            # 会把最近一条空行报成匹配来源，但它不是一条真规则（真实文件并未被忽略）。
            # 这类命中一律不算违规，否则会误伤「以 / 结尾的目录引用」。
            continue
        if pattern.endswith('/'):
            root = pattern
            remainder = tok[len(root):] if tok.startswith(root) else ''
            if not remainder:
                continue            # 裸目录名（target/）——不算引用
            if '<' in remainder or '>' in remainder:
                continue            # 占位符形态——规则陈述，不算引用
        violations.append((rel, line, col, tok, pattern))
    return violations, exemptions


def scan():
    files = tracked_text_files()
    candidates = []
    for rel in files:
        text = read_text(rel)
        if text is None:
            continue
        for (line, col, tok) in path_like_tokens(text):
            candidates.append((rel, line, col, tok))
        # #521：裸文本抽取仅对 .md 启用（证据出处的主体面；源码/脚本里的
        # 裸文本引用面另行登记，见 issue #521 的 scope 决定）。
        if rel.lower().endswith('.md'):
            for (line, col, tok) in bare_text_tokens(text):
                candidates.append((rel, line, col, tok))
    return candidates, files


def selftest():
    """判据必须能红能绿：反例必红、正例必绿，否则「门禁能抓到」只是自述。"""
    cases = [
        ('反例：具体文件在被忽略目录下', 'target/yx-scratch/close-44/evidence-summary.md', True),
        ('反例：被忽略目录下的子路径', 'target/yx-scratch/wbs317/', True),
        ('反例：通配形态', 'target/yx-scratch/yx49/*.patch', True),
        ('正例：占位符形态（规则陈述）', 'target/yx-scratch/<batch>/tree', False),
        ('正例：裸目录名', 'target/', False),
        ('正例：入库路径', 'docs/src/dev/architecture/09a-wbs-history.md', False),
        ('正例：URL', 'https://github.com/ChenXu233/YaoXiang/issues/475', False),
        ('正例：用户主目录路径', '~/.yaoxiang/settings.toml', False),
        ('反例：跨分批边界仍判红', 'target/yx-scratch/batch-%d/evidence.md' % 149, True),
    ]
    ok = True
    for name, token, should_fail in cases:
        got, _ex = classify([('fixture.md', 1, 1, token)])
        got = bool(got)
        status = 'OK' if got == should_fail else 'FAIL'
        if got != should_fail:
            ok = False
        print('[%s] %-34s %s -> %s（期望 %s）' % (
            status, name, token, '红' if got else '绿', '红' if should_fail else '绿'))
    print('selftest:', 'PASS' if ok else 'FAIL')
    if not ok:
        return 1

    # #521：裸文本抽取面（仅 .md 调用；红绿判定复用 classify）。
    bare_cases = [
        ('裸文本：路径状 token 抽出', '详见 target/yx-scratch/x/evidence.md 的记录',
         ['target/yx-scratch/x/evidence.md']),
        ('裸文本：反引号内不重复计（掩码后只剩裸写那处）', '见 `docs/src/a/b.md` 与 docs/src/c/d.md',
         ['docs/src/c/d.md']),
        ('裸文本：URL 不抽', '参见 https://github.com/ChenXu233/YaoXiang/wiki/a/b 页面', []),
        ('裸文本：主目录不抽', '配置在 ~/.yaoxiang/settings.toml 生效', []),
        ('裸文本：冒号行号剥除', '见 docs/src/dev/HOWTO.md:12 的表', ['docs/src/dev/HOWTO.md']),
    ]
    for name, text, expected in bare_cases:
        got = [t for (_l, _c, t) in bare_text_tokens(text)]
        status = 'OK' if got == expected else 'FAIL'
        if got != expected:
            ok = False
        print('[%s] %-40s %s（期望 %s）' % (status, name, got, expected))
    # 裸文本端到端：抽出的 token 进 classify 必须判红（豁免/占位符规则不变）
    toks = [t for (_l, _c, t) in bare_text_tokens('详见 target/yx-scratch/x/evidence.md 的记录')]
    red = bool(classify([('fixture.md', 1, 1, t) for t in toks])[0])
    print('[%s] %-40s classify 判红 = %s' % ('OK' if red else 'FAIL', '裸文本端到端判红', red))
    if not red:
        ok = False
    print('selftest(裸文本):', 'PASS' if ok else 'FAIL')
    return 0 if ok else 1


def main(argv):
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding='utf-8')
        except (AttributeError, ValueError):
            pass
    if '--selftest' in argv:
        return selftest()
    try:
        candidates, files = scan()
    except Exception as exc:                                  # noqa: BLE001
        sys.stderr.write('[check-docs-refs] 无法完成判定: %s\n' % exc)
        return 2
    violations, exemptions = classify(candidates)
    lines = []
    for (rel, line, col, tok, pattern) in sorted(violations):
        lines.append('%s:%d:%d  %s   （被 .gitignore 规则 %s 命中）' % (rel, line, col, tok, pattern))
    report = [
        'check-docs-refs —— 文档引用可解析性门禁',
        '扫描跟踪文本文件：%d 个；路径状 token：%d 个；违规：%d 处；豁免：%d 处'
        % (len(files), len(candidates), len(violations), len(exemptions)),
        '',
    ] + lines + [
        '',
        '豁免明细（每条都写明理由）：',
    ] + ['%s:%d:%d  %s —— %s' % (rel, line, col, tok, why) for (rel, line, col, tok, why) in exemptions]
    try:
        with io.open(REPORT, 'w', encoding='utf-8') as f:
            f.write('\n'.join(report) + '\n')
    except OSError:
        pass
    if violations:
        sys.stderr.write('[check-docs-refs] 发现 %d 处不可解析的文档引用（别人 clone 后打不开）：\n' % len(violations))
        for l in lines:
            sys.stderr.write('  ' + l + '\n')
        sys.stderr.write('[check-docs-refs] 判据：引用必须指向 git 跟踪的入库路径、revision、run id 或 issue 号；\n'
                         '            工作区（.gitignore 覆盖）里的路径只能作为规则陈述出现，不能当证据出处。\n')
        return 1
    print('[check-docs-refs] OK：%d 个跟踪文本文件、%d 个路径状 token，无不可解析引用' % (len(files), len(candidates)))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
