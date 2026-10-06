#!/usr/bin/env bash
# 门禁 9.6（WBS P2 / RFC-039）：IR 静态校验器语料门禁。
#
# 判据：全语料（tests/yaoxiang/ + src/std/tests/ 中到达 IR 生成的文件）产出的
# ModuleIR 必须过 verify_loose，且校验面非空（防「校验器没跑到」的假绿——
# 非空断言内置于测试：files > 200 且 fns/instrs > 0）。
#
# 跑红 = ir_gen 存在隐式缺陷（D38：不开豁免，修 ir_gen 而非放宽判据）。
set -euo pipefail
cd "$(dirname "$0")/../.."

output=$(cargo test --all-features --lib middle::core::tests::verify -- --exact     middle::core::tests::verify::test_verify_loose_corpus_green 2>&1) || {
    echo "$output" | tail -30
    echo "check-ir-verifier: FAIL（verify_loose 语料判据红）"
    exit 1
}
echo "$output" | tail -3
# 防假门禁双保险：测试必须真实执行且通过（1 passed）
echo "$output" | grep -q "1 passed" || {
    echo "check-ir-verifier: FAIL（判据测试未执行——过滤名失效？）"
    exit 1
}
echo "check-ir-verifier: PASS（全语料 verify_loose 绿，校验面非空）"
