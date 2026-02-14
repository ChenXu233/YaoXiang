---
title: RFC 016：量子原生支持与多重后端集成
---

# RFC 016: 量子原生支持与多重后端集成

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-02-13
> **最后更新**: 2026-02-13
> **目标实现周期**: 未来10个月

> **依赖**:
> - [RFC-001: 并作模型与错误处理系统](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有权模型设计](./009-ownership-model.md)
> - [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
> - [RFC-011: 泛型系统设计](./011-generic-type-system.md)

## 摘要

本文档定义 YaoXiang 语言的**量子原生支持**及**多重后端集成**方案。核心思想：**YaoXiang 的现有设计（默认 Move、所有权回流、结构子类型、DAG 调度器）天然构成量子编程语言的完整基础，无需引入任何新的量子专用语法**。我们通过添加少量内置类型（`Qubit`、`Complex`）和内置函数（量子门、测量），并利用现有语言机制，实现量子原生语义、自动并行最大化量子利用率、混合经典编程，以及多重后端支持。

## 动机

### 为什么需要量子原生支持？

当前量子编程生态存在严重割裂：

- **低级语言（QCIS、OpenQASM）**：直接操作物理量子门，但缺乏类型系统、抽象机制，难以编写复杂算法。
- **高级框架（Qiskit、Cirq、Q#）**：基于经典语言（Python、C#）扩展，量子语义通过库实现，导致：
  - 量子态不可克隆规则需用户手动遵守（或依赖线性类型系统后补）。
  - 量子门操作与经典代码语法割裂，学习成本高。
  - 自动并行优化依赖外部编译器，难以与经典控制流结合。
- **混合计算**：量子与经典部分需显式分离，缺乏统一数据流模型。

### 当前问题

YaoXiang 的已有设计恰好为解决这些问题提供了完备基础：

| 量子计算需求 | YaoXiang 已有设计 | 说明 |
|-------------|-------------------|------|
| 量子态不可克隆 | **默认 Move 语义** | 赋值即移动所有权，无隐式复制，天然符合 no-cloning 定理 |
| 量子门作为酉变换 | **所有权回流** | `q = H(q)` 消费原 qubit，返回新 qubit，精确对应门语义 |
| 纠缠态 | **结构子类型** | `{ Qubit, Qubit }` 自动表示纠缠对，无需特殊张量积语法 |
| 测量坍缩 | **空状态重用** | 测量后 qubit 变空，可重新初始化，模拟量子态坍缩 |
| 量子线路自动并行 | **DAG 调度器** | 函数内语句根据数据依赖自动并行，无依赖的门天然并发 |
| 混合经典-量子控制流 | **统一语法** | 量子操作与经典操作使用相同 `name: type = value` 形式 |

**YaoXiang 不是"添加量子支持"，而是发现自己的设计已经量子原生。**

### 设计目标

1. **零新语法**：不引入 `quantum`、`circuit` 等关键字，所有量子特性通过现有语言机制表达。
2. **类型安全**：编译器保证量子态不被复制、不被不合法使用。
3. **自动并行最大化利用率**：DAG 调度器自动分析量子门依赖，在量子硬件上实现门级并行，无需用户手动标注。
4. **多重后端透明**：同一份量子代码可编译到 QIR（通用生态）或 QCIS（国产量子指令集），通过命令行参数切换。
5. **混合经典无缝**：量子计算可自由调用经典函数，经典代码也可操作量子数据（通过 `ref` 共享，但受所有权约束）。

## 提案

### 核心设计

#### 1. 量子类型系统映射

**基础类型**：
```yaoxiang
Qubit: Type0 = primitive_qubit
Complex: Type0 = { re: Float, im: Float }
```
- `Qubit` 是一等公民类型，遵循所有权规则（Move、RAII）。
- `Complex` 用于表示振幅，编译器可内联优化。

**量子门作为函数**：
```yaoxiang
# 内置函数签名
H: (Qubit) -> Qubit = builtin_hadamard
X: (Qubit) -> Qubit = builtin_pauli_x
Y: (Qubit) -> Qubit = builtin_pauli_y
Z: (Qubit) -> Qubit = builtin_pauli_z
CNOT: (control: Qubit, target: Qubit) -> { Qubit, Qubit } = builtin_cnot
```
- 所有门消费输入 qubit，返回新 qubit（或纠缠对）。所有权回流语法 `q = H(q)` 直接对应数学语义。
- 多 qubit 门返回结构体，通过模式匹配或字段访问获取结果。

**测量**：
```yaoxiang
measure: (Qubit) -> Int = builtin_measure   # 消费 qubit，返回经典比特
measure_all: (List[Qubit]) -> List[Int] = builtin_measure_all
```
- 测量后 qubit 被消费（变空），用户可通过空状态重用重新初始化。

**初始化**：
```yaoxiang
qubit: (Int) -> Qubit = builtin_qubit   # 0 或 1 初始化基态
```

#### 2. 纠缠与结构子类型

纠缠态直接用结构体表示：
```yaoxiang
bell_pair: { Qubit, Qubit } = CNOT(H(qubit(0)), qubit(0))
```
- 结构体字段顺序不重要，编译器可识别为两个 qubit 的联合态。
- 通过字段访问获取单个 qubit 继续操作。

**所有权与量子语义的边界**

`{Qubit, Qubit}` 结构体提供语法操作能力，但**不保证量子后端层面这两个比特仍保持纠缠**。

用户必须遵守量子计算的正确使用顺序：
1. 制备纠缠
2. 对各比特执行操作/测量

在纠缠对上执行中间操作（如 `bell.q2 = H(bell.q2)`）会破坏纠缠，这是量子知识问题，不是编译器能检查的。

> **设计原则**：编译器只保证"不会复制 qubit"（所有权安全），不保证"正确使用量子语义"。
> 这与 Rust 的设计哲学一致——Rust 保证内存安全，但不保证多线程代码没有 data race。

### 量子语义安全保障：编译期能抓 90%

YaoXiang 的现有类型系统 + DAG 调度 + 所有权模型，已经能捕获大部分常见量子错误：

| 错误类型 | 捕获机制 |
|---------|---------|
| 复制 qubit | 所有权系统（Move 语义） |
| 测量后再次使用 | 空状态重用 + 数据流分析 |
| 门操作顺序非法 | DAG 调度器保证依赖正确 |
| 纠缠对误拆解 | **不透明类型**封装（推荐方案） |

#### 纠缠对的不透明类型封装

将纠缠对封装为不透明类型，只提供组合操作，禁止拆解：

```yaoxiang
# 内置不透明类型
BellPair: Type0 = primitive_bell_pair

# 内置函数 - 只能整体操作
CNOT: (Qubit, Qubit) -> BellPair
measure_bell: (BellPair) -> { Int, Int }
apply_cnot_to_bell: (BellPair, Qubit) -> BellPair
```

**关键设计**：
- 不提供字段访问器，只允许通过内置函数整体操作
- `measure_bell(bp)` 一次性消费整个纠缠对，返回经典比特
- 编译器能追踪纠缠对的完整生命周期

**与 Python/Qiskit 的对比**：
```
Python (Qiskit): 运行时构建电路，错误可能在提交后才发现
YaoXiang:       编译期捕获大部分逻辑错误
```

**剩余 10%**（如物理退相干、门误差）是硬件问题，不是语言能解决的。

#### 3. 所有权与量子态的线性流动

所有量子操作均遵循 Move 语义，确保 qubit 不被复制：
```yaoxiang
q = qubit(0)
q2 = q          # ❌ 编译错误：q 已移动，不可再使用
q = H(q)        # ✅ 消费 q，返回新 q
measure(q)      # ✅ 消费 q，之后 q 变空
q = qubit(0)    # ✅ 空状态重用
```

#### 4. 自动并行与 DAG 调度

在 Standard 或 Full Runtime 下，DAG 调度器自动分析量子程序：
```yaoxiang
apply_two_qubit_gates: () -> {Qubit, Qubit} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    # 以上两行无数据依赖，DAG 自动并行执行
    CNOT(q1, q2)   # 依赖 q1 和 q2，自动等待
}
```
- 调度器利用 `num_workers` 配置（物理量子处理器数量）实现真正并行。
- 用户无需手动安排门顺序，只需描述数据流。

#### 5. 混合经典计算

经典与量子代码完全融合：
```yaoxiang
grover_search: (target: Int) -> Int = () => {
    n = 4
    qubits = List[Qubit]()
    for i in 0..n {
        qubits.append(H(qubit(0)))
    }
    # 经典循环与量子操作混合
    oracle(qubits, target)   # oracle 是量子门序列
    qubits = diffusion(qubits)
    results = measure_all(qubits)
    return decode_result(results)   # 经典后处理
}
```
- 同一函数内可任意混合量子门和经典控制流。
- 所有权系统确保量子变量不会在经典分支中被错误复制。

#### 6. 多重后端支持架构

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   YaoXiang 源   │     │   类型检查      │     │   DAG 中间表示  │
│   (统一语法)     │────▶│   + 所有权分析   │────▶│   (数据流图)    │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
                          ┌─────────────────────────────────────────────┐
                          │           代码生成后端 (可插拔)              │
                          ├─────────────────┬───────────────────────────┤
                          │  QIR 后端       │  QCIS 后端                │
                          │  (通用生态)      │  (国产量子指令集)         │
                          ├─────────────────┼───────────────────────────┤
                          │  - 输出 .ll 文件 │  - 输出 .qcis 文本        │
                          │  - 适配多种 QPU  │  - 适配中科院/国盾硬件    │
                          └─────────────────┴───────────────────────────┘
```

- **编译流程**：前端统一 → DAG 构建 → 后端选择 → 目标代码生成。
- **QIR 后端**：将 DAG 节点映射到 QIR 的量子门 intrinsic，生成 LLVM bitcode，可进一步利用 LLVM 优化。
- **QCIS 后端**：将 DAG 序列化为 QCIS 指令（如 `H q0`），支持直接提交到量子芯片控制台。

#### 7. 彩蛋的量子映射

当编译器遇到 `Type: Type = Type` 时，输出特殊信息。在量子后端的上下文中，可额外生成一段校准序列，例如对所有可用 qubit 同时施加 H 门并测量，将二进制结果解释为 ASCII 字符输出哲学语句。此功能作为可选彩蛋，不影响常规编译。

### 示例

#### 贝尔态制备与测量
```yaoxiang
bell_measure: () -> {Int, Int} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    bell = CNOT(q1, q2)
    m1 = measure(bell.q1)
    m2 = measure(bell.q2)
    return {m1, m2}
}
```

#### 量子隐形传态（简化）
```yaoxiang
teleport: (msg: Qubit, bell: {Qubit, Qubit}) -> Qubit = (msg, bell) => {
    # Alice 操作
    msg, bell.q1 = CNOT(msg, bell.q1)
    msg = H(msg)
    a1 = measure(msg)
    a2 = measure(bell.q1)

    # 经典信息传递 (由调度器自动处理依赖)
    # Bob 操作
    q3 = bell.q2
    if a2 == 1 { q3 = X(q3) }
    if a1 == 1 { q3 = Z(q3) }
    return q3
}
```

## 详细设计

### 内置类型与函数定义

在 `compiler/builtins` 模块中增加：
```rust
builtins.insert("Qubit", Ty::Primitive(Primitive::Qubit));
builtins.insert("Complex", Ty::Record(vec![
    ("re", Ty::Primitive(Primitive::Float)),
    ("im", Ty::Primitive(Primitive::Float)),
]));

// 量子门
for (name, sig) in GATES {
    builtins.insert(name, Ty::Function(vec![Ty::Qubit], Ty::Qubit));
}
builtins.insert("CNOT", Ty::Function(
    vec![Ty::Qubit, Ty::Qubit],
    Ty::Record(vec![
        ("q0", Ty::Qubit),
        ("q1", Ty::Qubit)
    ])
));
builtins.insert("measure", Ty::Function(vec![Ty::Qubit], Ty::Primitive(Primitive::Int)));
builtins.insert("qubit", Ty::Function(vec![Ty::Primitive(Primitive::Int)], Ty::Qubit));
```

### 所有权检查器对 Qubit 的特殊处理

- `Qubit` 被标记为 `!Copy`（默认 Move），禁止隐式复制。
- 测量函数 `measure` 的参数为 `Qubit`（按值传递），消费所有权。
- 多 qubit 门返回的记录类型中，字段均为 `Qubit`，仍需遵守所有权规则。

### DAG 调度器对量子门的优化

- 量子门节点被视为纯函数（无副作用），调度器可任意重排无依赖的门。
- 调度器输出"量子指令序列"时，保留数据依赖，并将并行门分组（适用于多量子处理器）。
- 支持配置 `--target-num-qubits` 和 `--target-topology`，用于后续布局和路由（未来扩展）。

### QIR 后端详细映射

| YaoXiang 操作 | QIR 指令 |
|---------------|----------|
| `H(q)` | `call void @__quantum__qis__h__body(%Qubit* %q)` |
| `CNOT(q1, q2)` | `call void @__quantum__qis__cnot__body(%Qubit* %q1, %Qubit* %q2)` |
| `measure(q)` | `%result = call i1 @__quantum__qis__mz__body(%Qubit* %q)` |
| `qubit(0)` | `%q = call %Qubit* @__quantum__rt__qubit_allocate()` |

QIR 后端利用 LLVM 的 `-O2` 进一步优化，并输出与 QIR Alliance 兼容的 bitcode。

### QCIS 后端详细映射

| YaoXiang 操作 | QCIS 指令 |
|---------------|-----------|
| `H(q)` (q 对应物理比特 2) | `H 2` |
| `CNOT(q1,q2)` (q1→比特 0, q2→比特 1) | `CNOT 0 1` |
| `measure(q)` (比特 0) | `M 0` |
| `qubit(0)` 初始化 | 隐含在第一条使用指令中，无需额外指令 |

- 需维护从虚拟 qubit（YaoXiang 变量）到物理比特的映射表。
- 支持拓扑约束检查（未来实现）。

### 混合经典代码生成

- 经典部分（如循环、条件、整数运算）照常生成本地代码（x86/ARM），通过 FFI 或嵌入式调用与量子后端交互。
- 在 QIR 后端中，经典部分可降级为 LLVM IR，与 QIR 混合编译。

### 类型系统影响

- 新增 `Qubit` 和 `Complex` 原始类型。
- `Qubit` 自动具有 Move 语义，禁止复制。
- 量子门函数签名需在类型系统中注册。

### 向后兼容性

- ✅ 完全向后兼容
- 新增内置类型和函数，不影响现有代码
- 量子特性是可选的，不启用时无额外开销

## 权衡

### 优点

- **无新语法**：开发者只需学习少数内置函数，即可编写量子程序。
- **类型安全**：所有权系统自动防止 qubit 复制，避免常见量子编程错误。
- **自动并行**：DAG 调度器免费提供门级并行，无需额外编译器优化。
- **生态兼容**：QIR 后端使 YaoXiang 能运行在多家量子云平台上；QCIS 后端保障自主可控。
- **混合能力**：经典量子融合自然，适合编写复杂量子算法（如 Shor、Grover 中的经典控制）。

### 缺点

- **Qubit 数量静态**：当前设计假定 qubit 数量在编译时已知，动态分配需通过 `List[Qubit]`，但 `List` 的堆分配可能引入额外开销（可通过优化缓解）。
- **拓扑约束**：未内置量子芯片的连接图约束，初期用户需手动确保门操作符合物理拓扑（未来可增加布局与路由 pass）。
- **测量后重用**：空状态重用允许重新初始化 qubit，但物理量子比特可能存在弛豫时间，需运行时系统处理（目前由用户负责）。

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 引入 `quantum` 关键字和 `circuit` 类型 | 增加新语法，学习成本高，违背 YaoXiang 简洁设计原则 |
| 仅作为库实现量子支持 | 无法利用编译器保证量子态安全，无法与 DAG 调度器深度集成 |
| 等待量子硬件成熟再支持 | 错过量子编程语言设计的关键窗口期 |
| 复用现有量子框架（如 Qiskit） | 量子语义通过库实现，无法获得类型系统和所有权系统的安全保障 |
| 单独设计量子子语言 | 增加语言复杂度，维护成本高 |

## 实现策略

### 阶段划分

| 阶段 | 时间 | 内容 |
|------|------|------|
| Phase 1 | 1个月 | 基础量子类型与内置函数：在编译器中添加 `Qubit`、`Complex` 类型，实现内置函数的类型检查，扩展所有权检查器 |
| Phase 2 | 1个月 | DAG 调度器识别量子门：修改 DAG 构建逻辑，标记量子门为纯函数，实现并行门分组输出 |
| Phase 3 | 2个月 | QIR 后端原型：实现 DAG 到 QIR 代码生成器，集成 LLVM，连接 QIR 模拟器验证 |
| Phase 4 | 2个月 | QCIS 后端原型：实现 DAG 到 QCIS 指令翻译，设计虚拟-物理比特映射，连接国产量子平台验证 |
| Phase 5 | 2个月 | 混合经典增强：确保经典控制流与量子门交叉正确生成代码，支持 `List[Qubit]`，增加示例程序 |
| Phase 6 | 2个月 | 优化与文档：实现基本布局与路由，编写用户指南和量子编程教程，发布预览版 |

### 风险

1. **量子硬件可用性**：依赖外部量子模拟器和真实 QPU 的可用性。
   - **缓解**：优先对接开源模拟器（QIR runner、Qiskit Aer），真实 QPU 作为长期目标。

2. **后端实现的复杂性**：QIR 和 QCIS 规范可能发生变化。
   - **缓解**：抽象代码生成接口，隔离后端差异，便于后续适配。

3. **性能不确定性**：量子程序的性能特征与经典程序不同。
   - **缓解**：提供性能剖析工具，让用户了解门级并行效果。

## 开放问题

- [ ] **拓扑约束**：是否需要在语言层支持量子芯片的耦合图？初期可让用户手动指定映射，未来增加自动布局 pass。
- [ ] **动态量子寄存器**：`List[Qubit]` 在 QCIS 后端如何映射？可生成对应数量的物理比特，但需运行时分配机制。
- [ ] **错误缓解**：是否提供内置的错误缓解（如动态退耦）构造？可先作为库实现。
- [ ] **与现有量子 SDK 互操作**：能否导入 QASM 或 QIR 模块？未来可考虑 FFI。

## 参考文献

- [QIR Specification](https://github.com/qir-alliance/qir-spec)
- [QCIS: A Quantum Control Instruction Set](https://arxiv.org/abs/2005.12534) (中科大/国盾)
- [Rust Quantum Computing Examples](https://github.com/Rust-GPU/rust-gpu)
- [Qunity: A Unified Language for Quantum and Classical Computing](https://qunity-lang.org) (2025)

---

## 生命周期与归宿

```
┌─────────────┐
│   草案      │  ← 作者创建
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 社区讨论
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```

### 状态说明

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/draft/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档，进入实现阶段 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录，更新状态 |
