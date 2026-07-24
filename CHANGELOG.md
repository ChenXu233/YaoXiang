# Changelog

## :bookmark: V0.7.10: 类型空间收敛与单态化清理

> 发布日期: 2026-07-25

### 📦 版本信息

| 项目     | 值                       |
| -------- | ------------------------ |
| 发布日期 | 2026-07-25               |
| 版本变更 | `0.7.9` → `0.7.10`       |
| 提交数   | 71 个 commit（无 merge） |

### 📋 本次更新概要

本次发版聚焦类型系统内部的清理与收敛。typecheck 引入**语义 base 解析原语**（`resolve_base_kind`），方法注册从 `StructType.methods` 双存储收敛到 `method_bindings` 单一源；monomorphize **删除死代码子系统**并引入新扫描阶段（`feat(monomorphize): 收集和扫描泛型类型引用`），配合 E2E 测试覆盖。typecheck 端消除 **8 处静默兜底**（`fix(typecheck): 消除 8 处静默兜底`），修复 Float→Int 隐式转换拦截、curried 泛型 return 块等关键类型检查漏洞，CI 重新回到绿色。测试规范升级 —— `.yx` 文件统一从 `io.println` 切换到 `assert.assert` 断言（`refactor(test): .yx 测试文件从 io.println 切换到 assert.assert`），文档站多语言同步更新。

### ✨ 新功能

#### 语义 base 解析与类型空间收敛

typecheck 引入 `resolve_base_kind` 原语，方法解析和鸭子类型约束统一查 `StructType.methods` 类型空间。配套重构撤销了 `StructType.methods` 双存储分裂、让方法解析优先走类型空间、把登记侧按 base 语义分流。整套改动是 typecheck 内部架构统一的关键一步，对用户 API 无破坏。

- `feat(typecheck): 加 resolve_base_kind 语义 base 解析原语`
- `feat(typecheck): 语义 base 解析原语 + 类型空间落 StructType.methods`
- `feat(typecheck): 登记侧按 base 语义分流类型空间`
- `refactor(typecheck): 方法解析优先查 StructType.methods 类型空间`
- `refactor(typecheck): 鸭子类型约束查 StructType.methods`
- `refactor(typecheck): 撤销 StructType.methods 双存储，方法回归 method_bindings 单一源`

#### monomorphize 收集与扫描阶段

monomorphize 引入收集和扫描泛型类型引用的新阶段，为后续单态化接入做准备。配套 4 个测试提交（单元测试 + E2E + 补充覆盖 + 合规修复）保证新阶段的可靠性。

- `feat(monomorphize): 收集和扫描泛型类型引用`
- `test(monomorphize): 泛型类型单态化单元测试`
- `test(monomorphize): 泛型类型端到端测试`
- `test(monomorphize): 补充泛型类型单态化测试覆盖`
- `test(monomorphize): 泛型类型测试合规修复`

#### 值空间字段赋值 schema 校验

typecheck 在值空间字段赋值处增加 schema 校验，杜绝运行时才能发现的字段拼写错误。

- `feat(typecheck): 值空间字段赋值 schema 校验`

#### const 泛型扩展

parser 支持 const 泛型中的算术运算符，typecheck 实现函数级 const 泛型（RFC-011），为编译期常量计算打开新路径。

- `feat(parser): parse_type_annotation ConstExpr 支持算术运算符`
- `feat(typecheck): 实现函数级 const 泛型 (RFC-011)`

#### 类型构造器精化

typecheck 精化类型构造器，统一 `{}` 语义和 proof 函数，消解类型构造器多种用法引起的歧义。

- `feat(typecheck): 精化类型构造器——{}统一语义与 proof 函数`

### 🐛 Bug 修复

#### 静默兜底消除

`fix(typecheck): 消除 8 处静默兜底` 是本版最重要的修复之一 —— checker、ir_gen、translator、debug、normalizer 五条核心路径上 8 处悄悄把错误吞掉的代码被替换为显式错误信号，让开发期问题暴露更早。

- `fix(typecheck): 消除 8 处静默兜底`
- `fix(typecheck): 修复 doc_lazy_continuation 告警`
- `fix(middle): 修复局部变量持函数值调用失败`

#### 类型检查漏洞

修复多种类型检查漏洞：Float→Int 隐式转换拦截（拒绝精度丢失）、curried 泛型 return 块形式类型检查、Never 函数循环放行与 const 泛型 curry 格式化、From<ast::Type> 内建类型 bypass 拦截、resolve-then-check 替换盲跳过、void 字面量在表达式位置解析失败、纯函数声明。

- `fix(typecheck): 函数调用参数 Float→Int 隐式转换拦截`
- `fix(typecheck): curried 泛型函数 return 块形式类型检查`
- `fix(typecheck): Never 函数循环放行 + const 泛型 curry 格式化`
- `fix: resolve builtin type names in From<ast::Type> to prevent TypeRef bypass`
- `fix: replace blind TypeRef skip with resolve-then-check in function call args`
- `fix(parser): void 字面量在表达式位置解析失败`
- `fix(parser): 纯函数声明 (#168)`

#### CI / 构建 / 文档

- `fix(ci): 修复 8 个 clippy 错误使 CI 回归绿色`
- `fix(meta): clippy 冗余模式匹配`
- `fix(meta): run 命令编译失败时显式 exit(1) 确保非零退出码`
- `fix(docs): 删除 vitepress 配置中多余的右括号`

### ♻️ 重构优化

#### monomorphize 死代码清理

删除 monomorphize 死掉的泛型类型单态化子系统 —— 旧实现未被任何代码路径调用、保留只会增加维护负担。

- `refactor(monomorphize): 删除死掉的泛型类型单态化子系统`

#### 赋值统一

typecheck 引入 `Assign` 统一替代 `Var/Binding/ExternalBindingStmt` 三种赋值节点，简化赋值路径，配套完成"三个凑合修复重构"（消除状态标志与重复逻辑）。

- `refactor(typecheck): 赋值统一——Assign 替代 Var/Binding/ExternalBindingStmt`
- `refactor(typecheck): 三个凑合修复重构——消除状态标志与重复逻辑`

#### TypeRef 解析委托

类型解析统一委托给 `from_builtin_name`，移除多处硬编码 `TypeRef(Bool/Void)`，让内建类型解析走单一通路。

- `refactor: remove redundant inline TypeRef resolution in check_var_stmt`
- `refactor: delegate resolve_builtin_type_ref to from_builtin_name`
- `refactor: delegate resolve_type_ref_type builtins to from_builtin_name`
- `refactor: delegate resolve_type_refs leaf to from_builtin_name`
- `refactor: replace hardcoded TypeRef(Bool/Void) with concrete types`

#### FunctionBody 枚举统一

middle 层将 `FunctionBody` 枚举统一为类型定义和函数体共享的 IR 节点，让 codegen 不再分两条路径走。

- `refactor(middle): FunctionBody 枚举统一类型定义和函数体`
- `refactor(middle): 构造器识别查表化并清理死代码`

#### parser 清理

- `refactor(parser): 删除死代码 extract_generic_params`
- `refactor(parser): TypeDefinition 变体消除类型定义形状猜测`

#### 性能与代码风格

- `perf(list): pop/remove_at use heap.get_mut() instead of clone+write (#192)` — 列表原位修改不再 clone+write
- `style(formatter): cargo fmt——assert_eq! 参数分行`
- `style(typecheck): flatten nested match in function call resolver`

### ✅ 测试改进

#### .yx 测试文件 assert 化

按 `TEST_STANDARDS` 文档规范，`.yx` 测试文件从 `io.println` 切换到 `assert.assert` 断言。这是子代理扫尾工作的成果，3 个遗漏文件被修复。

- `refactor(test): .yx 测试文件从 io.println 切换到 assert.assert`
- `refactor(test): 修复子代理遗漏的 3 个 .yx 文件 + 更新 TEST_STANDARDS 文档`

#### 其他测试

- `test(test): 新增 curried 函数赋值后调用集成测试`
- `test(formatter): 测试合规——加 AAA 分段注释符合规范 §4.1`
- `test(parser): 修复测试规范违规——断言消息+命名+规范引用`
- `test: add failing tests for Float→Int function call argument`

### 📝 文档

- `docs(docs): 更新英文文档站内容与 i18n 缓存`
- `docs(docs): 同步日文和俄文文档站 i18n 翻译`
- `docs(docs): 清理过时文档并修复多处引用错误`
- `docs(meta): auto-dev-night 修复报告（撤出 #181，留待 PR #186）`
- `docs: auto-translate documentation`

### 🔧 其他变更

#### 依赖与 CI

- `chore(deps): Bump the production-dependencies group with 6 updates`
- `chore(build): 更新 production-dependencies 组 6 个依赖`
- `chore(ci): 所有 dependabot 更新指向 dev 分支`
- `ci(ci): 将版本徽章同步从本地钩子迁移至 CI 流程`
- `ci(workflows): 升级 GitHub Actions 版本至 Node.js 24`
- `chore(docs): 更新文档站构建依赖与站点配置`
- `refactor(build): 重组 scripts 目录结构`
- dependabot 自动 bump: `esbuild` / `vite`（scripts/i18n）、`brace-expansion` 2.0.2→2.1.2、`minimatch` 5.1.7→5.1.9

#### 文档清理

- `chore(docs): 移除过时的计划文档目录`

### 📝 提交记录

|   Hash    | 描述                                                                                       |
| :-------: | ------------------------------------------------------------------------------------------ |
| `7bdfd3b8` | test(monomorphize): 补充泛型类型单态化测试覆盖                                              |
| `198c1889` | test(monomorphize): 泛型类型测试合规修复                                                    |
| `e56adae8` | refactor(test): 修复子代理遗漏的 3 个 .yx 文件 + 更新 TEST_STANDARDS 文档                  |
| `babd1952` | refactor(test): .yx 测试文件从 io.println 切换到 assert.assert                              |
| `a448fb16` | test(monomorphize): 泛型类型端到端测试                                                      |
| `3098dd50` | test(monomorphize): 泛型类型单态化单元测试                                                  |
| `9cc66eed` | feat(monomorphize): 收集和扫描泛型类型引用                                                  |
| `fa137789` | refactor(middle): FunctionBody 枚举统一类型定义和函数体                                     |
| `046ad9c6` | chore(build): 更新 production-dependencies 组 6 个依赖                                      |
| `465370d3` | chore(deps): Bump the production-dependencies group with 6 updates                          |
| `3ec98aa5` | fix(typecheck): 修复 doc_lazy_continuation 告警                                             |
| `30f7ae63` | fix(typecheck): 消除 8 处静默兜底                                                          |
| `c6a9d95b` | test(test): 新增 curried 函数赋值后调用集成测试                                            |
| `afee9581` | fix(middle): 修复局部变量持函数值调用失败                                                  |
| `59d82cb8` | feat(parser): parse_type_annotation ConstExpr 支持算术运算符                                |
| `fed7b412` | refactor(monomorphize): 删除死掉的泛型类型单态化子系统                                      |
| `d8b62889` | refactor(typecheck): 撤销 StructType.methods 双存储                                         |
| `a6a8937b` | refactor(typecheck): 鸭子类型约束查 StructType.methods                                      |
| `70e2de20` | feat(typecheck): 值空间字段赋值 schema 校验                                                 |
| `c11eefc6` | refactor(typecheck): 方法解析优先查 StructType.methods 类型空间                             |
| `fbc61752` | feat(typecheck): 登记侧按 base 语义分流类型空间                                             |
| `4f2ed101` | feat(typecheck): 语义 base 解析原语 + 类型空间落 StructType.methods                         |
| `75ce5de2` | refactor(parser): 删除死代码 extract_generic_params                                         |
| `d0a37e4e` | feat(typecheck): 加 resolve_base_kind 语义 base 解析原语                                   |
| `7b103571` | build(deps): bump esbuild and vite in /scripts/i18n                                         |
| `f7f1cfe4` | build(deps): bump brace-expansion from 2.0.2 to 2.1.2                                       |
| `d159c1c3` | build(deps): bump minimatch from 5.1.7 to 5.1.9                                             |
| `9879a239` | chore(ci): 所有 dependabot 更新指向 dev 分支                                                |
| `372797ec` | refactor(build): 重组 scripts 目录结构                                                      |
| `82d9b18f` | docs(docs): 更新英文文档站内容与 i18n 缓存                                                  |
| `02e77fde` | docs(docs): 同步日文和俄文文档站 i18n 翻译                                                  |
| `6e58f45d` | chore(docs): 移除过时的计划文档目录                                                          |
| `348e7902` | chore(docs): 更新文档站构建依赖与站点配置                                                  |
| `b22231d2` | ci(ci): 将版本徽章同步从本地钩子迁移至 CI 流程                                              |
| `893f2b64` | fix(docs): 删除 vitepress 配置中多余的右括号                                                |
| `14f8de90` | fix(ci): 修复 8 个 clippy 错误使 CI 回归绿色                                                |
| `5689b712` | style(formatter): cargo fmt——assert_eq! 参数分行                                            |
| `e4b5914e` | style(typecheck): flatten nested match in function call resolver                            |
| `04d3d511` | perf(list): pop/remove_at use heap.get_mut() instead of clone+write                         |
| `1a8bd3f3` | refactor: remove redundant inline TypeRef resolution in check_var_stmt                      |
| `c96d4194` | refactor: delegate resolve_builtin_type_ref to from_builtin_name                            |
| `d0a3f11b` | refactor: delegate resolve_type_ref_type builtins to from_builtin_name                      |
| `f8548d0d` | refactor: delegate resolve_type_refs leaf to from_builtin_name                              |
| `3817d580` | refactor: replace hardcoded TypeRef(Bool/Void) with concrete types                          |
| `f32a8634` | test(formatter): 测试合规——加 AAA 分段注释符合规范 §4.1                                    |
| `ef357129` | fix: replace blind TypeRef skip with resolve-then-check in function call args               |
| `89dc0659` | fix: resolve builtin type names in From<ast::Type> to prevent TypeRef bypass                |
| `6b053589` | test: add failing tests for Float→Int function call argument                                |
| `6723b505` | refactor(typecheck): 三个凑合修复重构——消除状态标志与重复逻辑                              |
| `2bc28237` | docs(meta): auto-dev-night 修复报告（撤出 #181，留待 PR #186）                              |
| `278902c6` | fix(parser): 纯函数声明 (#168)                                                              |
| `74e1a623` | fix(typecheck): 函数调用参数 Float→Int 隐式转换拦截                                        |
| `6aa4161f` | fix(typecheck): curried 泛型函数 return 块形式类型检查                                      |
| `a80f7d01` | fix(typecheck): Never 函数循环放行 + const 泛型 curry 格式化                                |
| `9bb5b1a4` | feat(typecheck): 精化类型构造器——{}统一语义与 proof 函数                                   |
| `41b36f65` | test(parser): 修复测试规范违规——断言消息+命名+规范引用                                      |
| `e901a816` | refactor(typecheck): 赋值统一——Assign 替代 Var/Binding/ExternalBindingStmt                 |
| `5b57101f` | fix(meta): clippy 冗余模式匹配                                                              |
| `13d9bcca` | fix(parser): void 字面量在表达式位置解析失败                                                |
| `2c4075cd` | fix(meta): run 命令编译失败时显式 exit(1) 确保非零退出码                                    |
| `df6b1f2e` | docs: auto-translate documentation                                                           |
| `12715cdd` | feat(typecheck): 实现函数级 const 泛型 (RFC-011)                                            |
| `01420910` | docs(docs): 清理过时文档并修复多处引用错误                                                  |
| `a4984fa0` | refactor(parser): TypeDefinition 变体消除类型定义形状猜测                                   |
| `4db5376e` | refactor(middle): 构造器识别查表化并清理死代码                                              |
| `456734c2` | ci(workflows): 升级 GitHub Actions 版本至 Node.js 24                                        |
| `1f0015d8` | refactor: FunctionBody 枚举统一 + 泛型类型单态化 + .yx 测试文件 assert 重构（squash merge，Closes #197, #200） |
