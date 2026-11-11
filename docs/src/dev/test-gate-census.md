---
title: '测试三扇门普查（0.6 大清理前置）'
description: '按 coding-rules 第七部分的三扇门，对全仓测试逐条机械分类，并给出退役候选清单与各批次退役记录'
---

# 测试三扇门普查（0.6 大清理前置）

WBS 0.6「大清理」的前置材料：把全仓 `#[test]` 按 [coding-rules 第七部分](./coding-rules.md) 的
**三扇门（条款 / 病史 / 证明）** 机械分类，产出一份可重跑的逐条判定表；条款门的抽样审计
（40 条）曾量出「引了条款但撑不住断言」的比例并据此**收紧判据**，该审计已于 0.6-19 废止并删除。
**本页与生成器本身只读不改测试**；测试的退役按批次执行（每批一个提交、可整体 `git revert`），
执行结果登记在下方各批次退役小节与 `09a`。

生成物：

| 产物 | 作用 |
| --- | --- |
| `docs/src/dev/test-gate-census.tsv` | 逐条判定表（2055 行 + 表头），与本页同目录 |
| `scripts/dev/test-gate-census.py` | 生成器（可重跑、可 `--check`、可 `--selftest`） |

## 口径

### 扫描面

`src/`、`tests/`、`benches/`、`tools/` 下所有 `.rs` 的 `#[test]`：

| 面 | 条数 |
| --- | --- |
| 根包 lib（`src/`，含各模块 `tests/` 子目录） | 1581 |
| `tests/integration` | 335 |
| `tests/formatter_integration.rs` | 52 |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs` | 2 / 1 |
| `tools/` | 75 |
| `benches/` | 9 |
| **合计** | **2055**（285 个文件） |

### 三扇门的机器判据

| 门 | 机器判据 | 命中含义 |
| --- | --- | --- |
| 条款门（0.6-10 收紧） | 文件头前 25 行出现**可核条目**：节号形态 `§<数字>`（含 `RFC-<数字> §<数字>`、`SPEC §<数字>`）；`src/**/tests/` 下的文件连父 `mod.rs` 一起看 | 该测试对得上一条可指名的条款 |
| 病史门 | 测试函数体、函数文档注释，或**文件头**（`//! 回归背景：issue #226` 这类）点名真实故障：`issue #N` / 回归 / 缺陷 / 事故 | 该测试钉的是一次已发生的事故 |
| 证明门 | **机器判不了**（「换一个正确的替代实现，它会不会红」需要人判）。故前两门都不过的一律落「待判」，不自动判成「均不通过」；唯一例外是体内**一条断言都没有**（含委托给 `assert*/check*` 助手的也没有、`match` + `panic!` 形态的也没有、也没有 `#[should_panic]`） | 只能给出「待判」或「无断言」 |

收紧的理由：0.6-9 的条款门只要求文件头**出现 `§`/`RFC`/`SPEC` 字样**，于是 2403 条被判「留」。
0.6-10 的 40 条分层抽样实测：**一半的引用撑不住断言**（错引率 50%，95% 区间 35%–65%）——
这条实测是**放弃「凭一句引用就留」的直接原因**。收紧后只认节号形态，且
**有引用仍不等于引用撑得住**——条款引用一律只到「待核」。
（该抽样审计已于 0.6-19 废止并删除：审计池随测试退役缩小后原样本 38 条已不存在，
`--check` 恒报「缺 38 条、多 38 条」；结论保留，审计产物不再维护。）

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
| 留 | 只给两种：病史门命中、证明门（spec-shaped 目录 / 现状锚名字） |
| 待核 | 引了条款（有节号，或只引整篇 RFC 无节号）但引用未经人工确认——要人核过才能留 |
| 退役候选 | 两门皆不过且断言面只有弱断言（或无断言）——候选不是判决，证明门仍需人工复核 |
| 待判 | 两门皆不过但有强断言，机器不给结论 |

## 分布（收紧后）

| 门 | 条数 |
| --- | --- |
| 条款（有可核条目） | 808 |
| 病史 | 313 |
| 证明（spec-shaped 判据面 / 现状锚） | 424 |
| 待判（两门皆不过，证明门待人工） | 499 |
| 均不通过（体内无任何断言） | 11 |
| **合计** | **2055** |

| 处置 | 条数 | 对比 0.6-9 |
| --- | --- | --- |
| 留 | **737** | 2897（−2160） |
| 待核 | 1054 | 0（新增档位） |
| 待判 | 203 | 412（第二批退役后 −1，0.6-15 判据修正 +3，第五批退役 −211） |
| 退役候选 | 61 | 136（第一批退役 −21、第二批 −11、生成器修正 +1、第三批 −2、0.6-15 判据修正 −3、冗余扫描 −1、第四批 −1、第五批 −37） |
| **合计** | **2055** | — |

两点口径变化要说明：

1. **留的结构**：737 = 证明门 424（spec-shaped 判据面 390 + 现状锚 34）+ 病史门 313。
   原 2897 里那 2121 条「凭一句引用就留」的，现在落待核（0.6-19 废止抽样审计后，原先被
   判「审计确认真实支撑」而留的 9 条也一并落待核——条款引用不再有「审计确认」这一档）。
2. **病史门 47 → 313**：不是新增病史，而是**标签归位**——0.6-9 里「条款与病史同时命中」记作条款，
   收紧后条款不再自动给留，这类必须由病史门接住，故计数并入病史。留的总数因此不受影响
   （该批测试在 0.6-9 也是留）。

文件口径：285 个测试文件里，文件头**有可核节号**的是 **136 个**（0.6-9 的旧判据是 221 个）。

## 与 0.6-2 勘察的对账

0.6-2 的勘察数字是第一版对照基线。差异逐项列原因：

| 项 | 0.6-2 勘察 | 本次 | 差异原因 |
| --- | --- | --- | --- |
| 全仓 `#[test]` | 3443 | 2055 | 0.6-2 之后新增 2 条（E3005 与 E3022 的 check 口径回归测试），又按首批清单删除 21 条无断言测试、按第二批清单删除 11 条弱断言测试、按第三批清单删除 2 条、第四批 1 条、第五批 1353 条；净 −388 |
| 测试文件数 | 296 | 285 | 第五批整文件退场 11 个 |
| 条款门 | 227 文件（77%） | 收紧后 136 文件；按条 808 | 判据从「出现字样」收紧为「有节号 `§<数字>`」，故文件数近乎腰斩；第五批退役 902 条，0.6-19 废止抽样审计后另有 9 条从「留」落待核 |
| 病史信号 | 442 条 / 148 文件 | 门内 313 条；三半径：测试自身 7 / 文件头 291（38 文件）/ 整文件 1218（79 文件） | 0.6-2 的 148 文件比本次整文件半径 79 文件还宽，其词表/范围更大；本次严格按卡面四信号 |
| 特征化强信号 | 1176（34%）/ 强 631（18%） | 不测 | 特征化不是三扇门之一，混进退役结论会越权 |
| 弱断言 | 473（14%） | 见 TSV「断言摘要」列 | 计数单位不同：0.6-2 按「只断言 is_ok/is_some」的模式统计；本次按词法计数并区分强/弱/委托断言 |

## 结论摘要

1. **条款门此前显著虚高**：旧判据下 2403 条判「条款/留」，抽样实测错引率 50%（95% 区间
   35%–65%）——「引用」与「引用支撑断言」是两件事，旧口径把前者当成了后者。
   （该抽样审计已于 0.6-19 废止并删除，结论保留；删除理由见下节「抽样审计废止」。）
2. **收紧后真正有条款支撑的约 1202 条**（区间 846–1557），其余要么待核补票，要么本来就该按
   病史门/证明门判定。
3. **退役面与条款错引面无关**：候选 136 条与条款无关（无断言/弱断言）；首批已退役 21 条、第二批已退役 11 条，生成器修正 +1，第三批 −2，0.6-15 判据修正 −3，冗余扫描 −1，第四批 −1，第五批 −1353（按「同符号同结果类别只留最强一条」的机械规则批量退役），候选降为 61。第五批退役量按门分布：条款 902 · 待判 428 · 证明 23 · 病史 0 · 均不通过 0——**病史门与零断言线一条没少**。
4. **病史门是最大的合法留来源**（313 条）：它证明「钉住一次真实事故」比「抄一句 RFC」更靠得住。

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

## 第二批退役（已执行，2026-11-06）

**第二批：`src/frontend/core/parser/pratt/tests/nud.rs` 的 18 条「弱断言」测试里退役 11 条、保留 7 条**
（单一目录簇 = pratt 前缀解析模块；判据 = coding-rules 第七部分的**三扇门皆不过**：条款门无、病史门无，
且体内只有 `is_some()` / `is_none()` 一类弱断言——不锁定绑定力、也不锁定解析结果）。

本批与第一批的差别要说明白：这些测试**体内有断言**，不是空测试。退役理由是「三扇门皆不过 +
零判据增量」，不是「没断言」——判删的每一条都指认了**同模块的单元级替代覆盖测试名**。

- **已退役 11 条**：`test_prefix_info_int_literal`、`test_prefix_info_float_literal`、
  `test_prefix_info_string_literal`、`test_prefix_info_bool_literal`、`test_prefix_info_lparen`、
  `test_prefix_info_lbracket`、`test_prefix_info_if`、`test_prefix_info_while`、`test_prefix_info_for`、
  `test_prefix_info_ref`、`test_prefix_info_return`。
  每条的替代覆盖都走**同一条** `parse_expression → parse_prefix → prefix_info` 路径且断言更强
  （锁定 AST 形状而非只判 `is_some`）：同文件的 `test_parse_int_literal` / `test_parse_float_literal` /
  `test_parse_string_literal` / `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` /
  `test_parse_for_expr` / `test_parse_return_expr`，以及 `src/frontend/core/parser/tests/expressions.rs` 的
  `test_grouped_expr` / `test_tuple` / `test_list_literal` / `test_list_empty` / `test_ref_expr`。
- **保留 7 条**：`test_prefix_info_match`、`test_prefix_info_unsafe`、`test_prefix_info_spawn`、
  `test_prefix_info_at`、`test_prefix_info_break_removed`、`test_prefix_info_continue_removed`、
  `test_prefix_info_eof`——逐条理由见 `09a-wbs-history.md` 的「六、第二批测试退役」判定表。
  保留的理由分两类：**证明门成立**（前三条与 `test_prefix_info_at` / `test_prefix_info_eof` 是对应事实在
  **单元层**的唯一走查点，仓库内没有第二条单元测试观察它）、**病史门成立**
  （`break_removed` / `continue_removed` 点名 #314）。
- **回退**：整批是一个提交（本页与 `test-gate-census.tsv` 同提交），`git revert <sha>` 即整体回退。

## 生成器修正（0.6-13）

**`body_span` 遇字面量里的花括号会截断函数体 —— 已修。**

旧实现按花括号配平取函数体，不区分真代码与字符串 / 字符字面量、注释里的花括号。Rust 源码里
`tokenize("{")` 这类写法很常见，配平因此被拉歪，函数体被截断：`test_prefix_info_lbrace` 的函数体
曾被截到文件尾，「断言摘要」取到文件余下部分（`matchesx11 assertx24 …`），因而分箱成「待判」而非
「退役候选」——**分箱错**。

修法：新增 `mask_noncode`，先把字符串字面量 / 字符字面量 / 行注释 / 块注释（可嵌套）/ 原始字符串
（`r"…"` / `r#"…"#` / `br"…"` / `cr#"…"#`）的内容换成空格（换行原样保留，行号与行数不变），
再配平花括号。**掩码只用于找函数体边界**；断言统计与病史门仍读原文，判据本身没有放宽。
`--selftest` 新增 4 例（普通字符串里成对 / 不成对的花括号、原始字符串与块注释里的花括号、
字符字面量里的花括号），每例后面跟一条哨兵测试——旧实现在这 4 例上全部取错边界，新实现全对。

### 受同一个 bug 影响的测试：11 条

数法：逐文件逐测试比较**修复前后的函数体边界**（旧实现按字面量配平 vs 新实现按掩码后配平），
边界不同的即为受影响。全仓 3413 条测试中 **11 条**的边界被旧实现取错：

| 文件 | 条数 |
| --- | --- |
| `src/frontend/core/lexer/tests/fstring.rs` | 3 |
| `src/formatter/handlers/tests/expr.rs` | 2 |
| `src/driver/tests/manifest_error.rs` | 1 |
| `src/frontend/core/lexer/tests/literals.rs` | 1 |
| `src/frontend/core/parser/pratt/tests/nud.rs` | 1 |
| `src/frontend/core/parser/statements/tests/imports.rs` | 1 |
| `src/frontend/core/parser/statements/tests/types.rs` | 1 |
| `src/frontend/core/parser/tests/expressions.rs` | 1 |

### 修复前后分箱差异：2 条

其余 9 条里 **8 条只影响「断言摘要」那一列**（分箱未变），**1 条连摘要都没变**
（`test_fstring_close_brace_escape_preserved_raw`：旧实现少取了收尾的 `}` 行，那行不含断言词）。
8 条摘要变化逐条见 `09a-wbs-history.md` 的本节。

| 测试名 | 文件 | 旧箱 | 新箱 | 依据 |
| --- | --- | --- | --- | --- |
| `test_prefix_info_lbrace` | `src/frontend/core/parser/pratt/tests/nud.rs` | 待判 / 待判 | 待判 / **退役候选** | 真实函数体只有 `assert!(state.prefix_info().is_some())` 一条弱断言；文件头与函数体无 §节号、无 issue/回归/缺陷/事故 ⇒ 条款门与病史门皆不过，体内只有 `is_some()` 一类弱断言 ⇒ 退役候选（候选不是判决，证明门仍需人工复核） |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs` | 病史 / 留 | 条款 / **待核** | 旧实现把函数体截到文件尾，吞进来的文字里含「回归 / 缺陷 / 事故」而被误判成病史门；真实函数体内无此类字样 ⇒ 病史门不再成立；该文件头有可核条目（`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012）⇒ 条款门命中但未经抽样审计确认 ⇒ 待核 |

分布随之变动：条款 1713 → 1714 · 病史 314 → 313 · 留 776 → 775 · 待核 2121 → 2122 ·
待判 412 → 411 · 退役候选 104 → 105。**全仓 `#[test]` 数不变（3413）**——本轮只改判据的实现，
不改任何测试。

## 第三批退役（已执行，2026-11-07）

**第三批：`src/middle/passes/codegen/tests/` 的 5 条退役候选逐条判定，退役 2 条、保留 3 条。**
（单一目录簇 = `middle::passes::codegen` 一个被测模块；判据 = coding-rules 第七部分的**三扇门皆不过**。）

本批沿用上一批立的**判留口径**：单元级观察点只能被另一条**单元测试**替代；语料 / 集成属进程边界层，
不能替代单元观察点。

- **已退役 2 条**：
  - `test_execute_module_with_global_slot`（`global_slot.rs`）——函数体与同文件的
    `test_global_slot_store_then_load_roundtrip` **逐字相同**（去注释后仅函数名不同）：都是
    `run_fixture()` + `assert_slot_is_42(&interp)`。它的文档注释声称验「`execute_module` 入口路径返回
    非 Void」，但体内从未观察 `execute_module` 的返回值（`run_fixture` 用 `.expect(...)` 把它丢掉了）——
    声称的意图没有实现，函数体是纯重复 ⇒ 零判据增量。替代覆盖 = `test_global_slot_store_then_load_roundtrip`。
  - `test_scope_manager_basic`（`flow.rs`）——同文件的 `test_scope_nesting` 严格覆盖：正查
    （`lookup("a").is_some()`）、负查（`lookup("b").is_none()`）、`scope_level` 计数、插入的 `Symbol`
    形状（`symbol("a", Int(64), Local(0), false, 0)` 与本条内联的 `Symbol` 同形）。
    「从未插入的名字查不到」与「弹出后查不到」在 `SymbolScopeManager::lookup` 里是同一条路径
    （逐层 `scope.get(name)`，找不到即 `None`）⇒ 本条零增量。替代覆盖 = `test_scope_nesting`。
- **保留 3 条**：
  - `test_global_slot_store_then_load_roundtrip`——「不走序列化直连执行」这条路径的唯一观察点
    （序列化变体是另一条路径），且 `assert_slot_is_42` 断言了具体值。
  - `test_global_slot_survives_serialization_roundtrip`——序列化往返（`write_to` → `read_from` → 执行）
    的唯一观察点。
  - `test_register_overflow`（`operand.rs`）——`OperandResolver::to_reg` 的 `id > 255` 拒绝分支的
    **唯一**单元观察点（字节码层有各自独立的溢出检查，不覆盖本函数；全仓无第二条用例让 `to_reg` 越界）。
    `is_err()` 是有区分力的断言（守卫被去掉就会红），不是「只证明没 panic」。
- **回退**：整批是一个提交（本页与 `test-gate-census.tsv` 同提交），`git revert <sha>` 即整体回退。

### 本批查出的一处普查盲点（**0.6-15 已修**，见下节「生成器判据修正」）

普查的「委托断言」识别 `\b(assert|check)[A-Za-z_]*\s*\(` **匹配不到名字里带数字的助手**：
`[A-Za-z_]*` 不含数字，于是 `assert_slot_is_42(&interp)` 这种调用**不被识别为断言**——
本批 5 条候选里的 2 条因此被误分箱成「体内无任何断言」。

全仓 15 条「均不通过」逐条复核（把助手名放宽到含数字、并额外识别 `match` + `panic!` 形态）后：
**4 条其实有断言**——3 条是上述数字助手（`assert_slot_is_42`），1 条是
`src/backends/interpreter/tests/weak.rs` 的 `test_weak_from_arc`（`match` + `panic!` 形态；
test-specification 规则 6.1 要求写成 `assert!(matches!(...))`，但它确实在断言）。其余 11 条确实一条断言都没有。

**这 4 条若按机器的「均不通过」直接带走，会误删有判据的测试**——本批逐条人判后才没有删；
生成器侧的修法见下节「生成器判据修正（0.6-15）」。

## 生成器判据修正（0.6-15）

**两处判据盲点已修：助手名含数字、`match` + `panic!` 形态。**
本轮**不删任何测试**（全仓 `#[test]` 3411 条不变），只改生成器对「体内有没有断言」的识别。

### 修了什么

1. **委托断言的名字部分放开数字**：`\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`。
   旧式样匹配不到 `assert_slot_is_42(&interp)` 这类含数字的助手名，于是含它的测试被当成
   「体内一条断言都没有」。
2. **新增 `match` + `panic!` 形态**：`match <expr> { … panic!(…) … }`——断言写在某个臂里，
   词法上看不到 `assert!` 宏（test-specification 规则 6.1 要求写成 `assert!(matches!(...))`，
   但存量代码有此形态）。识别方式：`match` 关键字后到第一个 `{`（扫描表达式）开出的块，
   块边界用**花括号配平**确定，块内出现 `panic!(` 才算。**判定在掩码行上做**——
   字符串与注释里的花括号不参与配平，注释里的 `panic!` 也不会被当成断言。

### 分箱差异：3 条（门与处置变了）

| 测试名 | 文件 | 旧箱 | 新箱 | 依据 |
| --- | --- | --- | --- | --- |
| `test_weak_from_arc` | src/backends/interpreter/tests/weak.rs | 均不通过 / 退役候选 | 待判 / 待判 | 体内是 `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`——断言变体，形态原先未被识别；现认作断言，机器不判其覆盖力 |
| `test_global_slot_store_then_load_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | 均不通过 / 退役候选 | 待判 / 待判 | 断言委托给 `assert_slot_is_42(&interp)`——助手名含数字，旧式样匹配不到；现认作断言 |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | 均不通过 / 退役候选 | 待判 / 待判 | 同上（序列化往返那条） |

另有 **77 条只改了「断言摘要」那一列**（门与处置不变）：多数是摘要里多出 `match_panicxN`
（体内有 `match` + `panic!` 形态），4 条多出 `helperxN`（`src/driver/tests/manifest_error.rs` 3 条、
`src/package/tests/cache.rs` 1 条——助手名含数字）。

**放宽只改「均不通过」一档**：该档是机器的**确信判决**（体内一条断言都没有），判准是硬要求；
「退役候选」只是待人工复核的池子，池子大小不是判决。故弱断言池等其余分箱一律按原判据走。
结果：门 均不通过 14 → 11 · 待判 923 → 926；处置 退役候选 103 → 100 · 待判 411 → 414。

### 现存「均不通过」11 条：逐条复核结论

全仓 11 条「均不通过」逐条读原文复核后，**11 条确实一条断言都没有**（含 `assert*` / `check*` 助手、
`match` + `panic!` 形态都没有，也没有 `#[should_panic]`）：

| 测试名 | 文件 | 体内实际做什么 | 复核结论 |
| --- | --- | --- | --- |
| `test_run_bytecode_file_roundtrip` | src/backends/interpreter/tests/bytecode_load.rs | 5 处 `.expect(...)`，只保证不崩 | 真无断言（只证明没崩） |
| `test_normalizer_context` | src/frontend/core/types/eval/tests/normalizer.rs | 取 `context()` 句柄即丢弃 | 真无断言 |
| `test_type_computer_new_and_with_config` | src/frontend/core/types/eval/tests/reducer.rs | 构造两个 `TypeComputer` 取 context 即丢弃 | 真无断言 |
| `test_type_computer_compute_type_ref` | src/frontend/core/types/eval/tests/reducer.rs | 算 `compute(TypeRef)` 后丢弃 | 真无断言 |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs | 链式别名 `reduce` 后丢弃 | 真无断言 |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs | 自指别名 `reduce` 后丢弃 | 真无断言 |
| `test_document_symbol_items_invalid_code` | src/lsp/handlers/tests/completion.rs | 对非法内容调 `document_symbol_items` 后丢弃 | 真无断言（只证明没 panic） |
| `test_handle_formatting` | src/lsp/handlers/tests/formatting.rs | 调 `handle_formatting` 后丢弃 edits | 真无断言（只证明没 panic） |
| `test_handle_range_formatting` | src/lsp/handlers/tests/formatting.rs | 调 `handle_range_formatting` 后丢弃 edits | 真无断言（只证明没 panic） |
| `test_cli_modules_available_on_native` | src/tests/feature_gate.rs | 构造 lsp / repl 即丢弃（wasm32 cfg 门内） | 真无断言 |
| `test_z3_modules_available_on_native` | src/tests/feature_gate.rs | 构造 `Z3Backend`；`let _: Z3_context = std::ptr::null_mut();` 只是类型标注 | 真无断言 |

其中 5 条（`test_normalizer_context` 与 reducer.rs 的 4 条）是 0.6-11a 判定表里**指认不出替代覆盖**
而判留的；本轮复核结论与之一致。

### 逆向抽查：放宽有没有过头

从 88 条「新认作断言」的条目里按固定种子（20261107）抽 5 条读原文，确认它们真的在断言：

| 抽查条目 | 文件 | 原文里的断言 |
| --- | --- | --- |
| `test_substitute_through_assoc_type` | src/frontend/core/types/tests/substitute.rs | `match` 命中的臂里两条 `assert_eq!`，未命中则 `panic!("Expected AssocType")` |
| `test_unlisted_file_rejected` | src/package/tests/yxpkg.rs | 命中臂里 `assert!(msg.contains("src/evil.yx"))`，未命中 `panic!("expected InvalidPackage, got …")` |
| `test_assignment_constraint_dynamic` | src/frontend/core/sema/inference/tests/assignment.rs | 命中臂里 `assert_eq!` 比对约束类型，未命中 `panic!("不是 Dynamic 变体…")` |
| `test_delta_add_token` | src/lsp/handlers/tests/semantic_tokens.rs | 命中臂里 `assert!(!delta.edits.is_empty())`，未命中 `panic!("Expected TokensDelta variant")` |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs | 命中臂里三条 `assert_eq!` / `assert!` 校验错误码与消息，未命中 `panic!("不是 Disproved…")` |

5 条全都在断言具体值或具体变体（把实现换成另一条正确路径会让它们红）——**放宽没有过头**。
反向对照也进了 `--selftest`：`match` 里没有 `panic!`、`panic!` 只出现在注释里，两种都**不得**被认作断言。

### 判据有牙（发育性证明）

把生成器退回旧实现（`HELPER_RE` 不含数字 + 关掉 `match` + `panic!` 识别），新加的 `--selftest` 用例
**全红**、退出码 1；新实现全过。

### 回退

生成器与普查表同提交，`git revert <sha>` 即整体恢复。

## 第四批退役（已执行，2026-11-07）

**第四批：目录簇 `src/backends/interpreter/`（解释器后端的测试目录）的 10 条「退役候选」里退役 1 条、保留 9 条**
（单一目录簇，三个文件一次读完再判：`tests/bytecode_load.rs` 4 条 · `tests/weak.rs` 2 条 ·
`executor/tests/debug.rs` 4 条）。判据 = coding-rules 第七部分的**三扇门**；判留口径 =
「单元级观察点只能被另一条单元测试替代，语料/集成属进程边界层」；**判删必须指认具体替代覆盖测试名，
指认不出或读不懂即判留**。

- **已退役 1 条**：`test_capture_stack_includes_current_frame_during_step`
  （`src/backends/interpreter/executor/tests/debug.rs`）——同文件的
  `test_capture_stack_during_execution` 走同一条路径（`make_module` + `load_module_for_stepping`
  加一次 step 再取 `capture_stack`），且断言**严格更强**：它断言栈非空且 `stack[0].function_name == "main"`
  （钉住栈顶位置），被列者只断言 `stack.iter().any(|f| f.function_name == "main")`（任意位置即可）
  ——前者为真必蕴含后者 ⇒ 零判据增量。
- **保留 9 条**：`test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` · `test_run_file_with_yxbc_magic_uses_bytecode_load_path` ·
  `test_weak_upgrade_some` · `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`——逐条读原文后，它们各自都是
  某条性质的**唯一单元观察点**（load 的魔数/版本两条分支互不相同、YXBC 路由层、Weak 的 Some/None
  两侧、无帧 step、step_out 弹栈、run 到结束栈空），指认不出能替代它们的单元测试 ⇒ 按硬要求留。
  逐条依据见 `docs/src/dev/architecture/09a-wbs-history.md` 的第十一节（判定表放实录，不进本页）。
- **数字**：全仓 `#[test]` 3409 → 3408；判删 1 = `git diff` 计 `-#[test]` 1 行、`+#[test]` 0 行；
  支配者 `test_capture_stack_during_execution` 单独跑通过。普查：退役候选 99 → 98 · 待判 925 → 924 ·
  条款 1713 · 病史 313 · 证明 447 · 均不通过 11 · 待核 2121 · 留 775。
- **回退**：整批是一个提交（本页与 `test-gate-census.tsv` 同提交），`git revert <sha>` 即整体回退。

## 第五批退役（已执行，2026-11-07）

**第五批：按「同符号同结果类别只留最强一条」的机械规则，批量退役 1353 条单元测试**
（只动 `src/**/tests/**` 的单元测试；E2E 面 `tests/yaoxiang/**`、`tests/integration/**`、
`tests/formatter_integration.rs`、`tests/yx_runner*.rs`、`tests/project-corpus/**` 一条未动）。

- **规则（机械执行，不逐条论证）**：同一个被测符号 + 同一个断言结果类别，只留断言最强的一条，其余删除。
  - 被测符号 = 测试体内调用的仓库内符号（含沿本文件助手函数传递一层）；触达多个符号时按**主符号**
    （第一个仓库内调用）归类。符号按仓库内路径限定，不按裸名跨模块合并。
  - 结果类别 = 具体值 / 某个错误码（E 码）/ 某个变体匹配 / 仅判 ok / 仅判 err / 仅 should_panic。
  - 强度 = `assert_eq!`/`assert_ne!` > `assert!`/`matches!`（或 match + `panic!`） >
    `is_ok`/`is_some`/`is_err` > `should_panic` > 无；同组等强时保留文件中最先出现的一条。
  - 一个测试同时断言多个类别（无法判定主类别）时**保留**，不猜。
- **三条例外（一条没少）**：① 名字或注释点名 `issue #` / 回归 / 缺陷 / 事故的（病史门）；
  ② 某个符号只有一条测试的（删了该符号就没有单元观察点）；③ `#[should_panic]` 且是该符号唯一
  should_panic 用例的。病史门 313 条与「均不通过」11 条**退役 0 条**。
- **数字**：全仓 `#[test]` 3408 → 2055；判删 1353 = `git diff` 计 `-#[test]` 1353 行、`+#[test]` 0 行。
  普查（按门，0.6-19 废止抽样审计后的现值）：条款 808 · 病史 313 · 证明 424 · 待判 499 ·
  均不通过 11。普查（按处置，现值）：留 737 · 待核 1054 · 待判 203 · 退役候选 61。
  本批的退役量按门分布：条款 902 · 待判 428 · 证明 23（0.6-19 把 9 条「审计确认真实支撑」的
  条目改判为待核，故按门/按处置的现值与第五批收口当时不同）。
- **随删清理**：被删测试独占的助手函数、随之失效的 import 与结构体字段一并清掉
  （`cargo clippy --all --all-features --all-targets -- -D warnings` 0 告警）。
- **两处需留档的越界删除**：普查原判「留」而本批规则仍删的有 29 条——现状锚 23 条（名字含
  unchanged/anchor/guard/preserve/baseline，其中多为「guard」「preserves」这类**领域词**误命中，
  如 `test_while_guard_scope_balanced`）与审计确认条款真实支撑 6 条。本批按卡面三条例外的封闭清单
  执行，未额外保护这两类；要恢复只需按本页的保留/删除对照表回填。
- **抽样审计的失效与废止**：40 条抽样按固定种子从测试池抽取，第五批退役后原样本里 38 条已不存在，
  审计的 `--check` 因此恒报「缺 38 条、多 38 条」。该抽样审计已于 0.6-19 废止并删除
  （脚本、数据文件与生成器对它的依赖一并清掉），见下节「抽样审计废止」。
- **回退**：整批是一个提交（本页与 `test-gate-census.tsv` 同提交），`git revert <sha>` 即整体回退。

## 抽样审计废止（0.6-19）

**判据**：抽样审计（0.6-10）测的是「条款引用的真实支撑率」，而 0.6-18 之后保留规则已经不看引用
（改成「同符号同结果类别只留最强一条」，测试池从 3408 降到 2055）。原 40 条样本里 38 条已不存在，
审计的 `--check` 恒报「缺 38 条、多 38 条」——留着只会持续报警。故**废止并删除**。

**删了什么**：`scripts/dev/` 下的抽样审计脚本（按固定种子复现 40 条样本的那个）与其 JSON
数据文件（40 条逐条判定与依据）。

**生成器同步去依赖**：生成器原先读该 JSON，把「审计确认真实支撑」判成「条款门 + 留」；
该分支已删除——**条款引用不再有「审计确认」这一档**，一律落「条款 / 待核」。

**分布变化**（全仓 `#[test]` 2055 条**一条未动**，TSV 已重生成）：留 746 → 737（−9：原被审计
确认为真实支撑的 9 条落回待核）；待核 1045 → 1054；按门：条款 811 → 808、待判 496 → 499
（其余不变：病史 313 · 证明 424 · 均不通过 11）。

**回退**：整批是一个提交（本页与 `test-gate-census.tsv`、生成器同提交），`git revert <sha>` 即整体回退。

## 复跑与自检

```bash
python scripts/dev/test-gate-census.py            # 重算并覆写 TSV（两次输出逐字节相同）
python scripts/dev/test-gate-census.py --check    # CI/提交前：TSV 是否与当前树一致
python scripts/dev/test-gate-census.py --selftest # 最小正/反例自检（不扫仓库）
```

生成器只做词法判定，边界如实说明：

- **委托断言只按名字识别**（`assert*/check*` 助手函数）。若某测试的断言藏在名字不带这两类前缀的
  助手里，它会被误判成「无断言」——首批 26 条（退役 21 / 留 5）与第二批 18 条（退役 11 / 留 7）
  均已逐条人工复核过函数体。
- **函数体边界（0.6-13 已修）**：生成器取函数体时按花括号配平，字符串 / 字符字面量与注释里的花括号
  会把配平拉歪，函数体因此被截断——`test_prefix_info_lbrace`（体内有 `tokenize("{")`）曾被截到文件尾，
  「断言摘要」取到文件余下部分，分箱成「待判」而非「退役候选」。现已改为先掩码非代码区域
  （字符串 / 字符字面量 / 行注释 / 块注释 / 原始字符串）再配平，见下节「生成器修正」。
- **「委托断言」识别（0.6-15 已修）**：旧式样 `\b(assert|check)[A-Za-z_]*\s*\(` 的 `[A-Za-z_]*` 不含数字，
  故 `assert_slot_is_42(&interp)` 这类调用不被识别为断言——含这种调用的测试会被误判成「体内无任何断言」。
  实测当时全仓 15 条「均不通过」里有 4 条其实有断言（3 条数字助手 + 1 条 `match` + `panic!` 形态）。
  现已把名字部分放宽为 `[A-Za-z0-9_]*` 并新增 `match` + `panic!` 形态识别，见下节「生成器判据修正」。
- **证明门不判**：任何「两门皆不过」的测试只落「待判」或「退役候选」，生成器不宣布「该删」。
- **不跨文件去重**：完全重复的测试不在本表识别（0.6-2 已实测仅 1%，不构成退役面）。
- **条款引用一律待核**：文件头有可核节号只说明「对得上一条可指名的条款」，不等于这条引用撑得住断言；
  机器不判「撑不撑得住」，故条款命中也只到「待核」，要人核过才能留。
