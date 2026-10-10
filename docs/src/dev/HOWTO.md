# 实现者手册（HOWTO）

> **本页是所有改动的统一入口，不是规则本体。** 规则的唯一权威：
>
> - 改代码 → [coding-rules.md](coding-rules.md)（三条禁令 + D0–D4，精确规则本体）与
>   [RFC-039 决议登记](../rfc/accepted/039-compiler-architecture.md)（D1–D52）
> - 改文档 → [docs-rules.md](docs-rules.md)（Diátaxis 分类规则 + 维基式语言风格）
>
> 本页只做两件事：**动工前强制自检**和**"补丁式修复"的判定**。若本页与规则本体冲突，以规则本体为准。

## 第 0 步：判定你要改什么

| 你的改动                   | 读哪一节 | 规则本体                           | 提交前验证   |
| -------------------------- | -------- | ---------------------------------- | ------------ |
| 改代码（`src/`、`tests/`） | 第一节   | [coding-rules.md](coding-rules.md) | 见第一节末尾 |
| 改文档（`docs/`）          | 第二节   | [docs-rules.md](docs-rules.md)     | 见第二节末尾 |
| 两者都改                   | 两节都读 | 两本都守                           | 两套命令都跑 |

注意：改代码常常必须同步改文档——错误码表、CLI 参数、用户可见行为都在文档里有断言。第一节的验收通过不等于完事，先回答「这个行为在哪篇文档里被描述过，要不要跟着改」。

## 一、改代码

### 你在哪个阶段

施工总表：[09-execution-wbs.md](architecture/09-execution-wbs.md)。**先找到你的三级任务条目**（如 4.2.7），确认它的：前置是否完成、验收判据是 C1–C6 哪一级（判据定义见
[07](architecture/07-equivalence-oracle.md)）、是否标注"独占 commit"。

### 动工前自检（全部过一遍，写代码之前）

- [ ] 我读过本任务在 [09](architecture/09-execution-wbs.md) 的三级条目原文，不是只看了任务标题
- [ ] 我读过被改代码的**实际上下文**（打开文件看过，不是凭任务描述想象）
- [ ] 我知道本任务的验收判据级别（C1 zero-diff / C2 诊断集 / C3 诊断码 / C4 行为等价 / C5
      AST+诊断+行为 / C6 纯删除）
- [ ] 我跑过
      **D0**：本次改动触碰任何表（错误码/opcode/类型/阶段）吗？权威源模块是哪个？——写不出模块名就是没查
- [ ] 我跑过
      **D1/D2**：新加的类型/枚举/常量表，能用既有概念+参数表达吗？与既有概念同语义吗？（同语义禁止用 import 别名弥合）
- [ ] 我跑过 **D4**：新增代码属于目标模块**已有的职责类别**吗？（职责类别权威定义见
      [01-routing.md](architecture/01-routing.md) 目录职责表）
- [ ] 我跑过 **取舍判据**：我（或方案提出方）的否决理由是否只基于**正确性**与**可读性**？
      以「改动面大／要引入新机制／成本高」为由否决方案 = 无效否决，review 打回
      （coding-rules 第二部分）；需用户拍板的取舍按 [AGENTS.md](../../AGENTS.md) 给全四件
- [ ] 我的新机制是「登记-消费」型吗（一方登记数据、另一方消费）？是的话，生产端与消费端
      **直驱**的端到端测试同批交付了吗？消费端测试手工构造登记数据 = 静默通道，review 打回
      （coding-rules 第六部分；先例见 RFC-039 决议登记 D55）
- [ ] 若本任务要写/改测试：我读过 [test-specification.md](test-specification.md)——命名 `test_<what>_<scenario>` 前缀、超 5 行 AAA 三段注释、断言带自定义消息、枚举匹配 `assert!(matches!(...))`、无 `use super::*`、无 inline `mod tests {}`
- [ ] 若我的改动会改变 IR 或语料运行行为（前端/ir_gen/std/语料 `.yx`）：我计划**同提交**更新 IR 快照与语料差分基线（命令见 AGENTS.md 常用命令区），并人工 review 两者 diff——并行流 #385 曾漏同步快照被门禁拦（先例 dabdfd96）
- [ ] 若任务说"先红后绿"（如 2.4.1、8.8.1），我确认红判据已存在且当前确实是红的
- [ ] 我没有为了过验收而准备"删掉失败测试"或"放宽判据"——**做不到是实现缺陷，如实报告，不是放宽的理由**（D24/不存在 C5′）
- [ ] 我问过自己：这个行为在哪篇文档里有描述，本次改动要不要同步改文档（改→第二节）

### 这是不是"补丁式修复"（D3，命中即止）

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**第 3 条是本手册存在的理由。**
"改 ≥3 处同义映射"意味着你在维护一份平行事实——正确动作是消灭平行，不是再加一处。历史事故：3 套运算符枚举、5 个编译入口、手写类型同义词表（详见
[08](architecture/08-maintenance-mechanism.md) 事故表）。

### 几条容易踩的硬规矩

- **不看的代码不改**：没有打开过文件、没有 grep 过引用点，就不许动它。行号以仓库现状为准，文档行号会漂移。
- **不留待实现遗留**：核心功能不许出现 `todo!()` / `Vec::new() // Not implemented yet` /
  "独立 issue 以后再修"。发现了就修，或登记进当前阶段任务——不许带出新遗留（D52 先例：`.42`
  三处数据丢失已全部收编 P7）。
- **核实项必须逐行核实**：任务标注"必须逐行核实，不能假定等价"（如 4.2.7、6.1.3）时，把核实结论写进 PR 描述。
- **删测试换绿灯 = 违规**：DoD 要求每阶段结束时测试数不低于开始时。
- **机械问题已被工具兜住的部分不要人肉 check**：fmt / clippy / `build.rs` 门禁 / P0 起的
  `scripts/ci/check-*.py`。review 只看机器查不了的（职责判定、D3）。

### 卡住了怎么办

| 情况               | 正确动作                                                                                    | 错误动作                       |
| ------------------ | ------------------------------------------------------------------------------------------- | ------------------------------ |
| 判据跑不绿         | 如实报告，修实现；怀疑判据错了就回到 [07](architecture/07-equivalence-oracle.md) 提案改判据 | 给判据加白名单/豁免            |
| 发现设计前提不成立 | 回到 [RFC-039 决议登记](../rfc/accepted/039-compiler-architecture.md) 改决定并说明原因         | 绕开决定悄悄做                 |
| 任务比预想大       | 在 [09](architecture/09-execution-wbs.md) 里把三级任务再拆细                                | 一个超大 commit 混多个验收判据 |
| 发现文档与代码矛盾 | 先信代码，再修文档（先例：`layers/README.md` 层序反了，D7）                                 | 照着错的文档写代码             |

## 二、改文档

### 分类：这篇文档属于哪个目录

用 [docs-rules.md](docs-rules.md) §1 的决策树回答，答案必须唯一（tutorial / guide / reference /
explanation / dev / rfc）。**答不出唯一答案，说明这篇文档在服务两种受众——先拆分，再动手。**

### 动工前自检（全部过一遍，改文档之前）

- [ ] 我用 docs-rules §1 决策树确定了目录，答案唯一
- [ ] 我 grep 过同类内容；同一内容没有 ≥2 处副本（有则合并，不新增第 3 处）
- [ ] 我查过 docs/glossary.json 与既有文档，沿用既有术语，没有生造新名字
- [ ] 新增文档：frontmatter `title` 已写；侧边栏已挂（`config.js` +
      `i18n/en.json`；generateSidebar 扫描目录内的文件无需手工挂载）
- [ ] 移动/重命名文档：六处同步全部完成（见下方「移动文档六同步」）
- [ ] 语言风格已过 docs-rules §2：主动语态、确定陈述句、现在时、术语一致、无营销词

### 移动文档六同步（移动 = 改 URL，缺一不可）

1. 用 `git mv` 移动（保留文件历史）
2. 全站链接：grep 旧路径逐一更新（构建的死链检查会兜底，但不替你做）
3. 侧边栏与导航：`docs/src/.vitepress/config.js` 与 `docs/src/.vitepress/i18n/en.json`
4. en/ 镜像：同步移动对应英文文件
5. 引用了该路径的脚本与配置：`scripts/`、`docs/scripts/`、`.github/`
   工作流、`AGENTS.md`、`README.md`
6. `docs/src/.i18n-cache.json` 的缓存键（键含文件路径）

### 这是不是"补丁式修复"（文档版 D3，命中即止）

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### 提交前验证（改文档）

```bash
cd docs
pnpm docs:build                                # 死链零容忍，构建必须绿
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # 文档示例真实执行
python ../scripts/ci/check-docs-orphan.py      # 孤儿页与重复 H1
python ../scripts/ci/check-docs-truth.py       # 版本注入与事实对账
```

改了 `rfc/` 的，再加一条：

```bash
python ../scripts/rfc/check_tracking.py        # RFC 状态一致性
```

## 参考

- [coding-rules.md](coding-rules.md) — 三条禁令、D0–D4、红线、review 清单（**代码规则本体**）
- [docs-rules.md](docs-rules.md) — Diátaxis 分类规则、维基式语言风格（**文档规则本体**）
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md)
  — 历史事故表、外部参照、被否方案论证（诊断记录，非规则来源）
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — 三级任务表、依赖、并行分组
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 判据定义
- [01-routing.md](architecture/01-routing.md) — 职责类别权威定义、目标目录结构
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — 总纲与决议登记 D1–D52
