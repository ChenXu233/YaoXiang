## 变更描述

<!-- 简要描述这个 PR 做了什么 -->

## 职责归属与决策程序（必填，D0–D4）

> 规则本体：[coding-rules.md](../docs/src/dev/coding-rules.md)；入口：[HOWTO.md](../docs/src/design/compiler-architecture/HOWTO.md) / [CONTRIBUTING.md](../CONTRIBUTING.md)。**写不出 D0 权威源模块名 = 没查，review 直接打回。**

- **职责归属**（必填文字）：本次新增代码属于哪个目录的哪条职责？（引用 [01-routing 职责表](../docs/src/design/compiler-architecture/01-routing.md)）
  <!-- 例：sema/check/ —— 模块级检查 · 注解类型名校验 -->
  
- **D0**（必填文字）：本次是否触碰任何表（错误码 / opcode / 类型 / 阶段）？若是，该表的唯一权威实现模块是：
  <!-- 写"无"或模块路径，如 tools/code-tables -->
  
- **D3 补丁判定**（勾选，命中任一条则必须附设计文档链接）：
  - [ ] 同一行为需要在 ≥2 处复制
  - [ ] 新增了"第 N 个入口 / 接线点"
  - [ ] 一次修改要同步改 ≥3 处同义映射
  - [ ] 以上皆否（局部补丁已附回归测试）
- [ ] 我不是在做"不看实际情况的补丁式修复"：我已打开被改文件阅读上下文，并 grep 过相关引用点
- [ ] 若本 PR 属于 RFC-039 施工，我已核对 [09-execution-wbs.md](../docs/src/design/compiler-architecture/09-execution-wbs.md) 中对应三级任务的前置与验收判据

## 文档影响评估

> AI reviewer 会自动解析此区块。请勾选一项。

- [ ] 此 PR 不影响文档（纯 bugfix、重构、测试、文档自身更新）
- [ ] 此 PR 影响文档，我已更新相关文档（列出更新路径）
- [ ] 此 PR 影响文档，但我未更新文档（需说明理由，会被 block）

**如果选择了"影响文档并已更新"，请列出更新/创建的文件：**
<!-- 例：docs/src/guide/xxx.md -->
- `docs/src/...`

## 变更类型

<!-- 勾选适用的类型 -->

- [ ] ✨ feat: 新功能
- [ ] 🐛 fix: Bug 修复
- [ ] ♻️ refactor: 重构（无功能变更）
- [ ] 📝 docs: 文档更新
- [ ] 🔧 chore: 构建/CI/工具链
- [ ] ✅ test: 测试补充
- [ ] ⚡ perf: 性能优化

## 测试说明

<!-- 描述你如何验证这个变更 -->

- [ ] 本地测试通过
- [ ] 新增/修改了对应测试用例
- [ ] 无破坏性变更

## 关联 Issue

<!-- 用 "Closes #123" 或 "Relates to #123" 关联 -->

## Checklist

- [ ] `cargo fmt` 已执行
- [ ] `cargo clippy` 无新增 warning
- [ ] `cargo test` 全部通过
- [ ] 变更不破坏现有用户可见行为（Never break userspace）
