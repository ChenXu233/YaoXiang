# YaoXiang 仓库指南（人类与 AI 实现者共用）

YaoXiang 是一门编程语言及其编译器（Rust 实现）。`src/` 约 16 万行，正按 [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) 做架构重构（四层模型：编排/前端/语义与 IR/执行）。

## 元规则（强制）

1. **文档里不得出现私有内容**：写进仓库的任何文字（AGENTS.md、WBS、实录、任务契约、报告、提交信息、代码注释）都必须**任何人可解析**。判据：换一个既没有本机环境、也没有本会话记录的贡献者来读——他能不能顺着这段文字独立找到同一事实？不能＝私有内容，必须改写。
   - **典型的私有内容**：记忆系统节点 id、会话 id / 任务 attempt id、个人缓存目录、仅本机存在的绝对路径（例外：讲本机工作台 / 环境配置的文档，那里的路径是**内容**而非引用）、只有某个私有账号或模型能访问的资源、未公开的内部代号、**工作区内被 `.gitignore` 排除的目录**（如批次验证树与探针目录——它字面在仓库内，却**不在仓库里**：别人 clone 后根本没有这个目录。门禁：`scripts/ci/check-docs-refs.py`）。**例外**：构建期生成物（如错误码参考页 `docs/src/reference/error-code/`）不是私有内容——它由仓库内的 `scripts/docs/gen-error-code-docs.py` 从码表与 `locales/*.json` 生成，任何人 clone 后都能重建。
   - **正确写法**：写**事实与证据本身**——谁在何时测到什么数字、哪条命令、哪个 run id / issue 号 / revision。
2. **先查证，再断言**：被问「有没有这条规则 / 做过这件事 / 这是谁定的」时，**先查**（仓库文档 → 变更记录 → 可用的记忆或检索工具（若有）），再回答；**不得**只凭对话上下文直接答「没有」。查不到时要说明**查了哪些地方**。

3. **规则文本保持清爽**：规则变更就地改写，只保留当前有效规则；历史对照不进本文件，历史留在 git 与实录。

## 改代码之前（强制）

**先读，再动手——这条不分角色。** 主会话、子代理、带着明确任务的成员，只要这次会改代码（含测试、脚本、生成物、文档里的判据），**必须先读完** [实现者手册 HOWTO.md](docs/src/dev/HOWTO.md) 与 [coding-rules.md](docs/src/dev/coding-rules.md) 全文，再动第一行。

- 「任务很明确」「只改一行」「我是子代理，任务写死了」**都不是豁免理由**——规则约束的是动作，不是身份。
- **派工的 Lead 有责任**：给子代理的任务提示里必须带上这条要求（见「团队编排约定」第 4 条）。
- **子代理的第一次改动前**，要在回复里说明**这次适用哪几条红线**（写不出 = 没读）。这是给人看的自检，不是机械门禁——它拦不住敷衍的人，但能让敷衍留下痕迹。

**六条红线本体在 [coding-rules.md](docs/src/dev/coding-rules.md) 第三部分；下方「红线（review 必打回）」是它的 review 清单。**

1. **读 [实现者手册 HOWTO.md](docs/src/dev/HOWTO.md)**——动工前自检表 + “补丁式修复”判定（D3）。它是入口；规则本体在 [coding-rules.md](docs/src/dev/coding-rules.md)。
2. 施工任务查 [09-execution-wbs.md](docs/src/dev/architecture/09-execution-wbs.md) 的三级任务表（带 `- [ ]` 看板）。
3. 三条禁令（概要，全文见 [coding-rules.md](docs/src/dev/coding-rules.md)）：**不得生造**（新概念先证明与既有概念不重复）、**不得职责累积**（一个模块一类职责）、**该重构不补丁**（同一行为 ≥2 处复制 / 新增第 N 个入口 / 一次改 ≥3 处同义映射 → 停工走设计流程）。

### 改文档之前（强制）

**文档规则的唯一本体是 [docs-rules.md](docs/src/dev/docs-rules.md)，不是 coding-rules.md**——coding-rules 管代码，docs-rules 管文档（docs-rules 开头即写明二者同级）。改文档前先读它，再读下面三样。

同一条「先读再动手」也适用于文档。改任何规范 / 设计 / 计划文档前，必须读：

1. **上游设计依据**：这次改动对应的 RFC 决议（[RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) 的决议登记 D1–D64，或该领域的专题 RFC）；
2. **同层相邻文档**：同一张表 / 同一组术语是否在别处已有定义（两处各写一份 = 「不得生造」的翻版）；
3. **[docs-rules.md](docs/src/dev/docs-rules.md)**（文档规则本体；§5 是权威链与修改前提）。

**方向规则**：RFC 是 SPEC 的设定依据——SPEC 可以写「设计依据见 RFC」，**RFC 不得以 SPEC 为依据**，否则 RFC 再也不能改 SPEC。**更正就地写入**，不留「勘误 / 原为…现改为…」的对照，历史在 git 里。

## 请求用户裁决时（强制）

凡需要用户拍板的决策（架构取舍、语义偏差、登记/不登记、破坏性变更、规则固化、排序插队等），**必须一次性给全以下四件**，缺一件视为未完成汇报：

1. **情况**：现状是什么、撞上该决策点的原因、以及支撑证据（文件:行、实测输出、探针结论）；未核实的事实要显式标注"未核实"，不许用推测冒充结论。
2. **每个选项的优点与缺点**：逐项列清，不许只给结论；缺点里**不许拿"改动面大 / 要引入新机制"当否决理由**（判据见 [coding-rules.md](docs/src/dev/coding-rules.md)：否决理由只能是正确性与可读性）。
3. **自己的推荐**：明确写出推荐哪个、为什么，并如实说明推荐项自身的代价与风险。
4. **不可逆或破坏性的部分要单独点出**（基线再生、行为变更、对外契约变更等）。

只报"有问题，等你定"而不给优缺点与推荐，等于把工作推回给用户。

**语义偏差的适用面（RFC-039 D59 分流）**：**入口判据是「旧行为与已写条款是否一致」**，不是「编译器此前是否接受」——编译器误接受的程序不算成功。能指到 RFC/SPEC 节号并给出最小反例（应报未报 / 应过未过 / 静默给错结果）⇒ **A/B/C 三类可顺路修**，条件是**先红后绿 + 条款与反例 + 不改变「原先符合规范」程序的对外表现 + 同提交登记 + 基线同提交再生与人工 review**；**指不到条款（规范未写明、歧义、旧行为本已合规）⇒ D/E/F，仍须先找用户裁决**。规范未写明 ≠ 违反规范。判据见 [07](docs/src/dev/architecture/07-equivalence-oracle.md) §语义修正的分流判据 与 [RFC-039 决议登记 D59](docs/src/rfc/accepted/039-compiler-architecture.md)。

## 团队编排约定（强制）

**一支团队的范围 = 一个 WBS 二级任务**（如 `4.3` 义务账本，`4.3.1` / `4.3.2` 同属一支）。其下三级任务是团队内的工作单元，按需追加，不逐轮重审。**为什么**：按轮建队会把同一二级任务拆散、反复付建队成本、清空成员已积累的仓库认知。

1. **建队与增量**：该二级任务的前置（其下各三级任务前置的并集）满足时组队；先摆「首轮 DAG + 后续轮次骨架」交用户复核，**批准后**才启动。同一二级任务内的后续三级任务直接追加，不重建队、不重复审批。**同一时刻只有一支活动团队。**
2. **跨框**：必须做但属另一个二级任务的活——先登记 WBS → 报用户裁决 → 获准才并入并在案记录；未获准留给该任务自己的团队。
3. **归档**：该二级任务下全部三级任务收口——验证/复审通过 + 提交已推送 + 本地门禁全绿（清单见 HOWTO「提交前验证」）。CI 结论不作前置（见「推送到 dev 之后」）。
4. **派工**：给任何成员的提示必须含一句「动工前先读 [HOWTO](docs/src/dev/HOWTO.md) 与 [coding-rules](docs/src/dev/coding-rules.md) 全文」；成员第一次改动前要说明本次适用哪几条红线（写不出 = 没读）。缺这句的派工无效。
5. **成员路由与上下文**：审核 / 验证 / 复审必须与实现者不同模型家族或档次——同一个模型审自己的产出等于共享盲区；具体 provider / 模型 / 额度属各人环境设定，不写进本文件。建成员时设 `context_max_tokens`，让插件空闲时自动压缩；到阈值的成员先 `/compact` 再接任务，压缩后仍紧的不再派工。
6. **契约的 `inScope` 必须覆盖机械必需面**：改一个形状会连带触到的一切都要圈进来——生成物（诊断码表、`locales/`、生成器再生的文档）、被改形状的直驱测试、与门禁对账的文档表、被裁决点名的载体（含代码与文档注释）、受影响变体或指令的定义家与全仓穷尽 `match` 的补臂点、`tests/baselines/` 与 `src/middle/ir/tests/snapshots/`、新测试的家目录与 `mod.rs`。**写法**：只写可被路径匹配器解析的前缀（`src/util/diagnostic/codes/`），散文限定会被 `changedPaths` 拒收。**判据**：动契约前 grep「谁断言这个形状 / 谁与它交叉对账」，命中即进 `inScope`。
7. **验证证据**：量测只在 `scripts/dev/scratch.py` 的批次树上做——冻结 revision、仓库内 `target/yx-scratch/<batch>/tree`、一个批次一棵树、禁止自建树或自创构建命令，收口 `gc`；变异写成补丁走 `trial`（机械还原，手工改回去不算），`trial` 之后必须 `build` 再量测。探针要覆盖同类路径的**未修分支**——只证「新功能对」不足以放行。复核别人报的发现，要在**被复核的那个 revision** 上核（`git show <rev>:<file>`），不读正在动的工作树。流程见 HOWTO「验证量测」。
8. **推送归属可核**：成员不跑 `git push` / `fetch` / `pull`；同步与推送由队长统一做，同步一律 `pull --rebase origin dev`，出现 merge 提交即流程偏差。归属用 `git reflog --date=iso` 与 `git reflog show origin/dev` 核——**不凭报告措辞推断**；发现远端或工作树被第三方改动须当场报告。可核项写进任务验收，派工消息里的口头约定不算。

## 发现值得记录的规则时

工作中若发现值得固化的规则（如被反复纠正的做法、新的红线、新的判定标准），**先向用户提出，经同意后**再按归属落位——代码规则进 [coding-rules.md](docs/src/dev/coding-rules.md)，文档规则进 [docs-rules.md](docs/src/dev/docs-rules.md)，动工入口进 [HOWTO.md](docs/src/dev/HOWTO.md)，RFC 级裁决进对应 RFC 的决议登记。规则以文档为准，不要只留在对话记录里。**且规则文本（不只是意图）必须先贴给用户过目，获同意后才能提交**——用户批准了「要加这条规则」不等于批准了你的措辞。

## 常用命令

```bash
cargo fmt                          # 格式化（提交前必跑）
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/rfc/check_tracking.py   # RFC 状态一致性

# 测试怎么跑（#486：资源画像标记 + 预算通道）——
python scripts/dev/testsuite.py plan             # 只打印计划：预算 / 波次 / 每波峰值 / 实际 jobs 与 threads
python scripts/dev/testsuite.py run fast         # 提交前必跑：--lib + 集成侧四个 tests 目标；装不下的波逐条报「跳过 N 条」
python scripts/dev/testsuite.py run full         # 推送前与 CI 口径：预算不足即 exit 2 并列出缺口，不静默跳过
python scripts/dev/testsuite.py markers --check  # 标记 ↔ cargo test --list 双向对账（新增 #[ignore] 必须带 cost: 标记）
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test   # 手跑兜底：必须自带资源上限，否则重测会与其它测试并发
# 禁止裸跑 cargo test -- --ignored 全量：会执行 mode=manual（快照再生改写基线）与红态常驻用例。
# pre-commit 两段：先跑上面的 run fast（快反馈），再跑 cargo test --all --quiet 拦提交。
# doctest（cargo test --doc）与语料差分不在 run fast 覆盖面内，由 CI 兜底。
python scripts/ci/check-test-channel.py         # 测试口径同源：本区 / test-specification 18.1 / pre-commit entry 三处一致
# 测试的准入与退役见 coding-rules 第七部分（三扇门：条款 / 病史 / 证明）。

# 基线再生——改动影响 IR 或语料行为时必跑，diff 人工 review 后同提交入库：
UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored    # IR 快照（src/middle/ir/tests/snapshots/）
cargo run --quiet --example corpus_probe > tests/baselines/corpus-parity.jsonl  # 语料差分
```

提交与 PR 规范见 [commit-convention.md](docs/src/dev/commit-convention.md)——提交粒度（一笔一个理由、禁止纯簿记提交、生成物随代码、收口文档最后落）与消息格式都在那里；PR 模板含“职责归属与决策程序”必填块（D0–D4），写不出权威源模块名就是没查，review 会打回。

## 推送到 dev 之后

`dev` 上挂着自动翻译（`.github/workflows/auto-translate.yml`）：**任何改动
`docs/src/**/*.md` 的 push 都会让 bot 追加一个 `:pencil: docs: auto-translate documentation`
提交并回推 dev**。于是本地立刻落后 1 个提交，下一次 `git push` 报
`non-fast-forward`——**这不是并行开发流**。确认与处理：

```bash
git log --oneline dev..origin/dev   # 作者是 github-actions[bot] 即是它
git pull --rebase origin dev        # 与对端无冲突（merge-tree 干净）；已应用的补丁会被自动丢弃
```

**issue 不随 dev 上的提交自动关闭**：自动关闭关键词（`关闭 #N` / `Closes #N`）只在提交进入**默认分支**时由 GitHub 处理；本仓默认分支是 `main`，日常推送目标是 `dev`，所以 dev 提交信息里的 `关闭 #N` 当下不会关闭任何 issue。修完 issue 后在 GitHub 上**手动关闭**（如 `gh issue close <N> --comment <证据>`），评论里写清修复提交 sha 与回归测试——关闭理由必须能独立核验。

**门禁不在会话里等**：推送后不看 run、不轮询；**CI 结论也不写进文档**——按 sha 在 GitHub 上就能查，写进去没有消费者，而回填提交又会拉起新一轮 CI。只在两处看：① **红了**——收到失败通知或顺手看到，查日志定位并修（那是诊断输入，不是门禁仪式）；② **发版 / 合并前的统一核对**。连推取消前一轮 run 属正常（`concurrency`），不必记录覆盖关系。

## 红线（review 必打回）

- 不看代码就改：没打开文件、没 grep 过引用点，不许动
- 核心功能留 `todo!()` / “Not implemented yet” / 无限期“独立 issue”
- 删测试或放宽判据换绿灯（做不到是实现缺陷，如实报告）；测试的**准入与退役**按 [coding-rules 第七部分](docs/src/dev/coding-rules.md)——三扇门（条款 / 病史 / 证明）一扇都过不了才可退役，「因为红了就删」永远打回
- 裸跑 `cargo test -- --ignored` 全量（会执行快照再生的 `manual` 档与红态常驻用例——等于悄悄改写基线）
- 写/改测试不读 [test-specification.md](docs/src/dev/test-specification.md)——命名必须 `test_<what>_<scenario>` 前缀、超 5 行必须 AAA 三段注释、断言必须带自定义消息、枚举匹配用 `assert!(matches!(...))`；判据红态常驻属设计形态（如 2.4.4），不是「永久 ignore」违规
- 改 IR/语料行为而不同步基线：改 `src/`（前端/ir_gen/std）或语料 `.yx` 会改变 IR 或运行行为时，IR 快照与语料差分基线必须**同提交**更新（命令见常用命令区），各自 git diff 人工 review；快照/差分门禁红了说明基线欠账，不许绕
- 用 import 别名弥合同语义概念（`as AstBinOp` 这类）
- 新增 `include!`、新增跨层反向依赖（L2→L3 等）、`pub(crate)` 跨层泄漏增加
- PR 必填块必须如实填写：D0 权威源模块名写不出 = 没查；「错误类型（A–F，按 D59）」漏标或与改动实际不符，同样打回

## 关键文档地图

| 文档 | 解决什么问题 |
| --- | --- |
| [HOWTO.md](docs/src/dev/HOWTO.md) | 动工前自检、补丁判定 |
| [coding-rules](docs/src/dev/coding-rules.md) | 三条禁令、D0–D4、红线、review 清单（规则本体） |
| [09](docs/src/dev/architecture/09-execution-wbs.md) | 施工任务表（P0–P10，三级 WBS） |
| [01](docs/src/dev/architecture/01-routing.md) | 加一个特性该改哪里、目录职责表 |
| [07](docs/src/dev/architecture/07-equivalence-oracle.md) | 等价性判据 C1–C6 |
| [test-specification](docs/src/dev/test-specification.md) | 测试怎么写（命名/AAA/断言消息——写测试前必读） |
| [docs-rules](docs/src/dev/docs-rules.md) | 文档怎么写（分类/风格/权威链——改文档前必读） |
| [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) | 总纲、决议登记 D1–D64 |
