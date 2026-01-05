# Contributing Guide / 贡献指南

> Thank you for your interest in YaoXiang! We welcome contributions of all kinds.
> 感谢您对 YaoXiang 项目的兴趣！我们欢迎各种形式的贡献。

---

## Table of Contents / 目录

- [Ways to Contribute / 贡献方式](#ways-to-contribute--贡献方式)
- [Getting Started / 快速开始](#getting-started--快速开始)
- [Submitting Changes / 提交流程](#submitting-changes--提交流程)
- [RFC Process / 设计提案流程](#rfc-process--设计提案流程)
- [Code Standards / 代码规范](#code-standards--代码规范)
- [Code Review / 代码审查](#code-review--代码审查)
- [Community Resources / 社区资源](#community-resources--社区资源)
- [Code of Conduct / 行为准则](#code-of-conduct--行为准则)

---

## Ways to Contribute / 贡献方式

| English | 中文 |
|---------|------|
| Report bugs in GitHub Issues | 在 GitHub Issues 中报告问题 |
| Propose new features or designs | 功能建议或设计讨论 |
| Write or improve documentation | 改进文档或撰写教程 |
| Submit code fixes or new features | 修复问题或实现新功能 |
| Help with design (logo, UI, etc.) | 语言设计、Logo、UI |

---

## Getting Started / 快速开始

### Prerequisites / 环境准备

```bash
# Install Rust (recommended: rustup) / 安装 Rust（建议使用 rustup）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository / 克隆项目
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# Build the project / 构建项目
cargo build --release

# Run tests / 运行测试
cargo test
```

### Code Style / 代码风格

```bash
# Format code / 格式化代码
cargo fmt

# Type checking / 类型检查
cargo check

# Run all checks / 运行所有检查
cargo clippy
```

---

## Submitting Changes / 提交流程

### 1. Create a Branch / 创建分支

```bash
git checkout -b feature/your-feature-name
```

### 2. Commit Convention / 提交规范

Follow the [commit convention](docs/guides/dev/commit-convention.md):
遵循 [提交规范](docs/guides/dev/commit-convention.md)：

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types / 类型**：

| Type | English | 中文 |
|------|---------|------|
| `feat` | New feature | 新功能 |
| `fix` | Bug fix | Bug 修复 |
| `docs` | Documentation changes | 文档更新 |
| `style` | Code formatting (no functional changes) | 代码格式（不影响功能） |
| `refactor` | Code refactoring | 重构代码 |
| `test` | Add or modify tests | 添加测试 |
| `chore` | Build tool or auxiliary changes | 构建工具或辅助功能更新 |

**Examples / 示例**：

```
feat(frontend): Add type inference feature

Implemented basic polymorphic type inference algorithm.

Closes #123
```

```
feat(frontend): 添加类型推断功能

实现了基本的多态类型推断算法。

Closes #123
```

### 3. Submit PR / 提交 PR

1. Push your branch: `git push origin feature/your-feature-name`
2. 推送分支：`git push origin feature/your-feature-name`
3. Visit GitHub to create a Pull Request
4. 访问 GitHub 创建 Pull Request
5. Fill in the PR template
6. 填写 PR 模板
7. Wait for code review
8. 等待代码审查

---

## RFC Process / 设计提案流程

### Submit an RFC / 提交 RFC

For new features or major changes, please submit an RFC first:
对于新功能或重大变更，请先提交 RFC：

1. Read the [RFC Template](docs/design/rfc/RFC_TEMPLATE.md)
2. 阅读 [RFC 模板](docs/design/rfc/RFC_TEMPLATE.md)
3. Reference the [Full Example](docs/design/rfc/EXAMPLE_full_feature_proposal.md)
4. 参考 [完整示例](docs/design/rfc/EXAMPLE_full_feature_proposal.md)
5. Create a new RFC file in `docs/design/rfc/`
6. 在 `docs/design/rfc/` 目录创建新 RFC 文件
7. Set status to "Draft" (草案) or "Review" (审核中)
8. 状态设为 "草案" 或 "审核中"
9. Submit a PR for discussion
10. 提交 PR 进行讨论

### RFC Lifecycle / RFC 状态流转

```
Draft (草案) → Review (审核中) → Accepted (已接受) → accepted/
                                   → Rejected (已拒绝) → stays in rfc/
```

See [RFC Lifecycle](docs/design/rfc/RFC_TEMPLATE.md#lifecycle) for details.
详见 [RFC 生命周期](docs/design/rfc/RFC_TEMPLATE.md#生命周期与归宿)。

---

## Code Standards / 代码规范

### Rust Code / Rust 代码

- Follow `rustfmt` default formatting / 遵循 `rustfmt` 默认格式
- Use `clippy` for linting / 使用 `clippy` 进行静态检查
- Add appropriate comments and documentation / 添加适当的注释和文档
- Write rustdoc for public APIs / 为公开 API 编写 rustdoc

### Documentation / 文档

- Use Chinese headings / 使用中文标题
- Mark code blocks with language / 代码块标注语言
- Keep terminology consistent / 术语保持一致
- Follow [Documentation Maintenance](docs/maintenance/MAINTENANCE.md)

### Tests / 测试

- Add unit tests for new features / 为新功能添加单元测试
- Update integration tests if needed / 更新集成测试（如果需要）
- Ensure all tests pass / 确保所有测试通过

---

## Code Review / 代码审查

### Review Checklist / 审查要点

| English | 中文 |
|---------|------|
| Code is functionally correct | 代码功能正确 |
| Follows code standards | 符合代码规范 |
| Has appropriate tests | 有适当的测试 |
| Documentation is updated | 文档已更新 |
| No performance regression | 没有引入性能回退 |

### Responding to Feedback / 响应反馈

| English | 中文 |
|---------|------|
| Reply to review comments in a timely manner | 及时回复审查意见 |
| Explain your design decisions | 解释您的设计决策 |
| Be open to reasonable suggestions | 愿意接受合理的建议 |

---

## Community Resources / 社区资源

| English | 中文 |
|---------|------|
| GitHub Issues: Report bugs | GitHub Issues: 报告问题 |
| GitHub Discussions: Community chat | GitHub Discussions: 讨论交流 |
| Project Documentation | 项目文档 |

---

## Code of Conduct / 行为准则

This project follows our [Code of Conduct](CODE_OF_CONDUCT.md).
Please be respectful and friendly when contributing to our community.

本项目遵循我们的[行为准则](CODE_OF_CONDUCT.md)。
贡献社区时请保持尊重和友善。

---

> Thank you for your contribution! / 再次感谢您的贡献！
>
> For questions, feel free to discuss in GitHub Discussions. / 如有问题，欢迎在 GitHub Discussions 中讨论。
