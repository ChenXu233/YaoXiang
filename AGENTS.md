# YaoXiang 仓库指南（人类与 AI 实现者共用）

YaoXiang 是一门编程语言及其编译器（Rust 实现）。`src/` 约 16 万行，正按 [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) 做架构重构（四层模型：编排/前端/语义与 IR/执行）。

## 改代码之前（强制）

1. **读 [实现者手册 HOWTO.md](docs/src/dev/HOWTO.md)**——动工前自检表 + “补丁式修复”判定（D3）。它是入口；规则本体在 [coding-rules.md](docs/src/dev/coding-rules.md)。
2. 施工任务查 [09-execution-wbs.md](docs/src/dev/architecture/09-execution-wbs.md) 的三级任务表（带 `- [ ]` 看板）。
3. 三条禁令（概要，全文见 [coding-rules.md](docs/src/dev/coding-rules.md)）：**不得生造**（新概念先证明与既有概念不重复）、**不得职责累积**（一个模块一类职责）、**该重构不补丁**（同一行为 ≥2 处复制 / 新增第 N 个入口 / 一次改 ≥3 处同义映射 → 停工走设计流程）。

## 常用命令

```bash
cargo fmt                          # 格式化（提交前必跑）
cargo clippy --all --all-features -- -D warnings
cargo test                         # 测试；测试数只许增不许减
python scripts/rfc/check_tracking.py   # RFC 状态一致性
```

提交与 PR 规范：`docs/src/dev/commit-convention.md`；PR 模板含“职责归属与决策程序”必填块（D0–D4），写不出权威源模块名就是没查，review 会打回。

## 红线（review 必打回）

- 不看代码就改：没打开文件、没 grep 过引用点，不许动
- 核心功能留 `todo!()` / “Not implemented yet” / 无限期“独立 issue”
- 删测试或放宽判据换绿灯（不存在 C5′；做不到是实现缺陷，如实报告）
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
| [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md) | 总纲、决议登记 D1–D52 |
