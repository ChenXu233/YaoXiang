# 多级施工任务表（WBS）

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档，把 RFC 的阶段序列展开为**可独立提交、可独立验证**的三级任务。
>
> 阶段顺序与验收门禁以 RFC-039 正文为唯一权威；本文只负责拆解到可执行粒度，并登记尚未裁决的冲突。

## 定位与范围

| 层级 | 数量 | 含义 |
| --- | --- | --- |
| **一级** | 11（P0–P10） | 阶段，与 RFC-039 的阶段序列一一对应 |
| **二级** | 45 | 任务组，来自各篇文档的「实施要点」 |
| **三级** | 122 | 可独立 commit 的动作 |

**本文覆盖**：任务拆解、依赖关系、并行分组、验收判据绑定。

**本文不覆盖**：任务的设计理由（见 `02`–`08`）、阶段顺序的论证（见 RFC-039）。

## 现状：拆解时发现的缺口

在把 8 篇文档的「实施要点」汇总成本表时，发现**文档体系本身有 5 处硬缺口**。全部已收敛：

| # | 缺口 | 收敛结果 |
| --- | --- | --- |
| G1 | **P5 无任何施工步骤**。RFC 定义了 P5（checker 拆分）与验收（C1 zero-diff），但 7 篇文档中没有一节描述这个拆分怎么做；`01-routing.md` 的目录树里出现过 `annotations.rs` 一行，是全批唯一提及 | P5 步骤由本文补齐（见下） |
| G2 | **`include!` 改造三处归属、零处落地**。RFC 列在 P4；`02` 的参见归给自己；`06` 的 S7 也列了。但 `02` 的改动清单（6 新增 + 20 修改）里**没有 `semantic_tokens.rs`** | 施工步骤归本文 §P5 5.1（前置 P4 完成）；RFC 与 06 的引用已同步指向这里 |
| G3 | **CI 脚本三套并存清单**。`RFC` 列 4 个 `scripts/ci/*.py`；`01` 列 4 脚本 + 1 `cargo test`；`07` 另列 4 个检查（命名全不同）；`04` 还要 `check-synth-boundary.py`。去重后**至少 9 个新脚本，无统一清单**；`check-obligations.py` 在 P4 与 P9 双重归属 | 统一清单见本文 §P9（10 个脚本，各归属一个阶段），其他文档一律引用该表 |
| G4 | **`precedence_inline.rs` 生死冲突**。`06` 把它算进「1005 行需复活」，同一份文档的 B6 又说它与 `Precedence` 枚举「必须同时删除」，`05` 主张删除 | **P1 复活、P8 随 Pratt 删除**（决议登记 C1）——先让 CI 重新获得覆盖，删除动作在 P8 的 8.6 随死阶梯一并发生 |
| G5 | **L2 目标表述冲突**。RFC 与本目录索引写"文法驱动语法"，但 `05` 自评"**没有采用真正的文法驱动（LALRPOP），只做到了声明式查表**" | 已收敛为 **LALRPOP 完整文法驱动**（05 §语法：完整文法驱动），G7/G8 验收门禁按该承诺执行 |

## 目标设计：阶段序列

```
P0  仓库维护机制与代码放置规程     08        ← 新增，先立规矩
P1  复活孤儿测试并修缺陷           06 §S1
P2  建立等价性判据基线             07 全部
P3  修正确性漏洞（最小方案）        02 §S3-漏洞半部
P4  阶段契约与统一 Driver          02
P5  checker 文件内拆分             02 + 本文补齐
P6  类型表示单一化                 03
P7  中间表示 SSA 化                04      } 可并行
P8  前端范式变更                   05      }
P9  防反弹门禁（含统一脚本清单）     01 §防反弹 + 08 §可机检规则
P10 其余清理与状态修正             06 §S2/S3/S4/S5/S6
```

**新增的 P0 理由**：三条禁令（不得生造 / 不得无尽填充 / 该重构却打了补丁）约束的是 P1–P10 的每一个动作。规矩必须在动工前立好，否则 P1 复活测试时就已经在往 `tests/mod.rs` 里加 `mod` 声明——那正是 G4 冲突的成因。

## 详细设计：三级任务表

### P0 仓库维护机制与代码放置规程

来源：[08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [ ] **0.1 规程成文**（3 个三级任务；0.1.2 触点 + 0.1.3 规则抽取已于 2026-10-05 提前落地，剩 0.1.1 CONTRIBUTING 正文 review）
- [ ] **0.2 门禁实现**（3 个三级任务）
- [ ] **0.3 表门禁推广**（1 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 0.1 规程成文 | 0.1.1 `CONTRIBUTING.md` 写入 D0–D4 + 三条禁令；职责类别引用 `01-routing.md` 的目录职责表 | — | 人工 review 通过 |
| | 0.1.2 实现者触点落地（**2026-10-05 已完成**）：新建 `AGENTS.md`（根目录入口）+ `compiler-architecture/HOWTO.md`（动工自检 + D3 补丁判定）；PR 模板加「职责归属与决策程序」必填块（D44）；`CONTRIBUTING.md` 加「代码放置与变更规程」节 | — | 已落地；review 核对四触点内容 |
| | 0.1.3 规则本体抽取（**2026-10-05 已完成**）：新建 `docs/src/dev/coding-rules.md`（精确规则描述，长期有效，不随 RFC-039 归档）；08 加迁移头注转为诊断记录；四触点全部改指 coding-rules | — | 已落地 |
| 0.2 门禁实现 | 0.2.1 `check-concepts.py`（禁令一 A/B/C/D） | 0.1.1 | **必须在未改动代码上报出 3 套运算符枚举、2 套平行类型表示（`ir::Type` 别名一并计入）**；本阶段 report-only，P9 收口转硬 |
| | 0.2.2 `check-fanout.py`（禁令三 A/B/C） | 0.1.1 | **故意新增第 6 个入口式接线必须红**；本阶段 report-only，P9 收口转硬 |
| | 0.2.3 `check-boundary.py`（禁令二判据 C） | 0.1.1 | **故意加一处 L2→L3 反向 `use` 必须红**；`pub(crate)` 泄漏计数记初始值 |
| 0.3 表门禁推广 | 0.3.1 `tools/code-tables` 扩展覆盖 opcode / 类型表 | 0.1.1 | 依赖 P6 的 T1；单独收口 |

> **没有规模基线任务**。2026-10-03 决策取消一切行数 / 体积门禁，规模问题由职责分离解决（见 `08` 禁令二判据 A，人工判断）。
> **0.2.1 是本阶段的核心验收**：门禁若报不出已知的平行表示，说明判据设计错了，是假门禁。

### P1 复活孤儿测试并修缺陷

来源：[06](06-cleanup-inventory.md) §实施要点 S1 + [05](05-frontend-paradigm.md) §测试重建

- [ ] **1.1 复活 `lexer/tests/`**（4 个三级任务）
- [ ] **1.2 复活活目录中的 4 处**（1 个三级任务）
- [ ] **1.3 修复暴露的缺陷**（2 个三级任务）

| 二级 | 三级 | 文件:行 | 前置 | 验收 |
| --- | --- | --- | --- | --- |
| 1.1 复活 `lexer/tests/` | 1.1.1 加 `#[cfg(test)] mod tests;` | `lexer/mod.rs:106` 后 | — | 测试数上升 |
| | 1.1.2 补 `mod lexer_mod;` / `mod symbols;` | `lexer/tests/mod.rs:15-25` | 1.1.1 | **不补则 159 行仍不跑** |
| | 1.1.3 删 7 个空壳文件 + 其 `mod` 声明 | `lexer/tests/` 7 文件 | 1.1.1 | C6 无判据 |
| | 1.1.4 删 `pub use ...::*;` 再导出块 | `tests/mod.rs:28-38` | 1.1.3 | 该块正是掩盖空壳的原因 |
| 1.2 复活活目录中的 4 处 | 1.2.1–1.2.4 分别接 `pratt/tests/precedence_inline.rs`、`passes/tests/overload_inline.rs`、`emitter/tests/json.rs`、`package/template/tests/` | 各父模块 | — | 测试数上升；**1.2.1 与 8.0.1 冲突，见冲突登记 C1** |
| 1.3 修复暴露的缺陷 | 1.3.1 四个基数扫描器溢出路径 | `literals.rs` | 1.1.x | `cargo test` 绿 |
| | 1.3.2 `\x`/`\u` 非法转义 + `scan_leading_dot` | `literals.rs` | 1.1.x | 同上 |

**验收**：测试数上升；**预期暴露真实缺陷**，需预留修复时间。逐文件 revert，但**修复的缺陷不应回滚**。

### P2 建立等价性判据基线

来源：[07](07-equivalence-oracle.md) 全部（该篇无「实施要点」节，按结构反推）

- [ ] **2.1 IR 静态校验器**（3 个三级任务）
- [ ] **2.2 规范化快照**（2 个三级任务）
- [ ] **2.3 语料差分**（3 个三级任务）
- [ ] **2.4 漏洞专门判据**（3 个三级任务）
- [ ] **2.5 回归门禁**（1 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 2.1 IR 静态校验器 | 2.1.1 `verify.rs` 双模式 `verify_loose` / `verify_ssa` | — | C4 |
| | 2.1.2 **`verify_loose` 在现有非 SSA IR 上跑绿** | 2.1.1 | **硬门槛，P7 批 a 的准入条件** |
| | 2.1.3 实现 7 项不变量（支配性/唯一定义/Phi一致/jump目标/全局越界/类型一致/内层隔离） | 2.1.1 | C4 |
| 2.2 规范化快照 | 2.2.1 规范化工具（剥 Span / 临时值重命名 / 槽位相对化 / 前驱排序） | — | C1/C3/C5 |
| | 2.2.2 快照入库 + 人工 review 流程（`src/middle/core/tests/snapshots/`） | 2.2.1 | C1/C3/C5 |
| 2.3 语料差分 | 2.3.1 差分框架 + 293 语料基线 | — | 诊断/退出码/stdout 逐项 |
| | 2.3.2 **多文件语料层**（新建 `tests/yaoxiang-multifile/`，带 `yaoxiang.toml` 的项目夹具——决议 D48） | — | **无条件必做（D40），是 P4 唯一可执行的行为判据来源** |
| | 2.3.3 C4 行为差分必覆盖清单（10 类语义）+ 性能基线（criterion 冒烟基准） | — | C2/C4/C5；性能基线供 P4/P7/P8 对比 |
| 2.4 漏洞专门判据 | 2.4.1 `test_multifile_proof_obligation_not_dropped` | 2.3.2 | **必须先为红** |
| | 2.4.3 `test_no_silent_pass_on_unproven` | — | 拦 `checker.rs:5179/5318/5448` |
| | 2.4.4 `test_release_plan_spans_consumed` | — | **无白名单，差集必须为空（D41）** |
| 2.5 回归门禁 | 2.5.1–2.5.4 见 P9 统一清单 | 2.1–2.3 | 见 P9 |

> 原 2.4.2（`test_program_stage_coverage`）与 2.4.5（`test_obligations_drained` 骨架）**移出 P2**：两者引用的 `Program` / `Obligations` 类型在 P4 才存在，P2 无法编译。前者即 P4 的 4.1.4（勿双重登记），后者的 `#[ignore]` 红骨架并入 4.3.1。

### P3 修正确性漏洞（最小方案）

来源：[02](02-stage-contract.md) §实施要点 S3 的漏洞修复半部

- [ ] **3.1 `proof_calls` 消费端**（2 个三级任务）
- [ ] **3.2 第二个静默丢弃点**（1 个三级任务）
- [ ] **3.3 panic 转诊断**（1 个三级任务）

| 二级 | 三级 | 文件:行 | 前置 | 验收 |
| --- | --- | --- | --- | --- |
| 3.1 `proof_calls` 消费端 | 3.1.1 `proof_calls` 收成私有 + 唯一 getter | `types.rs:29` | 2.4.1 | 漏洞判据转绿 |
| | 3.1.2 三个消费点补齐 | `orchestrator.rs:273` / `:450` / `:1374` | 3.1.1 | C2 |
| 3.2 第二个静默丢弃点 | 3.2.1 `Unproven` 空 match 臂产出诊断 | `checker.rs:1303-1314` | 2.4.3 | `test_no_silent_pass_on_unproven` |
| 3.3 panic 转诊断 | 3.3.1 `.expect()` → `SMTResult::Unknown` + 诊断 | `predicate.rs:34-36` | — | Z3 缺失下不再 panic |

**验收**：漏洞判据转绿，且**故意撤掉修复必须重新变红**。**风险**：多文件路径新增 E4018 是破坏性变更。

> **止血通道（2026-10-05 裁决）**：P3 不必等 P2 全部完成。3.1/3.2 的唯一硬前置是 **2.3.2（多文件语料层）+ 2.4.1/2.4.3（两个红判据）**；2.1/2.2/2.3.1/2.3.3（IR 校验器、快照、单文件差分、性能基线）可与 P3 并行推进。正确性漏洞（`Sorted(3)` 静默通过）的止血不应被快照基础设施建设阻塞。3.3（panic 转诊断）无前置，可随时落地。

### P4 阶段契约与统一 Driver

来源：[02](02-stage-contract.md) §实施要点 S1–S5。**本阶段与 P3 的修复必须分两个 commit。**

- [ ] **4.1 声明式阶段表**（5 个三级任务）
- [ ] **4.2 入口合并**（8 个三级任务）
- [ ] **4.3 义务账本**（2 个三级任务）
- [ ] **4.4 证明层与 wasm 收尾**（3 个三级任务）
- [ ] **4.5 跨层契约 PlanId 化（D20）**（2 个三级任务）
- [ ] **4.6 LSP 语义数据管线统一（D55）**（1 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 4.1 声明式阶段表 | 4.1.1 `stage.rs`（`Stage` 变体 + `Stage::ALL` + `StageScope`） | P0 | `test_program_stage_coverage` |
| | 4.1.2 `program.rs` / `unit.rs` | 4.1.1 | 同上 |
| | 4.1.3 `Pipeline::run` 改走 Driver（`pipeline.rs:141-227`） | 4.1.2 | **单文件诊断集与退出码逐字节相同** |
| | 4.1.4 阶段覆盖断言 | 4.1.3 | C2 |
| | 4.1.5 全语料 zero-diff | 4.1.4 | 诊断/退出码/stdout 全 zero-diff（293 语料） |
| 4.2 入口合并 | 4.2.1 `compile_project` 瘦身为 Program 构造器（`orchestrator.rs:99-237`） | 4.1.5 | 四入口诊断集归一后相同 |
| | 4.2.2 `check_project`（`:273-399`） | 4.2.1 | 同上 |
| | 4.2.3 `check_source_in_project`（`:450`） | 4.2.2 | 同上 |
| | 4.2.4 `compile_embedded_module`（`:1374`） | 4.2.3 | 同上 |
| | 4.2.5 删 `check_single_file`（`diagnostic/mod.rs:621-661`） | 4.2.1 | `check` 项目内外一致 |
| | 4.2.6 删 LSP 手工阶段序列（`lsp/handlers/diagnostics.rs:161-227`） | 4.2.1 | LSP 与 CLI 诊断集相同 |
| | 4.2.7 `Aggregation` 参数驱动二选一（`orchestrator.rs:486-494`） | 4.2.1 | **实施时逐行核实两函数内部差异**（冲突登记 C5 已裁决为必做核实项） |
| | 4.2.8 wasm 改走 `ProgramKind::WasmPlayground`（`wasm/src/lib.rs:30-36,42-51`） | 4.2.1 | 见 C6 |
| 4.3 义务账本 | 4.3.1 `obligations.rs` + `assert_drained()`（新建；含 `#[ignore]` 红骨架先行） | 4.1.3 | `test_obligations_drained` 转绿 |
| | 4.3.2 义务诊断 W→E 升级 | 4.3.1 | 人工 review 每处新增 E |
| 4.4 证明层与 wasm 收尾 | 4.4.1 `layers/README.md` 层序改实际顺序 | — | **独立于 4.1–4.3 走** |
| | 4.4.2 `default_solver()` 改 `&'static` 单例（`proof/smt/backend.rs:67-72`） | — | 缓存命中率可观测 |
| | 4.4.3 降级路径加 warning（`checker.rs:1283-1293`） | — | — |
| 4.5 跨层契约 PlanId 化（D20） | 4.5.1 `ReleasePlan` 键 `Span` → `PlanId`（`layers/ownership.rs:31` 产出侧分配 + `ir_gen.rs:1942` 消费侧匹配） | 4.3.1 | `test_release_plan_spans_consumed` 差集为空（D41） |
| | 4.5.2 `overload_resolutions` 键 `Span` → `PlanId` | 4.5.1 | C2；span 失配类静默失效归零 |
| 4.6 LSP 语义数据管线统一 | 4.6.1 项目内文件的语义数据（SemanticDB）走 orchestrator 同管线，与诊断同源；跨文件引用带 `resolves_to` | 4.2.1 | 项目内跨文件跳转命中定义；语义数据与诊断出自同一次编译 |

### P5 checker 文件内拆分 —— 本文档补齐（原缺口 G1）

来源：RFC 定义 + `01-routing.md` 目录树 + 硬约束（`collect_used_in_type` 的 `pub(crate)` 路径不得变）

- [ ] **5.1 `include!` → 真 `mod`**（2 个三级任务）
- [ ] **5.2 拆出 `refinement`**（2 个三级任务）
- [ ] **5.3 拆出 `annotations`**（2 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 5.1 `include!` → 真 `mod` | 5.1.1 `checker.rs:5618` 的 `include!` 改为真 `mod` 声明 | P4 完成 | C1 zero-diff；`semantic_tokens.rs` 获得模块身份 |
| | 5.1.2 补 `use super::*` 或逐项引入（`include!` 时代隐式继承作用域） | 5.1.1 | `cargo build` 绿 |
| 5.2 拆出 `refinement` | 5.2.1 迁出精化块（现 `checker.rs` 3862–5452 + 5469–5617，含 `ReturnRefinement` / `RefinedWalkCtx`） | 5.1.1 | C1 zero-diff |
| | 5.2.2 `collect_refined_binding_checks` 签名改造：`&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)` | 5.2.1 | C1；调用点仅 `1235` |
| 5.3 拆出 `annotations` | 5.3.1 迁出注解校验（现 `3441-3661`，含 `is_predicate_head`） | 5.2.2 | C1 zero-diff |
| | 5.3.2 **`collect_used_in_type`（`3607`，`pub(crate)`）保持 `pub(crate)` 且在 `checker` 模块路径下** → `checker.rs` 顶部加 `pub(crate) use annotations::*;`，使 `inference/statements.rs:16` 的 import **一字不改** | 5.3.1 | **零调用方改动**是本步的硬验收 |

**硬约束**：`collect_used_in_type` 被 `inference/statements.rs:16` 跨模块使用。做法是 `pub(crate) use` 再导出，让拆分成为**纯搬迁**。

**只拆确证的大块。** 不拆只有 200–400 行的模块——增加导航成本、降低内聚性、零解耦收益。其余子模块（signatures / type_defs / imports）待文件长回 1,500 行再拆。

### P6 类型表示单一化

来源：[03](03-type-unification.md) §实施要点 阶段 0–6，判据 C3。**阶段 0→6 严格串行。**

- [ ] **6.0 门禁先行**（2 个三级任务）
- [ ] **6.1 名称规范化**（3 个三级任务）
- [ ] **6.2 字节码类型统一**（3 个三级任务）
- [ ] **6.3 死变体删除**（2 个三级任务）
- [ ] **6.4 parser 数据流**（5 个三级任务）
- [ ] **6.5 AST 死变体**（2 个三级任务）
- [ ] **6.6 门禁转硬**（1 个三级任务）
- [ ] **6.7 目录改名（D1，纯搬移批次）**（1 个三级任务）
- [ ] **6.8 标注检查模式与 Fn 参数表示（D54，2026-10-05 增补）**（2 个三级任务）

| 二级 | 三级 | 文件:行 | 验收 |
| --- | --- | --- | --- |
| 6.0 门禁先行 | 6.0.1 新建 `tools/type-tables`，实现 T1/T2/T3，**T1 先只报告不失败** | 新 crate | T1 准确列出 13 个前向零构造变体 **+ 反向桥重建点分列** |
| | 6.0.2 在未改动代码上取全量基线 | — | 无代码改动 |
| 6.1 名称规范化 | 6.1.1 统计语料非规范类型名出现次数 | `tests/yaoxiang/` | 决定能否直接删同义词 |
| | 6.1.2 `from_builtin_name` 砍同义词 → lexer 别名表 | `mono.rs:618-643` | C3 |
| | 6.1.3 核实 `classify_generic_params` 消费哪个参数列表 | `types.rs:162-165` | **必须逐行核实，不能假定等价** |
| 6.2 字节码类型统一 | 6.2.1 三处改 `MonoType` | `bytecode.rs:812/814/854`、`image.rs:44` | C3 + `dump_bytecode` 定向对拍 |
| | 6.2.2 删 `From<MonoType> for IrType` | `bytecode.rs:2352-2390` | 同上 |
| | 6.2.3 `type_table` 元素类型改 `MonoType` | `image.rs:44` | 同上 |
| 6.3 死变体删除 | 6.3.1 删 11 个变体 + 清 match 臂 + 兜底 | `ast.rs`、`mono.rs:699-708`、`types.rs:836` | C3 + `cargo build` 绿 |
| | 6.3.2 **必须与 6.3.1 同批**改写反向桥 | `passes/mono/function.rs:507-529`（`mono_to_ast_type` 重建其中 7 个） | 泛型替换定向对拍，否则**静默改行为** |
| 6.4 parser 数据流 | 6.4.1 新增 `probe.rs` | 新建 | C3 |
| | 6.4.2 改 `parser_state.rs:46-58` | — | 同上 |
| | 6.4.3 删 `operator_interfaces::spec()` 调用（**2 处**：`declarations.rs:509`、`ast.rs:930`） | — | 同上 |
| | 6.4.4 合并两份 `name_used_as_type*` | `ast.rs:847` + `declarations.rs:42` | 同上 |
| | 6.4.5 `CONST_PARAM_TYPES`（`ast.rs:912`）经 `NameKind::Builtin` 接线 | — | 同上 |
| 6.5 AST 死变体 | 6.5.1 删 `Expr::FnDef`（16 处引用） | `ast.rs:37-43` + 4 处消费 + 2 处穷举臂 | `tests/integration/` 18 模块绿（**分 2 commit**） |
| | 6.5.2 删 `Assign.signature_params` + `NamedParen` 迁移 | `ast.rs:241-249` | 标注**语义风险** |
| 6.6 门禁转硬 | 6.6.1 T1 由 warning 改 `panic!` | `tools/type-tables` | **故意引入零构造变体必须红** |
| 6.7 目录改名（D1，纯搬移批次） | 6.7.1 `typecheck/`→`sema/`；`middle/core/`→`middle/ir/`；`parser/ast.rs` 拆迁顶层 `ast/`；字节码域合并（两个 `bytecode.rs` → `bytecode/`）+ `opcode.rs` 迁 `middle/bytecode/` | 全仓 | **C1 快照 zero-diff**；独占 commit；越界 `use` 基线不回退 |
| 6.8 标注检查模式与 Fn 参数表示（D54） | 6.8.1 `Type::Fn.params: Vec<Type>` → `Vec<Param>`（名字与类型同栖），与 6.5.2 删 `Assign.signature_params` 同批收口——6.5.2 只删旧表示，本项给出替代表示 | `ast.rs:451-454`、消费面约 20 文件（机械改） | C3（纯表示重塑批，对照未启用）；分独立 commit |
| | 6.8.2 声明驱动的检查模式：参数类型按位驱动 lambda 头（与名字无关）、出口（尾表达式与 return）统一对照标注返回类型、非 lambda 值 infer+unify（与 `m: Int = "hello"` 同构）；红判据先行——u1–u4/v1–v5 探针入语料（先红后绿）；删 `declarations.rs:337-376` parser 名字合并 | `statements.rs:1170-1914` | 对照启用批为**行为修复**：新诊断码、基线更新，不适用 C3；v1–v5 五种语法形态同一判定（消灭「同语义不同语法一红一绿」） |

**硬验收**：`git diff --stat tests/ src/std/` 必须为空。

### P7 中间表示 SSA 化

来源：[04](04-ssa.md) §实施要点 批 a–d，判据 C4，**批内严格串行**。

- [ ] **7a 切断多次定义**（3 个三级任务）
- [ ] **7b SSA 形态切换**（6 个三级任务；7b.1 已按 DoD 6 拆为 1a/1b）
- [ ] **7c 隐式契约显式化**（3 个三级任务）
- [ ] **7d 最后做**（2 个三级任务）
- [ ] **7e `.42` 数据丢失修复**（3 个三级任务；2315/2341 由"独立 issue"收编，2026-10-05）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 7a 切断多次定义 | 7a.1 删 3 处语句级回收 | **2.1.2 `verify_loose` 跑绿** | C4 |
| | 7a.2 6 处 save/restore 换 RAII guard | 7a.1 | C1 快照 zero-diff |
| | 7a.3 口径统一（`1798` vs `2001`） | 7a.2 | C1 |
| 7b SSA 形态切换 | 7b.1a `ir.rs` 结构变更 #1-2（`Operand` 变体收敛 + `Instruction::Phi` 引入） | 7a.3 + **P6 完成** | C4；独占 commit |
| | 7b.1b `ir.rs` 结构变更 #3-5（`FunctionIR` / `BasicBlock` / 类型标注字段） | 7b.1a | C4；独占 commit |
| | 7b.2 `next_temp_reg` 改 `Operand::Value` | 7b.1 | 同上 |
| | 7b.3 `translator.rs` 加 `Phi` 臂 | 7b.1 | C4 + `.42` 往返测试 |
| | 7b.4 线性扫描分配器（u8/255 槽模型内） | 7b.3 | 同上（+500~1000 行） |
| 7c 隐式契约显式化 | 7c.1 `synth.rs` 边界 + 边界检查脚本 | 7b.4 | C4 + 新脚本入 CI |
| | 7c.2 span 消费计数 | 7c.1 | 消费计数**零触发** |
| | 7c.3 `method_def_ordinals` 只读化 | 7c.1 | 归属存疑（C7） |
| 7d 最后做 | 7d.1 `generate_call_expr_ir` 拆为 `CallArgs` + 6 个 `emit_*` | 7c.x | C1 快照 zero-diff |
| | 7d.2 「补 0」兜底改返回诊断 | 7d.1 | C4 |
| 7e `.42` 数据丢失修复 | 7e.1 `upvalue_count: 0` 修正（`bytecode.rs:2312`）+ `VERSION` 4→5（D17）+ 已分配未使用 opcode 回收（D32/D34 随升版） | 7d.x | C4 + `.42` 往返测试；独占 commit |
| | 7e.2 `exception_handlers` 落盘（`bytecode.rs:2315`——throw/try 是语言核心语义，**不得留待实现**） | 7e.1 | `.42` 直跑异常用例差分 |
| | 7e.3 `globals` 落盘（`bytecode.rs:2341`） | 7e.1 | `.42` 直跑全局变量用例差分 |

**预期管理**：行数**净增 900–1600 行**。若立项时以"代码变短"为成功标准，本阶段会被判定为失败。

### P8 前端范式变更

来源：[05](05-frontend-paradigm.md) §实施要点 阶段 0–4，判据 C5。

- [ ] **8.0 死阶梯清理（C6）**（2 个三级任务）
- [ ] **8.2 词法收敛**（2 个三级任务）
- [ ] **8.3 建 LALRPOP 文法**（3 个三级任务）
- [ ] **8.4 双解析器差分**（1 个三级任务）
- [ ] **8.5 切流**（1 个三级任务）
- [ ] **8.6 删 Pratt**（2 个三级任务）
- [ ] **8.7 `parse_assign_after_target` 拆分**（1 个三级任务）
- [ ] **8.8 测试补全**（3 个三级任务）
- [ ] **8.9 参数位形态归一（D53，2026-10-05 增补）**（1 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 8.0 死阶梯清理（C6） | 8.0.1 删 `Precedence` 枚举 + `PrecedenceContext`（96 行） | **与 1.2.1 冲突，见 C1** | C6 无判据 |
| | 8.0.2 裸魔数具名化（`(6,7)` / `(11,1)` / `12`） | — | 同上 |
| 8.2 词法收敛 | 8.2.1 四基数扫描器合并 + 三处转义合并 + 多行串合并 | — | C5；`literals.rs` 减约 500 行 |
| | 8.2.2 f-string 嵌套编译消除 | 8.2.1 | **span 必然变化**，须先建基线 |
| 8.3 建 LALRPOP 文法 | 8.3.1 文法文件 `grammar/yaoxiang.lalrpop` | 8.2.x | `cargo build` 通过；对 293 语料能产 AST（**不比对**） |
| | 8.3.2 动作代码 `grammar/actions.rs`（22 个 `Expr` 变体各一函数） | 8.3.1 | 同上 |
| | 8.3.3 错误产生式（保 `Expr::Error` / `StmtKind::Error` 占位语义） | 8.3.2 | 同上 |
| 8.4 **双解析器差分** | 8.4.1 两套 parser 各跑 293 语料 + `src/std/tests`，AST 规范化后**逐位比对** | 8.3.3 | **合法程序 AST 全等价——这是整个文法迁移的等价性证明** |
| 8.5 切流 | 8.5.1 `parse()` 改调 LALRPOP，Pratt 留为 `parse_legacy()` | 8.4.1 | 全量语料行为等价 + 诊断 code+span 逐条相同（**不放宽**，不存在 C5′） |
| 8.6 删 Pratt | 8.6.1 删 `nud.rs`(1326) + `led.rs`(451) + 两套 BP 阶梯 + 约 89 处引用 | 8.5.1 | 语料全绿；`git grep BP_` 零命中 |
| | 8.6.2 删 `is_old_function_syntax`（36 行，自然失效） | 8.6.1 | 同上 |
| 8.7 `parse_assign_after_target` 拆分 | 8.7.1 拆 8 个职责段落 + 抽出 `skip_balanced_parens` | 8.6.x | 该函数只做派发 |
| 8.8 测试补全 | 8.8.1 新增结合性用例（**先红后绿**） | 8.2.x | C5 |
| | 8.8.2 补四类字面量错误路径 | — | 覆盖未测路径 |
| | 8.8.3 补 `pratt/tests/mod.rs` 接线自检断言 | — | 防孤儿复发（**`pratt/tests/` 在 8.6 后已不存在，改为 `parser/tests/`**） |
| 8.9 参数位形态归一（D53） | 8.9.1 文法产生式 `Param ::= Identifier ':' TypeExpr \| TypeExpr`：裸标识符按类型命名空间解析、解析不到报 E；无名签名要求 lambda 头自带参数名（RFC-007:47 既有规则）；RFC-010 形态表与接口示例（`(Surface)`）随批对账 | 8.3.2 + 6.8.1 | 语料普查零迁移（std/语料裸标识符全为真实类型）；未知类型标识符新 E 码 |

**风险**：8.4 双解析器差分是**不可跳过的等价性关口**。若它跑出不等价，必须在 8.5 切流前解决，而不是切流后靠 C5′ 掩盖。8.6 的约 89 处 BP 引用迁移若漏改会**静默改变结合性**（不报错，只改解析结果）。

> **8.3–8.6 依赖 `08` 的 D0/D2**：文法文件若放在 `parser/` 以外、或动作代码反向引用 `sema`，会在 `check-boundary.py` 处被拦。

### P9 防反弹门禁（统一脚本清单 —— 本文档补齐原缺口 G3）

来源：[01](01-routing.md) §防反弹机制 + [08](08-maintenance-mechanism.md) §可机器检查的规则

**这是全部 CI 脚本的唯一权威清单**（10 个脚本）。脚本在「引入阶段」接入 CI；标注 report-only 的在 P9 统一转硬（从"报告"改为"失败"）——存量违规（平行表示、消歧别名等）要等 P6/P8 才清零，提前转硬会让 CI 长期红、门禁被人工绕过，P9 转硬时门禁应当恰好全绿。P2 的 2.5、P4 的 4.3.2、P7 的 7c.1 均引用本表，不另立清单。

- [ ] **9.1–9.10 十个脚本按引入阶段接入，P9 统一转硬**（明细见下表）

| # | 脚本 | 检查什么 | 引入阶段 | 验收 |
| --- | --- | --- | --- | --- |
| 9.1 | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` 双向断言 | P4 | 故意漏一个阶段必须红 |
| 9.2 | `check-obligations.py` | 字段出现 ≥2 次（定义 + 写入）但无第三文件读取 | P4（**唯一归属，不再在 P9 重复**） | 故意留一个无消费字段必须红 |
| 9.3 | `check-boundary.py` | 禁 `include!`；`pub(crate)` 跨层泄漏只许减；禁 L2→L3（**唯一归属**，`08` 的 P0 定义了它） | **P0** | 故意加 `include!` 必须红 |
| 9.4 | `check-test-wiring.py` | 有 `tests/` 目录但父模块无 `mod tests;` → 失败 | P1 | 故意造孤儿目录必须红 |
| 9.5 | `check-snapshot-drift.sh` | 快照有 diff 而提交信息无 `snapshot-update` 标记 | P2 | 同上 |
| 9.6 | `check-ir-verifier.sh` | 全语料 `verify_loose` 非空 | P2 | 同上 |
| 9.7 | `check-corpus-parity.py` | 语料差分非空（诊断 code+span 逐条比对，**无任何分组放宽**） | P2 | 同上 |
| 9.8 | `check-synth-boundary.py` | `synth.rs` 之外出现 `ast::Expr` 构造 | P7 | 同上 |
| 9.9 | `check-concepts.py` | 禁令一 A/B/C/D（平行表示、消歧别名、同义词表） | **P0**（report-only，P9 转硬） | 必须在未改动代码上报出 3 套运算符枚举 |
| 9.10 | `check-fanout.py` | 禁令三 A/B/C | **P0** | 故意新增第 6 个入口式接线必须红 |

> **没有行数 ratchet 脚本**（`check-file-size.py` + `baseline.toml` 已于 2026-10-03 取消）。规模问题由 `08` 禁令二判据 A（职责分离，人工判断）解决。

### P10 其余清理与状态修正

来源：[06](06-cleanup-inventory.md) §实施要点

- [ ] **S2 删纯占位死代码**（1 个三级任务）
- [ ] **S3 空头设计文档**（1 个三级任务）
- [ ] **S4 RFC 状态修正**（1 个三级任务）
- [ ] **S5 `pub` 项降可见性**（1 个三级任务）
- [ ] **S6 opcode 死路径清理**（1 个三级任务）

| 二级 | 三级 | 验收 |
| --- | --- | --- |
| S2 删纯占位死代码 | `instance.rs:416-830`（415 行）/ dispatch 链 / 死阶梯 / 旧语法探测 / `pipeline/tests`（14 行）/ `undefined/` 空目录；**E2：`lib.rs:568` / `:582` 两处 `_ => todo!()` 改有意义的降级输出** | `cargo test` 通过；`clippy -D warnings` 无新增 |
| S3 空头设计文档 | C1+C2 删除后重写为指向 `src/frontend/module/`（约 2,670 行）；C3 改写边界表；**同步清理 `config.js:259-268`** | 文档站无 404；`check-docs-truth.py` 通过 |
| S4 RFC 状态修正 | C4 RFC-018 移回 `draft/`（D46）；C5 RFC-028 补 `impl_status: 'not-started'`；F3 `TRACKING.md` 加「实现状态」列（D36） | `check_tracking.py` 退出码 0（**不手改 TRACKING.md**） |
| S5 `pub` 项降可见性 | B2 `TypeSystem` / B4 `check_type_equivalence`（**`is_subtype` 必须保留**） | 编译器能指出真正的死代码 |
| S6 opcode 死路径清理 | B8 `TailCall` 与 B9 `Switch`（D32/D34：删，opcode 值回收已在 7e.1 随升版落地）；B10 `UnaryOp::Not`（D33：`opcode()` 区分 `op` 字段）；**F7+F8 opcode 生成期门禁**（`tools/code-tables` 扩展 opcode 提取器约 30 行 + 5 处事实表达单源化 + `size()` 全 opcode 对拍测试） | opcode 事实 5 处表达收敛为 1 处权威源；`size()` 注释与实际编码不符（`bytecode.rs:2181-2182`）修复 |

## 实施要点：依赖与并行

### 强制串行主链

`P0 → P1 → P2 → P3 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 止血通道（2026-10-05 裁决）**：P3 的前置收窄为 **2.3.2 + 2.4.1 + 2.4.3**（多文件语料层 + 两个红判据）。P2 的其余部分（2.1 IR 校验器、2.2 快照、2.3.1/2.3.3 差分与性能基线）可与 P3 并行推进。正确性漏洞的止血不被判据基础设施建设阻塞；但 **P4 仍必须等 P2 全部完成**——统一 Driver 需要三层判据全部就位。

| 约束 | 原因 |
| --- | --- |
| P0 → P1 | 三条禁令约束 P1 的每一个动作（1.2.x 就是在往 `mod.rs` 加 `mod` 声明） |
| P1 → P2 | 复活测试会改变测试数量与语料基线 |
| P2（收窄后）→ P3 | 漏洞判据必须**先写成红的**（2.4.1/2.4.3）+ 多文件语料层（2.3.2）存在，红测试才有地方跑 |
| P3 → P4 | 先修 bug 再重构；反序会让 bug 被阶段表固化成"既定行为" |
| P4 → P5 | `include!` 改造与 checker 拆分动同一文件 |
| P5 → P6 | 同一文件的连续改动必须分开，否则回归无法二分定位 |
| P6 → P7 / P8 | 类型表示不先收敛，新 IR 会长成第三套表示 |
| P7/P8 → P9 | 9.5 的快照基线与 9.3 的 `pub(crate)` 泄漏计数在 P7/P8 之后才取最终形态 |

### 可并行分组

| 组 | 成员 | 依据 |
| --- | --- | --- |
| **A（唯一被 RFC 授权）** | P7 ∥ P8 | 文件集不相交：`ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*` |
| B | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4 | 各自独立补 `mod`，逐文件 revert |
| C | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4 | 三层判据彼此独立 |
| D | 4.4（证明层收尾） ∥ P6 整条线 | `02` 明示 S5「应独立于 S1-S4 走」 |
| E | 4.2.5 ∥ 4.2.6 ∥ 4.2.8 | 均在 4.1 完成后互不依赖 |
| F | 9.3 ∥ 9.9 ∥ 9.10（P0 引入的三个脚本可并行开发） | 9.5 的快照基线必须最后取 |
| G | S3 ∥ S4 ∥ S5 | 文件集不相交（已核实 2026-10-05：S3 动 `docs/src/dev/design/check/` + `config.js`；S4 动 `docs/src/rfc/` + `check_tracking.py` 生成器；S5 动 `src/frontend/core/typecheck/` 两处 `pub`；两两无交集） |

**不可并行（文件重叠）**：P1(1.1.x) ∥ P8(8.0.x)；1.2.1 ∥ 8.3.2；6.0 ∥ S2（B6/E5 同批文件）；4.3.2 ∥ 9.2（已统一到 P4）。

**文档内部新增约束**：6.0→6.1→6.2→6.3→6.4→6.5→6.6 严格串行；7a→7b→7c→7d 严格串行，且 7b 的 `Phi` 臂必须先于 7d。

## 关键决策与理由

- **P0 独立成阶段而非并入 P9**——门禁是"事后检查"，规程是"事前判断"。P9 的脚本无法阻止"该重构却打了补丁"，只有 D0–D4 的 review 清单能。
- **P9 作为 CI 脚本的唯一权威清单**——原缺口 G3 的三套并存清单导致无法收口。本表合并后 10 个脚本各归属一个阶段，report-only 与转硬时点显式可查。
- **P5 的步骤由本文补齐而非并入 P4**——P4 已是最大单点风险，再叠加 checker 拆分会超出可回滚粒度。
- **5.3.2 以"零调用方改动"为硬验收**——拆分必须能独立 revert。

## 已知局限与风险

- **1.3.x（修复暴露的缺陷）无回归判据**——只说"修复的缺陷不应回滚"，缺"修的是什么"的记录要求。**执行约定**：每个 1.3.x 修复的 commit message 必须引用被修复测试名与缺陷描述，作为事后二分依据。
- ~~**7b.1 粒度过粗**~~ **已修（2026-10-05）**：按 DoD 6 拆为 7b.1a / 7b.1b 两个独占 commit。
- **8.2.1 粒度过粗**——约 89 处改名引用，漏改**静默改变结合性**。
- ~~**D17 / D20 无对应三级任务**~~ **已修（2026-10-05）**：D17 落地为 7e.1，D20 落地为 4.5.1/4.5.2；D32/D34 的 opcode 回收并入 7e.1，删除动作留 P10 S6。
- ~~**`2315` / `2341` 数据丢失缺陷原定"独立 issue"**~~ **已修（2026-10-05）**：收编为 7e.2 / 7e.3。异常表（throw/try）是语言核心语义，不得留待实现遗留。
- ~~**E2（`lib.rs:568/582` `todo!()`）无阶段归属**~~ **已修（2026-10-05）**：并入 P10 S2。
- ~~**F7（opcode 门禁提取器）归属含糊**~~ **已修（2026-10-05）**：并入 P10 S6 的生成期门禁任务。

## 冲突登记：已全部裁决

> **原「冲突登记」C1–C8 与各篇的开放问题已全部收敛到 [RFC-039 决议登记](../../rfc/accepted/039-compiler-architecture.md)（D1–D50）。** 本文不留任何待定项。逐条对应：

| 原编号 | 议题 | 裁决 |
| --- | --- | --- |
| C1 | `precedence_inline.rs` 在 P1 复活、P8 删除 | **复活**（P1 的前提是复活有真实断言的测试），P8 的 8.6 随 Pratt 一并删除该文件 |
| C2 | "文法驱动"承诺 vs 查表化 | **已做完整**：采用 LALRPOP，见 `05`「语法：完整文法驱动」 |
| C3 | C6 类别定义与 S1 归类冲突 | **S1 归入 C6**（复活不需要等价性判据，只需回归测试）；C6 定义扩为"纯删除或纯复活" |
| C4 | `test_release_plan_spans_consumed` 白名单 | **无白名单**，差集必须为空；空不了说明 `ReleasePlan` 契约有缺陷 → 改 `PlanId`（D20/D41） |
| C5 | `Aggregation` 两函数内部差异未核实 | **P4 的 4.2.7 已把它列为必做核实项**，实施时逐行确认 |
| C6 | wasm playground 修复需要 P4 的 `ProgramKind` | **wasm 路径的漏洞修复并入 4.2.8**；P3 只处理三个 native 入口 |
| C7 | `method_def_ordinals` 归属 | **归 04**（P7 批 c），见 D21 |
| C8 | `upvalue_count` 修正与 `.42` 版本号 | **修 + `VERSION` 从 4 升到 5**（格式头已有 `MAGIC` + `VERSION` 字段，`codegen/bytecode.rs:14-16`），见 D17 |

## 参见

- [RFC-039 正文](../../rfc/accepted/039-compiler-architecture.md) — 阶段序列、DoD、全局验收门禁 G1–G10 的唯一权威
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 判据类别定义
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0 的详细设计与 D0–D4 决策程序
- [01-routing.md](01-routing.md) — 目标目录结构（任务完成后的形态）
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) / [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — 各阶段的设计依据
