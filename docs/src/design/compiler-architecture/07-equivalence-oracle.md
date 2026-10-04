# 重构等价性判据

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../rfc/draft/039-compiler-architecture.md) 的附属文档。四层模型、验收判据分级与执行阶段顺序见 RFC-039 正文;各附属文档的定位见 [本目录索引](index.md)。

## 定位与范围

本文定义重构过程中"如何证明行为未变"的判据。判据**按重构步骤的类别分级**——因为不同类别需要不同强度的等价性：纯搬移要求逐字节相同，而 SSA 化必然改变 IR 形态、只能判行为等价。

**本文是全部重构的前置条件。** 没有判据，[SSA 化](04-ssa.md) 不可安全实施。

本文**不覆盖**：阶段契约本身（见 `02`）、类型表示收敛的设计（见 `03`）、死代码清理的清单（见 `06`）。

## 现状

### 当前最危险的代码恰好是零测试覆盖的代码

`src/middle/core/ir_gen.rs`（8448 行）：

- 文件内**无 `#[cfg(test)]`、无 `mod tests`**
- `src/middle/core/tests/` 的 29 个测试中，`def_assign` / `local_slots` 只正向钉住 DefId 分配与槽位命名的哨兵行为；`bytecode.rs` 中 4 处提到 ir_gen 的断言全是**反向断言**（"ir_gen 不构造某指令"）
- 即：**IR 结构不变量（支配性、唯一定义、jump 目标、类型一致）目前零校验**——这正是本文第一层判据要补的

而这个文件里恰好有三类**失败时不会产生任何错误**的缺陷：

**缺陷类 1：寄存器错位。** 文件中 6 处手工 save/restore 成对语句，分离在 70 余行之外：

| 保存点 | 恢复点 | 所在函数 |
| --- | --- | --- |
| `ir_gen.rs:1745-1759` | `1819-1822` | `generate_method_ir` |
| `1858-1866` | `2026-2029` | `generate_function_ir` |
| `2116` | `2180` | curry 中间函数 |
| `2224-2229` | `2284-2288` | curry 最内层函数 |
| `2768-2777` | `2820-2851` | `generate_anon_binding_ir` |
| `4686-4695` | `4767-4774` | `generate_lambda_body_ir` |

文件自己在 `ir_gen.rs:248-249` 写下了这个契约：「**必须与 `next_temp` 的保存/恢复点物理相邻**，否则内层函数的名字会串到外层」。

漏一次 restore 的后果：`next_temp` 泄漏、临时值跳号。**IR 仍然自洽、仍然通过所有检查、仍然能编译，只是值错。** 普通断言测试抓不到。

**缺陷类 2：`arg_regs` 语义重排错位。** `generate_call_expr_ir`（`7269-8014`，747 行）内部 6 个分支共享同一个 `Vec<Operand>` 并做语义重写：

| 行 | 重排语义 |
| --- | --- |
| `7395` | 命名空间调用：命名实参重排 |
| `7724` | 结构体构造：字段重排 |
| `7835-7848` | 函数调用：命名实参重排 |

且中途还会追加默认值寄存器。**错位不报错、只错值。**

更危险的是：这一类错误在"规范化快照"下**也可能测不出来**——如果快照工具把临时寄存器号 normalize 掉（这是常规做法），那么"第 3 个实参本该用第 5 个寄存器"这类错误在快照里是**完全相同**的。

**缺陷类 3：span 键控的跨层契约静默失效。** `ReleasePlan` 以 `Span` 为键（`layers/ownership.rs:31` 产出），经 `checker.rs:1440` 传递，最终由 `ir_gen.rs:1942` 的 `self.release_plan.get(&stmt.span)` 匹配并发射 `Instruction::Drop`。两侧 span 计算方式任何一处不一致 → **Drop 指令静默消失，无任何错误**。`overload_resolutions` 同为 span 键控。

> **`method_def_ordinals` 不是 span 键控**：它是 `HashMap<String, usize>`（`ir_gen.rs:224`，key 为无模块限定的 `"Type.method"`）——其风险性质不同（定义序/重名失配而非 span 失配），且 SSA 化不解决它（处置见 [04](04-ssa.md) 机制三）。因此 span 键控契约是 **2 个**（`ReleasePlan` / `overload_resolutions`）。### 现有测试资产

好消息是判据的原料是充足的：

| 资产 | 规模 | 可用于 |
| --- | --- | --- |
| `tests/yaoxiang/**/*.yx` | **293 个**语料，按规范章节编号（`00-smoke` … `06-compile-errors`、`99-demos`）。**⚠️ 全部为单文件路径**——见下方限制 | 端到端行为差分（**仅单文件侧**） |
| `src/std/tests/*.yx` | 一批 | 标准库行为差分 |
| `tests/integration/` | 18 模块 / 5846 行 | CLI 端到端（其中 `multifile.rs` 27 个测试是**唯一**的多文件侧覆盖） |
| `src/middle/core/tests/bytecode.rs` | 1131 行 / 24 test，含 `test_every_opcode_roundtrips_not_silently_nop`（`bytecode.rs:958`，逐 opcode 往返） | 字节码层往返 |

> **⚠️ 语料覆盖的结构性限制（2026-10-03 补充）**
>
> `tests/` 目录下**没有任何 `yaoxiang.toml`**（实测 0 个）。因此这 293 个语料全部走 `util/diagnostic/mod.rs:565` `check_files_with_diagnostics` 的 `standalone` 分支 → `check_single_file` → **`Pipeline::run`（单文件路径）**。
>
> **后果**：第三层判据**只能验证单文件路径的改动**。对 [`02`](02-stage-contract.md) 的多文件侧（`orchestrator` 的 `check_project` / `compile_project` / `check_source_in_project` / `compile_embedded_module`）与 [`03`](03-type-unification.md) / [`04`](04-ssa.md) 的多文件表现，**当前没有任何语料覆盖**。
>
> **必须在 P2 建立多文件语料层**，否则「统一 Driver」这个最大的重构阶段没有可执行判据。`tests/integration/multifile.rs` 的 27 个测试是唯一现存的多文件侧覆盖，但它测的是语义结果而非编译阶段契约。

**但这些资产有一个共同缺陷**：它们测的是"输入 → 输出"，不测"中间表示是否正确"。IR 层的正确性目前**完全没有判据**。

## 目标设计

### 核心原则：判据按重构类别分级

**这是本文 最重要的一条。** 用单一判据（通常是"IR 快照全等"）覆盖所有重构类型是错的——因为有些重构**必然改变 IR 形态**。

| 重构类别 | 判据类型 | 强度 | 涉及文档 |
| --- | --- | --- | --- |
| **C1 纯搬移**（拆文件、改目录、提取子模块） | IR 规范化快照 **zero-diff** | 最强 | `02` 的 checker 拆分、`04` 的 lowering 拆分 |
| **C2 编排变更**（阶段化、统一 Driver） | 各入口的**诊断集**相同 + 语料行为相同 | 强 | `02` |
| **C3 类型表示收敛** | 诊断码与消息**相同**（顺序可规范化） | 强 | `03` |
| **C4 IR 形态变更**（SSA 化） | **行为等价**（执行结果相同）+ IR 结构不变量 | 中 | `04` |
| **C5 前端范式变更**（文法驱动） | AST 快照相同 + **诊断 code+span 逐条相同** + 行为相同 | 强 | `05` |
| **C6 纯删除** | 无需等价性判据，只需确认无引用 | — | `06` |

**C1 与 C4 的区别是关键**：C1 要求 IR 逐字节相同（任何变化都是回归）；C4 承认 IR 会变，改用"程序行为相同 + IR 满足不变量"。

> **⚠️ 明确不存在 C5′**
>
> 错误恢复维度的诊断要求**不放宽**：LALRPOP 的错误恢复模型与 `synchronize()` 不同，正因如此，[05](05-frontend-paradigm.md) 的文法**必须用显式错误产生式把 `synchronize()` 的行为建模出来**，使诊断逐条一致。做不到是实现缺陷，**不构成放宽判据的理由**。
>
> 若阶段 3a/3b 实测发现"在 LALRPOP 下无法复现现有诊断"，正确动作是**如实报告该发现并重新评估方案**（含"维持 Pratt"这一选项），**不是修改判据**。

### 三层判据

#### 第一层：IR 静态校验器（覆盖全部类别）

在 `src/middle/ir/verify.rs` 提供 `verify(ir: &ModuleIR) -> Result<(), IrError>`。它比快照强得多，因为它检查的是**语义不变量**而非具体形态。

| 不变量 | 抓什么缺陷 | 典型成因 |
| --- | --- | --- |
| **use-before-def 支配性** | 读的每个 `Operand::Local(n)`，在 CFG 上必须有一条支配当前点的定义路径 | 寄存器错位、restore 遗漏 |
| **值定义唯一性**（仅 SSA 形态下） | 每个 SSA 值有且仅有一个定义点；汇合处必须有 `Phi` | SSA 化自身的正确性 |
| **Phi 一致性** | `Phi` 的输入数量 == 对应前驱基本块数量，且每个输入类型一致 | 遗漏前驱、类型不匹配 |
| **jump 目标存在** | 每条跳转的 label 在 `blocks` 中存在 | `rebase_jump_targets`（`ir_gen.rs:1337`）的平移错误 |
| **全局槽位越界** | `Operand::Global(i)` 的 `i < globals.len()` | 槽位分配错误 |
| **类型一致** | 指令的 `dst` 与 `src` 的 `MonoType` 相容 | 桥接类型表示错误 |
| **内层隔离**（对应 `ir_gen.rs:248-249` 的明文承诺） | 函数 A 的 `cur_locals` 不含函数 B 的临时名 | 6 处 save/restore 契约 |

**校验器必须先于 SSA 化存在**，且要在**当前（非 SSA）IR 上先跑绿**——否则无法用它证明"改造前后等价"。

#### 第二层：规范化 IR 快照（覆盖 C1/C3/C5）

IR 快照必须**规范化**，否则任何寄存器编号变化都会导致全量 diff 失败：

| 规范化项 | 规则 |
| --- | --- |
| 剥离 `Span` | 所有 span 字段置为哨兵值 |
| 临时值重命名 | 按首次出现顺序重命名为 `%0 %1 %2 ...`，**保留定义-使用拓扑顺序** |
| 全局槽位相对化 | 按模块内首次出现顺序重编号 |
| 前驱排序 | `successors` 与 `Phi` 输入按 label 排序 |
| 结构体字段排序 | 仅在字段名已知时排序（构造顺序是语义，不排序） |

**规范化工具的输出必须入库**（`src/middle/core/tests/snapshots/`），随 IR 定义变更显式更新，更新时人工 review diff。

**已知局限（必须写明）**：规范化会抹掉"第 N 个实参用了哪个寄存器"这一信息，因此**第二层单独不足以覆盖缺陷类 2**。C1 阶段的 `arg_regs` 重排安全性必须靠第一层的支配性检查 + 语料行为差分共同保证。

#### 第三层：端到端语料差分（覆盖 C2/C4/C5）

对 `tests/yaoxiang/` 全部 **293 个** `.yx` + `src/std/tests/*.yx`，在改造前后各跑一次，比对：

| 比对对象 | 归一化规则 |
| --- | --- |
| 诊断列表 | 按 `(code, span.file, span.line)` 排序；**消息文本不参与比对**（类型表示收敛会改措辞） |
| 退出码 | 直接比对 |
| 运行时 stdout/stderr | 排序后比对 |
| `dump_bytecode` 输出 | 仅在 C1/C2 阶段比对（字节码会随 IR 变） |

**C4（SSA 化）阶段的行为差分必须覆盖**：ref/borrow/move 三种所有权转移、闭包捕获、柯里化（`generate_curry_*`）、spawn 与迭代器 for、和类型 pattern 匹配、existential 强制点、`?` / Try 传播、方法重载（`overload_resolutions` span 键控）、精化约束的 Drop 序列（`ReleasePlan`）。

**性能基线**：语料差分保证"行为相同"，不保证"耗时相同"。P2 在建立差分基线的同时，用 dev-dependencies 已有的 `criterion` 建 2-3 个冒烟基准（全语料编译耗时 / 典型程序解释吞吐 / CLI 冷启动）并入库，作为后续管线类改动的性能判据——删槽位复用、Phi 展开为 Move 串、Driver 间接层都可能改变耗时，无基线则无人拦截。

### 修复正确性漏洞的专门判据

`[`02`](02-stage-contract.md) 要修的那个漏洞需要**独立判据**，不能混在上面三层里：

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // 同一份含 x: Sorted(3) 的源码，单文件与多文件两条路径的诊断集必须相等
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**这个测试必须先写成失败的**（red），确认它能捕获现有漏洞，再开始修复。如果它意外为绿，则漏洞分析需要重新复核。

同类判据（`[`02`](02-stage-contract.md)` 配套）：

| 测试 | 断言 | 拦住的缺陷 |
| --- | --- | --- |
| `test_program_stage_coverage` | 每个 `Program::stages()` ⊆ `Stage::ALL` 且等于期望数组 | 未来漏接阶段 |
| `test_no_silent_pass_on_unproven` | `ProofResult::Unproven` 在任一模式下必产诊断 | `checker.rs:5179/5318/5448` 的 extend 分支 |
| `test_release_plan_spans_consumed` | ownership 产出的 Span 集合 ⊆ IR 构造消费的 Span 集合 | `ReleasePlan` span 键静默丢 Drop |
| `test_obligations_drained` | 任何 `Obligations` 字段在全仓无消费点时 CI 失败 | 本类缺陷的整个类别 |

### 回归门禁

新增 CI 检查（与 `scripts/ci/` 既有惯例同构）：

| 检查 | 触发 | 失败条件 |
| --- | --- | --- |
| `check-snapshot-drift.sh` | C1/C3/C5 类改动 | 快照文件有 diff 而提交信息未含 `snapshot-update` 标记 |
| `check-ir-verifier.sh` | 全部 IR 改动 | `verify()` 对全语料返回非空 |
| `check-corpus-parity.py` | C2/C4/C5 类改动 | 语料差分非空 |
| `check-stage-contract.py` | 全部 | `Program::stages()` 与 `Stage::ALL` 不匹配 |
| `check-perf-regression.sh` | 触及编译管线或执行路径的改动（P4/P6/P7/P8） | 性能基线回归 >10% 且 PR 无说明 |

### 与各附属文档的绑定

| 文档 | 必须先有 | 判据强度 |
| --- | --- | --- |
| [02-stage-contract.md](02-stage-contract.md)（阶段化） | 本文的漏洞判据（先红） | C2：诊断集相同 |
| [03-type-unification.md](03-type-unification.md)（类型表示） | 第一层校验器的类型一致不变量 | C3：诊断码相同 |
| [04-ssa.md](04-ssa.md)（SSA 化） | **全部三层**，且校验器须在非 SSA 形态上先跑绿 | C4：行为等价 |
| [05-frontend-paradigm.md](05-frontend-paradigm.md)（前端生成器化） | AST 快照 + 诊断判据 | C5：三者全用 |
| [06-cleanup-inventory.md](06-cleanup-inventory.md)（纯删除） | 引用扫描 | 无 |

## 关键决策与理由

**分级而非单一判据。** 用"IR 快照全等"当 SSA 化（C4）的判据会直接失败，从而诱导团队放宽判据；分级让每一步用对它能承受的强度。同时第一层校验器覆盖支配性检查，把"不能测的错误"变成"能测的不变量"——这是快照做不到的。判据资产入库，随代码演进，不是一次性投入。

### 未采纳的方向

- **只用端到端语料差分**——缺陷类 1/2/3 恰好是"程序仍然按某种方式跑完了"的那一类，端到端差分对它们不敏感。SSA 化尤其危险：寄存器跳号可能只影响性能不影响输出，而 span 失配导致的 Drop 丢失**在特定输入下才会显现**，语料库未必覆盖。
- **只用 IR 快照**——快照对缺陷类 2 天生免疫（见「规范化」一节），且 C4 阶段完全失效。
- **只用 property-based testing**（proptest 已在依赖里）——随机程序难以覆盖所有权、existential、精化这些需要特定形状才触发的路径。**但可作为补充**：对第一类（简单表达式/算术）确实比固定语料覆盖更好，建议在语料差分稳定后作为 C4 阶段的补充判据引入。
- **先重构再补测试**——这是本项目已经踩过的坑。[`06-cleanup-inventory.md`](06-cleanup-inventory.md) 记录的 8 棵孤儿测试树（1005 行 / 78 个测试从未运行）就是"先建目录后接线失败"的产物。判据必须先于重构。

## 已知局限与风险

- **第一层校验器需要 SSA 前瞻信息**。在非 SSA 形态下，"唯一定义点"这个概念不存在（`Operand::Local(n)` 指向一个可被多次写的槽位）。因此校验器需要分两个模式：`verify_ssa`（严格，含 Phi 检查）与 `verify_loose`（仅支配性与类型，允许多次定义）。**`verify_loose` 必须在现有 IR 上先跑绿**，这是本设计最大的单项工作量。
- **第二层规范化会漏掉缺陷类 2**。这是已知的局限，只能靠第一层 + 第三层共同覆盖，不能靠加强规范化解决。
- **第三层的语料差分对非确定性输出敏感**。若 `.yx` 语料中存在依赖哈希迭代顺序 / 时间的输出，差分会假失败。需要在建立基线时筛出这类语料。
- **快照入库会带来维护成本**。IR 定义变更时快照要显式更新，需要 review 纪律。
- **语料只覆盖单文件路径**。`tests/` 下无 `yaoxiang.toml`，293 个语料全走单文件路径——详见「现有测试资产」的限制说明。

> **本节原列的开放问题已全部裁决。** 逐条决定见 [RFC-039 决议登记](../rfc/draft/039-compiler-architecture.md)（D1–D50）。**本文不留任何待定项。**
>
## 参见

- [RFC-039 编译器功能路由目录设计](../rfc/draft/039-compiler-architecture.md) — 上位总纲
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`，本仓库现有的"逐项往返"判据范例
- `src/middle/core/tests/bytecode.rs:769-814` — 用 `include_str!` 读源码做静态对拍的哨兵测试
- `src/middle/core/tests/bytecode.rs:1073-1131` — 双向差集断言 + 快照表，强制新接入前端时更新
- `ir_gen.rs:248-249` — save/restore 物理相邻契约的明文声明
