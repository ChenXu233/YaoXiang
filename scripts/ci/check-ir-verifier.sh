#!/usr/bin/env bash
# 门禁 9.6（WBS P2 / RFC-039）：IR 静态校验器语料门禁。
#
# 判据：全语料（tests/yaoxiang/ + src/std/tests/ 中到达 IR 生成的文件）产出的
# ModuleIR 必须过 verify_loose 与 verify_ssa，且校验面非空（防「校验器没跑到」的
# 假绿——非空断言内置于测试：files > 200 且 fns/instrs > 0）。
#
# 跑红 = ir_gen 存在隐式缺陷（D38：不开豁免，修 ir_gen 而非放宽判据）。
set -euo pipefail
cd "$(dirname "$0")/../.."

run_verify_test() {
    local test_name="$1"
    local label="$2"
    local output
    output=$(cargo test --all-features --lib middle::ir::tests::verify -- --exact "$test_name" 2>&1) || {
        echo "$output" | tail -30
        echo "check-ir-verifier: FAIL（$label 语料判据红）"
        exit 1
    }
    echo "$output" | tail -3
    # 防假门禁双保险：该条测试必须真实执行且通过（1 passed）
    if ! echo "$output" | grep -q "1 passed"; then
        echo "check-ir-verifier: FAIL（$label 判据测试未执行——过滤名失效？）"
        exit 1
    fi
}

run_verify_test middle::ir::tests::verify::test_verify_loose_corpus_green "verify_loose"
run_verify_test middle::ir::tests::verify::test_verify_ssa_corpus_green "verify_ssa"
echo "check-ir-verifier: PASS（全语料 verify_loose + verify_ssa 绿，校验面非空）"
