# YaoXiang 仓库指南（人类与 AI 实现者共用）

YaoXiang 是一门编程语言及其编译器（Rust 实现）。`src/` 约 16 万行，正按 [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) 做架构重构（四层模型：编排/前端/语义与 IR/执行）。

## 改代码之前（强制）

1. **读 [实现者手册 HOWTO.md](docs/src/dev/HOWTO.md)**——动工前自检表 + “补丁式修复”判定（D3）。它是入口；规则本体在 [coding-rules.md](docs/src/dev/coding-rules.md)。
2. 施工任务查 [09-execution-wbs.md](docs/src/dev/architecture/09-execution-wbs.md) 的三级任务表（带 `- [ ]` 看板）。
3. 三条禁令（概要，全文见 [coding-rules.md](docs/src/dev/coding-rules.md)）：**不得生造**（新概念先证明与既有概念不重复）、**不得职责累积**（一个模块一类职责）、**该重构不补丁**（同一行为 ≥2 处复制 / 新增第 N 个入口 / 一次改 ≥3 处同义映射 → 停工走设计流程）。

## 请求用户裁决时（强制）

凡需要用户拍板的决策（架构取舍、语义偏差、登记/不登记、破坏性变更、规则固化、排序插队等），**必须一次性给全以下四件**，缺一件视为未完成汇报：

1. **情况**：现状是什么、撞上该决策点的原因、以及支撑证据（文件:行、实测输出、探针结论）；未核实的事实要显式标注"未核实"，不许用推测冒充结论。
2. **每个选项的优点与缺点**：逐项列清，不许只给结论；缺点里**不许拿"改动面大 / 要引入新机制"当否决理由**（判据见 [coding-rules.md](docs/src/dev/coding-rules.md)：否决理由只能是正确性与可读性）。
3. **自己的推荐**：明确写出推荐哪个、为什么，并如实说明推荐项自身的代价与风险。
4. **不可逆或破坏性的部分要单独点出**（基线再生、行为变更、对外契约变更等）。

只报"有问题，等你定"而不给优缺点与推荐，等于把工作推回给用户。

**语义偏差的适用面（RFC-039 D59 分流）**：不是所有语义修正都要等裁决——**A/B/C 三类**（旧行为**非成功**：崩溃/爆栈、静默失败、静默错配、静默不动作、静默给错答案）可随重构**顺路修**，条件是**先红后绿 + 不改变「原先成功」程序的对外表现 + 同提交登记 + 基线同提交再生与人工 review**；**D/E/F 三类**（改变原先成功程序的可观察行为、对外契约表、RFC 级设计变更）**仍然必须先找用户裁决**。判据见 [07](docs/src/dev/architecture/07-equivalence-oracle.md) §语义修正的分流判据 与 [RFC-039 决议登记 D59](docs/src/rfc/accepted/039-compiler-architecture.md)。

## 团队编排约定（强制）

**一支团队的负责范围 = 一个 WBS 二级任务**（如 `4.3` 义务账本——其下的 `4.3.1` / `4.3.2` 同属一支团队）。二级任务内部的**三级任务是团队内的工作单元/轮次**，按需增量加任务，不逐轮重新审批、不另建团队。

1. **建队时机**：该二级任务的前置（其下各三级任务前置的并集）满足时组队；一次摆出"首轮 DAG + 后续轮次骨架"交用户复核，**经用户批准后**才启动（未批准的计划不产生任何工作）。
2. **增量**：同一二级任务内的后续三级任务与轮次**直接向现有团队追加任务**——不重复建队、不重复审批。
3. **加框（跨二级任务）**：发现“必须做但属另一个二级任务”的活，**先登记 WBS → 报用户裁决 → 获准才可并入**当前团队（并记录在案）；未获准则留给该任务自己的团队。
4. **归档条件**：该二级任务下的**全部三级任务**收口——含验证/复审通过、**提交已推送**、门禁（CI / Deploy Docs）绿。
5. **串行现实与上下文**：同一时刻只有一支活动团队；成员上下文跨轮次保留正是本条的目的之一——要给长上下文成员续派时**优先压缩上下文**，压缩不可用时才换人并在记录里写明原因。

6. **契约编写（inScope 必须覆盖机械必需面）**：写 implementation / repair 类任务的 inScope 时，必须把该目标的**机械必需面**一并圈进来，至少三类——① 新引入或变更的对外契约产物（诊断码表、`locales/`、生成器再生的文档、`build.rs`/门禁要求同步的文件）；② 任何断言被改结构**形状**的直驱测试文件（改字段/变体/枚举成员会连带它们编译不过）；③ 与门禁判据做**多源对账**的文档表；④ **被裁决直接点名的声明载体**——含代码注释/文档注释（「声明与事实不符」本身就是这类裁决要修的对象，只改文档一侧等于判据没兑现）。写契约前先 grep「谁断言这个形状 / 谁与它交叉对账」。复核别人报的发现时，必须在**被复核的那个 revision** 上核（工作区可能在动——本会话实例：我在 t10 进行中的树上核 t8 的一条发现，得出「不可复现」，实际是那条已被 t10 顺手修掉；换成 `git show <rev>:<file>` 或先问清被核 SHA 即可避免）。**写入文档的提交号必须在 push 之后取**——push 前的 `pull --rebase` 会重写本地哈希（2026-10-13 实例：`94ad03ce4` → `631eb3e3c`；本会话共 3 次因 inScope 写窄被迫 amend）。
7. **成员路由原则**：审核 / 验证 / 复审成员必须与实现者使用**不同的模型家族或档次**——让产出代码的同一个模型去审查，等于共享盲区，独立验证只剩形式。具体的 provider / 模型 / 额度回退属**各人的会话环境设定**（可用权限不同），**不写进本文件**。

8. **提交卫生（每次推送 ≤1 个文档提交）**：同一个二级任务内的 WBS 勾选、注记、登记、规则修改**攒到一处**，作为该次推送的**最后一笔**文档提交；生成器再生产的文档**并入其所属的功能提交**；与门禁做多源对账的文档表**必须与其代码同提交**（例：`02` §义务字段表 ↔ `check-obligations.py` 的三处对账）。禁止为「一次勾选」或「一处登记」单独发一笔提交；提交号一律在 push 之后取（见第 6 条）。
9. **临时工作区与清理（scratch）**：一切临时产物——独立检出、探针树、验证用的外部树——**只许建在本仓库工作区内**：`target/yx-scratch/<成员或任务>/`（`target/` 已被 `.gitignore` 覆盖，既不脏 `git status` 也不进提交）；仓库根的 `.yx-scratch/` 作为备选（已进本地 `.git/info/exclude`）。**禁止**在 `E:\git` 顶层或仓库之外的任何位置新建目录（2026-10-14 用户裁决）。任务收尾**必须删除自己建的 scratch 子树**，并在任务输出里写明「建了哪些路径、删了没有」；同一二级任务的收口任务把「`git status` 干净 + 仓库外无本批新增目录」列为验收项（本会话实例：`E:\git` 顶层曾累积 `verify45-ext`、`t8/t11-baseline-extract`、`yx39-*` 等 6 个目录约 780 MB，由队长 2026-10-14 清理）。若某类 `.yx` 探针被 CLI 的文件发现排除，换用备选路径并在输出里说明，**不得**因此跑到仓库外。

**为什么**：按"轮"建队会把同一二级任务拆散，任务点过细，反复付建队成本、清空成员已积累的仓库认知，拖慢整体计划（2026-10-13 用户裁决：团队范围由三级抬到二级）。

## 发现值得记录的规则时

工作中若发现值得固化的规则（如被反复纠正的做法、新的红线、新的判定标准），**先向用户提出，经同意后**再更新 [coding-rules.md](docs/src/dev/coding-rules.md) 等文档（HOWTO、红线清单、对应 RFC 决议登记等按归属就近落位）。规则以文档为准，不要只留在对话记录里。

## 常用命令

```bash
cargo fmt                          # 格式化（提交前必跑）
cargo clippy --all --all-features --all-targets -- -D warnings
cargo test                         # 测试；测试数只许增不许减
python scripts/rfc/check_tracking.py   # RFC 状态一致性

# 基线再生——改动影响 IR 或语料行为时必跑，diff 人工 review 后同提交入库：
UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored    # IR 快照（src/middle/core/tests/snapshots/）
cargo run --quiet --example corpus_probe > tests/baselines/corpus-parity.jsonl  # 语料差分
```

提交与 PR 规范：`docs/src/dev/commit-convention.md`；PR 模板含“职责归属与决策程序”必填块（D0–D4），写不出权威源模块名就是没查，review 会打回。

## 推送到 dev 之后

`dev` 上挂着自动翻译（`.github/workflows/auto-translate.yml`）：**任何改动
`docs/src/**/*.md` 的 push 都会让 bot 追加一个 `:pencil: docs: auto-translate documentation`
提交并回推 dev**。于是本地立刻落后 1 个提交，下一次 `git push` 报
`non-fast-forward`——**这不是并行开发流**。确认与处理：

```bash
git log --oneline dev..origin/dev   # 作者是 github-actions[bot] 即是它
git pull --rebase origin dev        # 与对端无冲突（merge-tree 干净）；已应用的补丁会被自动丢弃
```

推送后核对门禁：`gh run list --limit 5`（push 到 dev 会拉起 `ci.yml` 与 `docs-deploy.yml`）。

## 红线（review 必打回）

- 不看代码就改：没打开文件、没 grep 过引用点，不许动
- 核心功能留 `todo!()` / “Not implemented yet” / 无限期“独立 issue”
- 删测试或放宽判据换绿灯（不存在 C5′；做不到是实现缺陷，如实报告）
- 写/改测试不读 [test-specification.md](docs/src/dev/test-specification.md)——命名必须 `test_<what>_<scenario>` 前缀、超 5 行必须 AAA 三段注释、断言必须带自定义消息、枚举匹配用 `assert!(matches!(...))`；判据红态常驻属设计形态（如 2.4.4），不是「永久 ignore」违规
- 改 IR/语料行为而不同步基线：改 `src/`（前端/ir_gen/std）或语料 `.yx` 会改变 IR 或运行行为时，IR 快照与语料差分基线必须**同提交**更新（命令见常用命令区），各自 git diff 人工 review；快照/差分门禁红了说明基线欠账，不许绕
- 用 import 别名弥合同语义概念（`as AstBinOp` 这类）
- 新增 `include!`、新增跨层反向依赖（L2→L3 等）、`pub(crate)` 跨层泄漏增加

## 关键文档地图

| 文档 | 解决什么问题 |
| --- | --- |
| [HOWTO.md](docs/src/dev/HOWTO.md) | 动工前自检、补丁判定 |
| [coding-rules](docs/src/dev/coding-rules.md) | 三条禁令、D0–D4、红线、review 清单（规则本体） |
| [09](docs/src/dev/architecture/09-execution-wbs.md) | 施工任务表（P0–P10，三级 WBS） |
| [01](docs/src/dev/architecture/01-routing.md) | 加一个特性该改哪里、目录职责表 |
| [07](docs/src/dev/architecture/07-equivalence-oracle.md) | 等价性判据 C1–C6 |
| [test-specification](docs/src/dev/test-specification.md) | 测试怎么写（命名/AAA/断言消息——写测试前必读） |
| [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) | 总纲、决议登记 D1–D59 |
