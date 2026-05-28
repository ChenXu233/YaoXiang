# 运行时详细错误追踪（阶段 2）：Span 覆盖、多文件 SourceMap、Bytecode DebugSection、CLI 开关

> 状态：计划中（用于驱动实现与验收）  
> 关联：`detailed_runtime_errors.md`（阶段 1 已落地的核心链路）

## 目标（本阶段要解决的 4 个限制）
1. **Span 覆盖面**：为更多 IR 指令补齐 `Span`，并在 Codegen 阶段纳入 DebugMap。
2. **多文件/模块**：在 DebugMap 中记录来源文件；引入 `SourceMap`（`file_id -> path/content`）以支持跨模块定位与渲染。
3. **字节码文件 DebugSection**：扩展 `.42`（BytecodeFile）格式，加入可选 DebugSection；实现读写，用于离线执行/调试时保留定位信息。
4. **CLI 开关**：为 `run` / `build`（build-bytecode）等命令增加 `--debug-info`，控制 DebugMap 的生成与（本阶段实现的）序列化。

## 设计约束（必须满足）
- **解释器主循环零成本**：运行时执行逻辑仅依赖 `ip` 与 `StackFrame` 冒泡，不在热点路径访问/解析 debug 信息。
- **按需渲染**：仅在错误最终冒泡到 CLI 层时，才加载/查表/渲染源码片段。
- **可选开关**：关闭 `--debug-info` 时不产生 DebugMap，不写入 DebugSection，避免额外分配与文件膨胀。

## 数据结构方案（避免破坏性改动）
为避免将 `file_id` 侵入现有 `Span`（目前大量组件依赖 `Span { start, end }`），本阶段采用“组合式定位”：
- `Span` 仍仅表达行列/offset（保持现状）。
- 新增 `FileId`（整数 ID）与 `DebugSpan { file_id, span }`。
- 新增 `SourceMap`：按 `file_id` 索引 `SourceFile { name, content, ... }`。
- 将 `BytecodeFunction.debug_map` 的 value 从 `Span` 升级为 `DebugSpan`。

> 好处：不会大面积重写 Parser/LSP/TypeCheck 的 Span 逻辑；仅 Debug 链路做升级。

## BytecodeFile DebugSection（文件格式扩展）
### 触发条件
- `BytecodeFile.header.flags` 中设置 `DEBUG_INFO` 位（与 Codegen/CLI 对齐）。

### 存储内容（最小闭环）
- **SourceMap**：`file_count`，每个文件写入 `path` 与 `content`。
- **DebugMap**：按函数顺序存储 `ip -> DebugSpan` 映射：
  - `entry_count`
  - 每条 entry：`ip`、`file_id`、`Span.start(line/col/offset)`、`Span.end(line/col/offset)`

### 兼容性策略
- 未开启 `DEBUG_INFO`：不写 DebugSection，旧格式保持不变。
- 读取：若 flags 未开启则跳过 DebugSection；若开启则解析并填充 `FunctionCode.debug_map`。

## CLI 开关行为（验收标准）
### `yaoxiang run <file> --debug-info`
- 开启 Codegen 的 DebugMap 收集。
- 运行时错误输出包含源码片段高亮与（跨文件时）正确的文件名/路径。

### `yaoxiang build <file> -o out.42 --debug-info`
- 写入 DebugSection（包含 SourceMap + DebugMap）。
- 关闭开关时 `.42` 不包含 DebugSection。

## Span 覆盖面扩展（建议优先级）
> 以“可能触发运行时错误”的 IR 指令为优先，逐步补齐。

### P0（直接关联已有错误类型）
- `Call/CallVirt/CallDyn`：`FunctionNotFound`
- `Div/Mod`：`DivisionByZero`

### P1（为未来错误类型/检查预留）
- `LoadIndex/StoreIndex`：`IndexOutOfBounds`（未来结合 `BoundsCheck`）
- `LoadField/StoreField`：`FieldNotFound`
- `PtrDeref/PtrLoad/PtrStore`：`InvalidHandle` / unsafe 错误

## 测试与验收
- `BytecodeFile`：DebugSection 写入后可 `read_from` 完整还原 `SourceMap` 与 `DebugMap`（round-trip）。
- `render_runtime_error`：可根据 `DebugSpan.file_id` 从 `SourceMap` 选择正确文件渲染（单文件/多文件用例各 1 个）。
- CLI：`--debug-info` 开关能显著改变输出（开启：源码高亮；关闭：仅 ip/函数名）。

