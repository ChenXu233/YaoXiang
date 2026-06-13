# RFC-018 LLVM AOT 编译器与 L3 透明并发（DAG 延迟调度）实现计划

> **⚠️ 对齐说明**：本文档基于旧的并发模型（`@block`/`@eager`/`@auto` 注解、`Send`/`Sync` trait、L1/L2/L3 层级），已被 [RFC-024 新并发模型](/design/rfc/accepted/024-concurrency-model.md) 取代。本文档需要与 RFC-024 对齐后才能继续推进。当前并发模型以 `spawn {}` 块为唯一并行原语，无注解，无 Send/Sync。

> **任务**：实现 LLVM AOT 后端 + 运行时 DAG 调度器（~~落地 `@auto/@eager/@block` 三种调度策略~~ 已废弃，需对齐 RFC-024）  
> **基于 RFC**：RFC-018（草案）  
> **依赖 RFC**：RFC-001（并作模型与错误处理）、RFC-008（三层运行时）、RFC-009（所有权/Arc）  
> **日期**：2026-03-10  
> **状态**：进行中  
> **目标里程碑**：  
> - M1：LLVM AOT（可编译可执行，串行）  
> - M2：DAG 元数据 + 单线程调度（Standard Runtime，num_workers=1）  
> - M3：多线程并行调度 + 粒度控制（Full Runtime，num_workers>1）  
> - M4：延迟调度（Lazy Task Creation）+ **资源类型（Resource）副作用抽象** + **错误传播/错误图** + 注解贯通

---

## 摘要（实现闭环）

- 在 `yaoxiang` 内新增 LLVM 后端（feature gate），将 `BytecodeModule` 编译为机器码（COFF/ELF/Mach-O）并可在运行时加载执行。
- 引入**稳定 ABI**：AOT 生成代码与运行时通过 `extern "C"` 的 `RtValue/RtContext` 交互，避免 Rust enum ABI 不稳定问题。
- 落地 RFC-018 的核心：**函数块内 DAG** + **延迟调度（Lazy Scheduling）**。并发/串行由 **DAG 边（Data/Control/Spawn）** 与 **资源类型（Resource）规则**共同决定；错误按 RFC-001 沿依赖边传播并可形成错误图。

---

## 公共接口/行为变更（对外可见）

1. **Cargo features**
   - 新增 `llvm-aot` feature：启用 LLVM/inkwell 依赖与 AOT 后端；默认关闭（保证无 LLVM 环境也能构建）。
2. **CLI**
   - `yaoxiang run` 增加 `--backend {interpreter,llvm}`（默认 interpreter）。
   - 可选：增加 `--runtime {embedded,standard,full}` 与 `--workers <N>` 控制运行时层级与并发度（RFC-008）：
     - `--runtime embedded`：即时执行（无 DAG，无调度器特性，适合嵌入式/极简场景）
     - `--runtime standard`：DAG + Scheduler（num_workers=1 为异步；>1 为并行）
     - `--runtime full`：standard + WorkStealer（高级特性，可选）
3. **运行时 ABI（内部但跨模块）**
   - 新增 `RtValue`（`#[repr(C)]`）与 `RtContext`（仅包含指针/基本类型）作为 AOT 与 runtime 交互边界。

---

## 关键设计约束（与 RFC-001 / RFC-008 / RFC-018 对齐）

### A. 并发语义（L1/L2/L3 只是心智模型）

- **L3（默认 / @auto）**：透明并发；构建 DAG；遇到调用先返回“可延迟求值”的值，**需要值时才触发求值**。
- **L1（@block）**：由标准库提供（RFC-008），语义为“强制急切求值”，不进入 DAG 惰性队列；主要用于调试与关键顺序段。
- **L2（spawn）**：**只能在 @block 作用域内使用**（RFC-001/008），用于在同步代码中插入并发；属于 Full Runtime 能力。

### B. 运行时三层（RFC-008）

- **Embedded Runtime**：即时执行；可选择完全不构建 DAG（省内存/省启动）；用于受限环境。
- **Standard Runtime**：DAG + Scheduler 为核心（惰性求值天然支持异步/并行）。
- **Full Runtime**：在 standard 基础上增加 WorkStealer，以及标准库层的 `@block` / `spawn` 等能力。

### C. DAG 构建范围与内存（RFC-001/018）

- DAG 只在**单个函数体/块内**构建；不递归展开被调函数体（避免错误图与 DAG 节点爆炸）。
- DAG 元数据必须携带**节点/边的稳定 ID** 与 **Span**（用于错误传播与错误图定位）。

### D. 副作用抽象（RFC-001：资源类型）

- 不引入额外“显式副作用标注系统”；副作用统一为**资源操作**：
  - 参数类型包含 `Resource`（或其派生资源类型）的函数调用，视为资源操作；
  - 对同一资源的操作自动形成 **ControlEdge**（串行）；对不同资源可并行；
  - 无法静态判定是否同一资源时，默认保守串行（可后续引入显式 unsafe 并行提示作为扩展）。

### E. 错误传播（RFC-001）

- 错误沿 DAG 的依赖边向上传播（与实际并行执行顺序无关），并记录传播路径用于错误图。

---

## 阶段 0：前置与约束锁定（1-2 天）

### 0.1 锁定 LLVM/inkwell 版本与构建方式

**目标**
- 选定 LLVM 主版本 = **17**（统一团队环境；Windows/Linux/macOS 皆可获取对应发行包）。
- 在 `Cargo.toml` 增加 `inkwell`（启用 `llvm17-0` 对应 feature），并挂到 `llvm-aot` feature 下。

**验收标准**
- [ ] `cargo build`（不带 feature）可通过（无 LLVM 环境也能构建）。
- [ ] `cargo build -F llvm-aot` 在配置好 LLVM17 环境时可通过。

**测试项目**
- [ ] CI/本地：两组构建矩阵（`default` 与 `-F llvm-aot`）至少一平台跑通。
- [ ] 最小 smoke：`cargo test -F llvm-aot` 能启动并执行一个空测试模块（只验证链接）。

---

### 0.2 LLVM 环境探测与错误信息

**目标**
- 增加构建时/运行时探测说明：缺少 `llvm-config`/LLVM 目录时给出可操作的错误提示（如何安装/如何设置前缀变量）。

**验收标准**
- [ ] LLVM 缺失时，错误信息包含：期望版本（17）、可用环境变量（如 `LLVM_SYS_170_PREFIX` 或 `LLVM_CONFIG_PATH`）与示例路径。

**测试项目**
- [ ] 在无 LLVM 环境的机器上执行 `cargo build -F llvm-aot`，输出提示完整且不 panic（编译期报错即可）。

---

### 0.3 并作模型落地约束锁定（RFC-001/008 对齐）

**目标**
- 明确并固化以下实现约束（写进代码注释/开发文档与测试用例）：
  - `spawn` 仅允许出现在 `@block` 作用域内（解析/类型检查/IR 阶段均需防守）。
  - `@block` 语义为“急切求值”，由标准库能力提供（可先以编译器内置实现 MVP，但要保留未来下放标准库的接口）。
  - DAG 仅在函数块内构建；必须携带稳定的 `node_id` 与 `span`（支撑错误传播/错误图）。
  - 资源类型（Resource）驱动 ControlEdge 生成，避免引入额外用户可见 effect 注解体系。
  - **并行安全约束（RFC-001/009）**：仅当节点捕获/返回值满足 `Send + Sync`（或语言侧等价约束）时允许跨线程并行；否则必须降级为串行（或固定在单 worker 上执行）。

**验收标准**
- [ ] 编译器在不合法的 `spawn` 场景下给出明确错误（含 Span）。
- [ ] `@block/@eager/@auto` 的语义差异在最小示例中可观测且可测试。
- [ ] 文档（本计划）与 RFC-001/008/018 的关键决议一致，无自相矛盾条目。

**测试项目**
- [ ] `spawn` 位置限制测试：@block 之外出现 spawn 必须报错。
- [ ] DAG scope 测试：确认 DAG 不跨函数体展开（节点数与调用层级无关）。
- [ ] Send/Sync 约束测试：
  - `spawn` 捕获非 `Send` 值必须报错（含 Span）。
  - `@auto` 下包含非 `Send + Sync` 值的节点不得跨线程调度（可用 `std.concurrent.thread_id` 统计验证）。

---

## 阶段 1：LLVM 后端骨架与选择开关（1-2 天）

### 1.1 新增后端模块与对齐 RFC-018 目录结构

**目标**
- 新增 `src/backends/llvm/`，包含：`mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs`（允许后续合并/拆分）。
- 在 `src/backends/mod.rs` 中通过 `#[cfg(feature = "llvm-aot")] pub mod llvm;` 暴露模块。

**验收标准**
- [ ] `cargo test`（默认 feature）通过。
- [ ] `cargo test -F llvm-aot` 通过（哪怕 LLVM 后端暂未实现完整功能）。

**测试项目**
- [ ] 单元：`src/backends/llvm/tests.rs` 中至少 1 个编译期测试能运行（仅验证模块可引用）。

---

### 1.2 后端选择：CLI/库侧注入点

**目标**
- 在 CLI `Run` 子命令上新增 `--backend` 参数（ValueEnum）：`interpreter`（默认）/ `llvm`（需 feature）。
- 在 `yaoxiang::run_*` 路径增加后端选择分支，抽象为 `fn make_executor(kind, config) -> Box<dyn Executor>`（或枚举分发，避免 trait object 也可）。

**验收标准**
- [ ] `yaoxiang run file.yx` 仍走解释器，行为不变。
- [ ] `yaoxiang run --backend llvm file.yx`：若未开启 feature，给出明确错误；若开启 feature，进入 LLVM 执行路径（即使暂时返回 “not implemented” 也必须是可控错误）。

**测试项目**
- [ ] CLI 参数解析测试（在 `tests/integration` 增加）。
- [ ] 负向测试：无 feature 时传 `--backend llvm` 返回可读错误信息。

---

## 阶段 2：稳定 ABI（RtValue/RtContext）与 Runtime API（3-5 天）

> 该阶段是“LLVM 生成代码可执行”的关键：必须先把跨边界值表示稳定下来。

### 2.1 定义 `RtValue`（稳定 C ABI）

**目标**
- 定义 `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }`（或 16 字节结构，保持对齐简单）。
- 定义 `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }`（最小集合；后续扩展）。
- 约定：
  - Int：`payload` = `i64` bits
  - Float：`payload` = `f64` bits
  - Bool：0/1
  - Handle：`payload` = `usize`（扩展到 u64）
  - Async：`payload` = `TaskId`（u64）
  - Error：`payload` = 错误码或指针（MVP 可先用错误码）

**验收标准**
- [ ] `RtValue` 可在 Rust 内部安全构造/读取（无 UB），并具备 `Debug` 输出与基本断言工具函数。
- [ ] 与 LLVM IR 对齐：能够在 inkwell 中创建同布局的 struct type（字段顺序/大小一致）。

**测试项目**
- [ ] `RtValue` roundtrip：int/float/bool/unit 的 encode/decode 单元测试。
- [ ] ABI 大小测试：`size_of::<RtValue>()` 与 `align_of::<RtValue>()` 固定（写死断言，防止未来误改）。

---

### 2.2 定义 `RtContext`（运行时上下文）

**目标**
- 定义 `#[repr(C)] struct RtContext`，仅包含指针/整数：
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler`（或指向具体实现）
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph`（可选：用于 RFC-001 的错误传播记录；MVP 可为 null）
  - 预留字段（版本号/flags），但保持最小化（KISS）。

**验收标准**
- [ ] `RtContext` 可由解释器/LLVM executor 构造并传给生成代码。
- [ ] 不包含 Rust 非稳定布局字段（禁止直接内嵌 `Heap`/`FfiRegistry` 值）。

**测试项目**
- [ ] 构造/销毁 `RtContext` 的内存安全测试（无需真实 LLVM）。

---

### 2.3 Runtime C API：生成代码调用的最小函数集

**目标**
- 提供 `#[no_mangle] extern "C"` 函数（命名统一前缀 `yx_rt_*`），MVP 最少包含：
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*函数指针*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` 或 `yx_rt_trap(msg_ptr, len)`（调试用）
- 约束：AOT 生成代码**只通过上述 API 交互**，不直接操作 Rust 结构体。

**验收标准**
- [ ] 运行时 API 在无 LLVM 情况也可编译（受 `llvm-aot` feature 控制：API 可常驻或仅在 feature 下提供，但必须可测试）。
- [ ] `yx_rt_native_call` 能调用 `FfiRegistry` 的 handler（MVP 仅支持 Int/Float/Bool/Unit 参数与返回值；不支持则返回 Error RtValue），并在失败时记录 `node_id/span_id` 到错误图（若启用）。

**测试项目**
- [ ] 纯 Rust 单元测试：直接调用 `yx_rt_native_call`，验证 `std.io.println`（或自注册函数）路径可用。
- [ ] 错误路径测试：传不存在的 native 名称，返回 `Error` RtValue 且可转为 `ExecutorError::FunctionNotFound`。

---

## 阶段 3：LLVM Codegen 基础设施（2-3 天）

### 3.1 LLVM 上下文/模块/TargetMachine 初始化

**目标**
- `context.rs`：封装 inkwell `Context/Module/Builder` 生命周期。
- 初始化 Target：根据 `PlatformDetector`（支持 `LLVM_TARGET`）与宿主三元组设置 target triple + data layout。
- 支持输出：
  - LLVM IR（`.ll`）用于调试
  - Object（`.o/.obj`）用于 AOT

**验收标准**
- [ ] 对任意空 `BytecodeModule`，可生成一个包含 `main` 的 LLVM Module（哪怕函数体仅返回 Unit）。
- [ ] IR 可验证（调用 LLVM verify；失败时返回可读错误）。

**测试项目**
- [ ] 单元：生成最小 module 并 verify 通过。
- [ ] 快照测试（可选）：对 `.ll` 关键片段做字符串包含断言（避免 brittle 全量快照）。

---

### 3.2 `TypeMap`：YaoXiang Type → LLVM Type（MVP）

**目标**
- `types.rs`：实现 `fn llvm_type(yao_type: &Type) -> BasicTypeEnum`，先覆盖：
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void（或用 `RtValue(Unit)` 统一返回）
- 策略选择（为减少 ABI 面积）：**所有函数统一返回 `RtValue`**（而非按类型返回），让 codegen 与调度器/FFI统一处理；类型信息用于静态检查与生成 `RtValue` 构造/解构逻辑。

**验收标准**
- [ ] `TypeMap` 对上述类型映射稳定，且 LLVM IR 中函数签名一致：`fn(*mut RtContext, *const RtValue, usize) -> RtValue`。

**测试项目**
- [ ] `TypeMap` 单测：给定 `Type::Int/Float/Bool/Void` 生成 LLVM 类型成功。
- [ ] 生成的函数签名在 LLVM module 中可检索并断言参数/返回类型匹配。

---

## 阶段 4：指令翻译 MVP（5-8 天）

### 4.1 寄存器到 LLVM 值映射（SSA 化的最小实现）

**目标**
- `values.rs`：实现虚拟寄存器 `Reg(u16)` → LLVM `Value` 的映射表（按基本块作用域管理）。
- 约定：所有寄存器值都以 `RtValue` 表示（避免类型爆炸/ABI 不一致），运算/比较前通过 helper 强制/解包。

**验收标准**
- [ ] 生成代码在控制流分叉后能正确合并寄存器值（使用 phi 或在 `RtValue` 层以统一类型处理）。

**测试项目**
- [ ] 单元：对包含 if/else 的 BytecodeFunction 生成 IR，并 verify 通过。
- [ ] 回归：同一寄存器多次赋值不会导致 use-before-def（debug 模式下插入 trap/错误）。

---

### 4.2 翻译核心指令子集（覆盖“可跑起来”）

**目标**
- `codegen.rs`：实现至少以下 `BytecodeInstr`：
  - `LoadConst`（Int/Float/Bool/String 先限定：String 可降级为 Error 或先不支持）
  - `Mov`
  - `BinaryOp`（Add/Sub/Mul/Div：Int 与 Float 各自路径）
  - `Compare`（Eq/Lt/Gt 等）
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative`（通过 `yx_rt_native_call`）
  - `CallStatic`（两种策略：`@block` 直接 call；`@auto` 走 `yx_rt_lazy_call` 返回 Async）
- 强制规则：凡是参与算术/比较/分支的操作数都必须先 `yx_rt_force`（透明并发的“需要值时触发”）。

**验收标准**
- [ ] AOT 后端可执行简单程序：
  - 纯算术
  - if/else
  - 调用 `std.io.println` 输出
- [ ] 不支持的指令会产生可读错误（不是 panic）。

**测试项目**
- [ ] 集成：新增 `tests/integration/llvm_aot_smoke.rs`（feature gate），跑 5 个程序片段并断言结果/输出（输出可通过重定向 stdout 实现）。
- [ ] 负向：遇到 `MakeClosure/CallVirt/...` 返回明确 “未实现” 错误。

---

## 阶段 5：机器码产物与执行（AOT 闭环）（3-6 天）

### 5.1 产物格式：Object + 元数据（两文件，先简单）

**目标**
- `CompiledArtifact`（Rust 侧结构）至少包含：
  - `object_bytes: Vec<u8>`（COFF/ELF/Mach-O）
  - `dag_meta: DAGMetadata`（先可为空）
  - `entries: Vec<EntryPoint>`（至少 main）
  - `type_info: TypeInfo`（MVP 先空）
- 输出策略：
  - `yaoxiang build-aot input.yx -o out/` 生成 `program.obj` + `program.dag.ron`（或 `.json`）
  - `yaoxiang run --backend llvm` 默认走“内存编译+直接执行”（不落盘），便于开发。

**验收标准**
- [ ] build-aot 可生成两个文件，且元数据可反序列化。
- [ ] run/llvm 路径不依赖落盘文件也能执行。

**测试项目**
- [ ] 文件生成测试：校验 `.obj` 非空、`.dag.ron` 可解析且版本号匹配。
- [ ] 兼容测试：不同 build_mode（Debug/Release）输出不同优化等级（至少能区分）。

---

### 5.2 执行方式：先“内存执行”，再“落盘加载”（分两步验收）

**目标**
- Step A（先交付）：使用 LLVM ExecutionEngine（或 ORC JIT）执行已生成 module（用于验证语义闭环，开发效率最高）。
- Step B（符合 AOT）：使用 TargetMachine 生成 object bytes，并通过“动态库链接/加载”路径执行：
  - 将 object 链接为 `.dll/.so/.dylib`（调用系统链接器或 lld；作为 `llvm-aot` feature 的额外要求）
  - 用 `libloading` 加载符号并调用入口函数

**验收标准**
- [ ] Step A：`--backend llvm` 能在同进程内执行（不依赖外部链接器）。
- [ ] Step B：`build-aot` 生成的产物可被 `run-aot`（新增子命令或内部路径）加载执行。

**测试项目**
- [ ] Step A：单元/集成测试默认跑（开发快）。
- [ ] Step B：标记为“需要外部链接器”的可选集成测试（CI 有环境时启用；本地可手动）。

---

## 阶段 6：DAG 元数据生成（4-7 天）

### 6.1 定义 `DAGMetadata`（版本化）

**目标**
- `dag.rs` 定义：
  - `dag_version: u32`
  - `nodes: Vec<DAGNode>`（携带 `node_id` 与 `span_id`，用于错误传播）
  - `edges: Vec<DAGEdge>`（带边类型：Data/Control/Spawn）
- `DAGEdge` 至少包含：
  - `from: u32`
  - `to: u32`
  - `kind: EdgeKind { Data, Control, Spawn }`
- 冲突/调度规则（RFC-001）：
  - DataEdge + DataEdge：可并行（若无其它依赖）
  - 任意包含 ControlEdge 的组合：必须串行化（保持顺序）
- `DAGNode` 至少包含：
  - `node_id: u32`（函数内唯一）
  - `ip: u32`（call 指令位置）
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }`（从注解或默认策略而来）
  - `span_id: u32`（定位与错误图）
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }`（由类型系统推导；`LocalOnly` 节点禁止跨线程调度）
- 约定：节点只描述“可被调度的调用点”，参数在运行时 `yx_rt_lazy_call` 时捕获（避免静态表达式求值复杂度）。

**验收标准**
- [ ] `DAGMetadata` 可序列化/反序列化（使用现有 `ron` 或 `serde_json`）。
- [ ] `dag_version` 不匹配时加载报错。

**测试项目**
- [ ] 序列化 roundtrip 单测。
- [ ] 版本不匹配单测（手工构造旧版本）。
- [ ] `thread_safety` 推导测试：至少覆盖 1 个 `LocalOnly` 场景，并在 `num_workers>1` 下验证不会跨线程执行。

---

### 6.2 资源类型与 ControlEdge 生成（副作用抽象最小可用）

**目标**
> **更新**：按 RFC-001，副作用不通过额外 effect 系统表达，而是通过**资源类型（Resource）**抽象为资源操作，并生成 ControlEdge。

- 资源操作识别（MVP）：
  - 若调用的任一参数类型为 `Resource` 或其派生资源类型（如 `Console/FilePath/HttpUrl/DBUrl`），则该调用点为资源操作；
  - 标准库的资源操作函数必须具备可识别的类型约束（推荐做法：在 std 导出签名中显式携带资源类型；或在 FFI registry 的导出元数据中标记“资源操作”并关联资源入参位点）。
- ControlEdge 生成（MVP）：
  - 对**同一资源值/句柄（同一 SSA 值/同一常量驻留键）**的多次资源操作，按程序顺序添加 ControlEdge（自动串行）。
  - 无法判定是否同一资源（别名/复杂来源）时，默认保守串行（未来可引入显式 unsafe 并行提示作为扩展）。
  - **资源标识沿数据流传播（RFC-001）**：资源冲突检测以“值相等/同一来源”为准，而不是“资源类型相同”为准（两个不同 `FilePath` 值可并行；同一 `FilePath` 值必须串行）。

**验收标准**
- [ ] 对示例：`log → save → log` 因 Console/FilePath 资源形成 ControlEdge，稳定串行；不同资源操作可并行。
- [ ] 资源操作识别稳定（同一输入 module 多次结果一致）。

**测试项目**
- [ ] 单元：资源类型参数识别测试（Resource 参数存在时必须生成 ControlEdge）。
- [ ] 单元：同一资源值（同一变量/同一常量）上的两次资源操作必须生成 ControlEdge；不同资源值（不同变量/不同常量）可不生成。
- [ ] 集成：运行包含多次 `std.io.println/std.io.write_file` 的示例，断言输出/写入顺序与解释器一致。

---

### 6.3 L1 自动回退（小函数降级为 @block，避免调度开销）

> **来源**：RFC-001 5.2（L1 自动回退）。  
> **目的**：在不改变语义的前提下，减少小函数的 DAG/调度器开销（尤其是解释器后端与 AOT 后端的统一行为）。

**目标**
- 在编译期对函数做轻量级阈值判定，满足任一条件即将该函数（或该函数内某些块）默认策略降级为 `Serial`：
  - 指令数 `< 50`
  - DAG 节点数 `< 10`
- 通过 CLI/配置暴露开关（MVP：仅内部配置也可）：
  - `--l1-threshold=<N>` 调整阈值
  - `--no-l1-fallback` 禁用自动回退

**验收标准**
- [ ] 小函数在 `@auto` 下实际运行不进入 DAG/调度队列（可通过统计字段或 trace 验证），结果与未回退一致。
- [ ] 大函数不受影响；强制注解 `@eager/@block` 优先级高于自动回退。

**测试项目**
- [ ] 单元：构造边界值（49/50 指令、9/10 节点）验证是否触发回退。
- [ ] 回归：对同一源码开启/关闭回退，输出与返回值一致。

---

## 阶段 7：运行时 DAG 调度器（Lazy Scheduling 核心）（6-10 天）

### 7.1 实现任务模型（与 `RtValue::Async` 对接）

**目标**
- `scheduler.rs`（或迁移到 `src/backends/runtime/` 以实现“解释器/LLVM 共享”）实现：
  - `TaskId` 分配
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`：创建任务但可延迟启动（用于错误传播/错误图）
  - `force(task_id)`：按依赖拓扑触发执行并等待结果

**验收标准**
- [ ] `yx_rt_lazy_call` 返回 `Async(TaskId)` 且 task 被记录（不立即执行）。
- [ ] `yx_rt_force` 能触发任务执行并返回结果（包含依赖链）。

**测试项目**
- [ ] 纯 Rust：用 mock 的 “compiled fn” 指针（实际是 Rust `extern "C"` 函数）构造 3 节点 DAG，验证依赖顺序与结果正确。
- [ ] 错误传播：下游 force 得到 Error，且不会死锁。

---

### 7.2 调度策略落地（Serial/Eager/Lazy）

**目标**
- `Serial`（对应 `@block`）：不创建 Async；call 立即执行；调度器接口可绕过。
- `Eager`：创建任务但立刻 `force`（保证顺序），用于调试/语义对齐。
- `Lazy`（默认 `@auto`）：仅在需要值时 `force`；调度器可在后台窗口内提前启动“已就绪”任务（受并发数限制）。
- 自底向上（RFC-001/008）：运行时行为应体现“从需要结果的地方反向触发求值”的特性；**未被消费且不涉及资源操作（无 ControlEdge）的分支/孤岛 DAG 不应执行**，以降低开销；资源操作必须按 ControlEdge 保证顺序并完成（与解释器一致）。
- 后台 DAG（RFC-018）：当同一作用域内存在多个长期运行/无限循环任务时，调度器需提供**合作式切片**（例如基于 budget 或显式 `yield_now`），避免主 DAG 饥饿与“卡死在循环里”。

**验收标准**
- [ ] 同一程序在 Serial/Eager/Lazy 三种策略下结果一致。
- [ ] Lazy 下当某 call 结果从未被 force/使用，任务不会执行（Lazy Task Creation）。

**测试项目**
- [ ] 对比测试：三种策略输出一致。
- [ ] Lazy 跳过测试：写一个“计算但不使用”的分支/变量，断言对应 task 执行计数为 0（调度器统计字段）。
- [ ] 后台切片测试：构造 2 个长期运行任务 + 1 个主任务，断言在时间窗内三者均有进展（可用计数器或 `thread_id` + 日志统计）。

---

### 7.3 并发控制与粒度控制

**目标**
- 并发上限：`max_parallelism = num_workers * 2`（RFC-018 建议）。
- 资源约束：对同一资源的操作必须按 ControlEdge 串行执行（RFC-001 资源类型规则），调度器不得打乱 ControlEdge 顺序。
- 线程安全约束（RFC-001/009）：调度器必须尊重 `DAGNode.thread_safety`：
  - `SendSync`：可跨 worker 执行（受并发上限与依赖约束影响）
  - `LocalOnly`：禁止跨线程调度/禁止被 work-stealing 窃取；必要时降级为串行（或固定在创建它的 worker 上执行）
- 自适应粒度（MVP）：当待运行 task 数远大于并发上限时，将“多个就绪且**无 ControlEdge 约束**的任务”合并批量提交（实现为同一 worker 顺序执行一批，减少调度开销）。

**验收标准**
- [ ] 大量独立、无资源约束任务（1e4）不会导致内存爆炸（任务结构 O(并发数) 或可控上界）。
- [ ] `LocalOnly` 节点在 `num_workers>1` 下不会跨线程执行（可用 `std.concurrent.thread_id` 验证）。
- [ ] 资源操作（例如 `std.io.*`）的输出/副作用顺序严格保持解释器顺序。

**测试项目**
- [ ] 压测单元：构造 10000 个 mock task，峰值内存/任务数量受控（用统计断言，不做精确内存测量也可）。
- [ ] LocalOnly 集成测试：构造包含 `LocalOnly` 节点的示例，在 `num_workers>1` 下断言其执行线程 ID 不发生变化。
- [ ] 资源顺序集成测试：多个资源操作（println/write_file）必须按源码顺序输出/落盘。

---

### 7.4 错误传播与错误图记录（RFC-001 最小闭环）

**目标**
- 定义最小 `ErrorGraph` 数据结构（可先仅用于调试/trace）：
  - 节点：`node_id + span_id + message/error_code`
  - 边：`from_node_id -> to_node_id`（表示“错误从依赖节点传播到消费者节点”）
- 记录策略（RFC-001 决议）：
  - 错误沿依赖边向上游传播，**不依赖实际执行顺序**；
  - DAG 仅在函数内构建，因此错误图也限制在函数级，避免内存爆炸。
- 与 ABI 对齐：
  - `yx_rt_lazy_call/yx_rt_native_call` 必须携带 `node_id/span_id`（已在阶段 2.3 锁定）
  - 任务失败与 `force` 返回错误时，写入 `ErrorGraph`（若 `ctx.error_graph != null`）

**验收标准**
- [ ] 依赖链底部节点失败时，顶层消费点收到错误（并能定位到失败节点的 span）。
- [ ] 在并行执行下，错误传播路径稳定可复现（与调度顺序无关）。

**测试项目**
- [ ] 单元：构造 3 节点依赖链，模拟中间节点失败，断言 ErrorGraph 边为 `leaf->mid->root`。
- [ ] 并发回归：num_workers>1 下多次运行，ErrorGraph 结构一致。

---

## 阶段 8：语法注解贯通（@block/@eager/@auto）（5-8 天）

### 8.1 前端支持注解并下传到字节码/元数据

**目标**
- 解析层：识别函数/块注解 `@block`、`@eager`；默认 `@auto`。
- 解析/类型检查：强制 `spawn` 只能出现在 `@block` 作用域内（RFC-001/008）。
- IR/Bytecode：在 `BytecodeFunction` 或额外 side-table 中携带默认策略；在 call-site 处能决定走 lazy/eager/direct。

**验收标准**
- [ ] 无注解：默认 Lazy（@auto）。
- [ ] `@block`：该作用域内不创建 Async，行为与解释器串行一致。
- [ ] `@eager`：创建任务后立即 force（结果一致且便于调试）。

**测试项目**
- [ ] 前端：解析/IR 生成包含注解的测试（AST/IR 断言）。
- [ ] 后端：同一段源码分别加注解，运行行为符合策略。

---

### 8.2 标准库：`@block` 与 `spawn` 的运行时接口（Full Runtime）

> **来源**：RFC-008（@block 由标准库提供）、RFC-001（spawn 的等待语义由标准库控制）。

**目标**
- 增加标准库运行时模块（建议路径：`std.runtime` 或 `std.full`），提供：
  - `block`: 强制急切求值（等价于将作用域策略设为 `Serial`/不入 DAG 队列）
  - `spawn`/`join_all`（或隐式 join）：在 `@block` 作用域内创建并发任务并等待完成
- 编译器可先内置实现 MVP，但必须抽象出可下放到标准库的接口（避免未来重构成本）。

**验收标准**
- [ ] `@block` 函数内的 `spawn { ... }` 块可并发执行，并在作用域结束前完成（不会“静默后台泄漏任务”）。
- [ ] `@block` 的行为与 L3 默认并发行为可明确区分（例如：是否进入 DAG 队列、是否产生 Async 值）。

**测试项目**
- [ ] 集成：两个 `spawn { std.concurrent.sleep(50) }` 的示例在多 worker 下耗时接近单次 sleep（粗粒度验证并发）。
- [ ] 负向：@block 外使用 spawn 报错（与 0.3/8.1 保持一致）。

## 阶段 9：端到端与性能基准（持续推进）

### 9.1 与解释器一致性测试（语义对齐）

**目标**
- 选定一组“可覆盖指令子集”的用例：算术、分支、函数调用、native IO。
- 对同一源码分别用 interpreter 与 llvm backend 执行，比较：
  - 返回值（若有）
  - stdout 输出（需注入/重定向）
  - 错误类型（尽量对齐 `ExecutorError`）

**验收标准**
- [ ] 用例集合中，LLVM backend 与解释器结果一致。

**测试项目**
- [ ] `tests/integration/llvm_vs_interpreter.rs`（feature gate）至少 10 个用例。
- [ ] 回归：新增用例时必须同时跑两后端。

---

### 9.2 基准：解释器 vs AOT（粗粒度）

**目标**
- 在 `benches/` 增加基准：纯计算（无 IO）、大量 call 任务（并发收益）、混合 IO（顺序约束）。

**验收标准**
- [ ] AOT 在纯计算用例上显著快于解释器（不承诺具体倍数，但不能明显更慢）。
- [ ] Lazy 调度的开销可被观测与定位（输出 scheduler stats）。

**测试项目**
- [ ] criterion bench（手动/CI 可选）生成报告，记录基线。

---

## 假设与默认（未被业务需求覆盖时的选择）

- 默认 LLVM 主版本选 **17**；若团队工具链不同，统一修改 `inkwell` feature 与文档即可。
- AOT 执行路径采取“两步走”：先内存执行（开发验证），再落盘链接/加载（真正 AOT）。
- 初期 `llvm-aot` 仅承诺覆盖一套 MVP 指令子集；闭包/动态派发/异常等高级特性后续按需扩展（遇到即返回明确 “未实现” 错误）。
- DAG 依赖边**可由运行时从 args 的 Async TaskId 动态推导**；编译期 edges 字段先作为可选优化与调试校验，不阻塞 M2 交付。
  - **补充（RFC-001）**：ControlEdge 的主要来源是资源类型规则；若缺少资源信息，默认保守串行以保证正确性。
