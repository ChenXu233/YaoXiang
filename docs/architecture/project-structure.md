# YaoXiang 项目架构文档

> 版本：v1.0.0
> 状态：正式
> 作者：晨煦
> 日期：2025-01-04

---

## 目录

1. [概述](#一概述)
2. [目录结构](#二目录结构)
3. [核心模块](#三核心模块)
4. [编译器架构](#四编译器架构)
5. [运行时架构](#五运行时架构)
6. [模块依赖关系](#六模块依赖关系)
7. [开发工作流](#七开发工作流)

---

## 一、概述

YaoXiang 是一个采用 Rust 编写的实验性编程语言项目，采用现代编译器架构设计。项目整体遵循清晰的分层架构，从前端词法分析到后端代码生成，再到运行时执行，每个阶段都有明确的职责边界。

### 设计原则

- **模块化**：每个组件独立，易于测试和替换
- **分层架构**：前端、中端、后端清晰分离
- **零成本抽象**：编译时优化，运行时无额外开销
- **可扩展性**：支持 JIT/AOT 编译，易于添加新特性

---

## 二、目录结构

```
YaoXiang/
├── docs/                           # 文档系统
│   ├── architecture/              # 架构文档（本文件）
│   │   ├── project-structure.md   # 项目结构
│   │   ├── compiler-design.md     # 编译器设计
│   │   └── runtime-design.md      # 运行时设计
│   ├── guides/                    # 用户指南
│   │   ├── getting-started.md     # 快速开始
│   │   ├── error-system-design.md # 错误系统
│   │   └── dev/                   # 开发者指南
│   │       ├── commit-convention.md
│   │       └── release-guide.md
│   ├── works/                     # 工作文档
│   │   ├── old/                   # 历史方案
│   │   ├── phase/                 # 阶段性文档
│   │   └── plans/                 # 规划文档
│   ├── examples/                  # 示例代码
│   └── YaoXiang-book.md           # 语言指南
│
├── src/                           # 源代码
│   ├── frontend/                  # 前端：词法分析、语法分析、类型检查
│   │   ├── lexer/                # 词法分析器
│   │   ├── parser/               # 语法分析器
│   │   └── typecheck/            # 类型检查器
│   │
│   ├── middle/                    # 中端：优化、中间表示、代码生成准备
│   │   ├── ir/                   # 中间表示
│   │   ├── optimizer/            # 优化器
│   │   ├── monomorphize/         # 单态化
│   │   ├── escape_analysis/      # 逃逸分析
│   │   └── codegen/              # 代码生成器（字节码）
│   │
│   ├── runtime/                   # 运行时：执行、内存管理、并发
│   │   ├── dag/                  # 并作图（DAG）
│   │   ├── scheduler/            # 任务调度器
│   │   └── memory/               # 内存管理
│   │
│   ├── vm/                        # 虚拟机：字节码执行
│   │   ├── executor.rs           # 执行器
│   │   ├── frames.rs             # 调用帧
│   │   ├── instructions.rs       # 指令集
│   │   └── opcode.rs             # 操作码定义
│   │
│   ├── std/                       # 标准库
│   │   ├── io.rs                 # 输入输出
│   │   ├── string.rs             # 字符串操作
│   │   ├── list.rs               # 列表操作
│   │   ├── dict.rs               # 字典操作
│   │   ├── math.rs               # 数学函数
│   │   ├── concurrent.rs         # 并发原语
│   │   └── net.rs                # 网络
│   │
│   └── util/                      # 工具库
│       ├── span.rs               # 源码位置
│       ├── diagnostic.rs         # 诊断系统
│       └── cache.rs              # 缓存工具
│
├── tests/                         # 测试
│   ├── integration/              # 集成测试
│   │   ├── codegen.rs           # 代码生成测试
│   │   └── execution.rs         # 执行测试
│   └── unit/                     # 单元测试
│       └── codegen.rs
│
├── Cargo.toml                    # Rust 项目配置
├── clippy.toml                   # Clippy 配置
├── rustfmt.toml                  # 代码格式化配置
├── .gitignore
└── README.md
```

---

## 三、核心模块详解

### 3.1 前端 (src/frontend/)

#### 3.1.1 词法分析器 (lexer/)
**职责**：将源代码字符串转换为 Token 流

**核心文件**：
- `mod.rs` - 词法分析器入口
- `tokens.rs` - Token 定义和类型

**关键数据结构**：
```rust
pub enum Token {
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),
    // 关键字
    Type, Pub, Use, Spawn, Ref, Mut,
    // 符号
    LParen, RParen, LBrace, RBrace,
    // 运算符
    Equal, Arrow, Plus, Minus,
    // ...
}

pub struct Lexer {
    input: String,
    position: usize,
    line: usize,
    column: usize,
}
```

**处理流程**：
1. 按字符读取源代码
2. 识别关键字、标识符、字面量
3. 记录位置信息（行号、列号）
4. 生成 Token 流

#### 3.1.2 语法分析器 (parser/)
**职责**：将 Token 流转换为抽象语法树 (AST)

**核心文件**：
- `mod.rs` - 解析器入口和状态管理
- `ast.rs` - AST 节点定义
- `expr.rs` - 表达式解析
- `stmt.rs` - 语句解析
- `nud.rs`/`led.rs` - 表达式解析核心（Pratt Parser）
- `type_parser.rs` - 类型解析

**关键数据结构**：
```rust
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    Lambda { params: Vec<Param>, body: Box<Expr> },
    Call { func: Box<Expr>, args: Vec<Expr> },
    // ...
}

pub enum Stmt {
    Let { name: String, ty: Option<Type>, value: Expr },
    Function { name: String, params: Vec<Param>, ret_ty: Type, body: Expr },
    TypeDef { name: String, variants: Vec<Variant> },
    // ...
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    state: ParserState,
}
```

**解析策略**：
- **Pratt Parser**：处理表达式的优先级和结合性
- **递归下降**：处理语句和声明
- **双层处理**：解析层宽松，类型检查层严格

#### 3.1.3 类型检查器 (typecheck/)
**职责**：验证 AST 的类型正确性，进行类型推断

**核心文件**：
- `mod.rs` - 类型检查入口
- `types.rs` - 类型定义
- `infer.rs` - 类型推断
- `check.rs` - 类型验证
- `specialize.rs` - 泛型单态化
- `errors.rs` - 类型错误

**关键数据结构**：
```rust
pub enum Type {
    Primitive(PrimitiveType),
    Variable(TypeVar),
    Function { params: Vec<Type>, ret: Box<Type> },
    Generic { name: String, args: Vec<Type> },
    Constructor { name: String, args: Vec<Type> },
    // ...
}

pub struct TypeScheme {
    pub vars: Vec<TypeVar>,
    pub body: Type,
}

pub struct TypeContext {
    pub vars: HashMap<String, TypeScheme>,
    pub constraints: Vec<Constraint>,
}
```

**核心算法**：
- ** Hindley-Milner 类型推断**：支持泛型和多态
- **约束求解**：解决类型变量的约束
- **单态化**：将泛型函数展开为具体版本

### 3.2 中端 (src/middle/)

#### 3.2.1 中间表示 (ir/)
**职责**：定义编译器内部的中间表示形式

**核心文件**：
- `mod.rs` - IR 定义

**IR 特点**：
- SSA (Static Single Assignment) 形式
- 支持控制流图 (CFG)
- 易于优化和转换

#### 3.2.2 优化器 (optimizer/)
**职责**：对 IR 进行各种优化

**优化类型**：
- **常量折叠**：编译时计算常量表达式
- **死代码消除**：移除不可达代码
- **循环优化**：循环展开、不变量外提
- **内联优化**：函数内联

#### 3.2.3 单态化 (monomorphize/)
**职责**：将泛型代码转换为具体版本

**核心文件**：
- `mod.rs` - 单态化入口
- `instance.rs` - 实例管理

**处理流程**：
1. 收集所有泛型函数调用
2. 根据实际类型参数生成具体版本
3. 替换原调用为具体版本

#### 3.2.4 逃逸分析 (escape_analysis/)
**职责**：分析值的生命周期，优化内存分配

**核心文件**：
- `mod.rs` - 逃逸分析器

**作用**：
- 确定哪些值可以栈分配
- 识别需要堆分配的值
- 优化所有权转移

#### 3.2.5 代码生成器 (codegen/)
**职责**：生成字节码

**核心文件**：
- `mod.rs` - 代码生成入口
- `bytecode.rs` - 字节码定义
- `expr.rs` - 表达式代码生成
- `stmt.rs` - 语句代码生成
- `control_flow.rs` - 控制流代码生成
- `loop_gen.rs` - 循环代码生成
- `switch.rs` - 模式匹配代码生成
- `closure.rs` - 闭包处理

**字节码示例**：
```rust
pub enum Instruction {
    // 栈操作
    PushConstant(u32),
    Pop,
    Dup,
    
    // 变量操作
    LoadLocal(u32),
    StoreLocal(u32),
    LoadGlobal(u32),
    StoreGlobal(u32),
    
    // 函数调用
    Call(u32),        // 调用函数
    Return,           // 返回
    TailCall(u32),    // 尾调用优化
    
    // 控制流
    Jump(u32),        // 无条件跳转
    JumpIf(u32),      // 条件跳转
    Loop(u32),        // 循环跳转
    
    // 运算
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    
    // 并发
    Spawn(u32),       // 创建并作任务
    Await,            // 等待异步值
    
    // 类型操作
    Cast,             // 类型转换
    Is,               // 类型检查
    // ...
}
```

### 3.3 运行时 (src/runtime/)

#### 3.3.1 并作图 (dag/)
**职责**：管理并作任务的依赖关系

**核心文件**：
- `mod.rs` - DAG 模块入口
- `node.rs` - 节点定义
- `node_id.rs` - 节点 ID 管理

**关键数据结构**：
```rust
pub struct Node {
    pub id: NodeId,
    pub deps: Vec<NodeId>,        // 依赖的节点
    pub task: Option<Task>,       // 关联的任务
    pub result: Option<Value>,    // 计算结果
    pub status: NodeStatus,       // 节点状态
}

pub enum NodeStatus {
    Pending,      // 等待依赖
    Ready,        // 可以执行
    Running,      // 执行中
    Completed,    // 已完成
    Failed,       // 失败
}
```

#### 3.3.2 任务调度器 (scheduler/)
**职责**：并作任务的调度和执行

**核心文件**：
- `mod.rs` - 调度器入口
- `task.rs` - 任务定义
- `queue.rs` - 工作队列
- `work_stealer.rs` - 工作窃取算法

**调度策略**：
- **工作窃取 (Work Stealing)**：多线程负载均衡
- **惰性求值**：按需执行
- **优先级调度**：重要任务优先

**关键数据结构**：
```rust
pub struct Task {
    pub id: TaskId,
    pub fiber: Fiber,              // 协程/纤程
    pub priority: u8,
    pub affinity: Option<usize>,   // CPU 亲和性
}

pub struct Scheduler {
    pub workers: Vec<Worker>,
    pub global_queue: TaskQueue,
    pub blocking_pool: ThreadPool, // 阻塞任务线程池
}
```

#### 3.3.3 内存管理 (memory/)
**职责**：堆分配、垃圾回收（如果需要）、内存布局

**核心文件**：
- `mod.rs` - 内存管理入口
- `tests/` - 内存测试

**内存策略**：
- **所有权模型**：编译时确定生命周期
- **RAII**：自动资源管理
- **可选 GC**：基于引用计数或标记清除

### 3.4 虚拟机 (src/vm/)

#### 3.4.1 执行器 (executor.rs)
**职责**：执行字节码

**核心功能**：
- 指令分发
- 栈管理
- 寄存器操作
- 异常处理

**执行循环**：
```rust
loop {
    let instr = self.fetch();
    match instr {
        Instruction::PushConstant(idx) => {
            let value = self.get_constant(idx);
            self.push_stack(value);
        }
        Instruction::Add => {
            let b = self.pop_stack();
            let a = self.pop_stack();
            self.push_stack(a + b);
        }
        // ... 其他指令
    }
}
```

#### 3.4.2 调用帧 (frames.rs)
**职责**：管理函数调用栈

**关键数据结构**：
```rust
pub struct CallFrame {
    pub func_idx: u32,          // 函数索引
    pub return_addr: u32,       // 返回地址
    pub base_ptr: u32,          // 栈基址
    pub closure: Option<Closure>, // 闭包环境
}
```

#### 3.4.3 指令集 (instructions.rs)
**职责**：定义所有虚拟机指令

#### 3.4.4 操作码 (opcode.rs)
**职责**：操作码常量定义

### 3.5 标准库 (src/std/)

提供基础功能：
- **io.rs**：文件、控制台 I/O
- **string.rs**：字符串操作
- **list.rs**：动态数组
- **dict.rs**：哈希表
- **math.rs**：数学函数
- **concurrent.rs**：并发原语（Channel, Mutex, RwLock）
- **net.rs**：网络编程

---

## 四、编译器架构

### 4.1 编译流程

```
源代码
  ↓
[词法分析] → Token 流
  ↓
[语法分析] → AST
  ↓
[类型检查] → 带类型的 AST
  ↓
[中间表示] → IR
  ↓
[优化] → 优化后的 IR
  ↓
[单态化] → 具体 IR
  ↓
[逃逸分析] → 内存信息
  ↓
[代码生成] → 字节码
  ↓
[虚拟机执行] → 运行结果
```

### 4.2 关键设计决策

#### 4.2.1 双层处理策略
```
解析层（Frontend）
├─ 只验证语法结构
├─ 宽松处理，不报类型错误
└─ 生成结构化 AST

类型检查层（Typecheck）
├─ 验证语义正确性
├─ 严格的类型推断
└─ 保证运行时安全
```

#### 4.2.2 字节码 vs AOT
- **当前**：字节码 + 虚拟机（灵活性高）
- **未来**：可扩展为 AOT 编译（性能更高）
- **设计**：中间表示可切换后端

#### 4.2.3 并作模型实现
- **编译时**：构建依赖图 (DAG)
- **运行时**：惰性求值 + 自动等待
- **调度器**：工作窃取 + 优先级

---

## 五、运行时架构

### 5.1 并作执行模型

```
用户代码（同步思维）
  ↓
编译器（构建 DAG）
  ↓
运行时（并行执行）
  ├─ 节点 A（独立任务）
  ├─ 节点 B（依赖 A）
  └─ 节点 C（依赖 A 和 B）
  ↓
自动等待（在需要时）
  ↓
结果合并
```

### 5.2 内存布局

```
栈（Stack）
├─ 局部变量
├─ 函数调用帧
└─ 小对象（逃逸分析确定）

堆（Heap）
├─ 大对象
└─ 长生命周期对象

常量池（Constant Pool）
├─ 字符串字面量
└─ 数值字面量
```

### 5.3 线程模型

```
主线程
├─ 执行器（Executor）
├─ 调度器（Scheduler）
└─ 任务队列

工作线程池（可选）
├─ 工作窃取队列
└─ 负载均衡

阻塞线程池
├─ 文件 I/O
├─ 网络 I/O
└─ 其他阻塞操作
```

---

## 六、模块依赖关系

### 6.1 编译阶段依赖

```
frontend/lexer
  ↓ (被 parser 依赖)

frontend/parser
  ↓ (依赖 lexer, 输出 AST)
  ↓ (被 typecheck 依赖)

frontend/typecheck
  ↓ (依赖 parser, 输出带类型 AST)
  ↓ (被 middle 依赖)

middle/ir
  ↓ (依赖 typecheck)
  ↓ (被 optimizer, monomorphize, codegen 依赖)

middle/optimizer
  ↓ (依赖 ir, 修改 IR)

middle/monomorphize
  ↓ (依赖 ir, 生成具体 IR)

middle/codegen
  ↓ (依赖 ir, 生成字节码)
  ↓ (输出给 runtime)

runtime/scheduler
  ↓ (执行字节码)

runtime/dag
  ↓ (管理并作依赖)

vm/
  ↓ (执行字节码)
```

### 6.2 运行时依赖

```
vm/executor
  ↓ (依赖 vm/frames, vm/instructions)

runtime/scheduler
  ↓ (依赖 runtime/task, runtime/dag)

runtime/memory
  ↓ (被所有模块依赖)
```

---

## 七、开发工作流

### 7.1 添加新特性流程

```bash
# 1. 设计阶段
docs/works/plans/your-feature.md  # 设计文档

# 2. 前端实现
src/frontend/lexer/tokens.rs     # 添加 Token
src/frontend/parser/ast.rs       # 添加 AST 节点
src/frontend/parser/expr.rs      # 添加解析逻辑
src/frontend/typecheck/          # 添加类型规则

# 3. 中端实现
src/middle/ir.rs                 # 扩展 IR
src/middle/codegen/              # 生成字节码

# 4. 运行时实现
src/vm/instructions.rs           # 添加指令
src/vm/executor.rs               # 执行逻辑

# 5. 测试
tests/unit/codegen.rs            # 单元测试
tests/integration/               # 集成测试

# 6. 文档更新
docs/YaoXiang-book.md            # 语言指南
docs/architecture/               # 架构文档
```

### 7.2 代码质量检查

```bash
# 格式化
cargo fmt

# Clippy 检查
cargo clippy --all-targets --all-features

# 运行测试
cargo test

# 性能分析
cargo bench
```

### 7.3 版本管理

```
Git 分支策略：
├─ main          # 稳定版本
├─ develop       # 开发分支
├─ feature/*     # 功能分支
└─ hotfix/*      # 紧急修复
```

---

## 八、关键技术决策

### 8.1 为什么使用 Rust？

- **内存安全**：无 GC，无数据竞争
- **零成本抽象**：高性能
- **强类型系统**：编译时保证
- **优秀的工具链**：Cargo, Clippy, rustfmt

### 8.2 为什么选择字节码？

- **跨平台**：一次编译，到处运行
- **可移植性**：易于移植到不同架构
- **灵活性**：支持 JIT 和 AOT
- **调试友好**：易于实现调试器

### 8.3 为什么采用并作模型？

- **自然编程**：同步思维，并发执行
- **零认知负担**：无需管理线程/协程
- **自动优化**：编译器自动提取并行性
- **高性能**：充分利用多核

---

## 九、未来扩展方向

### 9.1 编译器优化
- [ ] LLVM 后端支持（AOT 编译）
- [ ] 更激进的优化（循环展开、向量化）
- [ ] 编译时间优化（增量编译）
- [ ] 错误恢复（继续编译）

### 9.2 语言特性
- [ ] 宏系统（卫生宏）
- [ ] 模块系统完善（包管理器）
- [ ] FFI 支持（C 互操作）
- [ ] 反射系统

### 9.3 工具链
- [ ] LSP 服务器（IDE 支持）
- [ ] 调试器（断点、单步）
- [ ] 性能分析工具
- [ ] 包管理器

### 9.4 运行时
- [ ] GC 优化（分代 GC）
- [ ] 实时编译 (JIT)
- [ ] 并发模型优化
- [ ] 异步 I/O 集成

---

## 十、参考资源

### 10.1 类似项目
- **Rust**：所有权模型、零成本抽象
- **LLVM**：编译器基础设施
- **V8**：JavaScript 引擎（字节码、JIT）
- **Lua**：轻量级虚拟机
- **Erlang**：并作模型、Actor 模型

### 10.2 学术资料
- **类型论**：HM 类型推断、依赖类型
- **编译原理**：龙书、虎书、鲸书
- **并发模型**：CSP、Actor、并行函数式
- **内存管理**：逃逸分析、区域推断

---

## 附录

### A. 术语表

| 术语 | 解释 |
|------|------|
| 并作 (Spawn) | YaoXiang 的异步并发模型 |
| 并作图 (DAG) | 并发任务的依赖关系图 |
| 柯里化 | 将多参数函数转换为单参数函数链 |
| 单态化 | 泛型函数展开为具体版本 |
| 逃逸分析 | 分析值的生命周期和分配位置 |

### B. 常用命令

```bash
# 构建项目
cargo build
cargo build --release

# 运行测试
cargo test
cargo test --test integration

# 代码分析
cargo clippy
cargo doc --open

# 性能分析
cargo bench
cargo flamegraph
```

### C. 贡献指南

1. 阅读架构文档理解整体设计
2. 查看现有代码风格和模式
3. 编写测试保证代码质量
4. 更新相关文档
5. 提交 Pull Request

---

**文档维护**：此文档应随项目演进同步更新，特别是添加新模块或修改架构时。

**最后更新**：2025-01-04
