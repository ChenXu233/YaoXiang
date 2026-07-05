---
title: "RFC-031：优化级别与 Pass 管理器"
status: "草案"
author: "晨煦"
created: "2026-06-16"
updated: "2026-07-05"
---

# RFC-031：优化级别与 Pass 管理器

> **参考**:
> - [RFC-011：泛型系统设计](../accepted/011-generic-type-system.md)
> - [RFC-028：JIT 编译器](./028-jit-compiler.md)
> - [RFC-018：LLVM AOT 编译器](../accepted/018-llvm-aot-compiler.md)

## 摘要

本文档提出为 YaoXiang 引入**优化级别系统**和**Pass 管理器**，将编译优化从"全有或全无"改为可配置的优化包。优化级别（O0-O3）定义不同的优化策略组合，Pass 管理器负责按依赖顺序执行优化 Pass。本文档同时定义优化 Pass 的标准接口，为后续扩展（单态化、内联、常量折叠等）提供架构基础。

**核心目标：让用户在编译速度、二进制大小、运行时性能之间做出明确权衡。**

## 动机

### 为什么需要优化级别？

当前编译器没有优化配置，所有代码都经过相同的处理流程。这导致：

1. **调试体验差**：调试时不需要优化，但无法关闭
2. **无法控制二进制大小**：泛型单态化会膨胀二进制，但无法禁用
3. **编译速度不可控**：无法根据场景选择"快速编译"或"深度优化"
4. **优化 Pass 无序**：未来多个优化 Pass 之间有依赖关系，需要统一管理

### 当前的问题

```yaoxiang
# 当前：所有代码都经过相同处理
# - 调试时：不需要优化，但无法关闭
# - 生产时：需要优化，但无法配置深度
# - 泛型函数：会生成多份代码，但无法控制

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # 会生成 identity_Int
s = identity("hello")   # 会生成 identity_String
# 用户无法选择"不单态化"（类型擦除模式）
```

### 优化级别的价值

| 场景 | 需求 | 优化级别 |
|------|------|----------|
| 开发调试 | 快速编译，保留调试信息 | O0 |
| 日常开发 | 基本优化，平衡编译速度 | O1 |
| 测试/CI | 标准优化，验证生产行为 | O2 |
| 生产发布 | 深度优化，极致性能 | O3 |
| 脚本/快速原型 | 自动选择（根据目标平台） | Auto |

## 提案

### 核心设计

#### 1. 优化级别定义

```rust
/// 优化级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// O0：不优化（调试模式）
    /// - 保留所有调试信息
    /// - 不做任何优化转换
    /// - 最快的编译速度
    /// - 适用：开发调试、快速迭代
    O0,

    /// O1：基本优化（默认）
    /// - 按需单态化（不生成未使用的特化版本）
    /// - 基本常量折叠
    /// - 基本死代码消除
    /// - 适用：日常开发
    #[default]
    O1,

    /// O2：标准优化
    /// - 按需单态化
    /// - 完整常量折叠
    /// - 完整死代码消除
    /// - 小函数内联
    /// - 尾调用优化
    /// - 适用：测试、CI、生产发布
    O2,

    /// O3：激进优化
    /// - 完全单态化（预生成所有可能的类型组合）
    /// - 激进内联
    /// - 所有优化 Pass
    /// - 可能增加编译时间和二进制大小
    /// - 适用：极致性能需求
    O3,

    /// Auto：自动选择
    /// - 根据目标平台和可用资源自动选择优化策略
    /// - 适用：脚本、快速原型
    Auto,
}
```

#### 2. 优化 Pass 接口

```rust
/// 优化 Pass 接口
pub trait OptimizationPass {
    /// Pass 名称（用于日志和依赖声明）
    fn name(&self) -> &str;

    /// 运行 Pass
    fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> PassResult;

    /// 这个 Pass 依赖哪些其他 Pass 必须先运行
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// 这个 Pass 是否应该在当前配置下运行
    fn should_run(&self, config: &PassConfig) -> bool {
        true
    }
}

/// Pass 配置
#[derive(Debug, Clone)]
pub struct PassConfig {
    /// 优化级别
    pub opt_level: OptLevel,
    /// 是否启用调试信息
    pub debug_info: bool,
    /// 目标平台
    pub target_platform: TargetPlatform,
}

/// Pass 运行结果
#[derive(Debug, Default)]
pub struct PassResult {
    /// 是否修改了 IR
    pub changed: bool,
    /// 统计信息
    pub stats: PassStats,
}

/// Pass 统计信息
#[derive(Debug, Default)]
pub struct PassStats {
    /// 内联的函数数量
    pub functions_inlined: usize,
    /// 单态化的函数数量
    pub functions_monomorphized: usize,
    /// 消除的死代码数量
    pub dead_code_removed: usize,
    /// 折叠的常量数量
    pub constants_folded: usize,
}
```

#### 3. Pass 管理器

```rust
/// 优化器
pub struct Optimizer {
    /// 注册的 Pass 列表（按依赖顺序排序）
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// 根据优化级别创建优化器
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// 创建指定级别的 Pass 列表
    fn create_passes_for_level(level: OptLevel) -> Vec<Box<dyn OptimizationPass>> {
        match level {
            OptLevel::O0 => {
                vec![
                    // 调试模式：最少优化，只做必要的清理
                    Box::new(ConstFoldPass::minimal()),
                ]
            }
            OptLevel::O1 => {
                vec![
                    // 基本优化
                    Box::new(ConstFoldPass::basic()),
                    Box::new(MonomorphizePass::on_demand()),
                    Box::new(DcePass::basic()),
                ]
            }
            OptLevel::O2 => {
                vec![
                    // 标准优化
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::on_demand()),
                    Box::new(DcePass::full()),
                    Box::new(InlinePass::small_functions()),
                    Box::new(TcoPass::new()),
                ]
            }
            OptLevel::O3 => {
                vec![
                    // 激进优化
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::full()),
                    Box::new(InlinePass::aggressive()),
                    Box::new(DcePass::full()),
                    Box::new(TcoPass::new()),
                    // 更多激进优化...
                ]
            }
            OptLevel::Auto => {
                // 自动选择：根据目标平台决定
                Self::create_passes_for_level(OptLevel::O1)
            }
        }
    }

    /// 运行所有优化 Pass
    pub fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> OptimizerResult {
        let mut total_stats = OptimizerStats::default();

        for pass in &self.passes {
            if !pass.should_run(config) {
                continue;
            }

            let result = pass.run(module, config);
            total_stats.merge(result.stats);
        }

        OptimizerResult {
            module: module.clone(),
            stats: total_stats,
        }
    }
}
```

### 示例

#### 命令行使用

```bash
# 调试模式：不优化
yaoxiang build --opt-level O0

# 日常开发：基本优化（默认）
yaoxiang build

# 生产发布：标准优化
yaoxiang build --opt-level O2

# 极致性能：激进优化
yaoxiang build --opt-level O3

# 自动选择
yaoxiang build --opt-level Auto
```

#### 配置文件

```json
{
  "optimization_level": "O2",
  "mono": {
    "enabled": true,
    "strategy": "OnDemand"
  },
  "debug_info": false
}
```

#### API 使用

```rust
use yaoxiang::frontend::{Compiler, CompileConfig, OptLevel};

// 调试模式
let config = CompileConfig::new()
    .with_opt_level(OptLevel::O0);
let mut compiler = Compiler::with_config(config);

// 生产模式
let config = CompileConfig::new()
    .with_opt_level(OptLevel::O2);
let mut compiler = Compiler::with_config(config);
```

### 语法变化

无语法变化。优化级别是编译器配置，不影响语言语法。

## 详细设计

### 优化级别与 Pass 映射

| Pass | O0 | O1 | O2 | O3 | 说明 |
|------|----|----|----|----|----|
| **常量折叠** | 最小 | 基本 | 完整 | 完整 | 编译期计算常量表达式 |
| **单态化** | ❌ | 按需 | 按需 | 完全 | 泛型函数特化 |
| **死代码消除** | ❌ | 基本 | 完整 | 完整 | 移除未使用的代码 |
| **函数内联** | ❌ | ❌ | 小函数 | 激进 | 将函数体插入调用点 |
| **尾调用优化** | ❌ | ❌ | ✅ | ✅ | 尾递归转循环 |
| **逃逸分析** | ❌ | ❌ | ❌ | ✅ | 决定栈/堆分配 |
| **循环优化** | ❌ | ❌ | ❌ | ✅ | 循环展开、不变量外提 |

### 单态化策略

```rust
/// 单态化策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// 不单态化 — 类型擦除，泛型函数只有一份代码
    /// 优点：二进制小，编译快
    /// 缺点：运行时有动态分发开销
    Erased,

    /// 按需单态化 — 只对实际使用的类型组合生成代码
    /// 优点：零开销抽象，无运行时开销
    /// 缺点：二进制可能膨胀
    #[default]
    OnDemand,

    /// 完全单态化 — 预生成所有可能的类型组合
    /// 优点：编译期确定所有调用
    /// 缺点：编译慢，二进制大
    Full,
}

/// 单态化配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonoConfig {
    /// 是否启用单态化
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 单态化策略
    #[serde(default)]
    pub strategy: MonoStrategy,

    /// 是否启用 DCE（死代码消除）
    #[serde(default = "default_true")]
    pub dce_enabled: bool,

    /// 最大特化深度（防止无限递归泛型）
    #[serde(default = "default_max_mono_depth")]
    pub max_depth: usize,
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: MonoStrategy::OnDemand,
            dce_enabled: true,
            max_depth: 100,
        }
    }
}
```

### 编译流程集成

```rust
// src/frontend/pipeline.rs

impl Pipeline {
    fn run_ir_generation(
        &mut self,
        source_name: &str,
        source: &str,
        ast: &Module,
        type_result: &TypeCheckResult,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> IRResult {
        let start = Instant::now();

        // 1. 生成基础 IR
        let mut ir = middle::generate_ir(ast, type_result)?;

        // 2. 根据优化级别运行优化 Pass
        let optimizer = Optimizer::for_opt_level(self.config.optimization_level);
        let pass_config = PassConfig {
            opt_level: self.config.optimization_level,
            debug_info: self.config.generate_debug_info,
            target_platform: TargetPlatform::detect(),
        };

        let result = optimizer.run(&mut ir, &pass_config);

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::Optimization, duration));

        IRResult::success(result.module)
    }
}
```

### 类型系统影响

无直接影响。优化 Pass 在 IR 层运行，不影响类型系统。

### 运行时行为

| 优化级别 | 运行时行为 |
|----------|-----------|
| O0 | 无优化，保留所有调试信息 |
| O1 | 基本优化，保留基本调试信息 |
| O2 | 标准优化，无调试信息 |
| O3 | 激进优化，无调试信息 |

**关键点：运行时不需要修改**。优化 Pass 只影响 IR 层和代码生成层，运行时通过函数名/ID 查找执行，不感知优化过程。

### 编译器改动

| 组件 | 改动 |
|------|------|
| `frontend/config.rs` | 新增 `OptLevel` 枚举和 `MonoConfig` |
| `frontend/pipeline.rs` | 集成 Pass 管理器 |
| `middle/passes/optimizer/` | 新增优化 Pass 模块 |
| `middle/passes/mono/` | 重构为标准 Pass 接口 |
| CLI | 新增 `--opt-level` 参数 |

### 向后兼容性

- ✅ 完全向后兼容
- 默认优化级别为 O1，行为与当前一致
- 用户可显式指定优化级别覆盖默认行为

## 权衡

### 优点

- **灵活性**：用户可根据场景选择优化策略
- **可扩展性**：标准 Pass 接口，易于添加新优化
- **可预测性**：明确每个优化级别的行为
- **调试友好**：O0 模式保留完整调试信息

### 缺点

- **复杂度增加**：需要维护多个优化级别
- **测试矩阵增大**：需要测试每个优化级别的行为
- **文档负担**：需要解释每个优化级别的含义

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 只有开/关两种状态 | 无法精细控制优化深度 |
| 使用 GCC/LLVM 风格的 `-O` 数字 | 与 YaoXiang 的配置系统不一致 |
| 每个优化 Pass 独立开关 | 用户需要了解每个 Pass 的细节，使用复杂 |
| 延迟到 v2.0 | 单态化已实现但未集成，需要先解决架构问题 |

## 实现策略

### 阶段划分

1. **阶段 1（当前）**：定义优化级别和 Pass 接口
2. **阶段 2**：实现单态化 Pass（基于现有 `mono/` 模块）
3. **阶段 3**：实现常量折叠和死代码消除 Pass
4. **阶段 4**：实现函数内联和尾调用优化 Pass
5. **阶段 5**：实现激进优化 Pass（逃逸分析、循环优化）

### 依赖关系

- 依赖 RFC-011（泛型系统）的单态化模块
- 依赖 RFC-028（JIT 编译器）的优化 Pass 接口
- 与 RFC-018（LLVM AOT）共享优化 Pass 设计

### 风险

- **性能回归**：优化 Pass 可能引入 bug，导致性能下降
- **编译时间增加**：优化 Pass 增加编译时间
- **二进制膨胀**：单态化可能导致二进制大小显著增加

## 开放问题

- [ ] O3 级别是否应该默认启用逃逸分析？（@晨煦：需要性能测试数据）
- [ ] 是否需要 `Os`（优化大小）和 `Oz`（极致优化大小）级别？
- [ ] 优化级别是否应该影响调试信息的详细程度？
- [ ] 如何处理优化 Pass 之间的循环依赖？

---

## 附录A：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 优化级别命名 | 使用 O0-O3 + Auto | 2026-06-16 | 晨煦 |
| 默认优化级别 | O1（基本优化） | 2026-06-16 | 晨煦 |
| 单态化策略 | 支持 Erased/OnDemand/Full | 2026-06-16 | 晨煦 |
| Pass 接口设计 | trait + 依赖声明 | 2026-06-16 | 晨煦 |

---

## 附录B：术语表

| 术语 | 定义 |
|------|------|
| **优化 Pass** | 对 IR 进行一次转换的独立模块 |
| **单态化** | 将泛型函数特化为具体类型的代码生成策略 |
| **常量折叠** | 在编译期计算常量表达式 |
| **死代码消除** | 移除程序中不可达或未使用的代码 |
| **函数内联** | 将函数体插入调用点，避免函数调用开销 |
| **尾调用优化** | 将尾递归转换为循环，避免栈溢出 |
| **逃逸分析** | 分析变量是否逃逸出作用域，决定栈/堆分配 |

---

## 参考文献

- [Rust 编译器优化](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC 优化级别](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan 优化管线](https://v8.dev/docs/turbofan)

---

## 生命周期与归宿

本 RFC 定义优化级别的架构设计，为后续优化 Pass 提供统一框架。

**与单态化的关系**：单态化是优化 Pass 之一，将在本 RFC 接受后作为第一个实现的 Pass。
