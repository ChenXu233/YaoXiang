#!/usr/bin/env bash
# 门禁 9.5（WBS P2 / RFC-039）：规范化 IR 快照漂移检测。
#
# 判据：全语料规范化 IR 文本与入库快照（src/middle/ir/tests/snapshots/）
# 逐字节等值。非空/对账断言内置于判据测试（WBS 6.6.16 起）：凡到达 IR 却拿不到
# 规范化文本的语料**跳过即红**并逐条点名，且 compared == reachable（到达 IR 的语料数）；
# 另有 reachable > 200 的语料面存在性守卫（防语料发现机制坏掉时静默绿）。
#
# 漂移 = IR 行为面变化：回归（修代码）或有意变更（走 07 §128 更新流程：
# UPDATE_SNAPSHOTS=1 更新 + git diff 人工 review + 同 PR 入库）。
set -euo pipefail
cd "$(dirname "$0")/../.."

output=$(cargo test --all-features --lib middle::ir::tests::snapshot -- --exact     middle::ir::tests::snapshot::test_ir_snapshots_stable 2>&1) || {
    echo "$output" | tail -30
    echo "check-snapshot-drift: FAIL（IR 快照漂移；有意变更请走 UPDATE_SNAPSHOTS 流程）"
    exit 1
}
echo "$output" | tail -3
echo "$output" | grep -q "1 passed" || {
    echo "check-snapshot-drift: FAIL（判据测试未执行——过滤名失效？）"
    exit 1
}
# 快照数动态取自快照目录实际 .snap 数量（不写死：语料/快照增长时不会过期）。
snap_count=$(find src/middle/ir/tests/snapshots -type f -name "*.snap" | wc -l | tr -d " ")
echo "check-snapshot-drift: PASS（${snap_count} 快照零漂移）"
