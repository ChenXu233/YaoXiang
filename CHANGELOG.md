# Changelog

## :bookmark: V0.7.9: Never 内建类型与类型系统深化

> 发布日期: 2026-07-21

### 📦 版本信息

| 项目     | 值                |
| -------- | ----------------- |
| 发布日期 | 2026-07-21        |
| 版本变更 | `0.7.8` → `0.7.9` |
| 提交数   | 56 个 commit      |

### 📋 本次更新概要

本次发版是类型系统的一次重大深化。Never 内建类型正式引入（爆炸原则、子类型判定、trait 实现），宇宙分层 predicative 弱检查和类型级递归终止性检查为泛型系统提供了更坚实的理论基础。流敏感假设集替代旧栈式 AssumptionStack 实现了更精确的 Γ 推导，const 泛型约束新增 `<` 和 `<=` 运算符支持。同时 std.assert 统一注册机制、TextMate grammar 重写和 formatter 泛型签名重建等改进进一步提升了开发体验。

### ✨ 新功能

#### Never 内建类型

正式引入 Never 底类型，完整实现爆炸原则（ex falso quodlibet）、子类型判定（Never 是任何类型的子类型）和 trait impl 注册。Never 在所有解析点正确解析，支持 const_eval size 计算，与 Void 类型形成了完整的底层类型体系。

- 新增 MonoType::Never 变体及穷举匹配 arm
- 注册为类型检查环境的内建类型
- 所有解析点正确解析 Never/never → MonoType::Never
- 实现爆炸原则、trait impls、const_eval size 支持
- 修复 trait_key、structurally_equal、unify 三处核心路径

#### IsTrue 类型族桥接

实现 IsTrue 类型族，将 `true` 映射到 `Void`、`false` 映射到 `Never`，为编译期条件判定提供了统一的类型级表达。

#### const 泛型约束运算符

parser 全面支持 const 泛型约束中的 `<` 和 `<=` 比较运算符，新增 E1062/W1063 错误码覆盖约束检查。

#### 流敏感假设集 Γ

流敏感假设集 Γ 配合 kill set 完全替换了旧的栈式 AssumptionStack，实现了更精确的类型推导路径追踪。

#### 宇宙分层与类型级递归

- 宇宙分层 predicative 弱检查 — Typeₙ <: Typeₘ 当 n ≤ m
- 类型级递归检测 + 结构递归终止性检查（AssociatedTypeDef::Recursive）
- dispatch 分派管道接线 — CompileTime/Runtime + mut kill

#### TextMate grammar 重构

lexer 层重构 TextMate grammar，完整支持 fstrings 和全部类型的高亮标记，配合新的构建流水线实现自动同步。

#### std.assert 统一注册

StdModule trait 扩展 type_families 方法，std.assert 模块通过统一注册机制接入，dispatch 分派管道支持 CompileTime/Runtime 双路径。

### 🐛 Bug 修复

#### parser 值参数处理

修复值参数过滤统一大小写规则导致的 Const 泛型错位问题，修正 merged 状态下的错位并移除了临时的清洗补丁代码。

#### formatter ConstExpr 还原

修复 ConstExpr 还原与类型体保序格式化问题，确保格式化输出与解析输入一致。

#### Never 子类型修复

修复 Never 在 trait_key、structurally_equal、unify 三条核心路径的错误 — 分别补全 MonoType::Never arm、对称递归判定和统一分支。

#### yx_runner 兼容性

将编译错误测试移至 06 目录，运行时失败标记 skip，确保 yx_runner 兼容运行。

### ♻️ 重构优化

#### 类型体与约束分离

Type::Struct 改为有序 TypeBodyItem，约束字段与普通字段分离 — Assert 走 constraints 路径不混入 fields。ConstExpr 统一到 const_data，StructType.constraints 和 Assert 硬编码删除。

#### 泛型实例化路径统一

统一泛型实例化路径，Layer 2 约束检查接管所有分支。消除 clippy match 简化警告，StdModule trait 扩展 type_families。

#### Binding 签名参数

Binding 的签名参数统一存储到 signature_params，消除了重复构造和双路径问题。

### ✅ 测试改进

- formatter 泛型类型定义 round-trip 集成用例
- parser const 约束 < 与 <= E2E 用例
- typecheck gamma_assume 测试补规范引用与 AAA 分段
- assert 运行时通过和 panic 的 E2E 测试
- 测试合规修复 — inline tests 拆出 + AAA + 断言消息 + 规范头
- IsTrue 测试合规修复
- Never 新增子类型/解析/条件判定三组测试

### 📝 文档

- 同步文档至统一声明语法
- 同步英文翻译并清理项目结构章节
- 重写模块语义 RFC-029
- RFC-011 类型体代码块与效应种子已实现
- RFC-030 assert 断言机制移入已接受并标注实现
- 标注 assert/Assert 统一方案 6 Phase 已实现 — #157-#162 已关闭
- 升级 Star History 图表支持暗黑模式
- 自动翻译文档同步

### 🔧 其他变更

- TextMate grammar 同步构建流水线
- 清理废弃的教程示例与更新 formatter 设计文档
- Bump production-dependencies (regex, toml, clap, tokio, lsp-server)
- 清理 Python 测试脚本中未使用的导入和格式问题
- 添加版本号 badge 自动同步 hook
- i18n 自动翻译 locale 文件

### 📝 提交记录

|   Hash    | 描述                                                                  |
| :-------: | --------------------------------------------------------------------- |
| `4b886b6` | feat(types): Never 内建类型 — add MonoType::Never variant with exhaustiveness arms |
| `490f523` | feat(types): Never 内建类型 — register as builtin type in typecheck environment |
| `ef6647c` | feat(types): Never 内建类型 — resolve Never/never to MonoType::Never at all parse points |
| `c502dd9` | feat(types): Never 内建类型 — add explosion principle, trait impls, and const_eval size support |
| `552c47f` | fix(types): Never — trait_key structural_equal unify 三处修复 |
| `c74fbc1` | test(types): Never — 新增子类型/解析/条件判定三组测试 |
| `db25f6c` | feat(types): 实现 IsTrue 类型族桥接 — true→Void false→Never |
| `a5c1554` | test(types): IsTrue — 测试合规修复 |
| `5e0eace` | feat(proof): 流敏感假设集 Γ + kill set — 替换纯栈 AssumptionStack |
| `c820535` | feat(typecheck): dispatch 分派管道接线 — CompileTime/Runtime + mut kill |
| `5a7e576` | feat(types): 宇宙分层 predicative 弱检查 — Typeₙ<:Typeₘ 当 n≤m |
| `90e6772` | feat(types): 类型级递归 + 结构递归终止性 — AssociatedTypeDef::Recursive |
| `56828f6` | feat(typecheck): 新增 E1062/W1063 错误码 — const 泛型约束 |
| `3b233b0` | refactor(typecheck): 统一泛型实例化路径 — Layer 2 约束检查 |
| `d340240` | feat(parser): 支持 const 泛型约束中的 < 和 <= 比较运算符 |
| `c6757f1` | test(parser): const 约束 < 与 <= E2E 用例 |
| `24e5f17` | refactor(typecheck): Type::Struct 分离约束字段 — Assert 走 constraints 不混入 fields |
| `6af2de1` | refactor(parser): Type::Struct 改为有序 TypeBodyItem |
| `63a8268` | refactor(types): 删 StructType.constraints 与 Assert 硬编码 |
| `f39daf4` | refactor(types): 统一 ConstExpr 到 const_data |
| `4ba6c52` | feat(typecheck): 顺序处理类型体收集 const 约束 |
| `b304574` | feat(typecheck): 效应种子 GammaAssume 接入 Γ |
| `24ff47f` | test(typecheck): gamma_assume 测试补规范引用与 AAA 分段 |
| `c96e5f0` | test(typecheck): 测试合规修复 — inline tests 拆出 + AAA + 断言消息 + 规范头 |
| `210e603` | docs(rfc): RFC-011 类型体代码块与效应种子已实现 |
| `1caa3ca` | refactor(std): StdModule trait 扩展 type_families + std.assert 模块 |
| `6a6985a` | refactor(typecheck): 清除硬编码 + 修复空 dep_env — 走 std.assert 正路 |
| `09c4f78` | test(test): assert 运行时通过和 panic 的 E2E 测试 |
| `616b265` | docs(design): RFC-030 标注 std.assert 统一注册已实现 — #169 已关闭 |
| `08f4a38` | docs(docs): 将 RFC-030 assert 断言机制移入已接受 |
| `abe8a8e` | docs(rfc): 标注 assert/Assert 统一方案 6 Phase 已实现 — #157-#162 已关闭 |
| `c9ff2f5` | fix(test): yx_runner 兼容 — 编译错误测试移至 06 + 运行时失败标记 skip |
| `089a1b8` | refactor(parser): Binding 签名参数统一存储 signature_params |
| `457b54b` | feat(formatter): 函数签名统一带名输出并删除双补丁函数 |
| `deb344f` | feat(formatter): 泛型类型定义重建 RFC-010 函数式签名 |
| `c04990b` | test(formatter): 泛型类型定义 round-trip 集成用例 |
| `75b588b` | fix(formatter): ConstExpr 还原与类型体保序格式化 |
| `123b66f` | fix(parser): 值参数过滤统一大小写规则修复 Const 泛型错位 |
| `8e6708d` | fix(parser): 修正值参数 merged 错位并移除清洗补丁 |
| `9e38255` | feat(lexer): 重构 TextMate grammar 支持 fstrings 和完整类型高亮 |
| `2fde7c1` | build(docs): 新增 TextMate grammar 同步构建流水线 |
| `6411127` | refactor(typecheck): 消除 clippy match 简化警告 |
| `9cd4990` | chore(docs): 清理废弃的教程示例与更新 formatter 设计文档 |
| `0b0e5bc` | docs(docs): 同步文档至统一声明语法 |
| `4f5e302` | docs: auto-translate documentation |
| `c7b5613` | docs: auto-translate documentation |
| `5b33ef6` | docs: auto-translate documentation |
| `0bab085` | docs: auto-translate documentation |
| `a3969dc` | docs: auto-translate documentation |
| `261126a` | chore(deps): Bump production-dependencies |
| `68d9ef2` | docs(design): 重写模块语义 RFC-029 |
| `2454ceb` | docs(docs): 同步英文翻译并清理项目结构章节 |
| `3795a00` | style(test): 清理 Python 测试脚本中未使用的导入和格式问题 |
| `7d23914` | chore(build): 添加版本号 badge 自动同步 hook |
| `3fe5b82` | docs(docs): 升级 Star History 图表支持暗黑模式 |
| `3afa123` | i18n: auto-translate locale files |
