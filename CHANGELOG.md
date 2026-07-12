# 📦 v0.7.8

| 项目 | 值 |
| ---- | ----- |
| 发布日期 | 2026-07-12 |
| 版本变更 | `v0.7.7` → `v0.7.8` |
| 提交数 | 42 个 commit |

## 📋 本次更新概要

本次发版以 ConstExpr 约束表达式系统为核心，完成了 `Assert(N > 0)` 泛型约束从解析到求值再到单态化的全链路实现，并将断言机制（assert/Assert）统一为一个精化原语的两面。配套完成了类型检查器 Layer 0/2 重构、Formatter validate_source 统一入口、以及 Monomorphizer 泛型单态化重构。大量文档同步更新了 RFC 与语言规范中 Never/Void 语义与断言设计的修订。

## ✨ 新功能

### ConstExpr 约束表达式系统（Assert 泛型约束）

`Assert(N > 0)` 泛型约束全链路实现：解析器扩展 Assert 语法、类型检查器 Layer 2 约束求值、单态化层处理 ConstExpr 传递，并配套端到端集成测试。

- AST 新增 `Type::ConstExpr(Box<Expr>)` 变体
- `convert_expr_to_const_expr` 转换函数支持字面量与运算
- Layer 2 constraint evaluation（`ConstGenericEval`）
- `validate_const_args` 泛型参数约束验证
- `constraints` 字段加入 `ConstVarDef`
- `check_const_bounds` fast-slow path 重构
- 单态化层 `ConstExpr` 传递处理
- `tests/yaoxiang/02-type-system/const_generic_assert.yx` 集成测试

### Formatter 增强

- 新增 `--dry-run` / `--no-verify` 参数
- 统一验证入口 `validate_source`，替代分散的格式验证逻辑
- `FormatError` 结构化错误类型，`verify` 字段标记验证失败
- 导入排序后注释与空行处理修复
- 链式调用换行格式统一

### CLI 与工具

- 完善 `yaoxiang eval` 命令
- RFC 元数据同步工具与多语言翻译支持

### 测试

- validate 测试迁移至独立文件，补充断言消息
- CLI 子命令集成测试套件

## 🐛 Bug 修复

### LSP

- 适配 lsp-server 0.9.0 Response API 变更（`response_kind` 替代 `result`/`error` 字段）

### Formatter

- 修复导入排序后注释丢失与空行错乱

## ♻️ 重构优化

### 类型检查器

- `is_subtype` 提取至 Layer 0，删除死代码 subtyping 与 compatibility
- 泛型推断模块移除，统一测试辅助函数
- `check_single_module` 统一为 `validate_source`

### 单态化

- 泛型类型单态化重构，`generic_types` / `monomorphized_types` 字段拆分

### 代码清理

- 删除废弃 ShareRef IR 指令及相关空行
- 删除未完成解构赋值 stub
- crossbeam-epoch 安全漏洞升级

## 📝 文档

### 语言规范

- `std.assert` 模块文档（运行时 assert + 编译期 Assert）
- 类型系统新增 §8.3 Assert 精化类型章节，Never/Void Curry-Howard 对应
- 语法规范补充标识符三层体系（关键字 / 字面量保留字 / 内建类型名）
- README 特性对比表更新

### RFC

- RFC-030 全面重构：assert 定位为 Assert 的值引入子，dispatch 分派管道
- RFC-010 新增内建类型名表
- RFC-011 新增 Never/Void 语义，移除重复 Assert 定义
- RFC-027 新增 dispatch 分派管道，编译期谓词擦除改为分派
- RFC 跟踪表新增 RFC-034/035 条目
- 新增 `assert-unification-discussion.md` blog 记录六小时设计讨论

## 🔧 其他变更

- commit-msg 校验器新增 `rfc` scope
- dead-code clippy 告警抑制
- pycache 历史文件清理

## 📋 提交记录

| Hash | 描述 |
| :---: | ----- |
| `c05c38b` | :memo: docs(design): 补充标识符三层体系说明 |
| `59efa7c` | :memo: docs(docs): 记录 assert/Assert 统一方案的六小时讨论过程 |
| `0765296` | :memo: docs(rfc): 更新 RFC-010/011/027 与 RFC-030 的 assert/Assert 统一设计 |
| `ff80cb9` | :memo: docs(design): 补充 Never/Void 类型定义与 assert/Assert 统一文档 |
| `0e8ec14` | :pencil: docs: auto-translate documentation |
| `62df06d` | :memo: docs(rfc): 重构 assert 机制设计方案 |
| `0dce985` | :white_check_mark: test(frontend): 迁移 validate 测试到独立文件，补充断言消息 |
| `ce18620` | :bug: fix(lsp): 适配 lsp-server 0.9.0 Response API 变更 |
| `8f16b56` | :white_check_mark: test(formatter): 添加语义验证集成测试 |
| `2ad85c7` | :sparkles: feat(formatter): dry-run / no-verify 参数 |
| `93caac7` | :recycle: refactor(util): check_single_module 使用 validate_source |
| `3590023` | :recycle: refactor(formatter): integrate validate_source, add FormatError, verify field |
| `89418b1` | :pencil: docs: auto-translate documentation |
| `90ceaf3` | :sparkles: feat(frontend): 添加 validate_source 前端验证统一入口 |
| `584cc76` | :arrow_up: chore(deps): Bump the production-dependencies group |
| `b0c05c2` | :wrench: chore(build): 将 rfc 加入合法 scope 列表 |
| `6cfda7b` | :memo: docs(docs): 更新 RFC 跟踪表，新增 RFC-034/035 |
| `6987f8f` | :sparkles: feat(types): 支持 ConstExpr 约束表达式系统 |
| `eeb5745` | :recycle: refactor(typecheck): extract is_subtype to Layer 0 |
| `c6dd7d9` | :sparkles: feat(typecheck): implement Layer 2 constraint evaluation |
| `9a47c2b` | :white_check_mark: test(typecheck): add check_const_bounds fast-path tests |
| `92c9b33` | :recycle: refactor(typecheck): rewrite check_const_bounds with fast-slow path |
| `0437c9b` | :sparkles: feat(typecheck): extract const constraints from struct body |
| `21386b9` | :sparkles: feat(types): add constraints field to ConstVarDef |
| `76023a7` | :recycle: refactor(typecheck): integrate const params via PolyType |
| `bf65539` | :sparkles: feat(typecheck): add validate_const_args |
| `6f104fd` | :wrench: chore(monomorphize): suppress dead-code clippy warnings |
| `6c8623e` | :sparkles: feat(types): add ConstKind::from_ast_type_name |
| `2b36c8d` | :pencil: docs: auto-translate documentation |
| `d1d7119` | :memo: docs(design): RFC 011 注册关联 issue #151 |
| `563388a` | :recycle: refactor(monomorphize): 重构泛型类型单态化 |
| `6adc299` | :sparkles: feat(monomorphize): 添加 generic_types 和 monomorphized_types |
| `bdf2020` | :fire: chore(typecheck): 删除未完成的解构赋值 stub |
| `639320d` | :wrench: chore(build): 升级 crossbeam-epoch 修复安全漏洞 |
| `866c4b6` | :recycle: refactor(typecheck): 移除泛型推断模块 |
| `8d359ac` | :art: style(formatter): 链式调用换行格式统一 |
| `35fc47e` | :wrench: chore(codegen): 清理 ShareRef 删除后的多余空行 |
| `0ea3d06` | :sparkles: feat(formatter): 修复导入排序后注释与空行处理 |
| `8d57dc6` | :fire: chore(middle): 删除废弃的 ShareRef IR 指令 |
| `6c31f4a` | :white_check_mark: test(package): 新增 CLI 子命令集成测试套件 |
| `ff2b42f` | :pencil: docs: auto-translate documentation |
| `d91f58f` | :memo: docs(design): 新增 RFC-034 统一调试工具链规范 |
| `48bf6e2` | :pencil: docs: auto-translate documentation |
| `cf8dff0` | :sparkles: feat(backends): 完善 yaoxiang eval 命令 |
| `1d402a0` | :pencil: docs: auto-translate documentation |
| `de1a26c` | :fire: chore(meta): 移除历史 pycache 文件 |
| `092a845` | :sparkles: feat(build): 新增 RFC 元数据同步工具与多语言翻译 |
| `aa07d5d` | :memo: docs(design): 同步 RFC 元数据与 GitHub Issue 引用 |
