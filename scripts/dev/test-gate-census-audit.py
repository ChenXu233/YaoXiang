#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""test-gate-census-audit: 条款门分层抽样审计（0.6-10）

背景
----
0.6-9 的普查把「文件头出现过 §/RFC/SPEC 字样」判成「对得上一条条款」（2403 条）。
存在引用不等于引用支撑断言：测试可以抄一句「依据 RFC-011 §4.1」而断言的东西与
§4.1 无关。本脚本按目录分层抽 40 条，供人工逐条判「条款是否真支撑断言」，并把
判定结果落成数据文件（被 test-gate-census.py 读取，用于把审计确认的条目标为「留」）。

抽样口径（冻结，可复现）
------------------------
* 候选池 = 0.6-9 条款门（**收紧前**的判据，冻结在本文件的 LEGACY_CLAUSE_RE）命中的测试；
  冻结的理由：抽样分母必须固定在「当时被判为条款」的集合上，否则收紧判据后再抽
  就是在改分母，结论不可比。
* 分层 = 目录簇：src/ 下取前两层（src/frontend、src/middle…），其余按顶层
  （tests、benches、tools）。
* 分配 = 按层内条数比例分配（最大余数法），每层至少 1 条（层内条数 ≥1 时）。
* 层内抽取 = 按 (文件, 行号, 测试名) 排序后用固定种子随机取样。
* 种子 = 20261106（写死；改种子等于换样本，必须在文档里说明并重跑审计）。

用法
----
    python scripts/dev/test-gate-census-audit.py --sample         # 打印 40 条样本
    python scripts/dev/test-gate-census-audit.py --check          # 校验数据文件覆盖 40 条且判定合法
    python scripts/dev/test-gate-census-audit.py --seed 20261106  # 显式指定种子（默认即此值）

退出码：0 = 通过；1 = 数据文件与样本不符 / 判定非法；3 = 环境错误。
"""
import argparse
import io
import json
import os
import random
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import importlib.util

_spec = importlib.util.spec_from_file_location(
    "test_gate_census", os.path.join(os.path.dirname(os.path.abspath(__file__)), "test-gate-census.py")
)
census = importlib.util.module_from_spec(_spec)
sys.modules["test_gate_census"] = census
_spec.loader.exec_module(census)

AUDIT_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "test-gate-census-audit.json")
SEED = 20261106
SAMPLE_SIZE = 40

# 0.6-9 的条款门（收紧前）——冻结用于抽样复现，不要跟着生成器一起改
LEGACY_CLAUSE_RE = re.compile(r"(§\s*\d|RFC[- ]?\d{3}|SPEC[- ]?[A-Z0-9]|language-spec|语言规范)")

VERDICTS = ("真实支撑", "不相关", "泛泛")


def force_utf8_stdio() -> None:
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            try:
                stream.reconfigure(encoding="utf-8", errors="replace")
            except Exception:  # pragma: no cover
                pass


def stratum_of(rel_path: str) -> str:
    parts = rel_path.split("/")
    if parts[0] == "src" and len(parts) >= 2:
        return "src/" + parts[1]
    return parts[0]


def clause_line(path: str, rel_path: str) -> str:
    """文件头里第一条命中条款判据的行（用来判断「它引的条款说什么」）。"""
    lines = census.read_text(os.path.join(REPO_ROOT, path)).splitlines()[:25]
    if rel_path.startswith("src/") and "/tests/" in rel_path:
        parent = os.path.join(os.path.dirname(os.path.join(REPO_ROOT, path)), "mod.rs")
        if os.path.isfile(parent):
            lines += census.read_text(parent).splitlines()[:25]
    for line in lines:
        s = line.strip()
        if LEGACY_CLAUSE_RE.search(s):
            return s
    return ""


def pool():
    # 用**冻结的**旧判据重跑分类：分母必须是 0.6-9 判为「条款」的那 2403 条，
    # 而不是「文件头出现过字样」的 2794 条（后者含 spec-shaped 目录，那些在
    # 0.6-9 里已先被判成「证明」）。
    census.CLAUSE_RE = LEGACY_CLAUSE_RE
    # 抽样分母必须与审计结论无关：关掉审计覆盖，否则已审计的条目会因「不相关/泛泛」
    # 落到待核而不是条款，样本就再也无法复现。
    _load = census.load_audit_index
    census.load_audit_index = lambda: {}
    try:
        rows = census.scan()
    finally:
        census.load_audit_index = _load
    out = []
    for r in rows:
        # 0.6-9 的优先级：spec-shaped/现状锚先判「证明」，其余只要文件头命中旧条款
        # 判据就记「条款」（当年连「条款+病史同时命中」也记条款，故这里不按病史短路）。
        if r["file"].startswith(census.NO_RETIRE_PATHS):
            continue
        if census.NO_RETIRE_NAME_RE.search(r["name"]):
            continue
        path = os.path.join(REPO_ROOT, r["file"])
        if not LEGACY_CLAUSE_RE.search(census.header_text(path, r["file"])):
            continue
        out.append(
            {
                "name": r["name"],
                "file": r["file"],
                "line": r["line"],
                "stratum": stratum_of(r["file"]),
                "clause": clause_line(path, r["file"]),
            }
        )
    return out


def allocate(strata, total):
    """按比例分配（最大余数法），每层至少 1 条。"""
    sizes = {k: len(v) for k, v in strata.items()}
    base = {k: 1 for k in sizes}
    remaining = total - sum(base.values())
    if remaining < 0:
        raise SystemExit(f"[error] 层数 {len(sizes)} 大于样本量 {total}")
    pool_total = sum(sizes.values())
    quota = {k: remaining * sizes[k] / pool_total for k in sizes}
    floor_q = {k: int(quota[k]) for k in sizes}
    left = remaining - sum(floor_q.values())
    order = sorted(sizes, key=lambda k: (-(quota[k] - floor_q[k]), k))
    for k in order[:left]:
        floor_q[k] += 1
    return {k: base[k] + floor_q[k] for k in sizes}


def sample(seed=SEED, size=SAMPLE_SIZE):
    items = pool()
    strata = {}
    for it in items:
        strata.setdefault(it["stratum"], []).append(it)
    alloc = allocate(strata, size)
    rng = random.Random(seed)
    picked = []
    for name in sorted(strata):
        members = sorted(strata[name], key=lambda r: (r["file"], r["line"], r["name"]))
        picked.extend(rng.sample(members, alloc[name]))
    picked.sort(key=lambda r: (r["file"], r["line"], r["name"]))
    return {"seed": seed, "size": size, "population": len(items), "allocation": alloc, "sample": picked}


def load_audit():
    with io.open(AUDIT_PATH, "r", encoding="utf-8") as fh:
        return json.load(fh)


def main(argv=None):
    force_utf8_stdio()
    parser = argparse.ArgumentParser(description="条款门分层抽样审计（0.6-10）")
    parser.add_argument("--sample", action="store_true", help="打印样本（含每条的条款引用行）")
    parser.add_argument("--check", action="store_true", help="校验审计数据文件与样本一致、判定合法")
    parser.add_argument("--seed", type=int, default=SEED, help=f"随机种子（默认 {SEED}）")
    args = parser.parse_args(argv)

    result = sample(args.seed, SAMPLE_SIZE)
    if args.sample:
        print(f"[audit] 池 {result['population']} 条，样本 {result['size']} 条，种子 {result['seed']}")
        print("[audit] 分层分配：" + " · ".join(f"{k} {v}" for k, v in sorted(result["allocation"].items())))
        for i, it in enumerate(result["sample"], 1):
            print(f"{i:2d}\t{it['file']}:{it['line']}\t{it['name']}\t{it['clause']}")
        return 0

    if args.check:
        if not os.path.isfile(AUDIT_PATH):
            print(f"[audit] 审计数据文件不存在：{os.path.relpath(AUDIT_PATH, REPO_ROOT)}", file=sys.stderr)
            return 1
        data = load_audit()
        expected = {(it["file"], it["name"]): it for it in result["sample"]}
        got = {}
        bad = []
        for rec in data["records"]:
            key = (rec["file"], rec["name"])
            got[key] = rec
            if rec.get("verdict") not in VERDICTS:
                bad.append(f"{rec['file']}::{rec['name']} 判定非法：{rec.get('verdict')}")
            if not rec.get("evidence"):
                bad.append(f"{rec['file']}::{rec['name']} 缺依据")
        missing = sorted(f"{f}::{n}" for (f, n) in expected if (f, n) not in got)
        extra = sorted(f"{f}::{n}" for (f, n) in got if (f, n) not in expected)
        if bad:
            print("[audit] 判定问题：\n  " + "\n  ".join(bad), file=sys.stderr)
            return 1
        if missing or extra:
            print(f"[audit] 样本与数据文件不一致（缺 {len(missing)} 条，多 {len(extra)} 条）", file=sys.stderr)
            for x in (missing + extra)[:10]:
                print("  -", x, file=sys.stderr)
            return 1
        print(f"[audit] OK：数据文件覆盖全部 {len(got)} 条样本，种子 {data['seed']}")
        return 0

    print(f"[audit] 池 {result['population']} 条，样本 {result['size']} 条，种子 {result['seed']}")
    print("[audit] 用 --sample 打印样本、--check 校验数据文件")
    return 0


if __name__ == "__main__":
    sys.exit(main())
