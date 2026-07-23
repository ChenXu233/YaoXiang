#!/usr/bin/env python3
"""
子 agent 辅助脚本。子 agent 调用来完成机械操作：
  1. 读取 RFC frontmatter
  2. 更新 frontmatter 字段（issue, issues_impl, pr_impl）
  3. 检查代码目录或文件是否存在
  4. 调用 gh CLI 搜索/创建/评论 issue

使用:
  python scripts/rfc/rfc_sync_agent.py read-frontmatter <path>
  python scripts/rfc/rfc_sync_agent.py update-frontmatter <path> <key> <value>
  python scripts/rfc/rfc_sync_agent.py update-frontmatter-list <path> <key> <item1> [item2 ...]
  python scripts/rfc/rfc_sync_agent.py check-modules <module1> [module2 ...]
  python scripts/rfc/rfc_sync_agent.py gh-search <query>
  python scripts/rfc/rfc_sync_agent.py gh-create-issue <title> <body>
  python scripts/rfc/rfc_sync_agent.py gh-comment <issue_number> <body>
"""

import json
import os
import re
import subprocess
import sys

# 复用 check_rfc_tracking.py 的解析函数
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from check_tracking import parse_frontmatter


def cmd_read_frontmatter(filepath):
    """读取 RFC 文件的 frontmatter"""
    data = parse_frontmatter(filepath)
    print(json.dumps(data, ensure_ascii=False))
    return 0


def cmd_update_frontmatter(filepath, key, value):
    """更新 RFC 文件 frontmatter 中的单值字段（如 issue: "#123"）

    自动处理:
    - 行首有缩进的字段（如 "   updated: ..."）
    - issue 字段: 自动将完整 URL 归一化为 #N 格式
    """
    # 自动归一化 issue 字段
    if key == "issue":
        import re
        m = re.search(r'#(\d+)', value)
        if m:
            value = f"#{m.group(1)}"
        elif not value.startswith("#"):
            value = f"#{value}"

    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    parts = content.split("---", 2)
    if len(parts) < 3:
        print(f"ERROR: {filepath} 缺少有效的 frontmatter", file=sys.stderr)
        return 1

    frontmatter = parts[1]
    lines = frontmatter.split("\n")

    found = False
    new_lines = []
    for line in lines:
        # 匹配带缩进的字段名（如 "   updated:"）
        stripped = line.lstrip()
        if stripped.startswith(key + ":"):
            indent = line[:len(line) - len(stripped)]  # 保持原始缩进
            new_lines.append(f"{indent}{key}: \"{value}\"")
            found = True
        else:
            new_lines.append(line)

    if not found:
        new_lines.append(f"{key}: \"{value}\"")
        # 确保 closing --- 在新行，不黏在前一行末尾
        new_lines.append("")

    parts[1] = "\n".join(new_lines)
    new_content = "---".join(parts)

    with open(filepath, "w", encoding="utf-8") as f:
        f.write(new_content)
    print(f"OK: {filepath} 的 {key} = {value}")
    return 0


def cmd_update_frontmatter_list(filepath, key, items):
    """更新 RFC 文件 frontmatter 中的列表字段（如 issues_impl: ["#1", "#2"]）"""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    parts = content.split("---", 2)
    if len(parts) < 3:
        print(f"ERROR: {filepath} 缺少有效的 frontmatter", file=sys.stderr)
        return 1

    frontmatter = parts[1]
    lines = frontmatter.split("\n")

    # 移除已有 key 及其列表项
    new_lines = []
    skip_block = False
    for line in lines:
        stripped = line.rstrip()
        if stripped.startswith(key + ":"):
            skip_block = True
            new_lines.append(f"{key}:")
            continue
        if skip_block:
            if re.match(r'^\s+- ', stripped):
                continue
            else:
                skip_block = False
        if not skip_block:
            new_lines.append(line)

    # 追加列表项
    for item in items:
        if item.startswith("#"):
            new_lines.append(f"  - \"{item}\"")
        else:
            new_lines.append(f"  - \"{item}\"")
    # 确保 closing --- 在新行
    new_lines.append("")
    parts[1] = "\n".join(new_lines)
    new_content = "---".join(parts)

    with open(filepath, "w", encoding="utf-8") as f:
        f.write(new_content)
    print(f"OK: {filepath} 的 {key} 已更新（{len(items)} 项）")
    return 0


def cmd_check_modules(modules):
    """检查模块/目录是否存在，返回 JSON 结果"""
    results = {}
    for module in modules:
        exists = os.path.exists(module)
        is_dir = os.path.isdir(module) if exists else False
        results[module] = {
            "exists": exists,
            "is_dir": is_dir,
            "full_path": os.path.abspath(module) if exists else "",
        }
    print(json.dumps(results, ensure_ascii=False))
    return 0


def cmd_gh_search(query):
    """调用 gh CLI 搜索 issue"""
    result = subprocess.run(
        ["gh", "search", "issues", "--repo", "yaoxiang-lang/YaoXiang",
         "--json", "number,title,url,state", "--", query],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"ERROR: gh search 失败: {result.stderr}", file=sys.stderr)
        return 1
    print(result.stdout)
    return 0


def cmd_gh_create_issue(title, body):
    """调用 gh CLI 创建 issue"""
    cmd = ["gh", "issue", "create",
           "--repo", "yaoxiang-lang/YaoXiang",
           "--label", "rfc",
           "--title", title,
           "--body", body]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"ERROR: gh issue create 失败: {result.stderr}", file=sys.stderr)
        return 1
    print(result.stdout.strip())
    return 0


def cmd_gh_comment(issue_number, body):
    """调用 gh CLI 回复 issue comment"""
    result = subprocess.run(
        ["gh", "issue", "comment", str(issue_number),
         "--repo", "yaoxiang-lang/YaoXiang",
         "--body", body],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"ERROR: gh comment 失败: {result.stderr}", file=sys.stderr)
        return 1
    print(f"OK: 已评论 issue #{issue_number}")
    return 0


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1

    cmd = sys.argv[1]
    if cmd == "read-frontmatter":
        if len(sys.argv) < 3:
            print("用法: rfc_sync_agent.py read-frontmatter <path>", file=sys.stderr)
            return 1
        return cmd_read_frontmatter(sys.argv[2])
    elif cmd == "update-frontmatter":
        if len(sys.argv) < 5:
            print("用法: rfc_sync_agent.py update-frontmatter <path> <key> <value>", file=sys.stderr)
            return 1
        return cmd_update_frontmatter(sys.argv[2], sys.argv[3], sys.argv[4])
    elif cmd == "update-frontmatter-list":
        if len(sys.argv) < 5:
            print("用法: rfc_sync_agent.py update-frontmatter-list <path> <key> <items...>", file=sys.stderr)
            return 1
        return cmd_update_frontmatter_list(sys.argv[2], sys.argv[3], sys.argv[4:])
    elif cmd == "check-modules":
        return cmd_check_modules(sys.argv[2:])
    elif cmd == "gh-search":
        if len(sys.argv) < 3:
            print("用法: rfc_sync_agent.py gh-search <query>", file=sys.stderr)
            return 1
        return cmd_gh_search(" ".join(sys.argv[2:]))
    elif cmd == "gh-create-issue":
        if len(sys.argv) < 4:
            print("用法: rfc_sync_agent.py gh-create-issue <title> <body>", file=sys.stderr)
            return 1
        return cmd_gh_create_issue(sys.argv[2], sys.argv[3])
    elif cmd == "gh-comment":
        if len(sys.argv) < 4:
            print("用法: rfc_sync_agent.py gh-comment <issue_number> <body>", file=sys.stderr)
            return 1
        return cmd_gh_comment(int(sys.argv[2]), sys.argv[3])
    else:
        print(f"未知命令: {cmd}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())