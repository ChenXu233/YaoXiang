---
title: 'LLVM AOT 后端接入执行计划'
description:
  '面向编译器贡献者：从 Linux 原生样例到双后端等价验收，明确共享 IR 前置条件、运行时边界和发布条件。'
---

# LLVM AOT 后端接入执行计划

**第一项可交付成果**：在 Linux
x86-64 上，把一份真实的 YaoXiang 源程序编译成 ELF 可执行文件。在没有 LLVM 和 YaoXiang
VM 的同类系统上运行它，既能算对结果，也能在出错时指出源码位置。生成一份 LLVM IR 文本不算完成。

**状态：批次 3 已开工（值证据、无 `Phi` CFG 子集与多文件项目入口），批次 4.1（运行时 ABI 首版与 `String` 判等/顺序）、4.2（`Int` 位运算与受检移位）与 4.4（模块级全局）与 4.5（`Float` 标量）已入库，发布验收未完成。**
[RFC-018](../../rfc/accepted/018-llvm-aot-compiler.md)
已接受 LLVM 提前编译（AOT）的方向，并在附录 B 登记构建产物契约；[跟踪议题 #134](https://github.com/ChenXu233/YaoXiang/issues/134)
是这项工作的入口。首批实现不占用 [RFC-039 施工表](../architecture/09-execution-wbs.md)
的编号。[RFC-039](../../rfc/accepted/039-compiler-architecture.md)
将后端选型与编译器重构分开：原型不需要等整个重构结束，正式代码生成也不能接管 P7 的 SSA 改造。

## 看板

勾选表示该条的交付物已入库可查；未勾选表示未开工或未达到放行条件。状态只记仓库内可解析的事实：实现提交短号、测试名与文件路径。各批次的判据正文见下文，本节不重复正文。

### 施工顺序

- [x] **批次 1 固定构建环境并隔离 LLVM 依赖**（`c7dcf603`、`abeaca98`）——可选 Cargo feature `llvm`
      与 Inkwell 0.11.0、LLVM 18.1 的固定组合入库；CI 组合动作
      `.github/actions/setup-llvm/action.yml` 安装 `llvm-18-dev` 并设置 `LLVM_SYS_181_PREFIX` 与
      `LLVM_SYS_181_STRICT_VERSIONING`；`check-no-llvm`
      作业验证默认通道不依赖 LLVM、原生请求不回退字节码；批次构建工具 `scripts/dev/scratch.py` 的
      `build` 支持
      `--features llvm`。缺链接器有独立用例（`test_native_missing_linker_preserves_output`）；LLVM 主版本不匹配由
      `LLVM_SYS_181_STRICT_VERSIONING` 在构建期拒绝。
- [x] **批次 2 交付第一个真实源程序**（`c7dcf603`）——AOT-0/AOT-1 原型入库（`src/backends/llvm/`）：`yx build --emit exe`
      走原生路径，`--emit bytecode` 走字节码路径，裸 `build` 仍默认生成 `.42`。判据在
      `tests/integration/native_aot.rs`。原型限单文件直线标量子集，发布验收未完成。
- [ ] **批次 3 接正式共享 IR，扩到控制流与多文件**——3.1/3.2/3.3/3.4 已收口（`151eaf3f`、`56e9da77`、`850841b2`、`3d389358`、`9a86e1d2`、`2f633910`、`1e27af37`、`e2501c3d`）：Linux 构建恢复、AOT 入口接到
      `verify_ssa`；单文件的无 `Phi`
      分支与循环已发射（break/continue、嵌套 if/else、嵌套 while、循环内 `Local`、跨块 `Value`
      均与 VM 同结果）；多文件项目入口与文件身份已落地（B-i：库 API 项目入口，CLI 单入口文件）；`Phi`
      与多前驱 `Local`
      汇合的拒绝边界已由批次 3.5 收口为发射（见“支持集与拒绝边界”）。等价账本的八条最小集已有可执行门禁（机器可读账本 + 检查器 +
      CI 作业，见“等价账本（最小集，八条）”）。判据在 `tests/integration/native_aot.rs` 与
      `src/backends/llvm/tests/analysis.rs`。
- [x] **批次 3.5 `Phi` 与 `Local` 汇合映射**（`17882acb`、`5e26ea2b`）——需要 `Phi`
      的汇合值从能力拒绝改为发射：`Phi` 在块首预建、incoming 按前驱 block label 登记；`Local`
      的 alloca 统一提到入口块，多前驱写—汇合读不再拒绝；`5e26ea2b`
      把显式未知 incoming 收紧为按可观察性分派——死合流放行（`undef`
      不可观察），live 合并值带位置拒绝（见“支持集与拒绝
      边界”）；账本 phi-boundary /
      local-merge-boundary 改 supported（见“支持集与拒绝边界”与“等价账本（最小集，八条）”）；判据在
      `tests/integration/native_aot.rs` 与 `src/backends/llvm/tests/analysis.rs`。
- [ ] **批次 4 链接原生运行时，再接 FFI 与并作**——4.1 已收口：运行时 ABI 首版与
      `String` 判等/顺序（见“运行时 ABI 约定（首版）”）；判据
      `test_native_string_equality_matches_vm`、`test_native_string_ordering_matches_vm`、
      `test_native_string_phi_merge_compares_by_content`；4.2 已收口：`Int` 位运算与受检
      移位（越界移位报 E6007 且与 VM 同源码位置，不静默 wrap，判据
      `test_native_shift_out_of_range_matches_vm_diagnostic`）；右移语义按有符号定义：
      `>>` 降 `Sar`（发 `ashr`）、`Shr` 归位逻辑右移（发 `lshr`，前端不产出，判据
      `test_native_shr_instruction_uses_logical_shift`）；4.4 已收口：模块级 `Int`/`Bool`/`String`
      全局（声明、module init 写入、函数内读写的同一套类型规则，判据
      `test_native_global_scalars_match_vm` 与 `test_native_project_global_mutation_matches_vm`）；
      4.5 已收口：`Float` 标量（四则/`Neg`/floor 取模/六比较/局部/形参/返回/`Phi`，混型带位置
      拒绝，判据 `test_native_float_arithmetic_matches_vm`、
      `test_native_float_nan_and_infinity_match_vm`、
      `test_native_float_mixed_int_operands_are_rejected`）。容器、闭包、资源、FFI 与并作未开始。

  **批次 4 前置（阻塞，2026-11-28 登记）**：

  - [ ] 调用点结果类型未进入 L3（issue [#593](https://github.com/ChenXu233/YaoXiang/issues/593)）——阻塞 std/`println`、字符串比较参与比较；下游 S3/S4/S5。
  - [ ] 闭包实例化类型未收敛（issue [#590](https://github.com/ChenXu233/YaoXiang/issues/590)）——阻塞闭包/Fn 值；下游 S6。
  - [ ] SSA 临时值释放事实缺失（issue [#591](https://github.com/ChenXu233/YaoXiang/issues/591)）——阻塞所有权/`Drop`/RC、字符串所有权与容器；下游 S3/S7。

### 平台验收

- [ ] **Linux ELF（`x86_64-unknown-linux-gnu`）**——原型能产出 ELF（`src/backends/llvm/mod.rs` 的
      `TARGET_TRIPLE`）；正式支持未发布，发布门槛按 RFC-018 逐项对账未完成。
- [ ] **Windows x64 MSVC COFF（`x86_64-pc-windows-msvc`）**——未开始；可选 feature 目前只覆盖 Linux
      x86-64。
- [ ] **macOS Mach-O（`x86_64-apple-darwin` 与 `aarch64-apple-darwin`）**——未开始。

### 前置

- [x] **IR 值契约（P7 的 SSA 生产端）**（`b503138b`、`ba482c8d`、`bdda6d53`、`aeb4cbea`）——生产端已做 SSA 构造：按支配边界在汇合块首插入
      `Phi`、按支配关系重命名、`ValueInfo.defining_block` 记真实定义块；`verify_ssa` 覆盖
      `Operand::Value` 定义唯一性、值使用受定义支配、`defining_block` 一致与 `Phi` 输入类型；门禁
      `scripts/ci/check-ir-verifier.sh` 同时跑 `verify_loose`、生产语料 `verify_ssa` 与负例自证。
- [x] **7b SSA 同步**（`151eaf3f`）——IR 已按 7b.1a 收敛：`src/middle/ir/ir.rs` 的 `Operand`
      只余 Const、Value、Local、Arg、Global 五个变体；`src/backends/llvm/analysis.rs` 与
      `src/backends/llvm/codegen.rs` 不再引用 `Operand::Temp`，AOT 入口调用
      `verify_ssa`。批次 3 的前置在 Linux+LLVM 18.1.3 通道实测满足。
- [x] **Linux + LLVM 18.1 验证通道**（`c7dcf603`、`abeaca98`）——与批次 1 同批入库：CI 固定 LLVM
      18.1 工具链、无 LLVM 通道隔离、批次树构建的 `--features llvm` 开关。

## 接在哪里

下表是开工时要复核的事实。源码链接固定在同一
[Git 修订](https://github.com/ChenXu233/YaoXiang/commit/e64139dc3d10fb6e376ac6980a04a2ef2e296432)，不是对未来目录布局的要求。

| 现有边界                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | 对施工的约束                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Driver 阶段表](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/driver/program.rs)：单文件在 IR 生成后做单态化；多文件先合并 IR，再处理全程序的实例化请求。                                                                                                                                                                                                                                                                                                                 | 在编译成功、义务结算后取最终 `ModuleIR`。项目走既有多文件编排，不另起解析或类型检查流水线。前端失败时不调用 LLVM。                                                                                                                                              |
| [构建命令](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/main.rs) 的默认输出是 `.42`；[字节码生成器](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/passes/codegen/mod.rs) 消费模块 IR；[`Executor`](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/backends/mod.rs) 消费字节码并执行。                                                                                          | 让字节码与 LLVM 在同一份模块 IR 处分流。LLVM 不从 `.42` 反推类型，不实现字节码执行接口。                                                                                                                                                                        |
| [槽位生成](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/ir_gen.rs) 用 `Int(64)` 补齐临时槽；[IR 校验器](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/verify.rs) 按展平指令下标解释当前跳转。[取模快照](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/tests/snapshots/tests/yaoxiang/01-syntax/basics/modulo_math.snap) 中同一槽先装字符串，后装整数。 | 槽号不是单赋值的值，补位类型不是类型证明。`verify_loose` 通过不等于可以翻译任意函数。现有[快照规范化器](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/normalize.rs) 只用于打印，不是消除栈指令的编译 pass。 |
| [模块 IR](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/ir.rs) 已携带初始化序列、入口、来源文件、FFI 绑定、释放指令及并作执行计划；[运行时接口](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/backends/runtime/facade.rs) 使用 Rust 任务闭包。                                                                                                                                                                        | 共享语义继续由原模块决定。原生后端只负责表示、调用约定和发射；Rust 闭包和 VM 的 `RuntimeValue` 都不是原生程序的二进制接口（ABI）。                                                                                                                              |

建议在 L1 编排层选择产物，在 L4 的 `backends::llvm`
内管理 Inkwell、LLVM 模块、优化和目标文件。L2/L3 不引入 LLVM 类型；L4 不反查 AST 或类型检查器。不预先发明通用后端 trait。首批不做 JIT、跨平台交叉编译、裸
`build` 默认产物切换或性能承诺。

## 开工前确认什么

| 决策点           | 推荐方案及代价                                                                                                                                                                                          | 另一方案及边界                                                                                                                            |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| 工具链与首个平台 | 按 RFC-018 采用 Inkwell，先选 `x86_64-unknown-linux-gnu`。只在原生宿主启用可选 Cargo feature；固定并验证 Inkwell、其 LLVM 绑定和系统 LLVM 的兼容组合。能复用现有 Linux CI，但不能据此宣布支持其他 ABI。 | 同时铺开多目标可更早暴露平台差异，但首个错误难以区分来自语言翻译、运行时还是平台 ABI。后续逐目标验收。                                    |
| 与 P7 的关系     | 先在可判定子集上验证工具链，正式控制流翻译等待共享类型与 SSA。代价是原型明确拒绝一部分合法程序。                                                                                                        | 全部等 P7 再开始，避免临时适配，却把链接和发行风险推迟到扩面时才发现。不选在 LLVM 后端自行恢复整套类型和 SSA 的方案：那会产生第二套语义。 |

后续批次继续按依赖关系登记和验收，不把 AOT-0/AOT-1 的原生子集当作 RFC 全量能力。

### 构建产物契约

RFC-018 附录 B 登记了构建产物的终态。`--emit exe` 进入原型原生路径， `--emit bytecode` 显式生成
`.42`；裸 `build` 在发布门槛达成前继续默认生成 `.42`。不使用 `--backend llvm` 作为长期用户接口。

| 命令形态                             | 默认切换后的行为                                             |
| ------------------------------------ | ------------------------------------------------------------ |
| `yx build demo.yx`                   | 生成宿主原生可执行文件；未指定 `-o` 时使用与输入同名的产物。 |
| `yx build demo.yx --emit exe`        | 显式生成宿主原生可执行文件。                                 |
| `yx build demo.yx --emit bytecode`   | 生成 `.42`；未指定 `-o` 时把输入扩展名替换为 `.42`。         |
| `yx run demo.yx` 或 `yx run demo.42` | 保持现有 VM 行为；字节码继续按内容魔数识别。                 |

迁移分两步：

1. 首批实现 `--emit exe` 和 `--emit bytecode`，裸 `build` 暂时保持 `.42`
   默认。仓内需要字节码的测试、基准和文档改用显式
   `--emit bytecode`；VM 与 AOT 基准使用独立的产物声明和数据序列。
2. 原生后端达到目标平台的发布门槛后，裸 `build` 默认切换为
   `exe`。无 LLVM 的引擎收到原生构建请求时明确报错，不退回字节码。默认切换后，未显式给出 `--emit` 且
   `-o` 使用 `.42` 后缀的旧命令报错并提示补写 `--emit bytecode`，不生成名为 `.42` 的原生文件。

默认产物与引擎的默认 Cargo
feature 分别验收。默认切换后，官方发行引擎必须提供 LLVM；无 LLVM 的开发构建仍可使用 VM 和显式字节码构建。

### 实现与扩面前的复核

在仓库根目录运行以下只读命令。前两条定位真正的编译分流点与 IR 校验入口；后两条核对批次工具支持的构建参数和测试预算。阶段表或校验契约已改变时，先更新任务前置，不照抄本页的基准修订。

```bash
git grep -n -e 'const SINGLE_FILE_STAGES' -e 'const MULTI_FILE_STAGES' -- src/driver/program.rs
git grep -n -e 'pub fn verify_loose' -e 'pub fn verify_ssa' -- src/middle/ir/verify.rs
python scripts/dev/scratch.py build --help
python scripts/dev/testsuite.py plan
```

## 生产化路线：先闭合值契约，再交付批次

生产化指：在声明的支持集上，原生产物与 VM 的可观察行为一致（值、诊断码/文件/行、顺序、资源生命周期）；在声明的目标上可复现构建；未支持的形态明确拒绝且不产出。验收以合约与门为准，不以“某批次能跑”为准。

生产化依赖四条不变量：

- **表示充分**：IR 能表达前端接受的构造。
- **语义确定**：每个操作数的定义点、类型与存储位置由合约唯一确定，消费者不自行推断。
- **执行忠实**：原生产物与 VM 在支持集上可观察等价。
- **证据可核**：每层边界有可执行判据，生产声明附红绿证据与支持集账本。

第二条的值契约已在生产端闭合（`b503138b`、`ba482c8d`、`bdda6d53`、`aeb4cbea`；收口登记见
[施工任务表](../architecture/09-execution-wbs.md) 的 P7 7f）：生产端按支配边界在汇合块首插入 `Phi`
并按支配关系重命名，`ValueInfo.defining_block` 记真实定义块；`verify_ssa` 的定义唯一性判据挂在
`Operand::Value` 上，另覆盖值使用受定义支配、`defining_block` 一致与 `Phi` 输入类型；门禁
`scripts/ci/check-ir-verifier.sh` 同时跑 `verify_loose`、生产语料 `verify_ssa` 与负例自证。

仍在的缺口有三处，各自指向后续阶段：

- **lower 侧的类型覆盖不全**：生产语料 9586 个值中 2403 个类型显式未知（25.1%，`b503138b`
  记录），未知集中在调用、字段、索引与容器构造形态；类型证据补齐归阶段 1。
- **未定义值只能物化为显式未知的
  `Load Void`**：全函数 0 定义与部分前驱未定义两类，生产端在入口物化一条
  `Load Void`（类型显式未知），语义等价但类型证据缺失；归阶段 1。
- **`translator` 没有 `Const` 源**：`OperandResolver::to_reg` 只接受槽位类操作数（`Local` / `Arg` /
  `Value`），`Move` 的源不能是 `Operand::Const`，生产端因此不把常量内联成 SSA 值——常量以
  `Operand::Const` 留在 IR 里，原生后端单独消费该变体；原生路径消费常量的证据归阶段 1。
  AOT 不得在 L4 后端重建 SSA 或类型（见“开工前确认什么”），因此这三处缺口按所在层立项补足，不在原生后端另造一套语义。

### 阶段与依赖

| 阶段                                                                   | 内容                                                                 | 入口条件        | 放行证据                                                                               |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------- |
| 0 值契约（**已闭合**：`b503138b`、`ba482c8d`、`bdda6d53`、`aeb4cbea`） | SSA 构造、校验器补全、基线再生（见下节）                             | 7b 形态切换完成 | `verify_ssa` 在生产语料全绿；快照与语料基线同提交并人工 review；VM 与 `.42` 行为零变化 |
| 1 AOT 核心                                                             | 值/类型证据、CFG 与 `Phi` 映射、多文件与入口身份、等价账本（批次 3） | 阶段 0          | 支持集上 VM 与原生 O0/O2 同结果；拒绝边界可核                                          |
| 2 运行时                                                               | 值/容器/闭包/资源、受检算术与断言、FFI、并作（批次 4）               | 阶段 1          | 对应语料与错误码等价                                                                   |
| 3 目标与链接                                                           | Linux ELF、Windows x64 MSVC COFF、macOS（见“发布与回退”）            | 阶段 2          | 每个目标实机编译、链接、运行；产物不依赖 LLVM 与 VM                                    |
| 4 证据与发布                                                           | 支持集账本、受控变异、RFC-018 附录 B 对账、默认产物切换              | 阶段 3          | 声明与账本一致；没有实测值不作规模或性能承诺                                           |

阶段 0 已落地；批次 3.5 起 AOT 消费生产端 `Phi` 与 `Local` 合并（提交
`17882acb`、`5e26ea2b`），不再为需要汇合值的程序保留能力拒绝。阶段 1 与批次 4 的支持集与拒绝边界由下节登记；新增可编译形态必须同时给出正例和带位置的拒绝判据。

### 支持集与拒绝边界（等价账本）

阶段 1 与批次 4（4.1、4.2）的可编译形态按可执行判据登记在同一张账本里；支持集之外必须显式拒绝并带源码位置，禁止静默回退。判据分两处：集成判据在
`tests/integration/native_aot.rs`，后端单元判据在
`src/backends/llvm/tests/analysis.rs`。本节逐条列边界；等价账本的最小交付边界见节末的八条表。

**支持集**：

- 单文件脚本或多文件项目；`Int`、`Bool`、`Float`、`String`、`Void` 标量与直接调用。
- 模块级全局（批次 4.4）：`Int`/`Bool`/`String` 全局在入口块按类型物化（`String` 仍是
  `{ptr, len}` 借用视图，无分配器），module init 按 IR 顺序写入，函数内可读可写、跨函数
  与跨单元可见；初始化顺序与 VM 一致（后一个全局的初始化读得到前一个的值）。模块 init 的
  临时寄存器由 `next_temp_reg` 分配、跨语句复用且没有声明类型：直线 init 里发射层按**每次
  写入**分配存储（`temp_locals`），具名槽仍按「一个槽一种类型」校验；含控制流的 init 保持
  原有的类型一变即拒绝，不按某条路径猜类型。判据 `test_native_global_scalars_match_vm`、
  `test_native_global_writes_from_function_match_vm`、
  `test_native_project_global_mutation_matches_vm`。
- `String` 判等与顺序（批次 4.1）：`Eq`/`Ne`/`Lt`/`Le`/`Gt`/`Ge` 接受两侧同为 `String`，
  语义是按 UTF-8 字节的字典序（对合法 UTF-8 与码点序一致），内容相等不是指针相等；
  运行期入口与值形状见“运行时 ABI 约定（首版）”。判据
  `test_native_string_equality_matches_vm`、`test_native_string_ordering_matches_vm`、
  `test_native_string_compare_through_return_matches_vm`，合成 IR 的合并值判据
  `test_native_string_phi_merge_compares_by_content`；后端单元判据
  `test_native_string_equality_comparison_is_accepted`、`test_native_string_ordering_comparison_is_accepted`。
- `Int` 位运算与受检移位（批次 4.2）：`And`/`Or`/`Xor` 接受两侧同为 `Int`，按二进制补码
  逐位处理，不做值范围检查；`Shl`/`Shr`/`Sar` 接受两侧同为 `Int`，位移量先截断成 `u32`
  （与 VM 的 `b as u32` 同口径），只有 `< 64` 才发射移位指令，越界进入带位置的运行时失败块
  （E6007，与 VM 同码同位置），值溢出不是错误（位被移出）。三条变体按语义分别映射：
  `Shl` 发 `shl`、`Sar` 发 `ashr`（有符号 `>>`，前端把 `>>` 降到 `Sar`）、`Shr` 发 `lshr`
  （逻辑右移，前端今天不产出）。判据
  `test_native_int_bitwise_matches_vm`、`test_native_shift_left_matches_vm`、
  `test_native_shift_right_matches_vm`、`test_native_shift_out_of_range_matches_vm_diagnostic`、
  `test_native_shift_amount_truncates_like_vm`、合成 IR 的
  `test_native_sar_instruction_uses_arithmetic_shift` 与
  `test_native_shr_instruction_uses_logical_shift`，均在 O0 与 O2 各跑一遍；
  后端单元判据 `test_native_int_bitwise_operands_are_accepted`、
  `test_native_shift_operands_are_accepted`、`test_native_shift_mixed_operand_is_rejected_with_location`。
- `Float` 标量（批次 4.5）：`Add`/`Sub`/`Mul`/`Div` 接受两侧同为 `Float`，直接发射
  `fadd`/`fsub`/`fmul`/`fdiv`；`Neg` 发射 `fneg`；`%` 是**floor 取模**（结果符号随除数），
  经运行时入口 `__yx_rt_floor_mod_f64` 而不是 LLVM `frem` 的截断语义（`-7.5 % 2.0` 得 `0.5`，
  `frem` 会得 `-1.5`）；六种比较按 IEEE/Rust 语义对齐 VM：`Eq`/`Lt`/`Le`/`Gt`/`Ge` 用 ordered
  谓词，`Ne` 必须用**无序** `UNE`（`NaN != NaN` 为真，ordered `ONE` 会判成假）；`Float` 的
  局部、形参、返回与 `Phi` 汇合都在支持集内。判据 `test_native_float_arithmetic_matches_vm`、
  `test_native_float_nan_and_infinity_match_vm`；混型 Int×Float 见拒绝边界。
- 直线指令；无 `Phi` 的分支和循环（`Jmp`、`JmpIf`、`JmpIfNot` 按 `BasicBlock` 发射），判据为
  `test_native_no_phi_branch_executes_both_edges`、`test_native_no_phi_loop_matches_vm`。
- `Phi` 汇合：生产端 SSA 的 `Phi`
  在 LLVM 块首发射，incoming 沿前驱边、按 IR 块 label 登记；dst 与已知输入的类型证据取
  `ValueInfo.ty`。显式未知输入按生产端 `resolve_phi_type`
  同一契约跳过已知类型比较，是否放行由可观察性判据决定：合并值没有使用点（死合流；生产端
  `while` 短路累加器的循环头 `Phi`
  即此类）时 `undef`
  不可观察，发射层按 dst 类型物化；合并值被使用（live）则带位置拒绝，见下文拒绝边界。判据为
  `test_native_phi_short_circuit_merge_matches_vm`、`test_native_if_value_merge_matches_vm`、`test_native_while_merge_value_matches_vm`、`test_native_nested_merge_value_matches_vm`，后端单元判据
  `test_native_phi_accepts_known_merge_type`、`test_native_phi_dead_unknown_incoming_is_allowed`。`Phi`
  汇合内的运行期失败与 VM 同诊断码与位置，判据为
  `test_native_phi_failure_path_matches_vm_diagnostic`。
- `Local` 合并：alloca 统一提升到函数入口块，分支只发射 store，汇合读等价 VM 帧槽语义；同一 `Local`
  在多个前驱块写、汇合后读不再拒绝，判据为
  `test_native_local_multi_arm_merge_matches_vm`，后端单元判据
  `test_native_local_multi_predecessor_write_is_allowed`；合成 IR 的单前驱 `Phi` 判据为
  `test_native_phi_module_builds_and_runs`。
- 控制流正例（真实 Driver 源文件，native O0/O2 与 VM 的退出码与 stdout 逐项对照）：break/continue
  `test_native_break_continue_matches_vm`、嵌套 if/else
  `test_native_nested_if_else_matches_vm`、嵌套 while `test_native_nested_while_matches_vm`、循环内
  `Local` 更新 `test_native_loop_local_update_matches_vm`、跨块 `Value` 直用
  `test_native_cross_block_value_matches_vm`。
- 失败路径：受支持的上下文里运行期失败与 VM 同诊断码与位置——循环内除零
  `test_native_loop_division_by_zero_matches_vm_diagnostic`（E6001）、直线溢出
  `test_native_add_overflow_reports_source`（E6007）。
- `Operand::Value` 作指令 dst 并读回：`test_native_value_destination_readback`。
- `Operand::Value` 的类型来自 `ValueInfo.ty`；`Local` 保持存储语义，`Arg`/`Global` 不变。
- 多文件项目（入口与合并）：库 API `build_native_project_with_options`
  复用编排层的发现、合并与初始化次序，CLI 的 `build <entry.yx> --emit exe` 在入口向上找到
  `yaoxiang.toml`
  时走项目路径，找不到项目根保持单文件路径（B-i）。多文件夹具（跨文件常量、子目录模块、init 次序）在 CLI 与 API 形态下均与 VM 同结果，判据为
  `test_native_multifile_use_constant_cli_and_api`、`test_native_multifile_subdirectory_module_cli_and_api`、`test_native_multifile_init_order_cli_and_api`。
- 入口身份：模块先执行 `__yx_module_init`，再按 `ModuleIR.entry_function` 调 Bin 入口；`Script` 的
  `entry_function` 为空时不隐式调用 `main`。
- 文件身份：函数按 `function_files`、模块 init 按 `init_file_ids`
  选择真实文件路径，带位置诊断与运行期错误指向真实文件，判据为
  `test_native_multifile_dependency_init_diagnostic_uses_real_file`、`test_native_multifile_dependency_function_diagnostic_uses_real_file`。

**拒绝边界**：

- `Phi`
  自身的结构拒绝（批次 3.5 收窄后的残余边界）：dst 不是 SSA 值、dst 类型显式未知或不受支持、已知输入类型不一致、incoming 不是 SSA 值或常量（`Local`/`Arg`/`Global`
  输入需要前驱出口 load，不在映射面内）、没有 incoming（无前驱块）、`Phi`
  不在块首分组，均显式拒绝且带源码位置；后端单元判据
  `test_native_phi_unknown_destination_is_rejected`、`test_native_phi_known_type_mismatch_is_rejected`、`test_native_phi_local_incoming_is_rejected`、`test_native_phi_entry_block_without_incoming_is_rejected`。LLVM
  verifier 兜底失败同样包装为 AOT 自有诊断，不外泄 LLVM 原文。
- `Phi` 的 live 显式未知 incoming：显式未知输入进入合并值被本函数任一指令使用（live）的
  `Phi` 时，按 dst 类型物化 `undef` 会让 VM 帧槽默认 Void 变成可观察的未定义值，与
  `Local`「读点在某条路径上没有写」同一口径，构建期带 `Phi` 源码位置拒绝、不产生产物；诊断点名值
  id 并说明 Void/undef 边界。Void 合并值没有 LLVM phi 形态、不物化 `undef`，不在此判据内。判据为集成
  `test_native_live_unknown_incoming_phi_is_rejected_with_location`（断言诊断含源码位置且失败构建不产出产物），后端单元判据
  `test_native_phi_live_unknown_incoming_is_rejected`。
- `Local`
  读点未写：读点在某条可达路径上从未写过该槽（VM 帧槽默认 Void，带类型的 alloca 无法物化），显式拒绝并带源码位置；后端单元判据
  `test_native_local_read_without_write_on_path_is_rejected`。顺序写与循环重复写合法，后端单元判据
  `test_native_local_sequential_write_is_allowed`、`test_native_local_loop_repeated_write_is_allowed`。
- 显式未知值与合成坏 IR：值类型显式未知 `TypeRef("?")` 或不支持（如
  `Float(64)`）时报带值 id 与原始类型的 unsupported，合成多定义在 `verify_ssa`
  入口先红；这是 lower 侧类型覆盖缺口，补齐归 lower，不在 AOT 推断或静默 Void。判据为
  `test_native_explicit_unknown_value_is_rejected`（集成与后端单元各一条）、`test_native_multi_definition_value_is_rejected`、`test_native_unsupported_value_type_is_rejected`；失败构建不覆盖上次产物，判据为
  `test_native_rejected_synthetic_ir_preserves_previous_output`。
- 函数体内的 `std` 调用：调用结果在 IR 中的值类型为显式未知
  `TypeRef("?")`，AOT 报带值 id 与原始类型的 unsupported；缺口归 lower 的类型覆盖补齐。**例外只有
  `std.assert` 一项**：该内建的返回类型由分析器直接拥有（`Void`），因此函数体内的断言可用；其余
  `std` 调用仍按显式未知拒绝，例外不外扩。
- `Script` 定义了 `main` 却没有任何调用：构建期显式拒绝（带 `source:line:column`
  的 AOT 能力边界诊断，不产生产物），文案说明「`Script` 定义 `main`
  但未调用、AOT 不隐式调用」并给出改用 Bin 项目或移除未调用 `main` 的路径；集成判据
  `test_native_script_unused_main_is_rejected`。`Script` 无 `main`
  仍可构建（`test_native_script_without_main_still_builds`），`Script` 显式调用 `main`
  与 Bin 项目入口不受影响（`test_native_project_bin_entry_function_is_called`）。
- `String` 支持集之外的字符串操作：`StringGetChar`（`s[i]`）、`StringConcat`、
  `StringFromInt`、`StringFromFloat` 与全部 `std` 调用（`std.assert.assert` 除外）仍显式拒绝并带
  `source:line:column`；混型比较（`String` 与 `Int` 等）同样带位置拒绝。判据
  `test_native_string_operation_outside_slice_is_rejected`、
  `test_native_string_mixed_comparison_is_rejected_with_location`。
- 混型 Int×Float 算术（批次 4.5）：操作数必须同型。VM 在 typecheck 层把 `Int` 提升成
  `f64`（RFC-011b 的 widening），但 IR 不落显式 widening 指令，AOT 侧无法从 IR 判出提升点，
  因此 `Int + Float` 一类带位置拒绝，不猜一种语义（判据
  `test_native_float_mixed_int_operands_are_rejected`，同一用例核对 VM 侧确实能跑）。
- 不支持的全局类型（批次 4.4，4.5 收窄）：容器、闭包、资源仍带位置拒绝、不产生产物，不静默
  按 `Void` 回落。`Float` **全局**仍被拒绝（4.5 只把 `Float` 加进标量支持集，模块级 f64 的
  物化未验收）：分析层在写入该全局的指令处带位置拒绝，声明的但从未读写的 `Float` 全局在
  全局播种处按名字拒绝（判据 `test_native_unsupported_global_type_is_rejected`，该用例断言
  初始化式的 `file:line`）；容器全局落在容器构造指令上（判据
  `test_native_container_global_is_rejected`），该指令的 span 由 lower 留空，诊断位置显示
  `0:0`——这是 lower 侧的 span 缺口，不由本层补造。
- 槽位缺指针（批次 4.3 收口：补齐证据 + 非 `Void` 硬失败）：`Void` 是唯一没有 LLVM 形态的
  标量（后端单元判据 `test_only_void_scalar_has_no_llvm_form`），分析层只放行 `Void`/`Int`/
  `Bool`/`Float`/`String`，另外四种槽位都在入口块拿到 alloca、全局槽位由 `declare_globals` 物化——
  因此「槽位缺指针」只可能是 `Void`。`Void` 槽位的读取按**类型**物化为 `Void`，与
  `Const(ConstValue::Void)` 同一规则，不依赖缺指针兜底（集成判据
  `test_native_void_slot_reads_are_explicit`；已声明 `Int` 全局的读取判据
  `test_native_declared_int_global_read_is_materialized`）。非 `Void` 槽位缺指针是发射层不变量
  被破坏：`bail!` 点名槽位与类型、构建失败且不产生产物，不再静默物化 `Void`。未声明的全局
  槽位在 `verify_ssa` 闸门按槽位越界拒绝并点名槽位（判据
  `test_native_undeclared_global_read_is_rejected`）——发射层收不到这条读取。

**L4 自有判据的性质（L3 裁决）**：批次 3.5 后该性质由 `src/backends/llvm/analysis.rs` 的
`check_local_read_definitions`（自建可达块 CFG、前驱表与 must-def 集合不动点）、`Phi`
的类型证据检查（dst 与已知输入取 `ValueInfo.ty`，显式未知输入按生产端 `resolve_phi_type`
同一契约跳过已知类型比较）与 `Phi`
的可观察性判据（见下）共同承担。这是**支持集判定**——判定程序是否落在 AOT
支持集内并给出带位置的拒绝，不是重建 SSA 或类型：不生成
`Phi`、不重命名，`Local` 合并由发射层把 alloca 提升到入口块实现，判据不跨层。

**可观察性判据的保守上界（提交 `5e26ea2b`）**：`live`
取全函数上界——`Phi` 的 dst 出现在本函数任一指令的 `use_operands`
里（含其他 `Phi`
的 incoming）即视为 live，不做可达性分析。取舍：宁可拒绝也不把不可证明的 VM 帧槽默认值静默物化成
`undef`；代价是误拒面——死 `phi` 链（被另一条死 `Phi`
的 incoming 引用）与不可达块里的使用点会被计成 live 而遭拒绝，尽管 `undef`
实际不可观察。后续复核条件：真实程序（生产语料或用户程序）被该上界误拒时，再把 `live`
改为可达性感知的使用点集合；当前生产端的死合流形态（`while` 短路累加器的循环头
`Phi`，无任何使用点）不受影响。

**L3 裁决（2026-11-27，批次 3.5 收口时记录）**：这套汇合判据保持 L4 自有，不在本批提升到 L3——`src/middle/`
的生产端 SSA 与 `verify_ssa` 不消费它，L4 不向 L3 反向输出判定接口。事实依据：实现提交 `17882acb` 中
`check_local_read_definitions` 仅被 `src/backends/llvm/analysis.rs` 调用；`Phi` 类型检查只读取
`ValueInfo.ty` 与 `is_explicit_unknown_type`
这两个 L3 已有定义。后续复核点：其他后端（如字节码或 Wasm 通道）需要同一支持集判定时，先裁决判据入 L3 的接口形态，再搬移；L3 自身不因本批新增任何汇合判据。

**登记项（2026-11-26 复审）**：

- **项目配置的取用有两条链**：CLI 从项目根的 `yaoxiang.toml`
  读出编译配置再灌入项目 native 构建；库 API 由调用方直接传入编译配置，库侧不读 toml。两条链汇到同一处（`build_native_project_with_options`
  的配置形参），因此项目 native 路径与 `run`/`check` 链读到同一份上限。
- **`--emit bytecode` 仍是单文件语义**：项目入口只作用于
  `--emit exe`；字节码路径恒按单个文件构建，不合并项目。
- **`Script` 不隐式调用 `main` 的保证是两条合起来**：发射层按 `ModuleIR.entry_function`
  调 Bin 入口，`Script` 的 `entry_function` 为空时不调 `main`；分析层再拒绝「定义了 `main`
  却不可达」的 `Script`。二者合起来，`Script` 加未调用 `main` 不会静默 exit 0。
- **assert 白名单是硬判**：分析层对 `std.assert.assert` 硬判参数形态（`Bool` 或
  `(Bool, String)`），返回类型由分析器直接拥有（`Void`），不推断其他未知值。
- **L4 补证据只有这一个例外**：`std.assert`
  的结果值类型由 L4 分析器拥有；其余显式未知仍按拒绝处理，缺口归 lower 的类型覆盖补齐。

**等价账本（最小集，八条）**：

| #   | 条目                         | 状态           | 判据                                                                                                         | 守的决策                                                                                                 |
| --- | ---------------------------- | -------------- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| 1   | 无 `Phi` 分支                | 已收口         | `test_native_no_phi_branch_executes_both_edges`                                                              | 支持集声明：无 `Phi` 的分支在支持集内，条件两条边都要发射                                                |
| 2   | 无 `Phi` 循环与 `Local` 存储 | 已收口         | 可执行账本同时钉住两条判据：`test_native_no_phi_loop_matches_vm`、`test_native_loop_local_update_matches_vm` | `Local` 保持存储语义：顺序写与循环重复写合法                                                             |
| 3   | 多文件 init 次序             | 已收口         | `test_native_multifile_init_order_cli_and_api`                                                               | 入口身份：模块先执行 `__yx_module_init`，再调 Bin 入口                                                   |
| 4   | 循环内除零（E6001）          | 已收口         | `test_native_loop_division_by_zero_matches_vm_diagnostic`                                                    | 失败路径与 VM 同诊断码与位置                                                                             |
| 5   | `Phi` 映射                   | 已收口         | `test_native_phi_short_circuit_merge_matches_vm`                                                             | `Phi` 汇合按前驱边发射 incoming、类型证据取 `ValueInfo.ty`，短路合流与 VM 同结果，不静默拒绝支持集内程序 |
| 6   | 合并 `Local`                 | 已收口         | `test_native_local_multi_arm_merge_matches_vm`                                                               | 多前臂写—汇合读由入口块 alloca 承载，与 VM 帧槽语义同结果，不外泄 LLVM 原文                              |
| 7   | 显式未知类型                 | 已收口（拒绝） | `test_native_explicit_unknown_value_is_rejected`                                                             | 未知类型不静默猜测、不静默 Void                                                                          |
| 8   | `String` 判等与顺序          | 已收口         | `test_native_string_equality_matches_vm`、`test_native_string_ordering_matches_vm`                           | 内容相等不是指针相等；六个算子按 UTF-8 字节字典序，空串、前缀、多字节为边界；合并值经 `Phi` 仍按内容比较 |

这八条是等价账本的最小交付边界：**已支持项不得移出集合换绿**——支持集只增不减，绿灯不得靠把已支持项移出集合获得。

可执行账本是机器可读的 `tests/aot-ledger.json`，由 `scripts/ci/check-aot-ledger.py`
对账并接入 CI 的 LLVM 作业：账本的 target 是集成测试目标
`native_aot`，八条的 id 与状态以脚本内嵌期望机械核对，账本只存 id、状态与判据名。**决策文本以本表为唯一来源。**
后端单元判据（顺序写与循环重复写合法、单支臂合并只有合成 IR 级证据等）不在账本 target 内，仍见「支持集」小节。

**裁决（2026-11-26）**：

- **B-i（入口形态）**：库 API 增加项目/多文件入口，CLI 保持单入口文件。项目 native 构建按编排层的发现、合并与初始化次序执行（见「支持集」）。
- **D（等价账本口径）**：less is
  more——账本只保留覆盖关键决策与拒绝边界的必要条目，不以夹具数量防御；账本按上表的八条最小集给出。

### 0. 值契约闭合（P7 的 SSA 生产端）

本阶段属于 P7 的生产端，不属于 AOT 后端。代码位置：

- `src/middle/ir/`：支配关系、SSA 构造与 `verify_ssa` 判据；
- `src/middle/lower/`：`finalize_code_body` 在块切分之后、值槽分配之前调用构造；
- `src/backends/llvm/` 只消费经校验的 IR，不重建 SSA。

三个工作单元（已落地，提交 `b503138b` / `ba482c8d` / `bdda6d53` / `aeb4cbea`）：

- **SSA 构造**（`src/middle/ir/ssa.rs` 的 `construct_ssa`）：对每个多次定义或跨块未受定义支配的
  `Operand::Value` 做 SSA 构造——计算支配树与支配边界，在汇合块首插入
  `Phi`，按支配关系重命名（新值编号；`defining_block` 记真实块 label）。`Locals`
  保持存储语义；`Const`/`Arg`/`Global` 不变；无值表的初始化路径跳过。产物满足：每个 `Operand::Value`
  单定义；每个使用被其定义支配；`Phi` 的 incoming 覆盖全部前驱，并按前驱 label 升序。
- **校验器补全**（`src/middle/ir/verify.rs`）：`verify_ssa` 的定义唯一性判据挂在 `Operand::Value`
  上——`Locals` 是存储，`7b.4` 的线性扫描分配器按活跃区间复用槽位，同一槽多次写是合法形态，对 `Local`
  的唯一性要求与槽复用语义冲突；同时有“值使用受其定义支配”与 `defining_block`
  一致性的判据，保留具名槽的 use-before-def 支配性检查。门禁 `scripts/ci/check-ir-verifier.sh`
  跑三条判据：全语料 `verify_loose`、生产语料 `verify_ssa`、负例自证（合成坏 IR 必须被拒绝）。
- **基线再生**：IR 快照与语料差分基线同提交再生，diff 人工 review；`translator` 的 `Phi`
  展开语义不变，VM 与 `.42` 行为由语料差分保证。

**放行证据**：`verify_ssa` 在生产语料全绿；快照与语料 diff 已人工审；本地门禁通过；每个
`Operand::Value` 单定义且使用受支配。

**边界**：本阶段不改变语言语义，不新增用户可见行为，不改 `Locals`
的存储语义，不在 L4 重建 SSA。支配边界在不可达块与回边上的处理由单测覆盖；值表重建导致快照全量漂移属预期，按基线再生流程人工 review。

**决策（2026-11-25）**：值契约取完整 SSA，由生产端构造；AOT 分支承载该工作，代码落在 middle 层。7b 的“形态切换”不含生产端
`Phi`，其闭合登记为 P7 7f（见施工任务表）。

## 施工顺序与交付物

**依赖关系**：工具链 → 原生样例；原生样例与共享 SSA 两项都满足后，进入正式代码生成。此后先确定原生值和资源的生命周期，再接 FFI 和并作。每批由实现者交付产物与复现步骤，独立验证者复核放行条件。语料差分从第一个可执行文件开始，不集中到发布前补做。

### 1. 固定构建环境并隔离 LLVM 依赖

- **实现**：构建负责人选定兼容版本并锁定依赖。记录宿主、目标三元组、LLVM 版本、系统链接驱动、目标运行时库的位置与获取方法。用 LLVM 的
  `TargetMachine` 给模块设置目标三元组和 `DataLayout`；对象布局不取宿主 Rust 类型的大小。
- **交付**：两个可复现的构建通道。无 LLVM 的通道构建默认 CLI、VM 与 Wasm
  Playground；有 LLVM 的通道构建可选后端。[现有 CI](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/.github/workflows/ci.yml)
  使用 `--all-features`，这些作业显式准备锁定的 LLVM，不删 feature 来换取通过。
- **固定组合**：Inkwell 0.11.0、LLVM 18.1、目标 `x86_64-unknown-linux-gnu`；系统 `cc` 编译并链接
  `src/backends/llvm/runtime.c` 中的最小 C ABI 运行时。`YAOXIANG_CC` 可指定其他 C 驱动。官方构建设置
  `LLVM_SYS_181_STRICT_VERSIONING=YES`，拒绝不匹配的 LLVM 主版本。批次工具分别用
  `python scripts/dev/scratch.py build <batch>` 和
  `python scripts/dev/scratch.py build <batch> --features llvm` 选择默认引擎与 LLVM 引擎。
- **放行**：无 LLVM 的环境能完成默认构建；有 LLVM 的环境能验证模块、生成目标文件、链接并运行。另验缺链接器、LLVM 主版本不匹配两种失败，能分清是环境错误还是源程序错误。

### 2. 交付第一个真实源程序，不等待 P7

- **输入**：让现有 Driver 编译一份单文件脚本。只接受能从常量、已确定的函数签名和直线指令确定类型的数据流。支持
  `Int`、`Bool`、直接调用、返回、顶层初始化，以及观察结果所需的最小断言入口。不承诺通用标准库支持。
- **生成**：先用 `verify_ssa`
  排除结构错误，再逐指令检查该子集的类型依据。先声明函数，再翻译函数体与模块初始化。对脚本只调用初始化序列；不按名字再找一次
  `main`。依次执行 LLVM 模块验证、目标文件发射、链接。链接成功后再交付最终文件；失败不覆盖上次成功的产物。
- **命令**：通过 `yx build demo.yx --emit exe` 进入原生路径，通过 `--emit bytecode`
  保持现有字节码路径。本批不翻转裸 `build` 的默认值。
- **正反例**：让脚本把参数传给算术函数，再通过既有
  [`std.assert.assert` 契约](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/std/assert.rs)
  比较返回值。预期正确时正常退出；故意改错预期时，VM 和原生程序都在同一源码行报告 E6005。断言符号按既有导出绑定解析，不在编译器里按样例文件名特判。不将源程序的函数返回值临时定义成进程退出码。
- **停止条件**：输入含不明类型、未证实的间接调用或不支持的可达指令，构建明确失败并指出来源，不生成半成品、不退回 VM。不能证明某段代码不可达时也不跳过。原型的失败是能力边界，不改变同一程序在 VM 上是否合法。
- **放行**：从同一源码得到 VM、原生 O0 与原生 O2 三份结果。在仅含目标系统基础库的环境运行原生产物；不携带 LLVM、编译器或字节码解释器。手工拼 LLVM 模块、只看退出码 0 或只保存
  `.ll` 均不能代替上述验证。

### 3. 接正式共享 IR，扩到控制流与多文件

**前置不看日期，看产物。** 按 [SSA 设计](../architecture/04-ssa.md)
和 P7 施工表核对：编译器真实产出逐值类型、区分可写存储与单赋值临时值、表达前驱及汇合值；`verify_ssa`
在含分支和循环的生产 IR 上通过。涉及量词为零的参数或证明字段时，还要核对相应擦除契约；不把所有泛型原件当作已擦除。条件缺一项，只保留第 2 批，不扩大支持声明。

1. 先建基本块，再翻译指令并补全 `phi`
   的前驱输入。SSA 值直接映射为 LLVM 值，可寻址的可变局部使用存储。对仍可达的栈指令，沿控制流核对栈深与元素类型再消除；不同前驱不一致时拒绝，不让无效 IR 进入优化器。
2. 按单态化后的签名声明调用目标。项目使用编排器给出的限定名、全局槽和初始化次序。Bin 先执行初始化，再调用指定入口；Script 即使定义了
   `main` 也不隐式调用。依赖模块初始化中的错误使用该模块的文件身份，不一律标到入口文件。
3. 以同一份源码验证两路分支、循环回边、跨模块泛型直接调用、导入初始化顺序和脚本显式调用 `main`
   恰好一次。用一次错误输入验证诊断文件和行。正式路径覆盖原型后移除原型专用适配，不保留两套指令翻译。

### 4. 链接原生运行时，再接 FFI 与并作

| 交付切片            | 实现边界                                                                                                                                                                                                                                           | 验收样例                                                                                                                                   |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| 值、闭包与资源      | 通过窄 C ABI 静态链接所需运行时。明确字符串的字节长度和所有权、容器布局、闭包环境以及释放方。消费 IR 的 `Drop`、`RcNew`、`ArcNew` 等操作；不在后端重做所有权分析。依据：[所有权设计](../../rfc/accepted/009-ownership-model.md)。                  | 闭包离开定义域后仍读到捕获值；被移动的值不重复析构；正常返回、提前返回及 `?` 传播各自的资源按约定顺序恰好释放一次。                        |
| 外部函数接口（FFI） | 消费绑定与已确定的签名，区分标准库处理器和外部 C 符号。按目标 ABI 做参数和返回值转换；不把 Rust 枚举或闭包布局当 C 布局。依据：[FFI 核心机制](../../rfc/accepted/026-ffi-core-mechanism.md)。                                                      | 用同一目标工具链编译一个有真实签名的 C 测试库；在 YaoXiang 与 C 之间往返传值。覆盖空指针、字符串边界和缺失符号；不能只验证简单调用未崩溃。 |
| 并作                | 消费 `Spawn` 和 `SpawnFromList` 携带的 `ExecutionPlan`，将任务入口、环境与依赖交给调度器。顺序代码不启动调度器。依据：[运行时架构](../../rfc/accepted/008-runtime-concurrency-model.md)、[并作语义](../../rfc/accepted/024-concurrency-model.md)。 | 有依赖的任务按序消费结果，冲突资源不同时访问，任务失败向调用者传播。对无依赖任务只验允许的结果，不把某一次线程调度顺序定为语义。           |

每个切片从生产者生成的 IR 起跑。若缺少必要的布局、类型或释放事实，先在该事实所属层立项补足；不得以猜测的布局补齐接口。运行时 ABI 在此阶段视为内部接口，不承诺用户自行链接的第三方兼容性。

## 运行时 ABI 约定（首版，内部接口）

本节固定编译器与原生运行时之间的调用约定。等价判据只看可观察行为——退出码、标准输出、结构化诊断的码与位置；内部表示因此可以演进，但不得为了当前切片改变函数签名：后续切片使用本节预留的位置扩展。设计依据是 [RFC-018](../../rfc/accepted/018-llvm-aot-compiler.md) §4 的指令翻译表（`String` 是 `{ ptr, len }`）与 [RFC-026](../../rfc/accepted/026-ffi-core-mechanism.md) 的 C 编组方向。

### 值种类与传递方式

| 语言值   | LLVM 表示      | 传递方式 | 备注                                      |
| -------- | -------------- | -------- | ----------------------------------------- |
| `Void`   | 无             | 不占位   | 无返回值、无参数                          |
| `Bool`   | `i1`           | 按值     | 进入运行时前零扩展为 `i64`                |
| `Int`    | `i64`          | 按值     | 受检算术入口见下                          |
| `String` | `{ i8*, i64 }` | 按值     | `ptr` 是 UTF-8 字节首地址，`len` 是字节数 |

`String` 的两个字段按值拆开传给运行时入口（`i8*, i64`），不传聚合体：运行时是 C，不依赖 LLVM 的结构体传参约定。

### String 布局

- `len` 是 UTF-8 字节数，不是码点数。
- 字符串不以 NUL 结尾，内容可以包含 0 字节；运行时一律按显式长度处理，禁止 `strlen` / `strcmp` / `printf("%s")` 一类以 NUL 为界的调用。
- 批次 4.1 的字符串只有静态字面量一种来源：LLVM 全局常量，生命期等于进程，**永不释放**，语义是借用视图。
- 所有权是**编译期事实**：由 IR 的释放计划决定谁在哪个语句边界释放，L4 不重做所有权分析；值里不携带运行时所有权标记。
- 未来的 owned 字符串在 `ptr` 之前放堆头（引用计数、种类、长度），值形状仍是 `{ ptr, len }`——新增分配不改变任何函数签名。
- 用户可见的长度有两套口径，不得混用：`StringLength` 指令对 `String` 是字节数（对容器是元素数）；`std.string.len` 与内建 `len` 对 `String` 是码点数。选择由调用方按语义决定，ABI 层不统一。

### 比较与浮点入口（批次 4.1 / 4.5 已实现）

| 入口                    | 签名                      | 语义                                              |
| ----------------------- | ------------------------- | ------------------------------------------------- |
| `__yx_rt_str_eq`        | `i64(i8*, i64, i8*, i64)` | 长度相同且逐字节相等返回 1，否则 0                |
| `__yx_rt_str_cmp`       | `i64(i8*, i64, i8*, i64)` | 按 UTF-8 字节字典序返回 -1 / 0 / 1                |
| `__yx_rt_floor_mod_f64` | `f64(f64, f64)`           | floor 取模：符号随除数（`fmod` + 符号修正）       |

- `String` 的六个比较算子都落到这两个入口：`Eq`/`Ne` 用 `__yx_rt_str_eq` 的结果与 0 比较，
  `Lt`/`Le`/`Gt`/`Ge` 用 `__yx_rt_str_cmp` 的结果与 0 比较。
- 语义是**内容**比较：比较只读 `{ptr, len}` 指向的字节，不看地址。
- 静态字面量落在可合并的 `.rodata.str1.1` 段（`SHF_MERGE | SHF_STRINGS`），由**链接器**把内容相同的字面量合并成同一地址：O0 的对象文件里仍是各自独立的一份（`yx.string.N` 各占不同偏移），O2 的 IR 优化会先折叠掉一部分，两种优化级最终都只剩一个地址。因此「内容相等而指针不等」在批次 4.1 没有可观察用例；拥有所有权的字符串切片才会产生两个内容相同、地址不同的值，届时该性质由用例直接观察。
- `Int` 与 `Bool` 的比较仍由 LLVM `icmp` 直接发射，不经过运行时；`Float` 比较用 `fcmp`，
  `Ne` 取无序 `UNE`，其余取 ordered 谓词，以便 `NaN` 与任何值（含自身）比较都「不等于」、
  四种序比较全假——与 VM 的 Rust `f64` 比较逐项一致。
- `__yx_rt_floor_mod_f64` 用 `fmod` 实现，构建期由系统 C 工具链编译并**显式链接 `-lm`**：
  不依赖目标环境的默认库组合。`NaN` 输入与除数为 0 时返回 `NaN`，与 VM 的
  `crate::util::arith::floor_mod_f64` 同结果。
- VM 侧 `exec_compare` 的 `String` 分支是 Rust `str` 比较（字节序），本节的入口与之同结果。

### 错误 ABI（现状）

- 可能失败的运行时入口在参数末尾接收位置：`i8* file, i64 file_len, i64 line, i64 column`。
- 失败的可观察面固定为：`error[E%04llu]: message`、` --> file:line:column`、退出码 1。等价判据只比较 E 码、文件与行，不比较译文。
- 批次 4.1 不新增错误通道，也不引入 `trap` 指令；后续切片统一失败入口时，可观察面不变。
- 错误码的权威源是 `src/util/diagnostic/codes/`，运行时按码与位置对齐，不复制诊断文案。

### 后续切片的预留

以下形状在本节固定，具体切片按此实现；改动这些形状属于对外契约变更，单独评审。

- **容器**：共享堆对象加头（种类、长度、引用计数），拷贝共享、原地写对别名可见；不得用内联值语义。布局事实取 `TypeDeclBody.fields` 与 `ValueInfo.ty`。结构体构造函数目前在字节码翻译期由 `TypeDeclBody` 合成，AOT 若要构造结构体，先裁决合成点的归属，不在两个后端各写一份。
- **闭包**：`{ fnptr, envptr }`；捕获是值拷贝，`env` 同时作为被调实参的前缀（与 VM 的 `CallDyn` 同语义）。前置事实：`MakeClosure` 的结果类型与闭包体参数类型在 IR 里仍是未知类型与类型变量。
- **FFI**：独立的 C ABI 转换层，按目标 ABI 编组；不把 Rust 枚举或闭包布局当 C 布局。当前 VM 的 C 机制只实现 `() -> i32` 且库需预加载，真实签名在 VM 侧没有等价 oracle。
- **并作**：先顺序执行——空闭包列表的 `Spawn` 是退化指令（与 VM 的 Embedded 模式同语义），目标是消费 `ExecutionPlan` 的单线程执行器；线程调度器单独裁决。
- **所有权与释放**：owned 字符串的释放方与释放点、SSA 临时值的释放事实、以及「恰好释放一次」的可观察判据都未落定。在这些事实补齐之前，AOT 只声明「静态字面量永不释放」，不声明资源已按约定释放。

### 与其它文档的关系

本节是运行时接口的唯一权威，`src/backends/llvm/runtime.c` 的实测入口以本节为准。平台 ABI 与发布门槛见 RFC-018 附录 B 与「发布与回退」。

## 如何判定两个后端等价

验证负责人在同一修订、同一目标与同一输入上比较 **VM / 原生 O0
/ 原生 O2**。复用[语料探针](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/examples/corpus_probe.rs)的三类预期：正常行为、编译错误、运行时错误。分别记录编译退出状态和运行退出状态，防止把“没编出来”错计成“按预期运行失败”。对已支持的语料不允许跳过。

| 要防的错误                 | 输入与判定                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 整数语义变成 LLVM 默认行为 | 用函数参数制造运行期计算：`-7 % 3 == 2`、`7 % -3 == -2`；除零报 E6001，最大有符号整数加一报 E6007，最小值除以 -1 走溢出诊断。不可无保护地发射 `sdiv` 或把裸 `srem` 当数学取模。现成对照：[取模](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/tests/yaoxiang/01-syntax/basics/modulo_math.yx)、[除零](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/tests/yaoxiang/06-compile-errors/runtime/div_zero.yx)、[溢出](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx)。 |
| 优化改变了行为             | 将 O0 和 O2 的可观察值、错误路径与 VM 各自比较。浮点扩面时加入 NaN、有符号零及混合数值运算；不启用会改变这些结果的 fast-math 假设。LLVM verifier 通过只说明结构合法。                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 顺序或位置错误被归一化掩盖 | 比较退出码、输出和结构化诊断的码、文件、行，不比较译文、机器码或整份 LLVM IR 的文本。现有探针对标准输出按行排序；初始化、析构等有序副作用另用有序事件记录或最终状态断言。                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 错把“不支持”当成“通过”     | 每次扩面记录支持的语料、应成功数、预期错误数、尚未支持的特性及其对应施工批次。已支持项出现后端拒绝即回归；不能把它移出集合使门禁变绿。前端已拒绝的程序，两条构建路径都不产出文件。                                                                                                                                                                                                                                                                                                                                                                                                                                                            |

沿用[等价性判据](../architecture/07-equivalence-oracle.md)及[实现者手册](../HOWTO.md)的快照、语料和资源预算规则。用受控变异证明关键探针有效，例如把数学取模替换成截断余数，负操作数用例必须变红。原生路径新增时，VM 基线不因 AOT 结果不同而更新。若发现语言规范与 VM 冲突，记录最小源程序并按 D59 分流；不在后端自行裁决语言语义。

## 发布与回退

- **原型、受限预览、正式支持分开标示。**
  原型只证明单平台链路。预览随发行说明列出支持范围及明确拒绝项。正式支持某一目标前，按 RFC-018 承诺逐项对账，包括标准库依赖、反射元数据以及声明支持的运行时模式；未交付项不计完成。
- **逐目标验收。** Linux ELF 通过后，再分别验证 Windows x64
  MSVC 的 COFF 与 macOS 的 Mach-O。每个目标在对应系统上实际编译、链接、运行；检查架构、动态库依赖和 FFI 调用约定。Windows 验
  `x86_64-pc-windows-msvc`；macOS 的 `x86_64-apple-darwin` 与 `aarch64-apple-darwin`
  分开记录。单独登记编译耗时、产物大小、启动与运行耗时；没有实测值不作提速承诺。
- **失败时的动作。**
  未发布的切片按批次回退；暂不发布未过门禁的平台，保留字节码默认直到「构建产物契约」的迁移条件达成。已发布的显式选项继续给出明确诊断，不能偷偷改回字节码。默认产物按已裁决的迁移序列切换；改变既有成功程序结果、对外公开 ABI 或改变已承诺的 ABI，仍单独评审。
- **移交证据。**
  每批交付关联提交或 PR、可公开复现的源码、工具链版本、目标三元组、命令及预期/实际结果。独立验证者复跑正反例后才扩大支持范围。文档通过评审不等于批准排期或发布。
