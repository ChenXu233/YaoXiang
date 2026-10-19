#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""批次验证树（scratch tree）的唯一入口 —— AGENTS.md 团队编排约定第 9 条。

为什么需要它
------------
验证 / 复审必须在**冻结的 revision** 上量测（结论要能被复现），而且变异驱动
验证会改源码——都不能在实现者正在写的主仓工作树里做。但「一个任务一棵树」
要付一次完整检出 + 一次全量构建（2~3 GB、约 5 分钟）。

关键事实：cargo 的指纹包含源码**路径**。
  * 换一棵树  => 全量重建
  * 同一棵树切 revision => 只重编差异（增量）

所以本脚本规定：**一个批次只建一棵树**，验证 / 复审通过 use/release 串行持有，
在树内切 revision 量测；批次收口时 gc 删除。

用法
----
  python scripts/dev/scratch.py init    <batch> [--rev <sha>]
  python scripts/dev/scratch.py use     <batch> --rev <sha> --by <member> --task <id> [--steal]
  python scripts/dev/scratch.py build   <batch> [--quiet]
  python scripts/dev/scratch.py release <batch> --by <member>
  python scripts/dev/scratch.py trial   <batch> --patch <p> -- <command...>   # 变异：apply→跑→restore 全自动
  python scripts/dev/scratch.py status  [--json] [--size]
  python scripts/dev/scratch.py rm      <batch> [--force]
  python scripts/dev/scratch.py gc      [--force]

硬约定
------
  * 树只能建在 <repo>/target/yx-scratch/<batch>/tree（仓库内、.gitignore 覆盖）
  * 同一个批次一棵树；确需真并行时才 init <batch>-b 第二棵（代价：再一次全量）
  * 同一时刻只允许一个成员持有（use 上锁 / release 解锁）
  * 禁止任何成员直接 git worktree add / git clone / 在仓库之外建目录
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")  # 控制台/管道编码不一致时中文会乱码
    except Exception:
        pass

BIN_NAME = "yaoxiang-rs"
BUILD_CMD = ["cargo", "build", "--bin", BIN_NAME]


def repo_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True, text=True, check=True,
    )
    return Path(out.stdout.strip())


def scratch_root(repo: Path) -> Path:
    return repo / "target" / "yx-scratch"


def batch_dir(repo: Path, batch: str) -> Path:
    return scratch_root(repo) / batch


def tree_path(repo: Path, batch: str) -> Path:
    return batch_dir(repo, batch) / "tree"


def meta_path(repo: Path, batch: str) -> Path:
    return batch_dir(repo, batch) / "meta.json"


def load_meta(repo: Path, batch: str) -> dict:
    p = meta_path(repo, batch)
    if not p.exists():
        die(f"没有这个批次树：{batch}（先 init）")
    return json.loads(p.read_text(encoding="utf-8"))


def save_meta(repo: Path, batch: str, meta: dict) -> None:
    meta_path(repo, batch).write_text(
        json.dumps(meta, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


def die(msg: str, code: int = 2):
    print(f"[scratch] 拒绝：{msg}", file=sys.stderr)
    sys.exit(code)


def run(cmd, cwd=None, capture=True):
    # git 的输出是 UTF-8（含 emoji/CJK 提交信息），必须显式解码，
    # 否则 Windows 上会按 gbk 解而在读取线程里抛 UnicodeDecodeError。
    return subprocess.run(cmd, cwd=cwd, capture_output=capture, text=True,
                          encoding="utf-8", errors="replace")


def head_rev(repo: Path) -> str:
    return run(["git", "rev-parse", "HEAD"], cwd=repo).stdout.strip()


def dir_size_mb(p: Path) -> float:
    total = 0
    for root, _dirs, files in os.walk(p):
        for f in files:
            fp = Path(root) / f
            try:
                total += fp.stat().st_size
            except OSError:
                pass
    return round(total / (1024 * 1024), 1)


def batch_names(repo: Path):
    root = scratch_root(repo)
    if not root.is_dir():
        return []
    return sorted(d.name for d in root.iterdir() if (d / "meta.json").exists())


# ───────────────────────── 子命令 ─────────────────────────

def cmd_init(args):
    repo = repo_root()
    batch = args.batch
    if meta_path(repo, batch).exists():
        die(f"批次树 {batch} 已存在（路径 {tree_path(repo, batch)}）。要重建先 rm。")
    rev = args.rev or head_rev(repo)
    path = tree_path(repo, batch)
    path.parent.mkdir(parents=True, exist_ok=True)
    r = run(["git", "worktree", "add", "--detach", str(path), rev], cwd=repo)
    if r.returncode != 0:
        die(f"git worktree add 失败：{r.stderr.strip() or r.stdout.strip()}")
    save_meta(repo, batch, {
        "batch": batch,
        "path": str(path),
        "base_rev": rev,
        "created_at": time.strftime("%Y-%m-%d %H:%M:%S"),
        "lock": None,
        "history": [],
    })
    print(f"[scratch] 已建批次树 {batch}")
    print(f"  路径   : {path}")
    print(f"  基线   : {rev}")
    print(f"  首次构建: python scripts/dev/scratch.py build {batch}   # 全量，约 5 分钟")
    print(f"  之后量测: python scripts/dev/scratch.py use {batch} --rev <sha> --by <member> --task <id>")


def cmd_use(args):
    repo = repo_root()
    meta = load_meta(repo, args.batch)
    lock = meta.get("lock")
    if lock and lock.get("by") != args.by and not args.steal:
        die(
            f"{args.batch} 正被 {lock['by']} 持有（task={lock.get('task')}，rev={lock.get('rev')}，"
            f"自 {lock.get('at')}）——同一时刻只允许一个持有者；确需抢占用 --steal。"
        )
    path = Path(meta["path"])
    if not path.is_dir():
        die(f"树目录不存在：{path}（用 rm 清掉元数据后重新 init）")
    prev = run(["git", "rev-parse", "HEAD"], cwd=path).stdout.strip()
    r = run(["git", "checkout", "--detach", args.rev], cwd=path)
    if r.returncode != 0:
        die(f"checkout {args.rev} 失败：{r.stderr.strip()}")
    now_rev = run(["git", "rev-parse", "HEAD"], cwd=path).stdout.strip()
    target_dir = path / "target"
    incremental = target_dir.is_dir()
    meta["lock"] = {
        "by": args.by, "task": args.task, "rev": now_rev,
        "at": time.strftime("%Y-%m-%d %H:%M:%S"), "prev_rev": prev,
    }
    meta.setdefault("history", []).append(meta["lock"])
    save_meta(repo, args.batch, meta)
    print(f"[scratch] {args.batch} 已交给 {args.by}（task={args.task}）")
    print(f"  树     : {path}")
    print(f"  revision: {now_rev}   (上一版 {prev})")
    print(f"  构建    : {'增量（target 已存在）' if incremental else '首次全量（target 尚不存在）'}")
    print(f"  命令    : python scripts/dev/scratch.py build {args.batch}")
    print(f"  二进制  : {path / 'target' / 'debug' / (BIN_NAME + '.exe')}")


def cmd_build(args):
    repo = repo_root()
    meta = load_meta(repo, args.batch)
    path = Path(meta["path"])
    print(f"[scratch] 在 {path} 构建（{' '.join(BUILD_CMD)}）")
    env = dict(os.environ)
    r = subprocess.run(BUILD_CMD, cwd=path, env=env, capture_output=args.quiet,
                       text=True, encoding="utf-8", errors="replace")
    if r.returncode != 0:
        if args.quiet:
            print((r.stdout or "")[-2000:])
            print((r.stderr or "")[-2000:], file=sys.stderr)
        die(f"构建失败（exit={r.returncode}）", code=1)
    print(f"[scratch] 构建完成：{path / 'target' / 'debug' / (BIN_NAME + '.exe')}")


def cmd_release(args):
    repo = repo_root()
    meta = load_meta(repo, args.batch)
    lock = meta.get("lock")
    if not lock:
        print(f"[scratch] {args.batch} 本来就没人持有")
        return
    if lock.get("by") != args.by and not args.force:
        die(f"{args.batch} 由 {lock['by']} 持有，{args.by} 不能替它释放（队长用 --force）。")
    meta["lock"] = None
    save_meta(repo, args.batch, meta)
    print(f"[scratch] {args.batch} 已释放")


def cmd_trial(args):
    """变异试验：git apply 变异补丁 → 跑命令 → git apply -R 还原 → 校验逐文件 blob 回到 HEAD。

    为什么必须机械可逆（AGENTS.md 第 10 条②）：验证者要证明「判据能失败」，
    唯一硬证据是故意把修复改坏看测试会不会红。但手工改文件、手工改回来，
    既可能忘、也可能改得不逐字节一致 —— 于是把它做成补丁 + 自动还原。
    """
    repo = repo_root()
    meta = load_meta(repo, args.batch)
    path = Path(meta["path"])
    patch = Path(args.patch)
    if not patch.is_absolute():
        patch = repo / patch
    if not patch.exists():
        die(f"补丁不存在：{patch}")
    dirty = run(["git", "status", "--porcelain"], cwd=path).stdout.strip()
    if dirty:
        die("树里已有未提交改动——先清理干净再 trial，否则分不清改动是谁的：\n" + dirty)
    r = run(["git", "apply", str(patch)], cwd=path)
    if r.returncode != 0:
        die(f"git apply 失败（树未改动）：{(r.stderr or r.stdout).strip()[:400]}")
    changed = [l[3:].strip() for l in run(["git", "status", "--porcelain"], cwd=path).stdout.splitlines() if l.strip()]
    if not changed:
        die("补丁没有产生任何改动（空补丁？）")
    head_blob = {f: run(["git", "rev-parse", f"HEAD:{f}"], cwd=path).stdout.strip() for f in changed}
    mutated_blob = {f: run(["git", "hash-object", f], cwd=path).stdout.strip() for f in changed}
    print(f"[trial] 已应用变异补丁：{', '.join(changed)}")
    for f in changed:
        print(f"[trial]   {f}: HEAD={head_blob[f][:12]}… → 变异后={mutated_blob[f][:12]}…")
    cmd = list(args.command)
    if cmd and cmd[0] == "--":
        cmd = cmd[1:]
    if not cmd:
        die("要给命令：scratch.py trial <batch> --patch <p> -- <命令...>")
    print(f"[trial] 执行：{' '.join(cmd)}")
    rc = subprocess.run(cmd, cwd=str(path), text=True, encoding="utf-8", errors="replace")
    print(f"[trial] 命令退出码 = {rc.returncode}")
    r2 = run(["git", "apply", "-R", str(patch)], cwd=path)
    fallback = r2.returncode != 0
    if fallback:
        run(["git", "checkout", "--", *changed], cwd=path)
    after_status = run(["git", "status", "--porcelain"], cwd=path).stdout.strip()
    restored_blob = {f: run(["git", "hash-object", f], cwd=path).stdout.strip() for f in changed}
    back_to_head = all(restored_blob[f] == head_blob[f] for f in changed)
    print(f"[trial] 还原：{'apply -R 成功' if not fallback else 'apply -R 失败（已 git checkout 兜底）'}")
    for f in changed:
        ok = '是' if restored_blob[f] == head_blob[f] else '否'
        print(f"[trial]   {f}: 还原后={restored_blob[f][:12]}…  与 HEAD 一致={ok}")
    print(f"[trial] 还原后 git status：{after_status if after_status else 'CLEAN'}")
    if after_status or not back_to_head:
        print("[trial] 警告：树没回到 HEAD 状态，请人工核对后再继续", file=sys.stderr)
        sys.exit(3)
    print("[trial] 结论：" + ("变异使命令失败（判据抓到了）——把上面的输出留档" if rc.returncode != 0 else "变异后命令仍成功——判读这是不是「判据抓不到」"))


def cmd_status(args):
    repo = repo_root()
    rows = []
    for b in batch_names(repo):
        meta = load_meta(repo, b)
        lock = meta.get("lock")
        row = {
            "batch": b,
            "path": meta["path"],
            "holder": lock["by"] if lock else "",
            "task": lock.get("task", "") if lock else "",
            "rev": (lock.get("rev", "") if lock else "")[:12],
            "since": lock.get("at", "") if lock else "",
            "size_mb": dir_size_mb(Path(meta["path"]).parent) if args.size else None,
        }
        rows.append(row)
    if args.json:
        print(json.dumps(rows, ensure_ascii=False, indent=2))
        return
    if not rows:
        print("[scratch] 没有批次树")
        return
    for r in rows:
        head = f"{r['batch']:<24} holder={r['holder'] or '-'}"
        if r["holder"]:
            head += f" task={r['task']} rev={r['rev']} since={r['since']}"
        if r["size_mb"] is not None:
            head += f" size={r['size_mb']}MB"
        print(head)
        print(f"    {r['path']}")


def _remove(repo: Path, batch: str, force: bool):
    meta = load_meta(repo, batch)
    lock = meta.get("lock")
    if lock and not force:
        die(f"{batch} 正被 {lock['by']} 持有，禁止删除（确需删用 --force）。")
    path = Path(meta["path"])
    freed = dir_size_mb(path.parent) if path.parent.exists() else 0.0
    run(["git", "worktree", "remove", "--force", str(path)], cwd=repo)
    shutil.rmtree(path.parent, ignore_errors=True)
    run(["git", "worktree", "prune"], cwd=repo)
    print(f"[scratch] 已删除 {batch}（释放约 {freed} MB）")
    return freed


def cmd_rm(args):
    repo = repo_root()
    _remove(repo, args.batch, args.force)


def cmd_gc(args):
    repo = repo_root()
    names = batch_names(repo)
    if not names:
        print("[scratch] 没有批次树，无需回收")
        return
    total = 0.0
    for b in names:
        meta = load_meta(repo, b)
        if meta.get("lock") and not args.force:
            print(f"[scratch] 跳过 {b}（仍被 {meta['lock']['by']} 持有）")
            continue
        total += _remove(repo, b, args.force)
    print(f"[scratch] gc 完成，共释放约 {round(total, 1)} MB")


def main(argv=None):
    p = argparse.ArgumentParser(description="批次验证树的唯一入口（AGENTS.md 第 9 条）")
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("init", help="建批次树"); s.add_argument("batch"); s.add_argument("--rev"); s.set_defaults(f=cmd_init)
    s = sub.add_parser("use", help="上锁并切 revision"); s.add_argument("batch"); s.add_argument("--rev", required=True)
    s.add_argument("--by", required=True); s.add_argument("--task", required=True); s.add_argument("--steal", action="store_true"); s.set_defaults(f=cmd_use)
    s = sub.add_parser("build", help="在树里构建 CLI 二进制"); s.add_argument("batch"); s.add_argument("--quiet", action="store_true"); s.set_defaults(f=cmd_build)
    s = sub.add_parser("release", help="解锁"); s.add_argument("batch"); s.add_argument("--by", required=True); s.add_argument("--force", action="store_true"); s.set_defaults(f=cmd_release)
    s = sub.add_parser("trial", help="变异试验（apply→跑→restore 全自动）")
    s.add_argument("batch")
    s.add_argument("--patch", required=True)
    s.add_argument("command", nargs="*")  # 用 -- 分隔；REMAINDER 会把 --patch 也吞掉
    s.set_defaults(f=cmd_trial)
    s = sub.add_parser("status", help="列出批次树"); s.add_argument("--json", action="store_true"); s.add_argument("--size", action="store_true"); s.set_defaults(f=cmd_status)
    s = sub.add_parser("rm", help="删除一个批次树"); s.add_argument("batch"); s.add_argument("--force", action="store_true"); s.set_defaults(f=cmd_rm)
    s = sub.add_parser("gc", help="回收所有未持有的批次树"); s.add_argument("--force", action="store_true"); s.set_defaults(f=cmd_gc)

    args = p.parse_args(argv)
    args.f(args)


if __name__ == "__main__":
    main()
