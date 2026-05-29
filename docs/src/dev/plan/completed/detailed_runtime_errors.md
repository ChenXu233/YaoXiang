# 运行时详细错误追踪：分离存储与延迟渲染（Detached Storage & Deferred Loading）

> 状态：核心链路已实现（2026-03-15）；DebugSection / CLI 开关已实现（2026-03）；更全面的 Span 覆盖仍属于后续工作。

## 1. 背景与目标
当前姚相 (YaoXiang) 解释器在抛出运行时错误（例如 `Runtime error: Function not found: int.to_string`）时，仅能输出函数名和指令指针（IP），如 `at main (ip: 0)`。这导致开发者难以直观地将错误定位到具体的源代码行和列。

为了提供类似于 Rust 编译器的华丽且详尽的代码高亮报错，同时**绝对不允许在核心解释执行循环中增加时间开销与内存占用**，本计划提出了“分离存储与延迟加载 (Detached Storage & Deferred Loading)”架构。

## 2. 核心设计原则
1. **零运行时成本**（Zero-Cost Runtime）：虚拟机执行逻辑（Interpreter/Runtime）绝不处理复杂的源文件关联对象（如 Span、文件标识），只保留发生异常时的指针 `ip` (Instruction Pointer)。
2. **侧路分离存储**（Side-Table Storage）：生成的 `Span` 追踪信息将被存储在专用的 `DebugSection / DebugMap` 结构中，不参与热点代码缓存以防拖慢指令读取性能。
3. **按需延迟渲染**（Deferred Rendering）：源码片段对齐与渲染只在“确切发生异常并向外冒泡抵达触发点（CLI/TUI）”的那一霎那才作处理。正常执行期间不会发生为了“可能防备出错”而去拼装字符串的行为。

## 3. 当前实现概览（已落地）
- **DebugMap 贯通**：在 Codegen 阶段可选生成 `ip -> Span` 映射，并透传到运行时的 `BytecodeFunction.debug_map`。
- **解释器零开销**：解释器主执行循环不访问 `Span/debug_map`；运行时错误仍只依赖 `StackFrame { function_name, ip }` 冒泡。
- **顶层延迟渲染**：CLI 顶层捕获 `ExecutorError` 后，使用 `debug_map` 将 `ip` 映射回 `Span`，调用现有 `TextEmitter` 输出带代码片段高亮的诊断，并附加 stack trace。

## 4. 具体实现落点（对应原计划阶段）

### 阶段 1：字节码数据结构的扩充 (Middle Core / Bytecode) ✅
`BytecodeFunction` 已包含 `debug_map: HashMap<usize, Span>` 字段，并在本次实现中打通了从 Codegen 到 Runtime 的数据链路：
```rust
pub struct BytecodeFunction {
    // ... 原有逻辑
    /// Debug info: mapping from IP to source Span
    pub debug_map: std::collections::HashMap<usize, crate::util::span::Span>,
}
```
补充：Codegen 侧的 `FunctionCode` 也增加了同名字段，并在 `BytecodeModule::from(BytecodeFile)` 转换时透传到 `BytecodeFunction`。

*后续*：字节码文件序列化/反序列化（`src/middle/passes/codegen/bytecode.rs`）的 DebugSection 写入/读取尚未实现。

### 阶段 2：在中间代码生成期收集映射 (Codegen Translation) ✅
在 `src/middle/passes/codegen/translator.rs` 工作流中：
- 在写入 `FunctionCode.instructions.push(...)` 之前，使用 `instructions.len()` 作为即将生成的 `ip`。
- 从当前 IR 指令中提取 `Span` 并写入 `debug_map[ip] = span`。
- 由 `CodegenContext::set_generate_debug_info(bool)` 控制是否生成 DebugMap（关闭时每个函数仅保留空 `HashMap`，避免额外分配）。

当前可精确定位的 IR 指令（具备 `Span`）：
- `Call / CallVirt / CallDyn`（函数调用相关运行时错误，如 `FunctionNotFound`）
- `Div / Mod`（如除零错误）
- `Store / StoreField / StoreIndex`（为未来更多运行时错误类型预留）

为支持上述映射，IR 层在 `src/middle/core/ir.rs` 中为 `Call/CallVirt/CallDyn/Div/Mod` 补齐了 `span: Span` 字段，且在 `src/middle/core/ir_gen.rs` 中填充（不会影响解释器运行时性能）。

### 阶段 3：运行时栈的精简冒泡 (Interpreter Execution) ✅
* 维持 `src/backends/mod.rs` 的 `StackFrame` 数据不变，仅携带：
  ```rust
  pub struct StackFrame {
      pub function_name: String,
      pub ip: usize,
  }
  ```
* 遇到异常（如：除零、`FunctionNotFound`）触发了 `ExecutorError` 后，连带着简陋的 `StackFrame` 层层返回弹栈（Err mapping）。

### 阶段 4：最顶层拦截与带高亮的延迟渲染 (CLI / Diagnostics) ✅
在 `src/util/diagnostic/mod.rs` 中新增 `render_runtime_error(...)` 并在 `run_file_with_diagnostics(...)` 捕获运行时错误后调用：
1. 捕获最终抛出的 `ExecutorError`，解包 `stack_trace()`。
2. 拿到触发崩溃的帧信息（第一帧），取得其中的 `function_name` 与 `ip`。
3. 在当前 `BytecodeModule` 中定位对应的 `BytecodeFunction`，利用 `debug_map.get(&ip)` 取回 `Span`。
4. **延迟渲染**：仅在错误发生时调用 `TextEmitter` 渲染代码片段高亮，并追加 stack trace 文本。

说明：当前 `Span` 不携带 `file_id`，单文件入口会在 CLI 顶层读入源码并构建 `SourceFile`（编译所需）。运行时错误渲染复用该 `SourceFile`，不会把 IO/字符串处理带入解释器主循环。

---

## 5. 任务分解清单 (Checklist)
- [x] **Data Model准备**：`BytecodeFunction.debug_map` 与 `FunctionCode.debug_map` 数据链路贯通。
- [x] **代码生成对齐**：Translator 在生成字节码时按 `ip` 收集 `Span`（可选开关）。
- [x] **配置与可选序列化**：`CodegenContext::set_generate_debug_info(bool)` + `.42` DebugSection 读写 + CLI `--debug-info` 已实现（2026-03）。
- [x] **顶层错误捕获层构建**：`run_file_with_diagnostics` 捕获 `ExecutorError` 并调用 `render_runtime_error`。
- [x] **终端渲染**：使用 `TextEmitter` 输出带源码片段高亮的运行时错误，并附加 stack trace。

## 6. 已知限制与后续工作
1. **Span 覆盖面**：已在阶段 1 的基础上扩展到 `LoadField/LoadIndex`（并纳入 DebugMap）；仍可继续为更多 IR 指令补齐 `Span` 以覆盖更多运行时错误类型。
2. **多文件/模块**：`Span` 本身仍仅包含行列信息，但 DebugMap 已升级为 `ip -> (file_id + span)`，并引入 `SourceMap`（`file_id -> path/content`）用于跨文件渲染；后续可在编译管线引入真实的多文件 `file_id` 分配策略。
3. **字节码文件 DebugSection**：已扩展 `.42` 格式加入可选 DebugSection（sources + ip 映射）并实现读写；可用于离线执行/调试时保留定位信息。
4. **CLI 开关**：已为 `run` / `build` 增加 `--debug-info`，用于控制 DebugMap 的生成与 `.42` DebugSection 的嵌入。
