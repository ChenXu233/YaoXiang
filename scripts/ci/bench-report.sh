#!/usr/bin/env bash
# bench-report.sh - 汇总基准结果，输出 Markdown 报告（+ 可选机器可读 JSON）
#
# 数据来源：
#   1. Criterion（target/criterion/**/new/estimates.json）—— Yaoxiang 自身的
#      微基准，用于**跨 nightly 的回归追踪**（每次跑同样 workload，比历史）
#   2. shootout JSON（--shootout-json）—— 多语言横比，渲染成表格
#   3. shootout 原始文本（--shootout）—— 结构化结果缺失时的降级路径
#
# 用法:
#   ./scripts/ci/bench-report.sh                            # 只汇总 Criterion
#   ./scripts/ci/bench-report.sh --shootout-json r.json     # 附多语言横比表
#   ./scripts/ci/bench-report.sh --meta m.json              # 报告头写明测于哪个提交
#   ./scripts/ci/bench-report.sh --shootout out.txt         # 降级：原样搬运
#   ./scripts/ci/bench-report.sh --out-json r.json          # 同时写机器可读结果
#
# 输出:
#   stdout   — Markdown 报告（workflow 里 tee 到 GITHUB_STEP_SUMMARY）
#   退出码 0 — 即使缺数据也成功（基准不应阻断 nightly）
#
# 例:
#   ./scripts/ci/bench-report.sh --shootout bench.txt | tee -a "$GITHUB_STEP_SUMMARY"

set -euo pipefail

CRITERION_DIR="target/criterion"
OUT_JSON=""
SHOOTOUT_FILE=""
SHOOTOUT_JSON=""
META_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --criterion-dir)  CRITERION_DIR="$2"; shift 2 ;;
    --shootout)       SHOOTOUT_FILE="$2"; shift 2 ;;
    --shootout-json)  SHOOTOUT_JSON="$2"; shift 2 ;;
    --out-json)       OUT_JSON="$2"; shift 2 ;;
    --meta)           META_FILE="$2"; shift 2 ;;
    -h|--help)        sed -n '2,22p' "$0"; exit 0 ;;
    *) echo "未知参数: $1" >&2; exit 2 ;;
  esac
done

# ── 选择 Python 解释器 ──
# CI（ubuntu）通常只有 python3；Windows 开发机的 `python3` 可能是 Store 存根
# （执行后静默无输出且退出码为 0），故优先 python，回退 python3。
PY=""
for cand in python python3; do
  if command -v "$cand" >/dev/null 2>&1 && "$cand" -c 'pass' >/dev/null 2>&1; then
    PY="$cand"
    break
  fi
done
if [[ -z "$PY" ]]; then
  echo "> ⚠ 未找到可用的 Python，跳过基准汇总" >&2
  exit 0
fi

# ── 解析 Criterion 目录 ──
# 调用方可能给出确切目录，也可能给出一个下载下来的 artifact 根目录
# （artifact 内部的层次取决于 upload-artifact 的路径推断，不稳定），
# 所以：若不是 criterion 目录，则在其下**搜索**含 estimates.json 的目录。
resolve_criterion_dir() {
  local given="$1"
  [[ -d "$given" ]] || { echo "$given"; return; }
  # 已经是 criterion 目录 → 直接用。
  # 判据必须是 `<bench>/new/estimates.json` 这种**两层结构**，而不是
  # 任意层级的 estimates.json——否则外层 artifact 目录（其下恰好也含
  # estimates.json）会被误判为「已是」，从而漏掉向下搜索。
  if [[ -n "$(find "$given" -mindepth 3 -maxdepth 3 -type f -name estimates.json -print -quit 2>/dev/null)" ]]; then
    echo "$given"
    return
  fi
  # 否则在其下找名为 criterion 的目录
  local found
  found="$(find "$given" -type d -name criterion -print -quit 2>/dev/null || true)"
  echo "${found:-$given}"
}
CRITERION_DIR="$(resolve_criterion_dir "$CRITERION_DIR")"

# ── 解析 artifact 内文件 ──
# 调用方可能给的确切路径不存在（artifact 内部层次随 upload-artifact 的路径推断
# 变化，不写死），则从**最近的已存在祖先目录**向下搜同名文件。
# 向上找祖先而非直接用 dirname：工件的中间层目录可能整层不存在。
resolve_artifact_file() {
  local given="$1"
  [[ -z "$given" ]] && { echo ""; return; }
  [[ -f "$given" ]] && { echo "$given"; return; }
  local base dir found
  base="$(basename "$given")"
  dir="$(dirname "$given")"
  while [[ -n "$dir" && ! -d "$dir" && "$dir" != "/" && "$dir" != "." ]]; do
    dir="$(dirname "$dir")"
  done
  [[ -d "$dir" ]] || { echo "$given"; return; }
  found="$(find "$dir" -type f -name "$base" -print -quit 2>/dev/null || true)"
  echo "${found:-$given}"
}

# ── 运行归因（测的是哪个提交、哪台机器）──
# 必须写在报告最前：不写这一行，读数变了就无从判断是代码变了还是环境换了。
# nightly 的基准 job 显式 checkout dev（见 nightly.yml），而 release 构件走
# 默认分支——不点明这层，读者会把 dev 的数字当成 release 构件的数字。
if [[ -n "$META_FILE" ]]; then
  META_FILE="$(resolve_artifact_file "$META_FILE")"
fi
if [[ -n "$META_FILE" && -f "$META_FILE" ]]; then
  "$PY" - "$META_FILE" <<'PYEOF'
import json
import pathlib
import sys

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, OSError):
    pass

meta = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
print(f"> 本次基准测于 `{meta.get('ref', '?')}@{meta.get('sha', '?')}`"
      f"（run {meta.get('run', '?')}，{meta.get('ts', '?')}）。")
if meta.get("cpu"):
    print(f"> 机器：{meta['cpu']}（{meta.get('os', '?')}，{meta.get('cpus', '?')} 核）。")
print()
PYEOF
fi

# ── Criterion 汇总（estimates.json 的 mean.point_estimate 单位是纳秒）──
"$PY" - "$CRITERION_DIR" "$OUT_JSON" <<'PYEOF'
import json
import math
import pathlib
import sys

# Windows 开发机的 Python 默认 stdout 编码可能是 GBK，而报告里有 ⚠️ 等
# 非 ASCII 字符，会导致 UnicodeEncodeError。强制 UTF-8 并替换不可编码字符
# （CI 是 UTF-8，此设置无害）。
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, OSError):
    pass

crit_dir = pathlib.Path(sys.argv[1])
out_json = sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] else None

rows = []
if crit_dir.is_dir():
    for est in sorted(crit_dir.glob("**/new/estimates.json")):
        # target/criterion/<bench>[/<group>]/new/estimates.json
        # 注意：crit_dir 可能是一个**外层目录**（如 artifact 根下的 target/criterion），
        # 用相对路径能自然去掉这层前缀。
        rel = est.relative_to(crit_dir)
        parts = rel.parts[:-2]  # 去掉 new/estimates.json
        if not parts or parts[0] == "report":
            continue
        try:
            data = json.loads(est.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        mean_ns = data.get("mean", {}).get("point_estimate")
        median_ns = data.get("median", {}).get("point_estimate")
        if mean_ns is None:
            continue
        rows.append({
            "name": "/".join(parts),
            "mean_ns": mean_ns,
            "median_ns": median_ns,
            "mean_ms": round(mean_ns / 1e6, 4),
            "median_ms": round(median_ns / 1e6, 4) if median_ns is not None else None,
        })

rows.sort(key=lambda r: r["name"])

lines = ["## 基准结果", ""]
if rows:
    lines += [
        "### Criterion 微基准（Yaoxiang 自身，跨 nightly 可比）",
        "",
        "| benchmark | mean | median |",
        "|---|---|---|",
    ]
    for r in rows:
        med = f"{r['median_ms']:.4f} ms" if r["median_ns"] is not None else "—"
        lines.append(f"| `{r['name']}` | {r['mean_ms']:.4f} ms | {med} |")
    lines.append("")
else:
    lines += [
        f"> ⚠ 未找到 Criterion 结果（查找 `{crit_dir}/**/new/estimates.json`）。",
        "",
    ]

print("\n".join(lines))

if out_json:
    pathlib.Path(out_json).write_text(
        json.dumps({"criterion": rows}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
PYEOF
if [[ -n "$SHOOTOUT_FILE" ]]; then
  SHOOTOUT_FILE="$(resolve_artifact_file "$SHOOTOUT_FILE")"
fi

# ── 多语言横比：渲染为表格 ──
# 优先用 runner 的 JSON（结构化、可渲染），没有再降级到原始文本。
#
# 历史：此前一律把 runner 的框线输出**原样**搬进 release notes，于是 nightly
# 的 release body 里出现了：cargo 的 ANSI 转义码（`^[[1m^[[92m`）、每次测量
# 后面的 13 行子进程输出（`6765` × 13）、以及宽度对不齐的标题框。
# 人读不了，机器也读不了。
if [[ -n "$SHOOTOUT_JSON" ]]; then
  SHOOTOUT_JSON="$(resolve_artifact_file "$SHOOTOUT_JSON")"
fi
# 明确传了 JSON 却没找到 → 显式告警。否则会静默降级到原始文本，
# 而原始文本正是被取代的那个可读性问题（无法从结果看出降级了）。
if [[ -n "$SHOOTOUT_JSON" && ! -f "$SHOOTOUT_JSON" ]]; then
  echo "::warning::未找到 shootout 结构化结果：$SHOOTOUT_JSON，降级为原始文本" >&2
  SHOOTOUT_JSON=""
fi

echo
if [[ -n "$SHOOTOUT_JSON" ]]; then
  "$PY" - "$SHOOTOUT_JSON" <<'PYEOF'
import json
import math
import pathlib
import sys

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, OSError):
    pass

try:
    data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as e:
    print(f"> ⚠ 解析 shootout JSON 失败：{e}")
    print()
    raise SystemExit(0)

rows = data.get("rows") or []
failures = data.get("failures") or []


def fmt_ms(ms):
    if ms is None:
        return "—"
    if ms >= 1000:
        return f"{ms/1000:.2f} s"
    if ms >= 1:
        return f"{ms:.2f} ms"
    if ms >= 0.001:
        return f"{ms*1000:.2f} µs"
    return f"{ms*1_000_000:.2f} ns"


# 按 benchmark 分组（保持出现顺序），组内按输入规模 + 耗时排
by_bench = {}
for r in rows:
    by_bench.setdefault(r["bench"], []).append(r)


def median_of(r):
    """头条数字用中位数：墙钟样本右偏，一次调度抖动就能拉高均值。"""
    m = r.get("median_ms")
    return m if m is not None else r["mean_ms"]


def input_key(r):
    # 输入规模按数值排序（避免字符串序把 100 排在 20 前）
    try:
        return (0, float(r["input"]))
    except (TypeError, ValueError):
        return (1, str(r["input"]))


def to_num(s):
    try:
        return (0, float(s))
    except (TypeError, ValueError):
        return (1, str(s))


meta = data.get("meta") or {}
toolchains = meta.get("toolchains") or {}

if by_bench:
    print("### 多语言横比（shootout）")
    print()
    print("同问题、同输入规模下各语言的运行耗时，单位越小越快。")
    print("`运行` 取**中位数**（跨夜趋势同一口径）；`相对最快` 是该输入下最快语言的倍数。")
    print("`输出` 是被测程序的实际输出——同输入下跨语言必须一致（runner 按金标/一致性判失败）。")
    if toolchains:
        print()
        print("工具链：" + "；".join(f"{k} `{v}`" for k, v in sorted(toolchains.items())))
    if meta.get("cpu"):
        print()
        print(
            f"机器：{meta.get('cpu')}"
            f"（{meta.get('os', '?')}/{meta.get('arch', '?')}，{meta.get('cpus', '?')} 核）"
        )
    print()

    # 每个 (bench, input) 的最快中位数 → 相对最快列
    fastest = {}
    for r in rows:
        key = (r["bench"], str(r["input"]))
        m = median_of(r)
        if m > 0 and (key not in fastest or m < fastest[key]):
            fastest[key] = m

    ratios = {}
    single_lang = []
    for bench, items in by_bench.items():
        desc = next((r.get("description") for r in items if r.get("description")), None)
        print(f"**{bench}**" + (f" — {desc}" if desc else ""))
        print()
        items_sorted = sorted(items, key=lambda r: (input_key(r), median_of(r)))
        print("| 输入 | 语言 | 运行（中位） | 相对偏差 | 相对最快 | 编译 | 输出 |")
        print("|---|---|---|---|---|---|---|")
        for r in items_sorted:
            out = (r.get("output") or "")
            # 表格里不能有裸 `|` 或换行
            out = out.replace("|", r"\|").replace("\n", " ")
            if len(out) > 40:
                out = out[:40] + "…"
            m = median_of(r)
            base = fastest.get((r["bench"], str(r["input"])))
            ratio = (m / base) if (base and m > 0) else None
            if ratio:
                ratios.setdefault(r["lang"], []).append(ratio)
            print(
                f"| {r['input']} | {r['lang']} | {fmt_ms(m)} "
                f"± {fmt_ms(r['stddev_ms'])} | {r['relative_pct']:.1f}% "
                f"| {f'{ratio:.2f}×' if ratio else '—'} "
                f"| {fmt_ms(r.get('compile_ms'))} | `{out}` |"
            )
        print()
        # 单语言测点没有横比对象：必须显式点出，否则「只有一行」会被读成「这语言最快」
        for input_val in sorted({str(r["input"]) for r in items}, key=to_num):
            langs = [r["lang"] for r in items if str(r["input"]) == input_val]
            if len(langs) == 1:
                single_lang.append(f"{bench}/{input_val}（仅 {langs[0]}）")

    # 几何平均比值：加/减一个 benchmark 不会让这个数跳动，适合跨夜追踪
    if ratios:
        print("**各语言相对最快的几何平均**（越小越接近最快）")
        print()
        print("| 语言 | geomean 比值 | 测点数 |")
        print("|---|---|---|")
        for lang in sorted(ratios):
            vals = [v for v in ratios[lang] if v and v > 0]
            if not vals:
                continue
            g = math.exp(sum(math.log(v) for v in vals) / len(vals))
            print(f"| {lang} | {g:.2f}× | {len(vals)} |")
        print()

    if single_lang:
        print(f"> ⚠ 单语言测点：{'、'.join(single_lang)}——只有自身读数，没有跨语言对照。")
        print()

# 失败必须显式列出：只「少一行」会读成「没配这项」，与「配了但挂了」不同。
# 上面 matrix 的 yaoxiang 实现就这样藏了很久（List→Vec 统一后类型报错）。
if failures:
    print(f"### 失败项（{len(failures)}）")
    print()
    for f in failures:
        where = f"输入 {f['input']}" if f.get("input") else "编译阶段"
        err = (f.get("error") or "").strip().replace("\n", " ")
        if len(err) > 160:
            err = err[:160] + "…"
        print(f"- **{f['bench']} / {f['lang']}**（{where}）：{err}")
    print()
PYEOF

# 降级：无 JSON 时原样搬运文本（至少剥掉 ANSI，否则 release notes 里是乱码）
elif [[ -n "$SHOOTOUT_FILE" ]]; then
  if [[ -f "$SHOOTOUT_FILE" ]]; then
    echo "### 多语言横比（shootout）"
    echo
    echo "> ⚠ 未找到结构化结果（JSON），以下为原始输出。"
    echo
    echo '```'
    # 剥 ANSI 转义：cargo 在 CI 里开彩色（CARGO_TERM_COLOR=always），
    # 经 `2>&1 | tee` 一并进文件
    sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' -e 's/\r$//' "$SHOOTOUT_FILE"
    echo '```'
  else
    echo "> ⚠ 未找到 shootout 输出文件 \`$SHOOTOUT_FILE\`（该 benchmark 可能未运行）"
  fi
fi

echo
echo "---"
echo
echo "_机器可读结果见 workflow artifact；性能历史（\`benches/history/\`）与趋势图（\`/perf/\`）由 nightly 自动入库；Criterion 目录：\`$CRITERION_DIR\`_"
