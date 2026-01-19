# 代码过度耦合问题分析与修复计划

> 生成日期: 2026-01-19

---

## 📊 分析摘要

通过代码审查，共识别出 **10 个过度耦合问题**，按严重程度分类：

| 严重程度 | 数量 | 描述 |
|---------|------|------|
| 🔴 严重 | 4 | 破坏架构边界的耦合，需要优先修复 |
| 🟡 中等 | 4 | 违反单一职责，需要重构 |
| 🟢 轻微 | 2 | 代码规范问题，可选修复 |

---

## 🔴 严重问题 (P0)

### 1. 类型检查 ↔ IR生成 紧耦合 已修复

**位置**: [src/frontend/typecheck/check.rs:151-162](src/frontend/typecheck/check.rs#L151-L162)

```rust
// 问题代码：类型检查阶段直接调用 IR 生成
let mut generator = AstToIrGenerator::new();
generator.generate_module_ir(module).map_err(...)
```

**问题描述**:
- 类型检查阶段应该只做类型验证
- IR 生成是独立的编译阶段
- 两者耦合导致：类型检查错误时仍可能触发 IR 生成

**影响范围**:
- 无法独立执行类型检查
- 错误处理复杂化
- 难以实现增量编译

**建议解耦方案**:
```
类型检查器 ──返回类型结果──> 独立调用 ──生成IR──> IR生成器
```

**验收标准**:
- [ ] `check_module` 只返回类型检查结果
- [ ] IR 生成在类型检查成功后显式调用
- [ ] 两者可通过 trait 解耦

---

### 2. CodegenContext 职责过重

**位置**: [src/middle/codegen/mod.rs:38-76](src/middle/codegen/mod.rs#L38-L76)

```rust
pub struct CodegenContext {
    module: ModuleIR,
    symbol_table: SymbolTable,           // 符号管理
    constant_pool: ConstantPool,          // 常量池
    bytecode: Vec<u8>,                    // 字节码缓冲区
    current_function: Option<FunctionIR>,
    register_allocator: RegisterAllocator, // 寄存器分配
    label_generator: LabelGenerator,      // 标签生成
    code_offsets: HashMap<usize, usize>,  // 偏移追踪
    jump_tables: HashMap<u16, JumpTable>, // 跳转表
    function_indices: HashMap<String, usize>, // 函数索引
    config: CodegenConfig,                // 配置
    scope_level: usize,                   // 作用域级别
    current_loop_label: Option<(usize, usize)>, // 循环标签
}
```

**问题描述**:
- 单一结构体持有 12+ 个状态字段
- 违反单一职责原则 (SRP)
- 难以测试和维护

**建议拆分为**:
```
CodegenContext
├── BytecodeBuffer          // 字节码生成
├── SymbolTable             // 符号表管理
├── ConstantPool            // 常量池
├── RegisterAllocator       // 寄存器分配
├── LabelManager            // 标签管理
└── JumpTableManager        // 跳转表管理
```

**验收标准**:
- [ ] CodegenContext 不再直接包含这些管理器
- [ ] 每个管理器可独立测试
- [ ] 减少单文件行数

---

### 3. VM Executor ↔ BytecodeGenerator 耦合

**位置**: [src/vm/executor.rs:209-215](src/vm/executor.rs#L209-L215)

```rust
// 问题代码：VM 执行器直接调用代码生成器
use crate::middle::codegen::generator::BytecodeGenerator;
for func_ir in &module.functions {
    let generator = BytecodeGenerator::new(func_ir);
    let func_code = generator.generate();
    self.functions.insert(func_ir.name.clone(), func_code);
}
```

**问题描述**:
- VM 应该只执行预生成的字节码
- 代码生成是编译时行为，不是运行时行为
- 编译时产物应该序列化保存，运行时直接加载

**建议解耦方案**:
```
编译时: BytecodeGenerator.generate() ──保存──> .yxb 文件
运行时: VM.load_from_file() ──执行──> 字节码
```

**验收标准**:
- [ ] VM 不包含代码生成逻辑
- [ ] BytecodeGenerator 产物可序列化/反序列化
- [ ] 区分编译时和运行时模块

---

### 4. Monomorphizer ↔ SendSyncChecker 耦合

**位置**: [src/middle/monomorphize/mod.rs:254-270](src/middle/monomorphize/mod.rs#L254-L270)

```rust
fn is_type_send(&self, ty: &MonoType) -> bool {
    use crate::middle::lifetime::send_sync::SendSyncChecker;
    let checker = SendSyncChecker::new();  // 每次调用都创建新实例！
    checker.is_send(ty)
}

fn is_type_sync(&self, ty: &MonoType) -> bool {
    use crate::middle::lifetime::send_sync::SendSyncChecker;
    let checker = SendSyncChecker::new();  // 重复创建！
    checker.is_sync(ty)
}
```

**问题描述**:
- 每次检查都创建 `SendSyncChecker` 新实例
- 性能浪费
- 实现细节直接暴露

**建议优化方案**:
```rust
// 方案1：注入依赖
impl Monomorphizer {
    fn with_checker(checker: SendSyncChecker) -> Self { ... }
}

// 方案2：缓存实例
struct Monomorphizer {
    send_sync_cache: SendSyncChecker,
}
```

**验收标准**:
- [ ] 单态化过程中不重复创建 SendSyncChecker
- [ ] SendSyncChecker 可被注入
- [ ] 性能测试验证优化效果

---

## 🟡 中等问题 (P1)

### 5. TypeInferrer "全能对象"

**位置**: [src/frontend/typecheck/infer.rs:18-33](src/frontend/typecheck/infer.rs#L18-L33)

```rust
pub struct TypeInferrer<'a> {
    solver: &'a mut TypeConstraintSolver,      // 约束求解
    send_sync_solver: SendSyncConstraintSolver, // Send/Sync 约束
    scopes: Vec<HashMap<String, PolyType>>,    // 作用域栈
    loop_labels: Vec<String>,                   // 循环标签
    current_return_type: Option<MonoType>,      // 返回类型
    current_fn_requires_send: bool,             // Send 标记
    current_fn_type_params: Vec<MonoType>,      // 泛型参数
}
```

**问题描述**:
- 类型推断器混合了 7 种不同职责
- 违反单一职责原则
- 难以独立测试各功能

**建议拆分为**:
```
TypeInferrer (核心推断)
├── ScopeManager        // 作用域管理
├── LoopLabelManager    // 循环标签管理
└── ConstraintManager   // 约束管理
```

**验收标准**:
- [ ] TypeInferrer 移除循环标签逻辑
- [ ] ScopeManager 独立可用
- [ ] 各组件可独立测试

---

### 6. 硬编码的标准库函数签名

**位置**: [src/frontend/typecheck/check.rs:529-573](src/frontend/typecheck/check.rs#L529-L573)

```rust
let stdlib_functions: HashMap<&str, PolyType> = [
    ("print", PolyType::mono(MonoType::Fn { ... })),
    ("println", PolyType::mono(MonoType::Fn { ... })),
    ("read_line", PolyType::mono(MonoType::Fn { ... })),
    ("read_file", PolyType::mono(MonoType::Fn { ... })),
    ("write_file", PolyType::mono(MonoType::Fn { ... })),
];
```

**问题描述**:
- 标准库函数定义在代码中硬编码
- 添加新标准库函数需要修改类型检查器
- 违反开放封闭原则 (OCP)

**建议方案**:
```
std/
├── io.yx      # 定义 print, println, read_line, read_file, write_file
├── math.yx    # 定义数学函数
└── ...
```

**验收标准**:
- [ ] 标准库函数从 .yx 源文件解析
- [ ] 类型检查器不包含硬编码签名
- [ ] 可扩展新的标准库模块

---

### 7. 循环标签栈的侵入式管理

**位置**: [src/frontend/typecheck/infer.rs](src/frontend/typecheck/infer.rs) 多处

```rust
// infer.rs:26
loop_labels: Vec<String>,

// infer.rs:724-732
if let Some(l) = label {
    self.loop_labels.push(l.to_string());
}
// ...
if label.is_some() {
    self.loop_labels.pop();
}
```

**问题描述**:
- 循环标签是控制流分析的子功能
- 混入类型推断器增加复杂性
- 难以复用

**建议方案**:
提取为独立的 `ControlFlowAnalyzer` 或 `LoopContextManager`

**验收标准**:
- [ ] 类型推断器不直接管理循环标签
- [ ] 循环分析逻辑独立
- [ ] break/continue 语义通过接口访问

---

### 8. VM 中的硬编码 "print" 分支

**位置**: [src/vm/executor.rs:657-666](src/vm/executor.rs#L657-L666)

```rust
} else if func_name == "print" {
    self.call_print(first_arg)?;
} else if func_name == "println" {
    self.call_println(first_arg)?;
}
```

**问题描述**:
- 特殊函数名硬编码
- 新增内置函数需要修改 VM
- 与外部函数注册表逻辑重复

**建议方案**:
```
print/println 也通过 EXTERNAL_FUNCTIONS 注册表处理
```

**验收标准**:
- [ ] VM 中无 print/println 硬编码
- [ ] 所有内置函数走统一注册表
- [ ] 可热注册新函数

---

## 🟢 轻微问题 (P2)

### 9. 业务逻辑与日志紧耦合

**问题描述**: `debug!()`, `trace!()` 等日志调用分散在业务代码中

**建议方案**: 使用日志切面或 AOP 模式

**验收标准**:
- [ ] 日志逻辑与业务逻辑分离
- [ ] 可配置日志级别

---

### 10. ModuleGraph 状态硬编码

**位置**: [src/middle/module/mod.rs:67-86](src/middle/module/mod.rs#L67-L86)

```rust
pub enum ModuleStatus {
    Created, Parsing, Parsed,
    TypeChecking, TypeChecked,
    Monomorphizing, Monomorphized,
    Failed,
}
```

**问题描述**: 编译流程状态硬编码，难以扩展新阶段

**建议方案**: 使用状态机模式，支持动态注册状态

**验收标准**:
- [ ] 可添加自定义编译阶段
- [ ] 状态转换规则可配置

---

## 📋 修复优先级排序

| 优先级 | 问题 | 预计工作量 | 风险 |
|--------|------|-----------|------|
| P0-1 | VM ↔ Generator 耦合 | 中 | 低 |
| P0-2 | 类型检查 ↔ IR 生成耦合 | 中 | 中 |
| P0-3 | Monomorphizer ↔ Checker 耦合 | 低 | 低 |
| P0-4 | CodegenContext 职责拆分 | 高 | 中 |
| P1-1 | 标准库函数硬编码 | 中 | 低 |
| P1-2 | TypeInferrer 职责拆分 | 高 | 中 |
| P1-3 | 循环标签管理分离 | 低 | 低 |
| P1-4 | VM print 硬编码 | 低 | 低 |
| P2-1 | 日志与业务分离 | 低 | 低 |
| P2-2 | ModuleGraph 状态扩展 | 中 | 中 |

---

## ✅ 验收标准汇总

### 必须完成 (P0)
- [ ] **P0-1**: VM 不包含代码生成逻辑
- [ ] **P0-1**: BytecodeGenerator 产物可序列化
- [ ] **P0-2**: 类型检查器只返回类型结果
- [ ] **P0-2**: IR 生成在类型检查后显式调用
- [ ] **P0-3**: SendSyncChecker 不重复创建
- [ ] **P0-4**: CodegenContext 拆分为多个管理器

### 建议完成 (P1)
- [ ] **P1-1**: 标准库函数从源文件解析
- [ ] **P1-2**: TypeInferrer 职责分离
- [ ] **P1-3**: 循环标签独立管理
- [ ] **P1-4**: VM print 走注册表

### 可选完成 (P2)
- [ ] 日志切面分离
- [ ] 状态机模式重构

---

## 🔗 相关文件索引

| 问题 | 关键文件 |
|------|---------|
| 1, 2 | src/frontend/typecheck/check.rs |
| 1, 2 | src/frontend/typecheck/infer.rs |
| 2 | src/middle/codegen/mod.rs |
| 3 | src/vm/executor.rs |
| 3 | src/middle/codegen/generator.rs |
| 4 | src/middle/monomorphize/mod.rs |
| 4 | src/middle/lifetime/send_sync.rs |
| 6 | src/frontend/typecheck/check.rs |
| 7 | src/frontend/typecheck/infer.rs |
| 8 | src/vm/executor.rs |
| 10 | src/middle/module/mod.rs |

---

*文档生成时间: 2026-01-19*
*下次审查建议: 修复 P0 问题后*
