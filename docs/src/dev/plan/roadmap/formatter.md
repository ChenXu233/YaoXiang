---
title: "格式化器状态"
---

# 格式化器（Formatter）

> **模块状态**：基本完成
> **位置**：`src/formatter/`
> **最后更新**：2026-06-01

---

## 模块概述

格式化器负责自动格式化 YaoXiang 源代码。支持完整的 AST 节点格式化，包括表达式、语句、类型等。

**代码量**：2,397 行（19 个源文件）

---

## 功能清单

### 已实现的功能

**表达式格式化**（handlers/expr.rs，覆盖全部 Expr 变体）：
- ✅ 字面量：Int / Float / Bool / Char / String（含转义）
- ✅ 变量引用
- ✅ 二元运算符 / 一元运算符（含行宽超长时自动换行）
- ✅ 函数调用（含命名参数）
- ✅ 函数定义（表达式形式 fn def）
- ✅ if/elif/else
- ✅ match 表达式（pattern 对齐，过长时换行）
- ✅ for / while 循环（含标签）
- ✅ 代码块（含注释保留、行末注释处理）
- ✅ return / break / continue
- ✅ 类型转换（as）
- ✅ 元组 / 列表 / 列表推导式 / 字典
- ✅ 索引 / 字段访问（链式调用换行）
- ✅ try 操作符（?）
- ✅ ref / borrow（& / &mut）
- ✅ unsafe 块
- ✅ eval 块（@block / @auto / @eager）
- ✅ spawn 块
- ✅ lambda 表达式（单表达式简洁形式）
- ✅ f-string（含格式化规格）
- ✅ Error 节点（插入 `/* error */` 占位符）

**语句格式化**（handlers/stmt.rs）：
- ✅ 变量声明（mut / 类型注解 / 初始化器）
- ✅ for 循环语句
- ✅ 统一绑定语句（函数 / 类型 / 方法绑定）
- ✅ use 导入语句（含 items、alias）
- ✅ if 语句
- ✅ 外部绑定语句（External / Anonymous / DefaultExternal）

**类型格式化**（handlers/types.rs，覆盖全部 Type 变体）：
- ✅ 基本类型：Int(size) / Float(size) / Char / String / Bytes / Bool / Void
- ✅ 命名类型 / 结构体 / 命名结构体
- ✅ Union / Enum / Variant
- ✅ 元组 / 函数类型 / Option / Result
- ✅ 泛型 / 关联类型 / Sum 类型
- ✅ 字面量类型 / 引用类型 / 指针类型 / MetaType

**其他功能**：
- ✅ 分隔列表自动换行（handlers/delimited.rs）
- ✅ 注释保留（source_map.rs）
- ✅ 导入排序（rules/sort_imports.rs）
- ✅ CLI 命令：check / write / stdout 模式（command.rs）
- ✅ 配置选项：line_width / indent_width / use_tabs / single_quote / sort_imports（options.rs）

---

## 未完全实现的规范

| 规范 | 状态 | 差异说明 |
|------|------|----------|
| §2.2 换行策略优先级 | ⚠️ 部分实现 | 仅实现了 binop 换行，未实现完整的优先级链 |
| §4.2 参数列表换行（尾随逗号） | ⚠️ 部分实现 | 参数列表格式化始终输出单行，不会自动换行 |
| §6.2 单行代码块 | ❌ 未实现 | 始终输出多行格式 `{ stmt }` |
| §6.3 空代码块 | ⚠️ 未完全实现 | 对空块输出 `{\n}`（两行），而非 `{}` |
| §8.3 单引号模式 | ❌ 未实现 | `single_quote` 字段存在但未读取 |
| §E2 退出码 | ❌ 未实现 | 未按规范使用特定退出码 |

---

## 测试覆盖

**51 个测试 + 1 个 proptest 幂等性测试**：

| 测试组 | 数量 | 覆盖内容 |
|--------|------|----------|
| handlers/tests/expr | 24 | 字面量、二元运算、函数调用、列表、字典、return、cast、match、f-string、try、unsafe、语法错误容错 |
| handlers/tests/types | 15 | int/float/bool/string/char/void/tuple/option/fn/ref/mut_ref/ptr/name/enum/sum |
| rules/tests/sort_imports | 2 | 分类函数 + 完整排序验证 |
| tests/source_map | 9 | 单行/多行/文档/嵌套注释、空白行、偏移量转换 |
| tests/properties | 1 | **幂等性属性测试**（proptest） |

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 功能完成度 | 90% | 所有 AST 节点均有格式化逻辑，6 条规范未完全实现 |
| 测试覆盖 | 中等 | 51 个测试 + 1 个 proptest，缺少端到端集成测试 |
| 文档完整度 | 高 | 源码注释齐全，设计文档详尽（18 条规则 + 4 条原则） |
| 代码质量 | 良好 | 模块划分清晰，handler/rules/tests 分层合理 |

---

## 待改进项（按优先级排序）

1. **空代码块输出 `{}` 而非 `{\n}`**（§6.3）
2. **`single_quote` 配置生效**（§8.3）
3. **参数列表支持自动换行**（§4.2）
4. **单语句代码块压缩为单行**（§6.2）
5. **换行策略优先级链完整实现**（§2.2）
6. **CLI 退出码按规范实现**（§E2）
