#!/usr/bin/env python3
"""bench-meta.py - 生成一次基准运行的元数据（供入库与报告归因）。

为什么单独一个脚本
------------------
元数据来自**环境**（git sha、runner、时间），不是来自测量。放在 workflow 里用
shell 拼 JSON 既易错又无法本地复跑；做成脚本后本地也能生成同样的文件，把入库
路径演练一遍。

⚠ sha/ref 必须取**工作树的 HEAD**，不能用 GITHUB_SHA / GITHUB_REF_NAME
--------------------------------------------------------------------
nightly 的 schedule 事件走默认分支（main），而基准 job 显式 checkout dev。
此时 GITHUB_SHA / GITHUB_REF_NAME 仍然是**事件本身**的 main sha/ref——照它们
记，会把 dev 的读数写成 main 的提交，历史数据从此指错地方。故一律以
`git rev-parse HEAD` 为准，环境变量只作兜底。

用法:
  python scripts/ci/bench-meta.py --kind shootout --out target/bench-meta-shootout.json
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import subprocess
import sys

NL = chr(10)


def sh(cmd: list) -> str:
    try:
        return subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL).strip()
    except Exception:
        return ""


def cpu_model() -> str:
    p = pathlib.Path("/proc/cpuinfo")
    if p.is_file():
        for line in p.read_text(errors="replace").splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    return os.environ.get("PROCESSOR_IDENTIFIER", "unknown")


def main() -> int:
    ap = argparse.ArgumentParser(description="生成基准运行元数据")
    ap.add_argument("--kind", required=True, help="shootout / benchmark / release")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    sha = sh(["git", "rev-parse", "HEAD"]) or os.environ.get("GITHUB_SHA", "")
    ref = sh(["git", "rev-parse", "--abbrev-ref", "HEAD"]) or os.environ.get("GITHUB_REF_NAME", "")
    meta = {
        "kind": args.kind,
        "sha": sha,
        "sha_short": sha[:7],
        "ref": ref,
        "ts": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "run": os.environ.get("GITHUB_RUN_NUMBER", ""),
        "attempt": os.environ.get("GITHUB_RUN_ATTEMPT", "1"),
        "runner": os.environ.get("RUNNER_NAME", "local"),
        "os": sys.platform,
        "cpus": os.cpu_count(),
        "cpu": cpu_model(),
    }
    out = pathlib.Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(meta, ensure_ascii=False, indent=2) + NL, encoding="utf-8")
    print(f"meta -> {out}: {meta['ref']}@{meta['sha_short']} run={meta['run']} attempt={meta['attempt']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
