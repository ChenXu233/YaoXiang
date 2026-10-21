# YaoXiang 仓库指南（人类与 AI 实现者共用）

YaoXiang 是一门编程语言及其编译器（Rust 实现）。`src/` 约 16 万行，正按 [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) 做架构重构（四层模型：编排/前端/语义与 IR/执行）。

## 元规则（强制，2026-10-14 用户裁决）

1. **文档里不得出现私有内容**：写进仓库的任何文字（AGENTS.md、WBS、实录、任务契约、报告、提交信息、代码注释）都必须**任何人可解析**。判据：换一个既没有本机环境、也没有本会话记录的贡献者来读——他能不能顺着这段文字独立找到同一事实？不能＝私有内容，必须改写。
   - **典型的私有内容**：记忆系统节点 id、会话 id / 任务 attempt id、个人缓存目录、仅本机存在的绝对路径（例外：讲本机工作台 / 环境配置的文档，那里的路径是**内容**而非引用）、只有某个私有账号或模型能访问的资源、未公开的内部代号。
   - **正确写法**：写**事实与证据本身**——谁在何时测到什么数字、哪条命令、哪个 run id / issue 号 / revision / file:line。
2. **先查证，再断言**：被问「有没有这条规则 / 做过这件事 / 这是谁定的」时，**先查**（仓库文档 → 变更记录 → 可用的记忆或检索工具（若有）），再回答；**不得**只凭对话上下文直接答「没有」。查不到时要说明**查了哪些地方**。

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

6. **契约编写（inScope 必须覆盖机械必需面）**：写 implementation / repair 类任务的 inScope 时，必须把该目标的**机械必需面**一并圈进来，至少三类——① 新引入或变更的对外契约产物（诊断码表、`locales/`、生成器再生的文档、`build.rs`/门禁要求同步的文件）；② 任何断言被改结构**形状**的直驱测试文件（改字段/变体/枚举成员会连带它们编译不过）；③ 与门禁判据做**多源对账**的文档表；④ **被裁决直接点名的声明载体**——含代码注释/文档注释（「声明与事实不符」本身就是这类裁决要修的对象，只改文档一侧等于判据没兑现）。写契约前先 grep「谁断言这个形状 / 谁与它交叉对账」。复核别人报的发现时，必须在**被复核的那个 revision** 上核（工作区可能在动——本会话实例：我在 t10 进行中的树上核 t8 的一条发现，得出「不可复现」，实际是那条已被 t10 顺手修掉；换成 `git show <rev>:<file>` 或先问清被核 SHA 即可避免）。**契约 `inScope` 一律写可被路径匹配器解析的路径前缀**：如 `src/util/diagnostic/codes/`、`docs/src/rfc/accepted/013-error-code-specification.md`；不得用散文限定（如 `docs/src/（仅限错误码生成物）`）——散文限定会被 `changedPaths` 校验拒绝，使机械必需面落在声明之外。**写入文档的提交号必须在 push 之后取**——push 前的 `pull --rebase` 会重写本地哈希（2026-10-13 实例：`94ad03ce4` → `631eb3e3c`；本会话共 3 次因 inScope 写窄被迫 amend）。
7. **成员路由原则**：审核 / 验证 / 复审成员必须与实现者使用**不同的模型家族或档次**——让产出代码的同一个模型去审查，等于共享盲区，独立验证只剩形式。具体的 provider / 模型 / 额度回退属**各人的会话环境设定**（可用权限不同），**不写进本文件**。

8. **提交卫生（每次推送 ≤1 个文档提交）**：同一个二级任务内的 WBS 勾选、注记、登记、规则修改**攒到一处**，作为该次推送的**最后一笔**文档提交；生成器再生产的文档**并入其所属的功能提交**；与门禁做多源对账的文档表**必须与其代码同提交**（例：`02` §义务字段表 ↔ `check-obligations.py` 的三处对账）。禁止为「一次勾选」或「一处登记」单独发一笔提交；提交号一律在 push 之后取（见第 6 条）。
9. **批次验证树与清理（唯一入口 `scripts/dev/scratch.py`）**：验证/复审必须在**冻结的 revision** 上量测，因此要独立的树；但 cargo 的指纹含源码**路径**——**换树 = 全量重建，同一棵树切 revision = 增量**。故：
   - **一个批次一棵树**，固定建在仓库内 `target/yx-scratch/<batch>/tree`（`git worktree`；`target/` 已被 `.gitignore` 覆盖）。**禁止**任何成员直接 `git worktree add` / `git clone` / 在仓库之外新建目录（含 `E:\git` 顶层与 `E:\`）。
   - 上锁与切换：`python scripts/dev/scratch.py use <batch> --rev <sha> --by <成员> --task <id>`；**同一时刻只允许一个持有者**，用完 `release`。**实现者不进这棵树**（它在主仓工作树写代码）。
   - 构建统一走 `python scripts/dev/scratch.py build <batch>`（内部固定 `cargo build --bin yaoxiang-rs`），不要各自发明构建命令。
   - 确需两个成员**真并行**量测不同 revision 时，才 `init <batch>-b` 开第二棵，并在批次记录里写明理由（代价 = 再一次全量构建）。
   - **及时删除**：批次收口任务跑 `python scripts/dev/scratch.py gc`，把 `status` 输出贴进任务输出；验收项 = 「`git status` 干净 + 仓库外无本批新增目录 + 无残留批次树」。
   - 零散探针（`.yx`）同样放 `target/yx-scratch/<batch>/`（实测 CLI 能直接跑，且不进语料差分）。
   - 参考数字：一棵树首次全量约 5 分钟 / 约 3 GB；此后每次切 revision 只重编差异面。本会话曾一度每任务一棵、并在仓库外散落 6 个目录约 780 MB（2026-10-14 已清理），本条即为根治。
10. **验证环境与提交证据（九则）**：
   - ① 独立检出 / 探针一律走第 9 条的 `scripts/dev/scratch.py`（批次树，仓库内），不另建树、不出仓库。
   - ② **变异必须机械可逆**：把「故意改坏」写成补丁，用 `python scripts/dev/scratch.py trial <batch> --patch <p> -- <命令…>` 执行（内部 `git apply` → 跑命令 → `git apply -R` 还原 → 逐文件比对 blob 与 `HEAD` 一致并要求 `git status` 干净）。**手工改文件再手工改回来不算**；报告里给补丁路径与还原前后 blob。为什么：验证者要证明「判据能失败」，唯一硬证据是破坏它看测试红不红——但还原必须是机械的，不能靠记性。
   - ③ **连推会取消前一轮 CI**（`concurrency`）：**不必为此在会话里等 run（见第 13 条）**——照常连推；两笔推送并存时**记明**「以后代 run 的结论覆盖前一笔」，核对环节按这条覆盖关系取结论。
   - ④ 报告 / 文档里引用的**行号必须在被引用的 revision 上重取**（`git show <rev>:<file>`），不接受凭记忆写号。**且引用必须是任何人都能解析的**：仓库内路径、revision、run id、issue 号——**不得**写只有某些会话才有的东西（记忆系统节点 id、会话 id、本机盘符路径如 `E:\…`、个人缓存目录）。限定范围：本条管的是**引用出处**；若文档本身就在讲本机工作台/环境配置（如 `branch-maintenance-guide.md`），那里的路径是**内容**而非引用，不受此限。
   - ⑤ 本地 verify 清单**必须包含 CI 独有门禁**（至少 `python scripts/ci/check-docs-truth.py`、`python scripts/ci/check-boundary.py`、`python scripts/ci/check-fanout.py`，以及重跑 `python scripts/docs/gen-error-code-docs.py` 后确认 `git diff --quiet`）；**改动诊断码或其发射点（增、删、行号平移）都必须重跑该生成器**（该生成器位置敏感，再生必带行号平移与自身计数更正——提交信息须分类说明，**禁止手改生成物**）。为什么写死这几个：`check-boundary.py` 的 `pub(crate)` 计数在 2026-10-21 从 167 涨到 171 一路漏到复审才被发现；`check-fanout.py` 的同名多定义同理；错误码生成物则在 3.17.1 删除 71 行判定、挪动若干行后**无人重跑**，导致 CI 自 2026-10-21 连红 5 轮（失败 job 只有「Error-code docs up to date」）。三者的共同点是**只在 CI 跑、本地清单没有**——门禁一旦不在本地，就等于不存在。
   - ⑥ 验证探针必须覆盖**同类路径的未修分支**：只证「新功能对」不足以放行（本批实例：t2 放行、t3 抓到 F1——t2 的探针全落在命中面）。
   - ⑦ **提交期间的 stash 窗口（操作约定，2026-10-16 用户同意）**：`pre-commit` 在跑钩子前会把**全部未暂存改动** stash 走、跑完再恢复（日志行 `[INFO] Stashing unstaged files to …` / `Restored changes from …`）。因此提交进行中的那几分钟里，其他成员直接读工作树会看到「只剩已 staged 文件」的假象（本会话实例：验证者据此报告「轮 2 改动丢失」，实为窗口内读取）。约定：① 提交前先给未暂存改动留 patch 备份；② 提交期间他人只按冻结 sha 量测、不读工作树；③ 每笔提交完成后用 `git status` 核对剩余改动完整性。
   - ⑧ **推送与同步归属可核（2026-10-17）**：成员**不得**运行 `git push` / `git fetch` / `git pull`；同步与推送由队长统一做，同步一律 `git pull --rebase origin dev`，**出现 merge 提交即视为流程偏差**。两条都可用 `git reflog --date=iso` 与 `git reflog show origin/dev` 直接核验——**流程归因不得凭报告措辞推断**（实例：队长据「报告里写已在 origin/dev」断言某成员自行 push，reflog 显示实为命令面之外的 `pull --tags` + push，归因撤回）；发现远端/工作树被第三方改动须当场报告。派工消息里的口头约定不构成判据——可核项必须写进任务契约的验收。
   - ⑨ **文档提交必须本地真构建一次（2026-10-17）**：markdownlint 抓不到 VitePress 的 Vue 编译错误（尖括号占位符被当成元素，报 `Element is missing end tag`），且 pre-commit 的 markdownlint 钩子在仅含 docs 的提交里会被判 `no files to check` 跳过。故：**占位符一律进反引号**，文档提交推送前**本地跑一次真实构建**（`npm run docs:build`）。实例：`042c07860` 使 Deploy Docs 红（run 39011946793），修复提交 `0c837e1b4`。
11. **写入方负载上限（2026-10-14 用户裁决；依据 = 上下文过载会导致质量下滑，见 ⑧ 的实测）**——**注意 ⑨：本条是「当前无上下文压缩能力」时的替代措施，压缩可用后 ① 即作废**：
   - ① **单一成员 attempt 上限 3**：指派前先数 `agent_teams_status` 里该成员已完成/失败的 attempt；到限即换人（`agent_teams_add_member` 新建 → `agent_teams_reassign_task` 改派 → `agent_teams_remove_member` 收回）。
   - ② **同一工作项内部的多轮 review→repair→再评审属正常返工，不占这 3 个额度**（本会话：t1→t2→t3→t9→t10 算 1 个工作项；t6→t7→t8→t12→t13 算 1 个）。
   - ③ **跨子系统（前端 / 中端 / 后端 / 标准库 / 包管理）绝不由同一成员连做**——本会话实例：同一实现者连做 typecheck + ir_gen + std 注册三处，产出的正是「纪律松弛型」缺陷（原生臂内联复制共享判据、两条路径裁决分歧，t8 F1）。
   - ④ **收口（integration / 登记文档）与实现分离**：不排给已跑满的成员（收口只需契约 + 事实清单，不需要实现上下文）。
   - ⑤ **评审轮换**：同一评审连续 5 轮以上即换人。
   - ⑥ **交接必须紧凑**：契约要点 + file:line 关键路径 + 已就绪产物位置 + 已知坑；不要让接手者读整份历史报告。
   - ⑦ **汇报义务**：每次向用户汇报时列出各成员累计 attempt，并说明是否触发换人。
   - ⑧ 依据是**上下文容量**，不是礼节，有两次实测：**①** 2026-10-02 会话：一个实现者连做 6 个 attempt、横跨五个子系统后，出现本地化回归与一处未经对抗验证的策略引入；**②** 2026-10-14（本批）：实现者连做 typecheck + ir_gen + std 三处后产出「纪律松弛型」缺陷（内联复制共享判据 → 两条路径裁决分歧，t8 F1），同日验证者因 `ran out of room` 中途换人。
   - ⑨ **失效条件（2026-10-14 用户裁决）**：本条的 ①（attempt 上限 3）是「**当前 harness 尚无上下文压缩（compact）**」时的**替代措施**——上下文不能压缩，就只能靠换人控制长度。**一旦压缩可用**（即第 5 条「要给长上下文成员续派时**优先压缩上下文**，压缩不可用时才换人」里的「优先压缩」真正可执行），**① 作废**，改按压缩后的实际容量判断（压缩后仍超载才换人）。**③（跨子系统连做禁令）与 ⑤（评审轮换）不随其失效**——它们防的是**认知盲区**，与上下文长度无关。
   - 注：另有「每批新工作用新 worktree + 全新小团队、不复用上一批团队」的说法与**第 5 条**（同一二级任务内成员上下文跨轮次保留）**相反，以第 5 条为准**；本条（负载上限）与第 5 条不冲突，两者并行有效。
12. **换人后的历史轮次（操作教训，2026-10-15）**：`agent_teams_remove_member` 会把该成员的**全部**任务退回共享池——**包括已判决的终态复审/验证轮**；调度器随后会**重跑**它们（本批实例：轮换复审时 t3/t8/t13 被退回，其中 t3/t8 立刻被新成员重新领走，既做重复工作又占住成员、堵住下游任务）。因此换人后必须**立刻**处置退回的历史轮次：① 判据 = 该轮的 verdict 与报告已登记 ⇒ 不应重跑；② 处置 = `agent_teams_reassign_task(<id>, assignee="captain")` 接管 → `agent_teams_update_task(<id>, status="cancelled")`（**终态任务不可直接改，必须先接管**）；③ 把「哪些历史轮次被退回、如何处置」写进批次收口记录。

13. **不等 CI（2026-10-16 用户裁决）**：推送后**不在会话里等 run**——`gh run watch`、反复轮询 `gh run list`、只为「看到绿」而空转，一律不做。一轮 CI 十几分钟起，等待烧的是上下文与 token，而这些时间本可以做下一件工作。门禁证据改交**专门环节**核对，并在当次记录里写明交给了哪一个：① 定时核对（挂一次带时间的核对提醒，到点只看结论）；② 发版 / 合并前的统一核对（把该批所有提交的 run 一起看）；③ 另起的只读核对会话。
    - **证据要求不放宽**：核对结果必须落回记录（run id + conclusion + 覆盖关系）——**不许**把「没等」当成「已绿」；⑩③ 在连推时照写。
    - **例外**：正在排障的那一笔失败可以查它的日志——那是诊断输入，不是门禁仪式。

**为什么**：按"轮"建队会把同一二级任务拆散，任务点过细，反复付建队成本、清空成员已积累的仓库认知，拖慢整体计划（2026-10-13 用户裁决：团队范围由三级抬到二级）。

## 发现值得记录的规则时

工作中若发现值得固化的规则（如被反复纠正的做法、新的红线、新的判定标准），**先向用户提出，经同意后**再更新 [coding-rules.md](docs/src/dev/coding-rules.md) 等文档（HOWTO、红线清单、对应 RFC 决议登记等按归属就近落位）。规则以文档为准，不要只留在对话记录里。**且规则文本（不只是意图）必须先贴给用户过目，获同意后才能提交**——用户批准了「要加这条规则」不等于批准了你的措辞。

## 常用命令

```bash
cargo fmt                          # 格式化（提交前必跑）
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/rfc/check_tracking.py   # RFC 状态一致性

# 测试怎么跑（#486：资源画像标记 + 预算通道）——
python scripts/dev/testsuite.py plan             # 只打印计划：预算 / 波次 / 每波峰值 / 实际 jobs 与 threads
python scripts/dev/testsuite.py run fast         # 本地提交前默认通道：非 ignore 全跑；mode=exclusive 各自独占一波
python scripts/dev/testsuite.py run full         # 推送前与 CI 口径：预算不足即 exit 2 并列出缺口，不静默跳过
python scripts/dev/testsuite.py markers --check  # 标记 ↔ cargo test --list 双向对账（新增 #[ignore] 必须带 cost: 标记）
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test   # 手跑兜底：必须自带资源上限，否则重测会与其它测试并发
# 禁止裸跑 cargo test -- --ignored 全量：会执行 mode=manual（快照再生改写基线）与红态常驻用例。
# 当前 pre-commit 仍跑 cargo test --all；把门禁切到 fast/full 的接线属 #486 后续排期。
# 测试数只许增不许减。

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

推送后门禁**不在会话里等**（见「团队编排约定」第 13 条）：交由定时核对 / 发版前统一核对 / 另起的只读核对会话，结论（run id + conclusion）落回当次记录。`push` 到 `dev` 会拉起 `ci.yml` 与 `docs-deploy.yml`。

## 红线（review 必打回）

- 不看代码就改：没打开文件、没 grep 过引用点，不许动
- 核心功能留 `todo!()` / “Not implemented yet” / 无限期“独立 issue”
- 删测试或放宽判据换绿灯（不存在 C5′；做不到是实现缺陷，如实报告）
- 裸跑 `cargo test -- --ignored` 全量（会执行快照再生的 `manual` 档与红态常驻用例——等于悄悄改写基线）
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
