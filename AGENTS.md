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

## 团队编排约定（强制）

**一支团队的负责范围 = 一个 WBS 三级任务**（如 `3.4.8`）。三级任务内部的子项/轮次（①②③④…）**不另建团队**，按需增量加任务，不逐轮重新审批。

1. **建队时机**：该三级任务的前置满足时建队；一次摆出"首轮 DAG + 后续轮次骨架"交用户复核（`approval=required`）。
2. **增量**：同一三级任务内的后续轮次用 `create_task` 直接加入现有团队——不重复建队、不重复审批。
3. **加框（跨三级任务）**：发现"必须做但属另一个三级任务"的活，**先登记 WBS → 报用户裁决 → 获准才可并入**当前团队（并记录在案）；未获准则留给该任务自己的团队。
4. **归档条件**：该三级任务全部子项收口——含验证/复审通过、**提交已推送**、门禁（CI / Deploy Docs）绿。
5. **串行现实与上下文**：同一时刻只有一支活动团队；成员上下文跨轮次保留正是本条的目的之一——要给长上下文成员续派时**优先压缩上下文**，压缩不可用时才换人并在记录里写明原因。

**为什么**：按"轮"建队会把同一三级任务拆散，反复付建队成本、清空成员已积累的仓库认知，拖慢整体计划。

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
| [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) | 总纲、决议登记 D1–D52 |
