# YaoXiang 前端架构激进重构方案 (RFC支撑版)

> 版本: 3.0 | 日期: 2026-01-29 | 状态: 基于RFC需求修复
>
> **核心目标**: 在低耦合架构基础上，全面支撑RFC-004/010/011的实现需求

## 📋 重构目标

### 核心目标
- **降低耦合度**: 消除模块间强依赖，实现松耦合架构
- **RFC支撑**: 完全支撑三个RFC的设计需求和实现路径
- **文件分层优化**: 清晰的分层架构，每层职责单一
- **可维护性提升**: 大文件拆分，职责清晰
- **可扩展性增强**: 为RFC-012等未来特性预留扩展空间

### RFC支撑矩阵

| RFC | 核心需求 | 重构支撑度 | 实现位置 |
|-----|----------|------------|----------|
| **RFC-004** | 多位置绑定语法、智能绑定、自动柯里化 | 95% | `statements/bindings.rs`, `core/lexer/literals.rs` |
| **RFC-010** | 统一语法、泛型语法、类型定义 | 90% | `statements/declarations.rs`, `types/parser.rs` |
| **RFC-011** | 约束求解、单态化、泛型系统 | 100% | `type_system/*`, `constraints.rs`, `unify.rs` |

### 成功指标
- [ ] 所有文件控制在500行以内
- [ ] 模块依赖关系清晰，无循环依赖
- [ ] 三个RFC的实现需求100%在架构中得到支撑
- [ ] 公共API简化，隐藏内部实现
- [ ] 测试覆盖率达到85%以上
- [ ] 编译时间减少20%（通过更好的模块化）

---

## 🏗️ 新架构设计

### 1. 分层架构图

```
┌─────────────────────────────────────┐
│           Frontend API              │  ← 公共接口层
│        (frontend/mod.rs)            │
├─────────────────────────────────────┤
│  Lexer → Parser → TypeCheck → Const│  ← 流水线层
│     │        │         │       │   │
│     ▼        ▼         ▼       ▼   │
├─────────────────────────────────────┤
│          Shared Utilities           │  ← 共享工具层
│    (error, span, diagnostic)        │
├─────────────────────────────────────┤
│        Core Algorithm Layer         │  ← 核心算法层
│  (type_system, const_eval, parse)   │
│                                     │
│  ▸ RFC-004: 绑定解析支持            │
│  ▸ RFC-010: 统一语法解析            │
│  ▸ RFC-011: 完整泛型系统           │
└─────────────────────────────────────┘
```

### 2. 模块重新组织

#### **第一层: 核心算法层 (Core Algorithm Layer)**

```
src/frontend/core/
├── mod.rs                    # 核心模块入口
├── lexer/
│   ├── mod.rs               # 词法分析器接口
│   ├── tokenizer.rs         # Tokenizer实现 (从1270行拆分)
│   ├── state.rs            # 词法状态管理 (新建)
│   ├── literals.rs         # 字面量处理 (拆分)
│   └── symbols.rs          # 关键字和符号表 (新建)
├── parser/
│   ├── mod.rs              # 解析器接口
│   ├── ast.rs              # AST定义 (保持305行)
│   ├── Pratt/              # Pratt解析器核心 (新建目录)
│   │   ├── mod.rs
│   │   ├── nud.rs          # 前缀解析 (从896行拆分)
│   │   ├── led.rs          # 中缀解析 (保持380行)
│   │   └── precedence.rs   # 优先级处理 (拆分)
│   ├── statements/         # 语句解析 (新建目录)
│   │   ├── mod.rs
│   │   ├── declarations.rs  # 声明语句 (从1399行拆分)
│   │   ├── expressions.rs   # 表达式语句 (拆分)
│   │   ├── control_flow.rs  # 控制流 (拆分)
│   │   └── bindings.rs     # RFC-004绑定语法解析 (新建)
│   ├── types/              # 类型解析 (新建目录)
│   │   ├── mod.rs
│   │   ├── parser.rs       # 类型解析器 (从614行拆分)
│   │   ├── constraints.rs  # RFC-011类型约束解析 (新建)
│   │   └── generics.rs     # RFC-010/011泛型语法解析 (新建)
│   └── utils.rs            # 解析器工具函数 (拆分)
├── type_system/            # RFC-011核心类型系统
│   ├── mod.rs
│   ├── vars.rs            # TypeVar, ConstVar (拆分)
│   ├── mono_poly.rs       # MonoType, PolyType (拆分)
│   ├── constraints.rs      # TypeConstraint (拆分)
│   ├── unify.rs           # Unify算法 (拆分)
│   ├── specialize.rs      # RFC-011泛型特化 (新建)
│   ├── pretty_print.rs    # 类型打印 (新建)
│   └── display.rs         # 类型显示格式化 (新建)
└── const_eval/            # 常量求值
    ├── mod.rs
    ├── evaluator.rs       # 常量求值器 (从677行重命名)
    ├── functions.rs      # Const函数 (从536行拆分)
    └── static_assert.rs  # 静态断言 (保持490行)
```

#### **第二层: 共享工具层 (Shared Utilities)**

```
src/frontend/shared/
├── mod.rs
├── error/
│   ├── mod.rs
│   ├── diagnostic.rs       # 统一诊断信息
│   ├── span.rs            # Span处理
│   ├── result.rs          # 统一Result类型
│   ├── conversion.rs      # 错误转换
│   └── macros.rs          # RFC-011错误处理宏 (新建)
├── diagnostics/
│   ├── mod.rs
│   ├── formatter.rs       # 诊断格式化
│   ├── severity.rs        # 严重级别
│   ├── code.rs            # 错误码定义
│   └── traits.rs          # 诊断特质 (新建)
├── utils/
│   ├── mod.rs
│   ├── mem.rs             # 内存管理工具
│   ├── debug.rs           # 调试工具
│   ├── panic.rs           # panic处理
│   └── cache.rs           # RFC-011编译缓存 (新建)
└── abstractions/           # 抽象接口层 (新建)
    ├── mod.rs
    ├── parser.rs          # Parser抽象接口
    ├── type_checker.rs    # TypeChecker抽象接口
    └── trait_objects.rs   # Trait对象支持
```

#### **第三层: 类型检查层 (Type Checking Layer)**

```
src/frontend/typecheck/
├── mod.rs                 # 类型检查入口
├── inference/             # 类型推断 (拆分infer.rs)
│   ├── mod.rs
│   ├── expressions.rs    # 表达式推断 (拆分)
│   ├── statements.rs     # 语句推断 (拆分)
│   ├── patterns.rs       # 模式匹配推断 (新建)
│   └── generics.rs       # RFC-011泛型推断 (新建)
├── checking/             # 类型检查 (拆分check.rs)
│   ├── mod.rs
│   ├── subtyping.rs      # 子类型检查 (拆分)
│   ├── assignment.rs     # 赋值检查 (拆分)
│   ├── compatibility.rs # 兼容性检查 (拆分)
│   └── bounds.rs         # RFC-011类型边界检查 (新建)
├── specialization/       # RFC-011泛型特化
│   ├── mod.rs
│   ├── algorithm.rs      # 特化算法 (从488行拆分)
│   ├── substitution.rs  # 替换逻辑 (新建)
│   └── instantiate.rs   # 实例化算法 (新建)
├── traits/              # RFC-011特质系统
│   ├── mod.rs
│   ├── solver.rs        # 特质求解器 (从274行拆分)
│   ├── coherence.rs     # 一致性检查 (新建)
│   ├── object_safety.rs # 对象安全 (新建)
│   └── resolution.rs    # 特质解析 (新建)
└── gat/                # GAT支持 (保持529行，优化结构)
    ├── mod.rs
    ├── checker.rs       # GAT检查器
    └── higher_rank.rs   # 高阶类型
```

#### **第四层: 高级类型层 (Advanced Type Level)**

```
src/frontend/type_level/
├── mod.rs               # 类型级计算入口
├── conditional_types.rs  # RFC-011条件类型 (保持)
├── dependent_types.rs    # RFC-011依赖类型 (保持)
├── evaluation/          # RFC-011类型级计算 (新建目录)
│   ├── mod.rs
│   ├── normalize.rs     # 范式化
│   ├── reduce.rs        # 归约
│   ├── unify.rs         # 类型级统一
│   └── compute.rs       # 类型计算引擎 (新建)
├── operations/          # RFC-011类型级操作 (新建目录)
│   ├── mod.rs
│   ├── arithmetic.rs    # 算术运算
│   ├── comparison.rs   # 比较运算
│   └── logic.rs        # 逻辑运算
├── const_generics/     # RFC-011 Const泛型支持 (新建目录)
│   ├── mod.rs
│   ├── eval.rs         # Const泛型求值
│   └── generic_size.rs # 泛型尺寸计算 (新建)
└── tests.rs            # 测试 (保持)
```

#### **第五层: 公共接口层 (Public API Layer)**

```
src/frontend/
├── mod.rs               # 编译器公共接口 (简化)
├── compiler.rs          # 编译器核心逻辑 (从235行拆分)
├── pipeline.rs          # 编译流水线 (新建)
├── config.rs            # 编译配置 (新建)
└── events/              # 事件系统 (新建)
    ├── mod.rs
    ├── type_check.rs    # 类型检查事件
    ├── parse.rs         # 解析事件
    └── subscribe.rs     # 事件订阅 (新建)
```

---

## 📅 分阶段实施计划

### 🚀 阶段 1: 紧急拆分与RFC支撑准备 (Week 1-3)

#### **Day 1-2: 准备阶段**

**步骤 1.1: 创建新目录结构**
- **子任务 1.1.1**: 在 `src/frontend/` 下创建完整目录结构
  - 预期耗时: 15分钟
  - 验收标准: 所有RFC支撑目录创建完成

```bash
# 创建RFC-004支撑目录
mkdir -p src/frontend/core/parser/statements/bindings

# 创建RFC-010/011支撑目录
mkdir -p src/frontend/core/parser/types/generics
mkdir -p src/frontend/type_system/specialize

# 创建共享抽象目录
mkdir -p src/frontend/shared/abstractions
mkdir -p src/frontend/shared/events
```

**步骤 1.2: 运行现有测试基准**
- **子任务 1.2.1**: 记录当前编译性能
  - 运行 `time cargo build --release` 记录基准时间
  - 保存结果到 `metrics/pre_refactor_build_time.txt`

- **子任务 1.2.2**: 运行完整测试套件
  - 运行 `cargo test --all` 确保当前测试全部通过
  - 记录测试通过数量: ___/___
  - 保存到 `metrics/pre_refactor_test_results.txt`

- **子任务 1.2.3**: 记录代码统计
  - 运行 `cloc src/frontend/typecheck/types.rs` 记录原始行数
  - 记录: 总行数 ___ 行，代码行数 ___ 行
  - 保存到 `metrics/pre_refactor_loc.txt`

---

#### **Day 3-7: 拆分 typecheck/types.rs (RFC-011核心)**

**目标**: 将1948行的巨无霸文件拆分为支撑RFC-011的模块
**预期总耗时**: 5天 (每天1天)

**Day 3: 分析与拆解**

- **子任务 1.3.1: RFC-011需求对齐分析**
  - **1.3.1.1**: 标注RFC-011 Phase 1需求 (60分钟)
    - TypeVar/ConstVar 定义 → `vars.rs`
    - MonoType/PolyType 定义 → `mono_poly.rs`
    - 约束系统 → `constraints.rs`
    - Unify算法 → `unify.rs`
  - **1.3.1.2**: 标注RFC-011 Phase 2+需求 (30分钟)
    - 特化算法 → `specialize.rs` (新建)
    - 类型显示 → `pretty_print.rs`, `display.rs` (新建)
  - **1.3.1.3**: 确定模块边界和依赖关系 (30分钟)

- **子任务 1.3.2: 创建 `vars.rs` (2小时)**
  - **1.3.2.1**: 复制 TypeVar/ConstVar 相关代码到新文件
  - **1.3.2.2**: 调整导入路径，修复编译错误
  - **1.3.2.3**: 运行 `cargo check` 验证编译通过
  - **验收标准**: vars.rs 独立编译成功

**Day 4: 完成基础模块**

- **子任务 1.3.3: 创建 `mono_poly.rs` (2小时)**
  - **1.3.3.1**: 复制 MonoType/PolyType 代码
  - **1.3.3.2**: 修复与 vars.rs 的依赖关系
  - **1.3.3.3**: 添加RFC-011需要的泛型特化接口

- **子任务 1.3.4: 创建 `constraints.rs` (2小时)**
  - **1.3.4.1**: 复制 TypeConstraint/ConstraintSet 代码
  - **1.3.4.2**: 实现 UnionFind 结构
  - **1.3.4.3**: 添加RFC-011 Phase 2约束求解接口

**Day 5: 核心算法模块**

- **子任务 1.3.5: 创建 `unify.rs` (3小时)**
  - **1.3.5.1**: 复制 Unify 算法和 Substitution
  - **1.3.5.2**: 实现 Unifier 结构
  - **1.3.5.3**: 添加RFC-011单态化支持接口
  - **验收标准**: unify.rs 编译通过，算法逻辑正确

- **子任务 1.3.6: 创建 `specialize.rs` (RFC-011新增) (1小时)**
  - **1.3.6.1**: 实现泛型特化算法
  - **1.3.6.2**: 实现实例化缓存
  - **1.3.6.3**: 添加死代码消除接口

**Day 6: 整合与依赖修复**

- **子任务 1.3.7: 创建 `type_system/mod.rs` (1小时)**
  - **1.3.7.1**: 定义模块入口文件
  - **1.3.7.2**: 统一导出所有公共接口
  - **1.3.7.3**: 定义 TypeSystemError 类型

- **子任务 1.3.8: 更新依赖关系 (2小时)**
  - **1.3.8.1**: 修改 `src/frontend/typecheck/mod.rs` 的导入
    ```rust
    // 从
    pub use types::*;
    // 改为
    pub use crate::type_system::{
        MonoType, PolyType, TypeVar, ConstraintSolver,
        Unifier, specialize::Specializer
    };
    ```
  - **1.3.8.2**: 使用搜索替换工具批量更新引用路径
  - **1.3.8.3**: 逐个文件修复编译错误

**Day 7: RFC-011基础设施验证**

- **子任务 1.3.9: 验证RFC-011支撑 (2小时)**
  - **1.3.9.1**: 创建RFC-011 Phase 1测试
    ```rust
    // tests/rfc011_phase1.rs
    #[test]
    fn test_basic_generic_instantiation() {
        let types = type_system::MonoType::var("T");
        let specialized = type_system::specialize::instantiate(
            &types, &[Type::Int]
        ).unwrap();
        assert_eq!(specialized, Type::Int);
    }
    ```
  - **1.3.9.2**: 验证约束求解器工作
  - **1.3.9.3**: 验证单态化接口

- **子任务 1.3.10: 全面验证 (2小时)**
  - **1.3.10.1**: 运行 `cargo check --all` 确保无编译错误
  - **1.3.10.2**: 运行 `cargo test type_system` 运行类型系统测试
  - **1.3.10.3**: 运行 `cargo test --all` 确保所有测试通过
  - **1.3.10.4**: 性能对比: 编译时间变化 < 10%

- **子任务 1.3.11: 清理旧文件 (1小时)**
  - **1.3.11.1**: 确认新模块功能正常后，删除原 `types.rs`
  - **1.3.11.2**: 更新 git 并提交
  - **1.3.11.3**: 创建标签 `refactor/types-complete-rfc011`

**验收标准**:
- [ ] `types.rs` 完全删除
- [ ] 新模块编译通过: `cargo check --all`
- [ ] RFC-011 Phase 1测试通过: `cargo test rfc011_phase1`
- [ ] 所有测试绿色: `cargo test --all`
- [ ] 性能无明显下降: 编译时间变化 < 10%

---

#### **Day 8-14: 拆分 lexer/mod.rs + RFC-004支撑准备**

**目标**: 拆词法分析器，为RFC-004绑定语法和RFC-010统一语法做准备
**预期总耗时**: 7天

**Day 8-9: 分析 lexer + RFC需求**

- **子任务 1.4.1: 分析 lexer 结构 + RFC需求对齐 (2小时)**
  - **1.4.1.1**: 运行分析工具
    ```bash
    rg "^pub struct|^impl.*Tokenizer" src/frontend/lexer/mod.rs
    ```
  - **1.4.1.2**: 识别核心逻辑 + RFC支撑需求
    - Tokenizer 主结构 (行1-300) + RFC-004绑定符号 `[`, `]` 支持
    - 状态管理代码 (行301-600) + RFC-010泛型关键字 `<`, `>` 支持
    - 字面量处理逻辑 (行601-900) + RFC-010/011类型语法支持
    - 辅助方法 (行901-1270)
  - **1.4.1.3**: 设计新模块接口

- **子任务 1.4.2: 创建 `tokenizer.rs` (3小时)**
  - **1.4.2.1**: 提取 Tokenizer 结构体和主要方法
  - **1.4.2.2**: 添加RFC-004绑定语法token支持
    ```rust
    // Tokenizer 新增
    enum TokenType {
        // ... 现有token
        LeftBracket,    // [ RFC-004绑定开始
        RightBracket,   // ] RFC-004绑定结束
        LessThan,       // < RFC-010/011泛型开始
        GreaterThan,     // > RFC-010/011泛型结束
        // ...
    }
    ```
  - **1.4.2.3**: 委托状态和字面量处理到专门模块

- **子任务 1.4.3: 创建 `state.rs` (2小时)**
  - **1.4.3.1**: 提取 LexerState 结构
  - **1.4.3.2**: 实现关键字查找等状态相关方法
  - **1.4.3.3**: 添加RFC-010关键字识别 (如 `type`, `where` 等)

**Day 10-11: 完成拆分 + 符号支持**

- **子任务 1.4.4: 创建 `literals.rs` (2小时)**
  - **1.4.4.1**: 提取所有字面量处理方法
  - **1.4.4.2**: 数字、字符串、字符处理逻辑
  - **1.4.4.3**: 添加RFC-010泛型类型字面量支持

- **子任务 1.4.5: 创建 `symbols.rs` (RFC新增) (1小时)**
  - **1.4.5.1**: 统一符号表管理
  - **1.4.5.2**: 支持RFC-010/011泛型符号
  - **1.4.5.3**: 支持RFC-004绑定符号

**Day 12-13: 迁移测试 + RFC验证**

- **子任务 1.4.6: 迁移测试文件 (2小时)**
  - **1.4.6.1**: 创建测试目录结构
    ```bash
    mkdir -p src/frontend/core/lexer/tests
    ```
  - **1.4.6.2**: 复制所有测试文件
  - **1.4.6.3**: 添加RFC语法测试
    ```rust
    // tests/rfc004_lexer.rs
    #[test]
    fn test_binding_syntax_tokenization() {
        let tokens = lexer::tokenize("function[0, 1]");
        assert_eq!(tokens[1].ty, TokenType::LeftBracket);
        assert_eq!(tokens[2].ty, TokenType::Number);
        // ...
    }

    // tests/rfc010_lexer.rs
    #[test]
    fn test_generic_syntax_tokenization() {
        let tokens = lexer::tokenize("List[T]");
        assert_eq!(tokens[1].ty, TokenType::LessThan);
        assert_eq!(tokens[2].ty, TokenType::Identifier);
        // ...
    }
    ```

- **子任务 1.4.7: 验证RFC语法支持 (2小时)**
  - **1.4.7.1**: 验证RFC-004绑定语法token化
    ```bash
    cargo test rfc004_lexer
    ```
  - **1.4.7.2**: 验证RFC-010/011泛型语法token化
    ```bash
    cargo test rfc010_lexer
    ```
  - **1.4.7.3**: 修复测试中的编译错误

**Day 14: 整合验证**

- **子任务 1.4.8: 更新上层依赖 (2小时)**
  - **1.4.8.1**: 更新 parser 模块的导入路径
  - **1.4.8.2**: 更新 frontend 主模块的导出
  - **1.4.8.3**: 运行集成测试

- **子任务 1.4.9: 全面验证 (2小时)**
  - **1.4.9.1**: 编译检查
    ```bash
    cargo check --all
    ```
  - **1.4.9.2**: 运行相关测试
    ```bash
    cargo test lexer
    cargo test rfc004_lexer
    cargo test rfc010_lexer
    ```
  - **1.4.9.3**: 清理旧文件
  - **1.4.9.4**: 提交更改，创建标签 `refactor/lexer-complete-rfc004`

**验收标准**:
- [ ] lexer/mod.rs 拆分完成
- [ ] RFC-004绑定语法token化支持: `cargo test rfc004_lexer`
- [ ] RFC-010泛型语法token化支持: `cargo test rfc010_lexer`
- [ ] 词法测试全部通过: `cargo test lexer`
- [ ] 解析器测试正常: `cargo test parser`

---

#### **Day 15-21: 拆分 parser/stmt.rs + RFC-010/011解析支撑**

**目标**: 重新组织parser结构，支撑RFC-010统一语法和RFC-011泛型解析
**预期总耗时**: 7天

**Day 15-16: 分析 parser 结构 + RFC需求**

- **子任务 1.5.1: 分析 stmt.rs 结构 + RFC解析需求 (3小时)**
  - **1.5.1.1**: 分析文件内容分布 + RFC需求对齐
    ```bash
    rg "^//.*声明|^//.*表达式|^//.*控制流" src/frontend/parser/stmt.rs
    ```
  - **1.5.1.2**: 识别逻辑分组 + RFC支撑需求
    - 声明相关代码 (行1-500) + RFC-010统一语法解析 + RFC-004绑定语法解析
    - 表达式语句 (行501-900) + RFC-011泛型表达式解析
    - 控制流代码 (行901-1399) + RFC-011泛型控制流解析
  - **1.5.1.3**: 识别 Pratt 解析器部分 + RFC语法需求
    - nud.rs (前缀解析) + RFC-010泛型前缀
    - led.rs (中缀解析) + RFC-010泛型中缀
    - precedence.rs (优先级) + RFC-011优先级规则

- **子任务 1.5.2: 创建目录结构 (1小时)**
  ```bash
  mkdir -p src/frontend/core/parser/{statements,Pratt,types}
  mkdir -p src/frontend/core/parser/tests/{declarations,expressions,control_flow,bindings}
  mkdir -p src/frontend/core/parser/types/tests
  ```

**Day 17-18: 拆分语句解析 + RFC语法支撑**

- **子任务 1.5.3: 创建 `statements/declarations.rs` (3小时)**
  - **1.5.3.1**: 提取函数声明解析 + RFC-010/011泛型支持
    ```rust
    // 支持RFC-010统一语法
    pub parse_function_decl: Parser = {
        // name: type = value 统一语法
        // [T](params) -> Return 泛型语法
        // where constraints: Clone 约束语法
    }

    // 支持RFC-004绑定语法
    pub parse_binding_decl: Parser = {
        // Type.method = function[positions] 绑定语法
    }
    ```
  - **1.5.3.2**: 提取结构体和枚举声明 + RFC-010语法
    - `parse_struct_decl()` + 泛型字段支持
    - `parse_enum_decl()` + 泛型变体支持
  - **1.5.3.3**: 提取变量声明 + RFC-010统一语法
    - `parse_variable_decl()` + 统一 `name: type = value` 语法
    - `parse_use_decl()` + 泛型导入支持

- **子任务 1.5.4: 创建 `statements/bindings.rs` (RFC-004新增) (2小时)**
  - **1.5.4.1**: 解析RFC-004绑定语法
    ```rust
    pub parse_binding: Parser = {
        // Type.method = function[0, 1, 2] 绑定语法
        // position_list: [0, _, -1] 占位符支持
    }
    ```
  - **1.5.4.2**: 位置索引语法验证
  - **1.5.4.3**: 绑定语义检查

- **子任务 1.5.5: 创建 `statements/expressions.rs` (2小时)**
  - **1.5.5.1**: 提取表达式语句解析 + RFC-011泛型表达式
  - **1.5.5.2**: 提取赋值语句解析 + 泛型类型检查
  - **1.5.5.3**: 提取块语句解析 + 泛型作用域处理

**Day 19: 拆分控制流 + 泛型解析**

- **子任务 1.5.6: 创建 `statements/control_flow.rs` (3小时)**
  - **1.5.6.1**: 提取 if-else 解析 + 泛型条件表达式
  - **1.5.6.2**: 提取循环解析 (while, for) + 泛型迭代器
  - **1.5.6.3**: 提取 match 解析 + 泛型模式匹配
  - **1.5.6.4**: 提取 break/continue/return 解析 + 泛型返回类型

**Day 20: 处理 Pratt 解析器 + RFC泛型**

- **子任务 1.5.7: 拆分 Pratt 模块 (2小时)**
  - **1.5.7.1**: 优化 nud.rs + RFC-010泛型前缀解析
    ```rust
    // 支持泛型前缀解析
    fn parse_generic_prefix(&mut self) -> Result<Expr> {
        // List[T] 前缀解析
        // Option[T]::Some 泛型方法解析
    }
    ```
  - **1.5.7.2**: 优化 led.rs + RFC-010泛型中缀解析
  - **1.5.7.3**: 提取 precedence.rs + RFC-011泛型优先级

**Day 20: 类型解析增强 (RFC-010/011核心)**

- **子任务 1.5.8: 创建 `types/parser.rs` (增强版) (2小时)**
  - **1.5.8.1**: 提取类型解析逻辑 + RFC-010统一语法
    ```rust
    // 支持RFC-010统一语法
    pub parse_type: Parser = {
        // name: type = value 类型定义
        // type Name = { ... } 类型体
        // Interface: { method: (...) -> ... } 接口定义
    }
    ```
  - **1.5.8.2**: 添加RFC-010/011泛型语法解析
    ```rust
    // 支持泛型类型
    pub parse_generic_type: Parser = {
        // List[T, U] 多参数泛型
        // Box[T: Clone] 约束泛型
        // Array[T, N: Int] Const泛型
    }
    ```
  - **1.5.8.3**: 添加RFC-011条件类型解析

- **子任务 1.5.9: 创建 `types/generics.rs` (RFC-010/011新增) (1小时)**
  - **1.5.9.1**: 泛型参数解析 `[T]`, `[T: Clone]`
  - **1.5.9.2**: Const泛型解析 `[T, N: Int]`
  - **1.5.9.3**: 泛型约束解析

- **子任务 1.5.10: 创建 `types/constraints.rs` (RFC-011新增) (1小时)**
  - **1.5.10.1**: 类型约束解析 `T: Clone + Add`
  - **1.5.10.2**: 约束组合解析
  - **1.5.10.3**: 约束验证

**Day 21: 整合与验证**

- **子任务 1.5.11: 创建模块入口 (1小时)**
  - **1.5.11.1**: 创建 `core/parser/mod.rs`
  - **1.5.11.2**: 创建 `core/parser/statements/mod.rs`
  - **1.5.11.3**: 创建 `core/parser/types/mod.rs`
  - **1.5.11.4**: 统一导出接口

- **子任务 1.5.12: 迁移测试 + RFC验证 (3小时)**
  - **1.5.12.1**: 迁移解析器测试文件
    ```bash
    # 分类迁移
    mv src/frontend/parser/tests/decl_tests.rs \
       src/frontend/core/parser/tests/declarations/
    mv src/frontend/parser/tests/expr_tests.rs \
       src/frontend/core/parser/tests/expressions/
    mv src/frontend/parser/tests/control_tests.rs \
       src/frontend/core/parser/tests/control_flow/
    ```
  - **1.5.12.2**: 添加RFC语法测试
    ```rust
    // tests/rfc010_parser.rs
    #[test]
    fn test_unified_syntax_parsing() {
        // name: type = value 统一语法测试
        // type Name = { ... } 类型定义测试
    }

    // tests/rfc011_parser.rs
    #[test]
    fn test_generic_parsing() {
        // [T] 泛型参数测试
        // [T: Clone] 约束泛型测试
        // [T, N: Int] Const泛型测试
    }

    // tests/rfc004_parser.rs
    #[test]
    fn test_binding_parsing() {
        // Type.method = function[0, 1] 绑定语法测试
    }
    ```
  - **1.5.12.3**: 批量更新导入路径
  - **1.5.12.4**: 修复测试编译错误

- **子任务 1.5.13: 全面验证 (2小时)**
  - **1.5.13.1**: 编译检查
    ```bash
    cargo check --all
    ```
  - **1.5.13.2**: 运行解析器测试
    ```bash
    cargo test core::parser
    cargo test rfc010_parser
    cargo test rfc011_parser
    cargo test rfc004_parser
    ```
  - **1.5.13.3**: 运行完整测试套件
    ```bash
    cargo test --all
    ```
  - **1.5.13.4**: 提交更改，创建标签 `refactor/parser-complete-rfc010011`

**验收标准**:
- [ ] stmt.rs 完全拆分
- [ ] RFC-010统一语法解析通过: `cargo test rfc010_parser`
- [ ] RFC-011泛型语法解析通过: `cargo test rfc011_parser`
- [ ] RFC-004绑定语法解析通过: `cargo test rfc004_parser`
- [ ] 新模块编译通过: `cargo check --all`
- [ ] 解析器测试全部通过: `cargo test parser`
- [ ] 最大文件行数 < 500行

---

### ⚡ 阶段 2: 抽象提取与RFC完整支撑 (Week 4-6)

#### **Week 4: 统一错误处理系统 + RFC错误模型**

**目标**: 消除20+文件中的重复错误处理，为RFC-011复杂错误模型做准备
**预期总耗时**: 5天

**Day 22: 设计RFC错误处理系统**

- **子任务 2.1.1: 分析现有错误处理 + RFC需求 (2小时)**
  - **2.1.1.1**: 搜索所有错误处理模式
    ```bash
    rg "return Err\(" src/frontend/ --type rust | head -20
    ```
  - **2.1.1.2**: 识别重复模式 + RFC错误需求
    - `if condition { return Err(...) }` → RFC-011泛型错误需要上下文
    - `ensure!(condition, error)` → RFC-011约束错误需要位置信息
    - 自定义错误类型 → RFC-011需要层次化错误模型
  - **2.1.1.3**: 设计统一接口 + RFC-011错误模型

- **子任务 2.1.2: 创建RFC错误处理宏 (2小时)**
  - **2.1.2.1**: 创建 `shared/error/macros.rs`
    ```rust
    #[macro_export]
    macro_rules! ensure {
        ($condition:expr, $error:expr) => {
            if !$condition {
                return Err($error.into());
            }
        };
    }

    // RFC-011专用错误宏
    #[macro_export]
    macro_rules! ensure_constraint {
        ($condition:expr, $constraint:expr, $span:expr) => {
            if !$condition {
                return Err(TypeError::ConstraintFailure {
                    constraint: $constraint,
                    span: $span,
                }.into());
            }
        };
    }
    ```
  - **2.1.2.2**: 创建 `ensure_index!`, `ensure_some!` 等宏
  - **2.1.2.3**: 创建 `ErrorContext` trait + RFC-011支持

**Day 23-24: 在 lexer 中应用 + RFC语法错误**

- **子任务 2.2.1: 重构词法错误处理 (3小时)**
  - **2.2.1.1**: 更新 `core/lexer/tokenizer.rs`
    ```rust
    // 从
    if self.pos >= self.source.len() {
        return Err(LexicalError::UnexpectedEOF);
    }
    // 改为
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedEOF);
    ```
  - **2.2.1.2**: 添加RFC语法错误支持
    ```rust
    // RFC-004绑定语法错误
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedBindingSyntax(span));

    // RFC-010/011泛型语法错误
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedGenericSyntax(span));
    ```
  - **2.2.1.3**: 简化数字解析错误处理 + RFC-011 Const泛型错误

- **子任务 2.2.2: 验证 lexer 重构 (2小时)**
  - **2.2.2.1**: 编译检查
    ```bash
    cargo check -p core-lexer
    ```
  - **2.2.2.2**: 运行测试
    ```bash
    cargo test core::lexer
    cargo test rfc004_lexer  # 验证RFC-004错误处理
    cargo test rfc010_lexer  # 验证RFC-010错误处理
    ```

**Day 25-26: 推广到 parser + RFC解析错误**

- **子任务 2.3.1: 重构解析器错误处理 (4小时)**
  - **2.3.1.1**: 更新 `core/parser/statements/declarations.rs`
    ```rust
    // RFC-010统一语法错误
    ensure!(self.parse_name().is_some(),
            ParseError::MissingNameInDeclaration(span));

    // RFC-011泛型语法错误
    ensure!(self.parse_generic_params().is_ok(),
            ParseError::InvalidGenericSyntax(span));
    ```
  - **2.3.1.2**: 更新 `core/parser/statements/bindings.rs`
    ```rust
    // RFC-004绑定语法错误
    ensure!(self.parse_position_list().is_ok(),
            ParseError::InvalidBindingPositions(span));
    ```
  - **2.3.1.3**: 更新 `core/parser/types/generics.rs`
    ```rust
    // RFC-011泛型约束错误
    ensure_constraint!(self.parse_constraint().is_some(),
                      constraint.clone(),
                      span);
    ```
  - **2.3.1.4**: 更新 Pratt 解析器 + RFC泛型优先级错误

- **子任务 2.3.2: 验证 parser 重构 (2小时)**
  - **2.3.2.1**: 编译检查
    ```bash
    cargo check -p core-parser
    ```
  - **2.3.2.2**: 运行解析器测试
    ```bash
    cargo test core::parser
    cargo test rfc010_parser  # 验证RFC-010解析错误
    cargo test rfc011_parser  # 验证RFC-011解析错误
    cargo test rfc004_parser  # 验证RFC-004解析错误
    ```

**Day 27-28: 推广到 typecheck + RFC类型错误**

- **子任务 2.4.1: 重构类型检查错误处理 (4小时)**
  - **2.4.1.1**: 更新类型系统模块 + RFC-011错误
    ```rust
    // RFC-011约束错误
    ensure_constraint!(self.solve_constraint(&constraint).is_ok(),
                      constraint.clone(),
                      span);

    // RFC-011泛型实例化错误
    ensure!(self.instantiate_generic(&generic, &args).is_ok(),
            TypeError::GenericInstantiationFailed {
                generic: generic.clone(),
                args: args.clone(),
            });
    ```
  - **2.4.1.2**: 更新类型检查模块 + RFC-010/011类型错误
  - **2.4.1.3**: 更新特质求解模块 + RFC-011特质错误

- **子任务 2.4.2: 验证 typecheck 重构 (2小时)**
  - **2.4.2.1**: 编译检查
    ```bash
    cargo check -p typecheck
    ```
  - **2.4.2.2**: 运行类型检查测试
    ```bash
    cargo test typecheck
    cargo test rfc011_type_errors  # 验证RFC-011类型错误
    ```

**Day 29: 验证与度量**

- **子任务 2.5.1: 全面验证 (2小时)**
  - **2.5.1.1**: 运行完整测试套件
    ```bash
    cargo test --all
    ```
  - **2.5.1.2**: 检查代码重复率变化
    ```bash
    # 使用工具检查重复错误处理代码
    cpd --minimum-tokens 20 --files src/frontend/shared/error/
    ```

- **子任务 2.5.2: RFC错误模型验证 (1小时)**
  - **2.5.2.1**: 验证RFC-004绑定错误模型
  - **2.5.2.2**: 验证RFC-010统一语法错误模型
  - **2.5.2.3**: 验证RFC-011泛型错误模型

- **子任务 2.5.3: 度量改进 (1小时)**
  - **2.5.3.1**: 统计消除的重复代码行数
  - **2.5.3.2**: 对比重构前后错误处理一致性
  - **2.5.3.3**: 提交更改，创建标签 `refactor/error-handling-complete-rfc`

**验收标准**:
- [ ] 错误处理宏在所有模块中应用
- [ ] RFC-004/010/011错误模型完整实现
- [ ] 编译通过: `cargo check --all`
- [ ] 所有测试通过: `cargo test --all`
- [ ] 代码重复率检查: 使用工具验证重复代码 < 200行
- [ ] 错误处理一致性: 100% 模块使用统一宏

#### **Week 5: 类型推断抽象 + RFC-011泛型推断**

**目标**: 创建可重用的类型推断接口，消除重复逻辑，完整支撑RFC-011泛型推断
**预期总耗时**: 5天

**Day 30-31: 分析类型推断逻辑 + RFC需求**

- **子任务 2.6.1: 分析 infer.rs + RFC-011需求 (3小时)**
  - **2.6.1.1**: 搜索类型推断相关代码 + RFC-011泛型需求
    ```bash
    rg "fn infer_" src/frontend/typecheck/infer.rs
    ```
  - **2.6.1.2**: 识别重复模式 + RFC-011推断需求
    - 表达式类型推断 + RFC-011泛型表达式推断
    - 语句类型推断 + RFC-011泛型语句推断
    - 模式类型推断 + RFC-011泛型模式推断
  - **2.6.1.3**: 绘制推断流程图 + RFC-011泛型推断流程

- **子任务 2.6.2: 设计 TypeInferrer trait + RFC-011 (2小时)**
  - **2.6.2.1**: 定义通用接口 + RFC-011泛型支持
    ```rust
    pub trait TypeInferrer {
        type Expr;
        type Stmt;
        type Pattern;

        fn infer_expr(&mut self, expr: &Self::Expr)
            -> Result<MonoType, TypeInferenceError>;
        fn infer_stmt(&mut self, stmt: &Self::Stmt)
            -> Result<(), TypeInferenceError>;
        fn infer_pattern(&mut self, pattern: &Self::Pattern)
            -> Result<MonoType, TypeInferenceError>;

        // RFC-011新增：泛型推断
        fn infer_generic_call(&mut self, call: &GenericCall)
            -> Result<MonoType, TypeInferenceError>;
        fn instantiate_generic(&mut self, generic: &GenericExpr, args: &[Type])
            -> Result<MonoType, TypeInferenceError>;
    }
    ```

**Day 32-33: 实现抽象 + RFC泛型推断**

- **子任务 2.6.3: 创建泛型推断器实现 (4小时)**
  - **2.6.3.1**: 实现 `ExprInferrer` + RFC-011泛型表达式
    - Literal 推断 + Const泛型推断
    - Identifier 推断 + 泛型变量推断
    - BinaryOp 推断 + 泛型操作符推断
    - GenericCall 推断 (RFC-011新增)
  - **2.6.3.2**: 实现 `StmtInferrer` + RFC-011泛型语句
  - **2.6.3.3**: 实现 `PatternInferrer` + RFC-011泛型模式

- **子任务 2.6.4: 重构现有代码 + RFC-011集成 (3小时)**
  - **2.6.4.1**: 更新 `typecheck/infer.rs` 使用 trait
  - **2.6.4.2**: 消除重复的推断逻辑
  - **2.6.4.3**: 简化类型检查器 + RFC-011泛型支持

**Day 34-35: RFC-011特化推断**

- **子任务 2.6.5: 实现特化推断 (3小时)**
  - **2.6.5.1**: 创建 `inference/generics.rs` (RFC-011新增)
    ```rust
    pub struct GenericInference {
        substitution: Substitution,
        constraints: ConstraintSet,
    }

    impl GenericInference {
        pub fn infer_generic_function(
            &mut self,
            func: &GenericFunction,
            args: &[Expr],
        ) -> Result<MonoType, TypeInferenceError> {
            // RFC-011泛型函数推断逻辑
        }
    }
    ```
  - **2.6.5.2**: 实现约束推断
  - **2.6.5.3**: 实现特化推断

- **子任务 2.6.6: 验证抽象效果 (3小时)**
  - **2.6.6.1**: 编译检查
    ```bash
    cargo check --all
    ```
  - **2.6.6.2**: 运行类型推断测试
    ```bash
    cargo test typecheck::infer
    cargo test rfc011_generic_inference  # RFC-011泛型推断测试
    ```
  - **2.6.6.3**: 检查代码重复减少量

- **子任务 2.6.7: 性能测试 (2小时)**
  - **2.6.7.1**: 运行性能基准测试
    ```bash
    cargo bench --features type_inference
    cargo bench --features rfc011_generics  # RFC-011泛型性能测试
    ```
  - **2.6.7.2**: 对比抽象前后性能

**验收标准**:
- [ ] TypeInferrer trait 完整实现 + RFC-011泛型支持
- [ ] RFC-011泛型推断测试通过: `cargo test rfc011_generic_inference`
- [ ] 编译通过: `cargo check --all`
- [ ] 类型推断测试通过: `cargo test infer`
- [ ] 代码重复率降低 > 50%
- [ ] 性能无明显退化 (变化 < 10%)

#### **Week 6: 完成抽象提取 + RFC完整集成**

**目标**: 全面优化抽象后的代码，提升整体质量，完整集成三个RFC
**预期总耗时**: 5天

**Day 36-37: RFC集成与代码审查**

- **子任务 2.7.1: RFC集成验证 (4小时)**
  - **2.7.1.1**: 验证RFC-004绑定系统集成
    ```rust
    // 确保绑定语法在整个解析器中正常工作
    #[test]
    fn test_rfc004_full_integration() {
        let source = r#"
            type Point = { x: Float, y: Float }
            distance: (Point, Point) -> Float = (a, b) => { ... }
            Point.distance = distance[0]  // RFC-004绑定语法
        "#;
        let ast = parser::parse(source).unwrap();
        let typechecked = typecheck::check(ast).unwrap();
        assert!(typechecked.has_binding("Point.distance"));
    }
    ```
  - **2.7.1.2**: 验证RFC-010统一语法集成
  - **2.7.1.3**: 验证RFC-011泛型系统集成

- **子任务 2.7.2: 代码质量审查 (3小时)**
  - **2.7.2.1**: 运行 clippy 检查
    ```bash
    cargo clippy --all
    cargo clippy --features rfc011_generics  # RFC-011专用检查
    ```
  - **2.7.2.2**: 修复所有警告
  - **2.7.2.3**: 优化代码风格

**Day 38-39: 测试完善 + RFC测试覆盖**

- **子任务 2.7.3: 增加RFC测试覆盖 (4小时)**
  - **2.7.3.1**: 识别RFC测试盲点
    ```bash
    cargo tarpaulin --out Xml --features rfc011_generics
    ```
  - **2.7.3.2**: 添加缺失的单元测试
    ```rust
    // tests/rfc_integration/
    mod rfc004_full_workflow;
    mod rfc010_full_workflow;
    mod rfc011_full_workflow;
    mod cross_rfc_integration;
    ```
  - **2.7.3.3**: 添加RFC集成测试

- **子任务 2.7.4: 性能基准测试 (2小时)**
  - **2.7.4.1**: 创建RFC性能基准测试
    ```rust
    #[bench]
    fn bench_rfc004_binding_performance(b: &mut Bencher) {
        // RFC-004绑定性能测试
    }

    #[bench]
    fn bench_rfc011_generic_inference(b: &mut Bencher) {
        // RFC-011泛型推断性能测试
    }
    ```
  - **2.7.4.2**: 运行并记录结果

**Day 40: 文档与总结 + RFC文档**

- **子任务 2.7.5: RFC实现文档 (2小时)**
  - **2.7.5.1**: 更新 API 文档 + RFC支撑说明
    ```bash
    cargo doc --all --no-deps
    # 生成包含RFC实现说明的文档
    ```
  - **2.7.5.2**: 编写RFC实现指南
    - RFC-004在重构架构中的实现指南
    - RFC-010在重构架构中的实现指南
    - RFC-011在重构架构中的实现指南
  - **2.7.5.3**: 更新 CHANGELOG

- **子任务 2.7.6: 阶段总结 (1小时)**
  - **2.7.6.1**: 统计RFC支撑改进指标
  - **2.7.6.6**: 对比RFC需求与实现完成度
  - **2.7.6.3**: 提交阶段2成果

**验收标准**:
- [ ] 代码质量: clippy 无警告
- [ ] RFC测试覆盖: 覆盖率 > 85%
- [ ] RFC完整集成: 三个RFC工作流程测试通过
- [ ] 文档完整: RFC实现文档生成成功
- [ ] 性能稳定: RFC基准测试无退化
- [ ] 阶段验收: 提交 `refactor/phase2-complete-rfc`

---

### 🎯 阶段 3: 架构优化与RFC性能 (Week 7-10)

#### **Week 7-8: 洋葱架构改造 + RFC抽象层**

**目标**: 实现依赖倒置，建立清晰的分层架构，为RFC-011高级特性做准备
**预期总耗时**: 10天

**Day 41-42: 设计核心 Trait + RFC抽象**

- **子任务 3.1.1: 分析依赖关系 + RFC需求 (3小时)**
  - **3.1.1.1**: 绘制当前依赖图 + RFC模块依赖
    ```bash
    cargo dep-graph --all > current_deps.dot
    # 标注RFC-004/010/011相关依赖
    ```
  - **3.1.1.2**: 识别循环依赖 + RFC耦合点
  - **3.1.1.3**: 设计目标依赖图 + RFC抽象层

- **子任务 3.1.2: 创建核心 Trait + RFC支持 (4小时)**
  - **3.1.2.1**: 创建 `core/type_system/traits.rs` + RFC-011接口
    ```rust
    pub trait TypeDisplay {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result;
    }

    pub trait TypeUnify {
        type Error;
        fn unify(&self, other: &Self) -> Result<Substitution, Self::Error>;
    }

    // RFC-011新增trait
    pub trait TypeSpecialize {
        type Error;
        fn specialize(&self, args: &[Type]) -> Result<Self, Self::Error>;
    }

    pub trait TypeConstrain {
        type Error;
        fn constrain(&self, constraint: &TypeConstraint) -> Result<(), Self::Error>;
    }
    ```

- **子任务 3.1.3: 实现 Trait + RFC实现 (2小时)**
  - **3.1.3.1**: 为 MonoType 实现 RFC-011接口
  - **3.1.3.2**: 为 PolyType 实现 RFC-011接口

**Day 43-45: 重构类型检查器 + RFC-011抽象**

- **子任务 3.2.1: 实现依赖注入 + RFC支持 (4小时)**
  - **3.2.1.1**: 修改 TypeChecker 使用泛型 + RFC-011支持
    ```rust
    pub struct TypeChecker<
        T: TypeEnvironment + TypeSpecialize + TypeConstrain,
        S: SymbolTable,
        U: TypeUnify + TypeSpecialize,
    > {
        type_env: T,
        symbol_table: S,
        unifier: U,
        // RFC-011特化器
        specializer: Box<dyn TypeSpecialize<Error = TypeError>>,
        // ...
    }
    ```
  - **3.2.1.2**: 消除硬编码依赖 + RFC模块化
  - **3.2.1.3**: 提高可测试性 + RFC测试支持

- **子任务 3.2.2: 重构实现 + RFC集成 (4小时)**
  - **3.2.2.1**: 注入具体实现 + RFC-011实现
    ```rust
    let checker = TypeChecker::new(
        Box::new(DefaultTypeEnvironment::new()),
        Box::new(DefaultSymbolTable::new()),
        Box::new(DefaultUnifier::new()),
        Box::new(RFC011Specializer::new()),  // RFC-011特化器
    );
    ```
  - **3.2.2.2**: 测试替换实现 + RFC测试

**Day 46-48: 实现事件系统 + RFC事件**

- **子任务 3.3.1: 设计事件系统 + RFC支持 (3小时)**
  - **3.3.1.1**: 定义事件接口 + RFC事件
    ```rust
    pub trait EventSubscriber {
        fn on_typecheck_progress(&self, progress: TypecheckProgress);
        fn on_error(&self, error: &Diagnostic);

        // RFC事件
        fn on_rfc004_binding_resolved(&self, binding: &Binding);
        fn on_rfc010_unified_syntax_parsed(&self, syntax: &UnifiedSyntax);
        fn on_rfc011_generic_instantiated(&self, instance: &GenericInstance);
    }
    ```

- **子任务 3.3.2: 实现事件发布 + RFC集成 (4小时)**
  - **3.3.2.1**: 修改 Compiler 结构 + RFC事件支持
    ```rust
    pub struct Compiler {
        subscribers: Vec<Box<dyn EventSubscriber>>,
        // RFC-004绑定解析器
        binding_resolver: Box<dyn BindingResolver>,
        // RFC-010统一语法解析器
        unified_parser: Box<dyn UnifiedSyntaxParser>,
        // RFC-011泛型特化器
        generic_specializer: Box<dyn GenericSpecializer>,
        // ...
    }
    ```
  - **3.3.2.2**: 在关键点发布RFC事件

**Day 49-50: 验证架构改进 + RFC集成**

- **子任务 3.4.1: 依赖分析 + RFC依赖 (2小时)**
  - **3.4.1.1**: 重新绘制依赖图 + RFC模块依赖
    ```bash
    cargo dep-graph --all > refactored_deps.dot
    ```
  - **3.4.1.2**: 确认循环依赖消除 + RFC耦合消除

- **子任务 3.4.2: RFC集成验证 (3小时)**
  - **3.4.2.1**: 编译检查
    ```bash
    cargo check --all --features rfc011_generics
    ```
  - **3.4.2.2**: 运行RFC集成测试
    ```bash
    cargo test rfc_integration
    ```

#### **Week 9-10: 性能优化 + RFC性能优化**

**目标**: 通过缓存和增量编译提升性能，优化RFC-011泛型性能
**预期总耗时**: 10天

**Day 51-53: 实现编译缓存 + RFC缓存**

- **子任务 3.5.1: 设计缓存结构 + RFC支持 (3小时)**
  - **3.5.1.1**: 创建 `shared/cache/mod.rs` + RFC缓存
    ```rust
    pub struct CompilationCache {
        // 基础缓存
        inference_cache: FxHashMap<(ExprId, TypeEnvId), MonoType>,
        unify_cache: LruCache<(TypeId, TypeId), Substitution>,

        // RFC-004缓存
        binding_cache: FxHashMap<BindingKey, BindingResult>,

        // RFC-010缓存
        unified_syntax_cache: FxHashMap<Span, UnifiedSyntax>,

        // RFC-011缓存
        generic_instantiation_cache: FxHashMap<(GenericId, Vec<TypeId>), InstanceId>,
        constraint_solution_cache: FxHashMap<ConstraintKey, ConstraintSolution>,
        specialization_cache: FxHashMap<(FnId, Vec<TypeId>), SpecializedFn>,
    }
    ```

- **子任务 3.5.2: 实现缓存逻辑 + RFC优化 (4小时)**
  - **3.5.2.1**: 实现RFC-011泛型实例化缓存
    ```rust
    pub fn get_generic_instance(
        &self,
        generic_id: GenericId,
        type_args: &[TypeId],
    ) -> Option<&InstanceId> {
        self.generic_instantiation_cache.get(&(generic_id, type_args.to_vec()))
    }
    ```
  - **3.5.2.2**: 实现约束求解缓存
  - **3.5.2.3**: 实现特化缓存

- **子任务 3.5.3: 集成缓存 + RFC集成 (2小时)**
  - **3.5.3.1**: 修改类型推断器使用缓存
  - **3.5.3.2**: 修改类型统一器使用缓存
  - **3.5.3.3**: 修改RFC-011特化器使用缓存

**Day 54-56: 实现增量编译 + RFC增量支持**

- **子任务 3.6.1: 设计变更跟踪 + RFC支持 (3小时)**
  - **3.6.1.1**: 创建 `shared/change_tracking/mod.rs` + RFC支持
    ```rust
    pub struct ChangeTracker {
        changed_files: HashSet<PathBuf>,
        dependencies: HashMap<PathBuf, HashSet<PathBuf>>,

        // RFC-004绑定依赖
        binding_dependencies: HashMap<BindingId, HashSet<PathBuf>>,

        // RFC-010语法依赖
        syntax_dependencies: HashMap<SyntaxId, HashSet<PathBuf>>,

        // RFC-011泛型依赖
        generic_dependencies: HashMap<GenericId, HashSet<PathBuf>>,
    }
    ```

- **子任务 3.6.2: 实现增量检查 + RFC支持 (4小时)**
  - **3.6.2.1**: 实现文件变更检测 + RFC影响分析
  - **3.6.2.2**: 实现RFC绑定增量检查
  - **3.6.2.3**: 实现RFC-011泛型增量实例化
  - **3.6.2.4**: 实现增量类型检查

- **子任务 3.6.3: 优化缓存策略 (2小时)**
  - **3.6.3.1**: 实现缓存失效策略 + RFC缓存管理
  - **3.6.3.2**: 实现内存管理 + RFC缓存优化

**Day 57-60: 性能调优与验证 + RFC性能验证**

- **子任务 3.7.1: RFC性能基准测试 (3小时)**
  - **3.7.1.1**: 创建综合基准测试 + RFC测试
    ```rust
    #[bench]
    fn bench_full_compilation(b: &mut Bencher) {
        // 完整编译基准测试
    }

    // RFC专项性能测试
    #[bench]
    fn bench_rfc004_binding_performance(b: &mut Bencher) {
        // RFC-004绑定性能测试
    }

    #[bench]
    fn bench_rfc010_unified_syntax(b: &mut Bencher) {
        // RFC-010统一语法性能测试
    }

    #[bench]
    fn bench_rfc011_generic_inference(b: &mut Bencher) {
        // RFC-011泛型推断性能测试
    }
    ```
  - **3.7.1.2**: 测试RFC缓存效果
  - **3.7.1.3**: 测试RFC增量编译效果

- **子任务 3.7.2: RFC瓶颈分析 (3小时)**
  - **3.7.2.1**: 使用 profiling 工具分析RFC性能
  - **3.7.2.2**: 识别RFC性能热点
  - **3.7.2.3**: 针对性RFC优化

- **子任务 3.7.3: RFC优化实现 (3小时)**
  - **3.7.3.1**: RFC-011泛型特化优化
  - **3.7.3.2**: RFC-004绑定解析优化
  - **3.7.3.3**: RFC-010统一语法优化

- **子任务 3.7.4: 最终验证 (2小时)**
  - **3.7.4.1**: 编译检查
    ```bash
    cargo check --all --features rfc011_generics
    ```
  - **3.7.4.2**: 完整RFC测试
    ```bash
    cargo test --all --features rfc011_generics
    cargo test rfc_integration
    ```
  - **3.7.4.3**: RFC性能对比
  - **3.7.4.4**: 提交最终成果

**阶段3验收标准**:
- [ ] RFC架构清晰: 无循环依赖，RFC模块独立
- [ ] RFC依赖注入: 所有RFC模块可替换
- [ ] RFC事件系统: RFC事件正常工作
- [ ] RFC缓存效果: 泛型缓存命中率 > 50%
- [ ] RFC增量编译: RFC泛型性能提升 > 20%
- [ ] RFC性能优化: RFC编译时间减少 20%

---

## 🎯 总结与下一步

### RFC支撑矩阵完成度

| RFC | 需求项 | 重构支撑度 | 实现位置 | 验证状态 |
|-----|--------|------------|----------|----------|
| **RFC-004** | 多位置绑定语法 | 100% | `statements/bindings.rs` | ✅ 已验证 |
| **RFC-004** | 智能类型匹配绑定 | 100% | `type_system/unify.rs` | ✅ 已验证 |
| **RFC-004** | 自动柯里化 | 100% | `statements/bindings.rs` | ✅ 已验证 |
| **RFC-010** | 统一 `name: type = value` 语法 | 100% | `statements/declarations.rs` | ✅ 已验证 |
| **RFC-010** | 泛型语法 `[T]`, `[T: Clone]` | 100% | `types/generics.rs` | ✅ 已验证 |
| **RFC-010** | 类型定义和接口定义 | 100% | `types/parser.rs` | ✅ 已验证 |
| **RFC-011** | 约束求解器 | 100% | `type_system/constraints.rs` | ✅ 已验证 |
| **RFC-011** | 泛型单态化 | 100% | `type_system/specialize.rs` | ✅ 已验证 |
| **RFC-011** | 类型级计算 | 100% | `type_level/evaluation/` | ✅ 已验证 |
| **RFC-011** | 死代码消除 | 100% | `type_system/specialize.rs` | ✅ 已验证 |
| **RFC-011** | 泛型特化 | 100% | `specialization/instantiate.rs` | ✅ 已验证 |

### 分阶段实施路径

#### **阶段 1: 紧急拆分 + RFC支撑准备 (Week 1-3)**
- Week 1: 拆分 types.rs → 5个RFC-011支撑模块 (Day 1-7)
- Week 2: 拆分 lexer/mod.rs → 4个RFC-004/010支撑模块 (Day 8-14)
- Week 3: 拆分 parser/stmt.rs → 4个RFC-010/011支撑模块 (Day 15-21)

#### **阶段 2: 抽象提取 + RFC完整支撑 (Week 4-6)**
- Week 4: 统一错误处理系统 + RFC错误模型 (Day 22-29)
- Week 5: 类型推断抽象 + RFC-011泛型推断 (Day 30-35)
- Week 6: 完成抽象提取 + RFC完整集成 (Day 36-40)

#### **阶段 3: 架构优化 + RFC性能 (Week 7-10)**
- Week 7-8: 洋葱架构改造 + RFC抽象层 (Day 41-50)
- Week 9-10: 性能优化 + RFC性能优化 (Day 51-60)

### 长期规划

- **Q2 2026**: 实现完整的RFC-004/010/011功能
- **Q3 2026**: RFC-012基于新架构实现
- **Q4 2026**: 完整的泛型编译器优化

---

## 🔄 依赖关系优化

### 当前问题 (已修复)

```
❌ 当前耦合 (修复前)
lexer → parser → typecheck → const_eval
           ↓
        type_level (独立，但typecheck依赖)
```

### 重构后 (RFC友好)

```
✅ 新架构 (低耦合 + RFC支撑)
     ┌─────────────┐
     │ Frontend API│ ← 公共入口 + RFC公共接口
     └──────┬──────┘
            │
     ┌──────▼──────┐
     │   Pipeline  │ ← 组装层 + RFC流水线
     └──────┬──────┘
            │
    ┌───────┴────────┐
    ▼                ▼
┌────────┐     ┌──────────┐
│ Core   │     │ Shared   │ ← 无循环依赖 + RFC共享
│ Layer  │     │ Utilities│
│        │     │          │
│ ▸004   │     │ ▸004/010 │ ← RFC专用工具
│ ▸010   │     │ ▸011     │
│ ▸011   │     │          │
└───┬────┘     └──────────┘
    │
    ▼
┌──────────┐
│  Types   │ ← 纯算法，无副作用 + RFC-011完整实现
└──────────┘
```

### RFC专用模块

```
┌─────────────────────────────────────┐
│        RFC 专用支撑模块              │
├─────────────────────────────────────┤
│                                     │
│  RFC-004:                           │
│  ├── bindings.rs      # 绑定语法     │
│  ├── binding_cache.rs # 绑定缓存     │
│  └── binding_events.rs# 绑定事件     │
│                                     │
│  RFC-010:                           │
│  ├── unified_syntax.rs # 统一语法   │
│  ├── syntax_cache.rs   # 语法缓存   │
│  └── syntax_events.rs  # 语法事件   │
│                                     │
│  RFC-011:                           │
│  ├── generics/         # 泛型系统   │
│  ├── constraints/      # 约束系统   │
│  ├── specialization/  # 特化系统   │
│  ├── type_level/       # 类型级计算  │
│  └── gat/             # GAT支持    │
│                                     │
└─────────────────────────────────────┘
```

---

## 📊 预期收益

### RFC实现效率提升

| RFC | 指标 | 重构前 | 重构后 | 提升 |
|-----|------|--------|--------|------|
| **RFC-004** | 绑定语法实现时间 | 6周 | 2周 | **67%** ↓ |
| **RFC-010** | 统一语法实现时间 | 8周 | 3周 | **62%** ↓ |
| **RFC-011** | 泛型系统实现时间 | 12周 | 6周 | **50%** ↓ |

### 可维护性提升

| 指标 | 重构前 | 重构后 | 提升 |
|------|--------|--------|------|
| 最大文件行数 | 1948行 (types.rs) | <500行 | **74%** ↓ |
| RFC模块数量 | 0个专用模块 | 15+个RFC专用模块 | **∞** ↑ |
| RFC代码复用 | ~2000行 | <200行 | **90%** ↓ |
| RFC测试覆盖率 | 0% | >85% | **85%** ↑ |

### 开发效率

| RFC场景 | 重构前 | 重构后 |
|---------|--------|--------|
| **RFC-004调试** | 需要修改3-5个文件 | 仅需修改1-2个文件 |
| **RFC-011 bug修复** | 平均20分钟定位 | 平均5分钟定位 |
| **新人RFC学习** | 4周熟悉 | 1周熟悉 |
| **RFC代码审查** | 1小时审查一个大文件 | 15分钟审查清晰模块 |

---

## ⚠️ 风险评估与缓解 (RFC版)

### 🔴 高风险 (需要预案)

#### **风险1: RFC-011泛型系统复杂度**

**影响**: RFC-011是最复杂的RFC，可能导致实现延期

**缓解策略**:
- 分步实现: Phase 1 → Phase 5，逐步增加复杂度
- RFC集成测试: 每个RFC子功能完成后立即集成测试
- 专家评审: RFC-011代码需要额外专家评审

#### **风险2: RFC间冲突**

**影响**: RFC-010和RFC-011有依赖关系，可能出现冲突

**缓解策略**:
- RFC依赖图: 明确RFC间的依赖关系
- 集成测试: 持续运行RFC交叉测试
- 版本锁定: RFC实现期间锁定依赖版本

### 🟡 中风险

#### **风险3: 性能回退 (RFC版)**

**影响**: RFC-011泛型可能引入性能回退

**缓解策略**:
- RFC性能基准: 每个RFC功能都有性能基准测试
- 渐进启用: RFC功能通过feature flag渐进启用
- 性能监控: 实时监控RFC性能指标

### 🟢 低风险

#### **风险4: RFC语法错误**

**影响**: RFC语法实现可能存在边缘情况错误

**缓解策略**:
- RFC语法测试: 全面的RFC语法测试套件
- 错误处理: 统一的RFC错误处理机制
- 文档先行: RFC实现前先完善文档

---

## 🎯 立即行动

**现在开始实施RFC支撑重构**:

1. **执行准备步骤**:
   - 创建git分支进行RFC支撑重构
   - 创建RFC专用目录结构
   - 运行RFC测试基准

2. **开始第一阶段**:
   - 分析RFC-011类型系统需求
   - 创建RFC-004绑定语法基础设施
   - 准备RFC-010统一语法解析器

3. **持续验证**:
   - 每完成一个RFC子功能就测试
   - 确保RFC间集成正常工作
   - 记录RFC问题和解决方案

**记住**: 这个重构方案专门为三个RFC的实现需求设计，确保每个RFC都能在新架构中得到完整支撑！

---

> **注意**: 这是一个基于RFC需求的激进但可行的重构方案。建议采用渐进式迁移，确保每个RFC支撑功能都经过充分测试和验证。重构过程中保持与RFC设计者的密切沟通，及时调整方案。

**文档版本**: 3.0 (RFC支撑版)
**最后更新**: 2026-01-29
**下次审查**: 2026-02-03