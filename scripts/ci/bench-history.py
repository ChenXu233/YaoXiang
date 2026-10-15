#!/usr/bin/env python3
"""bench-history.py - 把基准结果追加进仓库内的性能历史（JSONL），并派生图表数据。

为什么要有这个脚本
------------------
nightly 此前只把结果挂 artifact（保留 30 天），而 nightly 的 Pre-release 每晚被
删掉重建——两者都不是档案：超过 30 天就再无「上一夜是多少」可比。故把每次运行的
读数**写进仓库**，与代码同史：git log -p benches/history/ 就能把性能变化与改动
对应起来。

数据布局（JSONL，一行一条，自描述）
----------------------------------
  benches/history/shootout.jsonl   多语言横比（一行一个 bench x 语言 x 输入）
  benches/history/criterion.jsonl  Yaoxiang 自身微基准
  benches/history/releases.jsonl   发版标记（release 图上的竖线）
三种行：
  {"kind":"meta", "run":..., "sha":..., "ts":..., "toolchains":{...}, "cpu":...}
  {"kind":"row", "run":..., "sha":..., "bench":..., "lang":..., "median_ms":...}
  {"kind":"failure", "bench":..., "lang":..., "error":...}

幂等
----
同一 (run, attempt) 的结果**替换**而非追加：nightly 重跑（attempt 2）或多 job
重入都不会把同一夜的数据写两遍。

用法
----
  # 1) 追加多语言横比结果
  python scripts/ci/bench-history.py --kind shootout       --shootout-json target/shootout-results.json       --meta target/bench-meta-shootout.json --history-dir benches/history

  # 2) 追加 Criterion 自身微基准（bench-report.sh 产出的 JSON）
  python scripts/ci/bench-history.py --kind criterion       --criterion-json target/bench-results.json       --meta target/bench-meta-benchmark.json --history-dir benches/history

  # 3) 记录一次 release 标记
  python scripts/ci/bench-history.py --mark-release --version 0.8.5 --sha abc1234

  # 4) 从入库数据派生图表用聚合文件（按点数截断，原始 JSONL 不动）
  python scripts/ci/bench-history.py --render-chart       --out docs/src/public/perf/history.json
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import pathlib
import sys

try:  # Windows 控制台默认 GBK，而报告里有非 ASCII 字符
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, OSError):
    pass

NL = chr(10)
DEFAULT_HISTORY_DIR = "benches/history"
# 每个序列在**派生**文件里保留的最新点数。原始 JSONL 不截断，
# 派生文件随时可由原始数据重建，故这里压缩不丢信息。
DEFAULT_MAX_POINTS = 720


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def read_jsonl(path: pathlib.Path) -> list:
    if not path.is_file():
        return []
    rows = []
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = line.strip()
        if not line:
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError as e:
            raise SystemExit(f"{path}:{lineno} 不是合法 JSON 行：{e}")
    return rows


def write_jsonl(path: pathlib.Path, rows: list) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    body = "".join(json.dumps(r, ensure_ascii=False, sort_keys=True) + NL for r in rows)
    path.write_text(body, encoding="utf-8")


def upsert_run(path: pathlib.Path, run: str, attempt: str, new_rows: list) -> int:
    """替换同一 (run, attempt) 的既有行，返回写入后的总行数。"""
    kept = [
        r
        for r in read_jsonl(path)
        if not (str(r.get("run")) == run and str(r.get("attempt")) == attempt)
    ]
    merged = kept + new_rows
    merged.sort(
        key=lambda r: (
            str(r.get("ts", "")),
            str(r.get("run", "")),
            r.get("bench", ""),
            r.get("lang", ""),
            r.get("input", ""),
        )
    )
    write_jsonl(path, merged)
    return len(merged)


def load_meta(path) -> dict:
    if not path:
        return {}
    p = pathlib.Path(path)
    if not p.is_file():
        raise SystemExit(f"元数据文件不存在：{path}")
    return json.loads(p.read_text(encoding="utf-8"))


def inherit(meta: dict) -> dict:
    """每行都带最小归因：没有 sha/run 的行无法与代码对应，等于没存。"""
    return {
        "ts": meta.get("ts") or now_iso(),
        "run": str(meta.get("run", "")),
        "attempt": str(meta.get("attempt", "")),
        "sha": meta.get("sha", ""),
        "ref": meta.get("ref", ""),
    }


def cmd_shootout(args: argparse.Namespace) -> int:
    meta = load_meta(args.meta)
    data = json.loads(pathlib.Path(args.shootout_json).read_text(encoding="utf-8"))
    rows = data.get("rows") or []
    failures = data.get("failures") or []
    base = inherit(meta)
    rmeta = data.get("meta") or {}

    # 判失败的行不当作读数入库：算错的程序再快也不可比
    bad = {
        (f.get("bench"), f.get("lang"), str(f.get("input")))
        for f in failures
        if f.get("input")
    }

    out = [
        {
            "kind": "meta",
            **base,
            "toolchains": rmeta.get("toolchains") or meta.get("toolchains") or {},
            "cpu": rmeta.get("cpu") or meta.get("cpu", ""),
            "os": rmeta.get("os") or meta.get("os", ""),
            "rows": len(rows),
            "failures": len(failures),
        }
    ]
    for r in rows:
        key = (r.get("bench"), r.get("lang"), str(r.get("input")))
        out.append(
            {
                "kind": "row",
                **base,
                "suite": "shootout",
                "bench": r.get("bench", ""),
                "lang": r.get("lang", ""),
                "input": str(r.get("input", "")),
                "median_ms": _round(r.get("median_ms"), 4),
                "mean_ms": _round(r.get("mean_ms"), 4),
                "stddev_ms": _round(r.get("stddev_ms"), 4),
                "rel_pct": _round(r.get("relative_pct"), 2),
                "compile_ms": _round(r.get("compile_ms"), 2),
                "n": args.runs,
                "ok": key not in bad,
            }
        )
    # 失败也要入库：只记成功的行，历史会读成「那晚这门语言没配」，与「挂了」不同
    for f in failures:
        out.append(
            {
                "kind": "failure",
                **base,
                "suite": "shootout",
                "bench": f.get("bench", ""),
                "lang": f.get("lang", ""),
                "input": str(f.get("input") or ""),
                "error": (f.get("error") or "").strip().replace(NL, " ")[:400],
            }
        )

    path = pathlib.Path(args.history_dir) / "shootout.jsonl"
    total = upsert_run(path, base["run"], base["attempt"], out)
    print(
        f"shootout: 写入 {len(out)} 行（含 1 条 meta、{len(failures)} 条失败）"
        f"-> {path}，累计 {total} 行"
    )
    return 0


def _round(v, nd):
    return round(float(v), nd) if v is not None else None


def cmd_criterion(args: argparse.Namespace) -> int:
    meta = load_meta(args.meta)
    base = inherit(meta)
    data = json.loads(pathlib.Path(args.criterion_json).read_text(encoding="utf-8"))
    rows = data.get("criterion") or []

    out = [
        {
            "kind": "meta",
            **base,
            "toolchains": meta.get("toolchains") or {},
            "cpu": meta.get("cpu", ""),
            "os": meta.get("os", ""),
            "rows": len(rows),
        }
    ]
    for r in rows:
        out.append(
            {
                "kind": "row",
                **base,
                "suite": "criterion",
                "bench": r.get("name", ""),
                "median_ms": _round(r.get("median_ms"), 6),
                "mean_ms": _round(r.get("mean_ms"), 6),
                "n": None,
            }
        )
    path = pathlib.Path(args.history_dir) / "criterion.jsonl"
    total = upsert_run(path, base["run"], base["attempt"], out)
    print(f"criterion: 写入 {len(out)} 行 -> {path}，累计 {total} 行")
    return 0


def cmd_mark_release(args: argparse.Namespace) -> int:
    path = pathlib.Path(args.history_dir) / "releases.jsonl"
    rows = read_jsonl(path)
    entry = {
        "kind": "release",
        "ts": args.ts or now_iso(),
        "version": args.version,
        "sha": args.sha,
        "tag": args.tag or f"v{args.version}",
    }
    rows = [r for r in rows if r.get("version") != args.version]
    rows.append(entry)
    rows.sort(key=lambda r: str(r.get("ts", "")))
    write_jsonl(path, rows)
    print(f"release: 记录 {args.version}@{args.sha} -> {path}，累计 {len(rows)} 条")
    return 0


def cmd_render_chart(args: argparse.Namespace) -> int:
    """把原始 JSONL 派生成图表用的聚合文件（可重建，故可按点数截断）。

    输出必须落在 docs/src/public/ 下：那才是 VitePress 真正会拷贝进站点产物的
    public 目录。docs/src/.vitepress/public/ **不是**——实测该目录不被拷贝
    （站点里的 wasm 是靠 docs/build.js 手工复制才到位的）。放错地方不会报错，
    只是部署后页面 404。
    """
    hist = pathlib.Path(args.history_dir)
    runs = {}
    series = {}

    for name in ("shootout", "criterion"):
        for r in read_jsonl(hist / f"{name}.jsonl"):
            run_id = str(r.get("run", ""))
            if r.get("kind") == "meta":
                runs[run_id] = {
                    "run": run_id,
                    "ts": r.get("ts", ""),
                    "sha": r.get("sha", ""),
                    "ref": r.get("ref", ""),
                }
                continue
            if r.get("kind") != "row":
                continue
            v = r.get("median_ms")
            if v is None:
                continue
            if r.get("suite") == "shootout":
                sid = "shootout/{}/{}/{}".format(
                    r.get("bench"), r.get("lang"), r.get("input")
                )
            else:
                sid = "criterion/{}".format(r.get("bench"))
            series.setdefault(sid, []).append(
                [int(run_id) if run_id.isdigit() else 0, round(float(v), 4)]
            )

    releases = [
        {"version": r.get("version", ""), "ts": r.get("ts", ""), "sha": r.get("sha", "")}
        for r in read_jsonl(hist / "releases.jsonl")
        if r.get("kind") == "release"
    ]

    payload = {
        "generated_at": now_iso(),
        "unit": "ms",
        "runs": [runs[k] for k in sorted(runs, key=lambda x: int(x) if x.isdigit() else 0)],
        "series": [
            {"id": sid, "points": sorted(pts)[-args.max_points:]}
            for sid, pts in sorted(series.items())
        ],
        "releases": releases,
    }
    out = pathlib.Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + NL, encoding="utf-8"
    )
    print(
        f"图表数据: {len(payload['series'])} 条序列 / {len(payload['runs'])} 次运行"
        f" -> {out}（{out.stat().st_size} 字节）"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="基准性能历史入库与图表数据派生")
    ap.add_argument("--history-dir", default=DEFAULT_HISTORY_DIR)
    ap.add_argument("--kind", choices=["shootout", "criterion"])
    ap.add_argument("--shootout-json")
    ap.add_argument("--criterion-json")
    ap.add_argument("--meta", help="运行元数据 JSON（sha/ref/ts/run/attempt）")
    ap.add_argument("--runs", type=int, default=0, help="每个测点的样本数")
    ap.add_argument("--mark-release", action="store_true")
    ap.add_argument("--version")
    ap.add_argument("--sha", default="")
    ap.add_argument("--tag")
    ap.add_argument("--ts")
    ap.add_argument("--render-chart", action="store_true")
    ap.add_argument("--out")
    ap.add_argument("--max-points", type=int, default=DEFAULT_MAX_POINTS)
    args = ap.parse_args()

    if args.mark_release:
        if not args.version:
            raise SystemExit("--mark-release 需要 --version")
        return cmd_mark_release(args)
    if args.render_chart:
        if not args.out:
            raise SystemExit("--render-chart 需要 --out")
        return cmd_render_chart(args)
    if args.kind == "shootout":
        if not args.shootout_json:
            raise SystemExit("--kind shootout 需要 --shootout-json")
        return cmd_shootout(args)
    if args.kind == "criterion":
        if not args.criterion_json:
            raise SystemExit("--kind criterion 需要 --criterion-json")
        return cmd_criterion(args)
    ap.print_help()
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
