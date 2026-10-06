# 贡献指南

> 感谢您对 YaoXiang 项目的兴趣！我们欢迎各种形式的贡献。

> 🌐 **语言** | [English](docs/gh/CONTRIBUTING.en.md)

---

## 目录

- [贡献方式](#贡献方式)
- [快速开始](#快速开始)
- [提交流程](#提交流程)
- [设计提案流程（RFC）](#设计提案流程rfc)
- [代码放置与变更规程](#代码放置与变更规程)
- [代码规范](#代码规范)
- [文档更新检查清单](#文档更新检查清单)
- [贡献者许可协议（CLA）](#贡献者许可协议cla)
- [代码审查](#代码审查)
- [社区资源](#社区资源)
- [行为准则](#行为准则)

---

## 贡献方式

| 方式 |
| --- |
| 在 GitHub Issues 中报告问题 |
| 功能建议或设计讨论 |
| 改进文档或撰写教程 |
| 修复问题或实现新功能 |
| 语言设计、Logo、UI |

---

## 快速开始

### 环境准备

```bash
# 安装 Rust（建议使用 rustup）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# 构建项目
cargo build --release

# 运行测试
cargo test
```

### 代码风格

```bash
# 格式化代码
cargo fmt

# 类型检查
cargo check

# 运行所有检查
cargo clippy
```

---

## 提交流程

### 1. 创建分支

```bash
git checkout -b feature/your-feature-name
```

### 2. 提交规范

遵循 [提交规范](docs/src/dev/commit-convention.md)：

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**类型**：

| Type | 含义 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档更新 |
| `style` | 代码格式（不影响功能） |
| `refactor` | 重构代码 |
| `test` | 添加测试 |
| `chore` | 构建工具或辅助功能更新 |

**示例**：

```
feat(frontend): 添加类型推断功能

实现了基本的多态类型推断算法。

Closes #123
```

### 3. 提交 PR

1. 推送分支：`git push origin feature/your-feature-name`
2. 访问 GitHub 创建 Pull Request
3. 填写 PR 模板
4. 等待代码审查

---

## 设计提案流程（RFC）

### 提交 RFC

对于新功能或重大变更，请先提交 RFC：

1. 阅读 [RFC 模板](docs/src/rfc/RFC_TEMPLATE.md)
2. 参考 [完整示例](docs/src/rfc/EXAMPLE_full_feature_proposal.md)
3. 在 `docs/src/rfc/` 目录创建新 RFC 文件
4. 状态设为 "草案" 或 "审核中"
5. 提交 PR 进行讨论

### RFC 状态流转

```
Draft (草案) → Review (审核中) → Accepted (已接受) → accepted/
                                 → Rejected (已拒绝) → stays in rfc/
```

详见 [RFC 模板](docs/src/rfc/RFC_TEMPLATE.md)。

### RFC 实现与文档更新

> **每个被 accepted 的 RFC，其实现 PR 必须包含文档更新。**

- 实现 PR 的 PR template 必须勾选"影响文档，我已更新相关文档"
- 如果实现 PR 不包含文档更新，必须在 PR 描述中明确说明"不影响文档"并给出理由
- `scripts/rfc/check_tracking.py` 会自动检查：RFC 的 accepted 状态 + 对应实现 PR 是否包含文档变更
- 文档更新范围包括：tutorial（如果 RFC 引入新功能）、reference/language-spec（语言规范）、reference/error-code（新增错误码）等

---

## 代码放置与变更规程

> 本节是**强制规程**，适用于所有代码变更，尤其是编译器架构重构（[RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md)）期间。
> 动工前必读 [实现者手册 HOWTO.md](docs/src/dev/HOWTO.md)（自检表 + 补丁判定）；规则本体见 [coding-rules.md](docs/src/dev/coding-rules.md)。

### 三条禁令

1. **不得生造** — 新增 `pub` 类型 / 枚举 / 常量表前，必须证明它与既有概念不重复（判据 A–D 见 coding-rules：职责重叠、调用点不足、需要消歧别名、靠同义词表弥合，任一命中即违规）
2. **不得职责累积** — 一个模块只承担一类职责；要加新职责就新建模块或搬走既有职责。不设行数门禁，职责判定靠人工（这正是它不能被机器替代的原因）
3. **该重构不补丁** — 命中以下任一条即停工走设计文档流程：同一行为需 ≥2 处复制；新增"第 N 个入口"而非登记进声明式阶段表；一次修改要同步改 ≥3 处同义映射

### 决策程序（D0–D4）

每次改动按序过五道闸（每道是可判定的布尔条件，命中即止；全文见 coding-rules 第二部分）：

| 闸 | 问题 | 命中即 |
| --- | --- | --- |
| D0 | 触碰任何已存在的表（错误码/opcode/类型/阶段）？权威源模块是哪个？ | 写不出权威源 → ⛔ 先建立权威源 |
| D1 | 新概念能用「既有概念 + 参数」表达吗？ | 能 → ⛔ 禁止新增 |
| D2 | 与既有概念同语义（变体名重合 ≥ 半数）？ | 无 From/TryFrom → ⛔ 合并为一份，禁止 import 别名弥合 |
| D3 | ≥2 处复制 / 新增入口 / ≥3 处同义映射？ | 命中 → ⛔ 停工走设计流程；皆否 → 允许局部补丁 + 回归测试 |
| D4 | 新增代码属于目标模块**已有的职责类别**吗？（职责表见 [01-routing.md](docs/src/dev/architecture/01-routing.md)） | 第 2 类及以上 → ⛔ 新建模块 |

### 红线（review 必打回）

- 不看的代码不改：没打开文件、没 grep 过引用点，不许动
- 核心功能不留 `todo!()` / "Not implemented yet" / 无限期"独立 issue"
- 不删测试、不放宽判据换绿灯（不存在 C5′；做不到是实现缺陷，如实报告）
- PR 模板的"职责归属与决策程序"块为**必填**；写不出 D0 权威源模块名 = 没查

---

## 代码规范

### Rust 代码

- 遵循 `rustfmt` 默认格式
- 使用 `clippy` 进行静态检查
- 添加适当的注释和文档
- 为公开 API 编写 rustdoc

### 文档

- 使用中文标题
- 代码块标注语言
- 术语保持一致
- 遵循 [文档规则](docs/src/dev/docs-rules.md)

### 测试

- 为新功能添加单元测试
- 更新集成测试（如果需要）
- 确保所有测试通过

---

## 文档更新检查清单

> 每个 PR 在提交前必须问自己一个问题：**"这个改动是否需要更新文档？"**
>
> 以下判断标准帮助你快速决策。AI reviewer 会自动检查 PR 的文档影响评估。

### 需要更新文档的情况

| 情形 | 说明 |
|:----|:-----|
| 新增/修改了公开 API | 函数签名、类型定义、trait 等对外接口发生变化 |
| 改变了 CLI 行为 | 新增/删除/修改了命令行参数、子命令、输出格式 |
| 修改了配置格式或默认值 | `yaoxiang.toml` 等配置文件字段变化 |
| 新增/删除了功能 | 语言特性、编译器功能、工具链功能的变化 |
| 修复了文档与行为不一致的 bug | 文档描述与实际行为不符，修复后需同步更新文档 |
| 新增了错误码或警告码 | 必须更新 `docs/src/reference/error-code/` 或 `warning-code/` |

### 不需要更新文档的情况

| 情形 | 说明 |
|:----|:-----|
| 纯 bugfix（不涉及行为描述变更） | 修复了内部逻辑错误，不改变用户可见行为 |
| 纯重构（不改变外部行为） | 代码重组、性能优化，对外接口不变 |
| 纯测试补充 | 新增测试用例、修复测试，不涉及文档描述的功能 |
| 文档自身更新 | 直接修改文档内容，不需要额外说明 |
| CI/构建工具链变更 | 修改 CI 配置、构建脚本、依赖版本 |

### 自查流程

```
我的 PR 改了代码
  ├─ 是否改变了用户可见的行为？
  │   ├─ 是 → 需要更新文档 → 在 PR template 勾选"已更新"
  │   └─ 否 → 进入下一步
  ├─ 是否新增了公开 API？
  │   ├─ 是 → 需要更新文档 → 在 PR template 勾选"已更新"
  │   └─ 否 → 进入下一步
  └─ 以上都不符合 → 在 PR template 勾选"不影响文档"
```

> **注意**：如果选择了"影响文档但未更新"，CI 会 block 并要求补充理由。

---

## 贡献者许可协议（CLA）

在你的第一个 Pull Request 被合并之前，你需要签署我们的[贡献者许可协议（CLA）](CLA.md)。

CLA Assistant bot 会自动在你的 PR 下留言，引导你完成签署。

**CLA 覆盖的内容**：

| 内容 |
| --- |
| 授权项目使用你的贡献的版权许可 |
| 授权你的贡献可能涉及的专利许可 |
| 确认你的作品是原创的，且不侵犯第三方权利 |

---

## 代码审查

### 审查要点

| 要点 |
| --- |
| 代码功能正确 |
| 符合代码规范 |
| 有适当的测试 |
| 文档已更新（参见[文档更新检查清单](#文档更新检查清单)） |
| 没有引入性能回退 |

### 响应反馈

| 做法 |
| --- |
| 及时回复审查意见 |
| 解释您的设计决策 |
| 愿意接受合理的建议 |

---

## 社区资源

- GitHub Issues：报告问题
- GitHub Discussions：讨论交流
- 项目文档

---

## 行为准则

本项目遵循我们的[行为准则](CODE_OF_CONDUCT.md)。
贡献社区时请保持尊重和友善。

---

> 再次感谢您的贡献！
>
> 如有问题，欢迎在 GitHub Discussions 中讨论。
