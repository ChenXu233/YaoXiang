# 功能路由与依赖规范

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/draft/039-compiler-architecture.md) 的附属文档，定义四层模型之内的**模块边界、依赖方向、目标目录结构、功能路由表与防反弹机制**。
>
> 四层模型本身、术语表、执行阶段与验收标准见 RFC-039 正文。分层详细设计见本目录的其余文档。

## 依赖方向规范

以下规则由 CI 检查（见 [防反弹机制](#防反弹机制)）：

| 规则 | 检查方式 | 现状 |
| --- | --- | --- |
| 禁止 `include!` | `grep -rn 'include!' src/` 应为 0 | 1 处（`checker.rs:5618`） |
| L2 不得 `use` L3 | 扫描 `src/frontend/core/{lexer,parser}/` 中的 `use ...typecheck` | 已存在 2 处，需清除（见路由表 C） |
| L3/L4 不得 `use` L2 | 扫描 `sema/`、`proof/`、`middle/`、`backends/` 中的 `use ...frontend::{lexer,parser,module}` | 当前为 0，基线取 0（AST 为顶层域，无需豁免条款） |
| L4 不得反向引用 L1/L2 | 扫描 `src/backends/`、`src/middle/` 中的 `use ...frontend::core::{lexer,parser}` | 需先建立基线 |
| opcode 词表唯一归属 `middle/bytecode/` | 扫描 `src/backends/` 不得定义 opcode 常量（只 import）；`sema/` 不得出现 opcode 引用 | 现状词表在 `backends/common/opcode.rs`（形成 L3→L4 反向），随字节码域合并迁移 |
| `pub(crate)` 跨层泄漏只许减不许增 | 统计并入基线文件 | 至少 1 处（`collect_used_in_type` 被 `inference/statements.rs:16` 使用） |
| 禁止硬编码类型/谓词名 | 扫描 `src/frontend/core/parser/` 内的类型名字符串字面量 | `"Terminates"` 等 |

## 目标目录结构（施工完成后）

以下是**全部施工阶段完成后**的形态，不是第一步的目标。当前形态与目标形态的差距见下方映射表。

```
src/
├── driver/                        # L1 编排：唯一决策点
│   ├── mod.rs                     #   Driver、run()
│   ├── stage.rs                   #   Stage 枚举（可穷举）+ StageScope
│   ├── program.rs                 #   Program：SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations 账本 + assert_drained()
│   ├── unit.rs                    #   Unit：单个编译单元（路径 + 源 + tokens + ast + type_result + ir）
│   └── diagnostics.rs             #   跨阶段诊断聚合 + 退出码判定
│
├── frontend/                      # L2 词法与语法
│   ├── lexer/                     #   声明式：数字统一为 scan_radix(base)，
│   │   ├── mod.rs                 #     转义解码唯一实现（现 literals.rs 的 3 份合一）
│   │   ├── number.rs              #     替代 4 个逐行相同的基数扫描器
│   │   ├── escape.rs              #     唯一 escape 解码器
│   │   ├── string.rs              #     字符串 / 多行字符串
│   │   ├── fstring.rs             #     f-string；插值改为向 parser 递进，不嵌套 tokenize
│   │   └── token.rs               #     TokenKind（单一来源）
│   ├── parser/                    #   文法驱动：单套优先级表，无裸魔数
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     文法定义（文法驱动后新增）
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error 占位节点
│   └── module/                    #   多文件：发现 / 注册表 / 角色
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor 一致性（现 60 行近重复合并）
│       └── discover.rs            #     沿 use 追踪的发现策略
│
├── ast/                           # 唯一 AST + 唯一类型载体（顶层域，与 parser 平级；见 03）
│   ├── mod.rs
│   ├── expr.rs                    #   Expr（收敛后，见 03 关于死变体的处置）
│   ├── stmt.rs                    #   StmtKind（移除 signature_params）
│   ├── type_.rs                   #   唯一类型结构 Type + NameKind（26 → 收敛后变体集）
│   └── pattern.rs
│
├── sema/                          # L3 上半：类型与静态分析（现 typecheck/）
│   ├── types/                     #   MonoType 工作表示 + 求解（见 03 两套角色）
│   │   ├── mod.rs
│   │   ├── repr.rs                #     MonoType 定义（工作表示）
│   │   ├── universe.rs            #     UniverseLevel（现 mono.rs 内的）
│   │   ├── infer.rs               #     推断入口
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     常量求值（含唯一 const_data）
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     唯一 ConstExpr；运算符唯一来源（现 2 套合一）
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   表达式 / 语句推断
│   │   ├── mod.rs                 #     StatementChecker 所在
│   │   ├── call/                  #     拆分后的 infer_call_expr（见 03/04 的拆分轴）
│   │   │   ├── mod.rs             #       按"被调者形态"分派
│   │   │   ├── construct.rs       #       和类型构造 / 泛型类型构造
│   │   │   ├── generic.rs         #       显式类型实参 + 单态化
│   │   │   └── dispatch.rs        #       重载 / 方法解析 / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager（唯一三链模型）
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   模块级检查（现 checker.rs 拆分后）
│   │   ├── mod.rs                 #     TypeChecker 编排
│   │   ├── annotations.rs         #     注解类型名校验 + is_predicate_head
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     导入导出 / 未使用导入
│   │   └── semantic_tokens.rs     #     语义 token 收集（现 include! 拼入）
│   ├── refine/                    #   精化类型（现 checker.rs 3862-5616）
│   │   ├── mod.rs                 #     collect_refined_binding_checks
│   │   ├── binding.rs             #     revalidate_refined
│   │   ├── call_arg.rs            #     check_call_arg_refinements
│   │   ├── return_.rs             #     check_return_refinement
│   │   └── walk.rs                #     refined_walk_* + RefinedWalkCtx
│   └── layers/                    #   跨切面分析
│       ├── README.md              #     必须是实际层序，不是意图层序
│       ├── ownership.rs
│       ├── termination.rs
│       ├── predicate.rs
│       └── equivalence.rs
│
├── proof/                         # L3：证明与验证支撑（从 typecheck/proof/ 提为一级；driver 义务账本持有 proof_calls、sema 三个层共用 SMT 后端，跨层消费故提一级）
│   ├── mod.rs                     #   ProofResult / ProofContext / ProofFunctionCall
│   ├── context.rs
│   ├── verdict.rs
│   ├── dep_graph.rs
│   └── smt/                       #   Solver trait + Z3 后端 + 门禁
│       ├── backend.rs
│       ├── z3_backend.rs
│       └── translate.rs
│
├── middle/                        # L3 下半：IR
│   ├── ir/                        #   SSA IR（见 04）
│   │   ├── mod.rs
│   │   ├── operand.rs             #   Operand（Temp/Label/Register 死变体已删）
│   │   ├── instr.rs               #   Instruction（含 Phi）
│   │   ├── block.rs               #   BasicBlock
│   │   ├── fn_.rs                 #   FunctionIR / LocalSlot
│   │   └── verify.rs              #   verify_loose / verify_ssa
│   ├── lower/                     #   AST → IR（现 ir_gen.rs 拆分后）
│   │   ├── mod.rs                 #     AstToIrGenerator
│   │   ├── scope.rs               #     作用域与寄存器帧（RAII）
│   │   ├── module_ir.rs
│   │   ├── const_eval.rs
│   │   ├── globals.rs
│   │   ├── stmt.rs
│   │   ├── curry.rs
│   │   ├── loops.rs
│   │   ├── query.rs               #     纯查询工具
│   │   ├── expr/                  #     表达式 lowering
│   │   └── synth.rs               #     唯一允许构造 ast::Expr 的地方
│   ├── passes/                    #   IR 上的 pass
│   │   ├── mono/                  #     单态化（删 415 行闭包残余）
│   │   └── ...
│   ├── codegen/                   #   IR → 字节码 lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 栈槽（无真实寄存器文件）
│   └── bytecode/                  #   字节码域：opcode 词表 + 容器 + 编解码
│       ├── opcode.rs              #     唯一 opcode 词表（+ 生成期门禁）
│       ├── file.rs                #     BytecodeFile 序列化（MAGIC / VERSION）
│       ├── mod.rs                 #     BytecodeModule 门面 + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size()（同一文件便于对拍）
│       ├── decode/                #     解码（现 1400 行 From 拆分）
│       │   ├── mod.rs
│       │   ├── control.rs
│       │   ├── arith.rs
│       │   ├── call.rs
│       │   ├── concurrency.rs
│       │   ├── string.rs
│       │   ├── slot.rs
│       │   ├── aggregate.rs
│       │   ├── closure.rs
│       │   ├── rc.rs
│       │   └── field.rs
│       └── typeconv.rs
│
├── backends/                      # L4 执行
│   ├── common/
│   │   ├── heap.rs                #   L4 内部共享：堆布局
│   │   └── value.rs               #   L4 内部共享：runtime 值
│   ├── interpreter/               #   解释器（唯一 Executor 实现）
│   │   ├── executor.rs
│   │   ├── debug.rs               #   opcode 分派表
│   │   ├── ffi.rs
│   │   └── ops/                   #   按指令族切分
│   └── runtime/                   #   DAG 任务调度器（非第二执行后端）
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 标准库（Rust 实现 + .yx 混合）
├── util/                          # 跨层工具（诊断、span、缓存、i18n；保留既有名——08 引用的 Go "避免 util" 为中等适用，改名净 churn 无域收益，记为例外）
├── formatter/                     # 格式化
├── lsp/                           # 语言服务
├── repl/                          # REPL（若保留，见 06 的 D 节）
└── package/                       # 包管理
```

### 每个目录的职责与边界不变量

| 目录 | 职责 | 不变量 |
| --- | --- | --- |
| `driver/` | 决定跑哪些阶段、跑几遍、失败如何传播 | **不得**被 L2/L3/L4 反向依赖；阶段集合必须是可穷举枚举 |
| `frontend/lexer`、`frontend/parser` | 词法与语法，构造 `ast/` 节点 | **不得** `use` `ast` 以外的任何层（`sema` / `middle` / `backends`）。谓词名、类型名、运算符语义一律由数据下传 |
| `ast/` | 唯一的 AST 与类型表示（顶层共享数据域，与 parser 平级） | **不得**出现同一概念的第二份定义；不 `use` 其他层（纯数据与纯函数） |
| `frontend/module/` | 多文件发现、注册表、角色 | 只做发现与注册，**不跑分析阶段**；分析由 `driver` 调度 |
| `sema/` | 类型检查与静态分析 | **不得**构造 IR；不感知 opcode |
| `proof/` | 证明结果类型、SMT 后端 | Solver trait 是唯一后端抽象点；**不得** `.expect()` panic |
| `middle/ir/` | IR 定义与不变量 | IR 满足 SSA 纪律；`verify_ssa` 绿是合入前提 |
| `middle/lower/` | AST → IR | **唯一**允许构造 `ast::Expr` 的地方是 `synth.rs` |
| `middle/codegen/` | IR → 字节码 lowering | 只依赖 IR，不依赖 `sema`；不定义 opcode 词表 |
| `middle/bytecode/` | 字节码域：opcode 词表 + `.42` 容器 + 编解码 | **opcode 词表唯一归属**；`backends/` 只消费不定义；`sema` 不感知 opcode |
| `backends/` | 执行 | **不得**反向引用 L1/L2；不感知 AST |
| `middle/passes/mono/` | IR 上的 pass | 不得引用 `frontend` |
| `formatter/`、`lsp/`、`repl/`、`package/` | 外围工具层（不在 L1-L4 模型内） | 只消费 L1-L4 的 `pub` API；不得触碰 `pub(crate)`（该泄漏计数同样约束它们） |

### 现状 → 目标映射

| 现状 | 目标 | 归 |
| --- | --- | --- |
| `frontend/pipeline.rs`（740 行，5 阶段手写 if 链） | `driver/` + `Program::SingleFile` | 02 |
| `frontend/module/orchestrator.rs`（1528 行，4 入口，60% 分叉） | `frontend/module/` 只留发现/注册；编译逻辑归 `driver` | 02 |
| `frontend/compiler.rs`（267 行） | 瘦身为 `driver` 的薄封装 | 02 |
| `frontend/core/lexer/literals.rs`（1568 行，4 份基数扫描器 + 3 份转义解码） | `frontend/lexer/{number,escape,string,fstring}.rs` | 05 |
| `frontend/core/parser/pratt/`（手写 match 分派，两套 BP 阶梯） | `frontend/parser/grammar/` + 单套优先级表 | 05 |
| `frontend/core/parser/ast.rs`（1166 行：`Expr`/`StmtKind`/`Type`/`Pattern` 定义） | 拆迁顶层 `ast/`（`expr`/`stmt`/`type_`/`pattern`） | 03 / D1 |
| `frontend/core/parser/statements/declarations.rs`（1015 行，8-10 段职责） | `frontend/parser/` 拆分 + 删 120 行死旧语法 | 05 |
| `typecheck/checker.rs`（5618 行 + `include!` 1547 行） | `sema/check/` + `sema/refine/`；`include!` 改真 `mod` | 02 / 05 |
| `typecheck/checker/semantic_tokens.rs`（1547 行，无模块身份） | `sema/check/semantic_tokens.rs`（真模块） | 02 |
| `typecheck/inference/expressions.rs`（5525 行） | `sema/infer/{call/,expr,stmt}.rs` | 05 / 04 |
| `typecheck/inference/statements.rs`（2887 行） | `sema/infer/stmt.rs` + `sema/infer/mod.rs` | 05 |
| `typecheck/types/mono.rs`（1077 行，含同义词表） | `sema/types/{repr,universe}.rs` | 03 |
| `typecheck/types/const_data.rs`（第二套运算符枚举） | 与 `ast::Type` 的运算符合一 | 03 |
| `typecheck/types/inference/types.rs`（死代码 45 行） | 删除 | 06 |
| `typecheck/layers/dispatch.rs`（整链零调用） | 删除 | 06 |
| `typecheck/layers/` | `sema/layers/` + `README.md` 改为实际层序 | 06 |
| `typecheck/proof/` | 提为一级 `proof/` | 02 |
| `middle/core/ir_gen.rs`（8448 行，零测试） | `middle/lower/`（14 个模块） | 04 |
| `middle/core/ir.rs`（905 行） | `middle/ir/` + `verify.rs` | 04 |
| `middle/core/bytecode.rs`（2422 行，1400 行 `From`）+ `middle/passes/codegen/bytecode.rs`（763 行，序列化） | 合并为 `middle/bytecode/`：`opcode.rs` 词表 + `file.rs` 容器序列化 + `decode/`（10 个模块） | 06 |
| `backends/common/opcode.rs`（83 常量词表） | `middle/bytecode/opcode.rs`（消除 L3→L4 反向） | 06 |
| `middle/passes/mono/instance.rs`（415 行死代码） | 删 `416-830`，保留 340 行 | 06 |
| `std/fs.rs`、`std/net.rs` 等（原生 API 绑定） | 位置不变 | — |
| `repl/`（1152 行，零测试，不回显） | 保留或删除，见 06 D 节 | 06 |

**目录改名不是目的。** 变更分两类：

- **必须改的内部结构**：`middle/core/ir_gen.rs`（8448 行）、`typecheck/checker.rs`（7165 行含 `include!`）、`typecheck/inference/expressions.rs`（5525 行）这三个文件的**内部结构**。
- **目录层级：改名。** `typecheck/` → `sema/`、`middle/core/` → `middle/ir/`、`parser/ast.rs` 拆迁顶层 `ast/`（`Type` 定义入 `type_.rs`）、opcode 词表迁 `middle/bytecode/`，随 P5/P6 一并完成（决议 D1）。**不分阶段、不设"可选"。** 理由：目录名是职责的第一眼信号——`middle/core/` 里同时放着 `ir.rs`、`ir_gen.rs`、`bytecode.rs` 三件事，名字已经失效；AST 被全层消费，放 `frontend/` 下会误导下游路径。边界由 CI 检查保证，但**检查规则引用的是目录职责表，职责表必须与实际目录名一致**，否则 `check-boundary.py` 无从判断代码是否放对位置。

### 命名约定

| 规则 | 理由 |
| --- | --- |
| 一个概念只有一个归属模块 | 防止"3 套类型表示"这类平行定义 |
| `foo.rs` 的子模块目录叫 `foo/` | 与 Rust 2018 模块解析一致（`src/frontend/core/typecheck/checker.rs` → `checker/`） |
| 跨层引用必须走 `pub` API，不走 `pub(crate)` 泄漏 | `pub(crate)` 泄漏**计数**作为边界侵蚀指标，只许减不许增（这是语义边界指标，不是行数指标） |
| 内部实现不加 `pub`，除非是跨层 API | 减少可见性面 |
| 类型文件名 `type_.rs`、`fn_.rs` 避开关键字 | Rust 2024 起 `type` / `fn` 是保留字 |
| 不新增 `xxx_core` / `xxx_new` / `xxx_v2` 目录 | 见 06 D 节：靠并行副本表达"新版"是死代码的常见来源 |

## 功能路由表

**回答"我要加 X，应该改哪里"。** 这是本组文档最有长期价值的部分——它不随任何一次重构而失效。

### 路由表 A：语言特性

| 要加的东西 | L2 词法 | L2 语法 | L3 语义 | L3 IR | L4 执行 | 门禁/文档 |
| --- | --- | --- | --- | --- | --- | --- |
| **二元运算符** | `lexer/state.rs` 关键字表、`tokens.rs` 加 `TokenKind` | `pratt/precedence.rs` 加 BP 常量、`led.rs` 的 `infix_info` 加臂、`led.rs` 的 `parse_binary` 加映射、`ast.rs` 加 `BinOp` 变体 | `inference/expressions.rs` 的 `infer_binary` | `lower/` 对应 lowering | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 表 |
| **一元运算符** | 同上 | `nud.rs` 的 `prefix_info` 加臂 + 新 parse 函数 + `ast.rs` 加 `UnOp` 变体 | `infer_unary` | 同上 | 同上 | RFC-010 表 |
| **常量求值运算符** ⚠️ | — | — | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` 是与 `ast::BinOp`/`ast::UnOp` 并行的第二套枚举，二者无任何 `From`/`TryFrom`** | `ir_gen.rs:2317` 需 `use ast::BinOp as B` | — | **无门禁**——见路由表 C |
| **表达式形式** | — | `ast.rs` 加 `Expr` 变体 + `nud.rs`/`led.rs` | `inference/expressions.rs` 加 infer 方法 | `lower/exprs/` 加模块 | `opcode.rs` 或复用 | — |
| **语句形式** | — | `ast.rs` 加 `StmtKind` 变体 + `statements/` | `inference/statements.rs` | `lower/` | 同上 | — |
| **类型构造** | — | `ast.rs` 的 `Type`（见 03） | `types/mono.rs` + `solver.rs` | `FunctionCode.params` 类型 | 序列化 | RFC-011 |
| **模式形式** | — | `pratt/` 的 `parse_pattern` + `expr_to_pattern` | `inference/patterns.rs` | `lower/` 的 `compile_pattern` | `opcode.rs` | RFC-010b |
| **诊断码** | — | — | — | — | — | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 码表**（`build.rs` 门禁会自动校验） |
| **警告（W 前缀）** | — | — | — | — | — | 同上；注意不计入错误数 |

> **现状痛点**：一个二元运算符要改 **6-7 处**（parser/lexer 内）+ 下游 `BinOp::` 的 **17 个生产文件**。漏改能否被发现分三层：
>
> - **编译器会拦的**：对 `ast::BinOp` 做**穷尽 `match`** 的位置，漏改直接编译失败。这类占多数。
> - **编译器抓不到的（真正的盲区）**：`matches!` 宏、`_` 兜底臂、以及**跨枚举**三类。前两类漏改表现为"新运算符被当成别的运算符处理"；第三类（`ast::BinOp` ↔ `const_data::BinOp`）漏改则完全无声——两个类型各自独立编译通过。
> - **L4 不参与**：`backends/interpreter/` 对 `BinOp` 与 `ast::` **零引用**——解释器从不见 AST，它只认 opcode。因此下游压力集中在 L3，不在 L4。
>
> 目标：文法驱动后降到 1-2 处 + 1 处自动生成（见 05）；跨枚举部分由同源生成解决（见 05），bytecode 层的第三套运算符枚举（`BinaryOp`/`UnaryOp`/`CompareOp`）随 opcode 生成期门禁收口（见 06 F8）。

### 路由表 B：编译机制

| 要加的东西 | 落点 | 需要同步改的地方 |
| --- | --- | --- |
| **编译阶段** | `driver/stage.rs` 加 `Stage` 变体 | `driver/stage.rs` 的 `Stage::ALL`、对应 `Program::stages()`、`driver/mod.rs` 的 `dispatch`（穷尽 match 强制补全）、CI 阶段覆盖测试 |
| **跨阶段检查层**（类 ownership / termination） | `sema/layers/` 加模块 | 阶段表登记 + `layers/README.md` 更新（**当前该 README 声明的层序与实际执行顺序相反**）+ 产出并入 `Obligations` |
| **opcode** | `middle/bytecode/opcode.rs`（现状 `backends/common/opcode.rs`，随字节码域合并迁移，触点全部收进 `middle/` 一个目录） | **5 处**：`BytecodeInstr::opcode()`、`size()`、`opcode_name()`、`From<BytecodeFile>` 解码 match、`translator.rs` 的 `translate_*`。其中只有 2 处被编译器穷尽 match 强制，另外 3 处漏改只有跑 `.42` 产物才炸。要求生成期门禁（见 06） |
| **CLI 子命令** | `main.rs` 的 `Commands` 枚举 | `lib.rs` 的 `pub` 入口（当前 7 个）、`main.rs` 的 `match command`（当前 373 行） |
| **LSP 能力** | `lsp/handlers/` 加 handler | `lsp/server.rs` 的 `handle_request`（当前 274 行、13 处重复样板）、`lsp/world.rs` 状态 |
| **标准库 API** | `src/std/*.rs` | **自动门禁已有**：`gen_interfaces.rs` 逐字节比对 + `gen_docs.rs` 标记区间漂移检测。这是全项目最成熟的模式，应推广到 opcode 表与类型表 |

### 路由表 C：已知需要消除的反向依赖与平行定义

| 反向依赖 | 位置 | 消除方式 | 归属文档 |
| --- | --- | --- | --- |
| parser → typecheck（**仅 2 处，同一个函数**） | `parser/statements/declarations.rs:509` 与 `parser/ast.rs:930`，均调用 `typecheck::operator_interfaces::spec()` | 运算符接口规格作为数据下传给 parser，parser 不再 `use` typecheck | 03 |
| parser 内硬编码谓词名 | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`。`:494-497` 注释自述这是待解的开放集问题：「未来的开集方案：把谓词名下传给 parser，或改成语法层可判定的形态」 | 同上，谓词集随数据下传 | 03 |
| **为规避跨模块依赖而制造的重复** | `ast.rs:847 name_used_as_type_in` 与 `declarations.rs:42 name_used_as_type` **同义但独立实现**，`ast.rs:845-846` 注释自认「此处独立实现以避免 parser 内部跨模块依赖」 | 随 03 的 probe 数据流合并为单一实现 | 03 |
| checker → layers 内部函数 | `checker.rs:4857` / `4986` / `5246` / `5384` 调 `layers::termination::{negate_guard, substitute_const_expr}` | 上移到 `sema/refine/` 或 `sema/layers/termination` 的公开 API | 02 |
| 3 套平行类型表示 | `ast::Type`（`ast.rs:427-541`）/ `MonoType`（`types/mono.rs`）/ `ir::Type`（`ir.rs:3` 以 `pub use` 再导出）+ 序列化 `type_table`；靠 `From<MonoType> for IrType`（`bytecode.rs:2353-2390`）搭桥 | 收敛为单一表示 | 03 |
| 手写同义词表 | `types/mono.rs:618-643` `from_builtin_name` 维护 `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))` 等分支 | 随类型表示收敛而删除 | 03 |
| **3 套平行运算符枚举** | `ast.rs:191` `pub enum BinOp`（20 变体）/ `ast.rs:217` `pub enum UnOp`（4，独有 `Deref`）；`types/const_data.rs:234` `pub enum BinOp`（18，`Ne` 而非 `Neq`）/ `:321` `pub enum UnOp`（4，独有 `BitNot`）；`middle/core/bytecode.rs:70` `BinaryOp`（11，`Rem`/`Xor`/`Sar` 又一套命名）+ `:97` `UnaryOp` + `:106` `CompareOp`。**ast ↔ const_data 之间无任何 `From` / `TryFrom`**，只有 `const_eval.rs:185-216` 的手写有损自由函数（`Range`/`Assign`/`BitNot` 等返回 `None` 静默丢弃），迫使 5+ 文件在 import 处起别名（`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`），见 `types/eval/const_eval.rs:19,1061`、`typecheck/layers/tests/ownership.rs:19`、`typecheck/layers/tests/termination.rs:15,226`、`middle/core/ir_gen.rs:2317` | L2 两套由同源生成收敛（见 05）；bytecode 层一套由 opcode 生成期门禁对拍（见 06 F8） | 03 / 05 |
| `include!` | `checker.rs:5618` | 改为真 `mod` | 02 |

## 未来扩展指引

### 加一个语言特性时的检查清单

1. **先查路由表 A**，确定落点。不要凭直觉找文件。
2. **类型先行**：若特性涉及类型，先按 03 的单一表示走，不要新增第三套表示。
3. **门禁同步**：诊断码改动必须同时改 `codes/eNxxx.rs`、`locales/zh.json`、**RFC-013 码表**——`build.rs` 会强制三者一致。
4. **阶段登记**：若特性引入了新的编译期义务，登记进 `Stage` 枚举与 `Obligations`。
5. **反向依赖检查**：新代码不得让 L2 依赖 L3、或 L4 依赖 L1/L2。
6. **测试接线**：新测试目录必须同时在父模块加 `mod` 声明，否则它不会运行——这一点会被新加的 CI 检查拦截。

### 加一个"层"（如未来的新静态检查）时

1. 在 `sema/layers/` 建模块，**不依赖序号更大的层**。
2. 产出物归入 `Obligations`，由编排层统一消费。
3. 更新 `sema/layers/README.md`——**让它描述实际层序**，而不是意图层序。当前该文件描述的是一套未建成的架构。
4. 登记进 `Stage` 枚举，`Program::stages()` 里显式声明在哪些模式下运行。

---

## 维护机制去哪看

防反弹门禁（CI 检查清单、决策程序、三条禁令）已移至 [08-maintenance-mechanism.md](08-maintenance-mechanism.md)。
本篇只回答「东西放哪」：依赖方向规范、目标目录结构、功能路由表、命名约定。
