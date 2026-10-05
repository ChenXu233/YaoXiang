---
title: 'RFC-039: 编译器架构重构（总纲）'
status: '已接受'
author: 'ChenXu233'
created: '2026-10-03'
updated: '2026-10-05'
accepted: '2026-10-05'
issue: '#430'
---

# RFC-039: 编译器架构重构

## 摘要

本 RFC 为 YaoXiang 编译器（`src/` 160,002 行 / 509 个 `.rs`）提出一次架构重构：以**四层模型**（编排 / 前端 / 中间表示 / 执行）重新划定边界，建立**可穷举的编译阶段契约**与**义务账本**，并配套一套**等价性判据**与**防反弹门禁**。

详细设计放在 `docs/src/dev/architecture/` 的九篇附属文档中。本 RFC 只负责：问题、根因、四层模型、验收判据分级、执行阶段顺序。**审阅本 RFC 即可决定是否立项；附属文档是立项后的施工图。**

## 动机

### 一、边界存在，但没有强制力

目录名上的分层（`frontend` / `middle` / `backends`）是清晰的。但**边界只存在于人的自觉里，没有任何机制保证它不被侵蚀**。三个直接后果。

#### 1.1 编译阶段在五个入口各自手工接线，产生 11 处行为不一致

| 阶段 | 单文件 `run` | 多文件 `run` | `check` | LSP（项目内） | LSP（单文件） |
| --- | --- | --- | --- | --- | --- |
| lexing / parsing | 有 | 有 | 有 | 有 | 有 |
| typecheck | 有 | 有 | 有 | 有 | 有 |
| **proof_execution** | **有** | **无** | **无** | **无** | **无** |
| 死代码分析 | 有 | 无 | 有 | 无 | 无 |
| W1006 遮蔽诊断 | 无 | 无 | 有 | 无 | 无 |
| 单态化 | 有 | 无 | 无 | 无 | 无 |
| IR 生成 / 链接 | 有 | 有 | 无 | 无 | 无 |

五个入口：`src/frontend/pipeline.rs:141-227`（单文件，5 阶段在 `149` / `161` / `173` / `188` / `205`）、`src/frontend/compiler.rs:106`（包装）、`src/frontend/module/orchestrator.rs` 的四个（`compile_project:99` / `check_project:273` / `check_source_in_project:450` / `compile_embedded_module:1374`），以及 `src/lib.rs` 的 `run_file:140` / `run_project:154`。

最严重的一格导致一个**正确性漏洞**。形如 `x: Sorted(3)` 的证明函数约束，在多文件、check、LSP 三条路径下**静默通过**。证据链（全部可复核）：

1. `src/frontend/core/typecheck/layers/predicate.rs:232-239` 是全仓库**唯一**构造非空 `proof_calls` 的位置，语义为"此约束需要执行证明函数"。
2. `src/frontend/core/typecheck/checker.rs` 有三处同构分支处理 `Unproven`：`5164` / `5306` / `5420` 是 `if calls.is_empty()` → 推硬错误；`5179` / `5318` / `5448` 是 `ctx.proof_calls.extend(...)` → **不产生任何诊断**。`5165` 的注释原文写着「Unproven → 编译错误，无降级、无 silent pass……不得让 silent pass 复活」。
3. `TypeCheckResult.proof_calls`（`types.rs:29`）在全仓库**只有 `src/frontend/pipeline.rs:187` 一处读取**。
4. `orchestrator.rs` 的四个入口**均不读取该字段**。

即：代码自己声明的不变量，被架构而非逻辑破坏了。`compile_embedded_module`（`orchestrator.rs:1374`）的存在意味着**嵌入 std 模块自身的精化义务也走这条丢弃路径**。

**无测试能拦住它**：`tests/integration/multifile.rs` 的 27 个测试对 `Sorted` / `proof` / `refin` 零命中；RFC-027 的单测直接调 `check_module` 断言"`proof_calls` 非空"，**恰好停在 pipeline 之前**——它们验证"填充了"，而 bug 是"消费端没读"。

#### 1.2 模块边界可以绕过而不留痕迹

`src/frontend/core/typecheck/checker.rs:5618` 是 `include!("checker/semantic_tokens.rs");`——**全仓库唯一的一处 `include!`**。它把 1547 行文本拼入 `checker` 模块。该文件**没有被声明为任何模块**（`grep 'mod semantic_tokens'` 全仓零命中），第一行直接是 `impl TypeChecker {`，没有自己的 `use` 头，`checker/` 目录下只有它一个文件。

后果：没有模块身份、可见性隔离失效、rust-analyzer 的跳转与符号搜索对其失效、工具链分析时它被算作 `checker.rs` 的一部分。**`checker` 模块的真实体量是 5618 + 1547 ≈ 7165 行**，而非目录统计显示的 5618 行。

#### 1.3 测试接线靠人记，腐化静默发生

从 `src/lib.rs` / `src/main.rs` 两个 crate root 做模块图 BFS 可达性分析：**24 个文件 / 2628 行从未参与编译**。其中真实孤儿 23 个文件 / 1081 行。

| 孤儿 | 规模 | 内容 |
| --- | --- | --- |
| `frontend/core/lexer/tests/` 整棵子树 | 13 文件 / 686 行 | 7 个空壳共 19 行 + `mod.rs` 38 行 + **629 行 / 55 个真实测试** |
| `frontend/pipeline/tests/` | 3 文件 / 14 行 | 全是占位文档注释 |
| `package/template/tests/` | 3 文件 / 65 行 | 7 个 `#[test]` |
| `parser/pratt/tests/precedence_inline.rs` | 95 行 / 6 test | 藏在活的目录中 |
| `typecheck/passes/tests/overload_inline.rs` | 170 行 / 7 test | 同上 |
| `util/diagnostic/emitter/tests/json.rs` | 46 行 / 3 test | 同上 |

`cargo test` 无法发现：没有跑过的测试不会失败。CI（`.github/workflows/ci.yml:140`）也没有任何测试数量基线或接线检查。

**从未运行的真实测试合计 1005 行 / 78 个。** 词法层实际在跑的只有 `fstring.rs`（经 `#[path]` 属性旁路接入）。

### 二、设计能力是够的

必须同时承认：这个项目**已经有**成熟的强制机制，只是没有推广。

`src/util/diagnostic/codes/` 的错误码（137 个 E 码 + 8 个 W 码 = 145）由 `build.rs:19-55` 通过 `tools/code-tables` 做构建期硬门禁——解析注册表、校验唯一性与段位、逐条比对 RFC-013 的 markdown 码表，**任一不一致直接 `panic!` 拒绝编译**。RFC-013 文档和代码因此始终一致。

`src/package/`（76 文件 / 13,012 行）是完成度最高的区域：**测试 6,227 行、占 47.9%**；6 个 `tests/` 目录中 5 个正确接线（唯一例外是 `template/tests/`，见 A 节）。注意这个正面样本成立在**目录级接线**上，不在每个文件上——76 个 `.rs` 文件中只有 7 个含内联 `#[cfg(test)]`、5 个含 `mod tests;`。5 份 RFC 与代码逐条吻合，多处「未实现」落成显式错误而非静默 TODO（`build/mod.rs:172-174` 的 cmake、`error.rs:74-76` 的 `RegistryDeferred`）。`.yxpkg` 有五层安全防护且全部正确。

**同一个仓库、同样的作者，`package/` 在测试目录接线上做对了，`lexer/tests/` 一次都没跑过。差别不在能力，在于有没有人跑一次检查。**

### 三、空头设计

`docs/src/dev/design/check/` 下的三篇文档描述了一套**从未存在**的架构：`CheckSession`（含 `incremental-checking.md:31-41` 的 Rust 代码草稿）、`ModuleDependencyGraph`、`ModuleCache`、`HotReloader` 全仓零命中；`traits/` 目录不存在；`check_single_module` 函数不存在（只活在测试注释里）；连"已知限制"里引用的 `command.rs` busy-wait 都不存在（该文件既无 `Instant` 也无 `recv_timeout`）。

跨文件分析实际上由 `src/frontend/module/`（registry 439 + resolver 195 + roles 388 + orchestrator 1528，约 2670 行）实现——能力是真的，路径完全不同。**这份文档把"未来可能的设计"写成了"已实现系统的缺陷"。**

`src/frontend/pipeline/tests/compilation_cache.rs` 与 `incremental_scheduler.rs`（各 3 行）是"按设计建了测试骨架、发现管线里没有对应实现、就搁下了"的最干净的物证。

此外 `RFC-018`（LLVM AOT，1037 行，状态 accepted）在代码中有 0 行实现——`backends/mod.rs` 里 AOT 相关的 6 处命中**全部是注释**。

## 根因诊断

把三类症状合起来，根因是单一的：

> **这个项目把"设计"当作文档约定，而不当作可执行的约束。**

| 层面 | 缺的强制 | 已有但未推广的范例 |
| --- | --- | --- |
| 阶段边界 | "这次编译跑了哪些阶段"的单一决策点 | — |
| 模块边界 | 禁止 `include!`、限制 `pub(crate)` 跨层泄漏 | — |
| 交叉引用 | "字段被产出但无人消费"的可检测性 | `build.rs` 的错误码门禁 |
| 测试接线 | 声明了目录就必须有 `mod` 声明 | `src/package/` 的目录级接线惯例（6 个 `tests/` 中 5 个正确） |

本 RFC 的立场因此是：**把阶段、模块、交叉引用三类边界从"文档约定"变成"编译期或测试期可断言的事实"**，并且**在编排层建立一个唯一的阶段决策点**。

## 提案

### 四层模型

```
┌─────────────────────────────────────────────────────────────┐
│ L1 编排层 Orchestration                                      │
│   谁决定跑哪些阶段、跑几遍、失败如何传播                      │
│   现状: pipeline.rs(740) + orchestrator.rs(1528)             │
│         + compiler.rs(267) = 5 个入口, 11 处不一致           │
│   目标: 单一 Driver + 可穷举的 Stage 枚举 + Obligations 账本  │
│   详见: ../compiler-architecture/02-stage-contract.md           │
└───────────────────────────┬─────────────────────────────────┘
                            │ 编译单元 (Unit)
┌───────────────────────────▼─────────────────────────────────┐
│ L2 前端层 Frontend                                           │
│   词法、语法、AST 构造                                        │
│   现状: lexer/(3065) + parser/(6341)                        │
│         手工 lexer + 手工 Pratt; 加运算符要改 L2 内          │
│         6-7 处 + 下游 17 个生产文件                          │
│   目标: 词法收敛为唯一实现 + LALRPOP 文法驱动语法            │
│   详见: ../compiler-architecture/05-frontend-paradigm.md        │
└───────────────────────────┬─────────────────────────────────┘
                            │ Module (AST)
┌───────────────────────────▼─────────────────────────────────┐
│ L3 语义与中间表示 Semantic & IR                              │
│   类型检查、静态分析、IR 构造                                 │
│   现状: typecheck/(29466 生产) + ir.rs(905) + ir_gen.rs(8448) │
│         3 套平行类型表示 + 3 套平行运算符枚举;                │
│         ir_gen 单 impl 7952 行 / 116 方法; 无 IR 级校验器    │
│   目标: 类型表示单一化; IR 满足 SSA 构造纪律                  │
│   详见: ../compiler-architecture/03-type-unification.md         │
│         ../compiler-architecture/04-ssa.md                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ ModuleIR
┌───────────────────────────▼─────────────────────────────────┐
│ L4 执行层 Execution                                          │
│   字节码生成、解释器、运行时、标准库                          │
│   现状: codegen/(3436) + bytecode.rs(2422) + executor/(2000)  │
│         + runtime/(1800) + std/(7543)                         │
│   目标: opcode 事实单源化 + 生成期门禁; 执行层不感知 L1-L3    │
│   详见: ../compiler-architecture/06-cleanup-inventory.md        │
└─────────────────────────────────────────────────────────────┘
```

**四层是严格单向的**：L1 依赖 L2/L3/L4 的接口；L2 不依赖 L3；L3 不依赖 L2（只消费 AST 数据，不调用 parser）；L4 只依赖 L3 的产物格式。

已知的反向依赖与平行定义清单见 `../compiler-architecture/01-routing.md` 路由表 C。

### 附属设计文档

九篇，存放在 `docs/src/dev/architecture/`：

| 文档 | 层次 | 主题 |
| --- | --- | --- |
| `01-routing.md` | 跨层 | **功能路由表**、依赖方向规范、**施工完成后的目标目录结构**、未来扩展指引 |
| `02-stage-contract.md` | L1 | 阶段契约与义务账本：11 处不一致、漏洞证据链、`Stage` / `Obligations` / `Driver` / `ProgramKind`（含 wasm playground 一侧的 `WasmPlayground`） |
| `03-type-unification.md` | L2/L3 根因 | 类型表示单一化：26 个 `ast::Type` 变体逐个处置、13 个前向零构造变体、类型表门禁 |
| `04-ssa.md` | L3 | 中间表示 SSA 化：四类缺陷、SSA 形态定义、分配器评估、38 项改动清单 |
| `05-frontend-paradigm.md` | L2 | 词法收敛为唯一实现 / 语法 LALRPOP 文法驱动、改动面收敛、死阶梯处理、测试重建 |
| `06-cleanup-inventory.md` | 全域 | 死代码与空头设计清理：可达性方法、逐项处置、执行顺序 |
| `07-equivalence-oracle.md` | 跨层 | 等价性判据：三层判据、C1-C6 分级、门禁设计 |
| **`08-maintenance-mechanism.md`** | **跨层** | **仓库维护机制**：三条禁令（不得生造 / 不得无尽填充 / 该重构却打了补丁）、D0–D4 决策程序、可机检规则、代码审查清单、外部惯例参照 |
| **`09-execution-wbs.md`** | **跨分册** | **多级施工任务表**：11 一级 / 45 二级 / 122 三级任务、依赖与并行分组、**8 项冲突登记** |

`01` 与 `09` 是**长期参考文档**——前者不随任何一次重构失效，后者是施工清单。`08` 产出的三条禁令约束 P1–P10 的每一个动作。

### 等价性判据分级

**这是全部重构的安全网，必须先于任何代码变更建立。** 详细设计见 `07-equivalence-oracle.md`。

核心原则：**判据按重构类别分级，因为不同类别需要不同强度的等价性。** 用单一判据（通常是"IR 快照全等"）覆盖所有重构是错的——有些重构**必然改变 IR 形态**。

| 类别 | 重构内容 | 判据类型 | 强度 |
| --- | --- | --- | --- |
| **C1** | 纯搬移（拆文件、改目录、提取子模块） | IR 规范化快照 **zero-diff** | 最强 |
| **C2** | 编排变更（阶段化、统一 Driver） | 各入口诊断集相同 + 语料行为相同 | 强 |
| **C3** | 类型表示收敛 | 诊断**码**相同（消息措辞会变） | 强 |
| **C4** | IR 形态变更（SSA 化） | **行为等价** + IR 结构不变量 | 中 |
| **C5** | 前端范式变更 | AST 快照 + 诊断 + 行为，三者全用 | 强 |
| **C6** | 纯删除 | 无需等价性，只需确认无引用 | — |

**C1 与 C4 的区别是关键**：C1 要求 IR 逐字节相同；C4 承认 IR 会变，改用"程序行为相同 + IR 满足不变量"。强行对 C4 用快照会诱导团队放宽判据。

三层判据：

1. **IR 静态校验器** —— 支配性 use-before-def、jump 目标存在、类型一致、内层隔离。比快照强得多，覆盖"IR 自洽但值错"这类快照测不出的缺陷。必须**先在现有（非 SSA）IR 上跑绿**。
2. **规范化 IR 快照** —— 剥离 `Span`、临时值按出现序重命名、全局槽位相对化、前驱排序。入库，人工 review diff。**已知局限**：规范化会抹掉"第 N 个实参用了哪个寄存器"，因此**单独不足以覆盖 `arg_regs` 语义重排**。
3. **端到端语料差分** —— 诊断列表、退出码、stdout/stderr 逐项比对。

> **⚠️ 语料覆盖的结构性限制**：`tests/` 目录下**没有任何 `yaoxiang.toml`**（实测 0 个），因此 `tests/yaoxiang/` 的 **293 个** `.yx` 语料**全部走单文件路径**（`check_files_with_diagnostics` 的 standalone 分支 → `check_single_file` → `Pipeline::run`）。
>
> **后果**：第三层判据**只能验证单文件路径的改动**。对多文件侧（`orchestrator` 的四个入口）与类型/SSA 的多文件表现，**当前没有任何语料覆盖**。P4「统一 Driver」是全计划最大风险点，**多文件语料层必须无条件在 P2 建立**（决议 D40）——它是 P4 唯一可执行的行为判据来源。

## 实现策略

### 阶段序列

```
P0  仓库维护机制与代码放置规程 08        ← 先立规矩:三条禁令 + D0-D4 决策程序
P1  复活孤儿测试并修缺陷        06 §S1   ← 最便宜,收益最大
P2  建立等价性判据基线          07 全部
P3  修正确性漏洞(最小方案)      02 §修复
P4  阶段契约与统一 Driver        02
P5  checker 文件内拆分          09 §P5 补齐
P6  类型表示单一化              03
P7  中间表示 SSA 化             04     } 可并行
P8  前端范式变更                05     }
P9  防反弹门禁(统一脚本清单)     01 + 08
P10 其余清理与状态修正          06 §S2-S6
```

**核心串行约束（不可调换）：**

| 约束 | 原因 |
| --- | --- |
| P0 → P1 | 三条禁令约束 P1 的每一个动作——1.2.x 就是在往 `mod.rs` 加 `mod` 声明，那正是 `precedence_inline.rs` 生死冲突的成因 |
| P1 → P2 | 复活测试会改变测试数量与语料基线。先建基线会在 P1 后失效 |
| P2 → P3 | 漏洞判据必须**先写成红的**才能证明修复有效。**收窄（D51）**：P3 的前置仅为 2.3.2（多文件语料层）+ 2.4.1/2.4.3（两个红判据）；P2 其余部分可与 P3 并行，但 P4 必须等 P2 全部完成——正确性漏洞的止血不被判据基础设施建设阻塞 |
| P3 → P4 | 先修 bug 再重构。反序会让 bug 被阶段表固化成"既定行为" |
| P4 → P5 | P5 以 `include!` → 真 `mod`（09 §P5 5.1）开场，随后的 checker 拆分与之动同一个文件，必须在 P4 之后连续进行 |
| P5 → P6 | 同一文件的连续改动必须分开，否则回归无法二分定位 |
| P6 → P7 / P8 | 类型表示不先收敛，SSA 的新 IR 会长成第三套表示 |
| P7/P8 → P9 | 各门禁脚本的初始基线（如 `pub(crate)` 泄漏计数）在 P7/P8 之后才取最终值 |
| P7 ∥ P8 | 文件集不相交，可并行 |

**P0 为什么独立成阶段**：P9 的 CI 脚本是**事后检查**，只能拦住已经写下的坏代码；"该重构却打了补丁"是**事前判断**，只有 D0–D4 的决策程序能拦。两者不能互相替代。

### 各阶段要点

| 阶段 | 触及范围 | 验收 | 回滚点 |
| --- | --- | --- | --- |
| **P0** | `CONTRIBUTING.md` 规程节 + 3 个新门禁脚本（`check-concepts` / `check-fanout` / `check-boundary`）；**不改任何编译器源码、不设行数门禁** | `check-concepts.py` **必须在未改动代码上报出 3 套运算符枚举、2 套平行类型表示（`ir::Type` 别名一并计入）**；故意超限/新增第 6 个入口式接线必须红 | 纯新增，删脚本即可 |
| **P1** | 5 个文件补 `mod` 声明；**1005 行 / 78 个测试复活** | 测试数上升；**预期暴露真实缺陷**（`literals.rs` 溢出路径、`\x`/`\u` 非法转义从未被测过） | 逐文件 revert；**修复的缺陷不应回滚** |
| **P2** | 新增 `verify.rs`、快照基线、语料差分框架、**多文件语料层**、性能基线（criterion 冒烟基准）、`scripts/ci/check-*` | `verify_loose` 跑绿；漏洞判据**写成红的** | 判据代码可整体移除 |
| **P3** | `types.rs`（`proof_calls` 收成私有）、`orchestrator.rs` 三个消费点 | 漏洞判据**转绿**；**故意撤掉修复必须重新变红** | 纯行为修正 |
| **P4** | 新增 `src/driver/`；改写 5 个入口；**4.5：`ReleasePlan` / `overload_resolutions` 键 `Span` → `PlanId`（D20）** | 诊断集相同 + 语料行为相同（C2）；span 键控静默失效归零 | **最大风险点**；保留旧入口、Driver 未接线即可回滚 |
| **P5** | `include!` → 真 `mod`（5.1）；`checker.rs` 拆出 `refinement` / `annotations` | **IR 快照 zero-diff**（C1）；`statements.rs:16` 的 `pub(crate)` 路径不变 | 逐文件 revert |
| **P6** | `ast.rs`、`types/mono.rs`、`solver.rs`、`ir.rs:3`、`bytecode.rs:2353-2390`；阶段收尾执行目录改名（D1：`typecheck/`→`sema/`、`middle/core/`→`middle/ir/`） | 诊断**码**相同（C3）；改名批次为纯搬移（C1 zero-diff） | 分阶段提交，变体处置与同义词表删除分开；改名独占 commit |
| **P7** | `ir.rs`、`ir_gen.rs`（8448 行）、`bytecode.rs`、`translator.rs`；**7e：三处 `.42` 数据丢失修复（`upvalue_count` / `exception_handlers` / `globals`）+ `VERSION` 4→5（D17、D52）** | **行为等价 + `verify_ssa` 绿**（C4）。**不是 IR 快照全等**；`.42` 往返测试 | 逐批 revert |
| **P8** | `lexer/*`、`parser/*` | AST 快照 + 诊断 + 行为（C5） | 逐文件 revert |
| **P9** | 新增 `scripts/ci/check-*.py` 共 10 个（唯一清单见 [09](../../dev/architecture/09-execution-wbs.md) §P9：3 个随 P0、1 个随 P1、3 个随 P2、2 个随 P4、1 个随 P7 引入，P9 收口统一转硬） | **故意让义务无消费点必须红**；**故意造孤儿测试目录必须红**；**故意加 `include!` 必须红** | 删脚本 |
| **P10** | 文档处置、RFC 状态修正、`pub` 项降可见性、opcode 决策 | `check_tracking.py` 通过；文档站无 404 | 独立 PR |

### 每阶段的完成定义（DoD）

一个阶段算完成，必须同时满足：

1. 归属文档中对应的「实现策略」步骤全部提交
2. `07-equivalence-oracle.md` 中该类别（C1-C6）的判据**在 CI 中为绿**
3. `cargo test` 全绿，且**测试数不低于该阶段开始时**（防止"删测试换绿灯"）
4. `cargo clippy --all --all-features -- -D warnings` 干净
5. `python scripts/rfc/check_tracking.py` 退出码 0
6. 阶段有独立的 revert 单元——**如果一个阶段无法单独 revert，它就太大了，需要再拆**
7. 触及编译管线或执行路径的阶段（P4 / P6 / P7 / P8），07 的性能基线对比无未说明的 >10% 回归

### 全局验收门禁

全部阶段完成后，下列必须为真：

| # | 条件 | 验证方式 |
| --- | --- | --- |
| G1 | 编译阶段的决策点唯一 | 回答"跑了哪些阶段、哪些没跑、为什么"只需看一个地方 |
| G2 | 无"产出但无人消费"的字段 | `check-obligations.py` 绿 |
| G3 | `include!` 数量为 0 | `grep -rn 'include!' src/` |
| G4 | 无孤儿测试 | `check-test-wiring.py` 绿 |
| G5 | 无反向依赖 | `check-module-boundary.py` 绿 |
| G6 | 类型表示唯一 | 平行表示收敛；手写同义词表消失 |
| G7 | parser 不含类型/谓词硬编码 | `"Terminates"` 等字面量被数据流取代 |
| G8 | 加一个二元运算符的改动面 ≤ 3 处 | 人工 review + 路由表更新 |
| G9 | `layers/README.md` 描述实际层序 | 文档与 `check_module_impl` 一致 |
| G10 | `TRACKING.md` 有「实现状态」列 | 52 份 RFC 的实现状态可查 |

## 权衡

### 优点

- **给所有后续改动一个落点**。"加运算符要改十几处、漏了没人知道"无法靠记忆改善，只能靠路由表 + 门禁。
- **根因单一**。三类症状（阶段不一致、`include!`、测试接线断裂）都源于"边界是约定不是约束"，可一次性用同一类机制解决。
- **不阻塞**。本 RFC 零代码改动。四层模型是描述性的，采纳后仍可只执行其中一部分。
- **判据先行**。P2 在任何代码变更前建立，且分级设计避免了"快照过严 → 诱导放宽"的常见失败。

### 缺点与风险

- **串行链很长**。P1 → P2 → P3 → P4 → P5 → P6 → P7/P8 近乎完整串行，后期阶段要等前期完成。
- **P1 会引入新的 bug 工作量**，且这是最容易被"因为要修 bug 所以先不做"跳过的一步——但跳过它，后续所有阶段的验收判据都建立在虚假覆盖率上。
- **P4 是最大单点风险**：一次改动 5 个入口。
- **P6 影响面最广**：触及 parser、formatter、spawn、orchestrator 的共同依赖。
- **P7 的行数会净增 900-1600 行，不是净减**。拆分引入样板，`verify` 300-600 行，寄存器分配 600-1000 行。**若立项时以"代码变短"为成功标准，这一阶段会被判定为失败**——这一点必须在立项时对齐。

## 替代方案

### A. 只拆文件，不动范式

把 8448 行拆成 10 个 800 行的文件。

**未采纳。** 拆分不消除任何根因：6 处手工 save/restore 仍分散在 10 个文件里，`generate_call_expr_ir` 的 `arg_regs` 语义重排仍需人工推理，"加运算符改十几处"仍然是十几处。拆分改善导航，不改善正确性。

### B. 只加门禁，不改结构（含行数 ratchet）

**未采纳为唯一方案，但它是本 RFC 的一部分。** 门禁能防退化，不能修现状——`ir_gen.rs` 已经有 8448 行了，门禁只能把它冻结在这个数字。

### C. 推倒重来，新写编译器骨架

**未采纳。** `src/package/` 证明这个团队能把复杂系统做对。问题不在能力。而且现有 293 个 `.yx` 语料提供了现成的等价性判据来源，从零重写反而浪费这个资产。

### D. 用现成编译器框架（如 LLVM 作为唯一后端）

**未采纳。** `Executor` trait（`backends/mod.rs:356`）已预留抽象点但只有一个实现者。引入 LLVM 是 `RFC-018` 的范畴（1037 行设计、0 行代码），应当**独立决策**——它改变的是"编译到什么"而非"怎么组织代码"。

## 决议登记：全部未决项已定

> **本节是唯一权威。** 各附属文档中原有的 `- [ ]` 开放问题与「冲突登记」**全部收敛到本表**，不再各自保留。**没有"待定"，没有"推迟"，没有"可选"。**
> 每条给出决定与理由。若实施中发现某条决定的技术前提不成立，**正确动作是回到本表改决定并说明原因**，不是绕开。

### 目录与命名

| # | 议题 | **决定** | 理由 |
| --- | --- | --- | --- |
| D1 | 目录改名（`typecheck/`→`sema/`、`middle/core/`→`middle/ir/`、`parser/ast.rs` 拆迁顶层 `src/ast/`、opcode 词表迁 `middle/bytecode/`）是否值得做 | **做。** 不分阶段、不设"可选"，随 P5/P6 一并完成（AST 顶层域与字节码域合并同批，均为 C1 纯搬移） | 边界由 CI 保证，但**目录名是职责的第一眼信号**。`middle/core/` 里放 `ir.rs` + `ir_gen.rs` + `bytecode.rs` 三件事，名字已经失效；AST 被全层消费，放 `frontend/` 下会误导下游路径（业界 6/7 家均为与 parser 平级的顶层域） |
| D2 | 命名约定 | 保留 `type_.rs` / `fn_.rs`（避 Rust 2024 保留字）；禁止 `xxx_v2` / `xxx_new` 目录 | 并行副本是死代码的常见来源（`pratt/precedence.rs` 的 `Precedence` 枚举与实际生效的 BP 常量平行、生产零引用；`Expr::FnDef` 是一条永不执行的并行构造路径） |

### 阶段契约（02）

| # | 议题 | **决定** | 理由 |
| --- | --- | --- | --- |
| D3 | `check_module`（`typecheck/mod.rs:81`，fail-fast；早退实现在 `inference/statements.rs:2797-2799`）vs `check_module_collect_all`（`typecheck/mod.rs:93`，收集全部；调用点 `orchestrator.rs:124` / `:493`）是否合并 | **不合并。** 保留两个 API，由 `Program` 的聚合模式字段（`Aggregation: FailFast \| CollectAll`，见 02）驱动 | 两者语义确实不同（`CollectAll` 服务 LSP）。强行合并会引入 `Option` 噪声 |
| D4 | `Obligations` 的 16 个字段哪些"必须被消费" | **全部 16 个进账本。** 无消费点的字段（如 `module_namespaces`）在 P4 的 S4 一并删除，**不留孤儿字段** | 账本的意义就在于无例外 |
| D5 | `assert_drained()` 的诊断级别 | **最终 E 级**（错误）；P4 内先以 W 级观测上线、同阶段收尾升 E（02 §S4 的两步走） | 长期停在 W 等于没有门禁——W 不改变退出码；但一步升 E 会让 C2 判据不可用，故 E 是终态、W 只是 P4 内的过渡态 |
| D6 | `ownership.rs:627` 每次新建 Z3Backend | **改单例**，与 `predicate.rs:34-36` 的全局 `LazyLock` 统一为 `SolverProvider` | 三种获取策略、两种失败哲学（一个 panic 一个静默）必须收敛 |
| D7 | `layers/README.md` 层序与实际相反 | **修**，归 P4 的 4.4.1 | README 描述的是未建成的意图架构；修正会暴露此前被静默的 `Unproven`，**那是应该暴露的** |

### 类型表示（03）

| # | 议题 | **决定** | 理由 |
| --- | --- | --- | --- |
| D8 | `AssocType`（`ast.rs:463-471`）是否删除 | **删。** 生产零构造（全仓唯一构造在测试 `types/tests/mono.rs:178`，不改变结论），随 T1 门禁独立复核 | 与其余 11 个零构造变体同一处置；未来若启用关联类型语法，由新提案重定义 |
| D9 | T1 门禁用 `syn` 还是 Python | **用 `syn`。** `tools/code-tables` 已是 Rust crate，复用它 | 另建 Python 解析器是重复实现，违反禁令一 |
| D10 | `Type::Void` 兜底（`parser/statements/types.rs:836`）改为什么 | **改为 `Err(Diagnostic)` 返回** | 静默兜底是"生造"的一种——用假值掩盖缺失 |
| D11 | `NameKind` 判定 + LSP 路径 probe | **LSP 复用 `TypeEnvProbe`**，不另开 probe 通道 | `lsp/world.rs` 已持有 `SemanticDB`，复用成本低 |
| D12 | `const_data::BinOp` / `ast::BinOp` 谁生成谁 | **以 `ast::BinOp` 为准**，`const_data` 侧按语义对齐（不强行改名 `Neq`/`Ne`） | AST 是主表示；改名会波及 12 处引用而无收益 |
| D13 | `classify_generic_params` 用 `signature_params` 还是 `Lambda.params`；`ir_gen.rs:1380` 同理 | **不是开放问题，是 P6 阶段 6.1.3 的强制核实项**，结论写进 PR | 这是执行步骤，不是待定策略 |
| D14 | 293 语料中非规范类型名统计 | **同上是任务**（6.1.1），不是开放问题 | 同上 |
| D15 | 阶段 2「不改行为」与「行为变化」的归类 | **按 C3 判据归类：设计变更走独立 commit，措辞变化不计入** | 已在 07 定义 |
| D54 | Fn 标注与函数体的对照缺失（check 绿、运行期类型错） | **声明驱动的检查模式。** 绑定期参数类型按位驱动 lambda 头、出口（尾表达式与 return）统一对照标注返回类型、注册类型=标注形态；parser 名字合并、#295 append、`value_params` 按位填充的个数前置条件退役。表示前提 `Type::Fn.params: Vec<Param>`（名字与类型同栖），随 P6 新增 6.8 落地 | 对照机制现为三个互不衔接的碎片（parser 合并 / 注册标注形态 / 体检查按语法形态分叉），不变量「声明=实现」无人拥有——`f: () -> Int = () => "hello"` 通过而 `f: () -> Int = { "hello" }` 报错（同一语义不同语法），check 绿、运行期 E6007；这是 proof_calls 静默通道（§1.1）在标注子系统的同型发病。对照启用批为行为修复：新诊断、基线更新，不适用 C3 |

### 中间表示（04）

| # | 议题 | **决定** | 理由 |
| --- | --- | --- | --- |
| D16 | `Instruction::Phi` 静态选型 | **方案 A：IR-only，`codegen` 展开为 Move 串** | 不新增 `PHI` opcode，避免 `.42` 格式再扩 |
| D17 | `bytecode.rs:2312` `upvalue_count: 0` | **修；新增字段时把 `.42` 的版本号从 4 升到 5**（格式头已有 `MAGIC` + `VERSION: u32` 字段，`codegen/bytecode.rs:14-16`，读取侧按版本校验） | 这是数据丢失缺陷不是死代码；升版后旧 `.42` 被读取侧按既有行为拒绝（`.42` 是构建产物，跨版本兼容不是目标）；已分配未使用的 opcode 值随升版一并处理 |
| D18 | 删除后 u8/255 槽位上限是否够用 | **批 d 末尾必须实测**；不够则扩到 `u16` | 这是验证项不是待定项 |
| D19 | 线性扫描分配器是否先做原型计时 | **不做原型，直接实现。** 耗时在 CI 中实测 | 计时不改变设计决策，是拖延 |
| D20 | `ReleasePlan` / `overload_resolutions` 的 Span 键控 | **改为 `PlanId`**，消除跨层 span 契约；归 P4 | span 失配导致 Drop 静默丢失，是最隐蔽的一类缺陷 |
| D21 | `method_def_ordinals` 归 03 还是 04 | **归 04**（P7 批 c） | 它是 IR 构造期状态，不是类型问题 |
| D22 | `synth.rs` 边界是否推广到整个 L3 | **推广到整个 L3**；L4 另查后一并纳入 | 边界规则要么完整要么无效 |
| D23 | `compile_pattern` / `eval_const_expr` 是否"不处理" | **纳入批 d 范围**，不是豁免 | 用"标注为不处理"回避工作，是修补性思维 |

### 前端（05）

| # | 议题 | **决定** | 理由 |
| --- | --- | --- | --- |
| D24 | LALRPOP 能否完整复现现有诊断 | **必须复现。** C5 不放宽。用显式错误产生式把 `synchronize()` 的同步点集合与跳过时机写进文法 | 做不到是实现缺陷，**不是放宽判据的理由**；如实报告并重新评估（含"维持 Pratt"），不是改判据 |
| D25 | f-string span 在词法期确定会变，是否可接受 | **不可接受。** span 必须在词法期记录**绝对偏移**，差分比对 `span.file/line` | span 变化会影响诊断定位，不可接受 |
| D26 | 结合性是否需要显式声明 | **需要。** 文法中显式声明左/右结合，不靠 `bp_right = bp_left + 1` 约定 | 当前约定正是 `BP_RANGE` 写死 `(6,7)` 这类魔数的来源 |
| D27 | `is_old_function_syntax`（36 行）处置 | **选项 1：随文法迁移自然删除。** `f(Int) -> Int = ...` 无法匹配任何产生式 | 专门探测已移除语法是反模式 |
| D28 | 阶段 0 是否删 `pratt/mod.rs:11` 的 `pub use precedence::*;` | **删** | 它是 96 行死阶梯的暴露面 |
| D29 | `parse_assign_after_target` 的 8 段职责 | **全部拆完，无一保留。** `apply_semantic_side_effects` 移交给 P6 但**不删** | 拆分是职责重划，不是删除职责 |
| D30 | 阶段 2-4 全量语料耗时 | **P2 基线阶段实测并记录** | 是验证项不是待定项 |
| D53 | 参数位裸标识符（`(Int, Int) -> Int` 的 `Int`）读作什么 | **未命名带型参数。** checker 按类型命名空间解析，解析不到类型报 E；无名签名要求 lambda 头自带参数名（RFC-007 简写规则既有）；RFC-010 形态表与接口示例（`(Surface)`）一并对账。随 P8 新增 8.9 落地 | parser 现读作"推断参数名"（`parse_fn_type_with_names` 无冒号分支），标注零约束，且违反 RFC-007 §25"两边都省略将被拒绝"；std 接口层（`ok: (T) -> Result(T, E)` 等）与语料数十处实证裸形态的既定读法是未命名带型参数，迁移成本为零；强制 `名: 类型`（否决）会迁移整个 std 层且禁掉最直觉写法 |

### 清理与判据（06 / 07 / 08）

| # | 议题 | **决定** | 理由 |
| --- | --- | --- | --- |
| D31 | S1 复活测试暴露的缺陷在哪个 PR 修 | **同一个 PR**，测试复活与缺陷修复一起 | 拆开会诱使后人"下次再修" |
| D32 | `Switch` opcode（解释器有实现、无生产者） | **删 opcode 与解释器实现。** 随 `.42` 升版（D17）一并处理 | 无生产者的 opcode 是死路径，白名单只是掩盖 |
| D33 | `UnaryOp::Not` 静默退化为 `I64_NEG` | **修。** `opcode()` 必须区分 `op` 字段，不能忽略 | 静默改变语义是最危险的一类 |
| D34 | `Instruction::TailCall` 的编码分支 | **删**（全仓无构造点） | 与 D32 同理 |
| D35 | wasm 27 个文件的 cfg 分支 | **删 playground 场景下不可达者，保留可达者。** 归 P10 | wasm 目标已建成（独立 shim crate），分支是承重的，只删不可达部分 |
| D36 | `TRACKING.md` 是否加「实现状态」列 | **加。** `check_tracking.py` 增加生成逻辑 | 当前 26 份 accepted RFC 完全不记录实现状态，RFC-018 就是这么烂掉的 |
| D37 | `docs/superpowers/` 的 § 死引用（gitignored 文档） | **删注释** | 指向一份 gitignore 的计划文档是死引用 |
| D38 | `verify_loose` 若跑不绿 | **不开豁免。** 跑不绿说明 `ir_gen` 存在隐式"同槽多次写"依赖，**那是必须先修的缺陷** | 豁免清单是新的技术债，等于用门禁掩盖设计问题 |
| D39 | 快照基线体积与压缩 | **不引入 git-lfs，接受体积** | 快照是回归判据，可读性优先于体积 |
| D40 | 多文件语料层 | **无条件必做**（P2 的 2.3.2），不是"不做则 P4 暂缓" | 条件句是给自己留后门 |
| D41 | `test_release_plan_spans_consumed` 的白名单 | **无白名单，差集必须为空**（该测试为 P2 新建；仓库现存的 `WHITELIST = ["SWITCH"]` 属 opcode 往返测试，与本项无关）。空不了说明 ReleasePlan 契约有缺陷，修契约（见 D20） | 白名单就是把 bug 合法化 |
| D42 | `verify()` 接 `cargo test` 还是 CI | **CI** | 全语料校验耗时不适合每次本地跑 |
| D43 | 禁令一 A 判据的相似度算法 | **Jaccard ≥ 0.5** | 已在 `08` 正文定，删开放问题 |
| D44 | PR 是否强制声明职责归属 | **是。** PR 模板加必填字段 | 职责判定不可机器化，只能靠流程强制 |
| D45 | `// reason:` 豁免由谁 review | **并入 PR review checklist**，不设专人 | 没有团队分工时设专人等于不设 |

### 项目与流程

| # | 议题 | **决定** | 理由 |
| --- | --- | --- | --- |
| D46 | `RFC-018`（accepted，0 行代码） | **移回 `draft/`** | 按 `rfc/index.md:131` 的定义"已接受 = 进入实现阶段"，它没有 |
| D47 | 阶段与 Issue 映射 | **每个一级阶段开一个 Issue**，`check_tracking.py` 的 `issues_impl` 登记 | |
| D48 | 多文件语料层位置 | **`tests/yaoxiang-multifile/`**（新建） | 扩展 `multifile.rs` 会把语义测试与契约语料混在一起 |
| D49 | 文档站导航 | **加。** `config.js:255` 「工具设计」旁新增一段 | 不加则九篇附属文档不可达 |
| D50 | 阶段并行（多文件 typecheck） | **本轮不做。** 判据稳定后作为独立议题 | 并行会掩盖顺序依赖缺陷，与本轮排查目标冲突 |
| D51 | P3 止血通道 | **允许。** P3 前置收窄为 2.3.2 + 2.4.1 + 2.4.3；P2 其余部分（IR 校验器/快照/单文件差分/性能基线）可与 P3 并行；P4 必须等 P2 全部完成 | 正确性漏洞（`Sorted(3)` 静默通过）的止血不应被快照基础设施建设阻塞；P4 需要三层判据全就位故不放宽 |
| D52 | `.42` 三处硬编码丢弃（`bytecode.rs:2312`/`2315`/`2341`） | **全部收进 P7（7e），不留"独立 issue"。** `2315`（异常表）与 `2341`（全局变量）原定独立 issue，现收编 | 异常表丢失使 throw/try 在 `.42` 直跑时行为错误——throw/try 是语言核心语义；本次重构不留核心功能的待实现遗留。三处同修只需一次 `VERSION` 升版（4→5），比拆成两次升版便宜 |

### 唯一待补数据项

| # | 项 | 说明 |
| --- | --- | --- |
| — | RFC 的 `issue` 字段 | 已补齐：[#430](https://github.com/ChenXu233/YaoXiang/issues/430)（2026-10-05，接受当日建立追踪 issue） |

## 附录：术语表

| 术语 | 定义 |
| --- | --- |
| **阶段（Stage）** | 一次编译中可穷举的一步。阶段集合必须是可穷举枚举，不可运行时注册 |
| **编排层（L1）** | 决定跑哪些阶段、跑几遍、失败如何传播的层 |
| **前端层（L2）** | 词法与语法，产出 AST |
| **中间表示层（L3）** | 类型检查、静态分析、IR 构造 |
| **执行层（L4）** | 字节码生成、解释器、运行时、标准库 |
| **义务（Obligation）** | 某阶段产出、必须被下游消费否则编译失败的契约项 |
| **义务账本（Obligations）** | 全部义务的容器，在阶段表尾部统一结算 |
| **孤儿（Orphan）** | 从未被任何 `mod` 声明引用、因此从未参与编译的文件 |
| **空壳（Shell）** | 已被声明、进入编译产物，但只有文档注释或未使用 `use`、零断言的文件 |
| **等价性判据（Equivalence Oracle）** | 重构前后证明行为未变的可执行检查 |
| **C1-C6** | 重构类别分级，决定判据强度。**不存在 C5′** |
| **职责分离** | 规模问题的唯一解法：一个模块只承担一类职责。**不设行数 / 体积门禁** |
| **决议登记** | 本 RFC 的「决议登记」一节，全部未决项的最终裁决 |

## 附录：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
| --- | --- | --- | --- |
| 分层粒度 | 四层（L1-L4） | 2026-10-03 | ChenXu233 |
| 文档结构 | **1 个 RFC + 9 篇附属设计文档**（`docs/src/dev/architecture/`，与 `check/`、`formatter/` 同级） | 2026-10-03 | ChenXu233 |
| 目录改名 | **做**（D1），不分阶段 | 2026-10-03 | ChenXu233 |
| AST 归属 | **顶层 `src/ast/`**，与 parser 平级（业界多数派；check-boundary 免豁免条款） | 2026-10-04 | ChenXu233 |
| opcode 归属 | **`middle/bytecode/opcode.rs`**，随字节码域合并（词表与格式同域，依赖方向恢复 L4→L3） | 2026-10-04 | ChenXu233 |
| 规模门禁 | **全部取消**。规模由职责分离解决，采纳 Go 官方立场 | 2026-10-03 | ChenXu233 |
| 语法范式 | **完整 LALRPOP 文法驱动**；否决"保留 Pratt 查表化"这个中间态 | 2026-10-03 | ChenXu233 |
| 等价性判据 | **C1-C6 六类分级，不设任何放宽**（不存在 C5′） | 2026-10-03 | ChenXu233 |
| 删除策略 | 允许彻底删除；有真实断言的孤儿测试**复活而非删除** | 2026-10-03 | ChenXu233 |
| 验证方式 | 本组文档不执行 cargo；以静态证据 + 行号为准 | 2026-10-03 | ChenXu233 |
| 阶段顺序 | P0 维护机制 → P1 复活测试 → P2 判据 → … → P10 清理 | 2026-10-03 | ChenXu233 |
| 未决项 | **50 条全部定死**（决议登记 D1-D50），不留取舍 | 2026-10-03 | ChenXu233 |
| P3 止血通道 | **允许**（D51），前置收窄为 2.3.2+2.4.1+2.4.3 | 2026-10-05 | ChenXu233 |
| `.42` 数据丢失三处 | **全部收编 P7（7e），不留独立 issue**（D52） | 2026-10-05 | ChenXu233 |
| 参数位裸标识符 | **未命名带型参数**（D53），构造期拒绝落在"解析不到类型"；随 P8 | 2026-10-05 | ChenXu233 |
| Fn 标注对照 | **声明驱动检查模式**（D54），表示前提 `Type::Fn.params: Vec<Param>`；随 P6 | 2026-10-05 | ChenXu233 |

## 参考文献

- [RFC-010 统一类型语法](../accepted/010-unified-type-syntax.md) — 类型表示收敛的直接来源
- [RFC-011 泛型类型系统](../accepted/011-generic-type-system.md)
- [RFC-011a 接口实现](../accepted/011a-interface-implementation.md)
- [RFC-013 错误码规范](../accepted/013-error-code-specification.md) — 生成期门禁的范例
- [RFC-027 编译期求值与类型](../accepted/027-compile-time-evaluation-types.md)
- [RFC-029 模块语义](../accepted/029-module-semantics.md)
- [RFC-036 测试框架](../accepted/036-test-framework.md)
- [RFC-029a 模块缓存与增量重编译（草案）](../draft/029a-module-cache-incremental.md)
- `src/frontend/core/typecheck/layers/README.md` — 当前层序声明（与实际相反，D7 要求修正）
- `build.rs:19-55` — 本仓已有的 `panic!` 级门禁范例
- `docs/src/dev/design/check/` — 描述了从未存在架构的三篇文档（见 `06-cleanup-inventory.md` §C）
