# 多级施工任务表（WBS）

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档，把 RFC 的阶段序列展开为**可独立提交、可独立验证**的三级任务。
>
> 阶段顺序与验收门禁以 RFC-039 正文为唯一权威；本文只负责拆解到可执行粒度，并登记尚未裁决的冲突。

## 定位与范围

| 层级 | 数量 | 含义 |
| --- | --- | --- |
| **一级** | 11（P0–P10） | 阶段，与 RFC-039 的阶段序列一一对应 |
| **二级** | 67（2026-10-13：53 +2 = 3.13 / 3.14；2026-10-14：+1 = 3.15；2026-10-14：+1 = 3.16；2026-10-15：+2 = 3.17 / 3.18；2026-10-16：+1 = 0.4；2026-10-18：+3 = 3.19 / S7 / S8；2026-10-18：+2 = 3.21 / S9（该组原登记号 3.20，2026-10-21 迁至 3.21，3.20 号让给「死代码分析的既有误报」，见 [09a](./09a-wbs-history.md) 收口实录）；2026-10-21：+2 = 0.5 / 3.20） | 任务组，来自各篇文档的「实施要点」＋ 2026-10-07 审计补遗（P3 的 3.4）＋ 2026-10-10/11 裁决补登（P3 的 3.5/3.6/3.7/3.8/3.9）＋ 2026-10-12 登记（P3 的 3.10）＋ 2026-10-12 发现补登（P3 的 3.11）＋ 2026-10-12 发现再补登（P3 的 3.12） |
| **三级** | 176（2026-10-13：144 +2 = 3.13.1 / 3.14.1；2026-10-14：+2 = 3.15.1 / 3.16.1；2026-10-15：+2 = 3.17.1 / 3.18.1；2026-10-16：+2 = 4.6.2 / 4.9.6（`#450`，用户裁决 A）；2026-10-16：+1 = 0.4.1；2026-10-17：+1 = 4.9.7（`#464`）；2026-10-18：+3 = 3.19.1 / S7.1 / S8.1；2026-10-18：+12 = 4.9.8 / 3.14.2 / 4.10.4 / 4.10.5 / 3.21.1–3.21.4（原 3.20.1–3.20.4，2026-10-21 迁号）/ S9.1 / S9.2 / S2.2 / 4.4.4；2026-10-21：+7 = 3.6.2 / 3.6.3 / 3.6.4 / 0.5.1 / 0.5.2 / 3.20.1 / 3.20.2） | 可独立 commit 的动作 |

**编号留空（不回收）**：P4 无 `4.7`/`4.8`、P8 无 `8.1`。三者自 WBS 建立起即无对应条目——本文全史对 `**4.7` / `**4.8` / `**8.1` 三个标题零命中（`git log -S'**4.7' -- docs/src/dev/architecture/09-execution-wbs.md` 可复核），全仓亦无引用。`8.1` 对应的正是 `05` 的阶段 1（测试复活与补全）：该阶段已拆入 P1（`1.1`–`1.3`）与 `8.8`，因此 P8 的编号与其阶段号对齐（阶段 0 → `8.0`、阶段 2 → `8.2`，依此类推）。

**本文覆盖**：任务拆解、依赖关系、并行分组、验收判据绑定。

**本文不覆盖**：任务的设计理由（见 `02`–`08`）、阶段顺序的论证（见 RFC-039）。

**登记约定（2026-10-12 起）**：新登记的二级/三级行**必须带两个字段**——「**归属阶段**」（它真正属于哪个 P，可与所在章节不同）与「**是否阻塞主线**」；`登记待排期` 不再作为默认状态，未标注即视为**不可排期**。编号在全文档内**必须唯一**（跨轨道的任务用轨道号，如 `029g.x`，不得占用 `N.x`）。 **勾选/完成登记以「任务 id + 证据（门禁数值、run id、issue 号、file:line）」为准，不写提交号**——`pull --rebase` 会重写本地哈希，写死的号必失效（2026-10-13 实例：`94ad03ce4`→`631eb3e3c`）。提交号查 `git log`。

## 现状：拆解时发现的缺口

在把 8 篇文档的「实施要点」汇总成本表时，发现**文档体系本身有 5 处硬缺口**。全部已收敛：

| # | 缺口 | 收敛结果 |
| --- | --- | --- |
| G1 | **P5 无任何施工步骤**。RFC 定义了 P5（checker 拆分）与验收（C1 zero-diff），但 7 篇文档中没有一节描述这个拆分怎么做；`01-routing.md` 的目录树里出现过 `annotations.rs` 一行，是全批唯一提及 | P5 步骤由本文补齐（见下） |
| G2 | **`include!` 改造三处归属、零处落地**。RFC 列在 P4；`02` 的参见归给自己；`06` 的 S7 也列了。但 `02` 的改动清单（6 新增 + 20 修改）里**没有 `semantic_tokens.rs`** | 施工步骤归本文 §P5 5.1（前置 P4 完成）；RFC 与 06 的引用已同步指向这里 |
| G3 | **CI 脚本三套并存清单**。`RFC` 列 4 个 `scripts/ci/*.py`；`01` 列 4 脚本 + 1 `cargo test`；`07` 另列 4 个检查（命名全不同）；`04` 还要 `check-synth-boundary.py`。去重后**至少 9 个新脚本，无统一清单**；`check-obligations.py` 在 P4 与 P9 双重归属 | 统一清单见本文 §P9（11 个脚本，各归属一个阶段），其他文档一律引用该表 |
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

- [x] **0.1 规程成文**（3 个三级任务；0.1.2/0.1.3 于 2026-10-05 提前落地；0.1.1 于 2026-10-06 review 通过：内容核对 + CONTRIBUTING.md 全中文化，英文版迁 docs/gh/CONTRIBUTING.en.md）
- [x] **0.2 门禁实现**（3 个三级任务，2026-10-06 完成：check-concepts / check-fanout / check-boundary 均以 report-only 接入 CI（concepts job），验收负向探针全过（新增入口接线红 / L2→L3 反向 use 红 / 未改动代码报出 3 套运算符枚举 + 2 套平行类型表示含 ir::Type 别名），pub(crate) 基线 137，P9 统一转硬）
- [ ] **0.3 表门禁推广**（1 个三级任务）
- [x] **0.4 规程承诺与工具行为一致性**（1 个三级任务；**2026-10-16 完成**——`check` / `run` 的显式路径不再被默认排除表拦下，AGENTS.md 第 9 条的探针位置兑现；顺路修 `--exclude` 恒空；issue [#463](https://github.com/ChenXu233/YaoXiang/issues/463)）；**归属阶段：P0；是否阻塞主线：否**
- [x] **0.5 测试套件分层**（2 个三级任务，2026-10-21 登记，issue [#486](https://github.com/ChenXu233/YaoXiang/issues/486)；**0.5.1 已完成**——`cost:` 就地资源画像标记 + 独占锁 + `fast`/`full` 双通道最小消费面；**0.5.2 待排期**（pre-commit / CI 接线 + 2600 档实测校准 + `test-specification` 细则节）；**归属阶段：P0；是否阻塞主线：否**）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 0.1 规程成文 | 0.1.1 `CONTRIBUTING.md` 写入 D0–D4 + 三条禁令；职责类别引用 `01-routing.md` 的目录职责表 | — | 人工 review 通过 |
| | 0.1.2 实现者触点落地（**2026-10-05 已完成**）：新建 `AGENTS.md`（根目录入口）+ `compiler-architecture/HOWTO.md`（动工自检 + D3 补丁判定）；PR 模板加「职责归属与决策程序」必填块（D44）；`CONTRIBUTING.md` 加「代码放置与变更规程」节 | — | 已落地；review 核对四触点内容 |
| | 0.1.3 规则本体抽取（**2026-10-05 已完成**）：新建 `docs/src/dev/coding-rules.md`（精确规则描述，长期有效，不随 RFC-039 归档）；08 加迁移头注转为诊断记录；四触点全部改指 coding-rules | — | 已落地 |
| 0.2 门禁实现 | 0.2.1 `check-concepts.py`（禁令一 A/B/C/D） | 0.1.1 | **必须在未改动代码上报出 3 套运算符枚举、2 套平行类型表示（`ir::Type` 别名一并计入）**；本阶段 report-only，P9 收口转硬 |
| | 0.2.2 `check-fanout.py`（禁令三 A/B/C） | 0.1.1 | **故意新增第 6 个入口式接线必须红**；本阶段 report-only，P9 收口转硬 |
| | 0.2.3 `check-boundary.py`（禁令二判据 C） | 0.1.1 | **故意加一处 L2→L3 反向 `use` 必须红**；`pub(crate)` 泄漏计数记初始值 |
| 0.3 表门禁推广 | 0.3.1 `tools/code-tables` 扩展覆盖 opcode / 类型表 | 0.1.1 | 依赖 P6 的 T1；单独收口 |
| 0.4 规程承诺与工具行为一致性 | 0.4.1 默认排除表只作用于**遍历到的子项**（按 `file_name` 判，不看祖先路径）、不作用于用户显式给出的路径；`--exclude` 与比较基准同源（**2026-10-16 完成**——红→绿见 `tests/integration/cli_e2e.rs::test_e2e_check_explicit_path_under_target_is_honored`，三形态 + 一反向判据；issue [#463](https://github.com/ChenXu233/YaoXiang/issues/463)） | — | 仓库内 `check target/…/x.yx`（文件与目录两形态）可直接跑；嵌套 `target/` 子目录仍跳过；`--exclude` 真正生效（`check src/std --exclude src/std` ⇒ 零发现） |
| 0.5 测试套件分层 | 0.5.1 测试资源画像（`cost:` 就地标记）+ 重测独占锁 + `fast`/`full` 双通道最小消费面（**2026-10-21 已完成**：新增 `scripts/dev/testsuite.py`（`markers --check` 与 `cargo test --list` 双向对账、`plan` 预算装箱、`run fast` / `run full` 预算不足即报缺口不静默跳过、`manual` 档永不隐式执行）与 `src/util/test_support.rs`（跨进程文件锁，测试体内取锁 ⇒ 任何调用路径互斥）；就地标记 3 处（2600 档 exclusive，峰值由 5000 档实测线性外推**未实测**；5000 档 exclusive，15.6 GiB / 72.6 s 实测；快照更新 manual，峰值与耗时均为外推）；证据：`markers --check` 核对 3149 个 `#[test]`、3 例机械变异各自 exit 1、`run fast` 批量波 2793 passed / 47.7 s + 独占波 1 passed / 14.6 s、预算 1 GiB 时 `run full` 列出缺口 7.11 GiB、两进程争锁实测 `held→blocked→held(waited 2.0s)`） | — | 标记是单一事实源且与 `cargo test --list` 双向一致；独占档任何调用路径都互斥；预算不足必须报缺口不得静默跳过；**归属阶段：P0；是否阻塞主线：否** |
| | 0.5.2 通道接线与校准：pre-commit 的 `cargo test` 钩子与 CI 切到 `fast`（本地提交前）与 `full`（推送前与 CI 口径）双通道，2600 档资源画像由实测校准（现为外推值），`test-specification.md` 增补资源标记与独占档的细则节（**待排期**；#486 后续切片） | 0.5.1 | `pre-commit` 的 `cargo test` 钩子不再裸跑全量（现为 `cargo test --all` 全量；AGENTS.md「常用命令」节已写明 fast/full 双通道与「禁止裸跑 `--ignored` 全量」红线）；CI 与本地提交前走同一通道口径；2600 档标记的峰值与耗时为实测值而非外推值；`test-specification.md` 有对应细则节；**归属阶段：P0；是否阻塞主线：否** |

> **没有规模基线任务**。2026-10-03 决策取消一切行数 / 体积门禁，规模问题由职责分离解决（见 `08` 禁令二判据 A，人工判断）。
> **0.2.1 是本阶段的核心验收**：门禁若报不出已知的平行表示，说明判据设计错了，是假门禁。

### P1 复活孤儿测试并修缺陷

来源：[06](06-cleanup-inventory.md) §实施要点 S1 + [05](05-frontend-paradigm.md) §测试重建

- [x] **1.1 复活 `lexer/tests/`**（4 个三级任务，2026-10-06 完成：55 个词法测试上线；删 7 空壳；`#[path]` 旁路并入正接线避免重复运行）
- [x] **1.2 复活活目录中的 4 处**（2026-10-06 完成：precedence_inline 6 + overload_inline 7 + json 3 + template 7；另删 typecheck/tests/semantic_db.rs 残骸 5 行）
- [x] **1.3 修复暴露的缺陷**（2026-10-06 完成，**实测与预判不同**：溢出路径与 \x/\u 转义当前代码无恙，20 个探针测试补上从未有的覆盖；真正暴露的缺陷是「基数字面量后非法字母数字邻接被静默拆分」——`0b102` 编译通过且运行得错误值 2、`0o128` 得 10、`0x1FG`/`123abc` 报误导性 E1001。修复：`literals.rs` 新增共享助手 `reject_trailing_alnum`（禁令三合规：一处实现五处调用），5 个先红后绿测试全绿）

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

- [x] **2.1 IR 静态校验器**（3 个三级任务，2026-10-06 全部完成：verify.rs 双模式 + 7 不变量 + 语料跑绿，实测修复 ir_gen 三缺陷）
- [x] **2.2 规范化快照**（2 个三级任务，2026-10-07 完成：normalize.rs + 204 快照入库 + 门禁 9.5 入 CI）
- [x] **2.3 语料差分**（3 个三级任务，2026-10-07 全部完成）
- [x] **2.4 漏洞专门判据**（3 个三级任务，2026-10-06 全部完成：2.4.1/2.4.3 随 P3 转绿，2.4.4 红态就位待 P4/D20）
- [ ] **2.5 回归门禁**（1 个三级任务）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 2.1 IR 静态校验器 | 2.1.1 `verify.rs` 双模式 `verify_loose` / `verify_ssa`（**2026-10-06 已完成**：`src/middle/core/verify.rs`，07 §102 契约形态 `verify(ir, mode) -> Result<(), VerifyError>`，违规全收集非首错即停；def/use 提取穷尽 match 无通配臂，Phi 变体引入时编译失败强制补查） | — | C4 |
| | 2.1.2 **`verify_loose` 在现有非 SSA IR 上跑绿**（**2026-10-06 已完成**：`test_verify_loose_corpus_green` 全语料 204 文件/536 函数/11990 指令绿，校验面非空断言内置；实测按 D38 修复 ir_gen 三缺陷，语料差分基线自比零差分证行为等价） | 2.1.1 | **硬门槛，P7 批 a 的准入条件**——**已解锁** |
| | 2.1.3 实现 7 项不变量（支配性/唯一定义/Phi一致/jump目标/全局越界/类型一致/内层隔离）（**2026-10-06 已完成**：判定面按实测登记收窄——支配性=具名槽 must-defined（temp 槽 Void 预初始化是帧语义，ir_gen 有意依赖）、类型一致=参数槽签名+常量可信源（ir_gen 不写槽位类型表，完整面归 P7 值表 04 §262）、Phi 待批 b 变体引入；详见 verify.rs 文件头与 06 §G6） | 2.1.1 | C4 |
| 2.2 规范化快照 | 2.2.1 规范化工具（剥 Span / 临时值重命名 / 槽位相对化 / 前驱排序）（**2026-10-07 已完成**：`src/middle/core/normalize.rs`，07 五规则逐项实现；实测修正两处——DefId 按首次出现序相对化（intern 序跨进程不稳定，141/143 实测）、Arg 独立 arg% 前缀（与 Local 同池会丢变体区分）；指令打印穷尽 match） | — | C1/C3/C5 |
| | 2.2.2 快照入库 + 人工 review 流程（`src/middle/core/tests/snapshots/`）（**2026-10-07 已完成**：204 快照按语料相对路径镜像入库；更新流程 `UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored` + git diff review + 同 PR 入库，见 tests/snapshot.rs 文件头；篡改实测精准报红） | 2.2.1 | C1/C3/C5 |
| 2.3 语料差分 | 2.3.1 差分框架 + 293 语料基线（**2026-10-06 已完成**：探针 `examples/corpus_probe.rs` + 基线 `tests/baselines/corpus-parity.jsonl`（322 条）+ 门禁 `check-corpus-parity.py` 入 CI（硬门禁）；归一化含指针形态 scrub，两次全量运行门禁自比零差分） | — | 诊断/退出码/stdout 逐项 |
| | 2.3.2 **多文件语料层**（新建 `tests/yaoxiang-multifile/`，带 `yaoxiang.toml` 的项目夹具——决议 D48）（**2026-10-06 已完成**） | — | **无条件必做（D40），是 P4 唯一可执行的行为判据来源** |
| | 2.3.3 C4 行为差分必覆盖清单（10 类语义）+ 性能基线（criterion 冒烟基准）（**2026-10-07 已完成**：10 类语义逐项核对全覆盖，26 代表文件 26/26 在 corpus-parity.jsonl 差分基线内；冒烟基准 `benches/pipeline.rs` 首测——全语料编译 17.6s / CLI 冷启动 169.7ms，解释吞吐 benches/lib.rs hotpath 组已有覆盖） | — | C2/C4/C5；性能基线供 P4/P7/P8 对比 |
| 2.4 漏洞专门判据 | 2.4.1 `test_multifile_proof_obligation_not_dropped`（**2026-10-06 已完成：红态就位**，实测单文件 ["E4018"] vs compile_project []） | 2.3.2 | **必须先为红** |
| | 2.4.3 `test_no_silent_pass_on_unproven`（**2026-10-06 已完成：红态就位**，触发源在 compile_project 下实测静默） | — | 拦 `checker.rs:5179/5318/5448` |
| | 2.4.4 `test_release_plan_spans_consumed`（**2026-10-06 已完成：红态就位**，实测 168 文件 373 键未被消费——D20 契约缺陷实证，修契约归 P4/D20） | — | **无白名单，差集必须为空（D41）** |
| 2.5 回归门禁 | 2.5.1–2.5.4 见 P9 统一清单 | 2.1–2.3 | 见 P9 |

### P3 修正确性漏洞（最小方案）

来源：[02](02-stage-contract.md) §实施要点 S3 的漏洞修复半部 ＋ **2026-10-07 静默丢弃点审计（3.4）**

> **语义修正分流（D59，2026-10-11 用户裁决）**：本轨道及其后续阶段若发现「旧行为不算成功」的缺陷——**A 静默失败 / B 静默错配 / C 未定义行为与资源失控**（崩溃、爆栈、静默给错答案）——**可随重构顺路修**，四条缺一不可：①先红后绿（红判据先存在且确实红）；②不改变「原先能成功编译并正确运行」程序的对外表现（不得变成失败，不得删改其既有诊断码/文案/退出码）；③同提交登记（本行标注错误类型 + 该阶段修订注记 + PR 必填块「错误类型」一行）；④基线同提交再生 + 人工 review diff。**D 用户可见语义 / E 对外契约表 / F RFC 级设计变更三类仍必须先经用户裁决**。判据展开与 A–F 白名单见 [07 §语义修正的分流判据](07-equivalence-oracle.md)；**不采纳「默认超级重构 + 事后登记」**（理由见 RFC-039 D59：零红判据无法区分修复与回归，且与「不存在 C5′」「不设任何放宽」冲突）。
>
- [x] **3.1 `proof_calls` 消费端**（2 个三级任务，2026-10-06 完成）
- [x] **3.2 第二个静默丢弃点**（1 个三级任务，2026-10-06 完成；**表述与实测不符，见 3.2.1 行更正**）
- [x] **3.3 panic 转诊断**（1 个三级任务，2026-10-06 完成）
- [ ] **3.4 静默丢弃点补遗**（9 个三级任务，2026-10-07 审计新增；3.4.9 于 2026-10-13 补登；3.4.1/3.4.2/3.4.3 已实施，3.4.4–3.4.8 随 P4 前置推进；**3.4.4 / 3.4.5 / 3.4.9 已于 2026-10-13 清账完成**（跨二级加框并入 4.3 义务账本团队，红名单归零）；**3.4.8 ① 已完成** a9438007；**3.4.8 ②（含 ① 收口 D2）已完成** 375789d2、d922a551；**3.4.8 ③④（含 F1 补口）已完成** 8ecf56f93；**3.4.6 已完成**（2026-10-19 用户裁决「做」——多文件 `run` 改由同一判据输出死代码族与 W1003 警告，D 类，对外契约面 `compile_project` 返回值改为 `(ModuleIR, FileDiagnostics)`；原 2026-10-15 裁决丙「按 B1 缓议」的前置 4.9 已于 2026-10-17 收口）；**归属阶段：P3；是否阻塞主线：否**——P3 侧欠账清零（余下 3.4.8 ③④ 随 P4 前置推进），P4 闸口已实质越过、不卡 P5 前置）
- [x] **3.5 高阶类型参数调用路径补全**（3 个三级任务，2026-10-10 登记；用户裁决甲——RFC-010 元类型层级与实现齐备而调用侧断裂，属「已接受特性未实现」；**3.5.1–3.5.3 已完成（2026-10-21 收口登记）**：①合一器改在**头部位置**匹配构造器变量（判据单一源：`is_ctor_reference` 全仓 1 定义 3 消费，头部改名收敛到共享实现 `substitute_ctor_heads`）；②显式类型实参接受类型构造器（E1010 消失）；③mono 产物中构造器变量被替换为真实构造器；复审轮 1 的三条 findings（同名两份实现 / 内联复制头部改名 / 可见性越界）已真修，轮 2 复审 **pass**（8 条自建探针值与退出码逐条相同、4 份 IR dump 与修复前逐字节一致、变异复跑仍可失败）；门禁：lib 2794 passed + 2 ignored、integration 297 passed（0 failed）、语料 348/348 零漂移、IR 快照 204 零漂移、`check-boundary` 的 `pub(crate)` 计数 167 / frontend-core 26（= 3.5 前基线）、`check-fanout` 同名定义零命中、fmt / clippy / docs-truth 全 exit 0；**归属阶段：P3；是否阻塞主线：否**）
- [x] **3.6 函数类型值调用路径**（4 个三级任务，2026-10-10 登记；用户裁决甲——闭包变量按名调用运行期 E6006；语义裁 (i)：支持间接调用；**3.6.1–3.6.4 已完成（2026-10-21 收口登记）**：①局部函数类型值按名调用改走间接调用（IR 由 `LoadConst String` + `CallNative` 变为 `LoadGlobal` + `CallDyn`）；②`use` 别名导入的函数值按名调用；③**嵌套函数体引用外层帧值改报编译期 E1001**（判据改判：不是「让它跑对」，而是该形态按 spec 应编译期被拒；赋值路径由 E2010 兜住）；④`use lib` 整体导入后 `lib.f(...)` 走间接调用；判据单一源：`is_static_fn_name` 全仓 1 定义 1 调用；门禁：lib 2794 passed + 2 ignored、integration 297 passed（0 failed）、语料 348/348 零漂移、IR 快照零漂移、fmt / clippy / docs-truth 全 exit 0；**归属阶段：P3；是否阻塞主线：否**）
- [x] **3.7 多文件 init 段调用名限定**（4 个三级任务，2026-10-11 登记；用户裁决乙′——t3 发现的根因 F1；**3.7.1–3.7.3 已完成** d922a551；**3.7.4 已完成** 38bd4b9e（208 快照再生，def-lines 309 / other-lines 0））
- [x] **3.9 运行期栈守卫不可达（E6004）**（1 个三级任务，2026-10-11 登记；**3.9.1 已完成** 2026-10-12，团队 `yx-39-stackguard`：蹦床化 + limit 单点化 + 回显截断；实录见 [09a](./09a-wbs-history.md)）
- [ ] **3.8 native 对用户函数名的遮蔽**（1 个三级任务，2026-10-11 登记；t2 发现的既有怪癖，缺钉测试；**归属阶段：P3；是否阻塞主线：否**——验收要求先定语义（用户定义优先 vs 显式诊断），属裁决点）
- [x] **3.10 编译期无闸原生递归路径（抬高 `mono.max_depth` 后崩溃）**（1 个三级任务，2026-10-12 登记，issue #447；**3.10.1 已完成** 2026-10-12：定位 `MonoType::type_name` 并四处工作列表化；实录见 [09a](./09a-wbs-history.md)）
- [x] **3.11 错误码保真与深栈回显**（1 个三级任务，2026-10-12 发现并登记；**3.11.1 已完成** 2026-10-12，② 深栈回显并入 3.9.1；实录见 [09a](./09a-wbs-history.md)）
- [ ] **3.12 运行时模式间的执行器配置传播**（1 个三级任务，2026-10-12 发现并登记；Standard 模式的任务解释器由 `Interpreter::for_task`（`backends/interpreter/executor/executor.rs:201`，调用点 `:563`）按**默认** `ExecutorConfig` 建（max_stack_depth=1024），宿主上用户配置的 `max_stack_depth` 不传播 ⇒ ① Standard 模式 E6004 的 limit 恒为 1024；② 小上限配置在任务内被**静默忽略**。**属 D 类**（传播后会让「依赖该静默忽略而成功」的宿主程序变失败）⇒ **用户 2026-10-12 裁决乙：本轮不并入，另起队伍修**；**归属阶段：P3；是否阻塞主线：否**——D 类变更须在该队内先获裁决）
- [x] **3.13 ir_gen 方法绑定 panic（方法组当值再绑定 + 无签名标注）**（1 个三级任务，2026-10-13 登记，issue [#451](https://github.com/ChenXu233/YaoXiang/issues/451)；D59 **A 类**（崩溃）——最小复现已由队长独立复现：`cargo run --quiet -- check <file>` → `panicked at src\middle\core\ir_gen.rs:1678:50: called Option::unwrap() on a None value`、`exit=101`；**用户 2026-10-13 裁决：登记 + 排期**；**3.13.1 已完成 2026-10-15**：两形态由 panic 转为编译期响亮拒绝 E1110（`exit=1`，原 panic 点 `ir_gen.rs:1678` 转为错误返回）——t1 收口（沿用前轮交付）→ t2 独立验证（最小复现 `exit=101`→`exit=1`、typecheck 面零误伤、`--lib` 2724 passed、corpus 342 行等价）；复核与修复轮证据见 3.14 行（同批 t3→t9→t10 闭环））
- [ ] **3.14 方法级重载决议纳入命名实参**（1 个三级任务，2026-10-13 登记，issue [#452](https://github.com/ChenXu233/YaoXiang/issues/452)；D59 **D 类**（用户可见语义）——**用户 2026-10-13 裁决：直接排期**，终态 = 命名实参参与候选匹配；原现态：① 纯命名不进决议 ⇒ 默认绑定（不符签名报 E1002、恰好符合则碰巧跑通），③ 混合形态经 4.5.2 后为编译期 `error[W1090]`（C 类诚实边界）；由 4.5 收口后的下一支队伍承接；**3.14.1 已完成 2026-10-15**：命名实参与位置实参同表逐位匹配（求解器克隆沙箱，`expressions.rs:4692-4735` 一轮形态），纯位置/混合/纯命名值级矩阵 102/102/102、按名选候选 20（t2）；t3 复审 needs_revision（F1 元数守卫静默链等 F1–F5）→ t9 修复并新增对外码 E1111（码表/locales/RFC-013/生成器四件套同提交）→ t10 复审 pass；修复轮门禁 `--lib` 2727 passed、CI Docs Truth 38545762349 绿；终态判据单点收敛见 D60）；**2026-10-18 重开**：新增三级 3.14.2（issue [#466](https://github.com/ChenXu233/YaoXiang/issues/466)）
- [x] **3.15 `typecheck::infer_expression` 零调用点死 API**（1 个三级任务，2026-10-14 登记，issue [#453](https://github.com/ChenXu233/YaoXiang/issues/453)；4.5.2 复审 F3 发现——本身无害，但**接线即成 D20 同类缺口**：推断走了、决议不回流；**3.15.1 已完成 2026-10-15**：处置 = 删除死入口（连同悬空文档自指文本）；t2 独立验证直证：`git grep -w` 于 src/tests/examples 零命中、docs 仅 WBS 登记自指；门禁同 3.13/3.14 行（`--lib` 2724 passed @ t2 复核轮））
- [x] **3.16 stdlib 原生导出命名实参一致性**（1 个三级任务，2026-10-14 登记，issue [#454](https://github.com/ChenXu233/YaoXiang/issues/454)；t2 独立验证的边界发现 + 队长在 `45418b717` 亲自复现：`io.format_fallback(42, "int")` → `int(42)`，而 `(value = 42, type_name = "int")`（**形参名正确**）与 `(value = 42, kind = "int")`（**形参名错误**）都只输出 `()`、exit=0 零诊断 ⇒ 命名实参打到原生导出上被**整体忽略**，未知形参名/缺参/重复命名三类校验全缺席；D59 **C 类**（静默错值 / 静默不动作）；归因实验（verify14 在 `12b4fa2a9` 重建，同形同输出）证 **非 #452 引入**；**用户 2026-10-14 裁决①：并入本批次，收口前加一轮**；**3.16.1 已完成 2026-10-15**：t2 边界发现 → t6 修复（共享解析入口 + 原生臂按名结算，门禁 `--lib` 2730 passed、CI 38550504517 全 8 job 绿）→ t7 独立验证 PASS（值级矩阵 12 格、语料 342/342、5 处变异经 `scratch.py trial` 全抓获且还原字节一致）→ t8 复审 needs_revision（F1：原生臂内联副本缺规则④ ⇒ 方法/原生裁决分歧；ir_gen 第三份存量副本报旧码）→ t12 收敛共享判据 `bind_args_to_params`（`arg_binding.rs:35`，四调用点 `expressions.rs:2727` / `:5416`、`ir_gen.rs:7776` / `:8279`）→ t13 复审抓到 t12 引入的「原生可省略位」回归（原先成功程序误报 E1010）→ t14 修复（可选位判据与形参名同源，`signature.rs:236` 唯一出口）→ t15 复审 PASS（独立复现红→绿、mpA 下仓内钉板 FAILED 实证判据可失败）；收尾门禁 `--lib` 2730 passed / integration 284 passed / corpus 342 行零 diff / IR 快照零漂移；裁决登记见 RFC-039 D60）
- [ ] **3.17 手注 FFI 命名实参静默丢弃**（1 个三级任务，2026-10-15 登记，issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)；批次 3.13–3.16 复审 F4 遗留——`add_native_signature`（`environment.rs:789`）只灌签名表与 arity、**不灌形参名表** ⇒ 经该路径注册的原生函数命名实参调用被静默丢弃（位置实参正常）；仓内**零调用者**，今天不影响任何程序，风险面向宿主嵌入（FFI）；D59 **C 类**（静默不动作）；**归属阶段：P3；是否阻塞主线：否**）
- [ ] **3.18 约束泛型运算符派发的跨模块类型身份**（1 个三级任务，2026-10-15 登记；4.10.3 施工实证——`sum: (T: Add) -> (a: T, b: T) -> T = a + b` 在**类型与泛型定义不同模块**时（`lib.Vec3` + `use lib.{Vec3}`），mono 的派发改写产裸名 `Vec3.add`，merged 表实为 `lib.Vec3.add` ⇒ 运行期 E6006；**Bin 项目同形复现（预存在缺陷，非 4.10.3 引入）**；根因 = MonoType 用户类型名是裸名义名、不携带定义模块，mono 无法还原限定名——正确修法属 P6 类型身份收敛（`03-type-unification.md` 路由）；同模块情形已由 4.10.3 顺路修（`qualify_deferred_method` 查表验证）；D59 **C 类**（静默失败——MultiFile 路径下该程序今天即失败）；**归属阶段：P3（前置待 P6）；是否阻塞主线：否**）
- [ ] **3.19 精化谓词的 SMT 排序翻译崩溃**（1 个三级任务，2026-10-18 登记，来源 = 4.4.2 施工期探针；**归属阶段：P3；是否阻塞主线：否**）
- [ ] **3.20 死代码分析的既有误报**（2 个三级任务，2026-10-21 登记，来源 = WBS 3.4.6 施工期暴露的两个**既有** W 码误报（用户 2026-10-19 裁决丙：文档示例改为不触发误报的等价写法，误报本身登记待修）；**回滚义务**：本组修好后必须把 `docs/src/reference/stdlib/weak.md` / `docs/src/reference/stdlib/test.md` 中被改写以绕开误报的示例**恢复为自然写法**并再生基线（语料基线会再动一次）；**归属阶段：P3；是否阻塞主线：否**）
- [ ] **3.21 发版前审计（v0.8.4）补登缺口**（4 个三级任务，2026-10-18 登记，来源 = `v0.8.4..HEAD` 的发版前审计；**2026-10-21 迁号**：3.20 号让给「死代码分析的既有误报」，本组整体迁至 3.21（三级改 3.21.1–3.21.4，issue 锚不变）；**归属阶段：P3；是否阻塞主线：否**）
>
> **批次（2026-10-14 用户裁决）**：3.13 / 3.14 / 3.15 / 3.16 四个小项**相关性大**（方法绑定、决议、推断入口同属一条路径；3.16 是同一套「命名实参」判据的另一载体——stdlib 原生导出），**合并为一支团队处理**——团队范围由「一个二级任务」放宽为「一组相关小项批次」；批次构成、理由与偏差须随该队收口登记（实录见 [09a](./09a-wbs-history.md)）。

| 二级 | 三级 | 文件:行 | 前置 | 验收 |
| --- | --- | --- | --- | --- |
| 3.1 `proof_calls` 消费端 | 3.1.1 `proof_calls` 收窄可见性 + 唯一 getter（**2026-10-07 核实：实为 `pub(super)` + `pub fn proof_calls()`，非「私有」**） | `types.rs:36/84` | 2.4.1 | 漏洞判据转绿 |
| | 3.1.2 消费点补齐 | `orchestrator.rs:99` / `:273` / `:450` / `:1374`（实测四入口 + 单文件共 5 处） | 3.1.1 | C2 |
| 3.2 第二个静默丢弃点 | 3.2.1 `Unproven` 空 match 臂产出诊断（**2026-10-07 核实：`ownership.rs` 只构造 `Proved`/`Disproved`，本臂源码不可达，属防御性代码；判据转绿由 3.1 承担，本项无独立判据**） | `checker.rs:1303-1314` | 2.4.3 | `test_no_silent_pass_on_unproven` |
| 3.3 panic 转诊断 | 3.3.1 `.expect()` → `SMTResult::Unknown` + 诊断 | `predicate.rs:34-36` | — | Z3 缺失下不再 panic |
| 3.4 静默丢弃点补遗（2026-10-07 审计新增） | 3.4.1 const 泛型约束 `Unproven` 空臂改产诊断/记账（**审计新增：全仓文档零登记、零任务**）（**2026-10-07 已实施**：接线已注册的 W1063——事实链 environment→ExpressionInferrer→StatementChecker→checker 汇聚发射；bounds 层区分 TypeVar 内部探测实参防误报；红判据先行） | `environment.rs:422-426` | — | 新判据先红后绿：const 实参不可编译期求值时不得静默 |
| | 3.4.2 终止检查求解器缺失留痕（**2026-10-07 已实施**：新码 W1081——`count_unjudged_obligations` 纯函数取信号（递减+良基性两表合并），checker 汇聚发射；NotProved（求解器在但判不出）不触发，本码专属「求解器缺失」） | `checker.rs:1266-1268` | — | Z3 缺失时产 W 级诊断；与 3.3.1 同一降级哲学 |
| | 3.4.3 02 §91 阶段覆盖表第 1 行同步（**2026-10-07 已完成**：表保留为 9e02e4db 诊断快照，复核注记逐格标注现状去向；`#434` 裁决补登为 RFC-039 **D57**） | `02:95-107` | — | 人工 review：逐格与代码核对 |
| | 3.4.4 `variant_ctor_calls` 零消费者裁决（**2026-10-13 已完成**，631eb3e3c——取「删字段」：IR 侧 `ir_gen.rs:7448` `detect_variant_ctor_call` 就地做同一 AST 形态检测，函数 doc 明写 span 键跨系统不可靠，接线消费端会把已修的 span 失配缺陷引回；删除面 types.rs / environment.rs `VariantCtorCall` / checker.rs / inference 两文件的采集 push，类型推断与诊断不变） | 字段已删、`ir_gen.rs:7448`（形态检测） | 4.3.1 | ✅ `check-obligations.py`（9.2）不再报该字段；t7 独立验证 / t8 复审（进行中）；**2026-10-13 用户裁决：跨二级加框并入 4.3 义务账本团队** |
| | 3.4.5 `method_overload_ir_names` 与 `overload_resolutions` 平行表示二选一（禁令一）（**2026-10-13 已完成**，631eb3e3c——删 `TypeCheckResult` 上的跨阶段镜像 `types.rs:59-60`+`checker.rs:1558`；在相位候选名表保留、命名事实经 `overload_resolutions`（`ir_gen.rs:380`/`:7632`/`:7705`）到 IR，定义侧混编名由 `method_def_ordinals`（`ir_gen.rs:1828`）自算；镜像全仓零读者有 `git grep` 直证） | 镜像已删、`ir_gen.rs:380` | 4.3.1 | ✅ 同一事实单点表达（C3）；红名单 3→1；t7/t8 进行中 |
| | 3.4.6 `compile_project` 补 `warnings` 消费（多文件 `run` 永不报 W1001/W1002/W1003）（**2026-10-19 用户裁决「做」**——原 2026-10-15 裁决丙按 B1（02 §运行时行为表）缓议，前置 4.9 已于 2026-10-17 收口；**超集同获授权**：W1004 / W1005 / W1006 一并进入 `run`，理由 = 一份判据一个通道，只放 W1001/W1002/W1003 等于再造第二份过滤判决；**2026-10-21 收口登记已完成**：D 类（对外可观察）——四路径（单文件 run / 多文件 run / 单文件 check / 多文件 check）的 W 码集合恒为 {W1003@1, W1001@3, W1002@5}；语料基线 348 条中 46 条变化，新增 W 码 118 条（W1001 34 / W1002 12 / W1003 13 / W1004 42 / W1005 17）**逐条归因到真实未使用定义、0 条未定位**，纯增量（0 条删除、0 处 exit / stdout / check 变化）；**对外契约面变更**：`orchestrator::compile_project` 返回值由 `Result<ModuleIR, _>` 改为 `Result<(ModuleIR, FileDiagnostics), _>`（仓内消费者 2 个 + 2 个测试直驱点同步适配） | `src/frontend/module/orchestrator.rs:126`（`compile_project`）、`src/driver/arms.rs:480`（`dead_code_analysis` 分派）、`src/driver/program.rs:66`（`MULTI_FILE_STAGES`） | 4.9（2026-10-17 已完成） | C2：多文件与单文件诊断集归一 |
| | 3.4.7 LSP 非项目内路径接入统一消费端（**2026-10-09 已完成**，随 4.2.6 落地 bf582555：手工 `check_module_collect_all` 直调序列删除，统一走 `Program{SingleFile, CollectAll}`——proof/warnings/死代码分析全部接入） | `lsp/handlers/diagnostics.rs:161-206` | 4.2.6 | LSP 与 CLI 对同一文件同诊断集 |
| | 3.4.8 多文件单态化臂（**已实证缺陷**，2026-10-10 升级表述：病态泛型递归 `f(x)=f([x])` 在无 mono 的多文件路径下编译器进程爆栈 0xc00000fd——mono 深度闸是唯一编译期防线；「类型擦除兜底」假设经 RFC-033 反射裁决作废——`^^List(Int)` 需要实例化的真实身份。范围（2026-10-10 用户裁决）：①typecheck 对限定调用（`lib.f(x)`，FieldAccess 形态）产 instantiation_request，generic_id 用限定名（现只认裸 Var，expressions.rs:2455）；②MULTI_FILE 阶段表加 Monomorphization 于 Linking **之后**（消费 merged_ir，聚合全单元请求，containing_fn 限定化；单文件阶段表不动）；③CHECK 加 Linking（纯合并——E3020 入口校验留 RoleClassification 防双报）+ Monomorphization（纯检查不消费，同 IrGeneration 的 Check 形态）——**撤回 4.2.5 的 mono 排除**（02 §3 注记 #14）；④资源保护随全程序 BFS 覆盖跨单元互递归（**① 2026-10-10 完成**，a9438007：限定名实例化收集 + mono 恢复闸落地，实证并修复删除/改写粒度失配、恢复段顺序不确定、非改写面形态误查映射键三个不对称缺陷，机制与提交勘记见 02 §3 注记 #15；②③④ 随 R2–R4 推进。**R2 范围补遗（2026-10-10 裁决甲）**：① 收口——用户模块导出随 `Export.type_params` 携带 `generic_fn_type_params_snapshot()`（与 std `yx_sources.rs:62/108` 同构），签名反推降级为兜底；② 验收加一条——deferred 桶必须排水或响亮报错，非泛型容器下的符号请求不得静默不特化（单文件路径是否有同类桶待核实）。**2026-10-11 R2 完成**：① 收口（D2 裁甲）与 ② 随 375789d2、d922a551 落地；三轮独立验证（t3/t6/t11）与 round-2 复审（t7）全 pass——其中 t6 抓到具名导入 E3018 误伤合法程序的回归、由 t10 以「别名通道与 ir_gen 同源」修复；机制与回归勘记见 02 §3 注记 #17；③④ 待 R3/R4） | `expressions.rs:2455`、`arms.rs:838`、`program.rs:60-92` | 4.2.8 | C2：跨文件泛型矩阵与单文件等价；病态递归多文件 → 编译期 E3005（不再爆栈）；单文件 IR 快照零漂移 |
| | 3.4.9 `interface_impl_registry` 跨阶段消费者裁决（**2026-10-13 登记**：该字段**恒非空**——由 typecheck 预置表（std/接口登记）灌入；声明消费者「运算符查询/约束求解/LSP」中前两者在 typecheck 树内同相消费，跨阶段「镜像供 middle/LSP」**从未落地**（`module/mod.rs:92` 仅文档注释提及）⇒ 删该跨阶段声明 或 建镜像） | `types.rs:49` | 4.3.1 | 义务账本红名单该条清零；`check-obligations.py`（9.2）不再报该字段；**2026-10-13 用户裁决（甲）：加框并入 4.3 团队 + 删「跨阶段镜像」声明、保留树内在相消费、红名单归零**（任务 t9；**2026-10-13 已完成**：义务 14→13、红名单 1→0、新增非义务登记 8 条 + 反向 containment「字段集 = 义务 ∪ 非义务登记」，门禁 PASS 13·13·0·8 + 七夹具 selftest；t11 独立验证 PASS、t12 复审 PASS） |
| 3.5 高阶类型参数调用路径补全（**2026-10-10 登记，用户裁决甲**）：RFC-010（`010-unified-type-syntax.md:210-220`）的元类型宇宙层级 + `MetaLevel` + `Type::MetaType` + parser 齐备，**声明侧活**（探针绿）、**两条调用路径响亮失败**（推断传类型实参 E1002 / 显式给类型实参 E1010）——属「已接受特性未实现」，不是静默错误 | 3.5.1 合一器在**头部位置**匹配构造器变量（现为位置不敏感变量匹配）（**2026-10-21 收口登记已完成**：判据收敛为单一实现 `is_ctor_reference`（`pub(super)` 自由函数，全仓 1 定义 3 消费），头部改名由共享实现 `substitute_ctor_heads` 承担（两处调用方各自遍历 TypeRef / 变量表后交同一实现，两处 head_map 构造判据不变）；复审轮 1 的 F1/F2/F3 三条 findings 已真修，轮 2 复审 pass） | `src/frontend/core/typecheck/inference/expressions.rs:6307`（`is_ctor_reference`）、`src/frontend/core/types/substitute.rs:106`（`substitute_ctor_heads`） | — | 红判据先红后绿：推断传类型实参路径转钉测试 |
| | 3.5.2 显式类型实参调用路径打通（E1010 消失）（**2026-10-21 收口登记已完成**：显式类型实参接受类型构造器，红判据先存在且确实红；转钉测试随本项入库） | `src/frontend/core/typecheck/inference/expressions.rs`（显式类型实参分支） | 3.5.1 | 红判据先红后绿：显式类型实参路径转钉测试 |
| | 3.5.3 mono 对构造器变量做替换（**2026-10-21 收口登记已完成**：符号化类型实参判据按容器泛型参数名收窄 + 构造器变量进值级签名与单态化产物；语料与 IR 快照零漂移） | `src/middle/passes/mono/mod.rs`、`src/middle/core/ir_gen.rs` | 3.5.1 | 产物中构造器变量被替换为真实构造器；与 3.4.8 R2 的全程序 pass 无冲突 |
| 3.6 函数类型值调用路径（闭包变量按名调用）（**2026-10-10 登记，用户裁决甲**）：局部函数类型值按名调用被降格为按名 Call，运行期 `E6006 Native function not found: f`（位置 `__yx_module_init`）；非泛型亦复现，与 3.4.8 无关，形态归 #416 家族 | 3.6.1 定位并修复函数类型值的调用降格（**2026-10-21 收口登记已完成**：局部函数类型值按名调用改走间接调用，IR 由 `LoadConst String` + `CallNative` 变为 `LoadGlobal` + `CallDyn`；判据单一源 = `is_static_fn_name`（全仓 1 定义 1 调用），与变量求值路径的名字解析优先级对齐，未新增第二份解析表） | `src/middle/core/ir_gen.rs:4948`（`is_static_fn_name`） | — | **语义裁 (i)（2026-10-10 用户裁决）**：函数类型值按间接调用运行——判据为「该程序正常运行（退出码 0、结果正确）」，先红后绿；依据：`CallDyn` 机制已在（`ir_gen.rs:7945` 闭包表达式路径），断的是 `Var` callee 被降格为按名 `Call`，typecheck 本就接受该程序；**附注（t3 F3，2026-10-11）**：多文件 HOF 闭包实参报 E8001（非泛型即复现，与 mono 无关），归本家族后续收口 |
| | 3.6.2 跨文件导入的函数类型值按名调用（`use lib.{f}` 原报 E6006）——**2026-10-21 收口登记已完成**：把 `use` 别名臂纳入「名字是否解析为值」，按名调用改走间接调用；判据单一源 = `is_static_fn_name` 内加一条臂（未复制），新增唯一实现 `use_alias_value_slot` 承担「use 别名 → 限定名 → 全局槽位」两表联查（消费点 2 处：调用形态判据 + 变量求值路径）；红态基线：父提交上 `use lib.{f}` 与 `use lib.{f as g}` 均报 E6006 `lib.f`，修复后 exit 0 且值正确；转钉测试 3 条（含「导入的函数定义仍静态调用」回归防线） | `src/middle/core/ir_gen.rs:845`（`use_alias_value_slot`）、`:4948`（`is_static_fn_name`） | 3.6.1 | 先红后绿：跨文件导入的函数值按名调用由 E6006 转 exit 0 且值正确；对照形态（值位置调用 / 导入的函数定义）行为不变；**归属阶段：P3；是否阻塞主线：否** |
| | 3.6.3 嵌套 lambda 捕获函数类型参数后按名调用给 `void`（静默错值，D59 A/B 类）——**2026-10-21 收口登记已完成**：**判据改判**——不是「让它跑对」，而是「嵌套函数体引用外层帧值 → 编译期 E1001」（按 RFC-009 §2.3 + SPEC §12.3 该形态应编译期被拒）；修复前为「不报错、算错值（void）」的静默通道，修复后与既有未知变量路径同码同文案；赋值路径由 E2010 兜住（否则修复自身会新开一条静默通道）；判据单一源 = 捕获屏障（`enter_capture_barrier`），编译期名（类型参数）不挡；E1001 仍只在 typecheck 发出、全仓注册唯一 | `src/frontend/core/typecheck/inference/scope.rs:339`（`enter_capture_barrier`）、`:565`（`get_var_info_unbarriered`）、`src/util/diagnostic/codes/e1xxx.rs:7`（E1001 注册） | 3.6.1 | 先红后绿：嵌套函数体引用外层帧值由静默错值改为编译期 E1001（exit≠0）且定位到引用处；不误伤（静态函数名 / 全局槽位 / use 导入 / 表达式 lambda 捕获本函数参数 / 柯里化 / 无捕获六类形态行为不变）；**归属阶段：P3；是否阻塞主线：否** |
| | 3.6.4 `use lib` 整体导入后的限定名调用 `lib.f(...)`（走 FieldAccess callee 分支，原报 E6006）——**2026-10-21 收口登记已完成**：命名空间分支解出限定名后先判全局槽位布局 → 改走间接调用；限定名解析与槽位查询复用 `resolve_namespace_data_access`（与数据访问同源），未新增解析表；红态基线：父提交上 `use lib` + `lib.f(3)` 报 E6006 `lib.f`，修复后 exit 0；转钉测试 2 条（四形态行为 + 产物 `CallDyn` / 无静态 `lib.f`） | `src/middle/core/ir_gen.rs:482`（`resolve_namespace_data_access`） | 3.6.2 | 先红后绿：整体导入后的限定名调用由 E6006 转 exit 0；不回归五类形态 + 对照三断言全过；**归属阶段：P3；是否阻塞主线：否** |
| 3.7 多文件 init 段调用名限定（**2026-10-11 登记，用户裁决乙′**）：`qualify_names`（`ir_gen.rs:1166`）只遍历 `ir.functions` 重写调用名，`ir.init` 从不被访问——多文件下函数定义被限定（`main.f`）而模块顶层调用名仍是裸名 → 运行期 `E6006`（t3 用**非泛型**函数复现，与泛型无关）；既是既有正确性漏洞，也是 mono 侧三级名字解析（含全程序唯一后缀启发式）的成因 | 3.7.1 `qualify_names` 覆盖 `ir.init`：同一 rename/aliases 表、同一 `rewrite_call_names`（含方法前缀匹配）；单文件 `module_key=None` 行为不变 | `middle/core/ir_gen.rs:1210-1219` | — | 红判据先红后绿：非泛型顶层调用 + 泛型顶层调用 + 同名歧义三形态；全量门禁 + 基线/快照核对 |
| | 3.7.2 聚合侧用单元键把同单元请求名限定化（`aggregate_instantiation_requests` 已持 `units[i].key`），使请求名与 IR 调用指令同源 | `driver/helpers.rs:228` | 3.7.1 | 请求名与 IR 字符串逐字一致（调用点改写三元键不落空）；跨单元泛型矩阵与单文件等价 |
| | 3.7.3 删除 mono 的「全程序唯一后缀」启发式与未命中静默 `continue`，改确定性解析 + 响亮诊断 | `middle/passes/mono/mod.rs:154` / `:258` | 3.7.2 | 无静默不特化：未命中/歧义必产诊断（码 + 文本）；t4 findings R1–R4 同批修 |
| | 3.7.4 init 段 `DefId` 与函数体同口径（**2026-10-11 登记，实证为「一致性空白」非活体缺陷**：codegen 的名字回退可解析、四类 init 调用实测通过；残留风险是「函数体走 def、init 靠名字」的口径分叉） | `middle/core/ir_gen.rs:1087`（assign_defs） | 3.7.1 | `assign_defs` 覆盖 `ir.init` + 208 个 IR 快照再生（逐行 `(def=_)`→`(def=dN)`、无其它行，人工 diff 核对）+ t9 探针断言按新口径翻转（**2026-10-11 完成** 38bd4b9e：assign_defs 抽出 resolve_static_targets 单点供函数体与 init 共用，208 快照仅 309 对 def 行变化、other-lines 0；red/绿两态 codegen 翻译与执行值逐项一致） |
| 3.8 native 对用户函数名的遮蔽（**2026-10-11 登记**）：单文件里用户泛型名 `read` 被 native `read` 遮蔽，实测报 `read expects 2 arguments (file: File, n: Int)`；实现者以改名 `pick` 绕开，当前**无钉测试** | 3.8.1 定名解析优先级（用户定义应遮蔽 native/导入？）并配红判据 | `frontend/module`、native 注册面 | — | 先定语义（用户定义优先 或 显式诊断），再先红后绿；**2026-10-18 补登审计证据**：独立复现同形态（`check` 0 error / `run` 报 `E6007`）并给出两个方案与推荐（保持 native 优先 + 仅对「同名且被裸名调用」报诊断）；issue [#477](https://github.com/ChenXu233/YaoXiang/issues/477) |
| 3.9 运行期栈守卫不可达（E6004）（**2026-10-11 登记**）：解释器 `push_frame` 的 `max_stack_depth` 默认 1024（`executor.rs:27`/`backends/mod.rs:426`），但 Rust 层递归先爆栈——实测 `mono.max_depth=5000` 时编译期拦截被换成进程 `0xC00000FD`；且 E6004 的 `limit` 参数恒传 0（`util/diagnostic/mod.rs:204`），真触发时用户看到「递归深度超出限制 0」。**与泛型无关**：凡深递归程序同崩 | 3.9.1 守卫可达性：超限必然走 E6004 而非崩进程；`limit` 带真实上限 | `backends/interpreter/executor/executor.rs:318`、`backends/mod.rs:426`、`util/diagnostic/mod.rs:204` | — | 先红后绿：构造必然超限的程序，证明现在崩、修后报 E6004；良构深递归程序的对外行为不变。**登记待排期，不并入 3.4.8 团队**。****2026-10-12 完成**（`yx-39-stackguard`，提交 `809b9b936`）：蹦床化 + limit 单点化 + 回显截断（原 3.11.2 并入）；红态 `0xC00000FD` → 绿态 `E6004(limit 1024)`，边界 1023 帧通过 / 1024 帧报错** |
| 3.10 编译期无闸原生递归路径（**2026-10-12 登记，issue #447**）：`mono.max_depth`（`frontend/config.rs:15/33`，默认 100）抬高后，病态泛型递归（`f(x)=f([x])`）不再被闸门拦下，而是让编译器进程爆栈 `0xC00000FD`（Windows 实测）；**深度闸本身是队列驱动**（`mono/mod.rs:222 process_queue`，判定在 `:232`，循环内无原生递归）→ 崩溃在下游**无闸的原生递归路径**上，位置待定位。同类核实：E4012 常量求值闸（`types/eval/const_eval.rs:539`，默认 256）确为原生递归承载，但 `set_max_depth`（`:273`）全仓零调用点 → 当前不可配，暴露面低，只登记结论 | 3.10.1 定位崩溃点，并让「闸门放开后」的路径不再依赖原生栈：超限必然产出既定诊断（E3005）而非崩进程；默认配置行为不变 | `middle/passes/mono/mod.rs:232`、`frontend/config.rs:15/33`、`frontend/core/types/eval/const_eval.rs:539` | 3.9（同族不同层） | 先红后绿：基线提交上（抬高 `max_depth` + 病态递归）实测崩溃 → 修后产出 E3005；默认 `max_depth=100` 行为不变、IR 快照零漂移、语料零回归。**用户 2026-10-12 裁决：开 issue + 登记 WBS + 顺手修（并入 yx-39-stackguard）**。****2026-10-12 完成**（提交 `fix(monomorphize)`）：定位 + 四处工作列表化；`max_depth=5000` 由崩溃 → `E3005（5000，当前 5001）`；边界矩阵 `{100,150,1000,5000}` 独立验证通过** |
| 3.11 错误码保真与深栈回显（**2026-10-12 发现并登记**，源自 3.9.1 实现/验证过程）：① Standard 运行时模式（`--runtime standard`）下任务内部错误经 `backends/interpreter/executor/executor.rs:578-581` 的 `ExecutorError::runtime(format!("{e}"), stack)` 与 `:610-613` 的 `Err(sv(RuntimeValue::String(format!("{e}"))))` 字符串化 → **一律落 E6007**（机制对错误一视同仁，未逐码实测）；锚点是 `executor.rs:578`/`:610`、`backends/runtime/engine.rs:82 TaskOutcome` ② E6004 首次可达后回显 1024 帧回溯（`target/wbs39-evidence/green-fixed.txt` stderr 1035 行） | 3.11.1 任务结果通道承载结构化错误（`TaskResult = Result<SyncValue, SyncValue>` 错误侧结构化、`TaskOutcome::Err` 同步）：任务内各码在 Standard 模式保真，不再一律 E6007；3.11.2 深栈回溯截断策略（保留首尾 + 计数省略） | `backends/interpreter/executor/executor.rs:578`、`:610`、`backends/runtime/engine.rs:82` | 3.9（同源发现） | 3.11.1：任务内 E6001/E6003/E6004/E6008 各码保真（先红后绿）；3.11.2：超深回溯回显受控且带省略计数。**用户 2026-10-12 裁决甲：并入 yx-39-stackguard 本轮（issue #448）**。****2026-10-12 完成 3.11.1**（提交 `fix(backends)`）：任务边界码保真，Standard 模式四类错误码 E6001/E6003/E6004/E6008 与 Embedded 消息逐字一致；② 并入 3.9.1** |
| 3.12 运行时模式间的执行器配置传播（**2026-10-12 发现并登记**，源自 3.11.1 实现过程）：Standard 模式下任务解释器按默认 `ExecutorConfig` 建（`executor.rs:201 for_task`，调用点 `:563`），父配置不传播 ⇒ 任务内 limit 恒 1024、宿主自定义上限被静默忽略 | 3.12.1 让任务解释器继承父执行器配置（或在文档与诊断里显式声明「不继承」并给理由）；默认配置行为不变 | `backends/interpreter/executor/executor.rs:201`、`:563` | 3.11 | 先红后绿：宿主配置 `max_stack_depth=64` 时任务内超限报 `E6004 limit=64`（现恒为 1024）；默认配置零回归。**用户 2026-10-12 裁决乙：另起队伍（本轮不并入）**；D 类变更须在该队内先获裁决 |
| 3.13 ir_gen 方法绑定 panic（A 类崩溃） | 3.13.1 `Temp.<name> = <表达式>[<下标>]` 且无签名标注 ⇒ `ir_gen.rs:1661-1672` 的重绑定识别不命中（基底是 `FieldAccess` 而非 `Var`）⇒ 落到 `// MethodBind` 分支（`:1674-1682`）对 `type_annotation` 执行 `unwrap()` ⇒ **进程 panic、退出码 101**。正确行为 = **不 panic**，产出明确诊断（或在能唯一确定签名时正常编译） | `middle/core/ir_gen.rs:1678` | — | 先红后绿：以最小复现为红判据（断言「不 panic 且产出诊断」）；**不得**用 `#[should_panic]` 把崩溃钉成期望行为；issue #451 |
| 3.14 方法级重载决议纳入命名实参 | 3.14.1 决议候选匹配纳入命名实参（按形参名绑定到对应位置后与位置实参一起做类型匹配），消除默认绑定回落；判据覆盖纯命名 / 混合 / 命名与位置冲突 / 命名重复绑定四形态 | `frontend/core/typecheck/inference/expressions.rs`（决议段）、`middle/core/ir_gen.rs`（消费点） | 4.5.1（键已 PlanId 化、消费必达结算在位） | 先红后绿四形态各一判据；语料差分须人工 review（E1107 / E1002 可能新增或消失）；issue #452 |
| | 3.14.2 多候选重载下命名实参只按**首个声明候选**的形参名匹配（`candidate_arity_groups` 每个元数只保留首候选名字表）⇒ 合法调用被 E1014 误拒、且类型错误被归到另一候选；与 3.14.1 的自述目标相反。D59 **C 类**；先红后绿：补「命名实参命中非首候选」绿态 + 「名字不存在于任何候选」红态 | `frontend/core/typecheck/inference/expressions.rs` | 3.14.1 | issue [#466](https://github.com/ChenXu233/YaoXiang/issues/466) |
| 3.15 `typecheck::infer_expression` 死 API（复审 F3） | 3.15.1 `typecheck/mod.rs:138 infer_expression` 全仓**零调用点**，且不回流 `overload_resolutions`（无产出/消费台账）⇒ 接线即成 D20 同类缺口 | `frontend/core/typecheck/mod.rs:138` | — | 处置二选一：**删除**（按 P10 S2 死代码）或**接线前先补契约**；无行为面故不适用先红后绿，但删除须有零调用点直证（`git grep`）并记录被删逻辑的去向；issue #453 |
| 3.16 stdlib 原生导出命名实参一致性（C 类静默错值） | 3.16.1 原生导出调用路径的命名实参绑定：位置实参按声明顺序占位、命名实参按形参名绑定；未知形参名 / 缺参 / 同一形参重复命名 / 位置与命名争槽四类必须**编译期报错**，不得静默丢弃实参或静默接受未知名 | `middle/core/ir_gen.rs`（原生导出调用分支，约 `:7969`/`:8096`/`:8197`）、`frontend/core/typecheck/inference/expressions.rs`（决议段 / `bind_named_args`） | 3.14.1（方法级同一套判据） | 先红后绿四形态各一判据（形参名正确 / 形参名错误 / 纯命名 / 混合），且须证「原先成功程序」对外表现不变（语料 342 行逐行等价）；issue #454 |
| 3.18 约束泛型运算符派发跨模块类型身份（C 类） | 3.18.1 MonoType 用户类型携带定义模块（或在 mono 派发处接入类型身份表），使 `T.add` 派发改写能还原 `lib.Vec3.add` 形态限定名；先红后绿：`lib.yx` 定义 `Vec3` + Add 实现、`main.yx` `use lib.{Vec3}` 跨模块 `sum(a, b)` 运行期不再 E6006 | `frontend/core/types/mono.rs`（类型身份）、`middle/passes/mono/mod.rs`（`qualify_deferred_method` 已知缺口注记） | P6 类型身份收敛（或裁决降级为 3.5 家族） | 先红后绿：跨模块约束派发探针由 E6006 转 exit 0；同模块/FFI 原生回归零影响；**归属阶段：P3（前置待 P6）；是否阻塞主线：否** |
| 3.17 手注 FFI 命名实参静默丢弃（C 类） | 3.17.1 `add_native_signature` 与 stdlib 导出路径同源——改调 `typecheck::signature::export_signature_param_info` 唯一出口（`signature.rs:236`）拿形参名与必需下限，**不得出现第二份绑定判据实现**（#454 复审 F1 教训） | `frontend/core/typecheck/environment.rs:789`、`frontend/core/typecheck/signature.rs:236` | 3.16.1（同一套绑定判据） | 先红后绿：手注 FFI 注册的原生函数命名实参与位置实参同判——形参名错误 / 缺参 / 位置与命名争槽 / 命名后接位置四形态各一判据（E1111 / E1014）；补「同类路径未修分支」探针（AGENTS.md 第 10 条⑥）；issue #456；**2026-10-18 补登审计证据**：第二份实参绑定判据实测在位——`middle/core/ir_gen.rs` 的 `build_arg_slots`（约 70 行）与 `typecheck/inference/arg_binding.rs` 的 `bind_args_to_params` 各在生产路径上；同一语义类实测两个码：`add(a = 1, a = 2)` → E1015，`list.push(item = 9, item = 8, list = v)` → E1010。合并后约 −70 行 |
| 3.19 精化谓词的 SMT 排序翻译崩溃（**2026-10-18 登记**，来源 = 4.4.2 施工期探针；D59 **A/B 类**（崩溃）；**定性 = 预存在**——独立验证 t5 在 `b5e68b3c9` 与冻结 revision `d07993157` 上各跑一次**同一探针、同一输入**：stdout 逐字节一致（`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`）、均 exit 1（Z3 默认错误处理器 abort 进程）；静态归因一致：4.4.2 提交对 `translate.rs` 0 行改动、对 `z3_backend.rs` 仅 `cache_stats` 迁移） | 3.19.1 症状：`NonNegative: (x: Int) -> Type = { x >= 0 }`、`AtMost: (n: Int, x: Int) -> Type = { x <= n }` 与 `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }` 单次编译即触发 Z3 `Sort mismatch` 并 **abort 整个进程**（探针三行源码：`NonNegative: (x: Int) -> Type = { x >= 0 }`、`AtMost: (n: Int, x: Int) -> Type = { x <= n }`、`f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`；留档见 `target/yx-scratch/close-44/evidence-summary.md`）。成因指向 `proof/smt/translate.rs` 的排序推断（`infer_var_sorts`，`translate.rs:118`）——`z3_backend.rs` 的 `expr_to_z3`（`z3_backend.rs:261`）把 `SMTExpr::Atom` 一律建成 Int 常量，与推断出的 Bool 排序不一致时即出现「not(Int)」 | `frontend/core/typecheck/proof/smt/translate.rs`（排序推断/翻译） | — | 验收行 = 先红后绿：该形状不再 abort（保守降级或诊断），语料与 IR 快照零漂移；**归属阶段：P3；是否阻塞主线：否** |
| 3.20 死代码分析的既有误报（**2026-10-21 登记**；来源 = WBS 3.4.6 施工期暴露的两个**既有** W 码误报——用户 2026-10-19 裁决丙：文档示例改为不触发误报的等价写法，误报本身登记待修；**归属阶段：P3；是否阻塞主线：否**） | 3.20.1 W1001「未使用的私有函数」不跟踪**闭包内的引用**：`docs/src/reference/stdlib/test.md` 的 `t_add` 形态（顶层测试函数只在 `test.suite` 的闭包 `("add", () => t_add())` 内被引用，却报 W1001 未使用）；该形态在 3.4.6 之前的基线上同样复现，**非 3.4.6 引入**；示例已改为「主函数体内直接调用一次」以绕开误报 | `src/frontend/core/typecheck/passes/dead_code.rs:815`（`find_unused_defs`）、`src/util/diagnostic/codes/w1xxx.rs:7`（W1001 注册） | — | 闭包内被引用的定义不再报 W1001（且不引入新的未使用漏报）；**回滚义务**：本项修好后必须把 `docs/src/reference/stdlib/test.md` 中被改写以绕开误报的示例**恢复为自然写法**并再生语料基线（基线会再动一次）；**归属阶段：P3；是否阻塞主线：否** |
| | 3.20.2 W1003「未使用的导入」把**变体解构必需导入**判成未使用：`docs/src/reference/stdlib/weak.md` 的 `use std.option` 形态（变体集在场是 `match some(v)` / `none()` 的前置条件，删掉该导入即报 E1002）；该形态在 3.4.6 之前的基线上同样复现，**非 3.4.6 引入**；示例已改为「额外构造一次 `Option(Int).some(7)`」以绕开误报 | `src/frontend/core/typecheck/checker.rs:3279`（`collect_unused_import_warnings`）、`src/util/diagnostic/codes/w1xxx.rs:11`（W1003 注册） | — | 变体解构所需的导入不再报 W1003（今日会因删该导入而 E1002 的程序改判后仍能编译）；**回滚义务**：本项修好后必须把 `docs/src/reference/stdlib/weak.md` 中被改写以绕开误报的示例**恢复为自然写法**并再生语料基线（基线会再动一次）；**归属阶段：P3；是否阻塞主线：否** |
| 3.21 发版前审计（v0.8.4）补登缺口（**2026-10-18 登记**；来源 = `v0.8.4..HEAD` 发版前审计；**2026-10-21 迁号**：3.20 号让给「死代码分析的既有误报」；**归属阶段：P3；是否阻塞主线：否**） | 3.21.1 解释器**一元取负**对 `i64::MIN` 溢出直接 panic（`backends/interpreter/executor/ops/arith.rs` 的 `RuntimeValue::Int(-n)` 未走 checked 通道；二元溢出已正确报 E6007）；D59 **C 类**（崩溃） | `backends/interpreter/executor/ops/arith.rs:38` | — | issue [#468](https://github.com/ChenXu233/YaoXiang/issues/468) |
| | 3.21.2 诊断外显保真三处：中文 help 就地硬编（`typecheck/inference/arg_binding.rs` / `middle/core/ir_gen.rs`）· E4018 输出**空** `Counterexample:` 段且 `-3` 渲染成 `-(3)` · 致命调用错误后级联 `E1001`；D59 **D 类**（对外文本面，落地前需确认） | `frontend/core/typecheck/inference/arg_binding.rs`、`middle/core/ir_gen.rs`、`util/diagnostic/codes/e4xxx.rs` | — | issue [#469](https://github.com/ChenXu233/YaoXiang/issues/469) |
| | 3.21.3 `mono.max_depth` 无法从 CLI/`yaoxiang.toml` 配置——3.10.1 的抬高闸门档在 `run` 上不可复现 | `src/main.rs`、`frontend/config.rs` | — | issue [#471](https://github.com/ChenXu233/YaoXiang/issues/471) |
| | 3.21.4 生成物与事实不符：警告码页断言「W 族不会阻止编译」，与 W1090 实际 **error 级**矛盾（生成器写死散文）。修后须**重跑** `scripts/docs/gen-error-code-docs.py` 再生，**禁止手改生成物** | `scripts/docs/gen-error-code-docs.py`、`docs/src/reference/` | — | issue [#470](https://github.com/ChenXu233/YaoXiang/issues/470) |

**验收**：漏洞判据转绿，且**故意撤掉修复必须重新变红**。**风险**：多文件路径新增 E4018 是破坏性变更。

**3.4 验收**：3.4.1 / 3.4.2 必须先有红判据（禁止先改代码）；3.4.3 为人工 review；3.4.4–3.4.8 以 C2/C3 与 P9 的 9.2 门禁为准。**风险**：3.4.1 新增诊断属行为变更，须同步语料差分基线与 CHANGELOG。

> **止血通道（2026-10-05 裁决）**：P3 不必等 P2 全部完成。3.1/3.2 的唯一硬前置是 **2.3.2（多文件语料层）+ 2.4.1/2.4.3（两个红判据）**；2.1/2.2/2.3.1/2.3.3（IR 校验器、快照、单文件差分、性能基线）可与 P3 并行推进。正确性漏洞（`Sorted(3)` 静默通过）的止血不应被快照基础设施建设阻塞。3.3（panic 转诊断）无前置，可随时落地。

### 外部轨道 RFC-029g：删除 `pub` 与自动绑定（外部轨道 / #399 —— 排序约束 D56）

来源：[RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)（2026-10-02 已接受、实现未开工）、[#399](https://github.com/ChenXu233/YaoXiang/issues/399)。**本轨道不是重构自身的设计项，是被排序约束强制前置的规格符合性删除**；与 P10 S5「`pub` 项降可见性」（Rust 的 `pub(crate)` 收窄）是两件事。

**为什么必须整体先于 P4**：029g 的删除面与四个重构阶段的重写面重合，反序 = 在新文法 / 新 checker 布局 / 新 SSA IR 上重做同一批删除与基线更新。

| 029g 删除面 | 文件 | 被哪个阶段重写 |
| --- | --- | --- |
| `KwPub` 与语法分支 | `lexer/{tokens,state,mod}.rs`；`parser/parser_state.rs`、`statements/{declarations,imports,functions,bindings}.rs` | **P8**（8.2 词法收敛 / 8.3 建文法 / 8.6 删 Pratt / 8.7 拆 `parse_assign_after_target`） |
| AST `is_pub` ×2（`Assign` / `TypeDefinition`） | `parser/ast.rs:247/270` | **P6 6.5 AST 死变体** + P8 + D1 目录改名（`parser/ast.rs` → 顶层 `src/ast/`） |
| `auto_bind_to_type` 与 `collect_exports` 的 pub 分支 | `typecheck/checker.rs:2150/3802/3836`、`environment.rs:284` | **P5**（`include!`→真 `mod` 与 checker 拆分是**纯搬迁**，pub 分支会被原样搬进新文件） |
| 死码豁免 `exempt_pub` / `is_exported` 与角色分派 | `typecheck/passes/dead_code.rs`、`module/orchestrator.rs:389-407`（`check_project` 体内） | **P4 4.2.2**（`check_project` 瘦身） |
| `semantic_tokens` 的 `Public` modifier | `typecheck/checker/semantic_tokens.rs:450/498` | **P4 4.2.6** + P5 + 4.6（D55） |
| `ir_gen` 的 `is_pub` 忽略绑定 | `middle/core/ir_gen.rs:1378` | **P7**（`ir_gen.rs` SSA 化） |
| `src/std/` 31 处 `pub` | `std/list.yx`(25) / `json.yx`(4) / `option.yx`(1) / `result.yx`(1) | 无阶段直接改，但 **P6 硬验收要求 `git diff --stat tests/ src/std/` 为空**——只能在 P6 之前落 |

**附带收益**：029g 删除 `env.exports` 死表（生产端灌表、全仓零消费）与 `is_exported`/`is_visible` 两个访问器 → P4 的 4.3 义务账本少一个「产出但无人消费」字段。

- [x] **029g.1 词法与 parser 分支**（`KwPub` ×8；2026-10-08 完成）
- [x] **029g.2 AST `is_pub` ×2**（含生产构造点与测试 fixture 同步；2026-10-08 完成）
- [x] **029g.3 checker 侧**（`auto_bind_to_type`、`collect_exports` 的 pub 分支、`env.exports` 死表与两个访问器；2026-10-08 完成）
- [x] **029g.4 死码豁免收敛**（`exempt_pub` / `is_exported` 删除；`orchestrator` 角色分派同步；2026-10-08 完成）
- [x] **029g.5 外围回显**（formatter 的 pub 前缀、`semantic_tokens` 的 `Public`；2026-10-08 完成）
- [x] **029g.6 std 与诊断文案**（31 处 pub 前缀；4 条 W1xxx help 改 zh 源，译文走 bot；2026-10-08 完成）
- [x] **029g.7 语言参考收敛**（`syntax.md` 关键字表 18→17、`warning-codes.md`、`guide/language-overview.md`、`guide/modules.md`；2026-10-08 完成）

| 二级 | 三级 | 文件:行 | 验收 |
| --- | --- | --- | --- |
| 029g.1 词法与 parser | 029g.1.1 `KwPub` 变体与 `keyword_from_str` 条目删除 | `lexer/tokens.rs:83`、`state.rs:28`、`mod.rs:62` | 关键字表 18→17（`Kw*` 16→15，含 `tokens.rs:82` 计数注释） |
| | 029g.1.2 声明位的 pub 探测与 `pre_detected_pub` 删除 | `parser/statements/declarations.rs:672/675/731/747/880`、`functions.rs`、`bindings.rs` | `pub f = ...` 报解析错误（**红判据先行**） |
| | 029g.1.3 use 花括号条目内的 pub 跳过与语句前导 pub 删除 | `parser/statements/imports.rs:57`、`parser_state.rs:203` | `pub use m` 报解析错误（**红判据先行**） |
| 029g.2 AST | 029g.2.1 `Assign`/`TypeDefinition` 的 `is_pub` 删除 + 构造点同步 | `parser/ast.rs:247/270` 与各构造点 | 编译器内无 pub 残留（`grep -rn is_pub src/` 为空） |
| 029g.3 checker | 029g.3.1 `auto_bind_to_type` 删除（pub 是唯一调用理由） | `checker.rs:3802`、`environment.rs:284`、调用点 `checker.rs:2150` | 方法形式只剩显式组合（RFC-004），std 与语料零依赖 |
| | 029g.3.2 `collect_exports` 去 pub 分支 + `env.exports` 死表与 `is_exported`/`is_visible` 删除 | `checker.rs:3836`、`environment.rs:599/612` | 导出面唯一权威 = 模块注册表 `ModuleInfo.exports`；9.2 门禁少一字段 |
| 029g.4 死码豁免 | 029g.4.1 `exempt_pub` / `is_exported` 与豁免分支删除 | `dead_code.rs:23/44/83/90/174/180/191/195/591/629` | Lib/Script 未引用的顶层绑定报 W1001（**红判据先行**） |
| | 029g.4.2 `orchestrator` 角色分派同步 | `orchestrator.rs:389-407` | 单文件 / 多文件诊断集归一（C2），W 码不混入消费方 |
| 029g.5 外围回显 | 029g.5.1 formatter 的 pub 前缀 | `formatter/handlers/stmt.rs:81/117/146/151` | format 幂等（既有判据） |
| | 029g.5.2 `semantic_tokens` 的 `Public` modifier | `typecheck/checker/semantic_tokens.rs:450/498` | 语义着色不再产出 Public |
| 029g.6 std 与文案 | 029g.6.1 `src/std` 31 处 pub 前缀删除 | `std/{list,json,option,result}.yx` | 语料与 std 自检全绿；**须在 P6 前完成** |
| | 029g.6.2 诊断文案 | 4 条 W1xxx help | 不再宣称「pub 是对外接口」；与 `dead_code.rs:574` 的 Bin 角色行为对齐 |
| 029g.7 文档 | 029g.7.1 语言参考与指南 | `reference/language-spec/{modules,syntax,language-overview}.md`、`warning-code/warning-codes.md`、`guide/modules.md` | 规范不再出现与实现相反的「pub 仍可写」表述；`warning-codes.md:124`「pub 永不触发 W1001」一并修正 |

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
- [x] **4.2 入口合并**（9 个三级任务；4.2.1–4.2.9 全部完成 2026-10-09/10；九个工作提交均在史，dev 头 CI run 38372921921 七作业全绿覆盖其全部改动）
- [x] **4.3 义务账本**（2 个三级任务；4.3.1 / 4.3.2 均已完成 2026-10-13；含跨二级加框的 3.4.4 / 3.4.5 / 3.4.9 清账）
- [ ] **4.4 证明层与 wasm 收尾**（3 个三级任务，**2026-10-18 全部完成**——4.4.1 层序声明按实际调用点收口（t2 / t8）、4.4.2 SMT 求解器获取面统一为进程级共享单例（t1，按 D58）、4.4.3 降级路径加 warning（t3，新增码 W1082 经用户追认）；独立验证 t5 八项门禁全绿：`cargo test` 3128 passed / 0 failed、语料 348:348 逐行零差异、IR 快照零漂移、`check-obligations` 0、`check-docs-truth` 源码 156 / 文档 156、`cargo fmt --check` 0、`clippy --all --all-features --all-targets -- -D warnings` 0、错误码生成器重跑后树干净；三条变异经 `scratch.py trial` 全部机械可逆（还原后逐文件与 HEAD 一致、`git status` 干净）；复审轮 1 判 needs_revision（3 条 findings 均为文档行号锚失准，实质交付全 pass）→ 修复轮 → 轮 2 **PASS**、新 finding 0；证据 `target/yx-scratch/close-44/evidence-summary.md`；**归属阶段：P4；是否阻塞主线：是**——卡 P5 前置「P4 完成」，本批次收口（收口提交已推送）后解除）；**2026-10-18 重开**：新增三级 4.4.4（issue [#475](https://github.com/ChenXu233/YaoXiang/issues/475)）
- [x] **4.5 跨层契约 PlanId 化（D20）**（2 个三级任务；4.5.1 / 4.5.2 均已完成 2026-10-13，t1–t7 收口：实现 / 独立验证 / 复审各自独立，t5 的 F-1 / F-2 已按 §3.13 / §3.14 与本轮注记处置）
- [x] **4.6 LSP 语义数据管线统一（D55）**（2 个三级任务；4.6.1 / 4.6.2 均已完成 2026-10-16——4.6.2 为用户当日裁决的查库键容差（标识符起点），4.6.1 详情：诊断与语义数据同一次编译（`analyze_source_in_project` / `analysis.rs`），跨文件引用 `resolves_to` 命中真实定义（先红后绿：LSP e2e 修复前返回 None）；driver 级 6 钉 + LSP e2e 1 钉 + uri 规范化 4 钉；语料 342 行零漂移、IR 快照零漂移、check-obligations PASS（13·13·0·8）；**归属阶段：P4；是否阻塞主线：是**——卡 P5 前置，且硬阻塞 4.9.4）
- [ ] **4.9 引用池正确实现（RFC-029f 修正）**（7 个三级任务（2026-10-17：+1 = 4.9.7，登记 issue `#464`）。**2026-10-16 轮 1 收口（4.9.1 / 4.9.2 / 4.9.3，提交 `3863a9ff0`）**；**2026-10-17 轮 2 收口（4.9.4 / 4.9.5，提交 `e6cf1aeaa`；4.9.6 = `ddcbb12d0` + 修复轮 `b8380e6ab`，按用户 2026-10-17 裁决 A 收口；收口修复 `b940300ac`）**；4.9.7（跨单元方法调用的决议登记与接收者归因，issue `#464`）**2026-10-17 轮 3 完成（产出侧 `b8b3cc3f6` / 驱动侧 `d5c00235a`；注入面误报修复 `d5dd1a37b`、钉测试归位 `c2f887843`）**；#450 归属**已裁决（用户 2026-10-16 裁决 A）**——证据表明属「过载解析后名字未传回引用计数」类，仍卡在轮 1 已落地形态下复现；登记为本二级任务下的独立三级任务 `4.9.6` 并并入轮 2（不并入 4.9.4 / 4.9.5 的验收面）。**轮 1 详情**：(a) 4.9.1 池的根改为 `collect_package_refs`（`helpers.rs:481`，按 `ExplicitSurfaces` 的 `[[bin]]`/`[lib]`/`[exports]`/`[run].main` 根调 `discover_with_used`，不扫目录找入口；全根退化时退路 `collect_compilation_refs` / `helpers.rs:454`）；(b) 4.9.2 粒度升级为 `HashMap<ModulePath, HashSet<String>>`，装配唯一入口 `collect_ref_pool`（`helpers.rs:429-449`），消费端在 `arms.rs:603-609` 的 `other_module_refs` 把「除本单元外」的全部模块条目并集后送 `DeadCodeAnalyzer`，模块自有名（顶层定义与局部绑定，`use` 导入名例外）不再豁免别的模块同名定义；(c) 4.9.3 `arms.rs:611-623` 的 `match us.role` 只对 Bin/Internal（`set_exempt_pub`）与 Lib（`set_cross_file_refs`）装配，Script/Test 走 `_ => {}` 默认臂（不接池，按文件内定义-使用图判定）。**用户 2026-10-16 合并裁决**：三条三级任务并为轮 1（理由 = 它们同属「池的来源与身份」一面，合并落地可避免 `collect_project_refs` 删后的中间态反复重写；4.9.4 / 4.9.5 分属消费细化与测试池分离，依赖 4.6.1 的 `use` 路径解析，强行同批会扩大闸口面）。**门禁（t1 落库 + t2 独立验证 PASS attempt `aab92fe0` + t3 复审 pass attempt `96452696`）**：fmt --check 0 / clippy -D warnings 0 / lib 2754 / integration 287 / yx_multifile_runner / 语料基线仅 +2 行（两条新夹具 behavior run，338a339,340，另 342 行逐字节不变）/ IR 快照 204 条零漂移 / check-obligations 13·13·0·8 + selftest / docs-truth 绿。**证据**：`target/yx-scratch/yx49/EVIDENCE.md`（量测命令前缀 `python scripts/dev/scratch.py use yx49 --rev 3863a9ff0 --by verify-refpool --task t2`；变异 8 补丁经 `scratch.py trial` 全部机械可逆，对照组不误伤）。）（**2026-10-13 登记 `#450`**：重载方法被调用且决议生效，仍被误报 `W1005 Unused method`——引用计数对重载调用失真；归属在本二级任务内待定，见 issue；**归属阶段：P4；是否阻塞主线：否**——**用户 2026-10-16 裁决 A：登记为本二级任务下的独立三级任务 `4.9.6`，并入轮 2 实现**，不并入 4.9.4 / 4.9.5 的验收面）；**2026-10-18 重开**：新增三级 4.9.8（issue [#465](https://github.com/ChenXu233/YaoXiang/issues/465)）
- [ ] **4.10 多文件语义缺口**（3 个三级任务；4.10.1/4.10.2 已完成；**4.10.3 已完成 2026-10-15**——standalone run 统一走 `compile_project`（4.2.5 裁决 A 的 run 半边）、`entry_function` 按 `is_bin_role` 门控、#413 提示改项目探针键控；顺路修统一暴露的三处缺口（显式类型实参预特化名限定、Script `mut` 槽位重复配槽、同模块约束派发限定名查表验证）；语料基线再生 12 行纯限定名改名，跨模块约束派发登记 3.18；**归属阶段：P4；是否阻塞主线：是**——卡 P5 前置）；**2026-10-18 重开**：新增三级 4.10.4 / 4.10.5（issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467) / [#476](https://github.com/ChenXu233/YaoXiang/issues/476)）

| 二级 | 三级 | 前置 | 验收 |
| --- | --- | --- | --- |
| 4.1 声明式阶段表 | 4.1.1 `stage.rs`（`Stage` 变体 + `Stage::ALL` + `StageScope`）（**2026-10-07 已完成**） | P0 | `test_program_stage_coverage` |
| | 4.1.2 `program.rs` / `unit.rs`（**2026-10-07 已完成**） | 4.1.1 | 同上 |
| | 4.1.3 `Pipeline::run` 改走 Driver（**2026-10-09 已完成**，b42d8920） | 4.1.2 | **单文件诊断集与退出码逐字节相同** |
| | 4.1.4 阶段覆盖断言（**2026-10-09 已完成**——`test_program_stage_coverage` 随 4.1.2 预置，勿双重登记） | 4.1.3 | C2 |
| | 4.1.5 全语料 zero-diff（**2026-10-09 已完成**，实测 330 语料） | 4.1.4 | 诊断/退出码/stdout 全 zero-diff（293 语料） |
| 4.2 入口合并 | 4.2.1 `compile_project` 瘦身为 Program 构造器（**2026-10-09 已完成**，be60e01f；A1/B1/C3 裁决登记 02 §1/§3 修订注记） | 4.1.5 | 四入口诊断集归一后相同 |
| | 4.2.2 `check_project`（**2026-10-09 已完成**，6ec330d2；RoleClassification 臂接线收齐 12 变体，顺序归一 + E3020 归属裁决登记 02 §1 注记 #4） | 4.2.1 | 同上 |
| | 4.2.3 `check_source_in_project`（**2026-10-09 已完成**，83a6dd34；LSP 降级按文件来源分流裁决——磁盘文件同 Check 方案 B、缓冲区保留残缺 AST，登记 02 §3 注记 #8；顺手修复 LSP 对无关磁盘文件硬中止的怪癖） | 4.2.2 | 同上 |
| | 4.2.4 `compile_embedded_module`（**2026-10-09 已完成**，e6fa163a；`Program::with_shared_registry` 注入共享注册表（#94 契约显式化），orchestrator 四入口全部迁入 Driver） | 4.2.3 | 同上 |
| | 4.2.5 删 `check_single_file`（**2026-10-09 已完成**，0ef6ae6b；standalone 统一走 `Program{Check}` + 裁决 A 相对导入解析 + CHECK 加 GlobalSlotAlloc/IrGeneration 两阶段（用户裁决——runner 门禁实证 IR 级错误只在 ir_gen 产生）+ 嵌入 std 注册表面覆盖修复；**mono 排除部分经 3.4.8 实证撤回**——E3005 是病态泛型递归的唯一编译期防线，「零语料依赖」实为语料无病态 fixture 的证据盲区（02 §3 注记 #14） | 4.2.1 | `check` 项目内外一致 |
| | 4.2.6 删 LSP 手工阶段序列（**2026-10-09 已完成**，bf582555；SingleFile+CollectAll 新形态——Parsing 收全量 + 残缺 AST 继续 typecheck（编辑器哲学延伸），Typecheck 分派 `check_module_collect_all`；LSP 补齐 proof/W 码/IR 级错误，登记 02 §3 注记 #11） | 4.2.1 | LSP 与 CLI 诊断集相同 |
| | 4.2.7 `Aggregation` 参数驱动二选一（**2026-10-10 已完成**，04343d2c；C5 逐行核实：两入口已收敛于 `check_module_inner`/`check_module_impl` 单布尔分叉，唯一分岔 `set_collect_all_errors`；collect_all 重复诊断修复——模块结果边界按 (code, span, message) 去重，63 个语料条目去重、run 列零漂移，登记 02 §3 注记 #12） | 4.2.1 | ~~**实施时逐行核实两函数内部差异**（冲突登记 C5 已裁决为必做核实项）~~已核实；~~**含 collect_all 重复诊断**~~已修复 |
| | 4.2.8 wasm 改走 `ProgramKind::WasmPlayground`（**2026-10-10 已完成**，06d26410；`compile_playground` 统一构造，错误文本逐字节对齐旧 `CompileError` 前缀；wasm32 目标 check/clippy 实证，登记 02 §3 注记 #13） | 4.2.1 | 见 C6——旧路径零残留引用（`test_compiler` 仅探活 `Compiler` 类型存在） |
| | 4.2.9 `driver/mod.rs` 拆分为 `state.rs`/`arms.rs`/`helpers.rs`（**2026-10-09 已完成**，c59fb751——1766 行超 P5 的 1500 行拆分阈值；零行为变化，内部可见性收紧为 `pub(in crate::driver)`） | 4.2.4 | C1：全量回归绿 + `crate::driver::*` 公开路径不变 |
| 4.3 义务账本 | 4.3.1 `obligations.rs` + `assert_drained()`（**2026-10-13 已完成**，1941cc47a——7 处交付标记 + 唯一结算点 `driver/mod.rs:110` + 审计面 + 红名单三条（3.4.4/3.4.5/3.4.9）+ W1090 注册 + `check-obligations.py`（9.2）接入 CI，17 条新用例；t3 独立验证 PASS / t4 复审 PASS / t5 落 02 §2 注记；慢用例降档 99079bc52） | 4.1.3 | `test_obligations_drained` 转绿；**CI run 38337943411 七作业全绿** |
| | 4.3.2 义务诊断 W→E 升级（**2026-10-13 用户裁决：甲——同码显式 `.severity(Error)`**，不新增 E 码以免同一事实两个码=平行表示；含 t3 F1 / t4 F2 的门禁加固；（**2026-10-13 已完成**：变异证据「摘任一交付点 → `error[W1090]` + 退出码 1、还原复零」，健康态零发射 ⇒ 语料 342/342 零变化；门禁三项加固 F1 锚点校验 / F2 差值输出（21−13−8=0）/ F4 定义行=声明行，七夹具 selftest；t11 独立验证 PASS 6/6、t12 复审 PASS） | 4.3.1 | 人工 review 每处新增 E；D59 D 类破坏项已在 02 与 RFC-013 双处登记 |
| 4.4 证明层与 wasm 收尾 | 4.4.1 `layers/README.md` 层序改实际顺序（**2026-10-18 已完成**——README 重写为实际调用序（predicate → termination → ownership，调用点锚定 `checker.rs::check_module_impl`），equivalence 按真实身份描述；同源声明 7 处收口（01 / 02 / 039 / 06 / typecheck README / layers 两文件），机械对账脚本 11/11 PASS（批次树绿基线上；变异 (c) 回退旧声明即红——t5 的门禁与变异记录见 `target/yx-scratch/close-44/evidence-summary.md`）；独立验证 t5 PASS / 复审轮 2 t11 PASS；**归属阶段：P4；是否阻塞主线：否**） | — | **独立于 4.1–4.3 走** |
| | 4.4.2 SMT 求解器获取面统一为进程级共享单例（**按 D58**：真实修复位置 `backend.rs`——`LazyLock<Mutex<Option<Box<dyn Solver>>>>` + `with_shared_solver` 闭包口；`predicate.rs` 删私有 SOLVER、`checker.rs` termination 注入改闭包内判定（`TerminationChecker` 生命周期参数化）、`ownership.rs` 回边判定改共享口；`default_solver()` 仅保留测试用。**硬前置**：Unknown 不入缓存（否则超时结果经进程级缓存跨编译/跨测试固化）。89576fafd 只交付了计数器观测工具，本任务本体未动）（**2026-10-18 已完成**——独立验证 t5 判据②自构造读数：三点 hit/miss 之和 (4,4) = 共享实例累计 `Some((4,4))`；变异 (a) 单例退化 → predicate hit=0 即红；**编译生产链上 `default_solver()` 调用点 = 0**（`wasm/src/lib.rs` 的 `test_smt` 自检调用点是**既存**（早于本批）的非测试调用点，属「测试与独立场景」面，不构成 D58 禁令违反）；该笔提交含 `z3_backend.rs` 15 行——`cache_stats` 从固有方法迁入 `Solver` trait 块（返回 `Some(实例内计数)`）+ 1 行注释改指，无语义耦合；施工期发现 Z3 `Sort mismatch` 崩溃经 t5 对照判定为**预存在**（`b5e68b3c9` vs 冻结 revision 同探针同输入输出逐字节一致），登记 P3 3.19；证据 `target/yx-scratch/close-44/evidence-summary.md`；**归属阶段：P4；是否阻塞主线：否**） | — | 生产路径 `default_solver()` 调用点归零 + 缓存命中率跨三消费点可观测 |
| | 4.4.3 降级路径加 warning（`checker.rs` 的 `TypeChecker::ownership_inputs` 判据点）（**2026-10-18 已完成**——新增码 W1082 经用户追认（E 类对外契约表变更）；钉测试 2/2 绿（降级发射 / 健康路径零发射）；健康路径语料 348/348 零 diff；**降级分支生产不可达穷举成立**（CLI 四入口 + eval/repl + format + LSP + wasm playground + 全部 `ProgramKind` 汇入 `check_module_impl`：`init_body_checker` 无条件先于所有权块，函数内 0 个 return/break/continue，`body_checker` 无 `.take()`——W1082 无用户可见面）；变异 (b) 删发射 → 钉测试即红；**归属阶段：P4；是否阻塞主线：否**） | — | — |
| | 4.4.4 SMT 共享求解器观测账 `ConsumerStats`/`CONSUMER_STATS`：生产路径每点都写、生产侧 0 读取（读取口只有 `proof/smt/tests/shared.rs`）。**需裁决**：保留并把「唯一消费者 = 测试」写成 doc 契约（推荐，保 D58 可观测判据载体）／删掉（去掉每次求解的原子累加，但 4.4.2 验收要改写） | 4.4.2 | issue [#475](https://github.com/ChenXu233/YaoXiang/issues/475) |
| 4.5 跨层契约 PlanId 化（D20） | 4.5.1 `ReleasePlan` 键 `Span` → `PlanId`（**2026-10-13 已完成**：`PlanId` 载体 + 语句身份归属 + `ReleasePlan::consume` 唯一消费入口；消费必达结算 `ConsumptionLedger<K>` + `ir_gen.rs:1095` 完成标记，缺一即 E 级 `W1090`；t1 实现 / t2 独立验证 PASS / t3 复审 PASS） | 4.3.1 | `test_release_plan_spans_consumed` 差集为空（D41，`examined ≥ 173`，无白名单）；语料 342 条零 diff；**CI run 38385449219 全绿** |
| | 4.5.2 `overload_resolutions` 键 `Span` → `PlanId`（**2026-10-13 已完成**：调用点身份（`Expr::Call.plan_id`，同一分配器）；产出 `inference/expressions.rs:4550`、消费 `ir_gen.rs:7697`（有绑定）/ `:7772`（无绑定），读与记账同一次调用；t4 实现 / t5 独立验证 PASS / t6 复审 PASS） | 4.5.1 | `test_overload_resolutions_consumed` 差集为空 + **证据面地板**（examined ≥ 1 / consumed ≥ 2）；C2 语料 342 条零 diff；IR 快照零漂移；**CI run 38404887215 全绿**（覆盖 4.5.2 代码树——其自身 run 被后续文档推送取消） |
| 4.6 LSP 语义数据管线统一 | 4.6.1 项目内文件的语义数据（SemanticDB）走 orchestrator 同管线，与诊断同源；跨文件引用带 `resolves_to`（**2026-10-16 已完成**——`Export.definition_span` + `ModuleInfo.definition_file` 供给定义位置；checker 引用排空经 `import_def_locations` 重定向（std 导入回退 use 行）；`DriverOutcome.semantic_db` 通道（CollectAll 形态汇入，Lsp 注入非目标文件顶层导出定义面）；LSP `analysis.rs` 单编译入口取代 server.rs 手工管线，World 键规范化 file URI；实录见 [09a](./09a-wbs-history.md)） | 4.2.1 | 项目内跨文件跳转命中定义；语义数据与诊断出自同一次编译 |
| | 4.6.2 跳转/悬停/重命名的查库键改用光标下标识符自身的起点（**2026-10-16 用户裁决后同日完成**——`resolve_reference` 按引用起点精确匹配（D55 precise-only），光标在标识符中部时静默落空；definition/hover/references/rename 四 handler 统一改键，references/rename 的定义点回退判定对标识符起点同构成立；钉板：四 handler 中部光标各 1 枚 + 跨文件 e2e 双位置断言） | 4.6.1 | 光标在标识符任意位置与起点同效 |
| 4.9 引用池正确实现 | 4.9.1 池的根按 `ExplicitSurfaces` 解析（`collect_package_refs`，`helpers.rs:481`）：按 `[[bin]]`/`[lib]`/`[exports]`/`[run].main` 根调 `discover_with_used` 取编译集，**不扫目录找入口**；全根退化时退路 `collect_compilation_refs`（`helpers.rs:454`）。**2026-10-16 轮 1 完成（提交 `3863a9ff0`）** | 4.2.1 | `collect_yx` 不再递归扫描目录；**corpus 语料 diff 零未归因新增 W1001**（每条新增警告必须可追溯到某个真实未使用定义） |
| | 4.9.2 粒度升级为 `(module, name)`：池类型改为 `HashMap<String, HashSet<String>>`（键 = 模块键），装配唯一入口 `collect_ref_pool`（`helpers.rs:429-449`），消费端在 `arms.rs:603-609` 的 `other_module_refs` 把「除本单元外」的全部模块条目并集后送 `DeadCodeAnalyzer`；模块自有名（顶层定义与局部绑定，`use` 导入名例外）不再豁免别的模块同名定义。**2026-10-16 轮 1 完成（提交 `3863a9ff0`）** | 4.9.1 | 同名局部变量不再意外豁免顶级定义；**单测：a.yx 的 `foo` 和 b.yx 的 `foo` 独立计数** |
| | 4.9.3 Script 不用池：`arms.rs:611-623` 的 `match us.role` 只对 `Bin`/`Internal`（`set_exempt_pub`）与 `Lib`（`set_cross_file_refs`）装配，Script/Test 走 `_ => {}` 默认臂（按文件内定义-使用图判定）。**2026-10-16 轮 1 完成（提交 `3863a9ff0`）** | 4.9.2 | Script 单文件语义恢复；**单测：项目内其他文件的引用不影响 script.yx 的 W1001** |
| | 4.9.4 消费定义细化：区分"被 import"和"被 import 后真实使用"——只有后者算消费（需 `use` 项的路径解析 + 实际使用点追踪）。**2026-10-17 轮 2 完成（提交 `e6cf1aeaa`——`RefPool::consumed_from` 按 import 归因到定义模块，`resolve_import_module` 复用发现期同一份模块键集合）** | 4.9.3 + 4.6.1 | `use a::foo` 但从不调用 `foo()` 的情况下，a.yx 的 `foo` 仍报 W1001；**硬前置**：`use` 项路径解析（4.6.1 交付） |
| | 4.9.5 测试池分离：tests/ 的引用单独成池，不豁免生产代码（src/）的未使用定义。**2026-10-17 轮 2 完成（提交 `e6cf1aeaa`——`FileRole::Test` 模块不入消费池，退化路径与声明面路径两来源都喂角色）** | 4.9.4 | **单测：tests/helper.yx 引用 src/util.yx 的 `debug_print`，但 src/ 内无消费者时，`debug_print` 仍报 W1001**；RFC-029f 表格更新（测试豁免范围缩窄到仅测试自身） |
| | 4.9.6 方法可达性口径（`#450`；2026-10-13 登记 → **2026-10-16 用户裁决 A 并入轮 2**）：无项目根路径下**被调用且重载决议生效的方法**仍报 `W1005 Unused method`。现状机制（2026-10-16 代码走读 + 实测）：`compute_reachability` 只把 full name 放进 `reachable`，而 `is_reachable` 对方法按**短名**查同一集合 ⇒ 方法永不进入可达集（于是无池路径下每个方法都被报）；`all_defs` 以 full name 为键 ⇒ 同名重载互相覆盖，只可能报出最后一个。实测：`method_call_args.yx` / `generic_method_binding.yx` / `interfaces.yx` 各 3 条 W1005，`method_overload.yx` 只 1 条 | 4.9.3 | 先红后绿：① 被调用（含重载）的方法不报 W1005；② 同名重载各自独立判定（`all_defs` 不得按 full name 互相覆盖）；③ **不被调用**的方法仍如实报 W1005（不得放宽）；④ 与轮 1 三条口径（池的根 = 包目标 / `(module,name)` / Script 不接池）不冲突。**2026-10-17 轮 2 完成（提交 `ddcbb12d0`；修复轮 `b8380e6ab` 按用户 2026-10-17 裁决 A 收口——逐候选决议 + 未命中兄弟重载恢复报 W1005）；跨单元缺口登记 4.9.7；归属阶段：P4；是否阻塞主线：否** |
| | 4.9.7 跨单元方法调用的决议登记与接收者归因（**2026-10-17 登记**，issue [#464](https://github.com/ChenXu233/YaoXiang/issues/464)）：跨单元（registry 解析）的方法调用不产生决议条目 ⇒ 回落池的名字级豁免（`RefPool::field_names` 全局平铺）⇒ 跨单元同名同序的重载定义此前不被报；依赖 D20 产出侧扩展（决议登记带接收者类型身份）。**2026-10-17 轮 3 完成（产出侧 `b8b3cc3f6` + 驱动侧 `d5c00235a`；注入面误报修复轮 `d5dd1a37b` + 钉测试归位 `c2f887843`）**；**归属阶段：P4；是否阻塞主线：否** | 4.9.6 | **单测：跨单元同名同序的两个重载定义，仅一个被调用时另一个仍报 W1005**（先红后绿，钉在 `src/driver/tests/method_unused.rs` 的 `test_cross_unit_uncalled_same_named_method_is_reported`） |
| | 4.9.8 跨单元同名类型经**内联别名**/模块限定引入时方法派发落到**裸名归属单元**（静默错值、`check` 0 error）；D59 **C 类**；先红后绿 + 把 `CROSS_UNIT_A/B` 方法体改为**可区分**并加值级断言（现两单元实现完全相同，测试对缺陷是盲的） | 4.9.7 | issue [#465](https://github.com/ChenXu233/YaoXiang/issues/465) |
| 4.10 多文件语义缺口 | 4.10.1 check 路径 parse 错误硬中止降级（**2026-10-09 已完成**，d929c77b——用户裁决 rust 式收集语义 + 方案 B「带病文件退出编译单元」：parse 失败文件只报 parse 诊断、不进 registry，其余文件照常全阶段收集，CLI 汇总后非零退出；compile 路径 FailFast 硬中止保留为正确语义；LSP 路径的同类形态随 4.2.3 继承） | 4.2.2 | 含语法错误项目的 `yaoxiang check` 产出逐文件诊断而非整体中止（语料夹具 `parse-error-in-collect-all` + driver/CLI 钉板） |
| | 4.10.2 discover 自动注入 std.list（对齐单文件 `merge_embedded_std_ir` 的 #117 行为）（**2026-10-09 已完成**，5ba52eaf——用户裁决 fast-track 本轮修复，不再缓办）：多文件项目无显式 `use std.list` 的 for 循环曾**编译通过、运行期 E6006**（探针实证） | — | corpus 夹具 `for-loop-no-use-std-list` 绿；基线 331 语料既有 330 条零漂移 |
| | 4.10.3 standalone run 与 MultiFile 编排统一（4.2.5 裁决 A 的 run 半边，用户指示**插队**）（**2026-10-15 已完成**——完全体单队实施，非 DAG；先红后绿：driver 级 3 钉 + CLI e2e 3 钉；`--lib` 2733 / integration 287 / corpus 342 行差分 12 行纯限定名 / IR 快照零漂移；实录见 [09a](./09a-wbs-history.md)）：`run_file_with_diagnostics` 的 standalone 分支改走 `compile_project`——相对导入解析与 check 对齐。**前置**：3.4.8 多文件 mono 臂（compile_project 无 Monomorphization 阶段，standalone 泛型程序会被削能力）；另需 Linking 的 `entry_function` 按 `is_bin_role` 门控（Script 不自动调 main，#413）+ #413 Script 提示重新键控（`module.source_files.is_empty()` 标记消失） | 3.4.8 | standalone 相对导入 run/check 一致解析；#413 提示的测试钉板不动 |
| | 4.10.4 rootless 多文件 `run` 的各单元初始化序列按**发现顺序**拼接、无依赖拓扑保证 ⇒ 入口顶层代码先于依赖初始化执行、依赖顶层值读成 `void`（exit 0 无诊断）；D59 **C 类**；修法二选一（按 `use` 图拓扑序 / 对齐 Bin 把顶层语句抬到合并后入口）+ 顺序钉测试 | 4.10.3 | issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467) |
| | 4.10.5 rootless 定义 `main` 却未调用时打印提示但**退出码 0** ⇒ CI 误判成功；退出码属 D59 **D 类**（落地前需确认） | 4.10.3 | issue [#476](https://github.com/ChenXu233/YaoXiang/issues/476) |

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

**这是全部 CI 脚本的唯一权威清单**（11 个脚本）。脚本在「引入阶段」接入 CI；标注 report-only 的在 P9 统一转硬（从"报告"改为"失败"）——存量违规（平行表示、消歧别名等）要等 P6/P8 才清零，提前转硬会让 CI 长期红、门禁被人工绕过，P9 转硬时门禁应当恰好全绿。P2 的 2.5、P4 的 4.3.2、P7 的 7c.1 均引用本表，不另立清单。

- [ ] **9.1–9.11 十一个脚本按引入阶段接入，P9 统一转硬**（明细见下表）

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
| 9.11 | `check-classification.py` | PR 必填块「错误类型（A–F，按 RFC-039 D59）」标注：A/B/C 类须附红判据引用，D/E/F 类须附决议号或用户裁决 | **P4**（report-only，P9 转硬） | 故意漏标或错标一个 A 类改动必须红 |

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

| 二级 | 三级 | 验收 |
| --- | --- | --- |
| S2 删纯占位死代码 | `instance.rs:416-830`（415 行）/ dispatch 链 / 死阶梯 / 旧语法探测 / `pipeline/tests`（14 行）/ `undefined/` 空目录；**E2：`lib.rs:568` / `:582` 两处 `_ => todo!()` 改有意义的降级输出** | `cargo test` 通过；`clippy -D warnings` 无新增 |
| | S2.2 零消费者 `pub` 符号清账（发版前审计新增 2 处）：`frontend/core/typecheck/signature.rs` 的 `parse_signature_param_names`（真死，约 −3 行）· `frontend/module/orchestrator.rs` 的 `check_source_in_project`（`src/` 0 调用，但 `scripts/ci/check-fanout.py` 按 (文件,符号) 登记为锚点 ⇒ **删前先核该脚本对锚点缺失的行为**） | issue [#472](https://github.com/ChenXu233/YaoXiang/issues/472) |
| S3 空头设计文档 | C1+C2 删除后重写为指向 `src/frontend/module/`（约 2,670 行）；C3 改写边界表；**同步清理 `config.js:259-268`** | 文档站无 404；`check-docs-truth.py` 通过 |
| S4 RFC 状态修正 | C4 RFC-018 移回 `draft/`（D46）；C5 RFC-028 补 `impl_status: 'not-started'`；F3 `TRACKING.md` 加「实现状态」列（D36） | `check_tracking.py` 退出码 0（**不手改 TRACKING.md**） |
| S5 `pub` 项降可见性 | B2 `TypeSystem` / B4 `check_type_equivalence`（**`is_subtype` 必须保留**） | 编译器能指出真正的死代码 |
| S6 opcode 死路径清理 | B8 `TailCall` 与 B9 `Switch`（D32/D34：删，opcode 值回收已在 7e.1 随升版落地）；B10 `UnaryOp::Not`（D33：`opcode()` 区分 `op` 字段）；**F7+F8 opcode 生成期门禁**（`tools/code-tables` 扩展 opcode 提取器约 30 行 + 5 处事实表达单源化 + `size()` 全 opcode 对拍测试） | opcode 事实 5 处表达收敛为 1 处权威源；`size()` 注释与实际编码不符（`bytecode.rs:2181-2182`）修复 |
| S7 wasm32 门控 project-mode 子树（2026-10-18 登记，D35 假设修正；用户裁决 B） | S7.1 把整条 project-mode 子树在 wasm32 上门控（`#[cfg(not(target_arch = "wasm32"))]` 或等价 cfg 面）：D35 设想的「删不可达者」在 `orchestrator.rs` **无适用目标**（4 条 wasm32 正臂运行期不可达但编译期承重——调用点未门控；6 条 `not(wasm32)` 反臂 native 三条阶段表承重；逐条判定见 `06-cleanup-inventory.md` §「F2 判定」）。**三条未核实（原样保留，落地前须补）**：① 产物规模收益未量测；② shim 之外的 wasm32 消费者未穷举；③ native 承重为静态调用链未端到端 | 先红后绿不适用（无行为变化面）：门控后 wasm32 目标编译通过且产物缩小可测；native 全量回归零漂移（语料 + IR 快照）；`06` F2 行同步指向本登记 |
| S8 文档更正形式收敛（就地改写，删修订注记）——2026-10-18 登记（用户同意） | S8.1 按本批固化规则（`coding-rules.md` 第四部分）审计存量「修订注记」并就地改写。**scope（本次审计清单）**：① `02-stage-contract.md:431` 阶段表草图两条次序偏差集成进草图；② `02:486-492` 三档表与「为什么先做 W 级」段改写为终态 E 级事实；③ `02:780-798` Driver 草图改为落地形态；④ `02:461-479` Obligations 草图先逐条核对；⑤ `013-error-code-specification.md:517` 正文改写为现状、删旧引文；⑥ `09:130` 3.2 行写实测结论；⑦ 指针 6 处（`02:1217`、`02:1239`、`04-ssa.md:194/447/721`、`09a:141`）改指正文对应段 | `docs/src/dev/architecture/` 下 `grep -c 修订注记` 归零（`09a` 除外）+ 6 处指针改指正文 + 涉及代码事实处 file:line 在冻结 revision 重取 |
| S9 发版前审计的抽象化收口（**2026-10-18 登记**；**归属阶段：P10；是否阻塞主线：否**） | S9.1 `typecheck/inference/statements.rs` 的上下文灌注块两份（`infer_statement` 两条分支各抄一份，字段顺序已漂移）；本次把新 setter 同时写进两块 ⇒ **本次新增 2 份**。抽共有 setter ≈ **−14 行** | issue [#473](https://github.com/ChenXu233/YaoXiang/issues/473) |
| | S9.2 `MonoType` 全变体遍历两份手写副本（`frontend/core/types/mono.rs` 的 `type_name` 177 行 / `middle/passes/mono/instance.rs` 的 `type_name_hash` 157 行，均为本次新写）；加一个变体实测要动 **6 个文件**。**不建议合并这两个函数**（渲染 vs 哈希语义不同），只抽「子节点枚举」一份 | issue [#474](https://github.com/ChenXu233/YaoXiang/issues/474) |

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

**不可并行（文件重叠）**：P1(1.1.x) ∥ P8(8.0.x)；1.2.1 ∥ 8.3.2；6.0 ∥ S2（B6/E5 同批文件）；4.3.2 ∥ 9.2（已统一到 P4）；**3.4 ∥ P3.5**（`checker.rs` / `environment.rs` 同文件——外部轨道的删 pub 批次与静默通道修复必须串行）。

**文档内部新增约束**：6.0→6.1→6.2→6.3→6.4→6.5→6.6 严格串行；7a→7b→7c→7d 严格串行，且 7b 的 `Phi` 臂必须先于 7d。

## 关键决策与理由

- **P0 独立成阶段而非并入 P9**——门禁是"事后检查"，规程是"事前判断"。P9 的脚本无法阻止"该重构却打了补丁"，只有 D0–D4 的 review 清单能。
- **P9 作为 CI 脚本的唯一权威清单**——原缺口 G3 的三套并存清单导致无法收口。本表合并后 11 个脚本各归属一个阶段，report-only 与转硬时点显式可查。
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
