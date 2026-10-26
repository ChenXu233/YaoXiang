# 编译阶段契约与义务账本

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档。四层模型、验收判据分级与执行阶段顺序见 RFC-039 正文;各附属文档的定位见 [本目录索引](index.md)。

## 定位与范围

本文是 RFC-039 的 **L1 编排层**施工图。它处理一个问题：**「这次编译跑了哪些阶段、哪些没跑、为什么」当前分散在六条路径、十个入口函数里各自手工接线，没有任何一个地方能回答这个问题。**

当前状态可以概括为一句话：

> **代码自己声明的不变量，被架构而非逻辑破坏了。**

`checker/refinement.rs` 的 `revalidate_refined` 内注释原文写着（原 `checker.rs:5165`，P5 5.2.1 已搬迁）「Unproven → 编译错误，无降级、无 silent pass……不得让 silent pass 复活」。但这条不变量只在**单文件路径**上成立：形如 `x: Sorted(3)` 的证明函数约束，在多文件 `yaoxiang run`、`yaoxiang check`、LSP 三条路径下**静默通过**，证明函数从不执行，且不产生任何诊断。

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
| 4a | **CLI 单文件** | `lib::run_file` | `src/lib.rs` 的 `run_file` | → `run_with_source_name` → 路径 1 |
| 4b | **CLI 多文件** | `lib::run_project` | `src/lib.rs` 的 `run_project` | → `compile_project`（路径 3a） |
| 5 | **LSP / `yaoxiang check`** | `lsp::run_diagnostics` | `src/lsp/handlers/diagnostics.rs:146` | 项目内走 3c，否则手工 lex→parse |
|  |  | `check_files_with_diagnostics` | `src/util/diagnostic/mod.rs:565` | → `check_project`（`590-591`） |
| 6 | **wasm playground** | `run_code` / `test_compile` | `wasm/src/lib.rs` 的 `run_code` / `test_compile` | `compile_playground`（`Program { kind: WasmPlayground }` + `Driver::run`）→ 路径 1 |

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
> - **第 2/4/5 行已修复**（WBS 3.4.6，2026-10-19 用户裁决「做」）：多文件 `run` 现输出 W1003 与死代码族警告，四路径（单文件 run / 多文件 run / 单文件 check / 多文件 check）的 W 码集合归一——判据单一源在 `dead_code_analysis` 臂的共用实现上。
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

- **第 4/5 行的准确结论是**：`yaoxiang run` 在**多文件**路径下（`run_project` → `compile_project`）永不报 W1001/W1002/W1003；在**单文件**路径下（`run_file` → `Pipeline::run`）是报的。原因在 `compile_project` —— 它把完整 `TypeCheckResult`（含 `warnings`）压进 `type_results`，但该函数**全程无任何 `result.warnings` 读取点**，`result` 唯一的下游用途是 `generate_ir_with_context`。
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
| 形参精化校验（`revalidate_refined`，原 `checker.rs:5164`） | `if calls.is_empty()` → 推硬错误（原 `5170-5177`）；`ctx.proof_calls.extend(calls.clone())`（原 `5179`） | **不发任何诊断** |
| 调用点实参校验（`check_call_arg_refinements`，原 `checker.rs:5306`） | 推硬错误（原 `5308-5316`）；`ctx.proof_calls.extend(calls)`（原 `5318`） | **不发任何诊断** |
| 返回位义务（`check_return_refinement`，原 `checker.rs:5420`） | 推硬错误（原 `5442-5446`）；`ctx.proof_calls.extend(calls)`（原 `5448`） | **不发任何诊断** |

`checker/refinement.rs` 的 `revalidate_refined` 内注释原文（原 `checker.rs:5165-5169`）：

> `RFC-027 §4/§9：Unproven → 编译错误，无降级、无 silent pass。` …… `本分支因此成为真防线而非前瞻兑底：不得让 silent pass 复活。`

即：作者明知「不产诊断」是缺陷，把它明确归类为**已知的、无处可去的中间态**，设计上假定「一定有人会来读 `proof_calls`」。

**第 3 步 —— 该字段确实被填进了结果。**
`check_module_impl` 内声明局部 `proof_calls` 并收集（原 `checker.rs:1234-1235`），再以 `proof_calls, // Phase 2.5 预登记证明函数义务` 写入 `TypeCheckResult`（原 `:1439`）。字段定义在 `types.rs:29`。

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

（全仓 `proof_calls` 标识符的其他命中分三类，都不是本字段的消费者：`checker.rs` 与 `checker/refinement.rs` 的三处生产侧收集（原 `:1234/4564/5494`）；`verdict.rs:61` 是 `ProofResult` 的同名字段；`tests/rfc027_*.rs` 读的是 `ProofResult`。）

**第 5 步 —— orchestrator 的四个入口全部不经过那一层。**
`src/frontend/module/orchestrator.rs` 的 `compile_project`（`99`）、`check_project`（`273`）、`check_source_in_project`（`450`）、`compile_embedded_module`（`1374`）**均不调用 `pipeline.rs`**，直接调 `TypeChecker::check_module`（`124` / `493` / `1386`）。因此它们连 `pipeline.rs:187` 那个唯一读取点都碰不到。

**第 6 步 —— 后果：标准库自身的精化义务也走丢弃路径。**
`compile_embedded_module`（`1374`，`check_module` 调用在 `1386`）负责编译嵌入 std。这意味着**嵌入 std 模块自身的证明义务同样不被执行**。

**第 7 步 —— 后果：约束静默通过。**
`y: Sorted(3) = 5`（`Sorted: (x: Int) -> Type = { ... }`）在多文件 / check / LSP 三条路径下**编译通过、运行通过、无任何诊断**。证明函数从未被调用。

**第 8 步（补充核实）—— 层内还有第二个静默丢弃点。**
`check_module_impl` 处理 ownership 层结果时（原 `checker.rs:1303-1314`）：

```rust
// 原 checker.rs:1313（现 check_module_impl 内）
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

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` 正是这一点的实证。该文件标注 `// expect: compile-error E4018`（第 15 行），其头部注释第 7 行写着「状态: ❌ 应被编译期拒绝」——**它能通过，正因为它走的是唯一执行证明函数的那条路**。

**结论：第三层判据需要补充多文件语料层，否则它无法充当本文的验收工具。** 见「实施要点」S1。

### 义务字段：13 个「产出即契约」的字段，零机制保证

`src/frontend/core/typecheck/types.rs:16-76` 的 `TypeCheckResult` 中，有 **13 个字段**的文档注释明确写着「由 X 阶段产出 → Y 阶段消费」，即它们本质上是**跨阶段义务**：

| 字段 | 定义行 | 产出方 → 消费方 |
| --- | --- | --- |

> **取行约定**：定义行 = `types.rs` 里该字段的**声明行**（`pub` / `pub(super)` 那一行），不是 doc 注释首行——同一字段的注释块可能多行，声明行才是唯一可锚定的位置。本表上方那句里的**结构体范围**（`src/frontend/core/typecheck/types.rs:16-76`）随结构体行号浮动，由门禁 **F5** 机械对账（写错即红）；定义行列由 **F4** 对账。
| `proof_calls` | `36` | typecheck → **proof_execution** |
| `release_plan` | `38` | ownership → IR 生成（**`PlanId` 键控**：`ir_gen.rs:360` 读（`type_result.release_plan.clone()`）、`:3103` 用（`ReleasePlan::consume`）；结算完成标记 `:1095`） |
| `escaped_refs` | `40` | ownership → IR 生成 |
| `instantiation_requests` | `42` | typecheck → 单态化（`pipeline.rs:381`） |
| `existential_coercions` | `44` | typecheck → IR 生成 |
| `implementation_proofs` | `46` | typecheck → IR 生成 |
| `sum_types` | `54` | typecheck → IR 生成 |
| `sum_type_param_names` | `57` | typecheck → IR 生成 |
| `operator_dispatches` | `59` | typecheck → IR 生成（span 键控） |
| `overload_resolutions` | `62` | typecheck → IR 生成（**`PlanId` 键控**——调用点身份：产出 `inference/expressions.rs:4550`、消费 `ir_gen.rs:7697` / `:7772`（`OverloadResolutions::get`）；结算完成标记 `:1097`） |
| `try_expr_impls` | `65` | typecheck → IR 生成（span 键控） |
| `match_scrutinee_types` | `69` | typecheck → IR 生成（span 键控） |
| `module_namespaces` | `72` | typecheck → IR 生成 |

**改判记录（3.4.9，2026-10-13 用户裁决甲）**：原第 14 行 `interface_impl_registry`（types.rs:49）已**改判为树内事实**——它按设计由 typecheck 在相位消费、跨阶段零读取，不是义务：

- **在相位消费点（活体）**：`operator_interfaces.rs:623`（运算符查询读 `env.interface_impl_registry`）、`inference/expressions.rs` 七处约束求解查询（:1514 / :1646 / :3151 / :3876 / :3899 / :4400 / :4408）、`checker.rs:214-217` 只读访问器、`checker.rs:974` 注入 body_checker。
- **跨阶段零读取（直证）**：`git grep -n 'interface_impl_registry' -- src/middle` → 空；`src/driver` 侧仅账本自身的登记与文档（已随本轮移除）。
- **处置**：删掉文档注释里「镜像供 middle 端/LSP 消费」的**声明**，保留字段与在相位消费；该字段登记在 `src/driver/obligations.rs` 的 `NON_OBLIGATION_FIELDS`（含理由）。将来 LSP 真需要该表，按 4.6.x 新登记走，**不在本轮造消费者**。

**这些字段全部以 `Span` 或名字表为键跨层传递。** 等价性判据文档已指出 span 键控契约的失配会「静默失效，无任何错误」（`ReleasePlan` 是典型：两侧 span 计算方式任何一处不一致 → `Drop` 指令静默消失）。

`release_plan` 为什么存活而 `proof_calls` 死了？差别是**结构性的，不是偶然的**：

- `release_plan` 的消费者 `ir_gen.rs:360`（`release_plan: type_result.release_plan.clone()`）位于 `generate_ir_with_context` 内部，而 `compile_project:154-156` **恰好把完整的 `&TypeCheckResult` 传给了它**——消费者在下游模块，而下游模块在所有入口的必经之路上。
- `proof_calls` 的消费者在 `pipeline.rs` **顶层**，属于**另一个入口实现**。orchestrator 的四个入口根本不经过 `pipeline.rs` 这一层。

**已核实事实**：目前全仓库**没有任何机制**保证这 13 个字段被消费。唯一"保护"是 `ReleasePlan` 恰好搭上了 IR 生成的便车。

### 非义务字段登记：`TypeCheckResult` 里的通道 / 在相位 / 树内事实字段

**`NON_OBLIGATION_FIELDS`（8 条）**：`TypeCheckResult` 里不属跨阶段义务的字段逐一登记（字段名 + 理由），与上面「§义务字段」表**互补**：

| 字段 | 为什么不是跨阶段义务（要点） |
| --- | --- |
| `module_name` | 单元身份元信息（`src/middle` 0 命中） |
| `diagnostics` | 错误诊断通道（既有通道，非「产出即契约」） |
| `bindings` | 有跨阶段读取（ir_gen.rs:735/:5034），但文档注释未声明跨阶段契约 ⇒ 未收编 |
| `local_var_types` | 诊断渲染用（ir_gen.rs:852/:4543/:4794），同 `bindings`——未声明契约 |
| `semantic_db` | 消费者在 LSP（4.6.1 起经 `DriverOutcome.semantic_db` 通道外发，src/lsp/server.rs:482），不属编译阶段义务 |
| `trait_table` | 跨阶段读取 ir_gen.rs:4336/:6867，文档注释未声明契约 |
| `warnings` | 警告诊断通道（与 `diagnostics` 同类） |
| `interface_impl_registry` | 树内事实（3.4.9 用户裁决甲；在相位消费点见「§义务字段」的改判记录） |

**为什么不是「接线消费端」**：给这些字段造下游只会凭空引入假义务，把「按设计不需要跨阶段消费」伪装成契约缺陷；正确的闭合机制是**反向 containment**——`TypeCheckResult` 字段集 = 义务清单 ∪ 非义务登记（由 `scripts/ci/check-obligations.py` 判据 1 硬判：加字段忘登记、删字段忘同步、非义务登记缺理由、两表重叠都当场红）。

### 证明层的四个附加契约缺陷

以下均来自证明层分析，独立于入口分叉问题，但都属于「阶段契约没有被强制」这一类。

**（a）层序只在文档里声明，没有机制强制；`equivalence` 层不在管线里。**

`src/frontend/core/typecheck/layers/README.md` 描述的是 `checker.rs::check_module_impl` 的实际调用序（定位以函数名/驱动点为锚，行号按现树取）：

| 实际顺序 | 调用点 | 对应层 |
| --- | --- | --- |
| 1 | 精化走查驱动 `predicate::check_predicate`（`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` 三个站点） | `predicate.rs` |
| 2 | `termination::TerminationChecker::check_module` | `termination.rs` |
| 3 | `ownership::check_ownership` | `ownership.rs` |
| 4 | 类型约束求解 `env.solver().solve()`（不在 `layers/`） | — |
| — | `equivalence::check_type_equivalence` — **`checker.rs` 中零调用** | `equivalence.rs` |

层号标签（`equivalence.rs` = 0、`ownership.rs` = 1、`predicate.rs` = 3）只存在于各模块自己的头注释里，与调用顺序不对应；README 里没有「编号 → 调用时机」的映射。

`equivalence` 只被 `inference/assignment.rs:17` 作为 `is_subtype` 工具函数使用，与 `ProofResult` 无关。同时**不存在任何 short-circuit**——`check_module_impl` 把 termination 的错误逐条 `add_error` 后继续往下走，「下层失败上层不跑」不是事实。

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
| `termination.rs`（经 `check_module_impl` 注入，原 `checker.rs:1265-1268`） | 构造期注入 `with_solver_owned` | `None` → **静默跳过**（不注入） |
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
`check_module_impl` 的两处 `body_checker` 兜底（原 `checker.rs:1283-1287` 与 `1289-1293`）：

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker 为 None → 空类型账本
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker 为 None → 空调用所有权表
```

`body_checker` 为 `None` 时，ownership 检查拿到**空类型账本和空调用表**并照常运行——所有权分析退化为「什么都不冲突」，**无任何警告**。（处理建议：应记为一条 warning 级诊断，或至少在义务账本中留痕。）

`check_module_impl` 的注释称终止检查「在类型检查之后、约束求解之前运行」。实际 `self.env.solver().solve()` 在它之后（原 `checker.rs:1320`；termination 原 `:1256`、ownership 原 `:1296`）——**注释所说的「之前」实际是「两层之后」**。

### wasm 现状：shim crate 承载，29 个文件的分支是活的

wasm 目标**已建成并在 CI 中构建**，`cdylib` 不在主 crate 而在独立的 shim crate。

| 事实 | 证据 |
| --- | --- |
| 主 crate 仅 `rlib` | `Cargo.toml:29-31` |
| **shim crate 提供 `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11`（`crate-type = ["cdylib"]`）、`wasm/Cargo.toml:18`（`wasm-bindgen = "0.2"`） |
| shim 依赖主 crate（rlib）为库 | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }` |
| 主 crate 有 wasm 目标依赖段 | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"`（`Cargo.toml:131-137` 为对应的非 wasm 段：tokio/ureq/tempfile） |
| shim 目录排除出主 workspace | `Cargo.toml:3` `exclude = ["wasm", ...]` |
| **CI 有 4 处 wasm 构建** | `_build-wasm.yml`（可复用工作流，`:77` `wasm-pack build --target web --out-name yaoxiang`）；调用方 `dist-release.yml:267-273`（产物 `yaoxiang-wasm`）、`docs-deploy.yml:23-56`（解包进 `docs/src/.vitepress/public/wasm`，即 playground）、`nightly.yml:110-114` |
| Z3 wasm 静态库由 Emscripten 预构建 | `_build-z3-wasm.yml:220`、`_build-wasm.yml:35-59`（按固定 URL 拉取 `libz3.a`，缺失时降级为 warning） |
| **29 个文件**出现 `wasm32` 字面量 | 其中 **22 个**带实际 `#[cfg(...)]` 属性，另 7 个仅文档提及（清单与逐条判定见 `06-cleanup-inventory.md` 的「F2 判定」小节） |
| `orchestrator.rs` 16 处 | 10 条属性（4 正 + 6 反）+ 6 处注释 |
| `src/lib.rs` 11 处 | 全为属性（`28/31/33/47/145/147/151/170/187/196/269`） |

**结论：这 29 个文件的 `#[cfg(target_arch = "wasm32")]` 分支是承重的。** 它们决定 `wasm/src/lib.rs`（152 行）在 wasm 目标下能调用主 crate 的哪些 API——`src/lib.rs:151/170/187` 把 `run_file` / `run_project` / `build_bytecode` 整体门控掉（这三条都需要 `std::fs`），而 shim 走的是另一条路。

**但这带来一个与本文直接相关的事实**：`wasm/src/lib.rs` 的 playground 入口 `compile_playground`（`:84`）构造 `Program { kind: WasmPlayground }` 交 `Driver::run`（JS 侧入口 `run_code`，`:122`）——**单文件路径**。因此：

| 消费 `proof_calls` 的路径 | 是否执行证明函数 |
| --- | --- |
| 单文件 CLI（`src/lib.rs` 的 `run_file`，`:152`） | **是** |
| wasm playground（`wasm/src/lib.rs` 的 `run_code`，`:122`） | **是** |
| `build_bytecode`（`src/lib.rs`，`:188`） | **是** |
| 多文件 `run`（`compile_project`） | **否** |
| `check`（`check_project`） | **否** |
| LSP 项目内（`check_source_in_project`） | **否** |

即：**`wasm/` 的存在使得"proof_execution 只有一个消费者"变成了"有三个"**，但这三个全在单文件路径一侧。统一 Driver 时这条路径已由 `ProgramKind::WasmPlayground` 覆盖（`src/driver/program.rs:31`，与 `SingleFile` 共用同一条单文件阶段表，`:135`）；落地记录见 `09-execution-wbs.md` 的 **4.2.8** 行（`wasm` 改走 `ProgramKind::WasmPlayground`，`:274`）。

（清理范围——`orchestrator.rs` 的 10 条 wasm 属性（4 正 + 6 反）中哪些在 playground 场景下不可达——归 `06-cleanup-inventory.md` 的「F2 判定」小节：4 条 wasm 臂在 playground 运行期不可达但编译期承重、6 条 `not(wasm32)` 臂在 native 三条阶段表承重，无可直接删除的低风险项。RFC-039 开放问题中「建 wasm 目标，还是删这些分支」一条**已经过期**：目标已建成。）

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
    pub release_plan:            ReleasePlan,                // types.rs:38
    pub escaped_refs:            HashSet<String>,            // types.rs:40
    pub instantiation_requests:  Vec<InstantiationRequest>,  // types.rs:42
    pub existential_coercions:   Vec<ExistentialCoercion>,   // types.rs:44
    pub overload_resolutions:    OverloadResolutions,         // types.rs:62
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

> **修订注记（P4 实施，2026-10-13，4.3.1 落地）**：本注记登记上方草图与三档表的**实施形态、
> 实测数字与 4.3.2 接口**，逐条对应代码 file:line。落地提交 `1941cc47a`（验证树 `99079bc52`）。
> 它同时是红名单「不静默」三处承担里的**第三处**（另两处：审计面、静态门禁）。
>
> **一、落地物（file:line）**
>
> - 账本 `src/driver/obligations.rs`：13 条义务清单 `INVENTORY`（:201，字段名/消费者阶段/红名单）、
>   **非义务字段登记 `NON_OBLIGATION_FIELDS`（:150，8 条含理由）**、交付标记台账 `DeliveryLedger`、
>   纯结算 `settle_obligations`（:577）、外发 `emit_obligation_diagnostics`（:654）、
>   非消费方门控 `consumer_visible_unit`（:538）、跳过判据 `consumer_stage_skipped`（:549）、
>   IrGeneration 交付集 `IR_GENERATION_DELIVERED`（:285）、W1090 码串常量 `OBLIGATION_UNDRAINED_CODE`。
> - **唯一结算点**：`Driver::run` 阶段表尾部、`into_outcome()` 产出 `CompilationResult` 之前
>   —— `src/driver/mod.rs:110`（`State::assert_drained`，obligations.rs:570）。`Aggregation::FailFast` 的
>   早退路径同样结算（账要平）。
> - **七个交付点**（每臂一处；摘掉任一处，运行期 W1090 必响）：
>   ProofExecution `src/driver/arms.rs:554`（`proof_calls()` 是 `TypeCheckResult` 对该义务的唯一对外读取路径，`types.rs:170`）；
>   IrGeneration `arms.rs:672`（Check）/ `:726`（Embedded）/ `:781`（SingleFile·WasmPlayground）/ `:842`（MultiFile）
>   —— 四处同一交付口：`AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`；
>   Monomorphization `arms.rs:936`（单文件直读请求表）/ `:1221`（MultiFile·Check：Linking 臂聚合，
>   `driver/helpers.rs:295` 读各单元 `instantiation_requests`）。
> - 审计面：`DriverOutcome.obligations`（`src/driver/state.rs:73`）= `ObligationSettlement{ findings, pairs,
>   empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`；
>   守恒式 `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` 由用例钉住。
> - 诊断码：W1090 注册于 `src/util/diagnostic/codes/w1xxx.rs:32`（`span_exempt=true`，理由写在条目上方 :26-31
>   —— 结算粒度是「单元 × 义务」，没有用户可指的源码位置，不塞 dummy span）；六语言文案见 `locales/*.json`。
>   **发射级别**：4.3.2 起为 **error**（同码升 E，见第三段）——由 `to_diagnostic`（obligations.rs:444）的
>   显式 `.severity(Severity::Error)` 决定，码号仍是 W 族。
> - 静态门禁：`scripts/ci/check-obligations.py`；CI 新 job `obligations`（`.github/workflows/ci.yml:140-155`，含 `--selftest`）。
>
> **二、三档表的逐格落地 + 两条加档**
>
> | 上方三档表的情形 | 实现判据 | 代码 |
> | --- | --- | --- |
> | 义务被声明的消费者读取 | **交付标记存在**（标记 ↔ 真实读取由静态门禁对账，见第五段） | obligations.rs:618-623 |
> | 义务非空但无消费者 | 非空 + 消费者阶段已运行 + 无标记 → W1090（**error 级**，见第三段） | obligations.rs:624-630 |
> | 义务为空 | `Obligation::is_produced` 逐字段判空 → 通过 | obligations.rs:587-591 |
>
> 两条**加档**（2026-10-13 用户裁决，草图未含）：
>
> 1. **S1**：义务的消费者阶段不在本 `ProgramKind` 的阶段表里 → **不判欠**（obligations.rs:593-597）。
>    证据：`LSP_STAGES` 无 IrGeneration/Monomorphization（program.rs:112-119）、`EMBEDDED_STAGES` 无 Monomorphization（:121-126）。
> 2. **跳过豁免**：消费者阶段以 `SkipReason::UpstreamFailed` / `ConfigDisabled` 跳过 → 不判欠（obligations.rs:598-602）。
>    `NoObligations` **不**豁免——义务非空而该阶段自认无义务，正说明账不平，必须报。
>
> **三、外发口径（裁定「丙」）与它背后的实测数字**
>
> **4.3.1（观测期）**：本机制只外发 **W 级**（W1090）；**无 strict 开关**。
> **4.3.2（终态，2026-10-13 用户裁决甲）**：**同码升 E**——`W1090` 码号不变，
> 发射级别抬到 `error`（显式 `.severity(Severity::Error)`，`to_diagnostic` obligations.rs:444）；
> **不新增 E 码**（同一事实两个码即平行表示）。**升级只改发射级别、不改发射条件**。
> **这是 D59 的 D 类破坏项（改变对外可观察行为），如实登记**：健康程序零发射（义务全部有交付标记时不报），
> 故既有语料与用户程序零影响（342/342「差异: 空」）；但一旦某次运行真的命中未交付义务，
> 该诊断即**计入 `error_count`、渲染 `error[W1090]`、退出码非零**——与此前「W 级不阻断编译」不同。
> 码族与发射级别解耦的口径已登记 RFC-013「发射通道」节（码号前缀只决定编号归属，级别由构造点显式 severity 决定）。
> 机制事实（实测）：严重级取显式 `.severity()`，否则「码以 `W` 开头 → Warning」（`codes/builder.rs:186-203`），
> 注册表的 `ErrorCategory` **不参与**严重级；退出码由 `(error_count, warning_count)`（`util/diagnostic/command.rs:87`）
> 与 `--deny-warnings`（`src/main.rs:567-568`）决定，**W1090 因显式 error 而走 `error_count`（4.3.2 起）**。
>
> **红名单字段不进用户可见诊断流**（captain 2026-10-13 裁定丙；当前红名单已归零，见第四段），理由是可复算的：
>
> - 裁定甲（红名单按 W 外发）实测代价：`interface_impl_registry` 由 typecheck 预置表灌入、**恒非空**，
>   逐编译外发的是**常量信号**（无区分度），却会破坏三条既有对外契约——「干净程序零诊断 / LSP 零诊断 /
>   `warning_count == 0`」：`cargo test --lib --all-features` 实测 **12 个既有用例转红**，
>   其中 **8 个在 out-of-scope 路径**（`orchestrator/tests/namespace_data_access.rs` ×5、
>   `lsp/handlers/tests/diagnostics.rs` ×2、`util/diagnostic/tests/mod_tests.rs` ×1；另 4 个在 `src/driver/tests/`），
>   语料基线 24/342 个夹具变化，且唯一可行改法是「把 W1090 从零诊断断言里过滤掉」= 放宽既有判据。
> - 裁定乙（降 Info 级）仍会破坏其中的列表类断言 9/12 个，且 Info 不是「按裁决级别输出」。
> - 裁定丙的实测：既有用例 **零改动**、语料基线 **零变化**（342/342「差异: 空，行为等价」），
>   运行期 W1090 成为**有区分度的回归探测器**——健康树全程静默，摘掉任一交付标记才响
>   （三处交付点各做过摘标记变异，CLI `check` 在 Check 臂摘标记时打出 2 条 `warning [W1090]`）。
>
> 红名单的「不静默」由三处承担：① `DriverOutcome.obligations.findings` 审计面（每次运行都带出，含 WBS id 与证据指针）；
> ② `check-obligations.py` 每次运行打印红名单（当前为空）+ 陈旧检测；③ 本注记。
>
> **四、红名单现状（清账收口）与 3.4.4/3.4.5/3.4.9 的落地记录**
>
> 红名单 3 → **0**——三条清账任务已于 2026-10-13 全部落地：
>
> 注：以下行锚记录 **2026-10-22 当时**的文件状态；P5（2026-10-26）把 checker 拆成 `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs` 后，`checker.rs` 行号整体前移，下列锚点不再对应现文件，仅作历史引证。
>
> - **3.4.4 `variant_ctor_calls` → 删字段**。其声明职责（「ir_gen 生成 CreateVariant」）由 IR 侧
>   就地形态检测等价兑现：`src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call`（同一 AST 形态
>   `FieldAccess{Call(Var)/Var}` + 从 `sum_type_variants` 取声明序），调用点 `:7496` 将整个 Call 换成
>   `CreateVariant`；该函数 doc 明写「span 键跨系统不可靠（两端对 Call 节点 span 的语义不同），故就地检测」
>   ——**接线消费端会把已修掉的 span 失配缺陷引回来**。删除面：`types.rs` 字段、
>   `checker.rs:1489-1493/:1565` 的采集、`inference/expressions.rs` 与 `inference/statements.rs` 的采集链、
>   `environment.rs` 的 `VariantCtorCall` 结构；`infer_variant_ctor_call` 的类型推断与全部诊断**不变**。
> - **3.4.5 `method_overload_ir_names` → 删跨阶段镜像**（在相位候选名表与 `overload_resolutions` 保留）。
>   全部读写点：写在相位（`environment.rs:205-219` 与 `method_overloads` 同序登记 → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`）、读在相位（`expressions.rs:4490-4550`
>   决议点，唯一命中时 `:4550` 写入 `overload_resolutions`）；被删的是 `checker.rs:1558` 灌进
>   `TypeCheckResult` 的**零读者镜像**。命名事实仍单点表达：候选名表（在相位）+ span 键控决议
>   （跨阶段，`ir_gen.rs:380` 构造、`:7632/:7705` 消费）；定义侧混编名由 `method_def_ordinals`（`:1828`）自算。
>
> - **3.4.9 `interface_impl_registry` → 改判为树内事实**（用户裁决甲，详见本文 §义务字段表的改判记录）。
>   删的是文档注释里「镜像供 middle 端/LSP 消费」的**声明**（types.rs:47-48），保留字段与其在相位消费
>   （`operator_interfaces.rs:623` 运算符查询、`inference/expressions.rs` 七处约束求解查询、
>   `checker.rs:214-217` 只读访问器）；跨阶段零读取直证 `git grep -n 'interface_impl_registry' -- src/middle` → 空。
>   **为什么不接线消费端**：该表的内容是 typecheck 预置表灌入的环境镜像（恒非空），LSP 若真需要按 4.6.x 新登记走；
>   给「按设计不需要跨阶段消费」的字段造消费者，只会把假义务引入账本。
>
> 红名单归零后，**字段普查由「义务清单 ∪ 非义务字段登记（`NON_OBLIGATION_FIELDS`，含理由）」闭合**——
> 门禁判据 1 的反向 containment 端到端守住这条（加字段忘登记 / 删字段忘同步都当场红）。红名单**机制**保留：
> 门禁判据 4 的陈旧检测与 `--selftest` 的 `stale-red-list` 夹具继续覆盖其检测面，将来重新登记须走显式裁决。
>
> 随清账同步的口径：义务清单 16 → **13**（`INVENTORY` / `Obligation::ALL` / 本文 §义务字段表）；
> 门禁 `--selftest` 现覆盖七类夹具（新增「未登记字段」「锚点失效」与「红名单为空」三种形态），
> 见 `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`。
>
> **五、静态门禁（9.2）的实际判据**（比草图那句话多三条硬判据）
>
> 1. **三处对账 + 反向 containment（F2）+ 锚点校验（F1）**：`obligations.rs` 的 `INVENTORY` ↔ `TypeCheckResult` 字段定义 ↔ 本文「§义务字段」表（含表头条数声明），任一漂移即红；
>    并且 `TypeCheckResult` 的字段集必须**恰好等于**「义务清单 ∪ 非义务登记（`NON_OBLIGATION_FIELDS`，含理由）」——
>    加字段忘登记、删字段忘同步、非义务登记缺理由、两表重叠都当场红；
> 2. **读出证据**：非红名单义务必须在**声明的消费者文件**里有真读取（`.字段名` 成员访问；注释/测试/字符串不算）——这条把「交付标记 ↔ 下游真实读取」做成硬判据（机制甲的主要风险正是二者漂移）；
> 3. **交付标记**：非红名单义务必须在交付点被标记；红名单反之不得被标记；
> 4. **红名单陈旧检测**：红名单字段必须零真读取（长出消费者即红），且每条带 WBS id 与证据指针；
>    另有 `--selftest`：自带反例/正例夹具，反例必红、正例必绿（证明探测器不空转）。
>
> **六、与上方草图的逐处偏差**
>
> 1. 草图的 `pub struct Obligations { …字段… }` **没有按字段抄一份**：义务数据仍由 `TypeCheckResult` 持有，
>    账本只记「清单 + 交付标记 + 结算结论」。理由：抄一份即两处事实源（各字段分别产自 typecheck 的 checker 与
>    所有权（ownership）层），必然漂移；跨层数据面也不该复制到编排层。
> 2. 草图的 `take()` / 「标记 consumed」**未采用**，改为 `is_produced` 判空 + `DeliveryLedger` 交付标记。
>    理由：义务与产物是同一份数据，`take()` 会把下游要用的东西取走；且 11 条 IrGeneration 义务是「整份
>    `&TypeCheckResult` 交给 ir_gen」的形态，无法逐字段 take。
> 3. 草图签名 `pub fn assert_drained(&self) -> Vec<Diagnostic>` → 实际 `State::assert_drained(&mut self)`：
>    结论写入 `DriverOutcome.obligations`，诊断按 `ProgramKind` 分流：Check/Lsp 走逐文件收集通道；
>    其余形态按**级别**分流（4.3.2 起 W1090 为 E 级）——单文件走 `PipelineError::TypeCheck`（obligations.rs:654 外发臂），
>    多文件/嵌入另记 `OrchestratorError` 故障（否则 `result.errors` 无人消费）。
>    理由：返回 `Vec<Diagnostic>` 会丢掉形态分流——check 的诊断必须逐文件归属。
> 4. 三档表第一档判据从「义务被声明的消费者读取」落为「有交付标记」；「标记 ↔ 真实读取」交给静态门禁（第五段判据 2）。
> 5. 三档表第二档的「`strict` 模式升 E 级」**未走 strict 开关**，改为 **4.3.2 同码升 E**（无 strict 开关也升，
>    因为健康态零发射已足够安全，R4）。
> 6. 静态门禁判据由「字段出现 ≥2 次但无第三文件读取」升级为第五段的四条。
> 7. 草图未含的两条加档（S1、跳过豁免）见第二段。
> 8. S1/S4 表提到的单测试名 `test_obligations_drained`：实际落为 `src/driver/tests/obligations.rs` 的
>    **20 条用例**（先 `#[ignore]` 骨架 → 转绿；红态留档：摘除唯一结算点后 `pairs: 0`、6 条失败）。S1/S4 两行已同步为实际文件。
>
> **七、4.3.2 的落地记录（定级 + 门禁加固）**
>
> - **逐字段复核的输入**：`ObligationSettlement.drained_kinds`（本次运行真的被排空的义务种类，逐运行可得）
>   与五个计数（第二段守恒式）；配合审计面 `findings` 可给出「该字段是否被任何入口消费过」的事实。
> - **级别：同码升 E 已落地**（2026-10-13 用户裁决甲；见第三段的终态登记与 D59 D 类破坏项声明）。
>   实现面：`to_diagnostic` 显式 `.severity(Severity::Error)`；外发臂按级别分流错误通道。
>   变异证据：摘掉任一处交付标记 → 该形态的编译失败、`check` 退出码非零（`error [W1090]`）；
>   还原后健康态零发射（语料 342/342 零变化）。locale 文案已在六语言里写明「W 族码、发射级别为 error」。
> - **门禁加固项（F1/F2，均已落地）**：
>   - **F1/R2 锚点校验**：门禁现在解析**非义务登记理由与红名单证据**里的 `file:line` 锚点，校验
>     「文件可定位（唯一）+ 行号在册 + 锚点后紧跟的符号真的出现在被锚文件里」；锚点指到不存在的符号即红。
>     自带反例夹具 `tests/baselines/obligations-gate/stale-anchor/`（符号不存在 / 行号越界各一条）。
>     注：红名单当前为空，故该判据的实际落点是**非义务登记**与将来的证据指针（同一代码路径）。
>   - **F2/R3 反向 containment**：门禁输出「`TypeCheckResult` 字段数 − 义务数 − 非义务登记数 = 差值」，
>     差值非空即红；自带反例夹具 `unregistered-field/`（结构体新增字段未登记）。
> - **信息项 F3**：`SkipReason::UpstreamFailed` 豁免在真实入口不可达（typecheck 失败的单元不保留 `type_result`，
>   义务本就不存在）；该档语义由合成 `State` 用例钉住，真实端到端只覆盖 `ConfigDisabled`（t3 T06）。
>
> **八、4.3 二级任务收口（红名单归零 + 义务诊断终态 E 级）**
>
> **8.1 三轮落地时间线**（逐条对应代码 file:line；本轮只补登结论，不改代码）
>
> | 轮次 | 结论 | 代码锚点 |
> | --- | --- | --- |
> | 4.3.1 账本上线 | `State::assert_drained()` 唯一结算点；13 条义务清单 + 交付标记 + 三档结算；审计面 `DriverOutcome.obligations` | 调用点 `src/driver/mod.rs:110`；结算 `src/driver/obligations.rs:577`；外发 `:654`；七处交付标记 `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221` |
> | 3.4.4 清账 | `variant_ctor_calls` 删字段——声明职责由 IR 侧就地形态检测等价兑现 | `src/middle/core/ir_gen.rs:7448`（`detect_variant_ctor_call`）、`:7496`（调用点） |
> | 3.4.5 清账 | `method_overload_ir_names` 删跨阶段镜像（在相位候选名表 + `overload_resolutions` 保留） | 决议消费点 `ir_gen.rs:380/:7632/:7705`；定义侧 `method_def_ordinals` `:1828` |
> | 3.4.9 清账 | `interface_impl_registry` 改判**树内事实**：删「镜像供 middle/LSP 消费」声明、保留在相位消费；义务 14 → 13、红名单 1 → 0 | 声明载体 `src/frontend/core/typecheck/types.rs:52`（字段声明行；其 doc 注释见同文件 `:47-51`，裁决甲改写）；在相位消费 `src/frontend/core/typecheck/operator_interfaces.rs:623`、`src/frontend/core/typecheck/inference/expressions.rs` 七处（`:1514/:1646/:3151/:3876/:3899/:4400/:4408`）、`src/frontend/core/typecheck/checker.rs:214`（只读访问器 :214-217）；登记 `src/driver/obligations.rs:150` |
> | 4.3.2 定级 | **同码升 E**：显式 `.severity(Severity::Error)`，码号仍 `W1090`；外发按级别分流错误通道（首个失败优先） | `src/driver/obligations.rs:450`（severity）、`:676-699`（E 级分支）；码族与级别解耦的登记见 `src/util/diagnostic/codes/w1xxx.rs:26-36` |
>
> **8.2 收口结论**
>
> - **红名单归零**：`INVENTORY`（`src/driver/obligations.rs:201`）中 `red_list` 为 `Some` 的条目数 = **0**
>   （`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0）。机制**保留**：`RedListEntry`（`:120`）、
>   `Obligation::red_list()`、门禁判据 4 的陈旧检测与 `stale-red-list` 夹具；门禁输出「红名单 0 条——已清账归零」。
> - **义务诊断终态 E 级**：`W1090` 的**码号与 W 族归属不变**，发射级别为 `error`（D5 终态，2026-10-13 用户裁决甲）。
>   这**属 D59 的 D 类破坏项**（改变对外可观察行为），已在本注记第三段与 RFC-013「发射通道」节双处登记；
>   **健康态零发射 ⇒ 现有语料与用户程序零影响**（语料 342/342「差异: 空」、IR 快照零漂移；t11 独立验证与 t12 复审各自复核）。
> - **字段普查闭合**：`TypeCheckResult` 字段集 = 义务清单 ∪ 非义务登记（8 条，`src/driver/obligations.rs:150`，逐条含实测消费面理由）；
>   门禁 F2 显式输出差值（当前 `21 - 13 - 8 = 0`）。
> - **不留未落地承诺**：本注记登记的加固项全部**已落地**（F1/F2/F4/F5，见第五、七、八段）；
>   唯一未闭环的两项是 8.4 的**已知边界**与已转 issue 的 t7 F5，二者都不是本机制的未落地判据。
>
> **8.3 各轮验收 / 复审发现的处置状态**（逐条标明）
>
> | 来源 | 发现 | 状态 |
> | --- | --- | --- |
> | t3 F1 / t4 R2 | 红名单 evidence 是纯文本指针，替代消费者被摘除时会静默陈旧 | **已落地**：门禁 F1 锚点校验（文件可定位 + 行号在册 + 符号锚，`scripts/ci/check-obligations.py:251`）；反例夹具 `tests/baselines/obligations-gate/stale-anchor/` |
> | t3 F2 / t4 R3 | 门禁判据 1 缺反向 containment，新增字段忘登记时静默 PASS | **已落地**：F2 差值显式输出（`:577`）+ F4 定义行对账（`:455`）+ F5 范围对账；反例夹具 `unregistered-field/` |
> | t4 R4 | W-only 与升 E 的口径未登记 | **已落地**：本注记第三 / 七段 + RFC-013「发射通道」+ 六语言 `locales/*.json` 的 W1090 help 与 error_output 样例 |
> | t7 F5 | 死代码分析对重载方法引用计数失真（合法重载仍被误报 `W1005`） | **已转 issue `#450`**，归属 WBS **4.9 引用池正确实现**（`docs/src/dev/architecture/09-execution-wbs.md:237` 登记）——非本二级任务 |
> | t11 F7 | 门禁 `--root` 模式下 F1 不覆盖模块头散文注释里的锚点承诺 | **已知边界**（见 8.4） |
> | t11 F8 | 验证方工具侧自记：变异驱动曾误写主仓（即时 `git restore` 清除、受影响轮次输出作废重跑） | **信息项**（非交付物缺陷） |
> | t11 F9 | 主仓中间态（未暂存文件使本文表头与 `INVENTORY` 不同步）触发门禁红 | **信息项 / 已消解**：冻结提交自洽 PASS；「中间态直接提交会红门禁」正是 F2/F4 的设计意图 |
> | t12 F12-1 | 本文的结构体范围写 `16-73`，实际 `16-76`（t9 加 3 行注释后无机械对账） | **已落地 + 已机械化**：范围校正为 `16-76`，并新增门禁 **F5**（范围锚点，写错即红）；同类数字一并 grep 对齐（`17 → 20 条用例`、`16 个 span 键控字段` → 13） |
> | t12 F12-2 | 多文件形态的结算曾顶掉首个失败（`zzz.yx` 的类型错误被 W1090 覆盖） | **已修**（captain 已 amend 提交信息说明）：`src/driver/obligations.rs:686` 的 `failure.is_none()` 守卫；该回归由**既有钉板**用例 `test_driver_run_multi_file_proof_error_yields_to_typecheck_error` 抓出，非新增测试 |
>
> **8.4 已知边界（登记项，不是未落地承诺）**
>
> - **F1 的扫描面**：锚点校验只覆盖**在册条目**的文本——非义务登记理由（`NON_OBLIGATION_FIELDS`）与红名单 evidence；
>   **模块头散文注释里的 `file:line` 承诺不在扫描面**（当前无在册条目受影响）。将来若在模块头写锚点承诺，
>   需**人工 review 承担**，或扩 F1 的扫描面——两者取其一，不许默认无人管。
> - **门禁的静态边界**：交付标记的集合级校验在静态侧（判据 3），单臂摘除由**运行期结算**补位——
>   静态 + 运行双层是设计形态，不是缺口（t11 的 M11A/M11B/M11C 变异双向验证）。

> **修订注记（P4 实施，2026-10-13，4.5 跨层契约 PlanId 化落地）**：上方 13 条义务里**两条 span 键控契约**
> （`release_plan` / `overload_resolutions`）已按 RFC-039 **D20** 改为**结构身份键**（`PlanId`）。本注记登记落地形态、
> 判据、分类依据与已知边界；定义行仍由门禁 **F4** 对账（上表两行的行号即当前代码事实）。
>
> **一、落地形态（载体取甲：`PlanId`）**
>
> 键是随 AST 节点走的**结构身份**，不再靠「两侧各算一遍 span」的位置巧合：`PlanId`（`src/util/plan_id.rs`，`u32` 单调、
> 编译单元内唯一、`UNPLANNED = u32::MAX` 表解析器之外的合成节点）由**唯一分配器** `ParserState::alloc_plan_id` 发放。
> 载体两处：`Stmt.plan_id`（语句身份 → 释放计划的归属与消费）与 `Expr::Call.plan_id`（调用点身份 → 重载决议的产出与消费）；
> `Expr::call` 是全仓唯一的调用表达式构造器，解析器外的合成调用一律记 `UNPLANNED`（**不可被消费**，见第二段）。
>
> - `release_plan`：`drops` 为 `HashMap<PlanId, Vec<String>>`（`layers/ownership.rs`），消费唯一入口
>   `ReleasePlan::consume(stmt)`——**读与记账同一次调用**；`ir_gen.rs:360` 读、`:3103` 用（语句身份由 `walk_stmt` 的
>   置/恢复 + CFG 节点→语句身份表给出；作用域退出的 Drop 归属本作用域最后一条语句）。
> - `overload_resolutions`：`OverloadResolutions`（表 + 消费台账**同体**），产出点 `inference/expressions.rs:4550`
>   （按调用点 `PlanId` 写，dispatch 从 `Expr::Call { plan_id }` 解构透传），消费点 `ir_gen.rs:7697`（有绑定）/ `:7772`
>   （无绑定）经 `OverloadResolutions::get`（命中即记账）。
>
> **二、消费必达结算（复用 4.3 账本，不造第二套）**：消费台账是**单一实现** `ConsumptionLedger<K>`
> （`src/util/consumption.rs`），两张表各持一个实例；`generate_module_ir` 成功返回时**同点**标两条台账的
> 「完整跑完一轮」（`ir_gen.rs:1095` 释放计划 / `:1097` 重载决议）；三态映射唯一
> （`src/driver/obligations.rs` 的 `consumption_state`），唯一结算点对两条义务各接入一行——差集非空即
> **E 级 W1090**（同码升 E，口径见第三段）。豁免判据挂在**台账自身的完成标记**上（不是 `unit.ir` 是否存在），
> 故 `check` 形态取走 IR 产物不构成豁免。
>
> **三、D59 分类依据（C 类，先红后绿）**：`Type.method(位置实参, 名字 = 值)` 这一形态在改动前是**静默给错答案**
> （实测：`r: Int = t.pick(1, a = 2)` 编译干净、运行输出 `1` 而非 `20`——决议出的候选被丢弃、回落到定义序最后的绑定）；
> 改动后同一程序编译期 `error [W1090] ... 产出键未被消费`。修法取**响亮**而非「把决议名塞进带命名实参的调用」：
> 后者会丢命名实参，等于换一种静默（终态语义见 [09](./09-execution-wbs.md) §3.14 / issue #452）。
> 另：`Drop` 指令在解释器是 no-op（`src/backends/interpreter/executor/ops/control.rs` 通配臂仅 `advance()`），
> 故释放计划的键迁移不改运行行为——语料 342 条差分空是这一点的实证。
>
> **四、判据与证据**：D41 同型判据 `test_release_plan_spans_consumed`（按 `PlanId` 结算）与新增
> `test_overload_resolutions_consumed`（语料逐文件 lex→parse→typecheck→ir_gen，**差集必须为空 + 证据面地板**：
> 产出过决议的文件数 `≥ 1`、被真实消费条目 `≥ 2`，无白名单）；决议形态覆盖 9 项钉板见
> `src/frontend/core/typecheck/tests/overload_resolutions.rs`（期望返回 / 位置实参 / 类型体内绑定 / `?` 操作数 /
> 纯命名 / 位置+命名 / 歧义 / 变体构造 / 未分配身份）。变异驱动：消费键偏移一位 ⇒ 判据红 + 真实程序
> `error [W1090]` + 语料门禁差异；还原后复绿。门禁数值：`check-obligations.py` PASS、语料 342 条差分为空、
> `cargo test --lib` 2712 passed / 0 failed（IR 快照零漂移）。
>
> **五、已知边界（登记项，不是未落地承诺）**：`ir_gen.rs:7766-7790` 的**无绑定消费点**在当前语料与测试面
> **不可达**（方法声明统一经 `register_type_bindings` 走有绑定消费点），故该分支的键控正确性**无红态可证伪**；
> 保留理由 = 它是有绑定路径的对称分支，删除会把「无 `type_bindings` 的方法调用」形态改写成静默。由独立验证
> （t5 F-1）与复审（t6 F2）登记，**不新开任务**。

### 3. 统一 Driver：单一 `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project（LSP 项目内，结果过滤到目标文件）
    Embedded,          // compile_embedded_module（嵌入 std）
    WasmPlayground,    // wasm/src/lib.rs 的 compile_playground —— 单文件路径的第四个调用方
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
>     (i) 支持间接调用 / (ii) 编译期拒绝，**已裁 (i)**——支持间接调用：`CallDyn`
>     机制已在（`ir_gen.rs:7945` 闭包表达式路径），断的是 `Var` callee 被降格为按名
>     `Call`，且 typecheck 本就接受该程序。(5) **取舍判据固化**——方案
>     否决理由只能基于正确性与可读性，「改动面大 / 引入新机制 / 成本高」不构成否决
>     理由，落 `coding-rules.md` 第二部分与 HOWTO 自检表；需用户拍板的取舍按
>     `AGENTS.md`「请求用户裁决时（强制）」给全四件（情况+证据 / 逐项优缺点 /
>     明确推荐+代价 / 破坏性点名）。
> 17. 3.4.8 ② 落地与 F1 根因（2026-10-11，375789d2 + d922a551）。② 按 #14 设计落地：
>     `Stage::ALL` 改为 Linking 先于 Monomorphization、MULTI_FILE 表尾追加 mono；Linking 臂
>     在**取走各单元 IR 之前**聚合全程序请求并把 `containing_fn` 限定化（否则 deferred 桶键
>     与 merged IR 不同源、桶永不排水）。**① 收口（D2 裁甲）**：用户模块导出随
>     `Export::type_params` 携带声明期类型参数快照，与 std 路径（`yx_sources.rs:62/108`）同构。
>     **过程中实证并修复的既有缺陷**：(a) **F1**——`qualify_names` 只重写函数体，`ir.init`
>     从不被访问，多文件下函数定义已限定而模块顶层调用仍裸名 → 运行期 E6006
>     （**非泛型即复现**，与泛型无关）；修复为 init 复用同一 `rename`/`aliases` 与同一
>     `rewrite_call_names`。(b) mono 侧「全程序唯一后缀」启发式与未命中静默 `continue` →
>     删除启发式、未命中改**响亮 E3018**（歧义静默错配比不解析更糟）。**独立验证抓到并修复
>     的回归**：t6 发现具名导入 `use lib.{pick}` 后裸名调用被 E3018 误拒（IR 指令名是别名
>     限定名、请求名是裸名，聚合侧只按单元键限定化）→ 修复为聚合侧**直调 ir_gen 的**
>     `import_aliases_from` 与 `resolve_call_target`（抽为 pub 单点，一处生产一处消费），
>     并以矩阵钉测试覆盖具名导入两形态。**留档**：init 段 `DefId` 空白经实证为「一致性空白」
>     而非活体缺陷（codegen 名字回退可解析、四类 init 调用实测通过），已登记 WBS 3.7.4
>     以收口「函数体走 def、init 靠名字」的口径分叉。
> 18. init 段 DefId 口径分叉收口（3.7.4，2026-10-11，38bd4b9e）：`assign_defs` 抽出
>     `resolve_static_targets` 单点（Call/TailCall 字符串操作数 + MakeClosure），函数体与
>     `ir.init` 共用同一实现与同一 `resolve` 闭包；208 个 IR 快照再生后仅 309 对
>     `(def=_)`→`(def=dN)` 变化（other-lines 0；全树 init 段 0 条残留 `(def=_)`）。codegen
>     的 DefId 主路径与名字回退两态下翻译结果与执行值逐项一致（四类 init 调用实测），
>     语料基线零漂移；探针按新口径翻转并加强（身份 + 精确索引 + 执行值，回退 init 覆盖
>     即致红）。**勘误（2026-10-11）**：上述提交号经 `git pull --rebase` 重写——原
>     b554497b/e60dcc87/b7ac7396 依次对应 rebase 后的 375789d2/d922a551/38bd4b9e；
>     已推送的 dev 历史以新号为准（旧号为本地临时号，未进入远程历史）。
> 19. 3.4.8 ③④ 落地与验证（2026-10-11，8ecf56f93）。（③④ 的设计依据见 #14；② 的落地见 #17，本条是其收口。）**实现**：`CHECK_STAGES` 尾部 = `…IrGeneration → Linking → Monomorphization`；Check 的 `Linking` 只做纯合并（聚合请求 + `link_module_irs`），入口校验整块留给 `RoleClassification`；`ir_generation_check` 生成后留存 `us.ir`（仅内存）；mono 失败在 Check 进收集通道、在 MultiFile 保持 `fail_with`；`into_outcome` 先 `take_check_diagnostics` 再判故障（结构故障不再清空已收集诊断）。**验证**：四轮独立验证（t3/t4/t10 + t5 跨形态矩阵）与一轮复审（t6）全 pass；语料差分 340/340 逐行 IDENTICAL、IR 快照零漂移、测试数 2643→2656。**过程中发现并同轮修的既有缺口**：F1（rootless 多文件 check 的请求名与 IR 定义键不同源 → 静默不特化，见 WBS 3.4.8 行的落地注记）——修法为聚合侧按真实定义键精确反查（非启发式），首版修法为聚合侧按真实定义键精确反查（`flat_defs`）；**同日勘误**：该桥已在回归修复中删除，根因改由 `module_key` 一职两用解耦（`script_mode`）解决，三者同源为 `{单元键}.{名字}`。该回归（CI 文档示例门禁 258 块 10 失败 → 0 失败）的定位与修法见 WBS 3.4.8 行的落地注记。**留档（既有、非本轮回归）**：E3005 help 文案与资源保护语义不符；multi-file E3005 的 span 标签错位（HEAD 二进制可复现）；请求收集面的类型参数位置依赖（同面归 3.5）。

> 20. 3.9.1 / 3.10.1 / 3.11.1 落地与诊断面收口（2026-10-12，809b9b936 + 8605a081b + 后续两笔）。
>     **3.9.1（执行层守卫可达）**：解释型调用改**显式栈驱动**——`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/
>     `SpawnFromList` 压帧后把控制权交还外层 `step_one`，帧携带 `ReturnTarget{Host|CallerSlot|Discard}`，返回时
>     写回调用者槽位，多调用指令用帧内 `pending_calls` 续跑。基线实测每层解释调用耗原生栈 **2544 B**、崩溃临界在
>     `count_down(394)`（Windows 主线程 1 MiB）；修复后 20 000 层递归可在 512 KiB 栈线程上跑通（每层约 0）。
>     守卫（`push_frame`）由此在任何入口、任何 `max_stack_depth` 取值下可达；`ExecutorError::StackOverflow` 改为
>     携带真实上限（原 `util/diagnostic/mod.rs` 恒传 0）。**原 3.11.2（深栈回显截断）经用户裁决并入本任务**
>     （同处该文件、按文件不可分）：回显改首 16 帧 + 计数省略 + 末 8 帧，浅栈逐帧不变。
>     **3.10.1（编译期无闸原生递归，#447）**：先定位后改——崩溃点为 `MonoType::type_name` 递归（**328 B/层**，
>     SP 探针 + 逐操作临界确定），同路径另有派生 `Clone`（~400 B/层）、`substitute` 内深拷贝、`type_name_hash`
>     （164 B/层）、`collect_generic_type_refs`；四处改工作列表并去掉每层一次整请求深拷贝后，`mono.max_depth=5000`
>     的病态泛型递归产出 **E3005（含真实上限）**，不再爆栈。
>     **3.11.1（任务边界错误码保真，#448）**：任务结果错误侧由值改**结构化**（`TaskResult = Result<SyncValue,
>     TaskFailure>`、`TaskOutcome::Err(TaskFailure)`），跨边界按值还原 `ExecutorError`；任务内 E6001/E6003/
>     E6004/E6008 在 Standard 模式与 Embedded 消息逐字一致。两条 **pub API 载荷形态变更**已在提交里写
>     `BREAKING CHANGE`。
>     **提交策略勘记**：3.9.1 与 3.11.1 **在文件粒度上不可分**——`interpreter/executor/executor.rs` 同时承载
>     蹦床与错误通道的生产/消费点，`TaskFailure` 定义在 `runtime/engine.rs`，只暂存任一侧都编译不过（pre-commit
>     以**暂存子集**编译，实测两次拒绝），故按 #15 的同一口径并为**一笔原子提交**。
>     **已知边界与后续**：Drop 派生（~75 B/层）⇒ `mono.max_depth ≳ 13000` 仍可能崩；E4012（常量求值）闸门承载
>     在原生递归上（每层 ~3 696 B、默认 256 层 ≈ 924 KiB），当前零调用点、生产不可达，将来接线须同族收口；
>     Standard 模式任务解释器按默认 `ExecutorConfig` 构造 ⇒ 其 E6004 上限恒为 1024（配置传播登记为 WBS 3.12，
>     用户裁决乙：另起队伍）；测试运行时预算另开 issue #449（病态夹具 76s、clippy 双 target、提交跑全量），
>     不属本 RFC 施工面。
>     **语料目录分层（同轮，8605a081b）**：`02-type-system`（78）与 `06-compile-errors`（134）共 212 个夹具拆入
>     23 个按类别划分的子目录，88 个 IR 快照按镜像路径同步搬迁；语料基线同提交再生（342 行，独立复算逐行比对：
>     仅 `file` 字段与 2 条新增变化），41 个快照因文件头路径变长而重新生成，逐行核对**只有 span 偏移变化**
>     （去偏移归一化后零差异）。
> 21. LSP 语义数据管线统一（4.6.1，2026-10-16）。诊断与语义数据由**同一
>     次编译**产出，项目内跨文件引用携带 `resolves_to` 指向真实定义——
>     旧形态是 didOpen/didChange 触发**两趟**编译：诊断走统一 Driver 管线，
>     语义数据走 server.rs 的手工 lex→parse→单文件 typecheck（无模块注册表，
>     项目内 `use` 解析失败、引用事件不产生，跨文件跳转/重命名/引用查找
>     静默落空——D55 登记的静默失败在本任务收口）。**实现**：(1) `Export`
>     携带 `definition_span`（extract_module_info 自 AST 标识符节点收割，
>     与 typecheck 定义事件的 `def_span` 同源同值；native std 恒 None），
>     `ModuleInfo` 携带 `definition_file`（Registry 臂按 DiscoveredFile
>     填）；(2) checker 排空引用事件时经 `import_def_locations` 把跨文件
>     导入项的 `resolves_to` 从 use 行重定向到定义文件的真实位置（std
>     导入无登记，维持落 use 行）；(3) checker 模块身份由统一 `"<module>"`
>     改为单元文件路径（该值只消费于语义数据面，零诊断消费——实证
>     `env.module_name` 全部 4 个消费点）；(4) `DriverOutcome` 新增
>     `semantic_db` 通道——CollectAll 形态（Lsp 目标文件无论诊断成败 +
>     SingleFile 兜底）汇入，Lsp 臂尾从 registry 注入非目标文件的顶层
>     导出定义面（`external_definitions`，未打开的文件也能跳转；类型/
>     子模块导出不注入——typecheck 侧类型注册不产生定义事件，两侧保持
>     一致的空缺）；(5) orchestrator 新增 `analyze_source_in_project`
>     （`check_source_in_project` 转调并只取诊断半边）；(6) LSP 侧
>     `analysis.rs` 单编译入口取代手工管线（server.rs 的
>     `update_semantic_db` 删除），World 键统一为规范化 file URI
>     （`uri.rs`：`path_to_uri(uri_to_path(u))` 幂等归一，盘符大小写
>     与 %XX 编码差异归一到同键）。**顺路修（D59 分流 B 类——旧行为
>     静默不动作，不改变任何原先成功程序的对外表现）**：项目内文件的
>     跨文件跳转/重命名/引用查找自静默落空转为命中真实定义；词法错误
>     时旧手工管线删除语义信息改为残缺 AST 染色保留（4.2.6 编辑器哲学
>     延伸）；目标文件带类型错误时语义数据不再随 `type_result` 一并
>     丢弃（染色/跳转在打字中间态不消失）。**钉板**：driver 级 6 枚
>     （目标文件路径键/跨文件 resolves_to+外部定义面往返/std 导入回退
>     use 行/错误文件保留/单文件兜底路径键+错误保留）、LSP e2e 1 枚
>     （先红后绿实证——修复前跳转返回 None）、uri 规范化 4 枚。
**十个入口的改造方式（逐个指定函数）**：

| 入口 | 改造后 |
| --- | --- |
| `Pipeline::run`（`pipeline.rs:141-227`） | 删函数体，改为构造 `Program { kind: SingleFile, … }` 交 Driver |
| `Compiler::compile`（`compiler.rs:95`） | 不变（已是无状态的包装） |
| `compile_project`（`orchestrator.rs:99-237`） | 瘦身为 `Program { kind: MultiFile, aggregation: FailFast }` 构造器 |
| `check_project`（`orchestrator.rs:273-399`） | 瘦身为 `Program { kind: Check, aggregation: CollectAll }` 构造器 |
| `check_source_in_project`（`orchestrator.rs:450`） | 改为 Driver 的一次调用 + 结果过滤到目标文件 |
| `compile_embedded_module`（`orchestrator.rs:1374`） | 改为 `Program { kind: Embedded, units: [embedded] }` |
| `lib::run_file` | 不变（转调 `run_project` 或 `SingleFile`） |
| `lib::run_project` | 不变（转调 Driver） |
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
| `yaoxiang run app.yx`（单文件） | 5 阶段 | 同 5 阶段 + `assert_drained()` 结算（**4.10.3 修订，2026-10-15**：CLI `run` 对 standalone 不再走本管线——统一改走 `compile_project`（MULTI_FILE 阶段表），Script 语义由 rootless 驱动；本行现仅描述 `lib::run_file` / `Compiler::compile` 库路径） |
| `yaoxiang run`（多文件） | 无 proof_execution、无 W1001/W1002/W1003 | proof_execution 已在 P3 止血接入；**死代码族警告与 W1003 / W1004 / W1005 / W1006 现经同一判据进入 MultiFile `run` 路径**——多文件 `run` 的 W 码集合与单文件 `check` 归一（`dead_code_analysis` 臂按「非 pipeline 通道」分派，Check / Lsp / MultiFile 共用同一实现，无第二份判据；`typecheck_orchestrated` 的 FailFast 分支暂存 `result.warnings` 交该臂外发） |
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
| `src/frontend/core/typecheck/checker.rs` | `TypeChecker::ownership_inputs` 判据点 | `unwrap_or_default()` 降级路径加 warning 诊断 |
| `src/frontend/core/typecheck/checker.rs` | `1244` | 修正与 `1320` 不符的注释 |
| `src/frontend/core/typecheck/layers/README.md` | 全文 | 层序表改为**实际**执行顺序；删除「下层失败上层不跑」（不存在 short-circuit） |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72` | **按 D58**：新增进程级共享单例（`LazyLock<Mutex<Option<Box<dyn Solver>>>>`）+ `with_shared_solver` 闭包访问口；`predicate.rs`/`checker.rs`/`ownership.rs` 三消费点改走共享口。`&'static` 裸引用形态不可行（`dyn Solver` 不 `Sync`），原文表述以 D58 为准 |

**删除**：`src/util/diagnostic/mod.rs:621-661` 的 `check_single_file`；`src/lsp/handlers/diagnostics.rs:161-227` 的手工 lex/parse 序列。

**不动**：`Unproven` 分支的生产端逻辑（现分居 `checker/refinement.rs` 的 `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` 与 `checker.rs` 的 ownership 结果处理臂；原 `checker.rs:5164/5179/5306/5318/5420/5448`，P5 5.2.1 搬迁）——逻辑本身正确，**问题在消费端不在生产端**，改生产端会掩盖架构缺陷；`Cargo.toml`；任何 `#[cfg(target_arch = "wasm32")]` 分支（wasm 分支的可达性判定归 `06-cleanup-inventory.md`）。

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
| `src/driver/tests/obligations.rs` 骨架（先 `#[ignore]`） | 标 `#[ignore]`，S4 转绿（红态留档：摘除唯一结算点后 `pairs: 0`） |

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
| ownership 层 `Unproven` 空 match 臂修复（原 `checker.rs:1313`） | `test_no_silent_pass_on_unproven`：`Unproven` 在任一模式下必产诊断 |
| `predicate.rs:34-36` `.expect()` 改为诊断 | 移除 Z3 环境下不再 panic |

**回滚点**：漏洞修复与结构合并**分两个 commit**。若结构合并出问题，可只回滚结构 commit、保留漏洞修复 commit——此时 `test_multifile_proof_obligation_not_dropped` 保持绿；反向（保留结构、回滚修复）会转红，是不可接受的中间态，禁止合入。

### S4：启用义务账本

| 交付 | 验收 |
| --- | --- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 条用例转绿（实施形态见 §2 修订注记；条数以该文件为准） |
| `scripts/ci/check-obligations.py` | 静态门禁生效 |
| 义务诊断 W → E 升级 | 全语料差分后人工 review 每一处新增 E |

**回滚点**：`assert_drained()` 的严重级可由配置项控制，W/E 切换不需要改代码结构。

### S5：证明层与 wasm 收尾

| 交付 | 验收 |
| --- | --- |
| `layers/README.md` 层序改为实际顺序 | 文档与 `checker.rs` 调用点逐条对应 |
| `backend.rs` 进程级共享单例（D58；**硬前置：Unknown 不入缓存**——否则一次超时经进程级缓存跨编译固化、且污染 cargo test 线程间共享） | 生产路径 `default_solver()` 调用点归零（仅测试保留）；缓存命中率跨三消费点可观测（计数器已在 89576fafd 落地） |
| `checker.rs` 的 `TypeChecker::ownership_inputs` 判据点（所有权块 `:1296-1340`、发射点 `:1531`）降级加 warning | 无 `body_checker` 时有诊断 |
| `orchestrator.rs` 10 条 wasm 属性（4 正 + 6 反）的可达性判定 | 结论交 `06-cleanup-inventory.md` 的「F2 判定」小节，本文只登记判定需求 |

**注意**：修正层序会改变诊断集、可能暴露大量此前被静默的 `Unproven`。**这一项应独立于 S1-S4 走**，不要与入口合并混在同一个 PR。

## 关键决策与理由

| 决策 | 决定 | 理由 |
| --- | --- | --- |
| **阶段模型** | `Stage` 为 12 变体的编译期可穷举枚举，**拒绝运行期注册** | 穷尽 `match` 是零成本的编译期强制：新增变体时 `dispatch` 编译不过。这与 opcode 表在 RFC-039 路由表 B 中已被验证的机制同构。`StageScope` 进一步把「逐模块还是项目级」变成类型事实，消除需要人推理的调用次数问题 |
| **义务机制** | `Obligations` + `assert_drained()` 做**运行期**结算，与 `scripts/ci/check-obligations.py` 的**静态**门禁并用；**严重级终态 E 级**（2026-10-13 用户裁决甲：同码升 E、码号不变；**实施形态见 §2 修订注记**：账本不复制字段结构；红名单——清账后 0 条——机制保留、不进用户可见诊断流） | 修的是一整类 bug，不是一个 bug：当前已核实同类隐患至少 2 处（`proof_calls`、`checker.rs:1313`），13 个义务字段全在射程内。W→E 分两步是为了让每一步的 C2 判据都可用——如今义务清单已清账到 13 条、红名单归零，健康态零发射，故终态直接取 E（见 §2 修订注记第七、八段） |
| **修复范围** | 修**消费端**（编排层），**不动** `Unproven` 分支的生产端逻辑（现分居 `checker/refinement.rs` 三处与 `checker.rs` 一处；原 `checker.rs:5164/5179/5306/5318/5420/5448`） | 问题在消费端不在生产端。改生产端会把架构缺陷掩盖成"逻辑修好了"，而这三处分支的逻辑本身是正确的 |

这样设计额外消除两类裂缝：

- **「注释承诺」与「代码行为」的裂缝**。`checker/refinement.rs`（原 `checker.rs:5165`）的「不得让 silent pass 复活」是**注释级契约**，靠 `assert_drained()` 兑现。
- **「一个数组忘记 append」**。当前是五个函数各自手写调用序列；统一后是唯一一张阶段表，且新增阶段漏接会在编译期报错。

### 未采纳的方向

- **泛型阶段链 `Stage<A, B>`（`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`）**——11 处不一致中有 7 处（死代码分析两套实现、W1006、W1003、单态化、角色上下文、IR 生成/链接）**不涉及类型衔接**，是「要不要跑某个分析」的配置问题；它也表达不了 `Aggregation`，只能把 `check_module` / `check_module_collect_all` 的二选一从函数层搬到类型层。
- **`dyn Stage` + 运行期注册（`driver.register(Box::new(...))`）**——取消编译期穷尽性：新增阶段时 `dispatch` 不再编译失败，等于把「五个函数各自手写调用」换成「一个数组忘记 append」，正是当前 11 处不一致的成因。本仓库已有同形态反例：`docs/src/dev/design/check/` 描述的 `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` 全部零实现。
- **只补 bug 不动结构**——修得掉已知的 2 个 bug，但只覆盖义务字段中的 1 个，`Stage` 仍散落在 5 个函数里，下一个分叉点会继续从这里长出来。**它必须先做**（是 S1/S3 的一部分），因为结构改造需要一个已知红测试来证明判据有效。
- **把 `proof_calls` 改 `pub` 并加 `debug_assert`**——`check_module` 是通用入口，它不知道调用者是谁，`debug_assert!(<caller will handle>)` 无法成立；`#[must_use]` 只在字段被**整体**丢弃时警告。这是在错误的层面找 bug：bug 在编排层，检测必须在编排层。

## 已知局限与风险

- **`orchestrator.rs` 的瘦身在短期内会让它更难读**。139 行的 `compile_project` 拆成「Program 构造器 + 若干 driver 臂」后，读者需要跨两个文件才能理解流程。这是所有"把接线集中起来"的重构的共同代价。
- **S3 会显著改变诊断集，且改变的方向是「暴露此前被静默的问题」**。修复后可能出现一批"新错误"的用户报告——它们是真实存在的 bug，只是一直没被报出来。必须在 CHANGELOG 中明确说明。
- **`assert_drained()` 的严重级切换需要逐字段人工判断**。义务字段里有些（如 `module_namespaces`）的"无消费者"可能是设计上就不需要消费，不应报警。S4 需要逐字段过一遍，不能一刀切。
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
- `src/frontend/core/typecheck/checker/refinement.rs` — 「不得让 silent pass 复活」的明文声明（原 `checker.rs:5165-5169`）
- `src/frontend/core/typecheck/layers/README.md` — 层序调用表与 `equivalence` 的真实身份（无 short-circuit）
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — SMT 初始化硬失败的明文理由
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — 「调用方应保守降级」的契约声明（与 `predicate.rs` 矛盾）
- `src/frontend/core/typecheck/types.rs:16-76` — 13 个下游必须消费的义务字段（3.4.4/3.4.5/3.4.9 清账后）
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — 单测自认「停在 pipeline 之前」
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` — 语料自认「由 pipeline.rs 在 check_module 之后执行」
- `wasm/src/lib.rs` 的 `compile_playground`（`run_code` 经它编译）— playground 的第四个编译调用方（单文件路径，故执行 proof_execution）
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen 在 shim crate 而非主 crate
- `build.rs:19-55` — 错误码构建期门禁，本项目强制机制的范例
