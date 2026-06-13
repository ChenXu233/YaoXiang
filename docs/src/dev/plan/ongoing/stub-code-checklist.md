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

### 2. 控制流分析核心（2 处空函数体）

**文件**：`src/middle/passes/lifetime/control_flow.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 145-154 | `fn analyze_instruction(&self, _instr: &Instruction, _state: &mut HashMap<Operand, ValueState>, _pos: (usize, usize))` | 分析单条指令的生命周期状态变化 | 空实现 |
| 155-163 | `fn merge_block_state(&mut self, _block_state: &HashMap<Operand, ValueState>, _block_idx: usize)` | 合并来自不同基本块的 ValueState | 空实现 |

**上下文**：注释说明"目前为空实现，后续可根据需要扩展"、"控制流分析在 MoveChecker 中已有基本实现"。

**实现建议**：
- `analyze_instruction`：
  - 匹配指令类型（Move、Copy、Call、Branch 等）
  - 更新 `_state` 中对应 Operand 的 ValueState（Moved/Empty/Partial）
  - 处理函数调用参数的 Move
- `merge_block_state`：
  - 对于多个前驱块汇入的情况，取 ValueState 的最小上界（LUB）
  - 如果任一前驱是 Moved，则汇入后是 Moved
  - 如果前驱状态冲突，报告错误或标记为 MaybeMoved

**影响**：当前 lifetime pass 退化为 no-op，无法检测 use-after-move 错误。

---

### 3. Trait 对象安全检查（2 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/traits/object_safety.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 62-71 | `fn check_associated_types(&self, _trait_name: &str) -> Result<(), ObjectSafetyError>` | 检查关联类型是否对象安全 | 硬编码 `Ok(())` |
| 74-85 | `fn check_method_signatures(&self, _trait_name: &str) -> Result<(), ObjectSafetyError>` | 检查方法签名是否对象安全 | 硬编码 `Ok(())` |

**上下文**：注释"简化实现：假设没有关联类型或关联类型都是安全的"、"假设基本特质的方法都是对象安全的"。

**实现建议**：
- `check_associated_types`：
  - 获取 trait 的所有关联类型
  - 检查关联类型是否有 `Self` 约束（不安全）
  - 检查关联类型是否在方法签名中使用（不安全）
- `check_method_signatures`：
  - 检查方法返回类型是否包含 `Self`（不安全）
  - 检查方法是否有泛型参数（不安全）
  - 检查方法是否使用 `where Self: Sized` 约束（安全，但需特殊处理）

---

### 4. Trait 一致性检查（3 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/traits/coherence.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 38-43 | `fn check_conflicting_implementations(&self) -> Result<()>` | 检查是否有冲突的 trait 实现 | 硬编码 `Ok(())` |
| 46-50 | `fn check_orphan_rule(&self) -> Result<()>` | 检查孤儿规则 | 硬编码 `Ok(())` |
| 80-86 | `fn find_orphan_implementations(&self) -> Result<()>` | 扫描并检查所有 trait 实现 | 硬编码 `Ok(())` |

**上下文**：注释"简化实现：检查是否有重复的特质实现"、"确保特质实现符合孤儿规则"。

**实现建议**：
- `check_conflicting_implementations`：
  - 收集所有 trait 实现
  - 对于同一类型的多个实现，检查是否有重叠
  - 报告冲突的实现
- `check_orphan_rule`：
  - 对于每个 trait impl，检查 trait 或类型是否在当前 crate 定义
  - 如果都不在，报告孤儿规则违规
- `find_orphan_implementations`：
  - 遍历所有模块的 trait impl
  - 调用 `check_orphan_rule` 检查每个实现

---

### 5. Trait 实现签名检查（1 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/traits/impl_check.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 95-103 | `fn check_signature(&self, _trait_def: &TraitDef, _params: &[Param]) -> Result<()>` | 检查 impl 方法签名与 trait 定义是否一致 | 硬编码 `Ok(())` |

**上下文**：当前只检查方法名是否存在，签名检查为空。

**实现建议**：
- 对比参数类型（包括泛型参数）
- 对比返回类型
- 对比 mut 修饰符
- 对比生命周期参数
- 报告不匹配的具体位置

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

---

### 8. GAT 检查（2 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/traits/gat/checker.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 122-131 | `fn validate_generic_usage(&self, _ty: &MonoType) -> Result<()>` | 验证泛型参数的使用是否合法 | 硬编码 `Ok(())` |
| 174-184 | `pub fn check_associated_type_constraints(...)` | 检查关联类型约束 | 硬编码 `Ok(())` |

**实现建议**：
- `validate_generic_usage`：
  - 检查泛型参数是否在允许的位置使用
  - 检查是否有未使用的泛型参数
  - 检查是否有违反约束的使用
- `check_associated_type_constraints`：
  - 检查关联类型是否满足约束
  - 检查约束是否可满足

---

### 9. 高阶类型生命周期检查（1 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/traits/gat/higher_rank.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 100-109 | `fn check_lifetime_constraints(&self, _ty: &MonoType) -> Result<()>` | 检查生命周期参数的使用 | 硬编码 `Ok(())` |

**实现建议**：
- 检查生命周期参数是否在允许的位置使用
- 检查生命周期参数是否满足约束
- 检查是否有违反高阶生命周期规则的使用

---

### 10. 约束传播（1 处硬编码返回值）

**文件**：`src/frontend/core/typecheck/traits/solver.rs`

| 行号 | 函数签名 | 预期功能 | 状态 |
|------|----------|----------|------|
| 311-318 | `pub fn propagate_constraints_to_type_args(&self, _ty: &MonoType, _trait_name: &str) -> Vec<TraitConstraint>` | 从类型参数中提取子约束并传播 | 硬编码 `Vec::new()` |

**实现建议**：
- 获取类型的泛型参数
- 对于每个泛型参数，检查其约束
- 将约束传播到具体类型参数
- 返回传播后的约束列表

---

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

### 14-27. 死代码清单

| # | 文件 | 行号 | 元素 | 类型 | 建议 |
|---|------|------|------|------|------|
| 14 | `frontend/pipeline.rs` | 907-932 | `impl TypecheckResult` 块 | 死代码 | 删除 |
| 15 | `frontend/pipeline.rs` | 960 | `failed_proofs` 字段 | 死代码 | 删除 |
| 16 | `parser/statements/declarations.rs` | 103-112 | `fn_returns_meta_type` | 死代码 | 删除 |
| 17 | `parser/statements/declarations.rs` | 114-132 | `generic_params_from_constructor_params` | 死代码 | 删除 |
| 18 | `parser/statements/types.rs` | 41-63 | `looks_like_parenthesized_lambda` | 死代码 | 删除 |
| 19 | `types/eval/evaluator.rs` | 1039-1093 | `substitute_type` | 死代码 | 删除 |
| 20 | `types/eval/evaluator.rs` | 1117-1125 | `integrate_evaluator` | 死代码 | 删除 |
| 21 | `types/eval/evaluator.rs` | 1131-1154 | `sync_caches` | 死代码 | 删除 |
| 22 | `module/cache.rs` | 35-36 | `cached_at` 字段 | 死代码 | 删除 |
| 23 | `util/diagnostic/session.rs` | 13-14 | `cache` 字段 | 死代码 | 删除 |
| 24 | `middle/passes/lifetime/cycle_check.rs` | 22-23 | `MAX_DETECTION_DEPTH` 常量 | 死代码 | 删除 |
| 25 | `middle/passes/lifetime/intra_task_cycle.rs` | 26-27 | `value_defs` 字段 | 死代码 | 删除 |
| 26 | `typecheck/proof/budget.rs` | 59-65 | `record_time_ms` + `time_ms_used` | 死代码 | 删除 |
| 27 | `typecheck/layers/termination.rs` | 854-938 | 3 个函数 | 死代码 | 删除 |
| 28 | `typecheck/checker.rs` | 1661-1677 | `check_refined_binding` | 死代码 | 删除 |
| 29 | `typecheck/layers/ownership.rs` | 9-12 | 整个文件 | 死代码 | 删除 |
| 30 | `util/diagnostic/emitter/text.rs` | 259-262 | `hint_prefix` | 死代码 | 删除 |

---

### 28-31. 重复的 `substitute_type` 实现

| # | 文件 | 行号 | 签名 | 差异 | 建议 |
|---|------|------|------|------|------|
| 28 | `types/eval/evaluator.rs` | 1040 | `fn substitute_type(body, param_name, replacement)` | 仅 TypeRef 替换 | 删除（无调用方） |
| 29 | `types/solver.rs` | 685 | `fn substitute_type(&self, ty, substitution)` | 完整子节点替换 | 保留 |
| 30 | `types/traits/specialization/algorithm.rs` | 66 | `fn substitute_type(&self, ty)` | 完整子节点替换 | 保留 |
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

| 优先级 | 总数 | 已完成 | 剩余 |
|--------|------|--------|------|
| P0 | 12 | 0 | 12 |
| P1 | 11 | 0 | 11 |
| P2 | 19 | 0 | 19 |
| **总计** | **42** | **0** | **42** |

---

## 备注

- P2 死代码可以安全删除，不影响功能
- P0/P1 需要仔细设计，建议逐个实现并添加测试
- 部分函数可能有隐藏的调用方（通过 trait object 或宏），删除前建议再次确认
- 重复的 `substitute_type` 建议统一为单一实现
