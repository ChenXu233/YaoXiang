# 多级施工任务表（WBS）

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档，把 RFC 的阶段序列展开为**可独立提交、可独立验证**的三级任务。
>
> 阶段顺序与验收门禁以 RFC-039 正文为唯一权威；本文只负责拆解到可执行粒度，并登记尚未裁决的冲突。

## 定位与范围

| 层级 | 数量 | 含义 |
| --- | --- | --- |
| **一级** | 11（P0–P10） | 阶段，与 RFC-039 的阶段序列一一对应 |
| **二级** | 73 | 任务组：按阶段拆解的工作组，可独立建队、独立归档 |
| **三级** | 264 | 可独立提交、可独立验证的工作单元 |

**编号留空（不回收）**：P4 无 `4.7`/`4.8`、P8 无 `8.1`；LALRPOP 路线撤回后 `8.4` / `8.5` / `8.6` 亦留空（2026-11-25）。三者自 WBS 建立起即无对应条目——本文全史对 `**4.7` / `**4.8` / `**8.1` 三个标题零命中（`git log -S'**4.7' -- docs/src/dev/architecture/09-execution-wbs.md` 可复核），全仓亦无引用。`8.1` 对应的正是 `05` 的阶段 1（测试复活与补全）：该阶段已拆入 P1（`1.1`–`1.3`）与 `8.8`，因此 P8 的编号与其阶段号对齐（阶段 0 → `8.0`、阶段 2 → `8.2`，依此类推）。

**本文覆盖**：任务拆解、依赖关系、并行分组、验收判据绑定。

**本文不覆盖**：任务的设计理由（见 `02`–`08`）、阶段顺序的论证（见 RFC-039）。

**登记约定（2026-10-12 起）**：新登记的二级/三级行**必须带两个字段**——「**归属阶段**」（它真正属于哪个 P，可与所在章节不同）与「**是否阻塞主线**」；`登记待排期` 不再作为默认状态，未标注即视为**不可排期**。编号在全文档内**必须唯一**（跨轨道的任务用轨道号，如 `029g.x`，不得占用 `N.x`）。

**收口登记（2026-11-24 起）**：勾选行只写四项——任务 id、状态、**实现提交短号**（7 位）、issue 号（有才写），写成一行：`- [x] **3.24.1 模式匹配补全余项** ✅ 4d608df3（issue #405）`。**不写修法、不写判据链、不写门禁数值、不写文件行号、不写测试名、不写复审结论**：这些在提交信息里（`git show <短号>`），本表是看板，不是过程纪事。
**不写裁决标签，也不写结论复述**：「用户裁决甲／乙／丙」「甲案」「A 案」「最终实现为…」一律不进本表——看板只答「完成没有」，裁决与结果查提交信息与 RFC-039 决议登记。

**短号以推送后为准**：收口行写的是推送完成后 `origin/dev` 上的短号，顺序是**实现提交先推送、收口行后写**（取号规则见 [commit-convention](../commit-convention.md)「提交号在 push 之后取」）。收口提交自身不带号——自指不可能。

**计数栏只写数字**：`定位与范围` 的一/二/三级计数只放当前值；增减流水（`+1 = …`）不进表。

## 现状：拆解时发现的缺口

在把 8 篇文档的「实施要点」汇总成本表时，发现**文档体系本身有 5 处硬缺口**。全部已收敛：

| # | 缺口 | 收敛结果 |
| --- | --- | --- |
| G1 | **P5 无任何施工步骤**。RFC 定义了 P5（checker 拆分）与验收（C1 zero-diff），但 7 篇文档中没有一节描述这个拆分怎么做；`01-routing.md` 的目录树里出现过 `annotations.rs` 一行，是全批唯一提及 | P5 步骤由本文补齐（见下） |
| G2 | **`include!` 改造三处归属、零处落地**。RFC 列在 P4；`02` 的参见归给自己；`06` 的 S7 也列了。但 `02` 的改动清单（6 新增 + 20 修改）里**没有 `semantic_tokens.rs`** | 施工步骤归本文 §P5 5.1（前置 P4 完成）；RFC 与 06 的引用已同步指向这里 |
| G3 | **CI 脚本三套并存清单**。`RFC` 列 4 个 `scripts/ci/*.py`；`01` 列 4 脚本 + 1 `cargo test`；`07` 另列 4 个检查（命名全不同）；`04` 还列过 `check-synth-boundary.py`（该脚本经用户 2026-11-24 取消）。去重后**至少 9 个新脚本，无统一清单**；`check-obligations.py` 在 P4 与 P9 双重归属 | 统一清单见本文 §P9（10 个在用脚本，各归属一个阶段），其他文档一律引用该表 |
| G4 | **`precedence_inline.rs` 生死冲突**。`06` 把它算进「1005 行需复活」，同一份文档的 B6 又说它与 `Precedence` 枚举「必须同时删除」，`05` 主张删除 | **P1 复活、P8 保留**（决议登记 C1；Pratt 不再删除） |
| G5 | **L2 目标表述冲突**：RFC 与本目录索引曾写"文法驱动语法" | 已收敛为**声明式优先级表 + 无二义语言面**（[05](05-frontend-paradigm.md) §语法），见 P8 8.3 |

## 目标设计：阶段序列

```
P0 仓库维护机制与代码放置规程 08 ← 新增，先立规矩
P1 复活孤儿测试并修缺陷 06 §S1
P2 建立等价性判据基线 07 全部
P3 修正确性漏洞（最小方案） 02 §S3-漏洞半部
P4 阶段契约与统一 Driver 02
P5 checker 文件内拆分 02 + 本文补齐
P6 类型表示单一化 03
P7 中间表示 SSA 化 04 } 可并行
P8 前端范式变更 05 }
P9 防反弹门禁（含统一脚本清单） 01 §防反弹 + 08 §可机检规则
P10 其余清理与状态修正 06 §S2/S3/S4/S5/S6
```

**新增的 P0 理由**：三条禁令（不得生造 / 不得无尽填充 / 该重构却打了补丁）约束的是 P1–P10 的每一个动作。规矩必须在动工前立好，否则 P1 复活测试时就已经在往 `tests/mod.rs` 里加 `mod` 声明——那正是 G4 冲突的成因。

## 详细设计：三级任务表

### P0 仓库维护机制与代码放置规程

来源：[08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [x] **0.1 规程成文**（3 个三级任务）
- [ ] **0.2 门禁实现**（6 个三级任务：0.2.1–0.2.3 已完成，report-only 接入 CI；0.2.4 增补，issue [#521](https://github.com/ChenXu233/YaoXiang/issues/521)；0.2.5 准入与退役策略成文；0.2.6 增补，错误码参考页改构建期生成并退役「生成器幂等」门禁；本二级不整行勾完成）
  - [x] **0.2.6 错误码参考页改构建期生成 + 退役「生成器幂等」门禁** ✅ c6ea9f4f
- [ ] **0.3 表门禁推广**（1 个三级任务）
- [ ] **0.4 规程承诺与工具行为一致性**（3 个三级任务：0.4.1 已完成，issue [#463](https://github.com/ChenXu233/YaoXiang/issues/463)；0.4.2 增补；0.4.3 增补，locale 错误码 title 大小写统一（经用户裁决取 A1 机械规范化）；本二级不整行勾完成；归属阶段：P0；是否阻塞主线：否）
- [x] **0.5 测试套件分层**（3 个三级任务；issue [#486](https://github.com/ChenXu233/YaoXiang/issues/486)、[#502](https://github.com/ChenXu233/YaoXiang/issues/502)；归属阶段：P0；是否阻塞主线：否）
- [x] **0.6 检查与测试的策略落地**

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 0.1 规程成文 | 0.1.1 `CONTRIBUTING.md` 写入 D0–D4 + 三条禁令；职责类别引用 `01-routing.md` 的目录职责表 | — | 人工 review 通过 |
| | 0.1.2 实现者触点落地：新建 `AGENTS.md`（根目录入口）+ `compiler-architecture/HOWTO.md`（动工自检 + D3 补丁判定）；PR 模板加「职责归属与决策程序」必填块（D44）；`CONTRIBUTING.md` 加「代码放置与变更规程」节 | — | 已落地；review 核对四触点内容 |
| | 0.1.3 规则本体抽取：新建 `docs/src/dev/coding-rules.md`（精确规则描述，长期有效，不随 RFC-039 归档）；08 加迁移头注转为诊断记录；四触点全部改指 coding-rules | — | 已落地 |
| 0.2 门禁实现 | 0.2.1 `check-concepts.py`（禁令一 A/B/C/D） | 0.1.1 | — |
| | 0.2.2 `check-fanout.py`（禁令三 A/B/C） | 0.1.1 | — |
| | 0.2.3 `check-boundary.py`（禁令二判据 C） | 0.1.1 | **故意加一处 L2→L3 反向 `use` 必须红**；`pub(crate)` 泄漏计数记初始值 |
| | 0.2.4 `check-docs-refs.py` 裸文本覆盖（.md 面；issue [#521](https://github.com/ChenXu233/YaoXiang/issues/521)；来源 = #520 批次复审——裸写探针目录不设防） | 0.2.3 | selftest 全过；全量门禁零违规；负向能力 = 首跑实战抓红；issue [#521](https://github.com/ChenXu233/YaoXiang/issues/521) 的 .md 面验收达成（全仓现有 docs 零假阳性）；归属阶段：P0；是否阻塞主线：否 |
| | 0.2.5 检查准入与退役策略：绿灯只证伪；**唯一校验** = 换一个正确的替代实现它会不会红；**准入三扇门** = 条款 / 病史 / 证明；**退役举证责任倒置** = 三扇门一扇都过不了的删，删除按批次可 revert（git 可取回），不设观察期 | `docs/src/dev/coding-rules.md`（第七部分） | — |
| | 0.2.6 错误码参考页改**构建期生成**（zh + en 两侧，不入库）+ 退役「入库快照 == 生成器输出」门禁：页面由 `scripts/docs/gen-error-code-docs.py` 从码表注册表与 `locales/*.json` 在文档站构建时再生，站点上的错误码页因此恒与码表一致 | 0.2.5 | `node docs/gen-error-docs.js` 产出 zh/en 各 11 页且与码表一致；文档站构建产物含错误码页；`check-docs-truth.py` / `check-docs-orphan.py` / `check-docs-refs.py` 全绿；退役依据 = coding-rules 7.1（该门禁冻精确行号 = 实现选择）+ 7.3（删除可整体 revert）；归属阶段：P0；是否阻塞主线：否 |
| 0.3 表门禁推广 | 0.3.1 `tools/code-tables` 扩展覆盖 opcode / 类型表 | 0.1.1 | — |
| 0.4 规程承诺与工具行为一致性 | 0.4.1 默认排除表只作用于**遍历到的子项**（按 `file_name` 判，不看祖先路径）、不作用于用户显式给出的路径；`--exclude` 与比较基准同源（issue [#463](https://github.com/ChenXu233/YaoXiang/issues/463)） | — | 仓库内 `check target/…/x.yx`（文件与目录两形态）可直接跑；嵌套 `target/` 子目录仍跳过；`--exclude` 真正生效（`check src/std --exclude src/std` ⇒ 零发现） |
| | 0.4.2 `scratch.py trial` 参数形态与文档一致（归属阶段：P0；是否阻塞主线：否） | — | — |
| | 0.4.3 locale 错误码 title 大小写统一：提示词补字段级规则（新增码不再自选大小写）+ 机械规范化（存量 en 52 / ru 21 处句首大写，ja / 文言文 / 喵语以技术标识符开头者不动）+ vitest 门禁断言该不变量。**D 类（对外文本面）**——诊断标题的字面会变，经用户裁决取 A1 机械规范化 | — | en/ru 每个错误码 title 句首大写（vitest 断言）；`scripts/i18n/normalize-titles.ts` 幂等可重复执行；locale 差异只为首字母大小写（零其它改动）；归属阶段：P0；是否阻塞主线：否 |
| 0.5 测试套件分层 | 0.5.1 测试资源画像（`cost:` 就地标记）+ 重测独占锁 + `fast`/`full` 双通道最小消费面：`scripts/dev/testsuite.py`（`markers --check` 与 `cargo test --list` 双向对账、`plan` 预算装箱、预算不足即报缺口不静默跳过、`manual` 档永不隐式执行）与 `src/util/test_support.rs`（跨进程文件锁） | — | 标记是单一事实源且与 `cargo test --list` 双向一致；独占档任何调用路径都互斥；预算不足必须报缺口不得静默跳过；归属阶段：P0；是否阻塞主线：否 |
| | 0.5.3 语料基线只记**结构化事实**（issue [#502](https://github.com/ChenXu233/YaoXiang/issues/502)）：`corpus_probe` 的 `run.stderr` 不逐字保存渲染后的诊断文案——文案上游是会被 auto-translate 改写的 `locales/*.json`，收成 `diagnostics: [[码, 文件, 行], …]` + 仅保留**崩溃标记行**（panic / ICE / 回溯头） | 0.5.1 | 语料基线重生、`check-corpus-parity` 差分空；**改 locales 文案后重跑探针仍零差分**；代价 = 语料层不再验运行期文案，改由 locales 单测 / `yx explain` 快照 / 生成物对账承担；归属阶段：P0；是否阻塞主线：否 |
| | 0.5.2 通道接线与校准：pre-commit 的 `cargo test` 钩子与 CI 切到 `fast`（本地提交前）与 `full`（推送前与 CI 口径）双通道，2600 档资源画像由实测校准，`test-specification.md` 增补资源标记与独占档的细则节（**待排期**；#486 后续切片） | 0.5.1 | `pre-commit` 的 `cargo test` 钩子不再裸跑全量；CI 与本地提交前走同一通道口径；2600 档标记的峰值与耗时为实测值而非外推值；`test-specification.md` 有对应细则节；归属阶段：P0；是否阻塞主线：否 |

> **没有规模基线任务**。2026-10-03 决策取消一切行数 / 体积门禁，规模问题由职责分离解决（见 `08` 禁令二判据 A，人工判断）。
> **0.2.1 是本阶段的核心验收**：门禁若报不出已知的平行表示，说明判据设计错了，是假门禁。

### P1 复活孤儿测试并修缺陷

来源：[06](06-cleanup-inventory.md) §实施要点 S1 + [05](05-frontend-paradigm.md) §测试重建

- [x] **1.1 复活 `lexer/tests/`**（4 个三级任务）
- [x] **1.2 复活活目录中的 4 处**
- [x] **1.3 修复暴露的缺陷**

| 二级 | 三级 | 文件 | 前置 | 验收 |
| --- | --- | --- | --- | --- |
| 1.1 复活 `lexer/tests/` | 1.1.1 加 `#[cfg(test)] mod tests;` | `lexer/mod.rs` | — | 测试数上升 |
| | 1.1.2 补 `mod lexer_mod;` / `mod symbols;` | `lexer/tests/mod.rs` | 1.1.1 | **不补则该批测试仍不跑** |
| | 1.1.3 删 7 个空壳文件 + 其 `mod` 声明 | `lexer/tests/` 7 文件 | 1.1.1 | C6 无判据 |
| | 1.1.4 删 `pub use ...::*;` 再导出块 | `tests/mod.rs` | 1.1.3 | 该块正是掩盖空壳的原因 |
| 1.2 复活活目录中的 4 处 | 1.2.1–1.2.4 分别接 `pratt/tests/precedence_inline.rs`、`passes/tests/overload_inline.rs`、`emitter/tests/json.rs`、`package/template/tests/` | 各父模块 | — | 测试数上升；**1.2.1 与 8.0.1 冲突，见冲突登记 C1** |
| 1.3 修复暴露的缺陷 | 1.3.1 四个基数扫描器溢出路径 | `literals.rs` | 1.1.x | `cargo test` 绿 |
| | 1.3.2 `\x`/`\u` 非法转义 + `scan_leading_dot` | `literals.rs` | 1.1.x | 同上 |

**验收**：测试数上升；**预期暴露真实缺陷**，需预留修复时间。逐文件 revert，但**修复的缺陷不应回滚**。

### P2 建立等价性判据基线

来源：[07](07-equivalence-oracle.md) 全部（该篇无「实施要点」节，按结构反推）

- [x] **2.1 IR 静态校验器**（4 个三级任务）
- [x] **2.2 规范化快照**（2 个三级任务）
- [x] **2.3 语料差分**（3 个三级任务）
- [x] **2.4 漏洞专门判据**（3 个三级任务）
- [ ] **2.5 回归门禁**（1 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 2.1 IR 静态校验器 | 2.1.1 `verify.rs` 双模式 `verify_loose` / `verify_ssa`：契约形态 `verify(ir, mode) -> Result<(), VerifyError>`，违规全收集非首错即停；def/use 提取穷尽 match 无通配臂 | — | C4 |
| | 2.1.2 **`verify_loose` 在现有非 SSA IR 上跑绿**（全语料绿，校验面非空断言内置；语料差分基线自比零差分证行为等价） | 2.1.1 | **硬门槛，P7 批 a 的准入条件**——**已解锁** |
| | 2.1.3 实现 7 项不变量（支配性、唯一定义、Phi 一致、跳转目标、全局越界、类型一致、内层隔离）（判定面按实测收窄——支配性=具名槽 must-defined（temp 槽 Void 预初始化是帧语义）、类型一致=参数槽签名+常量可信源（完整面归 P7 值表）、Phi 待批 b 变体引入） | 2.1.1 | C4 |
| | 2.1.4 post-mono 内层隔离红态（两文件 `InnerIsolation`：`native_fn_member_value.yx` / `pureyx_export_fn_value.yx`；已完成——定性为判据假阳性（`MakeClosure` 是函数值的一等表示，不是词法嵌套关系），不变量 7 改按槽身份 `LocalSlot::origin` 判定）（归属阶段：P7；是否阻塞主线：否） | 2.1.1 | — |
| 2.2 规范化快照 | 2.2.1 规范化工具（剥 Span / 临时值重命名 / 槽位相对化 / 前驱排序）（`src/middle/core/normalize.rs`，07 五规则逐项实现；DefId 按首次出现序相对化（intern 序跨进程不稳定）、Arg 独立 arg% 前缀（与 Local 同池会丢变体区分）；指令打印穷尽 match） | — | C1/C3/C5 |
| | 2.2.2 快照入库 + 人工 review 流程（`src/middle/ir/tests/snapshots/`：快照按语料相对路径镜像入库；更新流程 `UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored` + git diff review + 同 PR 入库；篡改实测精准报红） | 2.2.1 | C1/C3/C5 |
| 2.3 语料差分 | 2.3.1 差分框架 + 语料基线（探针 `examples/corpus_probe.rs` + 基线 `tests/baselines/corpus-parity.jsonl` + 门禁 `check-corpus-parity.py` 入 CI（硬门禁）；归一化含指针形态 scrub，两次全量运行门禁自比零差分） | — | 诊断/退出码/stdout 逐项 |
| | 2.3.2 **多文件语料层**（新建 `tests/yaoxiang-multifile/`，带 `yaoxiang.toml` 的项目夹具——决议 D48；已完成） | — | — |
| | 2.3.3 C4 行为差分必覆盖清单（10 类语义）+ 性能基线（criterion 冒烟基准）（10 类语义逐项核对全覆盖，代表文件均在 corpus-parity.jsonl 差分基线内；冒烟基准 `benches/pipeline.rs` 就位） | — | C2/C4/C5；性能基线供 P4/P7/P8 对比 |
| 2.4 漏洞专门判据 | 2.4.1 多文件证明义务不得静默丢弃 | 2.3.2 | — |
| | 2.4.3 未证明不得静默放行 | — | — |
| | 2.4.4 发版计划 span 必须被消费 | — | — |
| 2.5 回归门禁 | 2.5.1–2.5.4 见 P9 统一清单 | 2.1–2.3 | 见 P9 |

### P3 修正确性漏洞（最小方案）

来源：[02](02-stage-contract.md) §实施要点 S3 的漏洞修复半部 ＋ **2026-10-07 静默丢弃点审计（3.4）**

> **语义修正分流（D59，2026-10-11 裁决）**：本轨道及其后续阶段若发现「旧行为不算成功」的缺陷——**A 静默失败 / B 静默错配 / C 未定义行为与资源失控**（崩溃、爆栈、静默给错答案）——**可随重构顺路修**，四条缺一不可：①先红后绿（红判据先存在且确实红）；②不改变「原先能成功编译并正确运行」程序的对外表现（不得变成失败，不得删改其既有诊断码/文案/退出码）；③同提交登记（本行标注错误类型 + 该阶段修订注记 + PR 必填块「错误类型」一行）；④基线同提交再生 + 人工 review diff。**D 用户可见语义 / E 对外契约表 / F RFC 级设计变更三类仍必须先经裁决。判据展开与 A–F 白名单见 [07 §语义修正的分流判据](07-equivalence-oracle.md)；**不采纳「默认超级重构 + 事后登记」**（理由见 RFC-039 D59：零红判据无法区分修复与回归，且与「不存在 C5′」「不设任何放宽」冲突）。
>
- [x] **3.1 `proof_calls` 消费端**（2 个三级任务）
- [x] **3.2 第二个静默丢弃点**（1 个三级任务）
- [x] **3.3 panic 转诊断**（1 个三级任务）
- [x] **3.4 静默丢弃点补遗**（9 个三级任务；归属阶段：P3；是否阻塞主线：否）
- [x] **3.5 高阶类型参数调用路径补全**（3 个三级任务；归属阶段：P3；是否阻塞主线：否）
- [x] **3.6 函数类型值调用路径**（6 个三级任务；✅ `480a00b7`、`c27aa3fa`；issue [#479](https://github.com/ChenXu233/YaoXiang/issues/479)、[#494](https://github.com/ChenXu233/YaoXiang/issues/494)；归属阶段：P3；是否阻塞主线：否）
- [x] **3.7 多文件 init 段调用名限定**（4 个三级任务）
- [x] **3.9 运行期栈守卫不可达（E6004）**（1 个三级任务）
- [ ] **3.8 native 对用户函数名的遮蔽**（1 个三级任务；缺钉测试；**归属阶段：P3；是否阻塞主线：否**——验收要求先定语义（用户定义优先 vs 显式诊断），属裁决点）
- [x] **3.10 编译期无闸原生递归路径（抬高 `mono.max_depth` 后崩溃）**（1 个三级任务）
- [x] **3.11 错误码保真与深栈回显**（2 个三级任务；issue [#500](https://github.com/ChenXu233/YaoXiang/issues/500)）
- [ ] **3.12 运行时模式间的执行器配置传播**（1 个三级任务，待裁决；D 类：宿主自定义 `max_stack_depth` 在任务内被静默忽略；归属阶段：P3；是否阻塞主线：否）
- [x] **3.13 ir_gen 方法绑定 panic（方法组当值再绑定 + 无签名标注）**（1 个三级任务；issue [#451](https://github.com/ChenXu233/YaoXiang/issues/451)）
- [x] **3.14 方法级重载决议纳入命名实参**（1 个三级任务；issue [#452](https://github.com/ChenXu233/YaoXiang/issues/452)、[#466](https://github.com/ChenXu233/YaoXiang/issues/466)）
- [x] **3.15 `typecheck::infer_expression` 零调用点死 API**（1 个三级任务；issue [#453](https://github.com/ChenXu233/YaoXiang/issues/453)）
- [x] **3.16 stdlib 原生导出命名实参一致性**（1 个三级任务；issue [#454](https://github.com/ChenXu233/YaoXiang/issues/454)）
- [x] **3.17 手注 FFI 命名实参静默丢弃**（1 个三级任务；issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)；归属阶段：P3；是否阻塞主线：否）
- [ ] **3.18 约束泛型运算符派发的跨模块类型身份**（3 个三级任务；3.18.1 待排期（P6 类型身份收敛）；3.18.2 已完成 → 3.24.3；3.18.3 已完成；归属阶段：P3；是否阻塞主线：否）
- [ ] **3.19 精化谓词的 SMT 排序翻译崩溃**（1 个三级任务，来源 = 4.4.2 施工期探针；**归属阶段：P3；是否阻塞主线：否**）
- [x] **3.20 死代码分析的既有误报**（4 个三级任务；归属阶段：P3；是否阻塞主线：否）
- [x] **3.21 发版前审计（v0.8.4）补登缺口**（4 个三级任务；issue [#509](https://github.com/ChenXu233/YaoXiang/issues/509)；归属阶段：P3；是否阻塞主线：否）
>
- [x] **3.22 函数类型注解的分解语义**（1 个三级任务；issue [#501](https://github.com/ChenXu233/YaoXiang/issues/501)；归属阶段：P3；是否阻塞主线：否）

- [x] **3.24 模式匹配补全余项（RFC-010b 范围）**（1 个三级任务；issue [#405](https://github.com/ChenXu233/YaoXiang/issues/405)；归属阶段：P3；是否阻塞主线：否）
- [x] **3.23 清单解析失败 / 缺必填字段的静默退化**（1 个三级任务；issue [#503](https://github.com/ChenXu233/YaoXiang/issues/503)；归属阶段：P3；是否阻塞主线：否）

> **批次（2026-10-14 裁决）**：3.13 / 3.14 / 3.15 / 3.16 四个小项**相关性大**（方法绑定、决议、推断入口同属一条路径；3.16 是同一套「命名实参」判据的另一载体——stdlib 原生导出），**合并为一支团队处理**——团队范围由「一个二级任务」放宽为「一组相关小项批次」；批次构成、理由与偏差须随该队收口登记。

| 二级 | 三级 | 文件 | 前置 | 验收 |
| --- | --- | --- | --- | --- |
| 3.1 `proof_calls` 消费端 | 3.1.1 `proof_calls` 收窄可见性 + 唯一 getter（核实：实为 `pub(super)` + `pub fn proof_calls()`，非「私有」） | `types.rs` | 2.4.1 | 漏洞判据转绿 |
| | 3.1.2 消费点补齐 | `orchestrator.rs`（四入口 + 单文件共 5 处） | 3.1.1 | C2 |
| 3.2 第二个静默丢弃点 | 3.2.1 `Unproven` 空 match 臂产出诊断（核实：`ownership.rs` 只构造 `Proved`/`Disproved`，本臂源码不可达，属防御性代码；判据转绿由 3.1 承担，本项无独立判据） | `checker.rs` | 2.4.3 | 未证明不得静默放行 |
| 3.3 panic 转诊断 | 3.3.1 `.expect()` → `SMTResult::Unknown` + 诊断 | `predicate.rs` | — | Z3 缺失下不再 panic |
| 3.4 静默丢弃点补遗 | 3.4.1 const 泛型约束 `Unproven` 空臂改产诊断/记账（审计新增：全仓文档零登记、零任务；已实施：接线已注册的 W1063，bounds 层区分 TypeVar 内部探测实参防误报） | `environment.rs` | — | 新判据先红后绿：const 实参不可编译期求值时不得静默 |
| | 3.4.2 终止检查求解器缺失留痕（已实施：新码 W1081，NotProved（求解器在但判不出）不触发，本码专属「求解器缺失」） | `checker.rs` | — | Z3 缺失时产 W 级诊断；与 3.3.1 同一降级哲学 |
| | 3.4.3 02 阶段覆盖表第 1 行同步 | — | — | — |
| | 3.4.4 `variant_ctor_calls` 零消费者裁决 | ir_gen.rs | 4.3.1 | — |
| | 3.4.5 `method_overload_ir_names` 与 `overload_resolutions` 平行表示二选一（禁令一） | ir_gen.rs | 4.3.1 | — |
| | 3.4.6 `compile_project` 补 `warnings` 消费（多文件 `run` 永不报 W1001/W1002/W1003） | src/frontend/module/orchestrator.rs、src/driver/arms.rs、src/driver/program.rs | 4.9（已完成） | — |
| | 3.4.7 LSP 非项目内路径接入统一消费端 | lsp/handlers/diagnostics.rs | 4.2.6 | — |
| | 3.4.8 多文件单态化臂 | expressions.rs、arms.rs、program.rs | 4.2.8 | — |
| | 3.4.9 `interface_impl_registry` 跨阶段消费者裁决 | types.rs | 4.3.1 | — |
| 3.5 高阶类型参数调用路径补全（：RFC-010 的元类型宇宙层级 + `MetaLevel` + `Type::MetaType` + parser 齐备，**声明侧活**（探针绿）、**两条调用路径响亮失败**（推断传类型实参 E1002 / 显式给类型实参 E1010）——属「已接受特性未实现」，不是静默错误 | 3.5.1 合一器在**头部位置**匹配构造器变量 | src/frontend/core/typecheck/inference/expressions.rs、src/frontend/core/types/substitute.rs | — | — |
| | 3.5.2 显式类型实参调用路径打通（E1010 消失） | src/frontend/core/typecheck/inference/expressions.rs | 3.5.1 | — |
| | 3.5.3 mono 对构造器变量做替换 | src/middle/passes/mono/mod.rs、src/middle/core/ir_gen.rs | 3.5.1 | — |
| 3.6 函数类型值调用路径（闭包变量按名调用）（：局部函数类型值按名调用被降格为按名 Call，运行期 `E6006 Native function not found: f`；非泛型亦复现，与 3.4.8 无关，形态归 #416 家族 | 3.6.1 定位并修复函数类型值的调用降格 | src/middle/core/ir_gen.rs | — | — |
| | 3.6.2 跨文件导入的函数类型值按名调用（`use lib.{f}` 原报 E6006）——已完成：把 `use` 别名臂纳入「名字是否解析为值」，按名调用改走间接调用；判据单一源 = `is_static_fn_name` 内加一条臂，新增唯一实现 `use_alias_value_slot` 承担「use 别名 → 限定名 → 全局槽位」两表联查；转钉测试 3 条（含「导入的函数定义仍静态调用」回归防线）（归属阶段：P3；是否阻塞主线：否） | src/middle/core/ir_gen.rs | 3.6.1 | — |
| | 3.6.3 嵌套 lambda 捕获函数类型参数后按名调用给 `void`（静默错值，D59 A/B 类）——已完成：**判据改判**——不是「让它跑对」，而是「嵌套函数体引用外层帧值 → 编译期 E1001」（按 RFC-009 §2.3 + SPEC §12.3 该形态应编译期被拒）；修复前为「不报错、算错值（void）」的静默通道，修复后与既有未知变量路径同码同文案；赋值路径由 E2010 兜住；判据单一源 = 捕获屏障（`enter_capture_barrier`），编译期名（类型参数）不挡（归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/inference/scope.rs、src/util/diagnostic/codes/e1xxx.rs | 3.6.1 | — |
| | 3.6.4 `use lib` 整体导入后的限定名调用 `lib.f(...)`（走 FieldAccess callee 分支，原报 E6006）——已完成：命名空间分支解出限定名后先判全局槽位布局 → 改走间接调用；限定名解析与槽位查询复用 `resolve_namespace_data_access`（与数据访问同源）；转钉测试 2 条（归属阶段：P3；是否阻塞主线：否） | src/middle/core/ir_gen.rs | 3.6.2 | — |
| | 3.6.5 选择式导入的函数值在顶层重绑后**静默返回 `void`**（issue [#479](https://github.com/ChenXu233/YaoXiang/issues/479)） | src/middle/core/ir_gen.rs | 3.6.2 | — |
| | 3.6.6 「限定名是什么」判据的**第三份实现**收敛（重构项，issue [#494](https://github.com/ChenXu233/YaoXiang/issues/494)；来源 = 本批批次复审的禁令三扫描）：原生名判定 / 命名空间臂 / 别名判据各答一次「这个名字是什么」，今日无可达分歧故不阻塞，但属「同一行为 ≥2 处复制」的苗头；已完成 480a00b7、c27aa3fa——收敛形态为**关注面参数化接口** `ModuleRegistry::qualified_name_is(qualified, QualifiedNameConcern)`：三臂对同一形态答案可相反（`std.io.println` = 是/否/是），三变体 NativeName / FunctionCodeExport / FunctionOrMethodDef 各自保真原臂语义 + 拆名机械面唯一基底 `split_qualified`（issue [#494](https://github.com/ChenXu233/YaoXiang/issues/494)；归属阶段：P3；是否阻塞主线：否） | src/frontend/module/registry.rs、src/middle/lower/query.rs、expr/access.rs、src/frontend/module/tests/registry.rs | 3.6.5 | — |
| 3.7 多文件 init 段调用名限定（：`qualify_names` 只遍历 `ir.functions` 重写调用名，`ir.init` 从不被访问——多文件下函数定义被限定（`main.f`）而模块顶层调用名仍是裸名 → 运行期 `E6006`（非泛型函数复现，与泛型无关）；既是既有正确性漏洞，也是 mono 侧三级名字解析（含全程序唯一后缀启发式）的成因 | 3.7.1 `qualify_names` 覆盖 `ir.init`：同一 rename/aliases 表、同一 `rewrite_call_names`（含方法前缀匹配）；单文件 `module_key=None` 行为不变 | `middle/core/ir_gen.rs` | — | 红判据先红后绿：非泛型顶层调用 + 泛型顶层调用 + 同名歧义三形态；全量门禁 + 基线/快照核对 |
| | 3.7.2 聚合侧用单元键把同单元请求名限定化（`aggregate_instantiation_requests` 已持 `units[i].key`），使请求名与 IR 调用指令同源 | `driver/helpers.rs` | 3.7.1 | 请求名与 IR 字符串逐字一致（调用点改写三元键不落空）；跨单元泛型矩阵与单文件等价 |
| | 3.7.3 删除 mono 的「全程序唯一后缀」启发式与未命中静默 `continue`，改确定性解析 + 响亮诊断 | `middle/passes/mono/mod.rs` | 3.7.2 | 无静默不特化：未命中/歧义必产诊断（码 + 文本） |
| | 3.7.4 init 段 `DefId` 与函数体同口径 | middle/core/ir_gen.rs | 3.7.1 | — |
| 3.8 native 对用户函数名的遮蔽（登记）：单文件里用户泛型名 `read` 被 native `read` 遮蔽，实测报 `read expects 2 arguments (file: File, n: Int)`；当前**无钉测试** | 3.8.1 定名解析优先级（issue [#477](https://github.com/ChenXu233/YaoXiang/issues/477)） | frontend/module | — | — |
| 3.9 运行期栈守卫不可达（E6004）（登记）：解释器 `push_frame` 的 `max_stack_depth` 默认 1024，但 Rust 层递归先爆栈——抬高闸门后编译期拦截被换成进程崩溃；且 E6004 的 `limit` 参数恒传 0，真触发时用户看到「递归深度超出限制 0」。**与泛型无关**：凡深递归程序同崩 | 3.9.1 守卫可达性：超限必然走 E6004 而非崩进程；`limit` 带真实上限 | backends/interpreter/executor/executor.rs、backends/mod.rs、util/diagnostic/mod.rs | — | — |
| 3.10 编译期无闸原生递归路径（登记，issue #447）：`mono.max_depth`（默认 100）抬高后，病态泛型递归（`f(x)=f([x])`）不再被闸门拦下，而是让编译器进程爆栈；**深度闸本身是队列驱动**（循环内无原生递归）→ 崩溃在下游**无闸的原生递归路径**上 | 3.10.1 定位崩溃点，并让「闸门放开后」的路径不再依赖原生栈：超限必然产出既定诊断（E3005）而非崩进程；默认配置行为不变 | middle/passes/mono/mod.rs、frontend/config.rs、frontend/core/types/eval/const_eval.rs | 3.9（同族不同层） | — |
| 3.11 错误码保真与深栈回显（发现并登记，源自 3.9.1 实现/验证过程）：① Standard 运行时模式下任务内部错误经 `ExecutorError::runtime` 字符串化 → **一律落 E6007**（机制对错误一视同仁）② E6004 首次可达后回显超深回溯 | 3.11.1 任务结果通道承载结构化错误（`TaskResult = Result<SyncValue, SyncValue>` 错误侧结构化、`TaskOutcome::Err` 同步）：任务内各码在 Standard 模式保真，不再一律 E6007；3.11.2 深栈回溯截断策略（保留首尾 + 计数省略） | backends/interpreter/executor/executor.rs、backends/runtime/engine.rs | 3.9（同源发现） | — |
| | 3.11.3 运行期 **E6007 文案在「环内读未初始化顶层值」上误导**（issue [#500](https://github.com/ChenXu233/YaoXiang/issues/500)，登记；来源 = 4.10 轮 2 的批次复审 F2/F3 与取证轮实测）：编译期已有 W1007 说清结构原因，但**运行期**仍报 `type mismatch in comparison Eq: Void vs Int(1)`——真相是「该顶层值在环内没有确定的初始化先后」。两条修法：A 只改文案/加 hint（零新运行期状态、可逆）、B 解释器识别「读未初始化全局槽」（要给全局槽加已初始化位，动运行期状态与 IR） | `src/backends/interpreter/executor/executor.rs`（二元 / 比较的 E6007 文案构造）、`src/backends/interpreter/executor/ops/arith.rs`（一元）、`src/util/diagnostic/codes/e6xxx.rs`（码定义） | 4.10.6 | **终态判据**：复现形态（`main → zlib → zother → zlib`，zlib 读 zother.o_val）的 stderr 含指向初始化顺序的 hint 或专门诊断；**E6007 在其他场景（真正的二元运算类型不匹配）文案逐字不变**；先红后绿 + 语料基线同提交再生；**不改** W1007 的告警面。**归属阶段：P3；是否阻塞主线：否** |
| 3.12 运行时模式间的执行器配置传播（发现并登记，源自 3.11.1 实现过程）：Standard 模式下任务解释器按默认 `ExecutorConfig` 建，父配置不传播 ⇒ 任务内 limit 恒 1024、宿主自定义上限被静默忽略 | 3.12.1 让任务解释器继承父执行器配置（或在文档与诊断里显式声明「不继承」并给理由）；默认配置行为不变 | `backends/interpreter/executor/executor.rs` | 3.11 | 先红后绿：宿主配置 `max_stack_depth=64` 时任务内超限报 `E6004 limit=64`（现恒为 1024）；默认配置零回归。：另起队伍（本轮不并入）**；D 类变更须在该队内先获裁决 |
| 3.13 ir_gen 方法绑定 panic（A 类崩溃） | 3.13.1 `Temp.<name> = <表达式>[<下标>]` 且无签名标注 ⇒ 重绑定识别不命中（基底是 `FieldAccess` 而非 `Var`）⇒ 落到 `// MethodBind` 分支对 `type_annotation` 执行 `unwrap()` ⇒ **进程 panic、退出码 101**。正确行为 = **不 panic**，产出明确诊断（或在能唯一确定签名时正常编译） | `middle/core/ir_gen.rs` | — | 先红后绿：以最小复现为红判据（断言「不 panic 且产出诊断」）；**不得**用 `#[should_panic]` 把崩溃钉成期望行为；issue #451 |
| 3.14 方法级重载决议纳入命名实参 | 3.14.1 决议候选匹配纳入命名实参（按形参名绑定到对应位置后与位置实参一起做类型匹配），消除默认绑定回落；判据覆盖纯命名 / 混合 / 命名与位置冲突 / 命名重复绑定四形态 | `frontend/core/typecheck/inference/expressions.rs`（决议段）、`middle/core/ir_gen.rs`（消费点） | 4.5.1（键已 PlanId 化、消费必达结算在位） | 先红后绿四形态各一判据；语料差分须人工 review（E1107 / E1002 可能新增或消失）；issue #452 |
| | 3.14.2 多候选重载下命名实参只按**首个声明候选**的形参名匹配（`candidate_arity_groups` 每个元数只保留首候选名字表）⇒ 合法调用被 E1014 误拒、且类型错误被归到另一候选；与 3.14.1 的自述目标相反。D59 **C 类**；先红后绿：补「命名实参命中非首候选」绿态 + 「名字不存在于任何候选」红态。已完成 4941f443（ir_gen 侧同步修复，`candidate_arity_groups` 收窄为元数结算专用；issue [#466](https://github.com/ChenXu233/YaoXiang/issues/466)） | frontend/core/typecheck/inference/expressions.rs | 3.14.1 | — |
| 3.15 `typecheck::infer_expression` 死 API（复审 F3） | 3.15.1 `typecheck` 的 `infer_expression` 全仓**零调用点**，且不回流 `overload_resolutions`（无产出/消费台账）⇒ 接线即成 D20 同类缺口 | `frontend/core/typecheck/mod.rs` | — | 处置二选一：**删除**（按 P10 S2 死代码）或**接线前先补契约**；无行为面故不适用先红后绿，但删除须有零调用点直证（`git grep`）并记录被删逻辑的去向；issue #453 |
| 3.16 stdlib 原生导出命名实参一致性（C 类静默错值） | 3.16.1 原生导出调用路径的命名实参绑定：位置实参按声明顺序占位、命名实参按形参名绑定；未知形参名 / 缺参 / 同一形参重复命名 / 位置与命名争槽四类必须**编译期报错**，不得静默丢弃实参或静默接受未知名 | `middle/core/ir_gen.rs`（原生导出调用分支）、`frontend/core/typecheck/inference/expressions.rs`（决议段 / `bind_named_args`） | 3.14.1（方法级同一套判据） | 先红后绿四形态各一判据（形参名正确 / 形参名错误 / 纯命名 / 混合），且须证「原先成功程序」对外表现不变（语料逐行等价）；issue #454 |
| 3.18 约束泛型运算符派发跨模块类型身份（C 类） | 3.18.1 MonoType 用户类型携带定义模块（或在 mono 派发处接入类型身份表），使 `T.add` 派发改写能还原 `lib.Vec3.add` 形态限定名；先红后绿：`lib.yx` 定义 `Vec3` + Add 实现、`main.yx` `use lib.{Vec3}` 跨模块 `sum(a, b)` 运行期不再 E6006 | `frontend/core/types/mono.rs`（类型身份）、`middle/passes/mono/mod.rs`（`qualify_deferred_method` 已知缺口注记） | P6 类型身份收敛（或裁决降级为 3.5 家族） | 先红后绿：跨模块约束派发探针由 E6006 转 exit 0；同模块/FFI 原生回归零影响；**归属阶段：P3（前置待 P6）；是否阻塞主线：否** |
| | 3.18.2 导入类型的注解身份：函数返回**经别名引入**的导入类型时，返回注解与实得类型不可判等——`make_b: () -> TempB` + `u: TempB = make_b()` ⇒ `E1012`（返回类型不匹配）+ `E1002`（类型不匹配）、退出码 1；双二进制逐条一致 ⇒ 与 #465 无关；归因同 3.18 的 MonoType 裸名义名族（issue [#504](https://github.com/ChenXu233/YaoXiang/issues/504)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/types/mono.rs、src/frontend/core/typecheck/inference/statements.rs | 3.18.1 | — |
| | 3.18.3 三条未收敛的短名派发臂：`print`/`println` 的零开销 `to_string` 派发与 `Try` 四方法派发仍按**短类型名**拼调用名、不经 `method_dispatch_name`；当前分别**不可达**与被 typecheck **`E1082` 硬错**挡住 ⇒ 无用户可见缺陷，但**放开任一前置即成静默错派**（#465 重演；issue [#506](https://github.com/ChenXu233/YaoXiang/issues/506)） | src/middle/lower/expr/call.rs、src/middle/lower/expr/op.rs、src/driver/tests/short_name_dispatch_pins.rs | 3.18.1 | — |
| 3.17 手注 FFI 命名实参静默丢弃（C 类） | 3.17.1 `add_native_signature` 与 stdlib 导出路径同源——改调 `typecheck::signature::export_signature_param_info` 唯一出口拿形参名与必需下限，**不得出现第二份绑定判据实现**（#454 复审 F1 教训） | src/frontend/core/typecheck/environment.rs、src/frontend/core/typecheck/signature.rs、src/frontend/core/typecheck/inference/arg_binding.rs、src/middle/core/ir_gen.rs | 3.16.1（同一套绑定判据） | — |
| 3.19 精化谓词的 SMT 排序翻译崩溃（登记，来源 = 4.4.2 施工期探针；D59 **A/B 类**（崩溃）；**定性 = 预存在**——独立验证在同一探针、同一输入下 stdout 逐字节一致（`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`）、均 exit 1（Z3 默认错误处理器 abort 进程）；静态归因一致：4.4.2 提交对 `translate.rs` 零行改动） | 3.19.1 症状：`NonNegative: (x: Int) -> Type = { x >= 0 }`、`AtMost: (n: Int, x: Int) -> Type = { x <= n }` 与 `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }` 单次编译即触发 Z3 `Sort mismatch` 并 **abort 整个进程**。成因指向 `proof/smt/translate.rs` 的排序推断（`infer_var_sorts`）——`z3_backend.rs` 的 `expr_to_z3` 把 `SMTExpr::Atom` 一律建成 Int 常量，与推断出的 Bool 排序不一致时即出现「not(Int)」 | `frontend/core/typecheck/proof/smt/translate.rs`（排序推断/翻译） | — | 验收行 = 先红后绿：该形状不再 abort（保守降级或诊断），语料与 IR 快照零漂移；**归属阶段：P3；是否阻塞主线：否** |
| 3.20 死代码分析的既有误报（登记；来源 = WBS 3.4.6 施工期暴露的两个**既有** W 码误报——：文档示例改为不触发误报的等价写法，误报本身登记待修；**归属阶段：P3；是否阻塞主线：否**） | 3.20.1 W1001「未使用的私有函数」不跟踪**闭包内的引用**：`docs/src/reference/stdlib/test.md` 的 `t_add` 形态（顶层测试函数只在 `test.suite` 的闭包 `("add", () => t_add())` 内被引用，却报 W1001）；非 3.4.6 引入（归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/passes/dead_code.rs、src/util/diagnostic/codes/w1xxx.rs | — | — |
| | 3.20.2 W1003「未使用的导入」把**变体解构必需导入**判成未使用：`docs/src/reference/stdlib/weak.md` 的 `use std.option` 形态（变体集在场是 `match some(v)` / `none()` 的前置条件，删掉该导入即报 E1002）；非 3.4.6 引入（归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/inference/expressions.rs、src/util/diagnostic/codes/w1xxx.rs | — | — |
| | 3.20.3 算子接口方法被算子派发**实际调用**仍报 `W1005 Unused method`：合法清单 + 类型体内 `Index(Temp, Int, Float)` / `Equal(Temp, Temp)` 声明下，`check` ⇒ 0 error + 4 条 W1005、`run` ⇒ 值正确 ⇒ 死代码可达性**不计算子派发**；与 #465 无关（✅ `c94bb869`；issue [#505](https://github.com/ChenXu233/YaoXiang/issues/505)、[#510](https://github.com/ChenXu233/YaoXiang/issues/510)） | src/frontend/core/typecheck/passes/dead_code.rs、src/driver/arms.rs、src/driver/helpers.rs | — | — |
| | 3.20.4 泛型约束形参的算子派发在死代码分析中不可见（issue [#510](https://github.com/ChenXu233/YaoXiang/issues/510)、[#511](https://github.com/ChenXu233/YaoXiang/issues/511)、[#512](https://github.com/ChenXu233/YaoXiang/issues/512)；归属阶段：P3；是否阻塞主线：否） | src/driver/helpers.rs、src/driver/arms.rs、src/driver/tests/method_unused.rs、tests/baselines/ | 3.20.3 | — |
| | 3.20.5 跨单元导出登记口只认 `(T: Type)`——接口约束 `(T: Add)` 与值依赖 const `(N: Int)` 的形参名不进 `declared_fn_type_params` ⇒ `Export::type_params` 为 None ⇒ 跨单元限定调用请求 `type_params` 空表走签名反推兜底（issue [#511](https://github.com/ChenXu233/YaoXiang/issues/511)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/checker.rs、src/frontend/core/typecheck/inference/statements.rs、src/driver/helpers.rs、docs/src/reference/ | 3.20.4 | — |
| | 3.20.6 E1106 调用点复检系结构性死码——到达时 `last_type_args` 恒空 ⇒ `get(idx)` 恒 None ⇒ `continue` 短路 ⇒ E1106 全路径不可达：「看起来有保护实际永不执行」，且其判据（接口 registry）与活执行面（mono 函数表）相抵，违背 RFC-011b「约束同判据」（issue [#519](https://github.com/ChenXu233/YaoXiang/issues/519)、[#512](https://github.com/ChenXu233/YaoXiang/issues/512)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/inference/expressions.rs、src/frontend/core/typecheck/inference/statements.rs、src/middle/passes/mono/mod.rs、src/std/mod.rs | 3.20.4（[#512](https://github.com/ChenXu233/YaoXiang/issues/512) 批次发现） | — |
| | 3.20.7 mono 派发改写未命中 → 编译期 E1106（issue [#519](https://github.com/ChenXu233/YaoXiang/issues/519)；归属阶段：P3；是否阻塞主线：否） | src/middle/passes/mono/mod.rs、src/util/diagnostic/codes/e1xxx.rs、src/middle/passes/mono/tests/monomorphizer.rs、locales/ | 3.20.6 | — |
| | 3.20.8 谓词排除判据收单点（issue [#522](https://github.com/ChenXu233/YaoXiang/issues/522)、[#523](https://github.com/ChenXu233/YaoXiang/issues/523)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/checker.rs、src/frontend/core/typecheck/inference/statements.rs、tests/integration/check.rs | 3.20.5 | — |
| | 3.20.9 类型级命名空间（TypeLevelNamespace）——Registry 阶段谓词可见性的架构解，#522 修复（issue [#524](https://github.com/ChenXu233/YaoXiang/issues/524)、[#523](https://github.com/ChenXu233/YaoXiang/issues/523)、[#522](https://github.com/ChenXu233/YaoXiang/issues/522)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/probe.rs、src/frontend/core/typecheck/checker.rs、src/frontend/core/typecheck/inference/、src/driver/arms.rs | 3.20.8 | — |
| | 3.20.10 curry 内层参数位精化未按 base 统一——`(N: Int) -> (x: IsPositive(N))` 完整应用假拒绝 E1002（issue [#523](https://github.com/ChenXu233/YaoXiang/issues/523)、[#524](https://github.com/ChenXu233/YaoXiang/issues/524)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/typecheck/inference/expressions.rs、src/frontend/core/typecheck/inference/statements.rs、tests/integration/check.rs、docs/src/reference/ | 3.20.9 | — |
| | 3.20.11 谓词约束体数据不跨单元传播——导入谓词 E1012+E1002 假拒绝（issue [#524](https://github.com/ChenXu233/YaoXiang/issues/524)、[#527](https://github.com/ChenXu233/YaoXiang/issues/527)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/sema/checker.rs、src/frontend/core/sema/predicate_resolver.rs、src/frontend/module/mod.rs、src/frontend/module/orchestrator.rs | 3.20.10 | — |
| | 3.20.12 泛型函数返回类型不实例化——注解收泛型调用结果被假拒绝 E1002（issue [#442](https://github.com/ChenXu233/YaoXiang/issues/442)、[#546](https://github.com/ChenXu233/YaoXiang/issues/546)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/sema/inference/statements.rs、src/frontend/core/sema/inference/expressions.rs、tests/integration/check.rs、docs/src/reference/ | 3.20.11 | — |
| | 3.20.13 build.rs 的 locales 重建触发面只覆盖 zh.json——假告警常驻且非 zh 变更漏检（issue [#458](https://github.com/ChenXu233/YaoXiang/issues/458)、[#548](https://github.com/ChenXu233/YaoXiang/issues/548)；归属阶段：P3；是否阻塞主线：否） | build.rs | 3.20.12 | — |
| | 3.20.14 导入环内读未初始化顶层值时 E6007 仍说 type mismatch——真实成因（环内初始化顺序）被诊断隐藏（issue [#500](https://github.com/ChenXu233/YaoXiang/issues/500)、[#549](https://github.com/ChenXu233/YaoXiang/issues/549)；归属阶段：P3；是否阻塞主线：否） | src/backends/interpreter/executor/executor.rs、src/backends/interpreter/executor/tests/、tests/integration/multifile.rs、tests/integration/check.rs | 3.20.13 | — |
| | 3.20.15 嵌套括号声明形态的 return 位 E1012 假拒绝——`make_box: (T: Type) -> ((v: T) -> Box(T)) = { return (v) => Box(v, 0) }` 报 expected 'Box(t38)'（issue [#546](https://github.com/ChenXu233/YaoXiang/issues/546)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/sema/inference/statements.rs、tests/integration/check.rs、docs/src/reference/error-code/ | 3.20.14 | — |
| | 3.20.16 多参数带约束 TypeDefinition 谓词的导入面与本地面不同构——本地面 proof-fn 形态可解析、导入面回退 Struct poly 被 E1012+E1002 拒绝（issue [#527](https://github.com/ChenXu233/YaoXiang/issues/527)、[#550](https://github.com/ChenXu233/YaoXiang/issues/550)、[#551](https://github.com/ChenXu233/YaoXiang/issues/551)；归属阶段：P3；是否阻塞主线：否） | src/frontend/module/mod.rs、src/frontend/core/sema/predicate_resolver.rs、src/frontend/module/orchestrator.rs、src/std/yx_sources.rs | 3.20.15 | — |
| | 3.20.17 build.rs 触发面输入面缺口的其余三处——host 分支 .z3 目录无触发声明、Z3_SYS_Z3_HEADER 与 CXXSTDLIB 环境变量无 rerun-if-env-changed（issue [#548](https://github.com/ChenXu233/YaoXiang/issues/548)；归属阶段：P3；是否阻塞主线：否） | build.rs | 3.20.16 | — |
| | 3.20.18 单参数谓词导入/本地面同构——proof-fn 形参清单统一 vars 直读（issue [#551](https://github.com/ChenXu233/YaoXiang/issues/551)、[#550](https://github.com/ChenXu233/YaoXiang/issues/550)；归属阶段：P3；是否阻塞主线：否） | src/frontend/module/orchestrator.rs、src/frontend/core/sema/checker.rs、src/frontend/module/mod.rs、tests/integration/check.rs | 3.20.17 | — |
| | 3.20.19 绑定注解位的谓词精化缺口——块局部绑定 `y: SumUpTo(1,2)=5`（可满足）被 E1002 假拒绝（issue [#550](https://github.com/ChenXu233/YaoXiang/issues/550)、[#552](https://github.com/ChenXu233/YaoXiang/issues/552)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/sema/inference/statements.rs、src/frontend/core/sema/inference/expressions.rs、src/frontend/core/sema/checker.rs、src/frontend/module/mod.rs | 3.20.18 | — |
| | 3.20.20 Eq/Neq 编译期放行跨类型与静态 Void 值比较——查证后**裁决包呈报，等裁决（完成查证与呈报，issue [#549](https://github.com/ChenXu233/YaoXiang/issues/549)）。**查证证伪「可按 C 类顺修」的前提**：探针矩阵实测容器异型 Eq 形态族（`[1,2]==(1,2)` / `[1]==["x"]` / `[1]==[2.5]` / `(1,2)==(1,"x")`）编译通过且**运行成功打印 false**——编译拒绝将改变这些原先运行成功程序的可观察行为 ⇒ D59 **D 类**，不满足顺路修条件；标量异型 / 静态 Void 比较 / 异型 Dict 比较运行期落 E6007（旧行为非成功，可顺修但不构成独立裁决点）；顺带观测**同型** Dict==Dict 运行期也 E6007（Handle 无比较臂，备案不裁决）。裁决点唯一 = 容器异型 Eq 是否编译拒绝（选项 A：编译拒绝、全量对齐排序比较的两侧同型静态纪律（判据本体 = infer_binary 比较臂）——推荐；选项 B：保留运行期 false、走设计流程）；完整裁决包四件在 issue [#549](https://github.com/ChenXu233/YaoXiang/issues/549) 评论。受影响面 grep 实测零依赖（语料 / 测试无此类夹具）；issue [#549](https://github.com/ChenXu233/YaoXiang/issues/549)；归属阶段：P3；是否阻塞主线：否） | — | 3.20.14 | — |
| | 3.20.21 方法面的编译期值参数实参不检查证据——`b.get(y)(y)` 运行期值填 q=0 槽静默通过（issue [#538](https://github.com/ChenXu233/YaoXiang/issues/538)、[#553](https://github.com/ChenXu233/YaoXiang/issues/553)、[#539](https://github.com/ChenXu233/YaoXiang/issues/539)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/sema/inference/expressions.rs、src/frontend/core/sema/checker.rs、src/frontend/core/sema/inference/statements.rs、src/frontend/tests/compile_time_evidence.rs | 3.20.19 | — |
| | 3.20.22 证明函数形参精化的嵌套义务展开——`f(42,7)` 违反 `Eq(7,0)` 零诊断（issue [#537](https://github.com/ChenXu233/YaoXiang/issues/537)；归属阶段：P3；是否阻塞主线：否） | src/frontend/core/sema/checker.rs、src/frontend/proof_execution.rs、src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs、src/frontend/tests/proof_execution.rs | 3.20.21 | — |
| | 3.20.23 跨模块导入的 const 泛型调用不检查证据（issue [#539](https://github.com/ChenXu233/YaoXiang/issues/539)、[#558](https://github.com/ChenXu233/YaoXiang/issues/558)、[#553](https://github.com/ChenXu233/YaoXiang/issues/553)；归属阶段：P3；是否阻塞主线：否） | src/frontend/module/mod.rs、src/frontend/module/orchestrator.rs、src/frontend/module/registry.rs、src/std/mod.rs | 3.20.22 | — |
| | 3.20.24 负字面量证明调用实参不可折叠——合法义务被 E2031 假拒绝（issue [#556](https://github.com/ChenXu233/YaoXiang/issues/556)） | src/frontend/core/sema/layers/predicate.rs、src/frontend/core/sema/layers/tests/predicate.rs、tests/integration/check.rs | 3.20.23 | — |
| | 3.20.25 一级求值器取负裸 -n——i128::MIN 可构造 panic（issue [#559](https://github.com/ChenXu233/YaoXiang/issues/559)） | src/frontend/core/types/eval/evaluator.rs、src/frontend/core/types/eval/tests/evaluator.rs、src/frontend/core/sema/layers/tests/predicate.rs | 3.20.24 | — |
| | 3.20.26 ast 层 fn[pos] 位置取负裸 -n——i64::MIN 形态用户源码可达 panic（issue [#560](https://github.com/ChenXu233/YaoXiang/issues/560)） | src/ast/mod.rs、src/ast/tests/binding_positions.rs、src/ast/tests/mod.rs、tests/integration/check.rs | 3.20.25 | — |
| | 3.20.27 SMT/Z3 整数字面量翻译三层失真——大字面量落自由变量 + 超界截断 + 单参数取负恒等丢失（issue [#561](https://github.com/ChenXu233/YaoXiang/issues/561)） | src/frontend/core/sema/proof/smt/z3_ffi.rs、src/frontend/core/sema/proof/smt/z3_backend.rs、src/frontend/core/sema/proof/smt/tests/backend.rs、src/frontend/core/sema/tests/rfc027_phase2_smt.rs | 3.20.26 | — |
| | 3.20.28 mono 特化名缺 function_files 条目——std 内嵌源码诊断行号贴上用户文件路径（issue [#443](https://github.com/ChenXu233/YaoXiang/issues/443)） | src/middle/passes/mono/mod.rs、src/middle/passes/codegen/translator.rs、src/middle/passes/mono/tests/monomorphizer.rs、tests/integration/cli_e2e.rs | — | — |
| | 3.20.29 字节码 Not 与 Neg 同码 I64_NEG——编码/解码往返把逻辑非改写为取负（issue [#562](https://github.com/ChenXu233/YaoXiang/issues/562)） | src/middle/bytecode/opcode.rs、src/middle/bytecode/mod.rs、src/middle/passes/codegen/translator.rs、src/middle/bytecode/tests/bytecode.rs | 3.20.28 | — |
| | 3.20.30 深嵌套表达式（≥300 层）编译器栈溢出零诊断——check 崩进程（issue [#540](https://github.com/ChenXu233/YaoXiang/issues/540)） | src/lib.rs、src/main.rs、src/frontend/core/parser/parser_state.rs、src/frontend/core/parser/pratt/mod.rs | 3.20.29 | — |
| | 3.20.31 SPEC syntax.md 类型位常量实参口径与 HEAD 对齐 + issue #543 收尾（issue [#543](https://github.com/ChenXu233/YaoXiang/issues/543)） | docs/src/reference/language-spec/syntax.md | 3.20.30 | — |
| | 3.20.32 方法面证据检查剩余形态——显式接收者与外部绑定判 E1113（issue [#553](https://github.com/ChenXu233/YaoXiang/issues/553)） | src/frontend/core/sema/checker.rs、src/frontend/core/sema/inference/expressions/call/instantiation.rs、tests/integration/check.rs、docs/src/reference/error-code/E1xxx.md | 3.20.28 | — |
| | 3.20.33 List(T) 注解落点归一 Vec 面——幽灵面假拒绝消灭 + user-List 遮挡退出（issue [#403](https://github.com/ChenXu233/YaoXiang/issues/403)） | src/frontend/core/types/mono.rs、src/frontend/core/sema/signature.rs、src/frontend/core/sema/inference/expressions/call/instantiation.rs、src/frontend/core/sema/inference/statements.rs | 3.20.32 | — |
| | 3.20.34 refactor 收账批——零消费者 pub 清账 + 消费台账转发面收敛 + 灌注块收敛（issue [#472](https://github.com/ChenXu233/YaoXiang/issues/472)、[#460](https://github.com/ChenXu233/YaoXiang/issues/460)、[#473](https://github.com/ChenXu233/YaoXiang/issues/473)） | signature.rs、orchestrator.rs、driver/program.rs、proof_execution.rs | 3.20.33 | — |
| | 3.20.35 调用形谓词体登记为证明函数——精化类型源码入口补齐（issue [#441](https://github.com/ChenXu233/YaoXiang/issues/441)） | src/frontend/core/sema/probe.rs、src/frontend/core/sema/checker.rs、tests/integration/check.rs、tests/integration/execution.rs | 3.20.34 | — |
| | 3.20.36 ureq 2.12.1→3.5.2 迁移——RUSTSEC-2026-0381 解除临时 ignore（issue [#489](https://github.com/ChenXu233/YaoXiang/issues/489)、[#571](https://github.com/ChenXu233/YaoXiang/issues/571)） | — | 3.20.35 | — |
| 3.21 发版前审计（v0.8.4）补登缺口（**归属阶段：P3；是否阻塞主线：否**） | 3.21.1 解释器**一元取负**对 `i64::MIN` 溢出直接 panic（`RuntimeValue::Int(-n)` 未走 checked 通道；二元溢出已正确报 E6007）；D59 **C 类**（崩溃）（✅ cfb3442d；issue [#468](https://github.com/ChenXu233/YaoXiang/issues/468)；归属阶段：P3；是否阻塞主线：否） | backends/interpreter/executor/ops/arith.rs | — | — |
| | 3.21.2 诊断外显保真三处：中文 help 就地硬编 · E4018 输出**空** `Counterexample:` 段且 `-3` 渲染成 `-(3)` · 致命调用错误后级联 `E1001`；D59 **D 类**（对外文本面）（✅ df89d6ee；issue [#469](https://github.com/ChenXu233/YaoXiang/issues/469)） | frontend/core/typecheck/inference/arg_binding.rs、middle/core/ir_gen.rs、util/diagnostic/codes/e4xxx.rs | — | — |
| | 3.21.3 `mono.max_depth` 无法从 CLI/`yaoxiang.toml` 配置——3.10.1 的抬高闸门档在 `run` 上不可复现（✅ a8ad0530；issue [#471](https://github.com/ChenXu233/YaoXiang/issues/471)） | src/main.rs、frontend/config.rs | — | — |
| 3.22 函数类型注解的分解语义（issue [#501](https://github.com/ChenXu233/YaoXiang/issues/501)；来源 = 4.10 轮 2 的批次复审 F4 追查）：返回位置的**括号化函数类型**在分解时层级错位——「声明返回函数类型的函数」**没有任何能过的注解写法**；注解本身解析正确（已实测排除 parser）。**归属阶段：P3；是否阻塞主线：否** | 3.22.1 见下 | `src/frontend/core/typecheck/inference/statements.rs` | — | **终态判据**：见 3.22.1 |
| | 3.22.1 统一「返回函数类型」的注解分解：`() -> ((x: Int) -> Int)` 的值参数组 = `()`、返回 = `fn(int64)->int64`，内层 `Fn` **不得**被吸收为参数组；同时把 lambda 体路径的期望口径与之对齐（现为两套）。判据覆盖四形态：lambda 体 / 函数名 RHS / 无内层括号 / 标量返回对照（✅ 7880ee17、f032582b；issue [#501](https://github.com/ChenXu233/YaoXiang/issues/501)、[#514](https://github.com/ChenXu233/YaoXiang/issues/514)、[#515](https://github.com/ChenXu233/YaoXiang/issues/515)） | src/frontend/core/parser/ast.rs、src/frontend/core/parser/statements/declarations.rs、src/frontend/core/typecheck/inference/statements.rs、src/frontend/core/typecheck/tests/paren_return_decomposition.rs | — | — |
| | 3.21.4 生成物与事实不符：警告码页断言「W 族不会阻止编译」，与 W1090 实际 **error 级**矛盾（生成器写死散文）。修后须**重跑** `scripts/docs/gen-error-code-docs.py` 再生，**禁止手改生成物**（✅ 3b6df553；issue [#470](https://github.com/ChenXu233/YaoXiang/issues/470)、[#509](https://github.com/ChenXu233/YaoXiang/issues/509)） | scripts/docs/gen-error-code-docs.py、docs/src/reference/ | — | — |
| | 3.21.5 #469 后续残留五项收账——诊断文案 locales 单源收敛（issue [#509](https://github.com/ChenXu233/YaoXiang/issues/509)） | src/util/diagnostic/codes/builder.rs、src/util/diagnostic/codes/tests/codes.rs、src/frontend/proof_execution.rs、src/frontend/tests/proof_execution.rs | 3.21.4 | — |
| 3.23 清单解析失败 / 缺必填字段的静默退化（issue [#503](https://github.com/ChenXu233/YaoXiang/issues/503)；来源 = 4.9.8 施工期实测）：清单不可读时 CLI 不报清单错误、退化为单文件语义 ⇒ 跨文件引用池失效 ⇒ 误报 `W1002`×2 且 `check` 退出 0 | 3.23.1 见下 | `src/driver/helpers.rs`（清单反序列化吞错面）、`src/package/manifest.rs`（`version` 为必填字段） | — | **终态判据**：见 3.23.1 |
| | 3.23.1 清单不可读时的静默退化：非法 TOML 或缺 `[package] version` 时，清单反序列化以 `.ok()` 吞掉错误 ⇒ 显式声明面为空 ⇒ 引用池按单文件语义装配 ⇒ 两个都被入口引用的类型被误报 `W1002`，且 `check` 仍退出 0（✅ 81d86810；issue [#503](https://github.com/ChenXu233/YaoXiang/issues/503)） | src/driver/helpers.rs | — | — |
| | 3.23.2 配置解析失败静默回退：`[tool.test]` 类型错误穿透 PackageManifest 后被 helpers.rs `.ok()` 静默吞（patterns 静默回退内建约定）/ run / format / test 三命令 yaoxiang.toml 及用户配置读失败解析失败静默默认（issue [#520](https://github.com/ChenXu233/YaoXiang/issues/520)） | src/driver/helpers.rs、src/main.rs、src/util/config/mod.rs、src/util/test_runner.rs | 3.23.1 | — |
| 3.24 模式匹配补全余项（RFC-010b 范围；issue [#405](https://github.com/ChenXu233/YaoXiang/issues/405)；RFC-010b 正文不改，SPEC syntax.md 是唯一规范载体） | 3.24.1 见下 | 见 3.24.1 行 | — | **终态判据**：见 3.24.1 |
| | 3.24.1 范围模式 RangePattern——match 双闭区间判定（issue [#405](https://github.com/ChenXu233/YaoXiang/issues/405)） | src/ast/mod.rs、src/frontend/core/parser/pratt/nud.rs、src/frontend/core/parser/tests/patterns.rs、src/frontend/core/sema/inference/expressions/expr.rs | — | — |
| | 3.24.2 Eq/Neq 非记录形态对齐「两侧同型」纪律——异型比较编译期拒绝（issue [#549](https://github.com/ChenXu233/YaoXiang/issues/549)） | src/frontend/core/sema/inference/expressions/expr.rs、src/backends/interpreter/executor/executor.rs、tests/integration/check.rs、tests/integration/multifile.rs | 3.24.1 | — |
| | 3.24.3 导入类型的返回注解误拒修绿——导入面名义名改绑（issue [#504](https://github.com/ChenXu233/YaoXiang/issues/504)） | src/frontend/core/types/substitute.rs、src/frontend/core/types/tests/substitute.rs、src/frontend/core/sema/inference/statements.rs、src/frontend/core/sema/checker.rs | 3.24.2 | — |
| | 3.24.4 native 遮蔽用户同名函数——lower 面裸名解析用户优先（issue [#477](https://github.com/ChenXu233/YaoXiang/issues/477)） | src/middle/lower/mod.rs、src/middle/lower/module_ir.rs、src/middle/lower/query.rs、src/middle/lower/expr/call.rs | 3.24.3 | — |
| | 3.24.5 W 码逐条抑制——check 命令 `--allow`（issue [#499](https://github.com/ChenXu233/YaoXiang/issues/499)） | src/main.rs、src/util/diagnostic/command.rs、src/util/diagnostic/mod.rs、src/util/diagnostic/tests/mod_tests.rs | 3.24.4 | — |
| | 3.24.6 fn[pos] 超界位置字面量编译期拒绝——静默截断与取负 panic 收口（issue [#563](https://github.com/ChenXu233/YaoXiang/issues/563)） | src/ast/mod.rs、src/ast/tests/binding_positions.rs、src/frontend/core/parser/statements/types.rs、src/frontend/core/parser/statements/tests/types.rs | 3.24.5 | — |
| 3.25 词法错误归因：`\x` 非法形态被「空字符字面量」覆盖（D4；**2026-11-24 登记**，来源 = 8.8 修复轮复审 R3 的 D4（见任务 8.8 的提交信息）；用户裁决「另立 P3 任务修」） | 3.25.1 见下 | 见 3.25.1 行 | — | **终态判据**：见 3.25.1 |
| | 3.25.1 `'\x1'` 的错误归因：`decode_hex_byte` 已发 `InvalidEscape`，随后 `scan_char` 的「空字符字面量」把它覆盖成 `InvalidToken{message: "Empty character literal"}` ⇒ 同一非法形态在字符串位报 E0012、在字符位报「空字符」 | `src/frontend/core/lexer/literals.rs`（`scan_char` / `decode_hex_byte`） | — | D59 **C 类**（静默给错错误码）：先红后绿——红态 = `'\x1'` 报 `InvalidToken/"Empty character literal"`（实测，见任务 8.8 的提交信息）；绿态 = 仍以 `InvalidEscape`（E0012）报出且位置指向 `\x`；不改变「原先成功」程序（`'\x1'` 本就失败）；不改码表（InvalidEscape 仍映射 E0012） |

**验收**：漏洞判据转绿，且**故意撤掉修复必须重新变红**。**风险**：多文件路径新增 E4018 是破坏性变更。

**3.4 验收**：3.4.1 / 3.4.2 必须先有红判据（禁止先改代码）；3.4.3 为人工 review；3.4.4–3.4.8 以 C2/C3 与 P9 的 9.2 门禁为准。**风险**：3.4.1 新增诊断属行为变更，须同步语料差分基线与 CHANGELOG。

> **止血通道**：P3 不必等 P2 全部完成。3.1/3.2 的唯一硬前置是 **2.3.2（多文件语料层）+ 2.4.1/2.4.3（两个红判据）**；2.1/2.2/2.3.1/2.3.3（IR 校验器、快照、单文件差分、性能基线）可与 P3 并行推进。正确性漏洞（`Sorted(3)` 静默通过）的止血不应被快照基础设施建设阻塞。3.3（panic 转诊断）无前置，可随时落地。

### 外部轨道 RFC-029g：删除 `pub` 与自动绑定（外部轨道 / #399 —— 排序约束 D56）

来源：[RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)（2026-10-02 已接受、实现未开工）、[#399](https://github.com/ChenXu233/YaoXiang/issues/399)。**本轨道不是重构自身的设计项，是被排序约束强制前置的规格符合性删除**；与 P10 S5「`pub` 项降可见性」（Rust 的 `pub(crate)` 收窄）是两件事。

**为什么必须整体先于 P4**：029g 的删除面与四个重构阶段的重写面重合，反序 = 在新文法 / 新 checker 布局 / 新 SSA IR 上重做同一批删除与基线更新。

| 029g 删除面 | 文件 | 被哪个阶段重写 |
| --- | --- | --- |
| `KwPub` 与语法分支 | `lexer/{tokens,state,mod}.rs`；`parser/parser_state.rs`、`statements/{declarations,imports,functions,bindings}.rs` | **P8**（8.2 词法收敛 / 8.3 Pratt 优先级收敛 / 8.7 拆 `parse_assign_after_target`） |
| AST `is_pub` ×2（`Assign` / `TypeDefinition`） | `parser/ast.rs` | **P6 6.5 AST 死变体** + P8 + D1 目录改名（`parser/ast.rs` → 顶层 `src/ast/`） |
| `auto_bind_to_type` 与 `collect_exports` 的 pub 分支 | `typecheck/checker.rs`、`environment.rs` | **P5**（`include!`→真 `mod` 与 checker 拆分是**纯搬迁**，pub 分支会被原样搬进新文件） |
| 死码豁免 `exempt_pub` / `is_exported` 与角色分派 | `typecheck/passes/dead_code.rs`、`module/orchestrator.rs`（`check_project` 体内） | **P4 4.2.2**（`check_project` 瘦身） |
| `semantic_tokens` 的 `Public` modifier | `typecheck/checker/semantic_tokens.rs` | **P4 4.2.6** + P5 + 4.6（D55） |
| `ir_gen` 的 `is_pub` 忽略绑定 | `middle/core/ir_gen.rs` | **P7**（`ir_gen.rs` SSA 化） |
| `src/std/` 31 处 `pub` | `std/list.yx` / `json.yx` / `option.yx` / `result.yx` | 无阶段直接改，但 **P6 硬验收要求 `src/std/` 无差异（`git diff --stat src/std/` 为空；`tests/` 直驱测试例外见 P6 硬验收段）**——只能在 P6 之前落 |

**附带收益**：029g 删除 `env.exports` 死表（生产端灌表、全仓零消费）与 `is_exported`/`is_visible` 两个访问器 → P4 的 4.3 义务账本少一个「产出但无人消费」字段。

- [x] **029g.1 词法与 parser 分支**
- [x] **029g.2 AST `is_pub` ×2**
- [x] **029g.3 checker 侧**
- [x] **029g.4 死码豁免收敛**
- [x] **029g.5 外围回显**
- [x] **029g.6 std 与诊断文案**
- [x] **029g.7 语言参考收敛**

| 二级 | 三级 | 文件 | 验收 |
| --- | --- | --- | --- |
| 029g.1 词法与 parser | 029g.1.1 `KwPub` 变体与 `keyword_from_str` 条目删除 | `lexer/tokens.rs`、`state.rs`、`mod.rs` | 关键字表 18→17（`Kw*` 16→15，含 `tokens.rs` 计数注释） |
| | 029g.1.2 声明位的 pub 探测与 `pre_detected_pub` 删除 | `parser/statements/declarations.rs`、`functions.rs`、`bindings.rs` | `pub f = ...` 报解析错误（**红判据先行**） |
| | 029g.1.3 use 花括号条目内的 pub 跳过与语句前导 pub 删除 | `parser/statements/imports.rs`、`parser_state.rs` | `pub use m` 报解析错误（**红判据先行**） |
| 029g.2 AST | 029g.2.1 `Assign`/`TypeDefinition` 的 `is_pub` 删除 + 构造点同步 | `parser/ast.rs` 与各构造点 | 编译器内无 pub 残留（`grep -rn is_pub src/` 为空） |
| 029g.3 checker | 029g.3.1 `auto_bind_to_type` 删除（pub 是唯一调用理由） | `checker.rs`、`environment.rs`、调用点 `checker.rs` | 方法形式只剩显式组合（RFC-004），std 与语料零依赖 |
| | 029g.3.2 `collect_exports` 去 pub 分支 + `env.exports` 死表与 `is_exported`/`is_visible` 删除 | `checker.rs`、`environment.rs` | 导出面唯一权威 = 模块注册表 `ModuleInfo.exports`；9.2 门禁少一字段 |
| 029g.4 死码豁免 | 029g.4.1 `exempt_pub` / `is_exported` 与豁免分支删除 | `dead_code.rs` | Lib/Script 未引用的顶层绑定报 W1001（**红判据先行**） |
| | 029g.4.2 `orchestrator` 角色分派同步 | `orchestrator.rs` | 单文件 / 多文件诊断集归一（C2），W 码不混入消费方 |
| 029g.5 外围回显 | 029g.5.1 formatter 的 pub 前缀 | `formatter/handlers/stmt.rs` | format 幂等（既有判据） |
| | 029g.5.2 `semantic_tokens` 的 `Public` modifier | `typecheck/checker/semantic_tokens.rs` | 语义着色不再产出 Public |
| 029g.6 std 与文案 | 029g.6.1 `src/std` 31 处 pub 前缀删除 | `std/{list,json,option,result}.yx` | 语料与 std 自检全绿；**须在 P6 前完成** |
| | 029g.6.2 诊断文案 | 4 条 W1xxx help | 不再宣称「pub 是对外接口」；与 `dead_code.rs` 的 Bin 角色行为对齐 |
| 029g.7 文档 | 029g.7.1 语言参考与指南 | `reference/language-spec/{modules,syntax,language-overview}.md`、`warning-code/warning-codes.md`、`guide/modules.md` | 规范不再出现与实现相反的「pub 仍可写」表述；`warning-codes.md`「pub 永不触发 W1001」一并修正 |

**验收（行为变化，红判据先行）**：

1. **先红后绿**：两条行为变化各自先落红判据再动代码——① `pub` 从「静默接受」变为解析错误；② Lib/Script 角色下未引用的顶层绑定新报 W1001（原先 pub 项绝对豁免）。
2. **测试数不得下降**：删 `typecheck/passes/tests/dead_code.rs` 的 `exempt_pub` 用例与 `is_pub` fixture 时，必须由等价新用例替换。
3. **基线同提交**：诊断集变化（新解析错误 + 新 W1001）→ 语料差分基线（`tests/baselines/corpus-parity.jsonl`）与 CHANGELOG 同提交更新；IR 快照预计 zero-diff，不为零须说明原因。
4. **独立 revert 单元**：整批占一个 commit 组，不与其他阶段混提（DoD 6）。
5. `cargo test` / `clippy -D warnings` / `python scripts/rfc/check_tracking.py` 全绿。

**排序**：与 3.4 文件重叠（`checker.rs`、`environment.rs`）→ 两者串行、不得并行；**整体先于 P4**（RFC-039 D56）。029g 轨道不依赖 3.4，先做哪个由实施者按在手上下文定。

> **提交对照（2026-10-08）**：本节的收编（P3.5 章节 + 主链/约束行）与 RFC-039 的 D56，在并行流清扫工作区时被一并并入 `97071c81b`——该提交的消息记的是「3.4 补遗登记 + D57/D58 裁决落档」，未提及本轨道。历史不改写，按既有体例（`d5b18390` 先例，见 P2 落地注）就地登记，作为事后二分依据。
>
### P4 阶段契约与统一 Driver

来源：[02](02-stage-contract.md) §实施要点 S1–S5。**本阶段与 P3 的修复必须分两个 commit。**

- [x] **4.1 声明式阶段表**（5 个三级任务）
- [x] **4.2 入口合并**（9 个三级任务）
- [x] **4.3 义务账本**（2 个三级任务）
- [x] **4.4 证明层与 wasm 收尾**（4 个三级任务；✅ `d07993157`、`657c3d5e3`、`1b593e993`；issue [#475](https://github.com/ChenXu233/YaoXiang/issues/475)；归属阶段：P4；是否阻塞主线：是）
- [x] **4.5 跨层契约 PlanId 化**（2 个三级任务）
- [x] **4.6 LSP 语义数据管线统一**（2 个三级任务；归属阶段：P4；是否阻塞主线：是）
- [x] **4.9 引用池正确实现（RFC-029f 修正）**（10 个三级任务：4.9.1–4.9.10；4.9.9 已完成（收窄后复审 pass），4.9.10 已实施（L0 / S1 / S2a / S2b 四笔）；✅ `3863a9ff0`、`e6cf1aeaa`、`ddcbb12d0`；issue [#465](https://github.com/ChenXu233/YaoXiang/issues/465)；归属阶段：P4；是否阻塞主线：否）
- [x] **4.10 多文件语义缺口**（8 个三级任务；issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467)、[#476](https://github.com/ChenXu233/YaoXiang/issues/476)、[#491](https://github.com/ChenXu233/YaoXiang/issues/491)；归属阶段：P4；是否阻塞主线：是）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 4.1 声明式阶段表 | 4.1.1 `stage.rs`（`Stage` 变体 + `Stage::ALL` + `StageScope`） | P0 | — |
| | 4.1.2 `program.rs` / `unit.rs` | 4.1.1 | — |
| | 4.1.3 `Pipeline::run` 改走 Driver | 4.1.2 | — |
| | 4.1.4 阶段覆盖断言 | 4.1.3 | — |
| | 4.1.5 全语料 zero-diff | 4.1.4 | — |
| 4.2 入口合并 | 4.2.1 `compile_project` 瘦身为 Program 构造器 | 4.1.5 | — |
| | 4.2.2 `check_project` | 4.2.1 | — |
| | 4.2.3 `check_source_in_project` | 4.2.2 | — |
| | 4.2.4 `compile_embedded_module` | 4.2.3 | — |
| | 4.2.5 删 `check_single_file` | 4.2.1 | — |
| | 4.2.6 删 LSP 手工阶段序列 | 4.2.1 | — |
| | 4.2.7 `Aggregation` 参数驱动二选一 | 4.2.1 | — |
| | 4.2.8 wasm 改走 `ProgramKind::WasmPlayground` | 4.2.1 | — |
| | 4.2.9 `driver/mod.rs` 拆分为 `state.rs`/`arms.rs`/`helpers.rs` | 4.2.4 | — |
| 4.3 义务账本 | 4.3.1 `obligations.rs` + `assert_drained()` | 4.1.3 | — |
| | 4.3.2 义务诊断 W→E 升级 | 4.3.1 | — |
| 4.4 证明层与 wasm 收尾 | 4.4.1 `layers/README.md` 层序改实际顺序（归属阶段：P4；是否阻塞主线：否） | — | — |
| | 4.4.2 SMT 求解器获取面统一为进程级共享单例（✅ `b5e68b3c9`；归属阶段：P4；是否阻塞主线：否） | — | — |
| | 4.4.3 降级路径加 warning（`checker.rs` 的 `TypeChecker::ownership_inputs` 判据点）（归属阶段：P4；是否阻塞主线：否） | — | — |
| | 4.4.4 SMT 共享求解器观测账 `ConsumerStats`/`CONSUMER_STATS`：生产路径每点都写、生产侧 0 读取（读取口只有 `proof/smt/tests/shared.rs`）。**2026-10-24 裁决 ① 保留**——把「唯一消费者 = 测试」写成 doc 契约（保 D58 可观测判据载体）；裁决 ②「删掉」不取（须同时改写 4.4.2 的验收判据，取舍理由 4.4 收口「十六」）。**终态**：契约三句写进 `src/frontend/core/typecheck/proof/smt/backend.rs` 的模块头与相关项文档注释——① 写入面 = 经共享口每次获取记一次原子累加；② 读取面**只有** `proof/smt/tests/shared.rs`（生产代码零调用）；③ `SolverConsumer::ALL` 是**测试专用常量**，生产代码不得依赖；**零行为变化**（改动面只有注释，4.4.2 的验收数字未动；issue [#475](https://github.com/ChenXu233/YaoXiang/issues/475)） | 4.4.2 | — |
| 4.5 跨层契约 PlanId 化（D20） | 4.5.1 `ReleasePlan` 键 `Span` → `PlanId` | 4.3.1 | — |
| | 4.5.2 `overload_resolutions` 键 `Span` → `PlanId` | 4.5.1 | — |
| 4.6 LSP 语义数据管线统一 | 4.6.1 项目内文件的语义数据（SemanticDB）走 orchestrator 同管线，与诊断同源；跨文件引用带 `resolves_to` | 4.2.1 | — |
| | 4.6.2 跳转/悬停/重命名的查库键改用光标下标识符自身的起点（**2026-10-16 同日完成**——`resolve_reference` 按引用起点精确匹配（D55 precise-only），光标在标识符中部时静默落空；definition/hover/references/rename 四 handler 统一改键，references/rename 的定义点回退判定对标识符起点同构成立；钉板：四 handler 中部光标各 1 枚 + 跨文件 e2e 双位置断言） | 4.6.1 | 光标在标识符任意位置与起点同效 |
| 4.9 引用池正确实现 | 4.9.1 池的根按 `ExplicitSurfaces` 解析（`collect_package_refs`，`helpers.rs:481`）：按 `[[bin]]`/`[lib]`/`[exports]`/`[run].main` 根调 `discover_with_used` 取编译集，**不扫目录找入口**；全根退化时退路 `collect_compilation_refs`（`helpers.rs:454`）。**2026-10-16 轮 1 完成（提交 `3863a9ff0`）** | 4.2.1 | `collect_yx` 不再递归扫描目录；**corpus 语料 diff 零未归因新增 W1001**（每条新增警告必须可追溯到某个真实未使用定义） |
| | 4.9.2 粒度升级为 `(module, name)`：池类型改为 `HashMap<String, HashSet<String>>`（键 = 模块键），装配唯一入口 `collect_ref_pool`（`helpers.rs:429-449`），消费端在 `arms.rs:603-609` 的 `other_module_refs` 把「除本单元外」的全部模块条目并集后送 `DeadCodeAnalyzer`；模块自有名（顶层定义与局部绑定，`use` 导入名例外）不再豁免别的模块同名定义。**2026-10-16 轮 1 完成（提交 `3863a9ff0`）** | 4.9.1 | 同名局部变量不再意外豁免顶级定义；**单测：a.yx 的 `foo` 和 b.yx 的 `foo` 独立计数** |
| | 4.9.3 Script 不用池：`arms.rs:611-623` 的 `match us.role` 只对 `Bin`/`Internal`（`set_exempt_pub`）与 `Lib`（`set_cross_file_refs`）装配，Script/Test 走 `_ => {}` 默认臂（按文件内定义-使用图判定）。**2026-10-16 轮 1 完成（提交 `3863a9ff0`）** | 4.9.2 | Script 单文件语义恢复；**单测：项目内其他文件的引用不影响 script.yx 的 W1001** |
| | 4.9.4 消费定义细化：区分"被 import"和"被 import 后真实使用"——只有后者算消费（需 `use` 项的路径解析 + 实际使用点追踪）。**2026-10-17 轮 2 完成（提交 `e6cf1aeaa`——`RefPool::consumed_from` 按 import 归因到定义模块，`resolve_import_module` 复用发现期同一份模块键集合）** | 4.9.3 + 4.6.1 | `use a::foo` 但从不调用 `foo()` 的情况下，a.yx 的 `foo` 仍报 W1001；**硬前置**：`use` 项路径解析（4.6.1 交付） |
| | 4.9.5 测试池分离：tests/ 的引用单独成池，不豁免生产代码（src/）的未使用定义。**2026-10-17 轮 2 完成（提交 `e6cf1aeaa`——`FileRole::Test` 模块不入消费池，退化路径与声明面路径两来源都喂角色）** | 4.9.4 | **单测：tests/helper.yx 引用 src/util.yx 的 `debug_print`，但 src/ 内无消费者时，`debug_print` 仍报 W1001**；RFC-029f 表格更新（测试豁免范围缩窄到仅测试自身） |
| | 4.9.6 方法可达性口径（`#450`；2026-10-13 登记 → **2026-10-16 用户并入轮 2**）：无项目根路径下**被调用且重载决议生效的方法**仍报 `W1005 Unused method`。现状机制（✅ `ddcbb12d0`、`b8380e6ab`；归属阶段：P4；是否阻塞主线：否） | 4.9.3 | — |
| | 4.9.7 跨单元方法调用的决议登记与接收者归因（**2026-10-17 登记**，issue [#464](https://github.com/ChenXu233/YaoXiang/issues/464)）：跨单元（registry 解析）的方法调用不产生决议条目 ⇒ 回落池的名字级豁免（`RefPool::field_names` 全局平铺）⇒ 跨单元同名同序的重载定义此前不被报；依赖 D20 产出侧扩展（决议登记带接收者类型身份）。**2026-10-17 轮 3 完成（产出侧 `b8b3cc3f6` + 驱动侧 `d5c00235a`；注入面误报修复轮 `d5dd1a37b` + 钉测试归位 `c2f887843`）**；**归属阶段：P4；是否阻塞主线：否** | 4.9.6 | **单测：跨单元同名同序的两个重载定义，仅一个被调用时另一个仍报 W1005**（先红后绿，钉在 `src/driver/tests/method_unused.rs` 的 `test_cross_unit_uncalled_same_named_method_is_reported`） |
| | 4.9.8 跨单元同名类型经**内联别名**/模块限定引入时方法派发落到**裸名归属单元**（静默错值；`check` 的警告面取决于清单是否可读——清单可读时 0 warning，清单缺 `version` 时 2 条 `W1002`，根因在清单解析面而非本判据）；D59 **C 类**；**2026-10-25 完成（含修复轮与异家族复审轮 2 pass）**——判别实验定死机制 = D20 决议覆盖 + 裸名前缀（`type_bindings` 在本形态为空 ⇒ 不计短名键碰撞）：两处调用都命中 `overload_resolutions` 且值为定义侧裸名 `Temp.tag`，再由 `qualify_names` 的别名前缀落到裸名提供者 ⇒ 判据改在派发名（写出名来源 = 注解 / 构造器，局部与顶层绑定同链）；**归属判定唯一在 `imported_type_owner`**（`src/middle/core/ir_gen.rs:1232`，调用点只有 `method_dispatch_name` 内的两处；权威限定名精确命中即返回、不再解析），**`method_dispatch_name`**（`:1265`）的生产调用点 **5 处**：绑定臂（`:8318`）/ 决议臂（`:8379`）/ 裸名臂（`:8532`）/ **Index 臂（`:6444`）** / **二元接口臂（`:7858`——全部「接口驱动二元运算」的共享臂：Equal + Add/Subtract/Multiply/Divide/Modulo；实测 `u + u` 由 `3.0` 转 `20.0`；029f 侧该臂的覆盖面表述已按实测更正）**；先红后绿：`CROSS_UNIT_A/B` 方法体改为可区分（`+1.0` / `*10.0`）+ 值级断言，钉测试 = 驱动级 **10 条**（`src/driver/tests/method_dispatch.rs`）+ 集成 **5 条**（`tests/integration/multifile.rs`），全为值级断言；变异关闭判据 ⇒ 跨单元用例翻红（调用名回 `a.Temp.tag`）、本地与阴性对照不回归，补丁机械可逆（`git apply -R` 后与被变异文件逐字节一致）；语料 356 行零 diff / IR 快照 216 文件零漂移（只改调用名归属，不改 IR 形状）；残余边界（写出名不可得时回落裸名；内联别名 × 同名重载沿用 4.9.7 候选收窄规则；跨单元值绑定转手；清单解析失败 / 缺必填字段时静默退化——已登记 3.23.1 / issue [#503](https://github.com/ChenXu233/YaoXiang/issues/503)）见 029f §跨单元同名类型的方法派发归因 | 4.9.7 | issue [#465](https://github.com/ChenXu233/YaoXiang/issues/465) |
| | 4.9.9 跨单元单定义方法的**命名实参**调用丢形参名表（2026-11-25 登记，7c 收口期发现）：跨单元（registry 注入）的方法用命名实参调用时报 `E1014 Function '…' has no parameter named '…' (available: )`——名字表为空；同方法的位置实参调用正常、单文件内的命名实参调用正常。最小复现：`lib.yx` 定义 `Point.scale_by: (self: &Point, k: Float) -> Float`，`main.yx` 里 `p.scale_by(k = 2.0)` ⇒ E1014，`p.scale_by(2.0)` ⇒ 6.0。**D59 分类：C 类**（旧行为非成功：命名实参静默丢参后运行期失败或静默错值）。**范围（2026-11-26 两轮收窄后的最终形态——只加三项，其余全是删除与登记）**：① `AstToIrGenerator::method_param_info`（`src/middle/lower/query.rs`）——按定义单元限定名 `{module}.{Type}.{method}[#N]` 从唯一跨单元载体 `ModuleInfo::method_overloads` 取声明期形参名（候选 ↔ 混编 IR 名 ↔ 形参名三向量同序，与 typecheck 注入面同一份）；调用点（`src/middle/lower/expr/call.rs`）用 `method_dispatch_name` 的限定名查名字表，不用两段裸名。这两处只修**裸名导入**（`local == target`）的跨单元方法：候选注入面维持 pre-4.9.9 的收窄规则（`local != target` 跳过），故别名导入的命名实参不在修复面内（见「已知缺口」）。② `ModuleRegistry::all_method_bindings`（`src/frontend/module/registry.rs`）按**模块键升序**合并——**C 类（独立于原始 bug）：消除 HashMap 迭代序导致的「同一输入多种结果」**，不是原始 bug 的修复；判据 = 同文件单元测试 `test_all_method_bindings_merge_is_deterministic_across_hash_seeds`（32 个插入序交替的注册表实例，同一方法键只允许一个单绑定；去掉升序合并后出现两种结果）。**判据（先红后绿）**：驱动级 `src/driver/tests/named_args_cross_unit_method.rs` 2 条（裸导入单候选 6.0 / 两候选按名取 6.0 与 30，均带值断言；文件头标病史门）；单元 `src/middle/lower/tests/query.rs`（`method_param_info` 空名表守卫 ⇒ None，正对照非空表原样返回）；单元 `src/frontend/module/registry.rs`（上列确定性判据）。项目级夹具 `tests/yaoxiang-multifile/named-args-cross-unit-method`（`alias-overload` 夹具随别名面退役一并删除）；语料差分基线 `tests/baselines/corpus-parity.jsonl` 现 404 行：**相对直接父提交 `ac810e97` 为 405→404**（中间态曾含 `alias-overload`）；**相对 pre-4.9.9 基线 `08ccacc5` 为 403→404**（+1 基础夹具 `named-args-cross-unit-method`，`alias-overload` 未进入最终历史）。行数变化 = 夹具进出的必然结果（`corpus_probe` 重生成后与入库基线逐字节一致），**不是覆盖丢失**。**已知缺口（逐项点名；旧行为 = 静默忽略或静默丢弃；D60② 该报未报，A 类）**：闭包值绑定 / 块体声明函数 / 具名嵌套函数声明 / 内置容器构造非 `len` 命名实参 / 整体模块导入 / RFC-004 绑定臂。**其中两条按 P0 实测更正口径**：① **静态函数形态**（本单元与跨单元、定义式与绑定式）今天**全部不可用**——本单元 `check` 报 E3006，跨单元定义式 `run` 报 E6007，跨单元绑定式 `run` 报 E6006；位置与命名实参同码，与命名实参无关，不是「命名实参被静默丢弃」；② **RFC-004 跨单元的位置实参同样失败**（`check` 通过、`run` E6006），根因是派发名不在合并后的函数表里，同样与命名实参无关。**已修（4.9.10-S2b，D59 A 类顺路修：旧行为 E6007/E1014 非成功，无原先成功程序变失败）**：① **别名导入命名实参**（`use a.{Temp as X}` + `t.bump(k = 2.0)`）；② **命名空间臂**（`lib.Temp(..)` 之后的命名实参）；③ **推断接收者命名实参**（接收者类型靠推断、无写出注解）。三者此前分别是运行期 E6007 静默丢参与 `check` 期 `E1014 (available: )` 假拒绝，现均按声明期形参名入槽（值断言 6.0；判据 `src/driver/tests/named_args_import_shapes.rs`，红态见 `src/driver/tests/named_args_import_shapes.rs` 文件头病史门）。另有两条身份面残余，其一已收口：本单元自有同名类型 + 别名导入（两个同名定义侧身份共存、导入面登记不到自有声明）**已随 4.9.10-S2a 的导入面收口**（项目夹具 `same-name-type-local-and-alias-import` 全绿）；剩余一条：函数体级/跨作用域导入按单元全局合并（共享导入表）——该形态本身不工作（既有缺陷：check 通过、run E6006），非本批引入、未修。**本轮删除（相对 4.9.9 前六轮）**：候选提供方优先级、候选签名比较、owner/相容推理、`frontend::module::import_face` 共享提供方表、IR 侧「拿不到名表 ⇒ E1014」兜底，以及**导入名解析点的 E5006 歧义禁令**——后者拒掉父提交上本来可运行的 mixed/别名-only 形态（D 类），2026-11-26 裁决撤回；本轮不引入任何新的歧义判定。**可见变化**：① 跨单元方法命名实参（**裸名导入**、含两候选重载）由 E1014 空表转为按名入槽（实测 6.0 / 30）；② 其余形态回到 pre-4.9.9 行为（命名实参静默忽略或丢弃，见「已知缺口」）；③ 同一输入的方法单绑定不再随 HashMap 迭代序变化（C 类）。**状态**：2026-11-26 完成（含收窄轮与复审 pass）。 | 4.9.7 | 判据见 `src/driver/tests/named_args_cross_unit_method.rs` 与 `src/driver/tests/named_args_import_shapes.rs`（后者随 4.9.10-S2b 增设）、`src/frontend/module/registry.rs`；根治见 4.9.10 |
| | 4.9.10 跨单元方法身份未定义（2026-11-25 登记，4.9.9 施工期确认的结构性问题）：方法候选键 `{Type}.{method}` 不含模块限定，同名类型跨模块时三个消费者——候选注入（`driver::arms::visible_method_overloads`）、typecheck 的形参名/候选匹配、IR 派发（`method_dispatch_name`）——各自按名字猜提供方，任一处改动都把「静默错值」挪到另一处。**根治方案**：类型身份升级为**模块限定身份**（`{module}.{Type}`），方法候选键随之改为 `{module}.{Type}.{method}`；由一处解析产出、三处只读。需动 `overload_resolutions` 的键形态与混编名，D59 **D 类**（须裁决）。**本轮处置（4.9.9 两轮收窄后的口径）**：4.9.9 **不引入任何新的拒绝**——前六轮的 D 类拒绝清单（命名实参一律 E1014 / 冲突形态位置实参一律 E1107 / 导入名解析点 E5006 歧义禁令）随机制一并删除、全部作废；父提交上可运行的 mixed / 别名-only / 冲突形态的**导入形态与位置实参**恢复可运行。**残余面（本轮收口后）**：① 函数体级/跨作用域导入按单元全局合并（共享导入表），该形态本身不工作（既有缺陷：check 通过、run E6006）；② 命名实参仍静默忽略或丢弃的形态（D60② A 类）：**静态函数形态**（本单元 `check` E3006；跨单元定义式 `run` E6007；跨单元绑定式 `run` E6006——位置与命名实参同码，与命名实参无关）、**RFC-004 跨单元**（位置实参同样 E6006）、**闭包值绑定**、**块体声明函数**、**内置容器非 `len`**（`size` 等形态：命名实参被忽略、`length` 为 0，运行期 E6005；`len` 形态正常）。**已随本轮收口的三项**：① 方法候选键/方法键改为模块限定 `{module}.{Type}.{method}`（L0 的 `DefPath` 双代表 + S2a 的表键与单点解析；登记时点名的三个消费者——`visible_method_overloads`、形参名反查 `method_param_info`、`method_dispatch_name`——已全部删除）；② 本单元自有同名类型 + 别名导入（两个同名定义侧身份共存）；③ 别名导入 / 命名空间臂 / 推断接收者的命名实参（三形态转绿，见 4.9.9 同段）。**L3 类型身份残余（迁移面，未修；L0 时同红，非本轮回归）**：① 类型名本身仍未模块限定——`MonoType` 里的静态类型名是**定义侧裸名**，同名类型跨单元在类型层仍分不出来，只能靠导入面 + 绑定身份绕开；② `VarInfo::type_identity` 只覆盖**局部绑定**登记处（写出注解 / 构造器写出路径），**形参、模式绑定、闭包形参、`for` 变量**未登记——这些位置的接收者仍只能按静态类型名解析（`set_type_identity` 全仓仅一个调用点）。**状态**：已实施（L0 / S1 / S2a / S2b 四笔）。 | 4.9.9 | 先给不变量与迁移面清单，再裁决是否实施：不变量 = typecheck 采用的候选集 ≡ IR 实际派发目标同源；迁移面 = 候选注入 / 形参名表 / 派发名 / `overload_resolutions` 键 / 混编名 / `all_method_bindings` 单绑定兜底 / 上述残余面（两条） |
| 4.10 多文件语义缺口 | 4.10.1 check 路径 parse 错误硬中止降级 | 4.2.2 | — |
| | 4.10.2 discover 自动注入 std.list（对齐单文件 `merge_embedded_std_ir` 的 #117 行为） | — | — |
| | 4.10.3 standalone run 与 MultiFile 编排统一（4.2.5 的 run 半边，用户指示**插队**） | 3.4.8 | — |
| | 4.10.4 rootless 多文件 `run` 的各单元初始化序列按**源文件路径字典序**拼接（issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467)） | 4.10.3 | — |
| | 4.10.5 rootless 定义 `main` 却未调用时打印提示但**退出码 0** ⇒ CI 误判成功；退出码属 D59 **D 类**（落地前需确认）——**2026-10-22 已完成**（issue [#476](https://github.com/ChenXu233/YaoXiang/issues/476)） | 4.10.3 | — |
| | 4.10.6 **导入环的诊断**（✅ `28c70a518`；issue [#491](https://github.com/ChenXu233/YaoXiang/issues/491)；归属阶段：P4；是否阻塞主线：否） | 4.10.4 | — |
| | 4.10.7 **原生函数作值**（✅ `19deb87d3`；issue [#492](https://github.com/ChenXu233/YaoXiang/issues/492)；归属阶段：P4；是否阻塞主线：否） | 3.6.5 | — |
| | 4.10.8 **纯 YaoXiang 的 std 导出作值**（✅ `5a2fa8829`、`19deb87d3`；issue [#493](https://github.com/ChenXu233/YaoXiang/issues/493)；归属阶段：P4；是否阻塞主线：否） | 3.6.5 | — |

### P5 checker 文件内拆分 —— 本文档补齐（原缺口 G1）

来源：RFC 定义 + `01-routing.md` 目录树 + 硬约束（`collect_used_in_type` 的 `pub(crate)` 路径不得变）

- [x] **5.1 `include!` → 真 `mod`**（2 个三级任务）
- [x] **5.2 拆出 `refinement`**（2 个三级任务）
- [x] **5.3 拆出 `annotations`**（2 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 5.1 `include!` → 真 `mod` | 5.1.1 `checker.rs` 末尾的 `include!` 改为真 `mod` 声明（行锚不写死，实测当时在文件末行） | P4 完成 | C1 zero-diff；`semantic_tokens.rs` 获得模块身份 |
| | 5.1.2 补 `use super::*` 或逐项引入（`include!` 时代隐式继承作用域） | 5.1.1 | `cargo build` 绿 |
| 5.2 拆出 `refinement` | 5.2.1 迁出精化块（原 `checker.rs` 的三段非连续区间；现为 `checker/refinement.rs` 单文件，含 `ReturnRefinement` / `RefinedWalkCtx`），保留面与 6 个跨界共享项原地不动 | 5.1.1 | C1 zero-diff |
| | 5.2.2 精化入口收口为「`&self` + 返回诊断」，**不脱 impl**：`collect_refined_binding_checks`（现 `checker/refinement.rs`，`pub(super)`）不再借 `&mut self` 隐式写错误通道，诊断集合作为返回值上抛，写入点唯一落在唯一调用方 `check_module_impl`（`checker.rs`，符号定位）。**不脱 impl 的理由**：该族字段面只有 `self.env`（纯技术面脱 impl 可行），但「入口与走查全转自由函数」会连带外移这些调用点：`resolve_type_annotation` 生产 8 处（6 处注解路径 + `refined_walk_stmt` + `collect_refined_var_names_in_stmt`）、`collect_param_refinements` 生产 2 处 + 测试 1 处，并制造「保留面（签名收集）→ refinement」反向依赖——与该步要消除耦合的意图相反 | 5.2.1 | — |
| 5.3 拆出 `annotations` | 5.3.1 迁出注解校验族 10 项（现 `checker/annotations.rs` 单文件；`check_annotation_type_names` 为 `pub(super)` 入口，其余 9 项族内私有，`is_predicate_head` 为族内自由函数） | 5.2.2 | C1 zero-diff |
| | 5.3.2 **`collect_used_in_type`（`pub(crate)`）保持 `pub(crate)` 且在 `checker` 模块路径下**：它是**共享面**函数（被 `collect_function_signature` 与 `inference/statements.rs:16` 两侧消费），5.3.1 按实测**未迁移它** ⇒ 「零调用方改动」判据**本已满足**，故**不新增** `pub(crate) use annotations::*;` 再导出。**约束保留**：若将来把它随族迁走（归入 annotations 或其它后续批次），必须在 `checker.rs` 加 `pub(crate) use annotations::*;` 再导出，使该 import 一字不改 | 5.3.1 | **零调用方改动**是本步的硬验收：`git diff src/frontend/core/typecheck/inference/statements.rs` 为空 |

**硬约束**：`collect_used_in_type` 被 `inference/statements.rs:16` 跨模块使用。**若**该函数将来随注解族迁走，**则**须在 `checker.rs` 加 `pub(crate) use annotations::*;` 再导出，使该 import 一字不改；5.3 按实测**未迁**它，故本批未新增再导出（判据本已满足）。

**只拆确证的大块。** 不拆只有 200–400 行的模块——增加导航成本、降低内聚性、零解耦收益。其余子模块（signatures / type_defs / imports）待文件长回 1,500 行再拆。

### P6 类型表示单一化

来源：[03](03-type-unification.md) §实施要点 阶段 0–6，判据 C3。**阶段 0→6 严格串行。**

- [x] **6.0 门禁先行**（2 个三级任务）
- [x] **6.1 名称规范化**（3 个三级任务）
- [x] **6.2 字节码类型统一**（3 个三级任务）
- [x] **6.3 死变体删除**（2 个三级任务）
- [x] **6.4 parser 数据流**（5 个三级任务）
- [x] **6.5 AST 死变体**（2 个三级任务）
- [x] **6.6 门禁转硬**（1 个三级任务）
- [x] **6.7 目录改名**（1 个三级任务）
- [x] **6.8 标注检查模式与 Fn 参数表示**（13 个三级任务）
- [x] **6.9 量词读数与证据内核**（5 个三级任务；✅ `46998477`、`cfdf9cf0`；issue [#529](https://github.com/ChenXu233/YaoXiang/issues/529)）

| 二级 | 三级 | 文件:行 | 验收 |
| --- | --- | --- | --- |
| 6.0 门禁先行 | 6.0.1 新建 `tools/type-tables`，实现 T1/T2/T3，**T1 先只报告不失败** | `tools/type-tables/` | — |
| | 6.0.2 在未改动代码上取全量基线 | — | — |
| 6.1 名称规范化 | 6.1.1 统计语料非规范类型名出现次数 | `tests/yaoxiang/` | — |
| | 6.1.2 内置类型名定稿：15 名 + 删 22 个实现内拼写 | `MonoType::from_builtin_name`（`src/frontend/core/types/mono.rs`） | — |
| | 6.1.3 核实 `classify_generic_params` 消费的参数列表 | `src/frontend/core/parser/ast.rs`（`classify_generic_params`） | — |
| 6.2 字节码类型统一 | 6.2.1 三处改 `MonoType` | `BytecodeFunction::params`（`src/middle/core/bytecode.rs:812`）、`BytecodeFunction::return_type`（同文件 :814）、`BytecodeModule::type_table`（同文件 :854）、`Image::type_table`（`src/backends/interpreter/image.rs:44`） | — |
| | 6.2.2 删 `From<MonoType> for IrType` | `impl From<MonoType> for IrType`（`src/middle/core/bytecode.rs`，实测 2353-2390） | — |
| | 6.2.3 `type_table` 元素类型改 `MonoType` | `Image::type_table`（`src/backends/interpreter/image.rs:44`） | — |
| 6.3 死变体删除 | 6.3.1 删 13 个变体 + 清 match 臂 + 兜底（03 §5.2 表体口径：Int/Float/Char/String/Bytes/Bool/Void/Union/Enum/Option/Result/Sum/AssocType） | `ast::Type`（`src/frontend/core/parser/ast.rs`）、`impl From<ast::Type> for MonoType`（`src/frontend/core/types/mono.rs`，实测 919-931）、`Type::Void` 错误兜底（`src/frontend/core/parser/statements/types.rs:836`） | — |
| | 6.3.2 **必须与 6.3.1 同批**改写反向桥与 AST→AST 重建 | `mono_to_ast_type`（`src/middle/passes/mono/function.rs`，实测 505-529，重建其中 7 个）+ `substitute_type_ast` / `substitute_type_in_ast`（同文件的两个替换函数；**区间由 T1 门禁 AST 派生，勿在散文里写死行号**；实测 320-497 与 499-657）里的 `Union`/`Option`/`Result`/`Sum`/`AssocType` 各 2（03 §2.2.2） | — |
| 6.4 parser 数据流 | 6.4.1 新增 `probe.rs` | `src/frontend/core/parser/probe.rs`（新建） | — |
| | 6.4.2 改 `ParserState` | `ParserState`（`src/frontend/core/parser/parser_state.rs`，改前 6 字段：`tokens` / `pos` / `errors` / `no_fat_arrow` / `predicate_names` / `next_plan_id`；**改后 7 字段**——增 `probe`，并增 `probe()` / `kind_of()` 两个访问器） | — |
| | 6.4.3 删 `operator_interfaces::spec()` 调用 | **全仓 13 处**（WBS 计划口径；落地态实测 **14 处**，差额 = driver 侧 1 处）——parser 侧 2 处（`src/frontend/core/parser/ast.rs` 的 `extract_generic_param_names`、`src/frontend/core/parser/statements/declarations.rs`）+ typecheck 侧 10 处（`src/frontend/core/typecheck/checker.rs`、`src/frontend/core/typecheck/inference/expressions.rs`、`src/frontend/core/typecheck/inference/statements.rs`）+ **middle 侧 1 处**（`src/middle/core/ir_gen.rs` 的 `is_constraint_param_ann`——**L3→L3 同层**：`check-boundary.py` 的 `L3_DIRS` 同含 `typecheck` 与 `middle`；WBS 原写「L3→L2 跨层反向依赖」为**层号误记**） | — |
| | 6.4.4 合并两份 `name_used_as_type*` | `name_used_as_type_in`（`src/frontend/core/parser/ast.rs`）+ `name_used_as_type`（`src/frontend/core/parser/statements/declarations.rs`）；**合并后单一实现落在 `src/frontend/core/parser/ast.rs:848` 的 `name_used_as_type`**，declarations 侧那份已删 | — |
| | 6.4.5 `CONST_PARAM_TYPES`（`src/frontend/core/parser/ast.rs:825`）经 `NameKind::Builtin` 接线 | `extract_generic_param_names`（`src/frontend/core/parser/ast.rs`） | — |
| 6.5 AST 死变体 | 6.5.1 删 `Expr::FnDef` | `Expr::FnDef`（`src/frontend/core/parser/ast.rs`；**基线实测 25 处 / 16 文件** = 定义 1 + 生产消费 15 + 测试构造 3 + 注释 6，其中代码引用 19 处 = `Expr::FnDef` 18 + 定义处裸名 1） | — |
| | 6.5.2 删 `Assign.signature_params` + `NamedParen` 迁移 | `StmtKind::Assign::signature_params` + **同名字段 `StmtKind::TypeDefinition::signature_params`**——两项同删（只删 `Assign` 会留下 `TypeDefinition` 的同名字段）；载体改为 `type_annotation: Option<Type>`（`src/frontend/core/parser/ast.rs:258` / `:280`），唯一取用入口 `Type::fn_params_flat`（同文件 `:985`）；`Type::NamedParen`（同文件 `:533`）**保留**为语法节点，binder 由类型检查器显式解释（`src/frontend/core/typecheck/checker/refinement.rs:421`） | — |
| 6.6 门禁转硬 | 6.6.1 T1 由 warning 改 `panic!`（✅ `28ded1fd8`） | `tools/type-tables` | — |
| 6.7 目录改名（D1，纯搬移批次） | 6.7.1 `typecheck/`→`sema/`；`middle/core/`→`middle/ir/`；`parser/ast.rs` 拆迁顶层 `ast/`；字节码域合并（两个 `bytecode.rs` → `bytecode/`）+ `opcode.rs` 迁 `middle/bytecode/` | 全仓 | — |
| 6.8 标注检查模式与 Fn 参数表示（D54） | 6.8.1 `Type::Fn.params: Vec<Type>` → `Vec<Param>`（名字与类型同栖），与 6.5.2 删 `Assign.signature_params` 同批收口——6.5.2 只删旧表示，本项给出替代表示 | `Type::Fn`（`src/frontend/core/parser/ast.rs:463-466`）、消费面约 20 文件（机械改）。**实测口径**：基线 `Type::Fn` **331 命 / 63 文件**（`grep -rn 'Type::Fn' --include='*.rs' src/`） | — |
| | 6.8.2 声明驱动的检查模式：参数类型按位驱动 lambda 头（与名字无关）、出口（尾表达式与 return）统一对照标注返回类型、非 lambda 值 infer+unify（与 `m: Int = "hello"` 同构）；红判据先行——u1–u4/v1–v5 探针入语料（先红后绿）；删 parser 名字合并段 | `src/frontend/core/typecheck/inference/statements.rs`（实测 1170-1914）+ `src/frontend/core/parser/statements/declarations.rs`（名字合并段，实测 337-376） | 对照启用批为**行为修复**：新诊断码、基线更新，不适用 C3；v1–v5 五种语法形态同一判定（消灭「同语义不同语法一红一绿」）。**6.5.2/6.8.1 批次已提前处置一部分**：名字合并段的 **HashMap 名字表**已删除、改为位置对齐（前置校验保证逐位同名）；整段合并仍留本项（残留：`declarations.rs` 值参数合并块），删除须待声明驱动改造 …**部分完成（2026-11-02）**：声明驱动对照已落地并推送（7 笔，`ada6c054`…`e46dbcfa`），矩阵 21 格改判 / 35 格零变化；**剩余面拆为 6.8.3**（`E1112` 柯里形态静默通过 + parser 停止填声明类型的设计补完）。 |
| | 6.8.3 声明来源的设计补完：parser 停止往实现头填声明类型，改由 IR 从声明签名渲染（`FunctionIR.params` 与调用侧可用名同源），使 E1011 判据不再需要「猜作者是否写过标注」；并定位「`ty=None` 形参进入下游后无限递归」的根因（求解器的自递归展开，非单态化专门化） | `src/ast/mod.rs`、`src/frontend/core/parser/statements/declarations.rs`、`src/frontend/core/parser/statements/tests/declarations.rs`、`src/frontend/core/parser/statements/tests/functions.rs`、`src/frontend/core/sema/inference/expressions.rs`、`src/frontend/core/sema/inference/statements.rs`、`src/frontend/core/sema/tests/fn_annotation_binding.rs`、`src/frontend/core/types/mono.rs`、`src/frontend/core/types/solver.rs`、`src/frontend/core/types/substitute.rs`、`src/frontend/core/types/tests/mono.rs`、`src/frontend/core/types/tests/solver.rs`、`src/frontend/core/types/tests/substitute.rs`、`src/middle/ir/ir_gen.rs`、`src/middle/ir/tests/fn_param_slots.rs`、`src/middle/ir/tests/snapshots/`| — |
| | 6.8.4 按 SPEC §7.1 实现编译期值参数的单一分类器：扫描声明中的**全部 `Type` 子树**（含函数返回类型与类型体）判定「被引用」；按**命名空间解析**判定是否命中形参（值/类型命名空间不查同一张表，故形参名与类型名同形不构成遮蔽）；自引用按**单调不动点**处理；删除 `const_used_layer_names` 等第二套启发式，让占位、元数、命名实参可用名都从同一分类器推导 | `src/ast/`（分类器）、`src/frontend/core/parser/statements/declarations.rs`、`src/frontend/core/sema/inference/statements.rs`、`src/middle/ir/ir_gen.rs` | — |
| | 6.8.5 const 实参的编译期特化：`f(42)` 与 `f(7)` 各自一份 IR，消除「调用点实参序列保留编译期值位、运行期却有值无人绑定」这一层账（6.8.4 保留下来的过渡形态）；同时重写 E1112 的报文，使作者看到的不再是「declares 0 parameters」 | `src/middle/ir/`、`src/frontend/core/sema/`、`src/util/diagnostic/codes/`、`docs/src/reference/error-code/`（生成物） | 判据：`f: (N: Int) -> (n: N) -> Int` 的两个不同 const 实参产生两份可独立执行的 IR；语料 375 条零改判；既有柯里写法（`add3`、`f(42)(5)`）零回归；E1112 报文重写后六语 locale 一并更新。**部分完成（2026-11-05）**：两份独立 IR 已成立（`Function #27 f(int64::42)` / `#28 f(int64::7)`）、E1112 报文六语改毕、语料与快照零改判零漂移；**擦除 const 运行期位暂缓**——与依赖类型设计冲突 6.8.5/6.8.6/6.8.7 与本仓库 issue #529；剩余面转 6.9.2，不在本行重复登记 |
| | 6.8.6 落空候选退化与 const binder 收拢：SPEC §7.1 规定形态是候选而用途未命中的形参**退化为运行时值参数**，实现却报 E1094；`checker.rs` 的 `collect_signatures` / `collect_function_signature` 两处窄扫描同样回答「声明级类型参数名 / E1094」，其中一处与本轮所修形态同型漏扫（只扫类型体、不扫形参标注与返回位）；E3006 的 help 文案亦不统一 | `src/frontend/core/sema/`、`src/util/diagnostic/codes/`、`src/locales/` | — |
| | 6.8.7 依赖类型构造值不可造：`Vec(int64::3)` 这类字面量长度类型没有可构造的值（`x: 3 = 3` 即报 E1002），故 `g1(3, [1, 2, 3])` 这类调用在类型层面无法满足；需判定是补「字面量类型可构造」还是收紧「依赖类型构造」的可用面 | `src/frontend/core/types/`、`src/frontend/core/sema/` | 判据：`x: 3 = 3` 与 `v: Vec(3) = [1, 2, 3]` 的判定自洽（要么都成立，要么都给出专用诊断）；不改变 6.8.4 已裁决的 3 格 D 类结果。**部分完成（2026-11-05）**：字面量类型已获得居民（期望类型驱动）、`int64::3 <: int64`、`n: N` 成为强制（`f(42)` 类型由 `fn(int64) -> int64` 变 `fn(int64::42) -> int64`），三条语料按裁决改写；**`Vec(3)` 长度槽待做**（拆为 6.8.7-3；实施归属 6.9.3） |
| | 6.8.8 类型位置判定单点化（#528 主体）：给 `ast::name_used_as_type` 补齐 `ConstExpr` / `MetaType` / `Paren` / `NamedParen` 四个变体，把 `const_param::used_as_const_in_annotation` 降为它的薄封装并接入命名空间解析，删除 `checker::collect_used_in_type` / `collect_used_in_expr` / `collect_used_const_names` 三条并行判据 | `src/ast/mod.rs`、`src/frontend/core/sema/const_param.rs`、`src/frontend/core/sema/checker.rs`、`src/frontend/core/sema/inference/statements.rs` | — |
| | 6.8.9 `extract_generic_param_names` 扫描面对齐（#528 §4-4）：扫描面从「其它形参的标注」改为「形参标注 ∪ 声明注解的整棵树」，使名字只由返回位引用的编译期值参数也进 IR 的 generic_params | `src/ast/mod.rs`、`src/middle/ir/ir_gen.rs`（+ 签名变更的调用点） | — |
| | 6.8.10 const 用途精筛的命名空间问答四处同源：Registry / 签名 / 构造器 / 函数体四个调用点注入**同一集合**（`env.types` 的键 = SPEC §7.1 字面的「已声明类型」），不再出现宽窄两派 | `src/frontend/core/sema/checker.rs`、`src/frontend/core/sema/const_param.rs`、`src/frontend/core/sema/inference/statements.rs` | — |
| | 6.8.11 `collect_name_refs` 补括号透明分支：`Paren` / `NamedParen` 递归内层（binder 名不计），使括号包裹的类型名不再被当成未引用 | `src/ast/mod.rs`、`src/ast/tests/name_refs.rs`、`tests/baselines/` | — |
| | 6.8.12 `01-routing.md` 的两处失效声明：`pub(crate)` 基线行的实例举的是已删除符号并带行号；「现状 → 目标映射」5.3 单元格仍称该符号「留在原处」 | `docs/src/dev/architecture/01-routing.md` | — |
| | 6.8.13 `collect_expr_ident_refs` 补 `Cast.target_type` 与 `Call.named_args`：前者是类型位置却漏遍历，后者是具名实参的表达式 | `src/ast/mod.rs`、`src/ast/tests/name_refs.rs` | — |
| 6.9 量词读数与证据内核（D62） | **C.1｜6.9.1 关系谓词调用点证据**（issue [#532](https://github.com/ChenXu233/YaoXiang/issues/532)）：多实参关系谓词代入已知实参后进入既有 dispatch；不得在参数位之外复制第二套证明入口；若须扩到注册/解析面，按跨二级加框先报裁决 | `src/frontend/core/sema/`、相关直驱测试与语料基线 | **I3 的一个特例，不是独立阶段。** 违反 `Eq(n, N)` 的调用编译期报 E4018；合法关系与守卫形态保持既有 dispatch 语义；入口供 6.9.5 构造点复用。**已完成（2026-11-09）**；#527 注册/解析面未纳入。 |
| | **A｜6.9.2 量词 IR：事实、视图与 q=0 擦除**（issue [#533](https://github.com/ChenXu233/YaoXiang/issues/533)，承接 6.8.5-2b）：q 事实单源；调用点实参视图与实现头绑定视图同源派生；闭项与开项/递归的 q=0 擦除；不新增临时 warning 通道 | `src/ast/`、`src/frontend/core/types/`、`src/frontend/core/sema/`、`src/middle/ir/`、`src/middle/passes/mono/`、`tests/baselines/`、`src/middle/ir/tests/snapshots/` | **I1 + I4。** 两视图单源；q=0 形参/实参运行期不占槽、调用侧仍占实参位；无证据 `f(y)` 报 E1113；闭项与开项/递归都成立；既有合法特化零回归；**C3 完成前 6.8.5-2b 不闭环**。**已落地**：A/B/C1/P0/C2b/C2c（2026-11-10/11）。**未落地**：C3（前置 = F4 裁决 + 6.9.3c 类型化出口；**F4 已由用户（2026-11-16）：甲-闭 + 丙-3 + T-b + bound 8，量测包 6.9.2-C3 F4 裁决**；R4=①（mono 求值 Disproved 不建请求+登记，不报错）；**C3 已实现（甲-闭 + 丙-3 + T-b + bound 8）**；**不宣称 I4 完全体**）；#541 TailCall 范围；#542 已落地（2026-11-19：DCE + post-mono 两入口接线 + C2c 规避移除）；#540 深嵌套炸栈需与 6.9.3 验收对齐。 |
| | **B｜6.9.3 defeq 内核与真实判定点**（issue [#535](https://github.com/ChenXu233/YaoXiang/issues/535)，承接 6.8.7-3）：失败代数 → 归一化/判等 → Unproven/E4023 → 判定点协议（unify→回滚→defeq）→ 接 `solver.unify`；预算一次判定一份 | `src/frontend/core/types/`（含 `solver.rs`）、`src/frontend/core/sema/`（含 `layers/equivalence.rs`、`proof/verdict.rs`）、`src/util/diagnostic/codes/`、`docs/src/reference/error-code/`（生成物）、相关测试与基线 | **I2 + I5 + I7。** `Array(Int, 2+3)` 与 `Array(Int, 5)` 保留为 normalize 层判据（类型位折叠后可能已同形，不得冒充生产可达）；**生产红判据已实测（P2）**：`Five: Type = 5` + `take: (a: Array(Int,5)) -> Int` + `take(b)`，`b: Array(Int, Five)` ⇒ E1002 `Expected 'Array(int64, int64::5)', found 'Array(int64, Five)'`；`Array(Int, 2+3)` 已在 AST→Mono 折叠（P4）、变量声明位被 `seed_container` 阶梯放行（P1），二者不得当红判据；`Vec(3)` 长度槽与值槽期望传播自洽（6.8.7-3）；递归与预算边界不崩溃、不静默放行（#540 深嵌套炸栈是既有边界，需显式排除或另案）；unify 失败先回滚绑定再 defeq；预算按一次判定一份并量测；E4023 用户可达且带真命题/原因/位置；subject 不得伪造；非预算 `EvalError→BeyondKernel` 有分类断言；生产调用图证据；语料/IR 零漂移。**已落地**：a/b/c（2026-11-11/12/13）；**d（2026-11-13）**——19 个判定点收进 `decide`、日志回滚、E4023 用户可达、6.9.3b 遗留 2 分类断言、subject 覆写与变异。**边界**：E4020 不带命题（裁决；`decide` 不产 E4020）；`is_subtype`/`exit_type_matches`/`check_type_equivalence` 未统一（已登记）；#540 深嵌套炸栈另案。 |
| | **C.2｜6.9.4 Γ 等式桥**（issue [#534](https://github.com/ChenXu233/YaoXiang/issues/534)）：Γ 中的常量等式事实作为 q=0 槽的第四证据源；唯一收集点 `check_if_stmt`；`frame_term_env` 把等式并入 `TermEnv`（`admits`/`normalize` 不改）；kill set/分支合流复用既有 Γ；**生产 Γ 原为 None，本轮改两处 `StatementChecker::new` 并实测**；进入同一 `TermEnv`/证据接口 | `src/frontend/core/sema/`（`proof/assumptions.rs`、`inference/statements.rs`、`inference/expressions.rs`、`checker.rs`、`checker/refinement.rs`）、`src/frontend/core/types/eval/const_eval.rs`、相关直驱测试与语料基线 | **I3 的第四源，不是新机制。** 分支内 `f(y)(42)` 合法且与 `f(42)(42)` 同形；分支外/else/嵌套/谓词守卫/kill 后仍 E1113；`f(y)(y)` 的第二值位不在本项（E1002 另案）；函数边界不泄漏；经典类型路径不变；生产调用图证据；语料/快照零漂移。**已落地（2026-11-15，P1–P9 6.9.4 实现轮）**：生产 Γ 两处 `StatementChecker::new` 由 None→Some（**唯一行为翻转点**），并补 `AssertModule.effect_specs()` 的生产登记（此前消费端只在直构测试里接过线）；T17–T20 的 barrier 三性质 + T18(b) 红→绿正控；新增 2 条语料 + IR 快照 +1 + 语料基线 +10/-0（含 B.4 轮 8 行欠账补录）。**边界**：只覆盖裸 `assert`（`assert.assert` 不扩）；只做常量等式；`f(y)(y)` 第二值位不在本项（E1002）；`&mut` 之外的写面（索引/字段/别名）不进 kill set；收集点只有 `check_if_stmt`（match/while/早返回不收集）；#539 未闭 ⇒ 只覆盖模块内；`rfc027_refined_transparency` 四条开约束 tripwire 不属本项。**F-1 / T23 前置修复（2026-11-16，同一缺陷类「按名不按绑定」）**：① Γ 等式事实带**绑定身份快照**（`VarInfo::definition_span`，与值环境 `const_bindings` 同一条解析链），读取口逐名要求「此刻解析到同一次绑定」⇒ `match` 臂遮蔽不串、出臂还原、无出处的等式不算证据；② B.4 来源前置改为**身份匹配**：`derive_libraries` 按 (id, 指纹) 去重、前置检查取该名字**全部**指纹（>1 个不同 ⇒ 歧义 ⇒ E4020）、谓词臂要求本地定义体；病史与 6.9.2 轮 t23（F1'）同源。9 条 E2E + 8 条 Γ 原语 + 1 条 T23 歧义负控 + 2 条语料（正/负）+ 变异 MF1a/MF1b/MT23a，语料基线 +2/-0、IR 快照 +1（新语料）。 6.9.4 修复轮 F-1。 |
| | **D｜6.9.5 Σ 依赖记录（B：witness 字段，2026-11-13 裁决）**（issue [#536](https://github.com/ChenXu233/YaoXiang/issues/536)，**不阻塞 P7**）：**H0 = B**——`w` 是编译期 witness，运行期无数据槽；A 已放弃。**B.0 设计已出**：witness = 记录层 q=0（1A）、证明不存 term/复用 ProofContext（2A）、运行期构造只含数据字段/`Box(3)`（3B）、`StructFieldInfo` 单字段重构、数据索引单一映射、值级 `b.w` 报错、witness-bearing 记录不可变（5A）、导出 B.4；**4A+5A 已定（2026-11-13 lead 第一性裁决：4A=不做用户 `b.w` 投影，5A=witness-bearing 记录整体不可变）**；**B.1 契约已出**：B.1a 表示**已落地（2026-11-13）**；B.1b 布局 + B.1c 5A **已同批落地（2026-11-14）**；**B.2 契约已出；B.2a/B.2b 已落地；B.3 已落地（错误类型 A；D2=A/admission/拒绝面/补臂/T17–T21/语料+快照/门禁全绿）；B.4 已落地（错误类型 A）**；B.d 已合并，文件面冲突解除。**不宣称 Π/Σ 完全体**；Π/`DepFn` 不碰。 | `src/ast/`、`src/frontend/core/sema/`、`src/frontend/core/types/`、`src/frontend/proof_execution.rs`、`src/middle/ir/`、`src/middle/bytecode/`、`src/backends/`、`tests/baselines/`、`src/middle/ir/tests/snapshots/` | **I6。** D.0+B.0 已出；**H0=B**；B.1–B.4 分阶段判据（B.1 数据索引/CreateStruct 数据-only/值级 b.w 拒绝；B.2 构造正负锚/ProofFunctionCall；B.3 类型级 b.w/`ConstExpr::FieldPath`（值级仍 E1114）；B.4 payload/跨模块）；H1–H10 硬前置；每阶段生产可达 CLI/语料。**B.1a/b/c 已落地；B.2 契约已出；B.2a 已落地（非泛型 Σ + C 类构造检查 + IR 量词 + T01–T29 的 B.2a 面）；B.2b 已落地（泛型记录/const 参数/proof family 实例化）**。 |

**6.9 模型与不变式（第一性重写，2026-11-13）**

> **模型**：D62 的终局是**统一项世界**——类型位置与值位置共享同一套编译期项。一次强度声明产生一个量词读数 `q ∈ {0,1,ω}`；`q` 是槽位分配的**唯一输入**，调用点实参视图与实现头绑定视图是同一事实的两个投影。判等是**纯判定**（两侧归一化后同一；不绑定类型变量）；证据是 `q=0` 槽的**唯一入口**（字面量/值环境/单值钉住/Γ 等式四源）；擦除是 `q=0` 的**运行期后果**（闭项与开项/递归都成立）；Σ 的构造点**复用**同一证据接口，不另立一套。**完成判据 = 不变量成立，不是子阶段名跑完。**

| 不变式 | 内容 | 当前状态 | 阶段 |
| --- | --- | --- | --- |
| I1 单源 | q 事实、两视图、槽位分配只有一个写点/读点，无第二本账 | 6.9.2-A/P0/C2b 已立；**C3 的开项表示已定并落地**（`GenericArg::Const(ConstExpr)` 单一表示、`normalize` 单一求值器、mono 绑定源仍只有 `name_map`） | A |
| I2 纯判等 | defeq 不绑定变量；Refined 谓词不同 ⇒ Unproven；判定点先 unify，失败先回滚全部绑定，再 defeq；预算一次判定一份 | 6.9.3a/b/c 内核已立；**6.9.3d 已接生产判定点**（`solver.unify` 的 unify→回滚→defeq；回滚与预算协议随 d 落地成文 6.9.3d 实现轮 §二） | B |
| I3 证据完备 | q=0 槽证据只来自四源；无证据 ⇒ 调用点 E1113，不静默 | 函数/方法面已立（6.9.2-B、#538）；**Γ 源已立（6.9.4，2026-11-15）并修 F-1 遮蔽串事实（2026-11-16）**；导入面 #539 未闭 | C |
| I4 擦除完备 | q=0 形参/实参运行期不占槽；闭项与开项/递归都成立；无死槽/死码 | 闭项 6.9.2-C2c 已立；**开项/递归 C3 已立**（甲-闭 + 丙-3 + T-b bound 8；到界边不擦除 ⇒ **不宣称完全体**）；**#541 已收（TailCall 进调用点改写/擦除面，2026-11-16）**；**#542 已落地（2026-11-19）**：post-mono DCE 两入口接线、C2c 规避移除；该族 verify 红转绿、语料 402/402 零 diff、函数表 852 删/0 增逐条归因；「无死槽/死码」的证据指针 = 该节 §五；生产 post-mono IR 的内层隔离红已清零（2.1.4 于 2026-11-20 落地，判据按槽身份重写 2.1.4 收口） | A |
| I5 诊断诚实 | Unproven ≠ False；E4023 带命题/原因/位置；不 ICE、不伪造、不静默 | 6.9.3d 起 E4023 用户可达（`l ≡ r` + 原因 + 位置）；subject 覆写有变异验证；E4020 不带命题（裁决） | B |
| I6 Σ | 字段可引用前序字段；构造点逐字段验证；witness 擦除 ≠ runtime check | B.0 设计已出；B.1a/b/c、B.2a/B.2b、B.3、B.4 **全部已落地**；残留补测轮已收口（F10 修 + T06/T15/T16/T18/T23 直驱；F11/F12 只登记）；**导入来源前置已按绑定身份修（同名不同绑定不得错配）** | D |
| I7 接线真实 | 判定接在真实生产判定点（solver.unify），有生产调用图证据 | **d 已做**：19 判定 / 15 试探 / 2 克隆沙盒 / 1 模板；生产入口直驱测试与 E4023 用户可达为证 | B |
| I8 等价可核 | 每片有语料/IR 快照零漂移或逐条分类；预算/回滚有量测 | 已完成片全部有记录：**6.9.3d 已接生产判定点（d 已做，I7）**；**6.9.4 已立**（2 语料 + 1 快照 + 基线 +10/-0，含 B.4 的 8 行欠账补录）+ **F-1/T23 修复轮**（+2 语料 +1 快照，base 402/402 差异空）；**6.9.5 已立**（B.1–B.4 + 残留补测，fast 2279 条；欠账已随本线补录）；**C3 已立**（`c1ac5b0c`：M1 开项链 8、D 级字节码 `CallStatic=factorial(::4)`、402/402 差异空、IR 零漂移、fast 2303） | E |

| D62 阶段 | 出口 | 工作单元 | 依赖 |
| --- | --- | --- | --- |
| A 量词 IR | I1 + I4 | 6.9.2 的 A/B/C1/C2a/C2b/C2c + **C3 已实现** | C3 依赖 6.9.3c 类型化出口 + F4 裁决（**已裁 A 并实现**） |
| B defeq 内核 | I2 + I5 + I7 | 6.9.3 的 a/b/c → d | d 依赖生产调用图 + 回滚 + 预算量测 |
| C 证据强制 | I3 + I5 | 6.9.1（关系谓词）+ 6.9.4（Γ）+ #538/#539 | 6.9.4 依赖 6.9.2-B 证据接口 + 6.9.3a 类型化出口；不依赖 defeq/6.9.3d |
| D Π/Σ | I6 | 6.9.5（B.0 已出；B.1 已落地；B.2a/B.2b 已落地；B.3 已落地（错误类型 A）；B.4 已落地（错误类型 A）+ 残留补测（ §十四） | B 设计包 + A/C.1；可能依赖 6.9.3 类型化出口 |
| E 验收 | I8 | 每片验收 + #537/#540/#541/#542 处置 | 横切 |

> **6.9 收口判据**：I1–I5、I7 全部成立，I6 按 6.9.5 的设计复核结论收口（不阻塞 P7），I8 每片有证据；残留项 #537/#539/#540/#541/#542/#552 全部有处置（修复/明确排除/另案）；#542 已于 2026-11-19 落地。**不以「子阶段勾完」代替不变量成立**。

> **偏序（不是串行链）**：A 事实 → {A.C3, B.d, C.2, D 设计}；B 内核 → {B.d, C.2, D 实现}；C.1 独立且已完成；F4 裁决 → A.C3；判定点协议 → B.d；设计复核 → D 实现。A.C3 与 B.d 可并行但需文件面协调（C3 偏 typecheck→mono，d 偏 types/solver→sema），C.2 可在 6.9.2-B/6.9.3a 后与 B.d 并行（不依赖 defeq）；D 的实现等 B 设计包确认；依赖 A（q=0 擦除）与 C.1（dispatch），可能依赖 6.9.3 的类型化出口。**P7 闸口 = A/B/C 出口；D 不阻塞。**

> **A.C3 的前置：F4 裁决（已裁 A，2026-11-16；量测）**：旧表述「`CheckOnly` 证据不进请求」已过期——`ConstEvidence`/`CheckOnly` 已删；现行机制是开项 `normalize` 返回 `Unproven` ⇒ 不建请求，表示层 `GenericArg::Const(ConstExpr)` 已能装 `k-1`。**裁决改为两条正交决策 + 一个终止子决策**：D-a 表示（甲 开项进 `deferred` 并在入队前归一化 / 乙 不特化）；D-b 求值点（只能 mono；丙-1 第二字段与丙-2 从 IR 复原均破单一表示/求值器，丙-3 = 甲）；**F4-T 终止**（朴素开火会把递归链推到 `max_depth` 撞 E3005；T-a 实例内折条件 / T-b 有界开火+到界退回原件 / T-c 终止性层或改规范）。**先做 ，再裁；未量测前不得把「甲已可实施」当结论。** **用户（2026-11-16）：D-a=甲-闭、D-b=丙-3、F4-T=T-b、bound=8；C3 实现另立任务，不宣称 I4 完全体。**

> **B.d 的前置：判定点协议（未成文）**：RFC-027 §12.1 规定「判定点先走统一；统一失败后允许改用判等，但必须先回滚统一尝试产生的部分绑定」。实现必须回答：入口是 `solver.unify` 的外层包装还是新 `decide` 层；defeq 只在两侧 ground（无未绑定变量）时尝试；快照/恢复的边界与成本；unify 与 defeq 共享一份预算还是各一份。**职责边界**：unify 继续负责子类型/Any/Never；defeq 只判等价，不接管子类型，双向子类型不得当等价（RFC-027 §12.1）。**先给协议与生产调用图，再写代码**；6.9.3c 的接线契约已因假设 `can_assign` 是判定点而浪费两轮、`AssignmentChecker` 生产零实例化。

> **E 验收与残留（I8，横切）**：每一片必须给出①语料/IR 快照零漂移或逐条分类；②`testsuite run fast` + 受影响 integration；③错误码生成器幂等（若动发射点/码表）；④行为变更按 D59 分类并先红后绿。**接线类任务额外要求**：生产调用图证据（哪个生产入口、几条路径、哪条可达），禁止用单测可达冒充用户可达；回滚与预算要有量测数字。**残留处置**：#537（D 类，证明函数形参精化静默丢弃；裁决前不修，6.9.4/6.9.5 不得依赖其修复）、#539（导入面 q=0；C 阶段边界，未闭则 6.9.4 只覆盖模块内）、#540（深嵌套炸栈；**已关闭**——3.20.30 于 2026-11-14 收口：编排层 64MB 大栈线程 + parser 深度闸 MAX_PARSE_DEPTH=1024 + E0019，见 09 的 3.20.30 行）、#541（TailCall；**已收**——2026-11-16 三处同进同退：`call_form_is_rewritten` / `replace_calls_in_function` / `drop_erased_call_args` 经单点 `rewritten_call_target` 覆盖 `Call`/`TailCall`，init 段与函数体共用一份站点改写；红→绿 + 三条变异）、#542（孤立共享层；**已落地（2026-11-19）**：DCE pass + post-mono 两入口接线 + C2c 临时规避移除；红→绿、402/402、函数表归因与偏差）、#552（导入多参数谓词证明执行；6.9.5 邻域，另案）。**归属待裁**：`rfc027_refined_transparency` 的四条开约束 tripwire（`Eq(N, n)`）不属 6.9.4 的常量等式桥；留在 6.9.4 outOfScope 还是改挂 6.9.1/E4018 证明函数面，需裁决。

> **不宣称/边界**：本项只覆盖记录 Σ（B：witness 字段），不碰 Π/`DepFn`，不宣称 Π/Σ 完全体；6.9.4 只做常量等式，不兑现开约束 `Eq(N,n)` 的 tripwire，`f(y)(y)` 第二值位不在本项；witness 擦除 ≠ Runtime check 擦除；#537 不修则 6.9.4/6.9.5 只覆盖不依赖证明函数形参精化的形态；#539 不修则 6.9.4 只覆盖模块内；#540 已关闭（3.20.30）；#542 已落地（2026-11-19）；2.1.4 的内层隔离红已于 2026-11-20 清零（判据按槽身份重写 2.1.4 收口）⇒ `verify` 升门禁的前置全部解除。

**硬验收**（**2026-10-27 口径修订，承 2026-10-26 裁决②**）：`git diff --stat tests/yaoxiang tests/baselines src/std/` 必须在 **6.1–6.7 的每轮**为空。`tests/` 下的**直驱测试文件**（驱动被改代码的测试，如 `tests/formatter_integration.rs`）在**改动机械要求**时可修改，须逐项登记 **file / 行数 / 理由**；**不得删测试、不得放宽断言**。**例外**：6.8.2 是**行为修复**批（新诊断码 + 基线更新），不适用本硬验收，单列一轮并单独裁决。

> **修订关系**：原口径（2026-10-26 裁决②）写作 `git diff --stat tests/ src/std/`，把整个 `tests/` 一刀切为零差异；本口径**只把直驱测试文件从「零差异」改为「可改但须登记 file / 行数 / 理由」**，**语料层（`tests/yaoxiang`）、基线层（`tests/baselines`）与标准库（`src/std/`）的零差异要求不变**。首次适用 = 6.1.2：仅 `tests/formatter_integration.rs` 改了 **3 行**（2 处 `assert_format_eq` 的 `i64`→`Int64` + 1 行注释），理由 = 被改的拼写要求该断言的期望类型名同步；未删测试、未放宽断言。详见 「P6 6.1 名称规范化收口」。

### P7 中间表示 SSA 化

来源：[04](04-ssa.md) §实施要点 批 a–d，判据 C4，**批内严格串行**。

- [x] **7.0 纯搬移拆分**（6 个三级任务）

> **7.0 设计依据与边界**：目标布局来自 [01-routing](01-routing.md) 的目录树与迁移表——`middle/ir/ir_gen.rs` → `middle/lower/`、`sema/inference/expressions.rs` → `sema/infer/`（调用裁决族 `call/`）。本批**只做纯搬移**，验收判据为「除路径外零语义 diff」（逐文件人工 review）+ C1 快照 zero-diff。它与 [RFC-039](../../rfc/accepted/039-compiler-architecture.md) 替代方案 A「只拆文件、不动范式」的否决不冲突：被否决的是以搬移**替代**范式工作，本批是范式工作的**前置**——7c.1 的 `lower/synth.rs` 依赖 `lower/` 目录就位，7d.1 的 `generate_call_expr_ir` 方法级拆分在搬移后的小文件上进行。expressions 侧按裁决走（只拆调用裁决族，`statements.rs` 原地不动）；`inference/ → infer/` 整体改名与 statements 归并属全量迁移，不在本批，留 P7 后另立设计轮。`synth.rs` 是新边界不是搬移，归 7c.1。勘察基线（2026-11-12）：`ir_gen.rs` 9804 行、148 个方法，主体为单个 `impl AstToIrGenerator` 块（约 9250 行）；`expressions.rs` 6825 行、93 个方法，主体为单个 `impl ExpressionInferrer` 块（约 6280 行）；最大方法 `infer_call_expr` 1671 行、`generate_call_expr_ir` 866 行（后者为 7d.1 对象）。

- [x] **7a 切断多次定义**
- [x] **7b SSA 形态切换**（5 个三级任务）

> **A/I4 的保留条件（2026-11-13 重写）**：6.9.2-C2c 的「保守保留原件」是 I4 未完成前的临时形态——原件仍带 q=0 槽，克隆体把体内读物化成常量。该规避已随 #542 落地移除（2026-11-19）：mono 不再按泛型名删原件、恢复闸删除，删除职责归 post-mono DCE；仍保留的「原件不擦」是 C3 T-b 到界退回的长期语义，不属于规避。**C3 完成后的重评估（2026-11-16）**：C3 的 T-b 到界退回使「第 9 次开火」的调用点留在原件上 ⇒ 原件被放回、共享层仍有引用者，故 #542 的面收窄到**非递归全覆盖形态**（`fn_const_generic` 族）；处置已按于 2026-11-19 落地（post-mono DCE 接线，连同本条规避一起移除）。

- [x] **7c 隐式契约显式化** ✅ 535933ce、4185dcdf、21ddf23f、138f9e0f

> **7c 遗留（非本笔，未修）**：① 跨单元单定义方法以命名实参调用会上报 E6007——新旧 revision 完全一致（非本笔引入、本次未放大；2026-11-25 复验）。② 精化谓词求解预算固定 100ms（`src/frontend/core/sema/proof/budget.rs:41`、`:50`，经 `src/frontend/core/sema/layers/predicate.rs:203`、`:239` 传给 Z3）：求解超过该预算时落 Unknown/Unproven 分支，语料 `tests/yaoxiang/06-compile-errors/refinement/call_arg_no_evidence_err.yx` 由 E4018 退化为 E2031；标准批次环境实测 5/5 为 E4018，高负载作为触发条件未获复现。③ 4.5.2 的 `overload_resolutions` 常驻差集判据（含 `examined`/`consumed` 证据面下限，`src/middle/ir/tests/overload_resolutions.rs`）已不存在——**2026-11-25 已更正口径**：模块头改为事实陈述（原文为「差集必须为空」，现写明该语料级用例已于 2026-11-07 批量退役中删除、仓内无自动执行点、同款证据面由 `release_plan_spans` 承担），恢复判据需过测试准入三扇门。

- [ ] **7d 最后做**（2 个三级任务）
- [ ] **7e `.42` 数据丢失修复**（3 个三级任务；2315/2341 由"独立 issue"收编，2026-10-05）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 7.0 纯搬移拆分（前置） | 7.0.1 `ir_gen.rs` → `middle/lower/` 第一批：建目录；`AstToIrGenerator` 结构定义与构造族、作用域/槽位/`note_*` written-type 族、纯查询族迁入（目标名 `mod.rs` / `scope.rs` / `query.rs`，按 01-routing 目标树） | — | 「除路径外零语义 diff」逐文件人工 review；C1 快照 zero-diff |
| | 7.0.2 第二批：语句与循环族、全局定义与模块装配族、常量求值族（目标名 `stmt.rs` / `loops.rs` / `globals.rs` / `module_ir.rs` / `const_eval.rs`） | 7.0.1 | 同 7.0.1 |
| | 7.0.3 第三批：柯里化族与表达式各类（目标名 `curry.rs` + `expr/`；调用裁决方法随 `expr/` 迁入，方法级拆分仍归 7d.1） | 7.0.2 | 同 7.0.1 |
| | 7.0.4 `inference/expressions.rs` → `inference/expressions/`（：调用裁决族拆出 `call/`（入口签名不变，子模块划分按方法簇定），其余为 `expr.rs`；目录名按命名约定「`foo.rs` 的子模块目录叫 `foo/`」；`statements.rs` 与 `inference/` 改名均不动 | 7.0.3 | 同 7.0.1 |
| | 7.0.5 独立验证轮（覆盖 7.0.1–7.0.4 全部）：IR 快照与语料差分基线同提交再生并人工 review（预期零漂移）；本地九门禁全绿；批次树按 `scripts/dev/scratch.py` 规程 gc | 7.0.4 | 快照与语料基线零漂移；九门禁 exit 0 |
| | 7.0.6 收口：`01-routing.md` 迁移表两行的落地状态更新、`inference/README.md` 结构段同步、实录进 | 7.0.5 | — |
| 7a 切断多次定义 | ~~7a.1 删 3 处语句级回收~~ **拆出，移至批 b；已随 7b.4 落地（2026-11-22）**（删回滚会让槽位数失控：2 条语料 354 / 323 槽越过 255，其余语料峰值 51→211；分配器接管后实测帧槽 15 / 24） | **2.1.2 `verify_loose` 跑绿** | C4 |
| | 7a.2 6 处 save/restore 换 RAII guard（另含第 7 处：嵌套函数定义路径的缺失保存） 7a | 2.1.2 | C1 快照 zero-diff |
| | 7a.3 口径统一 + E3014 共享检查（`take_body_locals` 单点，覆盖全部函数体形态） | 7a.2 | C1 |
| | 7a.3a 接口分发链兜底臂（既有缺陷：未知 tag 路径不写结果槽 ⇒ use-before-def）；登记 7a | 7a.3 | verify_loose 绿 + 语料零 diff；D59 C 类 |
| 7b SSA 形态切换 | 7b.1a `ir.rs` 结构变更 #1-2（`Operand` 变体收敛 + `Instruction::Phi` 引入） | 7a.3 + **P6 完成** | C4；独占 commit；**已完成（2026-11-20 7b.1a 收口）** |
| | 7b.1b `ir.rs` 结构变更 #3-5（`FunctionIR` / `BasicBlock` / 类型标注字段） | 7b.1a | C4；独占 commit；**已完成（2026-11-20 7b.1b 收口）** |
| | 7b.2 `next_temp_reg` 改 `Operand::Value` | 7b.1 | 同上；**已完成（2026-11-22 7b.2 收口）** |
| | 7b.3 `translator.rs` 加 `Phi` 臂 | 7b.1 | C4 + `.42` 往返测试；**已完成（2026-11-22 7b.3 收口）** |
| | 7b.4 线性扫描分配器（u8/255 槽模型内） | 7b.3 + **7a.1**（拆出的删回滚同批） | 同上（+500~1000 行）；**已完成（2026-11-22 7b.4 收口）** |
| 7c 隐式契约显式化 | 7c.1 `synth.rs` 边界（边界检查脚本经用户 2026-11-24 取消）+ 显式 `SynthEnv` / `BindingLocation` + spawn-for 双运行时 env / args / upvalue 修复；D59 **B 类**（嵌套闭包捕获错配、错误后捕获状态残留、spawn-for 匿名前缀槽位错配与双运行时 item/capture 错值；仅实际红态用例作修复证据，其余为回归覆盖） | 7b.4 | C4（不含门禁脚本）；**已完成（2026-11-25 收口：535933ce、4185dcdf、21ddf23f、8e5b89c1、f8987795）** |
| | 7c.2 既有消费台账核验（`ReleasePlan` / `OverloadResolutions` 共用 `ConsumptionLedger<PlanId>`，无第二套计数器） | 7c.1 | **核验 + 复审通过，无代码改动**；消费计数零触发 |
| | 7c.3 `method_def_ordinals` 只读化（定义名改由 typecheck 按声明身份产出，lowering 只读；D21 归 04） | 7c.1 | C4；**已完成（2026-11-25 收口：138f9e0f；定义身份映射落地 + 独立验证/复审通过）** |
| 7d 最后做 | 7d.1 `generate_call_expr_ir` 拆为 `CallArgs` + 6 个 `emit_*` | 7c.x | C1 快照 zero-diff |
| | 7d.2 「补 0」兜底改返回诊断 | 7d.1 | C4 |
| 7e `.42` 数据丢失修复 | 7e.1 `upvalue_count: 0` 修正（`bytecode.rs:2312`）+ `VERSION` 4→5（D17）+ 已分配未使用 opcode 回收（D32/D34 随升版） | 7d.x | C4 + `.42` 往返测试；独占 commit |
| | 7e.2 `exception_handlers` 落盘（`bytecode.rs:2315`——throw/try 是语言核心语义，**不得留待实现**） | 7e.1 | `.42` 直跑异常用例差分 |
| | 7e.3 `globals` 落盘（`bytecode.rs:2341`） | 7e.1 | `.42` 直跑全局变量用例差分 |

**预期管理**：行数**净增 900–1600 行**。若立项时以"代码变短"为成功标准，本阶段会被判定为失败。

### P8 前端范式变更

来源：[05](05-frontend-paradigm.md) §实施要点 阶段 0–4，判据 C5。

- [x] **8.0 死阶梯清理（C6）**（2 个三级任务）
- [x] **8.2 词法收敛**（4 个三级任务）
- [x] **8.3 Pratt 优先级收敛（回落方案）**（1 个三级任务；✅ `3e216b44`）
- [x] **8.7 `parse_assign_after_target` 拆分**（1 个三级任务；✅ `31902db`）
- [x] **8.8 测试补全**（3 个三级任务；✅ `b16e6bfc1`、`a15743f`）
- [x] **8.9 参数位形态归一（D53，2026-10-05 增补）**（1 个三级任务；✅ `01718bb`）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 8.0 死阶梯清理（C6） | 8.0.1 删 `Precedence` 枚举 + `PrecedenceContext`（96 行） | **与 1.2.1 冲突，见 C1** | — |
| | 8.0.2 裸魔数具名化（`(6,7)` / `(11,1)` / `12`） | — | — |
| 8.2 词法收敛 | 8.2.1 四基数扫描器合并 + 三处转义合并 + 多行串合并 | — | — |
| | 8.2.2 f-string 嵌套编译消除 | 8.2.1 | — |
| | 8.2.3 f-string 插值内诊断 span 为相对语义（既有缺陷登记，issue [#574](https://github.com/ChenXu233/YaoXiang/issues/574)） | — | **归属阶段：P8；不阻塞主线**；D/E 类语义变化须裁决；目标＝插值内诊断 span 由相对插值文本改源文件绝对偏移；四类复现形态（实测 15 探针 / 7 条插值内诊断全部 line=1、col 相对插值文本）：①单插值 ②多插值 ③嵌套 f-string ④多行 f-string；本次按 05:623 保持逐条不变，单独修复 |
| | 8.2.4 f-string 插值单遍词法与逐插值隔离语义裁决 | 8.2.2 | **归属阶段：P8；不阻塞主线**；D 类设计变更须裁决；目标＝兑现 05:107「含 N 个插值的 f-string 触发 N+1 次完整词法分析」/ 05:359「把 N+1 次降为 1 次」；计数点插桩到共享词法实现后实测（N=3）完整词法分析=4=1+N，与改前相同；单遍合并代价＝拼接源会让未终结字符串吞掉后续插值，须先裁决逐插值隔离语义 |
| 8.3 Pratt 优先级收敛（回落方案） | 8.3.1 单套声明式优先级表 + 两套 `BinOp` 同源互校（含 D1–D3 修正）（✅ `3e216b44`） | — | — |
| 8.7 `parse_assign_after_target` 拆分 | 8.7.1 拆 8 个职责段落 + 抽出 `skip_balanced_parens`（✅ `31902db`） | — | 该函数只做派发 |
| 8.8 测试补全 | 8.8.1 新增结合性用例（**2026-11-23 已完成**：新增 `parser/tests/associativity.rs` 15 条语言级用例，只经 `parser::parse_with_probe` 断言 AST 形状，`git grep -n 'pratt::'` 对新文件零命中——与 Pratt 内部解耦；覆盖 `-` / `/` 左结合、`*` 高于 `+`、比较低于算术、`<<` 高于 `&` / `\|` / `^` 且位运算同级左结合、`..` 链左结合（`a..b..c` → `(a..b)..c`）与 `0..n+2` 上界吞加法、`=>` 体右贪婪、赋值右结合、`?` 与调用链后缀、模式 `\|` 收链，另有括号分组对照一条；「层内绑定力严格递增、无重复」按 coding-rules 7.1 属实现形状，**不设门禁断言**（只留特征化说明）。**与原文「先红后绿」的偏差**：本批零生产改动、用例先于文法落地，无红态可走；等价锚职责后由 8.3.1 的 D1–D3 永久断言与语料 / IR 基线承担。**`as` 相对 `*` 一项未落成绿断言**：实测两方向都与 SPEC syntax §2.2 级 2/4 不符，已作缺陷登记，明细与 census 数字见任务 8.8 的提交信息；✅ `b16e6bfc1`） | 8.2.x | C5 |
| | 8.8.2 补四类字面量错误路径（**2026-11-23 已完成**：四类「旧口径 ↔ 现树路径 ↔ 已覆盖 / 新增」对照表见任务 8.8 的提交信息。新增 10 条用例落 `src/frontend/core/lexer/tests/literals.rs`：溢出后继续消费输入（`Lexer::next_token` 直驱观察流位置）、`scan_leading_dot` 整体五条（`.5` / `.5e3` / `.5_5` / `.5_` / `.5e`）、非法转义三条（`\x` / `\u` 各一组，字符串）、字符字面量 `\x` 一例（1+5+3+1=10）；「四基数溢出」由 8.2.1 的 `test_radix_literals_overflow_rejected_with_literal_text` 覆盖，不重复造。**门依据**：四基数与溢出属条款门（SPEC §1.6.1）；前导点 5 条与转义 4 条属证明门（C5 行为保持：同一输入的 token 流 / 诊断不变）/ 病史门（提交 1775f378 的 `t.0` 被读成 `t + 0.0`）——SPEC §1.6.2 / §1.6.3 的文法不含前导点与 `\x`，补正文留给语言设计流程；其中 `.5_` 一条为特征化（缺陷 D5，见任务 8.8 的提交信息），不设门禁；✅ `b16e6bfc1`） | — | 覆盖未测路径 |
| | 8.8.3 补 `parser/tests/wiring.rs` 接线自检断言（2026-11-25 调整：随 8.7 批执行；✅ `a15743f`） | — | 判据 = `parser/tests/` 有接线自检断言且无孤儿目录 |
| 8.9 参数位形态归一（D53） | 8.9.1 文法产生式（✅ `01718bb`）：裸标识符按类型命名空间解析、解析不到报 E；无名签名要求 lambda 头自带参数名（RFC-007:47 既有规则）；RFC-010 形态表与接口示例（`(Surface)`）随批对账 | 6.8.1 | 语料普查零迁移（std/语料裸标识符全为真实类型）；未知类型标识符复用既有 E1003，不新造码 |

**风险**：优先级表是唯一权威，消费点漏改会**静默改变结合性**（不报错，只改解析结果）。防线是 D1–D3 永久断言与算子属性互校测试；新增运算符必须同时进表与互校测试。

> **8.3–8.6 依赖 `08` 的 D0/D2**：文法文件若放在 `parser/` 以外、或动作代码反向引用 `sema`，会在 `check-boundary.py` 处被拦。

### P9 防反弹门禁（统一脚本清单 —— 本文档补齐原缺口 G3）

来源：[01](01-routing.md) §防反弹机制 + [08](08-maintenance-mechanism.md) §可机器检查的规则

**这是全部 CI 脚本的唯一权威清单**（10 个在用脚本；9.8 行保留为取消登记）。脚本在「引入阶段」接入 CI；标注 report-only 的在 P9 统一转硬（从"报告"改为"失败"）——存量违规（平行表示、消歧别名等）要等 P6/P8 才清零，提前转硬会让 CI 长期红、门禁被人工绕过，P9 转硬时门禁应当恰好全绿。P2 的 2.5、P4 的 4.3.2、P7 的 7c.1 均引用本表，不另立清单。

- [ ] **9.1–9.11 中十个在用脚本按引入阶段接入，P9 统一转硬**（9.8 行经用户 2026-11-24 取消；明细见下表）
- [x] **9.12 转硬前置：判据 C 接线点重定基与陈旧库存清账**（2 个三级任务；归属阶段：P9；是否阻塞主线：否）
  - [x] **9.12.1 判据 C 接线点重定基**
  - [x] **9.12.2 判据 C 覆盖面扩张与 `build_registry` 清账**

| # | 脚本 | 检查什么 | 引入阶段 | 验收 |
| --- | --- | --- | --- | --- |
| 9.1 | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` 双向断言 | P4 | 故意漏一个阶段必须红 |
| 9.2 | `check-obligations.py` | 字段出现 ≥2 次（定义 + 写入）但无第三文件读取 | P4（**唯一归属，不再在 P9 重复**） | 故意留一个无消费字段必须红 |
| 9.3 | `check-boundary.py` | 禁 `include!`；`pub(crate)` 跨层泄漏只许减；禁 L2→L3（**唯一归属**，`08` 的 P0 定义了它） | **P0** | 故意加 `include!` 必须红 |
| 9.4 | `check-test-wiring.py` | 有 `tests/` 目录但父模块无 `mod tests;` → 失败 | P1 | 故意造孤儿目录必须红 |
| 9.5 | `check-snapshot-drift.sh` | 快照有 diff 而提交信息无 `snapshot-update` 标记 | P2 | 同上 |
| 9.6 | `check-ir-verifier.sh` | 全语料 `verify_loose` 非空 | P2 | 同上 |
| 9.7 | `check-corpus-parity.py` | 语料差分非空（诊断 code+span 逐条比对，**无任何分组放宽**） | P2 | 同上 |
| 9.8 | `check-synth-boundary.py`（经用户 2026-11-24 取消） | 不再执行；原判据为 `synth.rs` 之外出现 `ast::Expr` 构造，脚本与 CI job 已删除 | P7 | — |
| 9.9 | `check-concepts.py` | 禁令一 A/B/C/D（平行表示、消歧别名、同义词表） | **P0**（report-only，P9 转硬） | 必须在未改动代码上报出 3 套运算符枚举 |
| 9.10 | `check-fanout.py` | 禁令三 A/B/C | **P0** | 故意新增第 6 个入口式接线必须红 |
| 9.11 | `check-classification.py` | PR 必填块「错误类型（A–F，按 RFC-039 D59）」标注：A/B/C 类须附红判据引用，D/E/F 类须附决议号或裁决 | **P4**（report-only，P9 转硬） | 故意漏标或错标一个 A 类改动必须红 |

> **没有行数 ratchet 脚本**（`check-file-size.py` + `baseline.toml` 已于 2026-10-03 取消）。规模问题由 `08` 禁令二判据 A（职责分离，人工判断）解决。

### P10 其余清理与状态修正

来源：[06](06-cleanup-inventory.md) §实施要点

- [ ] **S2 删纯占位死代码**（1 个三级任务）；**2026-10-18 补登 S2.2**：零消费者 `pub` 符号清账 2 处（issue [#472](https://github.com/ChenXu233/YaoXiang/issues/472)）
- [ ] **S3 空头设计文档**（1 个三级任务）
- [ ] **S4 RFC 状态修正**（1 个三级任务）
- [ ] **S5 `pub` 项降可见性**（1 个三级任务）
- [ ] **S6 opcode 死路径清理**（1 个三级任务）
- [ ] **S7 wasm32 门控 project-mode 子树**（2026-10-18 登记，D35 假设修正；1 个三级任务；**归属阶段：P10；是否阻塞主线：否**）
- [ ] **S8 文档更正形式收敛（就地改写，删修订注记）——2026-10-18 登记**（1 个三级任务；**归属阶段：P10；是否阻塞主线：否**）
- [ ] **S9 发版前审计的抽象化收口**（2 个三级任务，2026-10-18 登记；**归属阶段：P10；是否阻塞主线：否**）
- [ ] **S10 02 正文与事实对齐（行号锚 + 内容级过期）——2026-10-21 登记**（1 个三级任务；**归属阶段：P10；是否阻塞主线：否**）
- [ ] **S11 DateTime 迁出内置类型名并落到 std.time**（1 个三级任务，issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)；2026-10-27 登记；**归属阶段：P10；是否阻塞主线：否**）

| 二级 | 三级 | 验收 |
| --- | --- | --- |
| S2 删纯占位死代码 | `instance.rs:416-830`（415 行）/ dispatch 链 / 死阶梯 / 旧语法探测 / `pipeline/tests`（14 行）/ `undefined/` 空目录；**E2：`lib.rs:568` / `:582` 两处 `_ => todo!()` 改有意义的降级输出** | `cargo test` 通过；`clippy -D warnings` 无新增 |
| | S2.2 零消费者 `pub` 符号清账（发版前审计新增 2 处）：`frontend/core/typecheck/signature.rs` 的 `parse_signature_param_names`（真死，约 −3 行）· `frontend/module/orchestrator.rs` 的 `check_source_in_project`（`src/` 0 调用，但 `scripts/ci/check-fanout.py` 按 (文件,符号) 登记为锚点 ⇒ **删前先核该脚本对锚点缺失的行为**） | issue [#472](https://github.com/ChenXu233/YaoXiang/issues/472) |
| S3 空头设计文档 | C1+C2 删除后重写为指向 `src/frontend/module/`（约 2,670 行）；C3 改写边界表；**同步清理 `config.js:259-268`** | 文档站无 404；`check-docs-truth.py` 通过 |
| S4 RFC 状态修正 | C4 RFC-018 移回 `draft/`（D46）；C5 RFC-028 补 `impl_status: 'not-started'`；F3 `TRACKING.md` 加「实现状态」列（D36） | `check_tracking.py` 退出码 0（**不手改 TRACKING.md**） |
| S5 `pub` 项降可见性 | B2 `TypeSystem` / B4 `check_type_equivalence`（**`is_subtype` 必须保留**） | 编译器能指出真正的死代码 |
| S6 opcode 死路径清理 | B8 `TailCall` 与 B9 `Switch`（D32/D34：删，opcode 值回收已在 7e.1 随升版落地）；B10 `UnaryOp::Not`（D33：`opcode()` 区分 `op` 字段）；**F7+F8 opcode 生成期门禁**（`tools/code-tables` 扩展 opcode 提取器约 30 行 + 5 处事实表达单源化 + `size()` 全 opcode 对拍测试） | opcode 事实 5 处表达收敛为 1 处权威源；`size()` 注释与实际编码不符（`bytecode.rs:2181-2182`）修复 |
| S7 wasm32 门控 project-mode 子树（2026-10-18 登记，D35 假设修正；用户） | S7.1 把整条 project-mode 子树在 wasm32 上门控（`#[cfg(not(target_arch = "wasm32"))]` 或等价 cfg 面）：D35 设想的「删不可达者」在 `orchestrator.rs` **无适用目标**（4 条 wasm32 正臂运行期不可达但编译期承重——调用点未门控；6 条 `not(wasm32)` 反臂 native 三条阶段表承重；逐条判定见 `06-cleanup-inventory.md` §「F2 判定」）。**三条未核实（原样保留，落地前须补）**：① 产物规模收益未量测；② shim 之外的 wasm32 消费者未穷举；③ native 承重为静态调用链未端到端 | 先红后绿不适用（无行为变化面）：门控后 wasm32 目标编译通过且产物缩小可测；native 全量回归零漂移（语料 + IR 快照）；`06` F2 行同步指向本登记 |
| S8 文档更正形式收敛（就地改写，删修订注记）——2026-10-18 登记（ | S8.1 按本批固化规则（`coding-rules.md` 第四部分）审计存量「修订注记」并就地改写。**scope（本次审计清单）**：① `02-stage-contract.md:431` 阶段表草图两条次序偏差集成进草图；② `02:486-492` 三档表与「为什么先做 W 级」段改写为终态 E 级事实；③ `02:780-798` Driver 草图改为落地形态；④ `02:461-479` Obligations 草图先逐条核对；⑤ `013-error-code-specification.md:517` 正文改写为现状、删旧引文；⑥ `09:130` 3.2 行写实测结论；⑦ 指针 6 处（`02:1217`、`02:1239`、`04-ssa.md:194/447/721`、`:141`）改指正文对应段 | `docs/src/dev/architecture/` 下 `grep -c 修订注记` 归零（`` 除外）+ 6 处指针改指正文 + 涉及代码事实处 file:line 在冻结 revision 重取 |
| S9 发版前审计的抽象化收口（**2026-10-18 登记**；**归属阶段：P10；是否阻塞主线：否**） | S9.1 `typecheck/inference/statements.rs` 的上下文灌注块两份（`infer_statement` 两条分支各抄一份，字段顺序已漂移）；本次把新 setter 同时写进两块 ⇒ **本次新增 2 份**。抽共有 setter ≈ **−14 行** | issue [#473](https://github.com/ChenXu233/YaoXiang/issues/473) |
| | S9.2 `MonoType` 全变体遍历两份手写副本 ✅ `1aaeb3c5`、`d459b07f` | issue [#474](https://github.com/ChenXu233/YaoXiang/issues/474)（已关闭） |
| S10 02 正文与事实对齐（行号锚 + 内容级过期）（**2026-10-21 登记**；**归属阶段：P10；是否阻塞主线：否**） | S10.1 把 `02-stage-contract.md` 的 7 组既有漂移就地改正：行号锚按现树重取，内容级过期改写为终态事实。**scope（7 组清单，原样落账于 4.4 收口「十二」）**：① §(a) 入口表 3a-3d / 5 / 5b；② `02:1016`；③ `02:360-361`（Z3 wasm 构件获取，#435）；④ `02:127` 机制指针；⑤ `02:128`；⑥ `02:334`；⑦ 改动清单表其余行。口径 = 均为 **pre-existing 漂移**（非 4.4 引入），映射由 t11 在 `657c3d5e3`（= 推送后 `e184d88b6`）上独立抽验；**本条的登记号由原拟的 S9 顺延为 S10**（S9 已由「发版前审计的抽象化收口」占用，编号须全文档唯一） | 7 组逐条改正后 `python scripts/ci/check-docs-truth.py` 通过；每处行号锚按现树重取（`git show <rev>:<file>` 复核）；改正后 `09` 顶部计数表与看板行数按 §十五 的自洽口径对账 |
| S11 DateTime 迁出内置类型名（归属阶段：P10；是否阻塞主线：否） | S11.1 ✅ `1c6d5dd9`、`0da1cb2f` | issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)（已关闭） |

## 实施要点：依赖与并行

### 强制串行主链

`P0 → P1 → P2 → P3 → P3.5 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 止血通道（2026-10-05 裁决）**：P3 的前置收窄为 **2.3.2 + 2.4.1 + 2.4.3**（多文件语料层 + 两个红判据）。P2 的其余部分（2.1 IR 校验器、2.2 快照、2.3.1/2.3.3 差分与性能基线）可与 P3 并行推进。正确性漏洞的止血不被判据基础设施建设阻塞；但 **P4 仍必须等 P2 全部完成**——统一 Driver 需要三层判据全部就位。

| 约束 | 原因 |
| --- | --- |
| P0 → P1 | 三条禁令约束 P1 的每一个动作（1.2.x 就是在往 `mod.rs` 加 `mod` 声明） |
| P1 → P2 | 复活测试会改变测试数量与语料基线 |
| P2（收窄后）→ P3 | 漏洞判据必须**先写成红的**（2.4.1/2.4.3）+ 多文件语料层（2.3.2）存在，红测试才有地方跑 |
| P3 → P4 | 先修 bug 再重构；反序会让 bug 被阶段表固化成"既定行为" |
| P3.5 → P4 | RFC-029g（#399）的删除面覆盖 P4 4.2.2 / P5 / P6 / P7 / P8 各自的重写对象（清单见本文 §P3.5）——反序等于在新文法、新 checker 布局、新 IR 上重做同一批删除（D56） |
| P4 → P5 | `include!` 改造与 checker 拆分动同一文件 |
| P5 → P6 | 同一文件的连续改动必须分开，否则回归无法二分定位 |
| P6 → P7 / P8 | 类型表示不先收敛，新 IR 会长成第三套表示；**P7 闸口 = 6.9 的阶段 A/B/C 出口**（I1–I3、I5、I7；即 6.9.2 含 C3、6.9.3 含 d、6.9.4 完成），D（Σ）不阻塞；P7 只能消费量词派生视图与 defeq 接口，不另建平行账本；7.0 纯搬移批是 P7 前置，不受此闸口约束，但不得消费未定的 defeq/量词接口 |
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

**不可并行（文件重叠）**：P1(1.1.x) ∥ P8(8.0.x)；1.2.1 ∥ 8.3.2；6.0 ∥ S2（B6/E5 同批文件）；4.3.2 ∥ 9.2（已统一到 P4）；**3.4 ∥ P3.5**（`checker.rs` / `environment.rs` 同文件——外部轨道的删 pub 批次与静默通道修复必须串行）。

**文档内部新增约束**：6.0→6.1→6.2→6.3→6.4→6.5→6.6 严格串行；7.0 → 7a→7b→7c→7d 严格串行（7.0 为 P7 前置纯搬移批），且 7b 的 `Phi` 臂必须先于 7d。

## 关键决策与理由

- **P0 独立成阶段而非并入 P9**——门禁是"事后检查"，规程是"事前判断"。P9 的脚本无法阻止"该重构却打了补丁"，只有 D0–D4 的 review 清单能。
- **P9 作为 CI 脚本的唯一权威清单**——原缺口 G3 的三套并存清单导致无法收口。本表合并后 10 个在用脚本各归属一个阶段（9.8 经用户 2026-11-24 取消），report-only 与转硬时点显式可查。
- **P3.5 收编 RFC-029g 而非留作"独立轨道、等重构后做"**——它的删除面与 P4/P5/P6/P7/P8 的重写面重合（RFC-039 D56）。留待重构后执行，等于在新文法、新 checker 布局与新 IR 上把同一批删除重做一遍；这与 7e 收编 `2315`/`2341`、S4 收编 RFC 状态修正是同一条教训。
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
| C1 | `precedence_inline.rs` 在 P1 复活、P8 删除 | **复活**（P1 的前提是复活有真实断言的测试）；P8 不再删除该文件（Pratt 保留） |
| C2 | "文法驱动"承诺 vs 查表化 | **回落方案**：保留 Pratt + 单套声明式优先级表 + 两套 `BinOp` 同源互校（LALRPOP 路线撤回），见 P8 8.3 |
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
