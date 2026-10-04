---
title: 'E0xxx：词法与语法分析错误'
description: '词法分析器（Lexer）与语法分析器（Parser）阶段产生的错误。'
---

# E0xxx：词法与语法分析错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

词法分析器（Lexer）与语法分析器（Parser）阶段产生的错误。

本族共 **11** 个码，全部在 `define_codes!` 注册表中，类别为 `Lexer / Parser`。完整索引见[错误码首页](./index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E0001` | 无效字符 | `Lexer` | 否 | `无效字符：'{char}'` | ✅ 2 处 |
| `E0002` | 无效数字字面量 | `Lexer` | 否 | `无效数字字面量：'{literal}'` | ✅ 1 处 |
| `E0003` | 未终止的字符串 | `Lexer` | 否 | `字符串从第 {line} 行开始未终止` | ⚠ 暂未发射 |
| `E0004` | 无效字符字面量 | `Lexer` | 否 | `无效字符字面量：'{literal}'` | ⚠ 暂未发射 |
| `E0010` | 期望的令牌 | `Parser` | 否 | `期望 {expected}，找到 {found}` | ✅ 22 处 |
| `E0011` | 意外的令牌 | `Parser` | 否 | `意外的令牌：'{token}'` | ✅ 18 处 |
| `E0012` | 无效语法 | `Parser` | 否 | `无效语法：{reason}` | ✅ 13 处 |
| `E0013` | 不匹配的括号 | `Parser` | 否 | `不匹配的 {bracket_type}：在第 {open_line} 行、第 {open_col} 列打开，未闭合` | ⚠ 暂未发射 |
| `E0014` | 缺少分号 | `Parser` | 否 | `{statement} 后缺少分号` | ⚠ 暂未发射 |
| `E0016` | 期望表达式 | `Parser` | 否 | `期望表达式：{context}` | ✅ 4 处 |
| `E0018` | 关键字作名称 | `Parser` | 否 | `'{keyword}' 是关键字，不能用作名称` | ✅ 1 处 |

## 逐码说明

### E0001：无效字符

- **类别**：`Lexer`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_character(char)`
- **模板**：`无效字符：'{char}'`
- **消息**：源代码包含非法字符
- **帮助**：删除非法字符或使用有效的编码
- **发射点**：`src/frontend/core/lexer/tokens.rs:63`、`src/lsp/handlers/diagnostics.rs:172`

### E0002：无效数字字面量

- **类别**：`Lexer`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_number_literal(literal)`
- **模板**：`无效数字字面量：'{literal}'`
- **消息**：数字字面量格式不正确
- **帮助**：检查数字字面量的格式
- **发射点**：`src/frontend/core/lexer/tokens.rs:58`

### E0003：未终止的字符串

- **类别**：`Lexer`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unterminated_string(line)`
- **模板**：`字符串从第 {line} 行开始未终止`
- **消息**：字符串字面量缺少闭合引号
- **帮助**：为字符串添加闭合引号
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E0004：无效字符字面量

- **类别**：`Lexer`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_char_literal(literal)`
- **模板**：`无效字符字面量：'{literal}'`
- **消息**：字符字面量不正确
- **帮助**：字符字面量只能包含一个字符
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E0010：期望的令牌

- **类别**：`Parser`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::expected_token(expected, found)`
- **模板**：`期望 {expected}，找到 {found}`
- **消息**：解析器期望特定的令牌
- **帮助**：检查语法并添加期望的令牌
- **发射点**：`src/frontend/compiler.rs:177`、`src/frontend/pipeline.rs:250`、`src/frontend/core/parser/mod.rs:45` 等 22 处

### E0011：意外的令牌

- **类别**：`Parser`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unexpected_token(token)`
- **模板**：`意外的令牌：'{token}'`
- **消息**：遇到了意外的令牌
- **帮助**：删除或替换意外的令牌
- **发射点**：`src/frontend/compiler.rs:177`、`src/frontend/pipeline.rs:250`、`src/frontend/core/parser/mod.rs:45` 等 18 处

### E0012：无效语法

- **类别**：`Parser`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_syntax(reason)`
- **模板**：`无效语法：{reason}`
- **消息**：表达式或语句存在语法错误
- **帮助**：检查表达式或语句的语法
- **发射点**：`src/frontend/core/lexer/tokens.rs:44`、`src/frontend/core/lexer/tokens.rs:48`、`src/frontend/core/lexer/tokens.rs:53` 等 13 处

### E0013：不匹配的括号

- **类别**：`Parser`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::mismatched_brackets(bracket_type, open_line, open_col)`
- **模板**：`不匹配的 {bracket_type}：在第 {open_line} 行、第 {open_col} 列打开，未闭合`
- **消息**：圆括号、方括号或花括号不匹配
- **帮助**：确保所有括号都正确闭合
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E0014：缺少分号

- **类别**：`Parser`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::missing_semicolon(statement)`
- **模板**：`{statement} 后缺少分号`
- **消息**：语句缺少分号
- **帮助**：在语句末尾添加分号
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E0016：期望表达式

- **类别**：`Parser`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::expected_expression(context)`
- **模板**：`期望表达式：{context}`
- **消息**：解析器期望一个表达式
- **帮助**：在此位置提供一个有效的表达式
- **发射点**：`src/frontend/core/parser/pratt/nud.rs:129`、`src/frontend/core/parser/pratt/nud.rs:169`、`src/frontend/core/parser/pratt/nud.rs:186` 等 4 处

### E0018：关键字作名称

- **类别**：`Parser`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::keyword_as_name(keyword)`
- **模板**：`'{keyword}' 是关键字，不能用作名称`
- **消息**：关键字不能用作变量或表达式名称
- **帮助**：换一个不同的标识符代替关键字
- **发射点**：`src/frontend/core/parser/parser_state.rs:222`
- **源码注释名**：关键字作变量名
