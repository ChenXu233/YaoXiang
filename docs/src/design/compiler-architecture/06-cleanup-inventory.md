# 死代码与空头设计清理

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../rfc/draft/039-compiler-architecture.md) 的附属文档。四层模型、验收判据分级与执行阶段顺序见 RFC-039 正文;各附属文档的定位见 [本目录索引](index.md)。

## 定位与范围

本文给出 YaoXiang 仓库的死代码与空头设计清单、判定方法、逐项处置决定与执行顺序。清理对象四类：

| 类别 | 规模 |
| --- | --- |
| 从未参与编译的文件 | 24 个文件 / 2628 行（其中真实孤儿 23 文件 / 1081 行） |
| 已编译但零调用的死代码 | 1500+ 行 |
| 描述了从未存在架构的设计文档 | `docs/src/design/check/` 三篇 |
| 挂在默认入口、零测试、零输出的功能 | `src/repl/` 1152 行 |

**授权范围是"允许彻底删除"**（2026-10-03 决策）。但本文对其中若干项给出**与"直接删除"相反的处置建议**——理由见 [关键决策与理由](#关键决策与理由)。

本文**不覆盖**：模块边界与依赖方向（见 `01-routing.md`）、阶段契约（见 `02-stage-contract.md`）、判据设计（见 `07-equivalence-oracle.md`）。B3 记录的 `layers/dispatch.rs` 整链未接线，其删除动作归入 S2。

## 现状

### 死代码的三种负债形态

**形态 1：占位。** 代码存在、看起来在做某件事、实际不做。

- `src/frontend/core/typecheck/layers/dispatch.rs` 整条链（`dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`）在编译器里**零调用**。它是 `RFC-027 §4.1` 设计的"运行时检查插入"机制，**完全未接线**。
- `src/frontend/core/typecheck/layers/equivalence.rs:216` 的 `check_type_equivalence`（`ProofContext` 入口）仅被测试调用。**注意**同文件的 `is_subtype` 被 `inference/assignment.rs:17` 生产使用，因此**不能整文件删除**。
- `src/backends/runtime/engine.rs`（1047 行）是纯 DAG 任务调度器，`pub` 面只有 `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` / `RuntimeStats` / `RuntimeError` / `LocalRuntime`。它**不是第二个执行后端**——`backends/mod.rs:356` 的 `Executor` trait 当前只有一个实现者（解释器）。这一点需要澄清，因为目录名容易误导。

**形态 2：残留。** 上游决策已作出，但删除做了一半。

- `src/middle/passes/mono/instance.rs:416-830`（**415 行**）残留 6 个类型：`FunctionInstance`（416）、`GenericClosureId`（562）、`ClosureId`（628）、`ClosureInstance`（724）、`ClosureSpecializationKey`（760）、`CaptureVariable`。
  **来源明确**：`RFC-023` 废弃"闭包捕获模型"时声明「**随闭包模型一并删除约 850 行代码**」。这 415 行就是那 850 行里活下来的残余——**删除只做了一半**。
  **核实**：对这 6 个类型做全仓词边界搜索，`instance.rs` 之外命中数**全为 0**（含测试）。`middle/mod.rs:40` 的 `pub use passes::mono::instance::*;` 只是 glob 转手，不构成消费。

**形态 3：假覆盖。** 这是最危险的一种——**它让仓库看起来比实际更可靠**。

`src/frontend/core/lexer/mod.rs` 只有 5 个 `pub mod`（`literals` / `state` / `symbols` / `tokenizer` / `tokens`）+ 一个 `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`（`:104-106`）。**没有任何地方声明 `mod tests;`**，因此 `lexer/tests/` 整棵子树 13 个文件 686 行**从未参与编译**。

其中 7 个是空壳（`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 / `delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3，合计 19 行、0 个断言）。**629 行 / 55 个真实测试从未运行**：

| 文件 | 行 | `#[test]` |
| --- | --- | --- |
| `literals.rs` | 222 | 30 |
| `rfc010_lexer.rs` | 167 | 12 |
| `lexer_mod.rs` | 89 | 5 |
| `rfc004_lexer.rs` | 81 | 5 |
| `symbols.rs` | 70 | 3 |
| **合计** | **629** | **55** |

（`fstring.rs` 90 行 / 11 test 确实在编译，但走的是 `#[path]` 旁路而非 `mod tests;`；`mod.rs` 38 行是声明文件，无断言。）

**词法层实际在跑的只有一个文件、12 个测试，且全部只测 f-string 的花括号 raw 协议。**

这类问题的隐蔽性在于：`cargo test` 不会失败（没有跑过的测试不会失败），CI 也没有任何测试数量基线或接线检查。

### 空头设计

`docs/src/design/check/` 下的三篇文档描述了一套**从未存在**的架构。这不是"设计好了没做"，而是**文档描述的是另一套从未存在的系统**，且把"未来可能的设计"写成了"已实现系统的缺陷"。

## 方法与可信度声明

### 可达性分析方法

从 `src/lib.rs` / `src/main.rs` 两个 crate root 出发，解析全部 509 个 `.rs` 文件的 `mod` / `pub mod` / `#[path]` 声明，构建模块图并做 BFS 可达性分析。`include!` 目标单独判定（不计入可达性）。

**该方法有已知的误判模式，写作过程中实际触发过两次：**

1. `Split-Path` 在 Windows 返回 `\`，与正斜杠键不匹配 → 一度误报 499 个孤儿
2. **非 `mod.rs` 文件的子模块目录是 `自身名/` 而非 `父目录/`** → `orchestrator.rs` 的 `mod tests` 被解析成 `src/frontend/module/tests/`，差点把 753 行活的测试判成死代码

因此**每一条结论都经过「实际读文件 + 实际搜引用」双重确认**，未通过二次确认的条目不进入本文。

### 行数口径

本文所有行数用 `(Get-Content).Count` 统计。**注意 `Measure-Object -Line` 会跳过空行、系统性少算约 5%**（例：`manifesto.md` 用 `-Line` 报 658，实际 893）。本仓库早前的一轮分析因该口径给出了偏低的总数，此处已更正。### 已知局限

- **可达性分析只能发现"不可达"，不能发现"可达但无意义"**。形如 2 之外的形态需要逐项人工判断。
- **`pub` 项在 Rust 中不触发 `dead_code` 警告**，因此零调用的 `pub` 类型需要靠搜索发现，本方法不覆盖。
- 本文 不涉及运行编译器，因此所有"从未运行"类结论均为静态判定。

## A. 从未参与编译的文件

**总计 24 个文件 / 2628 行。** 其中 `semantic_tokens.rs` 属形态 4（见 F 节），其余 **23 个文件 / 1081 行是真实孤儿**。

| 路径 | 规模 | 内容 | 判定依据 |
| --- | --- | --- | --- |
| `src/frontend/core/lexer/tests/` **整棵子树 13 文件** | 686 行 | 7 空壳（19 行）+ `mod.rs` 38 行 + **629 行 / 55 个真实测试** | `lexer/mod.rs:4-8` 只声明 5 个 `pub mod`；`:104-106` 唯一测试声明是 `#[path]` 到 `tests/fstring.rs`。全目录 `grep 'mod tests'` 只命中 `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 文件 | 14 行 | 全是占位文档注释，0 个断言 | `src/frontend/pipeline.rs`(740 行) 全文无 `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` 声明（`Select-String` 零命中） |
| `src/package/template/tests/` 3 文件 | 65 行 | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 个 `#[test]`** | `src/package/template/mod.rs`(7 行) 只声明 `mod gitignore; mod main_yx;`，无 `mod tests;` |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | 95 行 / **6 test** | 藏在**活的**目录中 | `pratt/tests/mod.rs` 只声明 `led` / `nud` / `precedence` |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 行 / **7 test** | 同上 | `passes/tests/mod.rs` 只声明 `dead_code` / `overload` |
| `src/util/diagnostic/emitter/tests/json.rs` | 46 行 / **3 test** | 同上 | `emitter/tests/mod.rs` 只声明 `ansi` / `text` |

**最后 3 项合计 16 个测试 / 311 行代码从未运行。** 它们藏在活跃目录中，目录的 `mod.rs` 看起来是完整的，因此比整棵孤儿目录更具欺骗性。

其中 `lexer/tests/` 内另有两个文件同属"被声明但漏声明"形态，但已计入上表第一行的 629 行内，**不重复计数**：`lexer_mod.rs`（89 行 / 5 test，`lexer/tests/mod.rs` 声明 11 个 `mod` 但无 `mod lexer_mod;`）、`symbols.rs`（70 行 / 3 test，同上）。

**从未运行的真实测试合计 1005 行 / 78 个**：lexer 629/55 + pratt / passes / emitter 311/16 + package/template 65/7。

**特别注意 `precedence_inline.rs`**：它测的是 `precedence.rs` 里那套**生产零使用**的 `Precedence` 枚举 + `PrecedenceContext`（96 行死代码）。即使把它接上，它测的也是死路径。

## B. 已编译但零调用的死代码

| # | 位置 | 规模 | 证据 |
| --- | --- | --- | --- |
| B1 | `src/middle/passes/mono/instance.rs:416-830` | 415 行 / 6 类型 | 全仓词边界搜索，`instance.rs` 外命中全为 0（含测试）。来源：RFC-023 声明删除约 850 行，只删了一半 |
| B2 | `src/frontend/core/typecheck/inference/types.rs` | 45 行 | `TypeSystem` + 4 个纯函数。唯一引用链是 `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;`（都是转手）。全仓 `TypeSystem` 命中仅 5 处：定义 2 + re-export 1 + **无关的** `lexer/symbols.rs:503 TypeSystemValidator` 2。4 个方法零调用点。**因为是 `pub`，编译器永不报警** |
| B3 | `src/frontend/core/typecheck/layers/dispatch.rs` 整链 | — | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` 零调用。RFC-027 §4.1 的运行时检查插入未接线。注意它被 `layers/mod.rs:6` 声明 |
| B4 | `src/frontend/core/typecheck/layers/equivalence.rs:216` | — | `check_type_equivalence` 仅测试调用。**同文件 `is_subtype` 必须保留**（`inference/assignment.rs:17` 生产使用） |
| B5 | `src/frontend/core/typecheck/layers/termination.rs:959` | 策略 2 | 自述为"框架占位" |
| B6 | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133` | 96 行 | `Precedence` 枚举 + `PrecedenceContext`。**其唯一的外部引用者是 `pratt/tests/precedence_inline.rs`（95 行 / 6 test）——而该测试文件本身从未被编译**（`pratt/tests/mod.rs` 只声明 `led`/`nud`/`precedence`）。因此二者**必须同时删除**：单独删测试会留下 96 行死代码，单独删枚举会造成测试编译失败 |
| B7 | `src/frontend/core/parser/statements/declarations.rs:82`、`:120-122`、`:145-167` | 约 120 行 | `is_old_function_syntax` 在 `:719` 被调用、`skip_old_function_syntax` 在 `:725` 被调用——**它们是运行中的旧语法拒绝门禁，不是无引用死代码**。`skip_old_function_syntax`（`:120-122`）函数体只有一行注释 `// 旧语法已移除`，但**该函数被调用后不消费任何 token**——这是一个独立的行为问题（拒绝路径是否正确推进解析位置），不是死代码。**处置是随 05-frontend-paradigm.md 一并审查行为（决议 D27：随文法迁移自然删除），而非当作死代码直接删** |
| B8 | `Instruction::TailCall`（`src/middle/core/ir.rs`） | — | 全仓只有 match、无构造点。`bytecode.rs:2156` 的"无生成点（死路径）"注释**准确**（`translator.rs:530`/`1109` 的编码分支存在但不可达） |
| B9 | `Switch` opcode | — | `backends/common/opcode.rs` 已定义、有变体、有 `opcode()`/`size()` 臂、有解码臂，**但全仓零构造点**。解释器 `ops/control.rs:90` 有活跃实现。**已被测试显式登记为已知死路径**：`tests/bytecode.rs:1035-1040` 的 `WHITELIST: &[&str] = &["SWITCH"]`（差集非空即失败） |
| B10 | `UnaryOp::Not`（`bytecode.rs:101`） | — | `opcode()` 在 `:590` 用 `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` **忽略 `op` 字段**，解码器 `1236-1248` 也只构造 `Neg`。**经 `opcode()` 映射路径静默退化为 I64_NEG**（`.42` 直跑时 `!x` 变 `-x`；解释器内存路径 `ops/arith.rs:46-49` 反而正确）。仅由解释器 `ops/arith.rs:46,49` 与 `tests/logical_not.rs` 使用 |
| B11 | `#[allow(dead_code)]` 4 处 | — | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix`（这两处是真实死代码）；`proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown`（extern 绑定的保守豁免，**合理保留**）；`std/tests/stdlib_docs.rs:250` `docs_path_for`（测试辅助） |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef` | — | **生产零构造**（唯一构造点是 `parser/tests/ast.rs:828` 测试手工造节点），但有 **12 处生产消费 + 2 处穷举臂**（完整清单见 03-type-unification.md §2.6）：`spawn/placement.rs:122`、`spawn/analysis.rs:912`、`formatter/handlers/expr.rs:43`、`frontend/module/orchestrator.rs:1492`、`ir_gen.rs:4325/5097`、`checker.rs:1642`、`semantic_tokens.rs:1416`、`inference/expressions.rs:3374`、`inference/existential.rs:55`、`passes/dead_code.rs:305`、`layers/ownership.rs:1207`、`layers/termination.rs:752`，另有 `ast.rs:1026` `Expr::span()`、`pratt/mod.rs:47` `expr_end_line`。函数定义实际走 `Expr::Lambda`(`nud.rs:425-429`) + `StmtKind::Assign`(`declarations.rs:462-479`)。删除归 P6（03 §5.5 的 16 项改动清单） |

## C. 空头设计

| # | 对象 | 规模 | 事实 |
| --- | --- | --- | --- |
| C1 | `docs/src/design/check/incremental-checking.md` | 53 行 | 描述的 `CheckSession`（含 `:31-42` 的 Rust 代码草稿）、`ModuleDependencyGraph`、`affected_modules`、`ModuleCache`、`HotReloader` **全仓零命中**。`:46-47` 的"已知限制"自承 `check_incremental` 内部仍调 `check_files_with_diagnostics`（全量路径）；`:46` 还引用 `command.rs` 的 busy-wait 防抖——**`command.rs` 中既无 `Instant` 也无 `recv_timeout`**（全仓 `recv_timeout` 只出现在 `backends/runtime/tests/facade.rs`） |
| C2 | `docs/src/design/check/cross-file-analysis.md` | 43 行 | `:37` 声称 `traits/` 占位实现（coherence / impl_check / object_safety / resolution）未完成——**`traits/` 目录不存在**（`Test-Path` = False），且这些标识符全仓零命中。`:38` 声称 `check_single_module` 为每文件建独立 Compiler——**该函数不存在**，只在 `util/diagnostic/tests/mod_tests.rs:248` 的注释里出现 |
| C3 | `docs/src/design/check/index.md` | 29 行 | `:14` 声称"watch 模式只重检查受影响的文件"、`:27` 声称"check 只做前两步"。实际：`check_project` 还做 vendor 一致性、项目角色判定、死代码分析、W1006；`main.rs` 的 `Check` 子命令**没有 `--watch` 标志**（已逐个核对全部 `#[arg]` 声明） |
| C4 | `RFC-018` LLVM AOT | 1037 行设计 / **0 行代码** | `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` 的 AOT 命中**全是注释**。`Cargo.toml` 无 cranelift / llvm / inkwell。`:861-876` 自己留了一份全 `- [ ]` 的实现清单，但状态标 `accepted` |
| C5 | `RFC-028` JIT | 419 行设计 / **0 行代码** | `028-jit-compiler.md:99-126` 要求 `src/backends/jit/` 6 个文件 + `middle/passes/ir_normalize.rs`，**全部不存在**。状态是 `draft`（较诚实） |
| C6 | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 各 3 行 | **物证级证据**：文件名精准对应 C1 设计中的 `ModuleCache` 与增量调度。**有人按设计建了测试骨架，发现管线里没有对应实现，就搁下了** |
| C7 | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md` | — | `util/diagnostic/tests/mod.rs:4` 的 `//! §6.1: CheckSession 增量检查` 与 `mod_tests.rs:4` 的 `§4.4` / `§5.2` 引用的是这份**被 gitignore 的计划文档**（`.gitignore` 含 `superpowers/`）。这些 § 引用是**对已废弃 API 的考古层引用** |

### 跨文件分析的真实实现位置

C1-C3 描述的跨文件分析能力**是真的存在**的，只是路径完全不同：

| 组成 | 位置 | 行数 |
| --- | --- | --- |
| 模块注册表 | `src/frontend/module/registry.rs` | 439 |
| 解析器 | `src/frontend/module/resolver.rs` | 195 |
| 角色分类 | `src/frontend/module/roles.rs` | 388 |
| 一致性检查 | `src/frontend/module/consistency.rs` | 206 |
| 编排 | `src/frontend/module/orchestrator.rs` | 1528 |
| **合计** | | **约 2670** |

因此 C1-C3 的处置**不是删除能力，而是删除误导性文档**，并把 `check/index.md` 的边界表改写为指向 `src/frontend/module/` 的实际架构。

### RFC-019 的处置需要澄清

`docs/src/design/rfc/deprecated/019-typed-homoiconicity.md` **已在 `deprecated/` 目录**，且残留核查结果为**干净**（`homoicon` 全仓零命中，文档末尾有诚实的"若未来重启，从证明/原型实验入手，不要从 RFC 出发"结论）。**它不需要额外处置**——这一项在授权清单里，但核查后确认已是终态。

## D. 功能级死代码：REPL

`src/repl/` 4 文件 **1152 行**，**零 `#[cfg(test)]`、零 `#[test]`**。而 `src/main.rs:440` 是 `args.command.unwrap_or(Commands::Repl)`——**直接执行 `yaoxiang` 不带参数就进入 REPL**。

| # | 事实 | 证据 |
| --- | --- | --- |
| D1 | **`EvalResult::Value` 从未被构造** | 全 `src/repl/` 只出现 2 次：`eval.rs:23` 的定义、`mod.rs:180` 的 **match 臂**。因此 `Repl::format_value`（`mod.rs:493-502`）是死代码，**REPL 不回显任何求值结果** |
| D2 | **无跨轮状态** | `extract_definitions`（`eval.rs:345-377`）只调 `define_variable`（存**类型字符串**），从不调 `define_var`（存 `RuntimeValue`）；`wrap_code` 也不注入历史定义。`VariableInfo::Value`（`eval.rs:23`）的分支不可达。`REPLContext.variables` 纯粹服务补全，不参与求值 |
| D3 | **包装语法疑似过时** | `eval.rs:314`/`316` 产出 `main() -> () = () => { code }`（`name() -> Ret` 旧式函数声明头），而当前规范形态是 `main: () -> Void = { }`（`tests/yaoxiang/00-smoke/hello.yx:12`，RFC-007:94 的规范表全是 `name: (a: Type) -> Ret`）。**未运行编译器实测；若成立，REPL 每次求值都编译失败** |
| D4 | `:debug` 是桩 | `mod.rs:454-466` 只打印两行提示就返回；breakpoints 存了（`mod.rs:319`）但**解释器从不查询** |
| D5 | `:history` 是桩 | `mod.rs:387` 打印 "not yet implemented" |
| D6 | 与 `lib.rs` 重复 | `lib.rs:85` `eval_code` 与 `repl/eval.rs:wrap_code` 是**两套独立的"自动包装 main"逻辑**，包装形态还不一样 |

## E. 局部清理项

| # | 位置 | 处置 |
| --- | --- | --- |
| E1 | `undefined/temp/33-calib/` | 根级空目录树。几乎确定是某处把字符串 `"undefined"` 当路径拼接写进去了。**直接删** |
| E2 | `src/lib.rs:568`、`:582` | `dump_type_detail` / `dump_const_detail` 各有一个 `_ => todo!()` panic 路径。`ConstValue::LibraryRef \| ExternRef` 是 FFI 相关，panic 路径可预知。应改为有意义的降级输出 |
| E3 | `src/frontend/module/consistency.rs:71-135` vs `:142-199` | `check_vendor_lock_consistency` 与 `check_workspace_consistency` **约 60 行近重复**，仅依赖来源与 path-dep 跳过两处不同。**合并** |
| E4 | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167` | 旧函数语法探测与拒绝，约 120 行。其中 `skip_old_function_syntax`(`:120-122`) 函数体只有一行注释。**随 05-frontend-paradigm.md 一起删** |
| E5 | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133` | 96 行死代码（见 B6）。**随 05-frontend-paradigm.md 一起删** |
| E6 | `docs/src/.vitepress/config.js:259-268` | "工具设计"侧边栏显式列出 `check/diagnostic-system`、`check/cross-file-analysis`、`check/incremental-checking`。**若执行 C1-C3 的文档删除，此处必须同步清理**，否则文档站出现 404 链接 |

### 不是死代码，但同类问题：三条硬编码丢弃

`src/middle/core/bytecode.rs` 的 `impl From<BytecodeFile>`（`943-2350`，1400 行）有 3 处硬编码丢弃：

| 行 | 丢弃内容 | 后果 |
| --- | --- | --- |
| `2312` | `upvalue_count: 0` | upvalue 元信息丢失 |
| `2315` | `exception_handlers: Vec::new()` | **`.42` 产物丢失异常表** |
| `2341` | `globals: Vec::new()` | **`.42` 产物丢失全局变量信息** |

**这三处不是死代码，是数据丢失缺陷**——`.42` 产物在内存路径下才是完整的。**处置方式与死代码不同**：应作为独立 issue 跟踪。`2312` 随 P7 修复并按 D17 把 `.42` 的 `VERSION` 从 4 升到 5（格式头已有 `MAGIC` + `VERSION` 字段，`codegen/bytecode.rs:14-16`）；`2315` / `2341` 同样走升版路径，归 04-ssa.md 登记的独立 issue。此处列出仅为避免被误当作"死代码"清理掉。

## F. 不删、但要标记或决策的

| # | 对象 | 处置 |
| --- | --- | --- |
| F1 | `src/frontend/core/typecheck/checker/semantic_tokens.rs`（1547 行） | **不是死代码**，是 `checker.rs:5618` 通过 `include!` 拼入的——**全仓库唯一的一处 `include!`**。该文件**没有被声明为任何模块**（`grep 'mod semantic_tokens'` 全仓零命中），第一行直接是 `impl TypeChecker {`，无自己的 `use` 头，`checker/` 目录下只有它一个文件。后果：没有模块身份、可见性隔离失效、rust-analyzer 跳转与符号搜索失效、工具链把它算作 `checker.rs` 的一部分。**`checker` 模块真实体量是 5618+1547 ≈ 7165 行**。**改法：改为真 `mod`**（施工步骤归 [09](09-execution-wbs.md) §P5 5.1），不是删除 |
| F2 | 27 个文件的 `#[cfg(target_arch = "wasm32")]` 分支 | **不是死代码。** wasm 目标由**独立 shim crate** 承载：`wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`、`:18` `wasm-bindgen = "0.2"`、`:17` 依赖主 crate；根 `Cargo.toml:3` 的 `exclude = ["wasm", ...]` 正说明它是独立 crate。只看根 `Cargo.toml:31` 的 `crate-type = ["rlib"]` 会误判这些分支为防御性死代码——**该推断不成立**。CI 有 4 处构建（`_build-wasm.yml:75`，调用方 `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`）。**这 27 个文件的分支是承重的**，它们决定 `wasm/src/lib.rs`（73 行）在 wasm 目标下能调用主 crate 的哪些 API（`lib.rs:139/153/170` 把 `run_file` / `run_project` / `build_bytecode` 整体门控掉）。**真正的清理目标**是其中不可达的那部分——`wasm/src/lib.rs` 只调 `Compiler::new` / `compile_with_source` / `std::io::wasm_output` / `CodegenContext` / `Interpreter`，`orchestrator.rs` 的 12 个 wasm 属性中哪些在 playground 场景不可达，需逐一判定（决议 D35：删不可达者、保留可达者，归 P10） |
| F3 | `docs/src/design/rfc/TRACKING.md` 的列结构 | 只有「状态」列（文档状态），**没有「实现状态」列**——它**无法表达**"RFC 标记为完成但代码没有"这个错误。全仓只有 5/52 份 RFC 有 `impl_status` 字段（`027` in-progress、`017`/`014`/`014b` complete、`review/027a` partial）。**26 份 accepted RFC 完全不记录实现状态**，包括 009、009a、013、018、038 |
| F4 | `src/frontend/core/typecheck/layers/README.md:1-16` | 声明层序 equivalence → ownership → termination → predicate，**与实际执行顺序正好相反**（实际 predicate → termination → ownership），且**无任何 short-circuit**。README 描述的是一套未建成（或已部分放弃）的意图架构 |
| F5 | `docs/src/design/rfc/index.md` | `:79` 把 RFC-026a 放在"已接受RFC"表格里但状态列写"审核中RFC"；`:48-49` 的"审核中RFC"小节只列 032 和 027a，漏 026a。与 `TRACKING.md` 的 3 条 review 不一致 |
| F6 | `tools/cargo-dist/`（`dist.exe` 21,108,736 字节）与 `benches/shootout/out/`（8 个二进制） | **均未被 git 追踪**（`git ls-files` 返回空），被 `.gitignore` 排除。**不是问题**，仅记录以免后续审计误判为"提交了二进制" |
| F7 | `tools/code-tables/src/lib.rs:66-78` | `extract_code_from_entry_line` 是**行首前缀匹配器**（`trim_start().strip_prefix("(\"")` 取到下一个引号），依赖 `("E1001", ...)` 元组语法，**不是 parser**。它的架构（`parse` + `validate` + build.rs 门禁 + `--fix` 治愈）值得复用到 opcode 表，但**提取器需按 opcode 形态重写（约 30 行）**。归属 06-cleanup-inventory.md 的后续工作或独立 RFC |
| F8 | `src/backends/common/opcode.rs` 的门禁缺口 | 83 个常量、0 重复、范围 `0x00..0xE2`（span 227，**144 个空洞**）。同一份 opcode 事实被表达 **5 处**：`BytecodeInstr::opcode()`(`bytecode.rs:558-646`)/ `size()`(`649-803`)/ `opcode::opcode_name()`(`opcode.rs:120-206`)/ 解码 match(`bytecode.rs:978-2302`)/ 48 个 `translate_*`(`translator.rs:676-1577`)。**编译器只强制其中 2 处**（`backends/.../executor/debug.rs:194` 分派表 + 各 `ops/*.rs` 族函数的穷尽 match），`opcode()` / `size()` / 解码臂 / 编码器漏一处**只有跑 `.42` 产物才炸**。`size()` 表(649-803)的注释自述与实际编码不符（`bytecode.rs:2181-2182`），且**无系统性对拍测试**（现有 size 测试只覆盖 Nop/Mov/Borrow/Release 4 个）。运算符语义在此也走第三套平行枚举（`BinaryOp`/`UnaryOp`/`CompareOp`，`bytecode.rs:70/97/106`，`Rem`/`Xor`/`Sar` 又一套命名），随 opcode 生成期门禁一并收口。词表目标位置为 `middle/bytecode/opcode.rs`（随字节码域合并迁移，消除 L3→L4 反向，见 01） |

## 关键决策与理由

授权是"允许彻底删除"，但本文对若干项给出**不同建议**：

| 类别 | 处置 | 理由 |
| --- | --- | --- |
| **纯占位**（无断言、无实现的骨架） | **删** | B1（415 行）、B3（dispatch 链）、B6（96 行）、B7（空函数）、C6（14 行）、E1（空目录） |
| **空头设计文档** | **删或重写** | C1、C2 删除后重写为指向实际实现；C3 改写边界表；**必须同步清理 `config.js:259-268`**（E6） |
| **状态与实现不符的 RFC** | **改状态** | C4（RFC-018）移回 `draft` 或补 `impl_status: 'not-started'`；C5（RFC-028）补 `impl_status: 'not-started'`。**不删**——设计本身可能有价值，问题是状态标记失真 |
| **有真实断言的孤儿测试** | **复活，不是删除** | 合计 **1005 行 / 78 个测试**：lexer 629 行 / 55 个 + 藏在活目录里的 311 行 / 16 个（`precedence_inline` / `overload_inline` / `json`）+ `package/template/tests/` 65 行 / 7 个。**复活成本是一行 `mod` 声明，价值是让 CI 重新获得这些覆盖**。删除它们是净损失 |
| **零调用但语义重要的 `pub` 项** | **降可见性而非删除** | B2（`TypeSystem`）、B4（`check_type_equivalence`）——移到 `pub(crate)` 让编译器指认，或补实现。**B4 的 `is_subtype` 必须保留** |
| **产不出值的 opcode / 指令** | **补构造点或删 opcode** | B8（`TailCall`）、B9（`Switch`，opcode 往返测试已以 `WHITELIST=["SWITCH"]` 显式登记）、B10（`UnaryOp::Not` 静默退化）。三选一，不能维持现状；D32/D33/D34 已裁决为删，随 `.42` 升版一并处理 |
| **数据丢失缺陷** | **独立 issue** | E 节三条硬编码丢弃——不是死代码 |
| **无模块身份但有效** | **改造不删除** | F1（`semantic_tokens.rs`）改 `include!` → 真 `mod`（见 `02`） |
| **需要决策的** | **列为开放问题** | F2 的**不可达 wasm 分支**清理范围、层序修正会暴露新诊断 |

### 未采纳的方向

- **全部按"彻底删除"处理，孤儿测试也删**——1005 行有真实断言的测试是资产。删除成本是一行 `mod` 声明，收益是 CI 重新获得覆盖。授权是"可以删"，不是"应该删"。
- **只加门禁不动代码**——门禁能防退化，但 B1 的 415 行、dispatch 链、96 行死阶梯**已经存在**，门禁只会把它们冻结在基线上。
- **保留 RFC-018 / RFC-028 的 `accepted` 状态只补文档**——`rfc/index.md:131` 定义"已接受 = 进入实现阶段"，读者会合理假设已有实现。`TRACKING.md` 已有 `impl_status` 字段的先例（5 份 RFC 在用），补字段的成本远低于状态失真的代价。

## 实施要点

**本文不需要等价性判据**（属于 [C6 纯删除类别](07-equivalence-oracle.md)），但**仍需逐项确认无引用**。在 RFC-039 的全局阶段序列中，S1 对应 P1，S2-S6 对应 P10。

| 阶段 | 内容 | 验收 |
| --- | --- | --- |
| S1 | 复活有断言的孤儿测试（**1005 行 / 78 个测试**） | `cargo test` 中 lexer / pratt / passes / emitter / template 五个模块的测试数上升；**预期会暴露真实缺陷**（`literals.rs` 的溢出路径、`\x`/`\u` 非法转义此前从未被测过），需预留修复时间 |
| S2 | 删纯占位死代码（B1/B3/B6/B7 随 05 收口/C6/E1） | `cargo test` 通过；`cargo clippy -D warnings` 无新增警告 |
| S3 | 空头设计文档处置（C1-C3 + E6 同步清理 `config.js`） | 文档站构建无 404；`scripts/ci/check-docs-truth.py` 通过 |
| S4 | RFC 状态修正（C4/C5/F3——`TRACKING.md` 加「实现状态」列） | `python scripts/rfc/check_tracking.py` 通过（`TRACKING.md` 会自动重新生成，**不手改**） |
| S5 | `pub` 项降可见性（B2/B4） | 编译器能指出真正的死代码 |
| S6 | opcode 产不出值项决策落地（B8/B9/B10/F8，按 D32/D33/D34 执行） | 删 `Switch` / `TailCall` 死路径；B10 的 `opcode()` 区分 `op` 字段；F8 门禁落地 |
| S7 | `include!` 改造（F1，施工步骤归 [09](09-execution-wbs.md) §P5 5.1）+ 层序修正（F4，归 P4 的 4.4.1） | 见 `02-stage-contract.md` |

**S1 必须最先做，且必须预留缺陷修复时间。** 它的价值不是"清理"，而是**让 CI 重新获得 1005 行 / 78 个从未运行过的测试**——在此之前，任何关于测试覆盖率的判断都建立在虚假数字上。

## 已知局限与风险

- **S1 会暴露真实缺陷**。`literals.rs` 的四个基数扫描器有 4 处同构的"溢出后继续消费但不报错"分支、`scan_leading_dot` 整个函数、`\x`/`\u` 非法转义路径，全部从未被测试。复活测试后大概率需要修复代码，这会推迟后续重构。
- **B9/B10 涉及语言行为决策**。`Switch` 与 `UnaryOp::Not` 是"补构造点"还是"删 opcode"，需要考虑是否属于已发布语料依赖的表面。
- **C1-C3 删除后，跨文件分析的能力文档会短暂缺位**，直到 `check/cross-file-analysis.md` 被重写为指向 `src/frontend/module/` 的实际架构。
- **可达性分析只能发现"不可达"，不能发现"可达但无意义"**。形态 2 之外的判断必须逐项人工进行，且**本清单不构成穷尽证明**。
- **`pub` 项在 Rust 中不触发 `dead_code` 警告**，因此零调用的 `pub` 类型（B2、B4）不会被编译器自动发现，只能靠搜索。
- **删除 RFC-023 的半截残留**（B1）能兑现"闭包已从语言中删除"这个决策；**修正状态失真**（C4/C5/F3）能让 `TRACKING.md` 恢复参考价值。

> **本节原列的开放问题已全部裁决。** 逐条决定见 [RFC-039 决议登记](../rfc/draft/039-compiler-architecture.md)（D1–D50）。**本文不留任何待定项。**
>
## 参见

- [RFC-023 闭包捕获模型（已废弃）](../rfc/deprecated/023-closure-capture-model.md) — B1 残留的来源决策
- [RFC-027 编译期求值与类型](../rfc/accepted/027-compile-time-evaluation-types.md) — B3 未接线的 `§4.1` 运行时检查插入
- [RFC-018 LLVM AOT](../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT 编译器（草案）](../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a 模块缓存与增量重编译（草案）](../rfc/draft/029a-module-cache-incremental.md) — C1/C6 的正式载体
- [RFC-039 编译器功能路由目录设计](../rfc/draft/039-compiler-architecture.md) — 上位总纲
- [重构等价性判据](07-equivalence-oracle.md) — C6 类别的判据要求
- `src/frontend/core/typecheck/layers/README.md` — F4 层序声明失真
- `build.rs:19-55` — 本仓库生成期门禁的范例
