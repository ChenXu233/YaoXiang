#!/usr/bin/env bash
# 批量抽取 generate_expr_ir_inner 的 match 臂。
#
# 每个臂独立：快照 → 抽取 → 构建 → 等价性护栏；失败则只回滚该臂。
# 关键：不能用 git checkout 回滚（那会连带撤销同批次中已成功的臂），
# 必须用逐臂文件快照。
#
# 用法: bash tools/batch_extract.sh <arm1> <arm2> ...
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SRC=src/middle/core/ir_gen.rs
SNAP=/tmp/ir_gen_arm_snapshot.rs
PASS=()
FAIL=()
SKIP=()

for arm in "$@"; do
  echo "=========== $arm ==========="
  cp "$SRC" "$SNAP"

  if ! python tools/extract_arm.py "$arm" --apply >/tmp/ex.log 2>&1; then
    echo "跳过（抽取器拒绝）: $(tail -1 /tmp/ex.log)"
    cp "$SNAP" "$SRC"
    SKIP+=("$arm")
    continue
  fi
  rg -o "\([0-9]+ 行\) -> 委派 3 行" /tmp/ex.log | head -1

  if ! cargo build --bin yaoxiang-rs >/tmp/bd.log 2>&1; then
    echo "构建失败:"
    rg -v "locales 缺失" /tmp/bd.log | rg "^error" -A 4 | head -12
    cp "$SNAP" "$SRC"
    FAIL+=("$arm")
    continue
  fi

  if ! cargo test -q -p yaoxiang --lib >/tmp/ts.log 2>&1; then
    echo "测试失败:"
    rg -v "locales 缺失" /tmp/ts.log | rg "FAILED|panicked" | head -5
    cp "$SNAP" "$SRC"
    FAIL+=("$arm")
    continue
  fi

  out=$(bash tools/dump_equiv_check.sh 2>&1 | tail -2)
  if ! echo "$out" | rg -q "差异: 0"; then
    echo "护栏失败: $(echo "$out" | tail -1)"
    echo "$out" | rg "^差异:" | head -3
    cp "$SNAP" "$SRC"
    FAIL+=("$arm")
    continue
  fi
  echo "OK"
  PASS+=("$arm")
done

echo "================ 汇总 ================"
echo "通过 ${#PASS[@]}: ${PASS[*]:-}"
echo "跳过 ${#SKIP[@]}: ${SKIP[*]:-}"
echo "失败 ${#FAIL[@]}: ${FAIL[*]:-}"
