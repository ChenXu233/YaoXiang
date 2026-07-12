#!/usr/bin/env python3
"""
RFC-GH Issue 同步编排器

主 agent 调用入口。负责:
1. 扫描 RFC 目录，按状态和优先级排序
2. 输出 RFC 清单供分波下发
3. 收集子 agent 结果，写入 JSONL
4. 调用 check_rfc_tracking.py 更新 TRACKING.md

使用:
  python scripts/rfc_sync.py scan          # 扫描并打印 RFC 清单 (JSON)
  python scripts/rfc_sync.py wave --size 5  # 生成当前波次的 RFC 列表
  python scripts/rfc_sync.py append-jsonl <json>  # 追加一行到 JSONL
  python scripts/rfc_sync.py update-tracking      # 重新生成 TRACKING.md
  python scripts/rfc_sync.py summary              # 输出汇总报告
"""

import json
import os
import sys
import subprocess

RFC_ROOT = os.path.join("docs", "src", "design", "rfc")
TRACKING_FILE = os.path.join(RFC_ROOT, "TRACKING.md")
JSONL_FILE = os.path.join(".omp", "tmp", "rfc-sync-report.jsonl")

# 复用 check_rfc_tracking.py 的解析和扫描逻辑
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from check_rfc_tracking import parse_frontmatter, scan_rfcs, DIR_TO_STATUS

# RFC 状态优先级（数值越小越优先）
PRIORITY_ORDER = {
    "accepted": 0,
    "review": 1,
    "draft": 2,
    "rejected": 3,
    "deprecated": 4,
}


def cmd_scan():
    """扫描并打印按优先级排序的 RFC 清单（JSON 格式）"""
    records = scan_rfcs()
    if not records:
        print("[]")
        return []

    # 按优先级排序
    records.sort(key=lambda r: (
        PRIORITY_ORDER.get(r["dir"], 99),
        r["filename"]
    ))

    output = []
    for r in records:
        output.append({
            "id": r["filename"].replace(".md", ""),
            "filename": r["filename"],
            "relative_path": r["relpath"],
            "state": r["state"],
            "dir": r["dir"],
            "title": r["title"],
            "has_issue": bool(r["issue"]),
            "has_issues_impl": bool(r["issues_impl"]),
            "has_pr_impl": bool(r["pr_impl"]),
            "priority": PRIORITY_ORDER.get(r["dir"], 99),
        })
    print(json.dumps(output, ensure_ascii=False, indent=2))
    return output


def cmd_wave(size=5):
    """从 JSONL 读取已处理记录，返回下一波待处理的 RFC 列表"""
    records = scan_rfcs()
    if not records:
        print("[]")
        return []

    records.sort(key=lambda r: (
        PRIORITY_ORDER.get(r["dir"], 99),
        r["filename"]
    ))

    # 读取已处理记录
    processed_ids = set()
    if os.path.exists(JSONL_FILE):
        with open(JSONL_FILE, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        entry = json.loads(line)
                        rfc_id = entry.get("rfc_id", "")
                        if rfc_id:
                            processed_ids.add(rfc_id)
                    except json.JSONDecodeError:
                        continue
    # 跳过已处理的 RFC
    pending = []
    for r in records:
        filename = r["filename"]
        rfc_id = filename.replace(".md", "")
        # 提取数字前缀用于匹配（"009-ownership-model" → "009"）
        prefix = rfc_id.split("-")[0] if "-" in rfc_id else rfc_id
        if prefix not in processed_ids and rfc_id not in processed_ids:
            pending.append({
                "id": rfc_id,
                "filename": filename,
                "relative_path": r["relpath"],
                "state": r["state"],
                "dir": r["dir"],
                "title": r["title"],
                "priority": PRIORITY_ORDER.get(r["dir"], 99),
            })
        elif rfc_id in processed_ids:
            # rfc_id 已在 processed_ids 中（如 "009-ownership-model"）
            pass
    wave = pending[:size]
    print(json.dumps(wave, ensure_ascii=False, indent=2))
    return wave

def cmd_append_jsonl(json_str):
    """追加一行 JSON 到 JSONL 文件"""
    os.makedirs(os.path.dirname(JSONL_FILE), exist_ok=True)
    try:
        parsed = json.loads(json_str)
    except json.JSONDecodeError as e:
        print(f"ERROR: 无效 JSON: {e}", file=sys.stderr)
        return 1

    with open(JSONL_FILE, "a", encoding="utf-8") as f:
        f.write(json.dumps(parsed, ensure_ascii=False) + "\n")
    rfc_id = parsed.get("rfc_id", "?")
    print(f"OK: 已追加 {rfc_id} 到 {JSONL_FILE}")
    return 0


def cmd_update_tracking():
    """调用 check_rfc_tracking.py 重新生成 TRACKING.md"""
    scripts_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(scripts_dir)
    result = subprocess.run(
        [sys.executable, os.path.join(scripts_dir, "check_rfc_tracking.py")],
        capture_output=True, text=True, cwd=project_root
    )
    print(result.stdout, end="")
    if result.returncode != 0:
        print(f"WARNING: check_rfc_tracking.py 返回非零: {result.stderr}", file=sys.stderr)
    return result.returncode


def cmd_summary():
    """读取 JSONL 输出汇总报告到控制台"""
    if not os.path.exists(JSONL_FILE):
        print("无已处理记录")
        return

    total = 0
    matched = 0
    created = 0
    mismatched = 0
    errors = 0
    impl_statuses = {}

    with open(JSONL_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue
            total += 1
            action = entry.get("issue_action", "")
            if action == "matched":
                matched += 1
            elif action == "created":
                created += 1
            elif action == "mismatch":
                mismatched += 1
            elif action == "skipped":
                pass

            if entry.get("errors"):
                errors += len(entry["errors"])
            status = entry.get("impl_status", "unknown")
            impl_statuses[status] = impl_statuses.get(status, 0) + 1

    print("=== RFC 同步最终报告 ===")
    print(f"总计: {total}/38 处理完成")
    print(f"✅ 完全匹配: {matched}")
    print(f"✅ 已创建新 issue: {created}")
    print(f"⚠️ 不匹配: {mismatched}")
    print(f"❌ 错误: {errors}")
    print(f"📊 实现情况:")
    for status, count in sorted(impl_statuses.items()):
        print(f"   {status}: {count}")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1

    cmd = sys.argv[1]
    if cmd == "scan":
        cmd_scan()
    elif cmd == "wave":
        size = int(sys.argv[2]) if len(sys.argv) > 2 else 5
        cmd_wave(size)
    elif cmd == "append-jsonl":
        if len(sys.argv) < 3:
            print("用法: rfc_sync.py append-jsonl <json>", file=sys.stderr)
            return 1
        return cmd_append_jsonl(sys.argv[2])
    elif cmd == "update-tracking":
        return cmd_update_tracking()
    elif cmd == "summary":
        cmd_summary()
    else:
        print(f"未知命令: {cmd}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())