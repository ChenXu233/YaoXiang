# 空壳代码清单

> 生成日期：2026-06-13
> 检查范围：整个项目 (`src/`)
> 检查类型：`todo!()`、空函数体、硬编码返回值、死代码

## 统计概览

| 类型 | 数量 | 优先级分布 |
|------|------|-----------|
| `todo!()` | 4 处 | P0: 4 |
| 空函数体 | 6 处 | P0: 2, P1: 2, P2: 2 |
| 硬编码返回值 | 14 处 | P0: 5, P1: 8, P2: 1 |
| 死代码 | 14 处 | P2: 14 |
| 重复实现 | 4 处 | P2: 4 |
| **总计** | **42 处** | |

---

## P0 - 高优先级（核心功能缺失）

### 1. 调试器步进方法（4 处 `todo!()`）

**文件**：`src/backends/interpreter/executor/debug.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 32-34 | `fn step(&mut self) -> ExecutorResult<()>` | 单步执行一条指令 | `todo!()` |
| 36-38 | `fn step_over(&mut self) -> ExecutorResult<()>` | 单步跳过（不进入函数内部） | `todo!()` |
| 40-42 | `fn step_out(&mut self) -> ExecutorResult<()>` | 单步跳出（执行完当前函数） | `todo!()` |
| 44-46 | `fn run(&mut self) -> ExecutorResult<()>` | 继续运行到下一个断点 | `todo!()` |

**上下文**：`DebuggableExecutor` trait 的其他方法（`set_breakpoint`、`has_breakpoint`、`current_ip`、`current_function`、`breakpoints`）已实现，只有步进控制未实现。

**实现建议**：
- `step()`：执行当前 IP 指令，IP++
- `step_over()`：如果当前指令是函数调用，设置临时断点在下一条指令，然后 run
- `step_out()`：记录当前调用栈深度，run 直到栈深度减少
- `run()`：循环执行直到遇到断点或程序结束

---

## P1 - 中优先级（功能不完整）

### 6. LSP 进度通知（1 处空函数体）

**文件**：`src/frontend/events/subscribe.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 357-364 | `fn on_event(&self, _event: &dyn Any, _metadata: &EventMetadata)` | 将事件转换为 LSP 通知 | 空实现 |

**上下文**：注释"这里可以添加 LSP 通知逻辑，例如将进度事件转换为 $/progress 通知"。

**实现建议**：
- 检查事件类型（Progress、Diagnostic 等）
- 对于 Progress 事件，发送 `window/workDoneProgress/create` 和 `$/progress` 通知
- 对于 Diagnostic 事件，发送 `textDocument/publishDiagnostics` 通知

---

### 7. 旧语法跳过函数（1 处空函数体）

**文件**：`src/frontend/core/parser/statements/declarations.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 171-174 | `fn skip_old_function_syntax(_state: &mut ParserState<'_>)` | 跳过旧函数语法的整个声明 | 空实现 |

**上下文**：注释"旧语法已移除，此函数不再需要"。

**建议**：检查是否还有调用方，如无调用方直接删除。

### 11. 边界检查（2 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/inference/bounds.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 70-79 | `pub fn check_const_bounds(&self, _ty: &MonoType, _bounds: &[ConstBound]) -> Result<()>` | 检查 const 边界 | 硬编码 `Ok(())` |
| 81-90 | `pub fn check_lifetime_bounds(&self, _ty: &MonoType, _bounds: &[LifetimeBound]) -> Result<()>` | 检查生命周期边界 | 硬编码 `Ok(())` |

**实现建议**：
- `check_const_bounds`：
  - 检查 const 参数是否满足边界约束
  - 检查 const 表达式是否可求值
- `check_lifetime_bounds`：
  - 检查生命周期参数是否满足边界约束
  - 检查生命周期是否比约束更长

---

### 12. 解构赋值检查（1 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/inference/assignment.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 137-146 | `pub fn check_destructuring(&self, _lhs_patterns: &[Pattern], _rhs: &MonoType, _span: Span) -> Result<()>` | 检查解构赋值的形状是否匹配 | 硬编码 `Ok(())` |

**实现建议**：
- 检查左侧模式的数量是否与右侧类型匹配
- 检查每个模式的类型是否与右侧对应位置的类型匹配
- 报告不匹配的具体位置

---

### 13. 泛型约束解析（1 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/inference/generics.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 53-59 | `pub fn infer_generic_constraints(&mut self, _constraints: &[String]) -> Result<()>` | 从约束字符串解析为内部表示 | 硬编码 `Ok(())` |

**实现建议**：
- 解析约束字符串（如 `T: Clone + Debug`）
- 提取类型参数和约束
- 将约束转换为内部表示（TraitConstraint）
- 添加到类型环境中

---

## P2 - 低优先级（可安全删除的死代码）

### 28-31. 重复的 `substitute_type` 实现

| # | 文件 | 行号 | 签名 | 差异 | 建议 |
|---|------|------|------|------|------|
| 28 | `types/eval/evaluator.rs` | 1040 | `fn substitute_type(body, param_name, replacement)` | 仅 TypeRef 替换 | 删除（无调用方） |
| 29 | `types/solver.rs` | 685 | `fn substitute_type(&self, ty, substitution)` | 完整子节点替换 | 保留 |
| 31 | `middle/passes/mono/cross_module.rs` | 609 | `fn substitute_type(generic_type, type_args, type_params)` | 按参数列表替换 | 保留 |

---

## 合理的空实现（保留）

| 文件 | 函数 | 原因 |
|------|------|------|
| `frontend/events/mod.rs:131` | `NullEmitter::emit/emit_with` | Null Object Pattern |
| `backends/runtime/facade.rs:306,331` | `EmbeddedRuntime::cancel/drive_until` | 嵌入运行时语义 |
| `backends/common/allocator.rs:195` | `BumpAllocator::dealloc` | Bump 分配器特性 |
| `frontend/core/typecheck/passes/dead_code.rs:190` | `collect_definitions` | 已 deprecated，合理桩 |

---

## 实现进度跟踪

---

## 备注
- P0/P1 需要仔细设计，建议逐个实现并添加测试
- 部分函数可能有隐藏的调用方（通过 trait object 或宏），删除前建议再次确认
- 重复的 `substitute_type` 建议统一为单一实现
