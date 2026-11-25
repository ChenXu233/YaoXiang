#!/usr/bin/env bash
# 门禁 9.6（WBS P2 / RFC-039）：IR 静态校验器语料门禁。
#
# 三条判据（与下方三条 run_verify_test 一一对应）：
#   1. verify_loose：全语料（tests/yaoxiang/ + src/std/tests/ 中到达 IR 生成的文件）
#      产出的 ModuleIR 过宽松校验，且校验面非空（防「校验器没跑到」的假绿——
#      非空断言内置于测试：files > 200 且 fns/instrs > 0）。
#   2. verify_ssa：同批语料的 ModuleIR 过严格校验（值定义唯一性、值使用受定义支配、
#      defining_block 一致、Phi 一致性）。
#   3. verify_ssa_negative_controls：合成坏 IR 必须被拒绝（负例自证——证明校验器
#      确实拦得住，而不是只证明生产语料绿）。
#
# 跑红 = ir_gen 存在隐式缺陷（D38：不开豁免，修 ir_gen 而非放宽判据）。
set -euo pipefail
cd "$(dirname "$0")/../.."

run_verify_test() {
    local test_name="$1"
    local label="$2"
    local output
    output=$(cargo test --all-features --lib middle::ir::tests::verify -- --exact "$test_name" 2>&1) || {
        # 失败时打印完整 cargo 输出：语料判据会把全部违规拼在失败消息里，
        # 截尾会丢掉最早出问题的文件。
        printf '%s\n' "$output"
        echo "check-ir-verifier: FAIL（$label 判据红）"
        exit 1
    }
    echo "$output" | tail -3
    # 防假门禁双保险：精确匹配测试摘要，避免 "11 passed" 等子串误命中。
    if ! echo "$output" | grep -qE "test result: ok\. 1 passed; 0 failed"; then
        printf '%s\n' "$output"
        echo "check-ir-verifier: FAIL（$label 判据测试未真实执行或未通过）"
        exit 1
    fi
}

run_verify_test middle::ir::tests::verify::test_verify_loose_corpus_green "verify_loose"
run_verify_test middle::ir::tests::verify::test_verify_ssa_corpus_green "verify_ssa"
run_verify_test middle::ir::tests::verify::test_verify_ssa_negative_controls "verify_ssa_negative_controls"
echo "check-ir-verifier: PASS（全语料 verify_loose + verify_ssa 绿，负例自证绿，校验面非空）"
