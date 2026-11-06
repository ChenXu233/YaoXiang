---
title: '测试三扇门普查（0.6 大清理前置）'
description: '按 coding-rules 第七部分的三扇门，对全仓测试逐条机械分类，给出首批退役候选清单'
---

# 测试三扇门普查（0.6 大清理前置）

WBS 0.6「大清理」的第一步：把全仓 `#[test]` 按 [coding-rules 第七部分](./coding-rules.md) 的
**三扇门（条款 / 病史 / 证明）** 机械分类，产出一份可重跑的逐条判定表与首批退役候选清单。
**本页与生成器只读不写测试**——本阶段不删除、不修改任何测试。

生成物：

| 产物 | 作用 |
| --- | --- |
| `docs/src/dev/test-gate-census.tsv` | 逐条判定表（3445 行 + 表头），与本页同目录 |
| `scripts/dev/test-gate-census.py` | 生成器（可重跑、可 `--check`、可 `--selftest`） |

## 口径

### 扫描面

`src/`、`tests/`、`benches/`、`tools/` 下所有 `.rs` 的 `#[test]`：

| 面 | 条数 |
| --- | --- |
| 根包 lib（`src/`，含各模块 `tests/` 子目录） | 2971 |
| `tests/integration` | 335 |
| `tests/formatter_integration.rs` | 52 |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs` | 2 / 1 |
| `tools/` | 75 |
| `benches/` | 9 |
| **合计** | **3445**（296 个文件） |

### 三扇门的机器判据

| 门 | 机器判据 | 命中含义 |
| --- | --- | --- |
| 条款门 | 文件头前 25 行命中 `§\s*\d` / `RFC\s*\d{3}` / `SPEC\s*[A-Z0-9]` / `language-spec` / `语言规范`；`src/**/tests/` 下的文件连父 `mod.rs` 一起看 | 该测试对得上一条条款 |
| 病史门 | 测试函数体、函数文档注释，或**文件头**（`//! 回归背景：issue #226` 这类）点名真实故障：`issue #N` / 回归 / 缺陷 / 事故 | 该测试钉的是一次已发生的事故 |
| 证明门 | **机器判不了**（「换一个正确的替代实现，它会不会红」需要人判）。故前两门都不过的一律落「待判」，不自动判成「均不通过」；唯一例外是体内**一条断言都没有**（含委托给 `assert*/check*` 助手的也没有、也没有 `#[should_panic]`）——那不是判据薄弱，是根本没断言 | 只能给出「待判」或「无断言」 |

文件头半径的理由：本仓惯例是把「回归背景：issue #N」写在文件头，逐条测试注释里不重复；
只看测试自身注释会把整份文件的病史漏掉（实测：测试自身半径 7 条，文件头半径 291 条）。

### 永不判退役的类别

以下测试**一律「留」**，不进入退役候选：

- 语料 `tests/yaoxiang/**`、项目形态 `tests/project-corpus/**`（行为判据面）；
- `tests/integration/**`、`tests/formatter_integration.rs`、`tests/yx_runner*.rs`
  （spec-shaped，走进程边界）；
- 测试名含 `unchanged` / `anchor` / `guard` / `preserv` / `baseline` 的现状锚。

### 处置的语义

`退役候选` 是**候选不是判决**：它表示「条款门与病史门都没过，且断言面只有弱断言（或没有断言）」，
证明门仍需人工复核。卡片只要求列清单，不执行删除。

## 分布

| 门 | 条数 |
| --- | --- |
| 条款 | 2403 |
| 病史 | 47 |
| 证明（spec-shaped 判据面 / 现状锚） | 447 |
| 待判（两门皆不过，证明门待人工） | 512 |
| 均不通过（体内无任何断言） | 36 |
| **合计** | **3445** |

| 处置 | 条数 |
| --- | --- |
| 留 | 2897 |
| 待判 | 412 |
| 退役候选 | 136 |
| **合计** | **3445** |

文件口径：296 个测试文件里 **221 个（74.7%）** 的文件头有 SPEC/RFC/§ 引用。

退役候选按目录聚簇（前六）：

| 目录 | 候选数 |
| --- | --- |
| `src/frontend/core/types/eval/tests` | 26 |
| `src/lsp/handlers/tests` | 19 |
| `src/frontend/core/parser/pratt/tests` | 18 |
| `src/lsp/tests` | 14 |
| `src/package/vendor/tests` | 8 |
| `src/backends/interpreter/tests` | 7 |

## 与 0.6-2 勘察的对账

0.6-2 的勘察数字是本页的对照基线。差异逐项列原因，不用「口径不同」一句带过：

| 项 | 0.6-2 勘察 | 本次普查 | 差异原因 |
| --- | --- | --- | --- |
| 全仓 `#[test]` | 3443 | 3445 | 0.6-2 之后新增 2 条（E3005 与 E3022 的 check 口径回归测试），根包 lib 2969 → 2971 |
| 测试文件数 | 296 | 296 | 未变 |
| 条款门（文件口径） | 227（77%） | 221（74.7%） | 判据窗口不同：本次取「前 25 行 + `src/**/tests/` 的父 `mod.rs`」；0.6-2 只取文件头。差 6 个文件 |
| 病史信号 | 442 条 / 148 文件 | 三半径：测试自身 7 条；文件头 291 条 / 38 文件；整文件 1218 条 / 79 文件 | 0.6-2 的「148 文件」比本次**整文件半径的 79 文件**还宽，说明它扫的词表或范围更大（本次严格按卡面四信号：`issue #N`、回归、缺陷、事故）。本次按三半径如实列出，不合并成一个数字 |
| 特征化强信号 | 1176（34%）/ 强 631（18%） | 不测 | 特征化**不是**三扇门之一：它衡量「测试写得像不像快照」，本页只判「能不能退役」。要用它得另立判据，不混进退役结论 |
| 弱断言 | 473（14%） | 见 TSV「断言摘要」列（逐条给 `assert_eq×N / is_ok×N / 无断言`） | 计数单位不同：0.6-2 按「只断言 is_ok/is_some」的模式统计；本次按词法计数并区分强断言 / 弱断言 / 委托断言，且它是**处置判据的一部分**而不单独成表 |

## 结论摘要

1. **条款门是主门**：2403 条（69.8%）的测试所在文件头有 SPEC/RFC/§ 引用——说明「测试对得上条款」
   在这套代码库里是常态，不是特例。
2. **退役面很小**：136 条候选（3.9%）。0.6-2 已证明「完全重复的测试」只占 1%，故退役收益不在
   「删重复」，而在「删掉什么都没断言的冒烟测试 + 弱断言测试」。
3. **真正无断言的是极少数但刺眼**：36 条测试体内一条断言都没有（连委托断言助手都没有），
   它们是「跑一遍不 panic」的冒烟测试——首批清单就从这里挑。
4. **待判是最大的一块未决面**：512 条两门皆不过但有强断言，证明门需要人判；本页不为它们下结论。

## 首批退役清单（只列不删）

**批次：`src/frontend/core/types/eval/tests/` 的 26 条「无断言」测试**（单一目录簇，理由同族：
体内无任何断言）。选取理由：这一簇是全仓唯一的「整目录型无断言簇」，删除判据最硬——测试不
断言任何东西时，删掉不丢判据；同目录另有 327 条测试（含断言的源行数：`conditional.rs` 127、
`const_eval.rs` 158、`operations.rs` 72、`normalizer.rs` 60、`reducer.rs` 60）覆盖同一批 API
的真实行为。

| # | 测试名 | 文件 | 它断言什么 | 为什么删了不丢判据 |
| --- | --- | --- | --- | --- |
| 1 | `test_normalizer_with_config` | normalizer.rs | 无断言：构造 `TypeNormalizer` 后 `normalize(Int(32))` 丢弃结果 | 无断言可丢；范式化结果由同文件 `test_normalizer_normalize_primitives` 用 `assert_eq!` 锁定 |
| 2 | `test_normalizer_evaluator` | normalizer.rs | 无断言：只取 `evaluator()` 句柄，不调用 | 无断言可丢；句柄访问由同文件带断言的归约用例顺带覆盖 |
| 3 | `test_normalizer_context` | normalizer.rs | 无断言：只取 `context()` 句柄 | 同上 |
| 4 | `test_type_computer_new_and_with_config` | reducer.rs | 无断言：`new()` / `with_config()` 构造后丢弃 | 无断言可丢；构造是其余计算用例的必经步骤（它们带断言） |
| 5 | `test_type_computer_register_alias` | reducer.rs | 无断言：注册别名后 `compute()` 丢弃结果 | 无断言可丢；别名归约结果由 `test_reducer_delta_reduces_alias` 用 `assert_eq!` 锁定 |
| 6 | `test_type_computer_compute_primitive` | reducer.rs | 无断言：`compute(Int(32))` 丢弃 | 无断言可丢；基本类型计算由 `test_reducer_stuck_on_primitives` 等带断言用例覆盖 |
| 7 | `test_type_computer_compute_type_ref` | reducer.rs | 无断言：注册别名后 `compute(TypeRef)` 丢弃 | 无断言可丢；同 #5 |
| 8 | `test_type_computer_set_context` | reducer.rs | 无断言：只取 `context()`（注释自称「just verify can be created」） | 无断言可丢 |
| 9 | `test_type_computer_compute_type_var` | reducer.rs | 无断言：`compute(TypeVar)` 丢弃 | 无断言可丢；类型变量归约由 `test_reducer_reduce_type_var` 覆盖 |
| 10 | `test_type_computer_compute_list` | reducer.rs | 无断言：`compute(List<Int>)` 丢弃 | 无断言可丢；列表归约由 `test_reducer_reduce_list_type` 覆盖 |
| 11 | `test_type_computer_compute_tuple` | reducer.rs | 无断言：`compute((Int, String))` 丢弃 | 无断言可丢；元组归约由 `test_reducer_reduce_tuple_type` 覆盖 |
| 12 | `test_type_computer_compute_fn` | reducer.rs | 无断言：`compute(Fn)` 丢弃 | 无断言可丢；函数类型归约由 `test_reducer_reduce_function_type` 覆盖 |
| 13 | `test_reducer_reduce_with_limit` | reducer.rs | 无断言：`reduce_with_limit()` 丢弃结果 | 无断言可丢；步数上限语义由 `test_reducer_step_count`（`assert_eq!`）锁定 |
| 14 | `test_reducer_reduce_type_ref_with_alias_chain` | reducer.rs | 无断言：A→B→Int 链式别名归约丢弃结果 | 无断言可丢；单跳别名归约由 `test_reducer_delta_reduces_alias` 锁定 |
| 15 | `test_reducer_reduce_type_ref_self_referential` | reducer.rs | 无断言：自引用别名归约丢弃结果 | 无断言可丢；自引用停在原地由 `test_reducer_unknown_alias_stays_stuck` 的 `assert_eq!` 覆盖 |
| 16 | `test_unifier_different_types` | reducer.rs | 无断言：`unify(Int, String)` 后 `match` 三个分支全空 | 无断言可丢；不同类型不统一由 `test_unifier_unify_different_concrete` 覆盖 |
| 17 | `test_unifier_substitution_after_unify` | reducer.rs | 无断言：统一后取替换表即丢弃 | 无断言可丢；替换表由 `test_unifier_same_types` 等带断言用例覆盖 |
| 18 | `test_unifier_unify_option_same` | reducer.rs | 无断言：`unify(Option<Int>, Option<Int>)` 丢弃（注释写「may fail or succeed」） | 无断言可丢；该形态本就未定义结论，删掉不丢判据 |
| 19 | `test_unifier_unify_result_same` | reducer.rs | 同上（Result） | 同上 |
| 20 | `test_unifier_unify_dict_same` | reducer.rs | 同上（Dict） | 同上 |
| 21 | `test_unifier_unify_set_same` | reducer.rs | 无断言：`unify(Set<Bool>, Set<Bool>)` 丢弃 | 无断言可丢；容器类型统一由 `test_unifier_unify_composite_types` 覆盖 |
| 22 | `test_unifier_unify_range_same` | reducer.rs | 无断言（Range） | 同上 |
| 23 | `test_unifier_unify_arc_same` | reducer.rs | 无断言（Arc） | 同上 |
| 24 | `test_unifier_unify_weak_same` | reducer.rs | 无断言（Weak） | 同上 |
| 25 | `test_unifier_unify_enum_same` | reducer.rs | 无断言（Enum） | 同上 |
| 26 | `test_unifier_unify_struct_same` | reducer.rs | 无断言（Struct） | 同上 |

**删除前置条件（留给执行批次的卡，本卡不做）**：① 同目录带断言的用例仍覆盖同一 API（否则先把
断言补进那些用例）；② 删除与 `cargo test --lib` / `run fast` 同批验证；③ 同提交更新本页与
`test-gate-census.tsv`（`--check` 会红）；④ 按 [coding-rules](./coding-rules.md) 第七部分登记。

## 复跑与自检

```bash
python scripts/dev/test-gate-census.py            # 重算并覆写 TSV（两次输出逐字节相同）
python scripts/dev/test-gate-census.py --check    # CI/提交前：TSV 是否与当前树一致
python scripts/dev/test-gate-census.py --selftest # 最小正/反例自检（不扫仓库）
python scripts/dev/test-gate-census.py --stdout   # 只输出 TSV
```

生成器只做词法判定，边界如实说明：

- **委托断言只按名字识别**（`assert*/check*` 助手函数）。若某测试的断言藏在名字不带这两类前缀的
  助手里，它会被误判成「无断言」——首批 26 条已逐条人工复核过函数体，本页给出的理由以人工复核为准。
- **证明门不判**：任何「两门皆不过」的测试只落「待判」或「退役候选」，生成器不宣布「该删」。
- **不跨文件去重**：完全重复的测试不在本表识别（0.6-2 已实测仅 1%，不构成退役面）。
