#!/usr/bin/env bash
# bench-report.sh - 汇总基准结果，输出 Markdown 报告（+ 可选机器可读 JSON）
#
# 数据来源：
#   1. Criterion（target/criterion/**/new/estimates.json）—— Yaoxiang 自身的
#      微基准，用于**跨 nightly 的回归追踪**（每次跑同样 workload，比历史）
#   2. shootout（可选，--shootout <file>）—— 多语言横比。结果是文本，本脚本
#      只做搬运与格式化，不解析（格式由 runner 决定，避免两处维护）
#
# 用法:
#   ./scripts/ci/bench-report.sh                     # 只汇总 Criterion
#   ./scripts/ci/bench-report.sh --shootout out.txt  # 附带 shootout 文本
#   ./scripts/ci/bench-report.sh --out-json r.json   # 同时写机器可读结果
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

while [[ $# -gt 0 ]]; do
  case "$1" in
    --criterion-dir) CRITERION_DIR="$2"; shift 2 ;;
    --shootout)      SHOOTOUT_FILE="$2"; shift 2 ;;
    --out-json)      OUT_JSON="$2"; shift 2 ;;
    -h|--help)       sed -n '2,20p' "$0"; exit 0 ;;
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

# ── Criterion 汇总（estimates.json 的 mean.point_estimate 单位是纳秒）──
"$PY" - "$CRITERION_DIR" "$OUT_JSON" <<'PYEOF'
import json
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
# ── 解析 shootout 输出文件 ──
# 同理：调用方可能给的确切路径不存在，则按文件名在 artifact 根下搜索。
resolve_shootout_file() {
  local given="$1"
  [[ -z "$given" ]] && { echo ""; return; }
  [[ -f "$given" ]] && { echo "$given"; return; }
  local base found
  base="$(basename "$given")"
  # 搜给定路径的同名文件（可能位于 artifact 内的不同层次）
  found="$(find "$(dirname "$given")" -type f -name "$base" -print -quit 2>/dev/null || true)"
  echo "${found:-$given}"
}
if [[ -n "$SHOOTOUT_FILE" ]]; then
  SHOOTOUT_FILE="$(resolve_shootout_file "$SHOOTOUT_FILE")"
fi

# ── shootout 文本（原样搬运，不解析）──
if [[ -n "$SHOOTOUT_FILE" ]]; then
  echo
  if [[ -f "$SHOOTOUT_FILE" ]]; then
    echo "### 多语言横比（shootout）"
    echo
    echo '```'
    cat "$SHOOTOUT_FILE"
    echo '```'
  else
    echo "> ⚠ 未找到 shootout 输出文件 \`$SHOOTOUT_FILE\`（该 benchmark 可能未运行）"
  fi
fi

echo
echo "---"
echo
echo "_机器可读结果见 workflow artifact；Criterion 目录：\`$CRITERION_DIR\`_"
