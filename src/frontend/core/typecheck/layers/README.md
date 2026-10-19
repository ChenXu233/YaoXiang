# typecheck/layers/ — 证明层

**职责**：RFC-027 证明检查的各层模块。每个被调用的层提供一个「证明入口」
（返回 `ProofResult` 或 `Vec<ProofResult>`），由 `checker.rs::check_module_impl`
在类型检查的各步之间按固定顺序调用。

**这里没有调度器**：`mod.rs` 只做模块声明，不声明层序；本目录内部互不引用
（本目录内 `layers::` 与 `super::<层>` 的引用零命中；对外依赖见下「规则」）。
所谓「层序」＝ 调用点在 `check_module_impl` 里的先后。

**定位以符号为锚**：下文用「函数名 / 驱动点」指路；括号里的行号按现树取、随改动平移，
仅供快速跳读。下称 `checker.rs` = `src/frontend/core/typecheck/checker.rs`；
其余路径均为仓库根相对路径。

**实际调用顺序**：

| 序 | 调用点（符号锚；括号内为现树行号） | 证明入口 | 返回 |
|----|-----------------------------------|----------|------|
| 1 | `check_module_impl` 的精化走查起点 `collect_refined_binding_checks`（`:1240`） | `predicate::check_predicate`（三个站点 `:5229` / `:5382` / `:5503`，分别由 `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` 调用） | `ProofResult` |
| 2 | `check_module_impl` 的终止检查块（`:1263`–`:1284`）：闭包内 `TerminationChecker::new()`（`:1272`）→ `check_module` → `count_unjudged_obligations`（`:1278`） | `termination::TerminationChecker` | `Vec<ProofResult>` |
| 3 | `check_module_impl` 的所有权检查块（`:1296`–`:1340`）：`check_ownership`（`:1303`） | `ownership::check_ownership` | `(Vec<ProofResult>, ReleasePlan, HashSet<String>)` |
| 4 | `check_module_impl` 尾部 `env.solver().solve()`（`:1343`） | 类型约束求解器 `TypeConstraintSolver`——**不在 `layers/`** | — |

**「下层失败上层不跑」不成立（本目录无此实现）**：第 1/2/3 步的结果都被逐一消费、
不阻断后续层——终止结果循环（`:1285` 起）与所有权结果循环（`:1310` 起）
只把失败转成诊断（`self.add_error`），`check_module_impl`（`:1153` 起）全程无
`return` / `break` / `continue`。跨阶段的跳过另有实现：driver 的 `run` 循环按拓扑判跳过
（`src/driver/mod.rs` 的 `first_failed_upstream` / `record_skipped`，现树 `:102` / `:103`），
那是 `Stage` 粒度，且 `Skipped` 记录**不外发诊断**（同文件模块头，现树 `:29`）
——`layers/` 里没有任何 `Skipped` 机制。

**`equivalence.rs` 的现状**：`check_module_impl` 对它**零调用**。
`check_type_equivalence`（`equivalence.rs:216`）全仓库只被测试引用
（`src/frontend/core/typecheck/tests/rfc027_phase1_integration.rs:13`、
`src/frontend/core/typecheck/tests/rfc027_phase4_e2e.rs:15`）；同文件的 `is_subtype`
是生产路径上的共享助手（`src/frontend/core/typecheck/inference/assignment.rs:17`），
`structurally_equal` 现仅测试引用（`layers/tests/equivalence.rs:5`）。
它当下的身份是**共享类型比较工具 + 一个仅测试使用的证明入口**，不是被顺序调用的层。

**跨模块共享助手**：第 1 步的精化走查会调用 `termination` 的纯函数——`negate_guard`
（`checker.rs:4963`）、`substitute_const_expr`（`checker.rs:5092` / `:5352` / `:5490`）；
即 `termination` 同时充当工具库。

**规则**：
- 每个证明入口返回 `ProofResult` 或 `Vec<ProofResult>`
- 层与层之间零引用——不存在「某层直接调另一层内部函数」这回事；对外的 `use` 面
  主要是 `proof/`（`ProofResult` / `ProofContext` / SMT 后端）、`types/`（`MonoType` /
  `ConstExpr` / `Evaluator`）与 `crate::util::*`（`Span` / `PlanId` /
  `ConsumptionLedger`）；`ownership.rs` 另依赖
  `typecheck::inference::call_ownership::CallOwnershipTable`（完整清单见各文件 `use` 块）
- 新增层要两步落地：`mod.rs` 里 `pub mod` 声明 + 在 `check_module_impl`
  （或驱动它的走查）里补调用点；缺第二步的层永不执行，`check_type_equivalence` 即此类

**注意**：各模块头注释里的 `Layer N` 编号（`equivalence.rs` = 0、`ownership.rs` = 1、
`predicate.rs` = 3）是各文件自己的标签，与调用顺序不对应（实际顺序是 1 `predicate` →
2 `termination` → 3 `ownership`）；本目录内没有「编号 → 调用时机」的映射。
