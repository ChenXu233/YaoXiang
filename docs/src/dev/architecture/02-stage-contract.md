# 编译阶段契约与义务账本

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档。四层模型、验收判据分级与执行阶段顺序见 RFC-039 正文;各附属文档的定位见 [本目录索引](index.md)。

## 定位与范围

本文是 RFC-039 的 **L1 编排层**施工图。它处理一个问题：**「这次编译跑了哪些阶段、哪些没跑、为什么」当前分散在六条路径、十个入口函数里各自手工接线，没有任何一个地方能回答这个问题。**

当前状态可以概括为一句话：

> **代码自己声明的不变量，被架构而非逻辑破坏了。**

`src/frontend/core/typecheck/checker.rs:5165` 的注释原文写着「Unproven → 编译错误，无降级、无 silent pass……不得让 silent pass 复活」。但这条不变量只在**单文件路径**上成立：形如 `x: Sorted(3)` 的证明函数约束，在多文件 `yaoxiang run`、`yaoxiang check`、LSP 三条路径下**静默通过**，证明函数从不执行，且不产生任何诊断。

### 覆盖

- `Stage` 枚举（可穷举、非运行期注册）与 `StageScope`；
- `Obligations` 义务账本与 `assert_drained()` 结算；
- 统一 `Driver`（单一 `dispatch`）与十个入口函数的逐个改造方式；
- 阶段失败语义（`Continue` / `Abort` / `Warn`）与诊断聚合模式（`FailFast` / `CollectAll`）；
- 上述改动触及的全部文件与行号、兼容性影响、实施顺序。

### 不覆盖

- 四层模型、依赖方向规范、防反弹门禁 → `01-routing.md`
- 等价性判据（C1-C6 分级、三层判据）→ `07-equivalence-oracle.md`
- 三套平行类型表示的收敛 → `03-type-unification.md`；SSA 化 → `04-ssa.md`；前端范式 → `05-frontend-paradigm.md`
- 死代码与 wasm 分支可达性清理 → `06-cleanup-inventory.md`
- P1-P10 的全局执行顺序与 G1-G10 验收门禁 → RFC-039 正文

### 与 RFC-039 的分工

RFC-039 给出**为什么**重构与**按什么顺序**做；本文给出 L1 的**具体形态**、**逐文件改动清单**与**本文内部的实施阶段**（对应 RFC-039 全局序列的 P3「修正确性漏洞」与 P4「阶段契约与统一 Driver」）。凡本文与 RFC-039 冲突处，以 RFC-039 为准。

等价性判据按 [等价性判据文档](07-equivalence-oracle.md) 的 **C2（编排变更）**类别执行：各入口的诊断集相同 + 语料行为相同。

## 现状

> 本节全部为**已核实事实**，每条带文件路径 + 行号。行号基于 `9e02e4db`。

### 阶段边界是唯一「一处写错、全仓静默」的结构性缺陷

它与其他两类缺陷（模块边界、测试接线）性质不同：

| 缺陷 | 典型表现 | 是否有信号 |
| --- | --- | --- |
| 词法/语法错误 | 源码写错 | 有诊断 |
| 类型不匹配 | 类型写错 | 有诊断 |
| 阶段漏接 | 某个阶段根本没被调用 | **无任何信号** |
| 义务未消费 | 字段填了但没人读 | **无任何信号** |

后两类的共同特征是：**失败不产生错误**。因此它们不可能靠「更仔细地写代码」或「更严格地 review」解决——代码 review 只能看到写了什么，看不到**没写什么**。这正是 RFC-039 诊断的根因：「这个项目把『设计』当作文档约定，而不当作可执行的约束」。

### 本仓库已有的强制机制

同一个仓库里已经有成熟的强制机制，只是没有推广到阶段层：

- `src/util/diagnostic/codes/` 的 **145 个错误码**（137 个 E + 8 个 W）由 `build.rs:19-55` 通过 `tools/code-tables` 做**构建期硬门禁**，逐条比对 RFC-013 码表，任一不一致直接 `panic!` 拒绝编译。
- `src/package/`（**76 文件 / 13012 行**，其中 `tests/` 目录下 6227 行、占 47.9%）测试密度高，每个模块的测试子树都被声明接线——这是全仓测试接线最完整的一块，可作为阶段层门禁的形式参照。

**设计能力是够的。缺的是「在编排层也放一个同级别的门禁」。**

（行数口径：`(Get-Content).Count`，见 `06-cleanup-inventory.md` 的「行数口径」一节。）

### 六条编译路径、十个入口函数

| # | 路径 | 入口函数 | 位置 | 走哪条编译 |
| --- | --- | --- | --- | --- |
| 1 | **单文件管线** | `Pipeline::run` | `src/frontend/pipeline.rs:141-227` | 5 阶段直调 |
| 2 | **单文件包装** | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106` | 包装 `Pipeline` |
| 3a | **多文件编译** | `orchestrator::compile_project` | `src/frontend/module/orchestrator.rs:99-237` | 逐文件 `check_module` + IR + 链接 |
| 3b | **多文件检查** | `orchestrator::check_project` | `src/frontend/module/orchestrator.rs:273-399` | 逐文件 typecheck，收集全部诊断 |
| 3c | **LSP 项目内** | `orchestrator::check_source_in_project` | `src/frontend/module/orchestrator.rs:450` | `check_module_collect_all`（`493`） |
| 3d | **嵌入 std** | `orchestrator::compile_embedded_module` | `src/frontend/module/orchestrator.rs:1374` | `check_module`（`1386`）+ 独立 IR |
| 4a | **CLI 单文件** | `lib::run_file` | `src/lib.rs:140-147` | → `run_with_source_name` → 路径 1 |
| 4b | **CLI 多文件** | `lib::run_project` | `src/lib.rs:154-167` | → `compile_project`（路径 3a） |
| 5 | **LSP / `yaoxiang check`** | `lsp::run_diagnostics` | `src/lsp/handlers/diagnostics.rs:146` | 项目内走 3c，否则手工 lex→parse |
|  |  | `check_files_with_diagnostics` | `src/util/diagnostic/mod.rs:565` | → `check_project`（`590-591`） |
| 6 | **wasm playground** | `run_code` / `test_compile` | `wasm/src/lib.rs:42` / `30` | `Compiler::compile_with_source` → 路径 1 |

单文件路径的 5 个阶段（`pipeline.rs`）：

| 阶段 | 调用点 | 实现 |
| --- | --- | --- |
| lexing | `149` | `run_lexing`（`230-239`） |
| parsing | `161` | `run_parsing` |
| typecheck | `173` | `run_typecheck`（`268-285`） |
| **proof_execution** | **`187-203`** | `run_proof_execution`（`306-311`）— RFC-027 Phase 2.5 |
| ir_generation | `205` | `run_ir_generation`，**单态化内嵌其中**（`381-389`） |

### 11 处阶段覆盖不一致

下表逐格给出证据。**空格不代表该入口「不做这件事」，而是「该入口里没有任何代码做这件事」**——这正是缺陷的本质。

| # | 阶段 / 行为 | `pipeline`（单文件） | `compile_project` | `check_project` | `check_source_in_project`（LSP） | `compile_embedded_module` |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **proof_execution** | **有** `187-203` | **无** | **无** | **无** | **无** |
| 2 | 死代码分析 | 有 `275-278`（`config.dead_code.enabled` 门控） | **无** | 有 `356-377`（角色感知，无配置门控） | **无** | **无** |
| 3 | W1006 本地模块遮蔽 | **无** | **无** | 有 `336-348` | **无** | **无** |
| 4 | W1003 未使用导入 | 有（`277` 收 `type_result.warnings`） | **收集但从不输出** | 有 `350` | **无** | **无** |
| 5 | W1001/W1002 死代码族 | 有（同 2） | **无** | 有（同 2） | **无** | **无** |
| 6 | **单态化** | 有 `381-389`（`config.mono.enabled` 门控） | **无** | 不适用 | 不适用 | **无** |
| 7 | typecheck 分支 | `check_module` | `check_module`（`124`），**首错即返回**（`132`） | `check_module_collect_all`（经 `315` → `493`），收集全部 | `check_module_collect_all`（`493`） | `check_module`（`1386`） |
| 8 | 文件发现 | 不适用 | `discover`（`101`，丢弃 `used_by`/`shadow_events`） | `discover_with_used`（`275`） | `discover`（`454`） | 不适用 |
| 9 | 角色上下文（`surfaces`/`test_rules`/`roles::classify`） | **无** | **无** | 有（`321-328`） | **无** | **无** |
| 10 | 全局槽位分配 | 不适用 | 有 `allocate_global_slots`（`147`） | **无** | **无** | **无** |
| 11 | IR 生成 + 限定名重写 + 链接 | 有（`205`） | 有（`152-236`） | **无** | **无** | 有（独立 ModuleIR 后合并） |

> **复核注记（WBS 3.4.3，2026-10-07 逐格核对）**：本表是 `9e02e4db` 基线的诊断快照，留存不改。P3 落地后的现状差异：
>
> - **第 1 行已修复**：proof_execution 五入口共用 `frontend/proof_execution.rs` 同一实现（pipeline + orchestrator 四入口），唯一消费者缺陷不存。
> - **第 2/4/5 行现状不变**（多文件 `run` 仍不输出 W1003、不跑死代码族）→ 归 WBS 3.4.6（前置 4.2.1）。
> - **第 6 行**（单态化单文件独占）→ 归 WBS 3.4.8（前置 4.1.3，潜在风险未证缺陷）。
> - **第 7 行** check_module / check_module_collect_all 双入口 → 归 4.2.7（Aggregation 参数驱动）。
> - 代码侧引用的「#434 裁决」此前 docs 零登记——已补登为 RFC-039 **D57**。
> - **嵌入 std 纳入不对称（表外新事实，2026-10-09 补登）**：单文件路径由 `merge_embedded_std_ir` 无条件注入 std.list（for 循环脱糖必需，#117 硬切换），多文件 `discover` 此前只认显式 `use`——项目内 for 循环编译通过、运行期 E6006（探针实证）。已修复并勾销 WBS 4.10.2；同族「多文件缺单文件一步」的 parse 硬中止怪癖挂号 WBS 4.10.1——**亦已修复**（4.2.2 后随即落地，Check 路径降级为逐文件收集；本审计方法看「阶段覆盖/字段消费」维度，这条是编译单元成员差异，属漏网维度）。
>
> - **嵌入 std 注册表面覆盖（4.2.5 施工发现，已修）**：Registry 臂对嵌入
>   std 单元用 `extract_module_info` 重复收获注册，会顶掉 `with_std()`
>   已按「native + yx 表面合并」注册的 native 半面（`result.is_err` 等
>   丢失 → std.test 误报 E1043）。本审计的第三漏网维度：「同一模块键的
>   两个注册来源」的合并语义——单文件路径的注册表只经 `with_std()`
>   一次成形，多文件/Check 的 Registry 臂逐单元 insert 才有覆盖面。

**两处需要精确表述，否则会写错：**

- **第 4/5 行的准确结论是**：`yaoxiang run` 在**多文件**路径下（`lib.rs:154 run_project` → `compile_project`）永不报 W1001/W1002/W1003；在**单文件**路径下（`lib.rs:140 run_file` → `pipeline.rs:275-278`）是报的。原因在 `compile_project:138` —— `type_results.push(result)` 存下了完整 `TypeCheckResult`（含 `warnings`），但该函数**全程无任何 `result.warnings` 读取点**，`result` 唯一的下游用途是 `generate_ir_with_context`（`154-156`）。
- **第 11 行的入口 `main` 判据也不同源**：`check_project:385-395` 用 `surfaces.bins`（manifest 声明面）；`compile_project` 用 `is_bin_role`（`250-252`，仅判「有无 manifest」）。两个函数对「什么文件必须定义 `main`」给出不同答案。

### 正确性漏洞：证明义务被静默丢弃（完整证据链）

**这是本文的核心。以下八步全部可复核。**

**第 1 步 —— 义务的产生点唯一。**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` 是全仓库**唯一**构造非空 `proof_calls` 的位置：

```rust
// predicate.rs:232-239
return ProofResult::Unproven {
    reason: UnprovenReason::ProofFunctionRequired,
    proof_calls: vec![ProofFunctionCall {
        func_name: func.clone(),
        args: const_args,
    }],
    budget: budget_report,
};
```

语义即「此精化约束的谓词实参是编译期字面量，必须实际执行该证明函数才能判定」。

**第 2 步 —— 消费点在 checker 内部，但只发不消费。**
`src/frontend/core/typecheck/checker.rs` 有**三处同构分支**处理 `ProofResult::Unproven`：

| 分支 | 位置 | 行为 |
| --- | --- | --- |
| 形参精化校验 | `5164` `if calls.is_empty()` → 推硬错误（`5170-5177`）；`5179` `ctx.proof_calls.extend(calls.clone())` | **不发任何诊断** |
| 调用点实参校验 | `5306` → 推硬错误（`5308-5316`）；`5318` `ctx.proof_calls.extend(calls)` | **不发任何诊断** |
| 返回位义务 | `5420` → 推硬错误（`5442-5446`）；`5448` `ctx.proof_calls.extend(calls)` | **不发任何诊断** |

`checker.rs:5165-5169` 的注释原文：

> `RFC-027 §4/§9：Unproven → 编译错误，无降级、无 silent pass。` …… `本分支因此成为真防线而非前瞻兑底：不得让 silent pass 复活。`

即：作者明知「不产诊断」是缺陷，把它明确归类为**已知的、无处可去的中间态**，设计上假定「一定有人会来读 `proof_calls`」。

**第 3 步 —— 该字段确实被填进了结果。**
`checker.rs:1234-1235` 声明局部 `proof_calls` 并收集，`checker.rs:1439` 以 `proof_calls, // Phase 2.5 预登记证明函数义务` 写入 `TypeCheckResult`。字段定义在 `types.rs:29`。

**第 4 步 —— 全仓库只有一个读取点。**
`TypeCheckResult.proof_calls`（`types.rs:29`）在生产代码中**只有 `src/frontend/pipeline.rs:187` 一处被读取**（`189` 传参）：

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

（全仓 `proof_calls` 标识符的其他命中分三类，都不是本字段的消费者：`checker.rs:1234/4564/5494` 是生产侧收集；`verdict.rs:61` 是 `ProofResult` 的同名字段；`tests/rfc027_*.rs` 读的是 `ProofResult`。）

**第 5 步 —— orchestrator 的四个入口全部不经过那一层。**
`src/frontend/module/orchestrator.rs` 的 `compile_project`（`99`）、`check_project`（`273`）、`check_source_in_project`（`450`）、`compile_embedded_module`（`1374`）**均不调用 `pipeline.rs`**，直接调 `TypeChecker::check_module`（`124` / `493` / `1386`）。因此它们连 `pipeline.rs:187` 那个唯一读取点都碰不到。

**第 6 步 —— 后果：标准库自身的精化义务也走丢弃路径。**
`compile_embedded_module`（`1374`，`check_module` 调用在 `1386`）负责编译嵌入 std。这意味着**嵌入 std 模块自身的证明义务同样不被执行**。

**第 7 步 —— 后果：约束静默通过。**
`y: Sorted(3) = 5`（`Sorted: (x: Int) -> Type = { ... }`）在多文件 / check / LSP 三条路径下**编译通过、运行通过、无任何诊断**。证明函数从未被调用。

**第 8 步（补充核实）—— 层内还有第二个静默丢弃点。**
`checker.rs:1303-1314` 处理 ownership 层结果时：

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**所有权检查层的 `Unproven` 被空 match 臂吞掉，无诊断、无记账。** 这与 `5164` 分支「不得让 silent pass 复活」的承诺直接矛盾，且**独立于 orchestrator 问题**——即使入口层完全修好，这一处仍会静默。（处理时机：应与义务账本同一批处理，因为它与义务账本是同一类问题。）

### 为什么测试没有抓到

**`tests/integration/multifile.rs`（726 行 / 27 个 `#[test]`）对 `Sorted` / `proof` / `refin` 三个关键词的命中数为 0。** 多文件路径的证明义务零覆盖。

RFC-027 的三个单测（`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`、`rfc027_refined_transparency.rs`、`rfc027_return_refinement.rs`）确实断言了 `proof_calls` 非空——例如 `rfc027_return_refinement.rs:350-359` 断言 `result.proof_calls` 中存在 `SumUpTo` 调用。但它们**全部经由 `check_source` → `checker.check_module(&module)`**（`rfc027_return_refinement.rs:45/75`、`rfc027_refined_transparency.rs:27/57`），**恰好停在 `pipeline.rs` 之前**。

测试文件自己的文档注释已经写明了这一点。`rfc027_refined_transparency.rs:13-15`：

> `本文件只断言 check_module 能看到的东西。证明调用（E4018）由 pipeline.rs 在 check_module 之后执行，故精化违反的用例在 .yx 层`

**这正是问题的形状：测试验证「义务被填充了」，而 bug 是「消费端没读」。** 一个只测生产端、不测消费端的测试，对本类缺陷天然免疫。

### 更强的发现：等价性判据的主语料是单文件路径

[等价性判据文档](07-equivalence-oracle.md) 把 `tests/yaoxiang/` 293 个 `.yx` 语料的端到端差分作为**第三层判据**、作为 C2 阶段的主要验收手段。但实测：

- `tests/` 目录下**没有任何 `yaoxiang.toml`**（全目录 glob 零命中）。
- 因此 `check_files_with_diagnostics`（`diagnostic/mod.rs:565`）对每个语料文件都命中 `standalone` 分支（`614-616`）→ `check_single_file`（`623-661`）→ `Compiler::compile_with_source`（`635`）→ `Pipeline::run` → **proof_execution 执行**。
- 换言之，**293 个语料文件全部走单文件路径，全部覆盖 proof_execution，全部不覆盖多文件路径。**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` 正是这一点的实证。该文件标注 `// expect: compile-error E4018`（第 15 行），其头部注释第 7 行写着「状态: ❌ 应被编译期拒绝」——**它能通过，正因为它走的是唯一执行证明函数的那条路**。

**结论：第三层判据需要补充多文件语料层，否则它无法充当本文的验收工具。** 见「实施要点」S1。

### 义务字段：16 个「产出即契约」的字段，零机制保证

`src/frontend/core/typecheck/types.rs:16-70` 的 `TypeCheckResult` 中，有 **16 个字段**的文档注释明确写着「由 X 阶段产出 → Y 阶段消费」，即它们本质上是**跨阶段义务**：

| 字段 | 定义行 | 产出方 → 消费方 |
| --- | --- | --- |
| `proof_calls` | `29` | typecheck → **proof_execution** |
| `release_plan` | `31` | ownership → IR 生成（`ir_gen.rs:341` 读、`:1942` 用） |
| `escaped_refs` | `33` | ownership → IR 生成 |
| `instantiation_requests` | `35` | typecheck → 单态化（`pipeline.rs:381`） |
| `existential_coercions` | `37` | typecheck → IR 生成 |
| `implementation_proofs` | `39` | typecheck → IR 生成 |
| `interface_impl_registry` | `42` | typecheck → 运算符查询 / 约束求解 / LSP |
| `sum_types` | `44` | typecheck → IR 生成 |
| `sum_type_param_names` | `47` | typecheck → IR 生成 |
| `variant_ctor_calls` | `49` | typecheck → IR 生成（span 键控） |
| `operator_dispatches` | `51` | typecheck → IR 生成（span 键控） |
| `method_overload_ir_names` | `53` | typecheck → IR 生成 |
| `overload_resolutions` | `56` | typecheck → IR 生成（span 键控） |
| `try_expr_impls` | `59` | typecheck → IR 生成（span 键控） |
| `match_scrutinee_types` | `63` | typecheck → IR 生成（span 键控） |
| `module_namespaces` | `66` | typecheck → IR 生成 |

**这些字段全部以 `Span` 或名字表为键跨层传递。** 等价性判据文档已指出 span 键控契约的失配会「静默失效，无任何错误」（`ReleasePlan` 是典型：两侧 span 计算方式任何一处不一致 → `Drop` 指令静默消失）。

`release_plan` 为什么存活而 `proof_calls` 死了？差别是**结构性的，不是偶然的**：

- `release_plan` 的消费者 `ir_gen.rs:341`（`release_plan: type_result.release_plan.drops.clone()`）位于 `generate_ir_with_context` 内部，而 `compile_project:154-156` **恰好把完整的 `&TypeCheckResult` 传给了它**——消费者在下游模块，而下游模块在所有入口的必经之路上。
- `proof_calls` 的消费者在 `pipeline.rs` **顶层**，属于**另一个入口实现**。orchestrator 的四个入口根本不经过 `pipeline.rs` 这一层。

**已核实事实**：目前全仓库**没有任何机制**保证这 16 个字段被消费。唯一"保护"是 `ReleasePlan` 恰好搭上了 IR 生成的便车。

### 证明层的四个附加契约缺陷

以下均来自证明层分析，独立于入口分叉问题，但都属于「阶段契约没有被强制」这一类。

**（a）层序声明与实际执行顺序矛盾，且 `equivalence` 层根本不在管线里。**

`src/frontend/core/typecheck/layers/README.md:5-11` 声明的层序：

| 层 | 文件 | 依赖 |
| --- | --- | --- |
| 0 | `equivalence.rs` | types/eval |
| 1 | `ownership.rs` | Layer 0 |
| 2 | `termination.rs` | Layer 0, 1 |
| 3 | `predicate.rs` | Layer 0, 1, 2 |

README 第 3 行称「按层序执行，下层失败上层不跑」。实际 `TypeChecker::check_module` 内的调用点：

| 实际顺序 | 调用点 | 对应声明层 |
| --- | --- | --- |
| 1 | `termination` — `checker.rs:1256-1270` | Layer 2 |
| 2 | `ownership` — `checker.rs:1296` | Layer 1 |
| 3 | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3 |
| — | `equivalence` — **`checker.rs` 中零调用** | Layer 0 |

即：`termination` 与 `ownership` 顺序与声明**相反**；声明的 Layer 0 `equivalence` **根本没有出现在 `check_module` 的阶段序列里**（它只被 `inference/assignment.rs:17` 作为 `is_subtype` 工具函数使用，与 `ProofResult` 无关）。同时**不存在任何 short-circuit**——`checker.rs:1271-1276` 把 termination 的错误逐条 `add_error` 后继续往下走，README 承诺的「下层失败上层不跑」不成立。

**（b）SMT 后端有三种获取策略、两种失败哲学。**

> **复核注记（2026-10-07）**：硬失败 panic 已由 P3 的 3.3.1 消除（SOLVER 槽 Option 化，
> 缺失按 `SMTResult::Unknown` 保守降级）；「静默跳过（不注入）」由 3.4.2 补 W1081 信号。
> 三形态的统一（单例化）按 **RFC-039 D58** 执行，真实修复位置 `proof/smt/backend.rs`
> （新增进程级共享单例 + `with_shared_solver` 闭包口），02 改动清单的
> `default_solver() → &'static` 表述以 D58 为准（`&'static` 裸引用不可行：
> `dyn Solver` 不 `Sync`）。

| 消费点 | 获取策略 | 求解器不可用时 |
| --- | --- | --- |
| `predicate.rs:34-36` | 全局 `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs`（经 `checker.rs:1265-1268` 注入） | 构造期注入 `with_solver_owned` | `None` → **静默跳过**（不注入） |
| `ownership.rs:627`（回边切断判定） | **每次调用** `default_solver()` | `None => return false`（`629`）→ 保守不切断 |

`predicate.rs:31-32` 的注释明确选择了硬失败：「初始化失败保持**硬失败**：软化会把『Z3 未安装』误诊为『约束超出内核能力』」。而 `backend.rs:60` 的契约文档说的是「`None`：后端不可用……**调用方应保守降级**」。**同一份代码里两种哲学并存，且硬失败的那条会 panic 而非返回错误。**

**（c）每次回边判定新建 Z3 context，缓存形同虚设。**
`ownership.rs:627` 在回边切断判定的热路径上调用 `default_solver()`。而 `src/frontend/core/typecheck/proof/smt/backend.rs:67-72`：

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**这是一个工厂函数，不是单例**——每次调用都 `Z3Backend::new()`，即新建一个 Z3 context。`Z3Backend` 的缓存字段（`proof/smt/z3_backend.rs:20` `cache: RefCell<HashMap<u64, SMTResult>>`）是**实例内**的，而 `z3_backend.rs:17` 的文档注释宣称「SMT 查询结果缓存在 `cache` 中」。跨调用不共享 ⇒ **该缓存在此调用模式下永不命中**。

（对照：`predicate.rs:34-36` 的 `LazyLock` 确实是单例。同一后端，两种生命周期策略。）

**（d）checker 静默降级 + 注释与实际不符。**
`checker.rs:1283-1287` 与 `1289-1293`：

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker 为 None → 空类型账本
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker 为 None → 空调用所有权表
```

`body_checker` 为 `None` 时，ownership 检查拿到**空类型账本和空调用表**并照常运行——所有权分析退化为「什么都不冲突」，**无任何警告**。（处理建议：应记为一条 warning 级诊断，或至少在义务账本中留痕。）

`checker.rs:1244` 的注释称终止检查「在类型检查之后、约束求解之前运行」。实际 `self.env.solver().solve()` 在 `checker.rs:1320`——**termination 在 `1256`、ownership 在 `1296` 之后，即注释所说的「之前」实际是「两层之后」**。

### wasm 现状：shim crate 承载，27 个文件的分支是活的

wasm 目标**已建成并在 CI 中构建**，`cdylib` 不在主 crate 而在独立的 shim crate。

| 事实 | 证据 |
| --- | --- |
| 主 crate 仅 `rlib` | `Cargo.toml:29-31` |
| **shim crate 提供 `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11`（`crate-type = ["cdylib"]`）、`wasm/Cargo.toml:18`（`wasm-bindgen = "0.2"`） |
| shim 依赖主 crate（rlib）为库 | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }` |
| 主 crate 有 wasm 目标依赖段 | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"`（`Cargo.toml:125-131` 为对应的非 wasm 段：tokio/ureq/tempfile） |
| shim 目录排除出主 workspace | `Cargo.toml:3` `exclude = ["wasm", ...]` |
| **CI 有 4 处 wasm 构建** | `_build-wasm.yml`（可复用工作流，`:75` `wasm-pack build --target web --out-name yaoxiang`）；调用方 `dist-release.yml:267-273`（产物 `yaoxiang-wasm`）、`docs-deploy.yml:23-56`（解包进 `docs/src/.vitepress/public/wasm`，即 playground）、`nightly.yml:110-114` |
| Z3 wasm 静态库由 Emscripten 预构建 | `_build-z3-wasm.yml:220`、`_build-wasm.yml:35-59`（按固定 URL 拉取 `libz3.a`，缺失时降级为 warning） |
| **27 个文件**出现 `wasm32` 字面量 | 其中 **25 个**带实际 `#[cfg(...)]` 属性，另 2 个（`frontend/module/roles.rs:9`、`std/fs.rs:10`）仅文档提及 |
| `orchestrator.rs` 20 处 | 12 处属性 + 8 处注释 |
| `lib.rs` 11 处 | 全为属性（`27/30/32/46/133/135/139/153/170/179/238`） |

**结论：这 27 个文件的 `#[cfg(target_arch = "wasm32")]` 分支是承重的。** 它们决定 `wasm/src/lib.rs`（73 行）在 wasm 目标下能调用主 crate 的哪些 API——`lib.rs:139/153/170` 把 `run_file` / `run_project` / `build_bytecode` 整体门控掉（这三条都需要 `std::fs`），而 shim 走的是另一条路。

**但这带来一个与本文直接相关的事实**：`wasm/src/lib.rs:48` 的 playground 入口调用 `compiler.compile_with_source(...)`——**单文件路径**。因此：

| 消费 `proof_calls` 的路径 | 是否执行证明函数 |
| --- | --- |
| 单文件 CLI（`lib.rs:140 run_file`） | **是** |
| wasm playground（`wasm/src/lib.rs:48`） | **是** |
| `build_bytecode`（`lib.rs:171`） | **是** |
| 多文件 `run`（`compile_project`） | **否** |
| `check`（`check_project`） | **否** |
| LSP 项目内（`check_source_in_project`） | **否** |

即：**`wasm/` 的存在使得"proof_execution 只有一个消费者"变成了"有三个"**，但这三个全在单文件路径一侧。`Driver` 的 `ProgramKind` 必须新增 `WasmPlayground` 变体（见「目标设计」3），否则统一 Driver 时会漏掉这条路径。

（清理范围——`orchestrator.rs` 的 12 个 wasm 属性中哪些在 playground 场景下不可达——归 `06-cleanup-inventory.md`。RFC-039 开放问题中「建 wasm 目标，还是删这些分支」一条**已经过期**：目标已建成。）

---

## 目标设计

### 1. 阶段模型：`Stage` 枚举（可穷举，非运行期注册）

**核心约束：`Stage` 是编译期可穷举的枚举，禁止运行期注册。** 理由是 Rust 的穷尽 `match` 能在编译期强制「新阶段必须被编排层处理」——这正是 opcode 表已经证明有效的机制（RFC-039 路由表 B）。

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // 供应商一致性核对          Project
    Discovery,          // 文件发现                  Project
    Parsing,            // 词法 + 语法               PerModule
    Registry,           // 模块注册表构建            Project
    RoleClassification, // 角色分类（Script/Bin/…）   Project
    Typecheck,          // 类型检查（含内嵌证明层）   PerModule
    DeadCodeAnalysis,   // 死代码族分析              Project
    ProofExecution,     // 证明函数编译期执行         PerModule
    GlobalSlotAlloc,    // 全局槽位分配              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // 单态化                    Project
    Linking,            // 跨模块链接 / IR 合并       Project
}

impl Stage {
    /// 全部阶段。新增变体时本数组与 `dispatch` 的穷尽 match 都会编译失败。
    pub const ALL: &'static [Stage] = &[ /* 全部 12 个，按拓扑序 */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**`StageScope` 的用途**：`PerModule`（逐编译单元跑一次）与 `Project`（项目级跑一次）这把「哪些阶段必须逐模块跑、哪些必须项目级跑一次」变成类型上的事实。`Project` 作用域的阶段在 `dispatch` 中被结构性地保证只跑一次——这消除了当前 `orchestrator.rs` 里「`allocate_global_slots` 到底该不该每个文件调一次」这类需要人推理的问题。

**阶段表按拓扑序而非字母序排列**，因为失败传播依赖顺序。

> **修订注记（P4 实施，2026-10-07）**：两处与初稿的偏差按实施证据修正——
>
> 1. `Monomorphization` 移到 `IrGeneration` **之后**：单态化消费 IR 产物
>    （`Monomorphizer::monomorphize(&ir, …)`，pipeline.rs），初稿次序与数据流矛盾。
> 2. Check 形态的阶段表按 `Stage::ALL` 拓扑序归一为 死代码 **先于** proof：
>    `check_project` 现状是 proof 先于死代码执行，两者无数据依赖，诊断集相同
>    （C2 集合语义），仅文件内诊断顺序归一。单文件路径（死代码内嵌 typecheck、
>    先于 proof）本就符合 ALL 序，逐字节验收不受影响。
> 3. `Parsing` 移到 `Registry`/`RoleClassification` **之前**（C3，2026-10-09
>    用户裁决）：签名收集（`extract_module_info`）与 `ast_has_main` 都消费
>    AST 产物——初稿次序会逼 Registry 臂「隐藏 parse」，让阶段表对数据流
>    撒谎；且多文件 parse 因此从 2 次降为 1 次。附带的现状如实登记：多文件
>    路径 parse 错误曾是硬中止（`build_registry_from` 的 `?` 传播，与
>    CollectAll 语义的张力登记为 WBS 4.10.1）——**4.10.1 已修复**
>    （2026-10-09 用户裁决 rust 式收集语义 + 方案 B）：Check 路径 parse
>    失败降级为逐文件诊断收集、带病文件退出编译单元（不进 registry，
>    导入方报 E5001）；MultiFile 路径保留硬中止（FailFast 下坏文件产不出
>    IR，属正确语义，钉板长期有效）。
> 4. Check 形态落地（4.2.2，2026-10-09 用户裁决）补两条登记：
>    a. 数据依赖边 14→16：`RoleClassification` 消费 Discovery 的 used_by
>       边集、`DeadCodeAnalysis` 消费 Discovery 的 W1006 遮蔽事件——初稿
>       漏列（这两个产物在 orchestrator 时代由 `discover_with_used` 产出、
>       被 `discover` 包装丢弃，数据流没进表格就丢了）；
>    b. 文件内诊断顺序归一扩展为本注记 #2 的完整形态：E3020 入口校验
>       移入 RoleClassification 臂（check 无 Linking 阶段，握着 surfaces
>       与 AST 的臂负责校验），文件内诊断顺序 = 阶段拓扑序（E3020 →
>       typecheck → W1006/W1003/死代码 → proof）；诊断集合不变（C2
>       集合语义），仅 stderr 条目顺序变化。

### 2. 义务账本：`Obligations` + `assert_drained()`

**设计目标**：把「字段被产出但无人消费」从不可检测变成编译期或测试期可断言的事实。RFC-039 已把这条列为「价值最高的一项」。

```rust
// src/driver/obligations.rs
pub struct Obligations {
    pub proof_calls:             Vec<ProofFunctionCall>,     // types.rs:29
    pub release_plan:            ReleasePlan,                // types.rs:31
    pub escaped_refs:            HashSet<String>,            // types.rs:33
    pub instantiation_requests:  Vec<InstantiationRequest>,  // types.rs:35
    pub existential_coercions:   Vec<ExistentialCoercion>,   // types.rs:37
    pub overload_resolutions:    Vec<(Span, String)>,        // types.rs:56
    pub try_expr_impls:          Vec<(Span, String)>,        // types.rs:59
    // …其余 span 键控字段
}

impl Obligations {
    /// 结算：在阶段表尾部调用一次。
    /// 每个未消费字段产出一条 W 级诊断（默认）或硬错误（strict 模式）。
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**结算语义（分三档）**：

| 情形 | 判据 | 处置 |
| --- | --- | --- |
| 义务被声明的消费者读取 | `Obligations` 中该字段被 `take()`/标记 consumed | 通过 |
| 义务非空但无消费者 | 字段非空且未 consumed | **产诊断**（`W` 级，默认；`strict` 模式升 `E` 级） |
| 义务为空 | 字段为空 | 通过（无需消费） |

**为什么先做 W 级而不是 E 级**：修复义务丢弃会**改变诊断集**。C2 判据要求「各入口的诊断集相同」，若一步到位升到 E 级，`yaoxiang check` 会突然多出大量此前被静默的 `Unproven` 诊断。分两步（先 W 观测、再升 E）能让每一步的判据都可用。详见「实施要点」S4。

**`assert_drained()` 的调用点唯一**：`Driver::run` 在阶段表尾部、产出 `CompilationResult` 之前。

**配套的静态门禁**：RFC-039 已提议 `scripts/ci/check-obligations.py`（字段出现 ≥2 次但无第三个文件读取 → 失败）。`assert_drained()` 是**运行期**门禁，该脚本是**静态**门禁，二者互补，都需要。

### 3. 统一 Driver：单一 `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project（lib.rs:154）
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project（LSP 项目内，结果过滤到目标文件）
    Embedded,          // compile_embedded_module（嵌入 std）
    WasmPlayground,    // wasm/src/lib.rs:48 —— 单文件路径的第四个调用方
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// 只能来自 `ProgramKind::stages()` 的 6 个预定义组合之一
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` 只能是上述 6 个预定义组合（每个 `ProgramKind` 一个），不接受调用方自由传数组**。这是防止 `Program` 抽象退化成"什么都能传的自由参数"的关键约束，由 `test_program_stage_coverage` 断言。

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // 拓扑决定是否跳过，不由阶段返回值决定
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // 必须产诊断，见下
                continue;
            }
            match stage {                            // 穷尽 match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 个臂，一个都不能少
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // 唯一结算点
        Ok(state.into_result())
    }
}
```

> **修订注记（P4 实施，2026-10-09，4.1.3 落地）**：与上方草图的四处实施偏差，语义等价、登记备查——
>
> 1. §4 的 `StageOutcome` 三态不由阶段臂返回，由 `State` 失败标记 + `Aggregation` 门控表达（Continue / Warn / Abort 语义不变，省去每臂样板返回）；
> 2. `Driver` 无 `config` 字段——配置唯一来源是 `Program.config`（§5「避免 Driver 持可变全局状态」的判定覆盖草图字段）；
> 3. `Skipped` 诊断在 4.1.3 仅记入 `DriverOutcome.skipped` 内部台账，不外发——S2「刻意不修任何 bug」的 zero-diff 判据要求；外发随 4.3 义务账本；
> 4. `proof_execution` 模块可见性放宽为 `pub(crate)`：driver 臂是唯一新调用方（L1→L2 为允许方向）；orchestrator 存量调用点随 4.2 迁走后归位再议。
> 5. 多文件形态下 ProofExecution 是独立阶段、在**全部** typecheck 之后执行（A1，2026-10-09 用户裁决）：`compile_project` 原把 proof 穿插在逐文件 typecheck 循环内（文件 N 的 proof 先于文件 N+1 的 typecheck）。单重失败两者逐字节相同；「文件1 proof 失败 + 文件2 typecheck 失败」的多重失败场景，首报从 proof 错误变为 typecheck 错误（测试钉板）。
> 6. `DriverOutcome` 按 ProgramKind 分通道携带产物：`result`（pipeline 契约）/ `module` + `failure`（orchestrator 契约，4.2.1）——各入口的外部错误契约（PipelineError / OrchestratorError）不属 Driver 可统一的类型面。
> 7. Check 通道落地（4.2.2）：`DriverOutcome.check_diagnostics` 携带逐文件
>    诊断（每个发现文件都有条目，干净文件为空 Vec——check_project 现状
>    契约）；Discovery 产物扩展为「文件集 + used_by 边集 + 遮蔽事件」
>    三元组入 State（MultiFile 形态无下游消费者，仅记录不外发）；check
>    路径每文件 parse 随 C3 拓扑从 2 次降为 1 次。
> 8. Lsp 形态落地（4.2.3，按文件来源分流裁决，2026-10-09 用户定夺）：
>    磁盘文件 parse 失败同 Check 的方案 B（收集 + 退出编译单元——两端
>    降级收敛）；被编辑缓冲区 parse 失败保留残缺 AST 继续 typecheck
>    （编辑器哲学——打字中间态不该让语义功能消失）。Discovery 臂为
>    Lsp 保留缓冲区源码覆盖磁盘陈旧内容；Typecheck 在 Lsp 下只检目标
>    文件（其余单元仅为 registry 供签名）；文件内诊断顺序归一原则延伸
>    至 LSP（typecheck 诊断 → W 码警告 → proof 错误）。旧行为里 LSP
>    对无关磁盘文件 parse 错误硬中止、handler 静默退回单文件路径的
>    怪癖随本步修复。
> 9. Embedded 形态落地（4.2.4）：`Program` 新增 `shared_registry`
>    字段——嵌入 std 模块是子编译，注册表是**输入**而非产物（EMBEDDED_
>    STAGES 无 Registry 阶段），#94 的 SymbolTable 共享契约由此从
>    「调用方记得传同一个 registry」显式化为程序声明。错误路径文本从
>    `<std.test> (embedded std)` 归一为单元虚拟路径 `<std/test>`（仅
>    编译器内部错误面，无测试钉住）。至此 orchestrator 四入口全部迁入
>    Driver。
> 10. standalone check 统一（4.2.5，裁决 A + IR 阶段裁决，2026-10-09）：
>     Check 形态对无项目根程序即单文件语义的忠实承载——警告面只覆盖
>     入口文件（邻旁文件只收错误，宁漏勿误）；相对 `use` 沿导入者目录
>     解析（rustc 单文件 mod 对齐）。CHECK 阶段表加 GlobalSlotAlloc +
>     IrGeneration：standalone 旧路径（pipeline 全链）本就跑 IR 生成，
>     E3019/E1014/E1015 等只在 ir_gen 产生（runner 门禁实证）。
>     IrGeneration 的 Check 形态纯检查不消费 IR；Script/Bin 分形以
>     module_key 的 None/Some 表达（E3023 的既有开关，ir_gen.rs:1660）。
>     Monomorphization 不入 CHECK（只产资源超限/内部错误，零语料依赖）。
>     **〔2026-10-10 经 3.4.8 实证撤回〕**——「零语料依赖」实为语料无
>     病态递归 fixture 的证据盲区；E3005 是该类程序唯一的编译期防线。
>     见注记 #14。
>     附带修复潜伏缺陷：Registry 臂对嵌入 std 单元的重复收获注册会顶掉
>     with_std() 已合并的 native 半面（E1043 误报）——嵌入单元现跳过
>     重复注册。
> 11. LSP 单文件兜底统一（4.2.6）：`run_diagnostics` 的手工
>     lex→parse→check_module_collect_all 序列删除，改走
>     `Program { kind: SingleFile, aggregation: CollectAll }`——LSP
>     与 CLI 同一 Driver。SingleFile+CollectAll 形态：Parsing 收全量
>     parse 错误、保留残缺 AST 继续 typecheck（编辑器哲学自 4.2.3
>     裁决延伸；不标阶段失败，否则拓扑跳过让 typecheck 永远跑不到）；
>     Typecheck 分派 `check_module_collect_all` 自由函数。LSP 单文件
>     路径自此补齐 proof（E4018）、W 码警告（W1001–W1005，测试钉板）
>     与 IR 级错误；词法失败改报真实诊断（旧合成「E0001 词法错误」
>     文本随序列删除消失——真实诊断带精确 span，无信息损失）。
> 12. `check_module` / `check_module_collect_all` 逐行核实（4.2.7，
>     C5 必做项）：两入口早已各自收敛——mod.rs 层是
>     `check_module_inner(ast, env, collect_all)` 单布尔、checker 层是
>     `check_module_impl(module, collect_all)`；唯一分叉是
>     `init_body_checker(collect_all)` → `set_collect_all_errors`，
>     pass-3 与 drain 两模式同跑、差异由 collected_errors 是否为空承载；
>     statements.rs 五个收集点（函数体/use/for/块/while）。核实中证实
>     重复诊断缺陷并修复：收集点把首错同时放进 collected_errors 与
>     Err 返回通道，pass-3 对 Err 的 add_error 造成同码同 span 同消息
>     ×2（嵌套可 ×3）；另有注解校验签名形参与整体注解双访
>     （E1003/E1103）、所有权层同点双发（E2014/E2018）两条独立机制。
>     修复落于模块结果边界：按 (code, span, message) 去重——诊断是
>     位置事实，重复不携带信息；63 个语料条目去除重复副本，run 列与
>     退出码零漂移（borrow_conflict_err 的「重复」是基线三元组格式
>     不含列号的伪影——两条 E2018 实是不同列的合法诊断）。
> 13. wasm playground 落地（4.2.8）：`run_code`/`test_compile` 经
>     `compile_playground` 构造 `Program { kind: WasmPlayground }` +
>     Driver——`Compiler` 包装层不再经手。错误文本逐字节对齐旧
>     `CompileError` Display 前缀（Parse error:/Type error:/Internal
>     error:），playground UI 零可见差异。WasmPlayground 与 SingleFile
>     共用 SINGLE_FILE_STAGES + FailFast（4.1 既有声明，本步起有真实
>     生产者）。
> 14. 多文件单态化与 CHECK 阶段表修正（3.4.8，2026-10-10 用户裁决，
>     **部分撤回注记 #10 的 mono 排除**）。实证推翻「mono 只产资源限制
>     噪音」：病态泛型递归（`f(x)=f([x])`）在无 mono 的多文件路径下
>     编译器进程爆栈（0xc00000fd——解释器在 Rust 层递归，无优雅运行时
>     错误），mono 深度闸是唯一编译期防线；standalone check 在 4.2.5 前
>     走 pipeline 全链本有 mono，排除裁决造成静默覆盖回归（「零语料
>     依赖」只证明语料无病态 fixture）。「类型擦除兜底」假设经 RFC-033
>     反射裁决作废——`^^List(Int)` 需要实例化的真实身份，单态化是
>     反射的地基而非可选优化。设计四点：①typecheck 对限定调用
>     （`lib.f(x)`，FieldAccess 形态）产 instantiation_request，
>     generic_id 用限定名，与 merged IR 同命名空间（现只认裸 Var，
>     expressions.rs:2455）；②`Stage::ALL` 拓扑改为 Linking 先于
>     Monomorphization，MULTI_FILE 阶段表尾部加 Monomorphization——
>     消费 merged_ir、聚合全单元请求（deferred 桶的 containing_fn
>     限定化），mono 从单模块 pass 升格为全程序 pass；单文件阶段表
>     不动（无 Linking，子序列性质保持，IR 快照零漂移）；③CHECK 加
>     Linking（纯合并——E3020 入口校验留 RoleClassification，防双报）
>     与 Monomorphization（纯检查不消费，同 IrGeneration 的 Check
>     形态）两阶段；④资源保护随全程序 BFS 覆盖跨单元互递归。
>     Monomorphizer 本体不变（消费 ModuleIR + 请求集的契约恰好是
>     merged IR 形态）。
> 15. 3.4.8 R1（①）落地（2026-10-10，a9438007）。按 #14 设计落地限定调用
>     实例化收集：`ExpressionInferrer` 注入模块限定键表（`use` 三臂登记——普通
>     / 别名 / 逐项别名且仅 SubModule 类导出），`callee_generic_name` 经
>     `SymbolTable::qualify` 拼接与 merged IR 同源的限定名；限定调用首路径
>     arity 判据收紧（参数表解不出时落第二路径按签名取实参，避免跨模块同名函数
>     错配误报 E3018）。实施中实证 mono 侧**三个不对称缺陷**并同轮修复：
>     (1) 删除键（泛型名集合）与改写键（containing_fn + 名 + span 三元组）
>     粒度失配——解不出实参的站点退化为符号请求进 deferred 桶，容器函数非泛型
>     时桶永不排水，原件却已被按名删除 → 悬空调用（`list_ops.yx:53` 运行期
>     E6006 实证）；修复为恢复闸 `restore_generics_with_uncovered_call_sites`，
>     以 `build_call_site_map` 为单一事实源判定覆盖，未覆盖站点的原件保留。
>     (2) 恢复段 `HashSet` 迭代序致函数表顺序不确定（t2 复审 F1）→ 改 `Vec`
>     并按名排序，产物字节级可复现。(3) 非改写面形态 TailCall / MakeClosure
>     原照样查映射键（注释却声称保守）→ `call_form_is_rewritten` 单点判定、
>     非改写形态一律计未覆盖。三者各配钉测试（修复前红态实证判别力）。语料
>     差分基线同提交再生恰一行（remove_at 帧补 `(int64)`）。**提交策略勘记**：
>     typecheck 收集与 mono 修复分拆两笔时，仅含前者的暂存树触发 `stdlib_docs`
>     对 HEAD 既有缺陷的红（`list.is_empty([])` 同族 E6006）——两半互为对方的
>     绿灯前提，故并为一笔原子提交。
> 16. 2026-10-10 五项裁决登记（R1 收口后）：(1) **D2 裁甲**——用户模块导出随
>     `Export.type_params` 携带 `generic_fn_type_params_snapshot()`，与 std 路径
>     （`yx_sources.rs:62/108`）同构：声明名成为限定调用单态化的权威来源，签名反推
>     降级为兜底（并入 R2 范围）。(2) **HKT 声明侧活、调用侧断裂**（推断传类型实参
>     E1002 / 显式给类型实参 E1010）登记为 WBS 3.5（3 个三级任务）。(3) **deferred
>     桶在非泛型容器下永不排水**并入 3.4.8 ② 的验收子项：桶必须排水或响亮报错，
>     不得静默不特化。(4) **闭包变量按名调用运行期 E6006** 登记为 WBS 3.6；语义
>     (i) 支持间接调用 / (ii) 编译期拒绝 待用户裁决。(5) **取舍判据固化**——方案
>     否决理由只能基于正确性与可读性，「改动面大 / 引入新机制 / 成本高」不构成否决
>     理由，落 `coding-rules.md` 第二部分与 HOWTO 自检表；需用户拍板的取舍按
>     `AGENTS.md`「请求用户裁决时（强制）」给全四件（情况+证据 / 逐项优缺点 /
>     明确推荐+代价 / 破坏性点名）。

**十个入口的改造方式（逐个指定函数）**：

| 入口 | 改造后 |
| --- | --- |
| `Pipeline::run`（`pipeline.rs:141-227`） | 删函数体，改为构造 `Program { kind: SingleFile, … }` 交 Driver |
| `Compiler::compile`（`compiler.rs:95`） | 不变（已是无状态的包装） |
| `compile_project`（`orchestrator.rs:99-237`） | 瘦身为 `Program { kind: MultiFile, aggregation: FailFast }` 构造器 |
| `check_project`（`orchestrator.rs:273-399`） | 瘦身为 `Program { kind: Check, aggregation: CollectAll }` 构造器 |
| `check_source_in_project`（`orchestrator.rs:450`） | 改为 Driver 的一次调用 + 结果过滤到目标文件 |
| `compile_embedded_module`（`orchestrator.rs:1374`） | 改为 `Program { kind: Embedded, units: [embedded] }` |
| `lib::run_file`（`lib.rs:140`） | 不变（转调 `run_project` 或 `SingleFile`） |
| `lib::run_project`（`lib.rs:154`） | 不变（转调 Driver） |
| `lsp::run_diagnostics`（`lsp/handlers/diagnostics.rs:146`） | 项目内改走 `Program { kind: Lsp, aggregation: CollectAll }`；单文件分支的**手工 lex→parse→… 序列删除**，改调 `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics`（`diagnostic/mod.rs:565`） | `standalone` 分支（`614-616` → `check_single_file` `623-661`）删除，统一走 `Program { kind: Check }` |

**`check_project` 与 `compile_project` 的重复度（约 40% 共享 / 60% 分叉）**。共享的是：vendor 一致性核对、文件发现、`build_registry_from`、`all_method_bindings`、解析循环、逐文件 checker 装配、结果处理。分叉的是上表 11 项中的第 2/3/4/5/7/8/9/10/11 项。**这 40% 的共享骨架正是 `Driver` 的价值区间**——分叉的 60% 全部是「阶段集合不同」或「聚合模式不同」，而这恰好是 `Program` 的两个字段能完整表达的。

### 4. 失败语义：`Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // 阶段成功，继续
    Abort,     // 阶段失败，终止本次编译
    Warn,      // 阶段有非阻断问题，继续
}
```

**关键设计判定：「上游失败导致本阶段跳过」不属于阶段返回值。**

理由：跳过是**拓扑**决定的，不是阶段自己决定的。如果让每个阶段自己返回 `Skipped`，则「我为什么没跑」的信息就分散在 12 个阶段里，无法集中审计。改为由 `dispatch` 在循环开头按依赖关系判定：

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` 必须产诊断。** 这条规则直接针对本文要修的漏洞——当前 `pipeline.rs:187` 的 `if !proof_calls.is_empty()` 就是一个静默的「跳过」，它不产任何诊断，只是因为义务恰好为空。规则确立后，任何"因为 X 没做所以没跑 Y"都必须可解释。

诊断文案需区分三种跳过原因：

| 原因 | 文案方向 |
| --- | --- |
| 上游 `Abort` 导致 | 「阶段 Y 因上游阶段 X 失败未执行」 |
| 前置义务为空导致 | 「阶段 Y 因无待处理义务未执行（正常）」——**此项 severity = Info，不计入警告数** |
| 条件不满足（如 `config.mono.enabled == false`） | 「阶段 Y 因配置未启用未执行」 |

第二类是关键：它让「正常跳过」与「异常跳过」在诊断流里可区分，且不污染 `yaoxiang check` 的 `warning_count`（`diagnostic/mod.rs:637-641` 依赖该计数的非阻断契约）。

### 5. 诊断聚合模式：`FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // 首个错误即中止（compile 路径）
    CollectAll,  // 收集全部诊断（check / LSP 路径）
}
```

| 模式 | 使用者 | 现状对应 |
| --- | --- | --- |
| `FailFast` | `compile_project`（`orchestrator.rs:125-137` 首错返回 `OrchestratorError::TypeCheck`）、`Pipeline::run`（`pipeline.rs:150/162/174/193` 四处早退） | 已有 |
| `CollectAll` | `check_project`（`orchestrator.rs:313-397` 不早退）、`check_source_in_project`（`493` `check_module_collect_all`）、`lsp::run_diagnostics`、`check_files_with_diagnostics` | 已有 |

**`Aggregation` 必须是 `Program` 的字段，而不是 Driver 的全局设置**——LSP 在同一进程内既服务项目内文件也服务单文件，而 CLI 的 `run` 与 `check` 是两次独立调用；把它们放在 Program 上可以避免 Driver 持有可变全局状态。

**注意当前两个 typecheck 分支的语义差异**：`check_module`（`orchestrator.rs:124`）与 `check_module_collect_all`（`493`）不只是"早退与否"的差别——它们是两个不同的 checker 入口。统一后应由 `Aggregation` 参数驱动同一个实现，而不是保留两个函数（**这是本文需要额外核实的一项，见「已知局限与风险」**）。

---

## 详细设计

### 类型系统影响

| 改动 | 类型层影响 |
| --- | --- |
| 新增 `driver/` 模块 | **无**。不引入新的类型表示，不触碰 `MonoType` / `PolyType` / `ir::Type` |
| `TypeCheckResult` → `Obligations` 迁移 | **字段类型不变**，只改归属。`release_plan` 仍为 `ownership::ReleasePlan`，`proof_calls` 仍为 `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations` | **全新类型**，与语言类型系统无关 |

**本文不引入第四套类型表示。** 三套平行类型表示（`ast::Type` / `MonoType` / `ir::Type`）的收敛是 `03-type-unification.md` 的范围（RFC-039 路由表 C）。

**义务类型的归属与分层**：迁入账本的是**结算责任**，不是类型的归属模块。`ReleasePlan` 仍定义在 `layers/ownership.rs`、`ProofFunctionCall` 仍在 `proof/`、其余字段类型各留原处；`driver/obligations.rs` 只持聚合容器与 `assert_drained()`。L3 的消费者（如 `ir_gen` 读 `release_plan`）继续经参数接收具体字段类型，**不得 `use crate::driver`**——这与下方 driver 的不可反向依赖红线一致。

**依赖方向**（RFC-039 四层模型）：`driver`（L1）依赖 `frontend`（L2）/ `middle`（L3）/ `backends`（L4）的接口；**L2/L3/L4 不得反向 `use crate::driver`**。`Driver` 出现在 `TypeChecker` 的 import 里即视为违规，由 RFC-039 提议的 `scripts/ci/check-module-boundary.py` 拦截。

### 运行时行为

| 场景 | 改造前 | 改造后 |
| --- | --- | --- |
| `yaoxiang run app.yx`（单文件） | 5 阶段 | 同 5 阶段 + `assert_drained()` 结算 |
| `yaoxiang run`（多文件） | 无 proof_execution、无 W1001/W1002/W1003 | proof_execution 已在 P3 止血接入；**警告面维持不新增**（决策 B1，2026-10-09：死代码族警告不进 compile 路径——池语义正被 4.9 判缺陷修正，归一警告面留待 4.9 之后再议；本行初稿「新增警告输出」作废） |
| `yaoxiang check`（多文件） | 无 proof_execution | 已在 P3 止血接入 |
| LSP（项目内） | 无 proof_execution | 已在 P3 止血接入 |
| Z3 未安装 + 单文件 | `predicate.rs:35` **panic** | 改为 `Abort` + E 级诊断（**这是行为变更，见兼容性**） |
| Z3 未安装 + 多文件 | 静默跳过 | 同上，统一 |

**唯一有意的行为破坏**是 `predicate.rs:35` 的 `.expect()` 改为返回诊断。这符合 RFC-027 §8「不绑定具体求解器」的定位，也是 `backend.rs:60` 已声明的契约（「调用方应保守降级」）——当前实现与自己的契约文档相反。

### 编译器改动清单

**新增文件（6 个）**

| 文件 | 内容 |
| --- | --- |
| `src/driver/mod.rs` | `Driver`、`run()`、穷尽 `dispatch` |
| `src/driver/stage.rs` | `Stage`（12 变体）、`Stage::ALL`、`StageScope` |
| `src/driver/program.rs` | `Program`、`ProgramKind`、`Aggregation` |
| `src/driver/obligations.rs` | `Obligations`、`assert_drained()` |
| `src/driver/unit.rs` | `Unit`（单文件 1 个 / 多文件 N 个） |
| `src/driver/diagnostics.rs` | 跨阶段诊断聚合、`Skipped` 诊断构造 |

**修改文件**

| 文件 | 行号范围 | 改动 |
| --- | --- | --- |
| `src/lib.rs` | `24-36`（模块声明块） | 新增 `pub mod driver;` |
| `src/lib.rs` | `140-147` | `run_file` 改为构造 `Program { kind: SingleFile }` |
| `src/lib.rs` | `154-167` | `run_project` 改为构造 `Program { kind: MultiFile }` |
| `src/frontend/pipeline.rs` | `141-227` | `Pipeline::run` 函数体替换为 `Program` 构造 + Driver 调用 |
| `src/frontend/pipeline.rs` | `275-278` | 死代码分析移出，改为 `Stage::DeadCodeAnalysis` 臂 |
| `src/frontend/pipeline.rs` | `306-311` | `run_proof_execution` **保留**，移入 `driver` 并作为 `Stage::ProofExecution` 的实现 |
| `src/frontend/pipeline.rs` | `381-389` | 单态化移出，改为 `Stage::Monomorphization` 臂 |
| `src/frontend/module/orchestrator.rs` | `99-237` | `compile_project` 瘦身为 Program 构造器 |
| `src/frontend/module/orchestrator.rs` | `273-399` | `check_project` 瘦身为 Program 构造器 |
| `src/frontend/module/orchestrator.rs` | `450` | `check_source_in_project` 改走 Driver |
| `src/frontend/module/orchestrator.rs` | `1374` | `compile_embedded_module` 改为 `Program { kind: Embedded }` |
| `src/frontend/module/orchestrator.rs` | `486-494` | `typecheck_with_registry_in` 的 `check_module` / `check_module_collect_all` 二选一改由 `Aggregation` 参数驱动 |
| `src/lsp/handlers/diagnostics.rs` | `146-227` | `run_diagnostics` 的手工阶段序列删除，改调 Driver |
| `src/util/diagnostic/mod.rs` | `565-619` | `check_files_with_diagnostics` 统一走 `Program { kind: Check }` |
| `src/util/diagnostic/mod.rs` | `621-661` | `check_single_file` **删除** |
| `wasm/src/lib.rs` | `30-36`、`42-51` | `test_compile` / `run_code` 改为构造 `Program { kind: WasmPlayground }`（当前经 `Compiler::compile_with_source` 隐式落到路径 1） |
| `src/frontend/core/typecheck/layers/predicate.rs` | `34-36` | `.expect()` 改为返回 `SMTResult::Unknown` + 诊断（**行为变更**） |
| `src/frontend/core/typecheck/checker.rs` | `1303-1314` | `ProofResult::Unproven { .. } => {}` 空臂改为产诊断（**修复第二个静默丢弃点**） |
| `src/frontend/core/typecheck/checker.rs` | `1283-1293` | `unwrap_or_default()` 降级路径加 warning 诊断 |
| `src/frontend/core/typecheck/checker.rs` | `1244` | 修正与 `1320` 不符的注释 |
| `src/frontend/core/typecheck/layers/README.md` | `1-18` | 层序表改为**实际**执行顺序；删除「下层失败上层不跑」（不存在 short-circuit） |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72` | **按 D58**：新增进程级共享单例（`LazyLock<Mutex<Option<Box<dyn Solver>>>>`）+ `with_shared_solver` 闭包访问口；`predicate.rs`/`checker.rs`/`ownership.rs` 三消费点改走共享口。`&'static` 裸引用形态不可行（`dyn Solver` 不 `Sync`），原文表述以 D58 为准 |

**删除**：`src/util/diagnostic/mod.rs:621-661` 的 `check_single_file`；`src/lsp/handlers/diagnostics.rs:161-227` 的手工 lex/parse 序列。

**不动**：`checker.rs:5164/5179/5306/5318/5420/5448`（六处 `Unproven` 分支逻辑本身正确——**问题在消费端不在生产端**，改生产端会掩盖架构缺陷）；`Cargo.toml`；任何 `#[cfg(target_arch = "wasm32")]` 分支（wasm 分支的可达性判定归 `06-cleanup-inventory.md`）。

### 向后兼容性

| 变更 | 兼容性 | 处置 |
| --- | --- | --- |
| 多文件路径新增 proof_execution | **破坏**：此前静默通过的约束现在会报 E4018 | `test_multifile_proof_obligation_not_dropped` **先写红**再修；分批 release，预期诊断集变化走 CHANGELOG |
| 多文件路径新增 W1001/W1002/W1003 | **破坏**：编译通过的程序开始产生警告 | 警告为非阻断（`warning_count` 单独计数，`diagnostic/mod.rs:637-641`），不改变退出码 |
| `assert_drained()` 首次运行产出 W 级诊断 | **破坏**：诊断集新增 | 先 W 后 E，S2/S4 分两步 |
| `predicate.rs` panic 改为诊断 | **改善**：不再崩溃 | 无破坏 |
| `build` / `dump_bytecode` 子命令 | **无影响** | 这两条路径不经 proof_execution |
| `TypeCheckResult` 字段迁移到 `Obligations` | **内部重构** | `pub` API 面若有外部依赖需同步；仓库内消费者已在改动清单中列全 |

---

## 实施要点

本文对应 RFC-039 全局阶段序列的 **P3（修正确性漏洞）** 与 **P4（阶段契约与统一 Driver）**。下列 S1-S5 是本文内部的实施顺序，**每一步的验收判据引用 [等价性判据文档](07-equivalence-oracle.md) 的 C2 类别**（编排变更：各入口的**诊断集**相同 + 语料行为相同）。

### S1：建立判据（先红）

**先决条件，不可跳过。** 漏洞判据必须先写成失败的。

| 交付 | 验收 |
| --- | --- |
| `test_multifile_proof_obligation_not_dropped` | **必须为红**。若意外为绿，说明本文的漏洞分析需要重新复核 |
| 多文件语料层：新建 `tests/yaoxiang-multifile/`（决议 D48——不混入单文件语料树；带 `yaoxiang.toml` 的项目夹具） | 单文件与多文件两版语料的诊断集可对比 |
| `test_obligations_drained` 骨架 | 标 `#[ignore]`，S4 转绿 |

**回滚点**：无代码改动，纯新增测试。

### S2：引入 `Stage` + `Program`，不改行为

| 交付 | 验收（C2） |
| --- | --- |
| `driver/stage.rs`、`driver/program.rs`、`driver/unit.rs` | `test_program_stage_coverage`：`Program::stages()` ⊆ `Stage::ALL` 且等于期望数组 |
| `pipeline.rs:141-227` 改为 Driver 调用 | **单文件路径诊断集与退出码逐字节相同** |
| 全语料（293 个 `.yx`）差分 | 诊断列表（按 `(code, file, line)` 排序）、退出码、stdout/stderr **全部 zero-diff** |

**这一阶段刻意不修任何 bug**——它只把现有行为搬进 Driver。验收标准是 C1/C2 级别的 zero-diff。

**回滚点**：`git revert` 单个 commit，`pipeline.rs` 恢复原状即可。

### S3：合并 orchestrator 四个入口 + 修复漏洞

| 交付 | 验收（C2） |
| --- | --- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` 全部改为 Program 构造器 | 四入口诊断集按 `Aggregation` 归一后相同 |
| `check_single_file`（`diagnostic/mod.rs:621-661`）删除 | `yaoxiang check` 在项目内外行为一致 |
| LSP 手工阶段序列删除 | LSP 与 CLI 对同一文件给出相同诊断集 |
| **`test_multifile_proof_obligation_not_dropped` 转绿** | 漏洞已修 |
| `checker.rs:1313` 空 match 臂修复 | `test_no_silent_pass_on_unproven`：`Unproven` 在任一模式下必产诊断 |
| `predicate.rs:34-36` `.expect()` 改为诊断 | 移除 Z3 环境下不再 panic |

**回滚点**：漏洞修复与结构合并**分两个 commit**。若结构合并出问题，可只回滚结构 commit、保留漏洞修复 commit——此时 `test_multifile_proof_obligation_not_dropped` 保持绿；反向（保留结构、回滚修复）会转红，是不可接受的中间态，禁止合入。

### S4：启用义务账本

| 交付 | 验收 |
| --- | --- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` 转绿 |
| `scripts/ci/check-obligations.py` | 静态门禁生效 |
| 义务诊断 W → E 升级 | 全语料差分后人工 review 每一处新增 E |

**回滚点**：`assert_drained()` 的严重级可由配置项控制，W/E 切换不需要改代码结构。

### S5：证明层与 wasm 收尾

| 交付 | 验收 |
| --- | --- |
| `layers/README.md` 层序改为实际顺序 | 文档与 `checker.rs` 调用点逐条对应 |
| `backend.rs` 进程级共享单例（D58；**硬前置：Unknown 不入缓存**——否则一次超时经进程级缓存跨编译固化、且污染 cargo test 线程间共享） | 生产路径 `default_solver()` 调用点归零（仅测试保留）；缓存命中率跨三消费点可观测（计数器已在 89576fafd 落地） |
| `checker.rs:1283-1293` 降级加 warning | 无 `body_checker` 时有诊断 |
| `orchestrator.rs` 12 个 wasm 属性的可达性判定 | 结论交 `06-cleanup-inventory.md`，本文只登记判定需求 |

**注意**：修正层序会改变诊断集、可能暴露大量此前被静默的 `Unproven`。**这一项应独立于 S1-S4 走**，不要与入口合并混在同一个 PR。

## 关键决策与理由

| 决策 | 决定 | 理由 |
| --- | --- | --- |
| **阶段模型** | `Stage` 为 12 变体的编译期可穷举枚举，**拒绝运行期注册** | 穷尽 `match` 是零成本的编译期强制：新增变体时 `dispatch` 编译不过。这与 opcode 表在 RFC-039 路由表 B 中已被验证的机制同构。`StageScope` 进一步把「逐模块还是项目级」变成类型事实，消除需要人推理的调用次数问题 |
| **义务机制** | `Obligations` + `assert_drained()` 做**运行期**结算，与 `scripts/ci/check-obligations.py` 的**静态**门禁并用；严重级先 W 后 E | 修的是一整类 bug，不是一个 bug：当前已核实同类隐患至少 2 处（`proof_calls`、`checker.rs:1313`），16 个 span 键控字段全在射程内。先 W 再 E 是为了让每一步的 C2 判据都可用——一步升到 E 会让 `yaoxiang check` 突然多出大量此前被静默的诊断 |
| **修复范围** | 修**消费端**（编排层），**不动** `checker.rs:5164/5179/5306/5318/5420/5448` 六处 `Unproven` 分支的生产端逻辑 | 问题在消费端不在生产端。改生产端会把架构缺陷掩盖成"逻辑修好了"，而这三处分支的逻辑本身是正确的 |

这样设计额外消除两类裂缝：

- **「注释承诺」与「代码行为」的裂缝**。`checker.rs:5165` 的「不得让 silent pass 复活」和 `layers/README.md:3` 的「下层失败上层不跑」都是**注释级契约**——前者靠 `assert_drained()` 兑现，后者靠拓扑驱动的 `Skipped` 诊断暴露。
- **「一个数组忘记 append」**。当前是五个函数各自手写调用序列；统一后是唯一一张阶段表，且新增阶段漏接会在编译期报错。

### 未采纳的方向

- **泛型阶段链 `Stage<A, B>`（`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`）**——11 处不一致中有 7 处（死代码分析两套实现、W1006、W1003、单态化、角色上下文、IR 生成/链接）**不涉及类型衔接**，是「要不要跑某个分析」的配置问题；它也表达不了 `Aggregation`，只能把 `check_module` / `check_module_collect_all` 的二选一从函数层搬到类型层。
- **`dyn Stage` + 运行期注册（`driver.register(Box::new(...))`）**——取消编译期穷尽性：新增阶段时 `dispatch` 不再编译失败，等于把「五个函数各自手写调用」换成「一个数组忘记 append」，正是当前 11 处不一致的成因。本仓库已有同形态反例：`docs/src/dev/design/check/` 描述的 `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` 全部零实现。
- **只补 bug 不动结构**——修得掉已知的 2 个 bug，但只覆盖 16 个义务字段中的 1 个，`Stage` 仍散落在 5 个函数里，下一个分叉点会继续从这里长出来。**它必须先做**（是 S1/S3 的一部分），因为结构改造需要一个已知红测试来证明判据有效。
- **把 `proof_calls` 改 `pub` 并加 `debug_assert`**——`check_module` 是通用入口，它不知道调用者是谁，`debug_assert!(<caller will handle>)` 无法成立；`#[must_use]` 只在字段被**整体**丢弃时警告。这是在错误的层面找 bug：bug 在编排层，检测必须在编排层。

## 已知局限与风险

- **`orchestrator.rs` 的瘦身在短期内会让它更难读**。139 行的 `compile_project` 拆成「Program 构造器 + 若干 driver 臂」后，读者需要跨两个文件才能理解流程。这是所有"把接线集中起来"的重构的共同代价。
- **S3 会显著改变诊断集，且改变的方向是「暴露此前被静默的问题」**。修复后可能出现一批"新错误"的用户报告——它们是真实存在的 bug，只是一直没被报出来。必须在 CHANGELOG 中明确说明。
- **`assert_drained()` 的严重级切换需要逐字段人工判断**。16 个字段里有些（如 `module_namespaces`）的"无消费者"可能是设计上就不需要消费，不应报警。S4 需要逐字段过一遍，不能一刀切。
- **`Program` 抽象可能过早**。若某些入口的阶段集合长期不稳定，`stages()` 会退化成"每次调用都传不同数组"的自由参数，契约约束落空。**缓解措施是 `test_program_stage_coverage` 断言 `stages()` 只能来自 6 个预定义组合。**
- **判据依赖**：S1/S3 的漏洞修复判据依赖 [等价性判据文档](07-equivalence-oracle.md) 的漏洞测试先红。**若该文档未先行建立多文件语料层，S1 无法验收**——因为现有293 个语料全部走单文件路径，对本类缺陷零覆盖。本文不涉及 IR 校验器（`verify_loose`），那部分前置工作不在本文范围。
- **层序修正会暴露此前被静默的 `Unproven`**（`equivalence` 根本不在管线、`termination` 与 `ownership` 顺序与声明相反）。这是诊断集变化风险，应独立走。
- **SMT 后端改单例后的线程安全未定**。`backend.rs:13-19` 已说明 `Solver` 只 `Send` 不 `Sync`，`Z3Backend` 的缓存是 `RefCell`；改成跨编译单元共享的单例后需确认访问路径全部经 `Mutex`。
- **阶段并行化未评估**。多文件 typecheck 天然可并行，但并行会**掩盖顺序依赖缺陷**（如 termination 与 ownership 的实际顺序依赖）。应在等价性判据稳定后再开。

> **本节原列的开放问题已全部裁决。** 逐条决定见 [RFC-039 决议登记](../../rfc/accepted/039-compiler-architecture.md)（D1–D50）。**本文不留任何待定项。**
>
## 参见

- [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) — 四层模型、路由表 A/B/C、G1-G10 验收门禁、P1-P10 执行顺序
- [01-routing.md](01-routing.md) — 阶段表与依赖方向规范、`scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — 三套平行类型表示的收敛（本文不引入第四套）
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm 分支可达性清理；`checker/semantic_tokens.rs` 的 `include!` 改造（施工步骤归 [09](09-execution-wbs.md) §P5 5.1）
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 分级、三层判据、`test_multifile_proof_obligation_not_dropped` 漏洞判据
- [RFC-027 编译期求值与类型](../../rfc/accepted/027-compile-time-evaluation-types.md) — Phase 2.5 证明函数执行的设计来源
- [RFC-013 错误码规范](../../rfc/accepted/013-error-code-specification.md) — `build.rs:19-55` 门禁的对照码表
- `src/frontend/core/typecheck/checker.rs:5165-5169` — 「不得让 silent pass 复活」的明文声明
- `src/frontend/core/typecheck/layers/README.md:3` — 「下层失败上层不跑」的明文声明（实际无 short-circuit）
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — SMT 初始化硬失败的明文理由
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — 「调用方应保守降级」的契约声明（与 `predicate.rs` 矛盾）
- `src/frontend/core/typecheck/types.rs:16-70` — 16 个下游必须消费的义务字段
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — 单测自认「停在 pipeline 之前」
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — 语料自认「由 pipeline.rs 在 check_module 之后执行」
- `wasm/src/lib.rs:48` — playground 的第四个编译调用方（单文件路径，故执行 proof_execution）
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen 在 shim crate 而非主 crate
- `build.rs:19-55` — 错误码构建期门禁，本项目强制机制的范例
