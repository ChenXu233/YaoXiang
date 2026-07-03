#!/usr/bin/env python3
"""
check-rfc-tracking.py - 验证 RFC 目录中的 frontmatter 完整性并生成 TRACKING.md

扫描 docs/src/design/rfc/ 下所有状态子目录中的 .md 文件，
解析 YAML frontmatter，验证必填字段，生成 TRACKING.md 追踪表。

用法:
    python scripts/check-rfc-tracking.py

退出码:
    0 - 无错误（可能有警告）
    1 - 发现错误（新文件缺少必填字段）
"""

import os
import re
import sys
from datetime import date

# ── 常量 ─────────────────────────────────────────────────────────────────

RFC_ROOT = os.path.join("docs", "src", "design", "rfc")
TRACKING_FILE = os.path.join(RFC_ROOT, "TRACKING.md")

# 状态 -> 目录名映射
STATUS_DIR_MAP = {
    '草案': 'draft',
    '审核中': 'review',
    '已接受': 'accepted',
    '已实现': 'implemented',
    '已废弃': 'deprecated',
    '已拒绝': 'rejected',
}

DIR_TO_STATUS = {v: k for k, v in STATUS_DIR_MAP.items()}

REQUIRED_FIELDS = ['title', 'author', 'created', 'updated']
TRACKING_FIELDS = ['issue', 'issues_impl', 'pr_impl']

# 2026-07-03 之后创建的 RFC 必须填写 issue 字段
CUTOFF_DATE = date(2026, 7, 3)

# 需要跳过的文件（非实际 RFC）
SKIP_FILES = frozenset({
    'index.md',
    'RFC_TEMPLATE.md',
    'EXAMPLE_full_feature_proposal.md',
    'README.md',
    '.gitkeep',
})


# ── 前端解析 ─────────────────────────────────────────────────────────────

def parse_frontmatter(filepath):
    """解析 .md 文件的 YAML-like frontmatter。

    将 --- 分隔符之间的内容解析为字典。
    列表字段（issues_impl, pr_impl）会解析为字符串列表。

    Args:
        filepath: .md 文件的路径。

    Returns:
        包含 frontmatter 字段的字典。如果没有 frontmatter 则返回空字典。
    """
    result = {}
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except UnicodeDecodeError:
        try:
            with open(filepath, 'r', encoding='gbk') as f:
                lines = f.readlines()
        except (IOError, OSError, UnicodeDecodeError):
            print(f"  [ERROR] 无法读取文件（编码问题）: {filepath}", file=sys.stderr)
            return {}
    except (IOError, OSError) as e:
        print(f"  [ERROR] 无法读取文件: {e}", file=sys.stderr)
        return result

    # 查找第一个 --- 分隔符
    start = -1
    for i, line in enumerate(lines):
        if line.strip() == '---':
            start = i
            break

    if start == -1:
        return result  # 没有 frontmatter

    # 查找第二个 --- 分隔符
    end = -1
    for i in range(start + 1, len(lines)):
        if lines[i].strip() == '---':
            end = i
            break

    if end == -1:
        return result  # 没有闭合的 frontmatter

    # 解析 frontmatter 行
    current_list_key = None
    for line in lines[start + 1:end]:
        stripped = line.rstrip('\n\r')

        # 检查是否是列表项（以空格 + "- " 开头）
        list_match = re.match(r'^\s+-\s+(.+)$', stripped)
        if list_match and current_list_key is not None:
            item = list_match.group(1).strip().strip('"').strip("'")
            if isinstance(result.get(current_list_key), list):
                result[current_list_key].append(item)
            continue

        # 检查是否是键值对（key: value）
        kv_match = re.match(r'^([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*(.*)$', stripped)
        if kv_match:
            key = kv_match.group(1)
            value = kv_match.group(2).strip()

            # 如果值以引号开头和结尾，去除引号
            if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
                value = value[1:-1]
            elif len(value) >= 2 and value[0] == "'" and value[-1] == "'":
                value = value[1:-1]

            # 空值或无值表示这是一个列表字段的开始
            if not value:
                result[key] = []
                current_list_key = key
            else:
                result[key] = value
                current_list_key = None
        else:
            # 空行重置当前的列表键
            if not stripped.strip():
                current_list_key = None

    return result


# ── 状态推导 ─────────────────────────────────────────────────────────────

def derive_state(dirname):
    """从目录名推导 RFC 状态。

    Args:
        dirname: 目录名（如 'draft', 'review'）。

    Returns:
        中文状态字符串（如 '草案', '审核中'），未知目录返回 '未知'。
    """
    return DIR_TO_STATUS.get(dirname, '未知')


# ── 验证 ─────────────────────────────────────────────────────────────────

def validate_rfc(frontmatter, filepath, dirname):
    """验证 RFC frontmatter 的完整性。

    根据 RFC 创建日期决定验证严格程度：
    - CUTOFF_DATE 之后：缺少 issue 字段 → ERROR
    - CUTOFF_DATE 之前：缺少字段 → WARNING

    Args:
        frontmatter: 解析后的 frontmatter 字典。
        filepath: 文件的相对路径（用于错误信息）。
        dirname: 所在目录名。

    Returns:
        (level, message) 元组列表。level 为 'ERROR' 或 'WARNING'。
    """
    errors = []

    # 检查必填字段
    for field in REQUIRED_FIELDS:
        if field not in frontmatter or not frontmatter[field]:
            msg = f"{filepath}: 缺少必填字段 '{field}'"
            errors.append(('WARNING', msg))

    # 检查创建日期以决定 issue 验证严格程度
    created_str = frontmatter.get('created', '')
    try:
        created_date = date.fromisoformat(created_str) if created_str else CUTOFF_DATE
    except (ValueError, TypeError):
        created_date = CUTOFF_DATE  # 无法解析日期，按严格模式处理

    # 新 RFC 必须填写 issue
    if created_date >= CUTOFF_DATE:
        if 'issue' not in frontmatter or not frontmatter['issue']:
            msg = f"{filepath}: 新 RFC（创建于 {created_str}）缺少 'issue' 字段"
            errors.append(('ERROR', msg))
    else:
        if 'issue' not in frontmatter or not frontmatter['issue']:
            msg = f"{filepath}: 旧 RFC（创建于 {created_str}）缺少 'issue' 字段"
            errors.append(('WARNING', msg))

    return errors


# ── 扫描 ─────────────────────────────────────────────────────────────────

def scan_rfcs(rfc_root=None):
    """扫描所有 RFC 目录并返回 RFC 记录列表。

    遍历 rfc_root 下的每个状态子目录，解析每个 .md 文件的 frontmatter。

    Args:
        rfc_root: RFC 根目录路径。默认为 RFC_ROOT。

    Returns:
        dict 列表，每个 dict 包含一个 RFC 文件的信息。
    """
    if rfc_root is None:
        rfc_root = RFC_ROOT

    records = []

    if not os.path.isdir(rfc_root):
        print(f"[ERROR] RFC 根目录不存在: {rfc_root}", file=sys.stderr)
        return records

    for dirname in sorted(os.listdir(rfc_root)):
        dirpath = os.path.join(rfc_root, dirname)
        if not os.path.isdir(dirpath):
            continue
        if dirname not in DIR_TO_STATUS:
            continue  # 只处理已知的状态目录

        for filename in sorted(os.listdir(dirpath)):
            if filename in SKIP_FILES:
                continue
            if not filename.endswith('.md'):
                continue

            filepath = os.path.join(dirpath, filename)
            if not os.path.isfile(filepath):
                continue

            frontmatter = parse_frontmatter(filepath)
            relpath = os.path.join(dirname, filename)

            record = {
                'filename': filename,
                'relpath': relpath,
                'dir': dirname,
                'state': derive_state(dirname),
                'title': frontmatter.get('title', ''),
                'author': frontmatter.get('author', ''),
                'created': frontmatter.get('created', ''),
                'updated': frontmatter.get('updated', ''),
                'issue': frontmatter.get('issue', ''),
                'issues_impl': frontmatter.get('issues_impl', []),
                'pr_impl': frontmatter.get('pr_impl', []),
                '_frontmatter': frontmatter,
            }
            records.append(record)

    return records


# ── 生成 TRACKING.md ────────────────────────────────────────────────────

def generate_tracking_md(records):
    """从 RFC 记录列表生成 TRACKING.md 内容。

    生成包含以下列的 markdown 表格：
    编号 | 标题 | 状态 | 文件 | Issue | 实现 Issues | 实现 PRs

    Args:
        records: scan_rfcs() 返回的 RFC 记录列表。

    Returns:
        TRACKING.md 的字符串内容。
    """
    lines = []
    lines.append("# RFC 追踪表")
    lines.append("")
    lines.append("| 编号 | 标题 | 状态 | 文件 | Issue | 实现 Issues | 实现 PRs |")
    lines.append("| --- | --- | --- | --- | --- | --- | --- |")

    for r in records:
        filename = r['filename']
        title = r['title'] or '(无标题)'
        state = r['state']

        issue = str(r['issue']) if r['issue'] else '--'

        issues_impl = ', '.join(r['issues_impl']) if r['issues_impl'] else '--'
        pr_impl = ', '.join(r['pr_impl']) if r['pr_impl'] else '--'

        relpath = r.get('relpath', filename)
        lines.append(f"| {filename} | {title} | {state} | {relpath} | {issue} | {issues_impl} | {pr_impl} |")

    lines.append("")
    lines.append("> 此文件由 check-rfc-tracking.py 自动生成，请勿手动修改。")
    lines.append("")

    return '\n'.join(lines)


# ── 主入口 ───────────────────────────────────────────────────────────────

def main():
    """主入口函数。

    扫描、验证、报告并生成 TRACKING.md。

    Returns:
        退出码：0（无错误）或 1（有 ERROR）。
    """
    records = scan_rfcs()

    if not records:
        print("[INFO] 未找到 RFC 文件。")
        sys.stdout.flush()
        return 0

    # 验证所有记录
    all_errors = []
    for r in records:
        filepath = r['relpath']
        dirname = r['dir']
        frontmatter = r['_frontmatter']
        errors = validate_rfc(frontmatter, filepath, dirname)
        all_errors.extend(errors)

    # 报告结果
    has_error = False
    if all_errors:
        print("\n=== RFC 验证结果 ===\n")
        for level, msg in all_errors:
            prefix = f"[{level}]"
            if level == 'ERROR':
                has_error = True
                print(f"  {prefix} {msg}")
            else:
                print(f"  {prefix} {msg}")

    # 生成 TRACKING.md
    tracking_dir = os.path.dirname(TRACKING_FILE)
    if tracking_dir and not os.path.exists(tracking_dir):
        os.makedirs(tracking_dir, exist_ok=True)

    md_content = generate_tracking_md(records)
    try:
        with open(TRACKING_FILE, 'w', encoding='utf-8') as f:
            f.write(md_content)
        print(f"\n[INFO] 已生成 {TRACKING_FILE}")
    except (IOError, OSError) as e:
        print(f"[ERROR] 无法写入 TRACKING.md: {e}", file=sys.stderr)
        return 1

    print(f"[INFO] 共扫描 {len(records)} 个 RFC 文件，"
          f"{len([e for e in all_errors if e[0] == 'WARNING'])} 个警告，"
          f"{len([e for e in all_errors if e[0] == 'ERROR'])} 个错误")

    return 1 if has_error else 0


if __name__ == '__main__':
    sys.exit(main())
