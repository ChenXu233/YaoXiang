---
title: '测试三扇门普查（0.6 大清理前置）'
description: '按 coding-rules 第七部分的三扇门，对全仓测试逐条机械分类，并给出条款门抽样审计与首批退役候选清单'
---

# 测试三扇门普查（0.6 大清理前置）

WBS 0.6「大清理」的前置材料：把全仓 `#[test]` 按 [coding-rules 第七部分](./coding-rules.md) 的
**三扇门（条款 / 病史 / 证明）** 机械分类，产出一份可重跑的逐条判定表；再对条款门做一次
分层抽样审计（40 条），量出「引了条款但撑不住断言」的比例，并用**收紧后的判据**重算分布。
**本页与生成器只读不写测试**——本阶段不删除、不修改任何测试。

生成物：

| 产物 | 作用 |
| --- | --- |
| `docs/src/dev/test-gate-census.tsv` | 逐条判定表（3424 行 + 表头），与本页同目录 |
| `scripts/dev/test-gate-census.py` | 生成器（可重跑、可 `--check`、可 `--selftest`） |
| `scripts/dev/test-gate-census-audit.py` | 抽样审计：按固定种子复现 40 条样本（`--sample` / `--check`） |
| `scripts/dev/test-gate-census-audit.json` | 40 条逐条判定与依据（生成器读它把「审计确认真实支撑」判为留） |

## 口径

### 扫描面

`src/`、`tests/`、`benches/`、`tools/` 下所有 `.rs` 的 `#[test]`：

| 面 | 条数 |
| --- | --- |
| 根包 lib（`src/`，含各模块 `tests/` 子目录） | 2950 |
| `tests/integration` | 335 |
| `tests/formatter_integration.rs` | 52 |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs` | 2 / 1 |
| `tools/` | 75 |
| `benches/` | 9 |
| **合计** | **3424**（296 个文件） |

### 三扇门的机器判据

| 门 | 机器判据 | 命中含义 |
| --- | --- | --- |
| 条款门（0.6-10 收紧） | 文件头前 25 行出现**可核条目**：节号形态 `§<数字>`（含 `RFC-<数字> §<数字>`、`SPEC §<数字>`）；`src/**/tests/` 下的文件连父 `mod.rs` 一起看 | 该测试对得上一条可指名的条款 |
| 病史门 | 测试函数体、函数文档注释，或**文件头**（`//! 回归背景：issue #226` 这类）点名真实故障：`issue #N` / 回归 / 缺陷 / 事故 | 该测试钉的是一次已发生的事故 |
| 证明门 | **机器判不了**（「换一个正确的替代实现，它会不会红」需要人判）。故前两门都不过的一律落「待判」，不自动判成「均不通过」；唯一例外是体内**一条断言都没有**（含委托给 `assert*/check*` 助手的也没有、也没有 `#[should_panic]`） | 只能给出「待判」或「无断言」 |

收紧的理由：0.6-9 的条款门只要求文件头**出现 `§`/`RFC`/`SPEC` 字样**，于是 2403 条被判「留」。
0.6-10 的 40 条分层抽样实测：**一半的引用撑不住断言**（见下节）。收紧后只认节号形态，且
**有引用仍不等于引用撑得住**——未经审计的条款引用一律只到「待核」。

文件头半径的理由：本仓惯例是把「回归背景：issue #N」写在文件头，逐条测试注释里不重复；
只看测试自身注释会把整份文件的病史漏掉（实测：测试自身半径 7 条，文件头半径 291 条）。

### 永不判退役的类别

以下测试**一律「留」**，不进入退役候选：

- 语料 `tests/yaoxiang/**`、项目形态 `tests/project-corpus/**`（行为判据面）；
- `tests/integration/**`、`tests/formatter_integration.rs`、`tests/yx_runner*.rs`
  （spec-shaped，走进程边界）；
- 测试名含 `unchanged` / `anchor` / `guard` / `preserv` / `baseline` 的现状锚。

### 处置的语义

| 处置 | 含义 |
| --- | --- |
| 留 | 只给三种：病史门命中、证明门（spec-shaped 目录 / 现状锚名字）、**抽样审计确认真实支撑**的条款引用 |
| 待核 | 引了条款（有节号，或只引整篇 RFC 无节号）但未经审计/审计判为不相关或泛泛——要人核过才能留 |
| 退役候选 | 两门皆不过且断言面只有弱断言（或无断言）——候选不是判决，证明门仍需人工复核 |
| 待判 | 两门皆不过但有强断言，机器不给结论 |

## 分布（收紧后）

| 门 | 条数 |
| --- | --- |
| 条款（有可核条目） | 1713 |
| 病史 | 314 |
| 证明（spec-shaped 判据面 / 现状锚） | 447 |
| 待判（两门皆不过，证明门待人工） | 935 |
| 均不通过（体内无任何断言） | 15 |
| **合计** | **3424** |

| 处置 | 条数 | 对比 0.6-9 |
| --- | --- | --- |
| 留 | **776** | 2897（−2121） |
| 待核 | 2121 | 0（新增档位） |
| 待判 | 412 | 412（不变） |
| 退役候选 | 115 | 136（第一批退役 21 条） |
| **合计** | **3424** | — |

两点口径变化要说明：

1. **留的结构**：776 = 证明门 447 + 病史门 314 + 审计确认真实支撑 15（审计的 20 条真实支撑里，
   另 5 条所在文件本就因病史门先行留）。原 2897 里那 2121 条「凭一句引用就留」的，现在落待核。
2. **病史门 47 → 314**：不是新增病史，而是**标签归位**——0.6-9 里「条款与病史同时命中」记作条款，
   收紧后条款不再自动给留，这类必须由病史门接住，故计数并入病史。留的总数因此不受影响
   （该批测试在 0.6-9 也是留）。

文件口径：296 个测试文件里，文件头**有可核节号**的是 **113 个**（0.6-9 的旧判据是 221 个）。

## 条款门抽样审计（0.6-10）

### 抽样口径（可复现）

- **候选池 = 0.6-9 条款门判为「条款」的 2403 条**（冻结在 `test-gate-census-audit.py` 里的旧判据，
  抽样分母不能跟着新判据变，否则结论不可比）。
- **分层 = 目录簇**：`src/` 下取前两层（`src/frontend`、`src/middle`…），其余按顶层
  （`tests`、`benches`、`tools`）。
- **分配**：按层内条数比例（最大余数法），每层至少 1 条。
- **抽取**：层内按 (文件, 行号, 测试名) 排序后用固定种子随机取样。
- **种子 = 20261106**（写死在脚本里；换种子等于换样本，必须在文档里说明并重跑审计）。

分层结果：`src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 · `src/backends` 2 ·
`src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 · `src/lsp` 1 · `src/std` 1。

### 40 条逐条判定

判定标准（写死在这里，避免逐条各说各话）：

- **真实支撑**：引用能指名到具体节/注记/判定号，且其**主题**与断言对象同一；
- **不相关**：引用能指名到具体条目，但该条目讲的是**另一件事**（依据列写出它实际讲什么）；
- **泛泛**：引用只到整篇文档/整章/章节罗列/阶段名，没有一条能对上断言。

| # | 测试 | 文件 | 判定 | 依据（引的条款实际讲什么） |
| --- | --- | --- | --- | --- |
| 1 | `test_compile_time_value_param_spec_table` | `classify.rs` | 真实支撑 | type-system §7.1「编译期值参数」正是断言对象：值位置/类型位置参数判定的真值表逐行验。 |
| 2 | `test_id_stable_under_subsequent_registrations` | `value.rs` | 真实支撑 | RFC-010「运行时表示与相等」的和类型身份约定＝身份只由名字决定；断言 ID 不随后续登记变化。 |
| 3 | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs` | 真实支撑 | 02-stage-contract §3 注记 #14 第③点讲单态化命名同源；测试正是该注记的钉测（名字与断言都对得上）。 |
| 4 | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure` | `import_cycle.rs` | 真实支撑 | RFC-013 的 W1xxx 警告码 + 用户裁决 A（环不得静默）；断言良性环恰一条 W1007 且无运行期后果预判。 |
| 5 | `test_obligation_config_disabled_skip_is_not_owed` | `obligations.rs` | 真实支撑 | 02-stage-contract §2「义务账本」；断言 ConfigDisabled 跳过不构成欠账。 |
| 6 | `test_format_type_bool` | `types.rs` | 真实支撑 | formatter 规范 §9 类型注解；断言类型名原样输出。 |
| 7 | `test_fstring_multiline_single_quotes_inside` | `fstring.rs` | 真实支撑 | syntax §1.6.3 字符串字面量；断言多行 f-string 内容里的单个引号不终止。 |
| 8 | `test_float_underscore` | `literals.rs` | 真实支撑 | syntax §1.6 字面量（1.6.2 浮点数）覆盖下划线分隔；断言 1_000.5 词法化为 1000.5。 |
| 9 | `test_empty_unicode_escape_is_error` | `literals.rs` | 真实支撑 | syntax §1.6.3 的转义闭集；断言空的 \u{} 转义必须报错。 |
| 10 | `test_probe_accepted_const_type_is_registered` | `probe.rs` | 真实支撑 | RFC-039 P6 6.4 / WBS 6.6.9 就是「常量参数判定只听从注入探针」；断言探针认 Int 时 N 被登记。 |
| 11 | `test_direct_eval_with_bound_variable_proved` | `predicate.rs` | 真实支撑 | RFC-027 §3-4（路径条件传播 / 编译期证明管道）；断言 b=5 时 b>0 直接求值为 Proved。 |
| 12 | `test_unused_type_is_not_an_entry_point` | `dead_code.rs` | 真实支撑 | RFC-011 §7 死代码消除机制；断言未使用的类型定义不是可达根。 |
| 13 | `test_display_forall_multiple_vars_no_trailing_space` | `ast.rs` | 真实支撑 | RFC-027 §8 SMT 求解器；断言 forall 多变量的打印格式。 |
| 14 | `test_dep_graph_remove_dependant_clears_edges` | `rfc027_phase31_dep_vc.rs` | 真实支撑 | RFC-027 Phase 3.1 的 TypeDepGraph；断言移除依赖者后其边与约束一并清除。 |
| 15 | `test_condition_bool` | `conditional.rs` | 真实支撑 | RFC-011 §5 条件类型；断言 TypeCondition::Bool 的求值与确定态。 |
| 16 | `test_convert_expr_to_const_expr_int_literal` | `const_eval.rs` | 真实支撑 | RFC-011 §4 编译期泛型；断言 Int 字面量转成 ConstExpr::Lit。 |
| 17 | `test_definition_on_binding_site_returns_none_precise_only` | `definition.rs` | 真实支撑 | RFC-039 D55（precise-only，禁止按名兜底）；断言定义点自身无引用记录时返回 None。 |
| 18 | `test_type_consistent_arith_passes` | `verify.rs` | 真实支撑 | WBS 2.1 / RFC-039 P2 的 IR 静态校验器判据（07-equivalence-oracle 第一层）；断言类型一致的算术通过。 |
| 19 | `test_verify_loose_corpus_green` | `verify.rs` | 真实支撑 | 同一条校验器判据；断言语料跑一遍全绿。 |
| 20 | `test_parse_tag_tolerates_query_and_fragment` | `toolchain.rs` | 真实支撑 | RFC-037 §安装器支持；断言 release URL 的 query/fragment 被剥离。 |
| 21 | `test_release_unset_register` | `execute.rs` | 不相关 | #299 §2 讲的是 NewArray 定长数组指令（分配、Void 占位、字节码往返）；断言的是「对未设置寄存器执行 Release 不 panic」。 |
| 22 | `test_fstring_segment_interpolation` | `ast.rs` | 不相关 | 引「语言规范 §3（AST 结构）」，但 syntax §3 是语句章（3.1 语句分类 … 3.11 程序入口与顶层语句）；断言的是 FStringSegment::Interpolation 的字段。 |
| 23 | `test_expr_for` | `ast.rs` | 不相关 | 同一条「§3（AST 结构）」引用；syntax §3 讲语句，断言的是 Expr::For 的 AST 字段。 |
| 24 | `test_add_var_without_mut_stores_immutability` | `scope.rs` | 不相关 | 引「语言规范 §5.2」：syntax 无第五章，type-system §5.2 是「多重约束」；断言的是 ScopeManager 存的不可变标志。 |
| 25 | `test_type_evaluator_eval_nested_type` | `evaluator.rs` | 不相关 | 引 syntax §3.11（程序入口与顶层语句）与「RFC-027 §362」（不存在）；断言的是嵌套类型的求值。 |
| 26 | `test_get_ast_type_universe_level_plain` | `mono.rs` | 不相关 | 引「语言规范 §3」，syntax §3 是语句章；断言的是类型宇宙层级为 0。 |
| 27 | `test_unify_enum_same` | `solver.rs` | 不相关 | 引「语言规范 §3」；断言同名枚举统一成功，属类型统一，不在语句章。 |
| 28 | `test_weak_new_upgrade_some_while_original_alive` | `weak.rs` | 不相关 | 引 RFC-010 的**和类型身份**，断言的是 weak 引用的生命周期（#384：原 Arc 存活时 upgrade 为 Some）。 |
| 29 | `test_i18n_registry_english_titles` | `codes.rs` | 不相关 | §3.4 的 I3/I4 讲 E1090/E1091 的翻译；断言的是 E0001 的 en 标题与 help 非空。 |
| 30 | `test_format_return_expression` | `expr.rs` | 泛泛 | 引用是 §3、§4、§7、§8、§10–§13、§17、§18 的章节罗列，没有一条能对上「return 表达式怎么渲染」。 |
| 31 | `test_rfc010_generic_syntax` | `lexer_mod.rs` | 泛泛 | 引用是 syntax §1.6 + RFC-004/RFC-010 泛列；断言自称 flexible（允许任意括号形态），没有可核条目。 |
| 32 | `test_anon_binding_with_fn_annotation_keeps_extracted_signature` | `types.rs` | 泛泛 | 「spec §3, RFC-010, RFC-011」泛列，没有一条对应「匿名函数绑定的注解签名提取」。 |
| 33 | `test_consumer_unknown_token_returns_empty` | `ownership.rs` | 泛泛 | 「RFC-009 + RFC-009a」整篇罗列；断言 BrandTree 对未知令牌返回空消费者集。 |
| 34 | `test_const_expr_neg_int_literal_displays_by_value` | `const_data.rs` | 泛泛 | RFC-011 §4 讲编译期泛型，不涉及 ConstExpr 的打印形态；断言的是负字面量打印成 -5。 |
| 35 | `resolve_local_and_method` | `resolver.rs` | 泛泛 | 「RFC-029 重构核心」是整篇架构 RFC；断言 resolver 的限定名与 DefKind。 |
| 36 | `test_outdated_exec_smoke` | `outdated.rs` | 泛泛 | RFC-014a 只是注释里的背景（注册表未发布故用路径依赖）；断言是 exec 冒烟，体内无断言。 |
| 37 | `test_compatible_high_major_range` | `resolver.rs` | 泛泛 | 「RFC-014 Phase 3」是阶段名不是条目；断言版本区间兼容判定。 |
| 38 | `test_clean_and_size` | `cache.rs` | 泛泛 | 「RFC-014 Phase 3（全局缓存设计）」同为阶段名；断言 cache clean 与 dir_size。 |
| 39 | `test_strip_ansi_multiple_consecutive_sequences` | `ansi.rs` | 泛泛 | §4.2 是「ANSI 工具提取」工作项不是规范条款；断言 strip_ansi 的连续序列行为。 |
| 40 | `test_verify_sha256_accepts_matching_digest_file_content` | `dl.rs` | 泛泛 | 引「RFC-037：版本目录 = 发行包解压根目录」；断言 sha256 校验接受匹配的摘要文件。 |

分布：**真实支撑 20 · 不相关 9 · 泛泛 11**。

### 错引率与置信区间

- 错引（不相关 + 泛泛）= 20/40 = **50%**；
- Wilson 95% 置信区间（n=40）：**[35.2%, 64.8%]**；
- 换算到 2403：真正有条款支撑 ≈ **1202 条**（95% 区间 **846 ~ 1557**），错引 ≈ **1201 条**
  （区间 846 ~ 1557）。

### 对清理面的影响

- **「留」从 2897 掉到 776**（−2121）——正是那 2121 条「凭引用就留」的落为待核；收紧生效，
  不是分布调参。
- **退役候选与条款无关**：首批 26 条本就与条款无关（体内一条断言都没有），审计不改它们的判定；第一批 21 条已退役，候选 136 → 115。
  这也说明「退役面」与「条款错引面」是两个问题：错引的测试多数有断言，该做的是补条款或改引用，
  而不是删测试。
- 待核 2121 条是后续「条款门补票」的工作量：给它们补上能对上断言的具体条目，或改判退役候选。

### 局限（不得越界引用）

- 40 条只能给出**比例**，不能把某一条的判定外推到同目录其它测试；
- 判定是人工读码 + 读条款文档得出的，依据列已写明「引的条款实际讲什么」供复核；
- 抽样分母是 0.6-9 的旧判据池（2403），**收紧后的 1713 条**里有多少错引没有重新抽样——
  收紧只是把「不可核」的引用挡在门外，不等于新池内的引用都真撑得住（它们仍需逐条核）。

## 与 0.6-2 勘察的对账

0.6-2 的勘察数字是第一版对照基线。差异逐项列原因：

| 项 | 0.6-2 勘察 | 本次 | 差异原因 |
| --- | --- | --- | --- |
| 全仓 `#[test]` | 3443 | 3424 | 0.6-2 之后新增 2 条（E3005 与 E3022 的 check 口径回归测试），又按首批退役清单删除 21 条无断言测试；净 −19 |
| 测试文件数 | 296 | 296 | 未变 |
| 条款门 | 227 文件（77%） | 收紧后 113 文件；按条 1713 | 判据从「出现字样」收紧为「有节号 §<数字>」，故文件数近乎腰斩 |
| 病史信号 | 442 条 / 148 文件 | 门内 314 条；三半径：测试自身 7 / 文件头 291（38 文件）/ 整文件 1218（79 文件） | 0.6-2 的 148 文件比本次整文件半径 79 文件还宽，其词表/范围更大；本次严格按卡面四信号 |
| 特征化强信号 | 1176（34%）/ 强 631（18%） | 不测 | 特征化不是三扇门之一，混进退役结论会越权 |
| 弱断言 | 473（14%） | 见 TSV「断言摘要」列 | 计数单位不同：0.6-2 按「只断言 is_ok/is_some」的模式统计；本次按词法计数并区分强/弱/委托断言 |

## 结论摘要

1. **条款门此前显著虚高**：旧判据下 2403 条判「条款/留」，抽样实测错引率 50%（95% 区间
   35%–65%）——「引用」与「引用支撑断言」是两件事，旧口径把前者当成了后者。
2. **收紧后真正有条款支撑的约 1202 条**（区间 846–1557），其余要么待核补票，要么本来就该按
   病史门/证明门判定。
3. **退役面与条款错引面无关**：候选 136 条与条款无关（无断言/弱断言）；首批已退役 21 条，候选降为 115。
4. **病史门是最大的合法留来源**（314 条）：它证明「钉住一次真实事故」比「抄一句 RFC」更靠得住。

## 首批退役（已执行，2026-11-06）

**第一批：`src/frontend/core/types/eval/tests/` 的 26 条「无断言」测试里退役 21 条、保留 5 条**
（单一目录簇；判据 = coding-rules 第七部分的**三扇门皆不过**：条款门无、病史门无、体内无任何断言）。
逐条判定与「同格覆盖」由 0.6-11a 判定表给出（删 21 / 留 5），本页不复述理由，只登记执行结果。

- **已退役 21 条**（`normalizer.rs` 2 条 + `reducer.rs` 19 条）：`test_normalizer_with_config`、
  `test_normalizer_evaluator`、`test_type_computer_register_alias`、`test_type_computer_compute_primitive`、
  `test_type_computer_set_context`、`test_type_computer_compute_type_var`、`test_type_computer_compute_list`、
  `test_type_computer_compute_tuple`、`test_type_computer_compute_fn`、`test_reducer_reduce_with_limit`、
  `test_unifier_different_types`、`test_unifier_substitution_after_unify`、`test_unifier_unify_option_same`、
  `test_unifier_unify_result_same`、`test_unifier_unify_dict_same`、`test_unifier_unify_set_same`、
  `test_unifier_unify_range_same`、`test_unifier_unify_arc_same`、`test_unifier_unify_weak_same`、
  `test_unifier_unify_enum_same`、`test_unifier_unify_struct_same`。
- **保留 5 条**（判定表指认不出替代覆盖，按硬要求留）：`test_normalizer_context`、
  `test_type_computer_new_and_with_config`、`test_type_computer_compute_type_ref`、
  `test_reducer_reduce_type_ref_with_alias_chain`、`test_reducer_reduce_type_ref_self_referential`。
- **回退**：整批是一个提交（本页与 `test-gate-census.tsv` 同提交），`git revert <sha>` 即整体回退。

## 复跑与自检

```bash
python scripts/dev/test-gate-census.py            # 重算并覆写 TSV（两次输出逐字节相同）
python scripts/dev/test-gate-census.py --check    # CI/提交前：TSV 是否与当前树一致
python scripts/dev/test-gate-census.py --selftest # 最小正/反例自检（不扫仓库）
python scripts/dev/test-gate-census-audit.py --sample  # 复现 40 条抽样（种子 20261106）
python scripts/dev/test-gate-census-audit.py --check   # 审计数据文件是否覆盖全部样本
```

生成器只做词法判定，边界如实说明：

- **委托断言只按名字识别**（`assert*/check*` 助手函数）。若某测试的断言藏在名字不带这两类前缀的
  助手里，它会被误判成「无断言」——首批 26 条（退役 21 / 留 5）均已逐条人工复核过函数体。
- **证明门不判**：任何「两门皆不过」的测试只落「待判」或「退役候选」，生成器不宣布「该删」。
- **不跨文件去重**：完全重复的测试不在本表识别（0.6-2 已实测仅 1%，不构成退役面）。
- **审计只覆盖 40 条**：审计结论只用于给这 40 条定性，并给出 2403 条池的**比例**；不得把某条的
  判定外推到同目录其它测试。
