#!/usr/bin/env bash
# 门禁 9.5（WBS P2 / RFC-039）：规范化 IR 快照漂移检测。
#
# 判据：全语料规范化 IR 文本与入库快照（src/middle/core/tests/snapshots/）
# 逐字节等值；非空断言内置于判据测试（compared > 200，防假门禁）。
#
# 漂移 = IR 行为面变化：回归（修代码）或有意变更（走 07 §128 更新流程：
# UPDATE_SNAPSHOTS=1 更新 + git diff 人工 review + 同 PR 入库）。
set -euo pipefail
cd "$(dirname "$0")/../.."

output=$(cargo test --all-features --lib middle::core::tests::snapshot -- --exact     middle::core::tests::snapshot::test_ir_snapshots_stable 2>&1) || {
    echo "$output" | tail -30
    echo "check-snapshot-drift: FAIL（IR 快照漂移；有意变更请走 UPDATE_SNAPSHOTS 流程）"
    exit 1
}
echo "$output" | tail -3
echo "$output" | grep -q "1 passed" || {
    echo "check-snapshot-drift: FAIL（判据测试未执行——过滤名失效？）"
    exit 1
}
echo "check-snapshot-drift: PASS（204 快照零漂移）"
