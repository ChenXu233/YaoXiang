## 📦 版本信息

| 项目 | 值 |
| -------- | ----------------------- |
| 发布日期 | 2026-07-04              |
| 版本变更 | `0.7.6-patch1` → `0.7.7` |
| 提交数   | 56 个 commit            |

## 📋 本次更新概要

本次发版完成 C ABI FFI 全链路实现，从类型系统到代码生成到运行时，形成了完整的 Native C 调用能力。同时新增 `yaoxiang new` 项目脚手架命令、`parse_int`/`parse_float` 标准库函数，以及 RFC 工作流的自动化管理和 CI 校验。

## ✨ 新功能

### C ABI FFI 全链路

完整的 Native C 函数调用能力，从类型定义到字节码生成到运行时加载，打通整个调用链。

- `LibraryRef` / `ExternRef` MonoType 变体，类型系统层面支持外部库引用
- `typecheck` 注册 Native.c/rs 签名，添加 LibraryRef 调用规则
- IR 生成编译期求值产生 ExternRef 并注册绑定
- `native_bindings` 重构为 `ffi_libs` + `ffi_bindings`
- 字节码 `CallNative` 扩展，携带 mechanism/lib/symbol 元数据
- 执行器解码 CallNative 的 mechanism/lib/symbol
- C ABI libloading 运行时 + `OpaqueHandle` 类型
- `Native` 模块变量注册，使 Native.c 在类型系统中可解析
- 清理旧 native 机制死代码（`NativeBinding` / `FfiModule`）
- 配套 C ABI 集成测试（AAA 分段 + RFC 引用 + 错误场景）

### 项目脚手架（`new` / `init`）

新增 `yaoxiang new` 命令创建新项目，增强 `init` 命令支持 `--lib` 和当前目录初始化。

- `yaoxiang new` 创建完整项目结构
- `init` 命令支持 `--lib` 参数，在当前目录初始化
- 库项目模板生成函数
- 配套 i18n 消息和全面测试

### `parse_int` / `parse_float` 标准库

标准库新增类型转换函数，返回 Result 类型处理转换失败。

- `std::parse_int` 整数转换
- `std::parse_float` 浮点数转换
- 完整单元测试覆盖

### RFC 工作流自动化

RFC 管理流程实现自动化校验和 AI 代理辅助，提升设计文档管理效率。

- 添加 RFC 校验 GitHub Action，自动检查 RFC 格式
- 添加 RFC AI Agent 过渡 Action，辅助 RFC 审核流程
- RFC 工作流 AI 代理过渡脚本
- RFC 模板简化并添加 Issue 关联字段

### 字节码文件加载

解释器新增从文件加载并执行字节码的能力，支持独立编译产物执行。

### RFC 文档管理升级

RFC-026 被接受并拆分为两个子 RFC。多个 RFC 从草案升级为审核中。已实现 RFC 归档到独立目录并更新索引。

## 🐛 Bug 修复

### 类型系统

- 非 mut 变量遮蔽外层可变变量时补上类型赋值，避免类型丢失
- 解释器类型不匹配错误诊断改进，提供更准确的错误信息
- debug.rs `StoreUpvalue` 处理缺失分号导致的问题

### 工具和 CI

- 编译期嵌入 i18n registry 数据，消除运行时 panic
- i18n 多语言文件键缺失和语法问题修复
- 无 created 日期的 RFC 默认警告而非报错，更宽容
- 创建 nightly release 前先删除旧的同名 release，避免 API 限流
- 关闭已修复的 Issue #121

## ♻️ 重构优化

### RFC 状态管理

移除已实现 Rfc 的 derive_state 测试断言。已接受即终态，进度由 TRACKING.md 追踪，消除不必要的状态变更路径。

## 🔧 其他变更

- 依赖自动更新（production-dependencies group，4 个包）
- 移除废弃测试文件 `test_basic.yx`
- rustdoc 注释方括号转义，消除编译警告
- clippy 修复（collapsible_match、approx_constant、bool expr）
- cargo fmt 格式修复
- CI 格式检查、触发重新运行等维护性提交

## 📝 提交记录

|   Hash    | 描述 |
| :-------: | ----- |
| `6a309b9` | fix(backends): 解释器类型不匹配错误诊断改进 |
| `a3709de` | fix: add missing semicolon in debug.rs StoreUpvalue handling |
| `e785775` | chore(deps): bump the production-dependencies group across 1 directory with 2 updates |
| `5431625` | docs: auto-translate documentation |
| `1297cb0` | fix(meta): 无created日期的RFC默认警告而非错误 |
| `a1caa8c` | style(util): 修复 clippy collapsible_match lint |
| `305e895` | style(meta): 修复 CI clippy 错误（approx_constant + bool expr） |
| `b013c39` | ci: 触发 CI |
| `fd17d4a` | ci: 触发 CI 重新运行 |
| `28eae5f` | style(meta): cargo fmt 修复 CI 格式检查 |
| `1873cde` | docs: auto-translate documentation |
| `5aef3b9` | test(meta): 移除已实现Rfc的derive_state测试断言 |
| `fb3134a` | refactor(design): 移除已实现Rfc状态—accepted即终态，进度由TRACKING.md追踪 |
| `08cf07c` | ci(ci): 添加 RFC AI Agent 过渡 Action |
| `b1e7119` | feat(meta): 添加 RFC 工作流的 AI 代理过渡脚本 |
| `9576275` | ci(ci): 添加 RFC 校验 GitHub Action |
| `7c21f04` | docs(design): 添加已实现RFC状态到索引页 |
| `091035c` | docs(design): 添加已实现 RFC 归档目录 |
| `a910194` | fix(#121): 关闭已修复的 Issue#121 |
| `a624688` | feat(codegen): 实现字节码文件加载与执行 |
| `ee529d1` | docs(design): 简化 RFC 模板并添加 Issue 关联字段 |
| `137d4e3` | docs: auto-translate documentation |
| `df27a4f` | docs(design): 3 个 RFC 从草案升级为审核中 |
| `4fe9566` | docs: auto-translate documentation |
| `8d57ea9` | feat(typecheck): 注册 Native 模块变量使 Native.c 可解析 |
| `d30d442` | test(ffi): 合规重写 C ABI 集成测试（AAA 分段 + RFC 引用 + 错误场景） |
| `c651a0a` | test(ffi): C ABI 集成测试验证 Native.c 调用系统库 |
| `0945f5d` | feat(middle): 清理旧 native 机制死代码 |
| `4e35a30` | feat(std): 移除 NativeBinding 和 FfiModule（旧 native() 机制） |
| `ddff500` | feat(runtime): 执行器解码 CallNative 的 mechanism/lib/symbol |
| `ca7ca00` | feat(runtime): 添加 C ABI libloading 运行时和 OpaqueHandle 类型 |
| `b858f65` | feat(codegen): 扩展 CallNative 字节码携带 FFI 元数据 |
| `f3d0b79` | feat(middle): IR gen 编译期求值产生 ExternRef 并注册绑定 |
| `c2ff217` | feat(middle): 替换 native_bindings 为 ffi_libs 和 ffi_bindings |
| `a61cf35` | feat(typecheck): 注册 Native.c/rs 签名并添加 LibraryRef 调用规则 |
| `a2e2a9f` | feat(types): 添加 LibraryRef 和 ExternRef MonoType 变体 |
| `a24672a` | docs: auto-translate documentation |
| `0af83ef` | docs(design): RFC-026 接受并拆分两个子 RFC |
| `76d79c4` | docs(meta): update README |
| `b30ce80` | test(std): add unit tests for parse_int/parse_float and Result |
| `311ebcf` | feat(std): implement parse_int/parse_float with Result type |
| `f27031a` | feat(util): 移除 check 命令 unsupported yet 标注 |
| `ef37aec` | i18n: auto-translate locale files |
| `5c6110c` | fix(util): 编译期嵌入 i18n registry 数据，消除 panic |
| `236f96f` | test(package): 添加 new/init 命令的全面测试 |
| `8bebc1b` | feat(meta): 添加 yaoxiang new 命令，增强 init 命令 |
| `390b522` | feat: add InitOptions, exec_here for init in current dir, --lib support |
| `8c3f7da` | feat(package): 添加库项目模板生成函数 |
| `bc93b00` | fix(meta): 修复 i18n 多语言文件键缺失和语法问题 |
| `bb24cd4` | i18n: auto-translate locale files |
| `aaac021` | feat(util): 添加 new/init 脚手架相关 i18n 消息 |
| `accf878` | chore(test): 移除废弃的测试文件 test_basic.yx |
| `2e65c1a` | docs(middle): 转义 doc 注释中的方括号避免 rustdoc 警告 |
| `1579860` | fix(typecheck): 非 mut 变量遮蔽外层可变变量时补上类型赋值 |
| `bba3bde` | chore(deps): bump the production-dependencies group with 4 updates |
| `cf1d5fe` | fix(ci): 创建 nightly release 前先删除旧的同名 release，避免 Too many retries |