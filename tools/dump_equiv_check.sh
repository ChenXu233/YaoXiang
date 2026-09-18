#!/usr/bin/env bash
# 表达式生成器拆分的输出等价性护栏。
# 用法: bash tools/dump_equiv_check.sh [快照目录]
# 退出码 0 = 与基线逐字节一致；非 0 = 有差异（列出差异文件）
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="${1:-/tmp/expr_base}"
BIN="$ROOT/target/debug/yaoxiang-rs.exe"

if [ ! -d "$BASE" ]; then
  echo "基线目录不存在: $BASE" >&2
  exit 2
fi

diffs=0
checked=0
while IFS= read -r f; do
  safe=$(echo "$f" | sed 's|tests/yaoxiang/||; s|/|__|g')
  snap="$BASE/$safe.txt"
  [ -f "$snap" ] || continue
  checked=$((checked + 1))
  cur=$(cd "$ROOT" && "$BIN" dump "$f" 2>&1 | sed 's/\x1b\[[0-9;]*m//g')
  if [ "$cur" != "$(cat "$snap")" ]; then
    echo "差异: $f"
    diffs=$((diffs + 1))
  fi
done < <(cd "$ROOT" && find tests/yaoxiang -name "*.yx" | sort)

echo "---"
echo "已比对: $checked  差异: $diffs"
[ "$diffs" -eq 0 ]
