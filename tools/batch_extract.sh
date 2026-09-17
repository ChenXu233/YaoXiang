#!/usr/bin/env bash
# 批量抽取 generate_expr_ir_inner 的 match 臂。
# 每个臂独立：抽取 → 构建 → 护栏比对；失败则整臂回滚并记录，继续下一个。
# 用法: bash tools/batch_extract.sh <arm1> <arm2> ...
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PASS=()
FAIL=()
for arm in "$@"; do
  echo "=========== $arm ==========="
  if ! python tools/extract_arm.py "$arm" --apply >/tmp/ex.log 2>&1; then
    echo "抽取失败: $(tail -2 /tmp/ex.log)"
    git checkout src/middle/core/ir_gen.rs
    FAIL+=("$arm:extract")
    continue
  fi
  tail -3 /tmp/ex.log

  if ! cargo build --bin yaoxiang-rs >/tmp/bd.log 2>&1; then
    echo "构建失败:"
    rg -v "locales 缺失" /tmp/bd.log | rg "^error" -A 5 | head -14
    git checkout src/middle/core/ir_gen.rs
    FAIL+=("$arm:build")
    continue
  fi

  out=$(bash tools/dump_equiv_check.sh 2>&1 | tail -2)
  if ! echo "$out" | rg -q "差异: 0"; then
    echo "护栏失败: $out"
    git checkout src/middle/core/ir_gen.rs
    FAIL+=("$arm:equiv")
    continue
  fi
  echo "OK"
  PASS+=("$arm")
done

echo "================ 汇总 ================"
echo "通过 ${#PASS[@]}: ${PASS[*]:-}"
echo "失败 ${#FAIL[@]}: ${FAIL[*]:-}"
