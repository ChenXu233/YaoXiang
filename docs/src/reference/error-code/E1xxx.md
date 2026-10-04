---
title: 'E1xxx：类型检查错误'
description: '类型检查阶段产生的错误，涵盖类型匹配、模式匹配、泛型实例化、接口约束与 `?` 错误传播等。'
---

# E1xxx：类型检查错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

类型检查阶段产生的错误，涵盖类型匹配、模式匹配、泛型实例化、接口约束与 `?` 错误传播等。

本族共 **53** 个码，全部在 `define_codes!` 注册表中，类别为 `TypeCheck`。完整索引见[错误码首页](./index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E1001` | 未知变量 | `TypeCheck` | 否 | `未知变量：'{name}'` | ✅ 4 处 |
| `E1002` | 类型不匹配 | `TypeCheck` | 否 | `期望类型 '{expected}'，实际类型 '{found}'` | ✅ 83 处 |
| `E1003` | 未知类型 | `TypeCheck` | 否 | `未知类型：'{type}'` | ✅ 3 处 |
| `E1010` | 参数数量不匹配 | `TypeCheck` | 否 | `函数 '{func}' 期望 {expected} 个参数，找到 {found} 个` | ✅ 18 处 |
| `E1011` | 参数类型不匹配 | `TypeCheck` | 否 | `参数类型不匹配：期望 '{expected}'，实际 '{found}'` | ⚠ 暂未发射 |
| `E1012` | 返回类型不匹配 | `TypeCheck` | 否 | `返回类型不匹配：期望 '{expected}'，实际 '{found}'` | ✅ 1 处 |
| `E1013` | 函数未找到 | `TypeCheck` | 否 | `函数未找到：'{func}'` | ✅ 5 处 |
| `E1014` | 命名参数名未知 | `TypeCheck` | 否 | `函数 '{func}' 没有名为 '{name}' 的参数（可用：{params}）` | ✅ 2 处 |
| `E1015` | 参数重复指定 | `TypeCheck` | 否 | `函数 '{func}' 的参数 '{name}' 被重复指定` | ✅ 2 处 |
| `E1020` | 无法推断类型 | `TypeCheck` | 否 | `无法推断 '{expr}' 的类型` | ⚠ 暂未发射 |
| `E1021` | 类型推断冲突 | `TypeCheck` | 否 | `类型推断冲突：{reason}` | ⚠ 暂未发射 |
| `E1030` | 模式不完整 | `TypeCheck` | 否 | `模式不完整：缺少 {patterns}` | ✅ 2 处 |
| `E1031` | 不可达模式 | `TypeCheck` | 否 | `不可达模式：'{pattern}'` | ✅ 3 处 |
| `E1032` | 模式重复绑定 | `TypeCheck` | 否 | `模式重复绑定：'{name}' 在同一模式中出现多次` | ✅ 2 处 |
| `E1033` | 或模式绑定不一致 | `TypeCheck` | 否 | `或模式绑定不一致：\`\|\` 两侧必须绑定相同的名字集` | ✅ 1 处 |
| `E1034` | 结构体模式缺字段 | `TypeCheck` | 否 | `结构体模式缺少字段 '{field}'（'{struct}' 的字段须完整覆盖）` | ✅ 1 处 |
| `E1040` | 操作不支持 | `TypeCheck` | 否 | `类型 '{type}' 不支持操作 '{op}'` | ⚠ 暂未发射 |
| `E1041` | 索引越界 | `TypeCheck` | 否 | `索引越界：有效范围是 0..{max}，找到 {index}` | ✅ 5 处 |
| `E1042` | 字段未找到 | `TypeCheck` | 否 | `在结构体 '{struct}' 中找不到字段 '{field}'` | ✅ 9 处 |
| `E1043` | 模块成员未找到 | `TypeCheck` | 否 | `模块 '{module}' 没有导出 '{name}'` | ✅ 1 处 |
| `E1050` | 需要布尔操作数 | `TypeCheck` | 否 | `逻辑运算需要布尔操作数，实际为 '{left}' 和 '{right}'` | ✅ 1 处 |
| `E1051` | 逻辑 NOT 需要布尔操作数 | `TypeCheck` | 否 | `逻辑 NOT 需要布尔操作数，实际为 '{type}'` | ✅ 1 处 |
| `E1052` | 无效解引用 | `TypeCheck` | 否 | `无法解引用类型 '{type}'，期望指针类型` | ✅ 1 处 |
| `E1053` | 非结构体字段访问 | `TypeCheck` | 否 | `无法在非结构体类型 '{type}' 上访问字段` | ✅ 2 处 |
| `E1054` | 条件类型不匹配 | `TypeCheck` | 否 | `条件必须是布尔类型，实际为 '{type}'` | ✅ 3 处 |
| `E1055` | 约束在非泛型上下文中 | `TypeCheck` | 否 | `约束类型 '{type}' 只能在泛型上下文中使用` | ⚠ 暂未发射 |
| `E1060` | 类型参数数量不匹配 | `TypeCheck` | 否 | `期望 {expected} 个类型参数，实际 {found} 个` | ⚠ 暂未发射 |
| `E1061` | 无法实例化泛型 | `TypeCheck` | 否 | `无法用给定参数实例化泛型类型` | ⚠ 暂未发射 |
| `E1062` | const 泛型约束失败 | `TypeCheck` | 否 | `const 泛型约束失败: \`{constraint}\` 不成立` | ✅ 1 处 |
| `E1064` | 绑定位置索引无效 | `TypeCheck` | 否 | `绑定位置索引无效：{positions}（函数共 {total} 个参数）` | ✅ 1 处 |
| `E1065` | 对非函数值调用 | `TypeCheck` | 否 | `类型 \`{type}\` 不可调用——它不是函数` | ✅ 1 处 |
| `E1071` | 类型定义只能在模块级 | `TypeCheck` | 否 | `类型定义 '{name}' 只能在模块级（模块顶层）` | ✅ 2 处 |
| `E1081` | `?` 仅允许在返回可传播类型的函数内使用 | `TypeCheck` | 否 | `\`?\` 仅允许在返回实现 \`Try\` 的类型的函数内使用` | ✅ 2 处 |
| `E1082` | `?` 只能用于实现 Try 的类型 | `TypeCheck` | 否 | `'{type}' 未实现 \`Try\` 接口，不能使用 \`?\`` | ✅ 1 处 |
| `E1083` | `?` 的错误类型不匹配 | `TypeCheck` | 否 | `\`?\` 的错误类型不匹配：期望 '{expected}'，实际 '{found}'` | ✅ 1 处 |
| `E1090` | ✨ 不可言说 ✨ | `TypeCheck` | 否 | `Type: Type = Type` | ⚠ 暂未发射 |
| `E1091` | 无效的泛型元类型 | `TypeCheck` | 否 | `泛型元类型自引用不允许：'{decl}'` | ✅ 1 处 |
| `E1092` | 精化类型实参形态非法 | `TypeCheck` | 否 | `'{name}' 要求编译期常量实参，但所给实参无法转换` | ✅ 3 处 |
| `E1093` | 精化实参个数不匹配 | `TypeCheck` | 否 | `'{name}' 期望 {expected} 个实参，实际 {found} 个` | ✅ 2 处 |
| `E1094` | 未使用的编译期值参数 | `TypeCheck` | 否 | `'{param}' 声明为 '{type}' 的编译期值参数，但未在类型体中被引用` | ✅ 1 处 |
| `E1095` | 未知接口 | `TypeCheck` | 否 | `未知接口：'{name}'（类型体引用的名字不是已注册的类型构造器）` | ✅ 1 处 |
| `E1096` | 接口参数数量不匹配 | `TypeCheck` | 否 | `接口 '{name}' 期望 {expected} 个类型参数，找到 {found} 个` | ✅ 3 处 |
| `E1097` | 接口成员命名冲突 | `TypeCheck` | 否 | `类型 '{type}' 的成员 '{member}' 与接口成员冲突（字段与方法共享命名空间，§1.2）` | ✅ 1 处 |
| `E1098` | 接口方法未实现 | `TypeCheck` | 否 | `类型 '{type}' 未实现接口 '{interface}' 的方法 '{method}'` | ✅ 1 处 |
| `E1099` | 接口方法签名不匹配 | `TypeCheck` | 否 | `类型 '{type}' 的方法 '{method}' 与接口签名不符：期望 {expected}，实际 {found}` | ✅ 2 处 |
| `E1100` | 接口方法重复实现 | `TypeCheck` | 否 | `类型 '{type}' 的方法 '{method}' 被重复实现（同签名覆盖被禁止，§3）` | ✅ 1 处 |
| `E1101` | 类型未实现接口 | `TypeCheck` | 否 | `类型 '{type}' 未实现接口 '{interface}'，不能进入该存在类型位置` | ✅ 5 处 |
| `E1102` | 循环控制语句出现在循环外 | `TypeCheck` | 否 | `'{keyword}' 出现在循环外` | ✅ 2 处 |
| `E1103` | 类型位置不能使用方括号 | `TypeCheck` | 是 | `'{name}[...]' 不是类型语法——类型实参要用圆括号` | ✅ 1 处 |
| `E1104` | 接口实现不在类型的定义模块 | `TypeCheck` | 否 | `类型 '{type}' 不由本模块定义，不能在本模块为其实现接口 '{interface}'` | ✅ 1 处 |
| `E1105` | 变体构造器不可作为字段访问 | `TypeCheck` | 否 | `'{variant}' 是和类型 '{type}' 的变体构造器，不是数据字段` | ✅ 1 处 |
| `E1106` | 约束未满足 | `TypeCheck` | 否 | `类型 '{type}' 未实现 \`{interface}\`——泛型参数 \`{param}\` 的约束 \`{param}: {interface}\` 在此调用点不成立` | ✅ 1 处 |
| `E1107` | 方法重载歧义 | `TypeCheck` | 否 | `方法重载歧义：'{key}' 有 {count} 个候选匹配且无法区分` | ✅ 1 处 |
| `E1108` | 空块落入容器期望位 | `TypeCheck` | 是 | `此处期望容器类型，但 \`{}\` 是空块（值 Void）` | ✅ 2 处 |

## 逐码说明

### E1001：未知变量

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unknown_variable(name)`
- **模板**：`未知变量：'{name}'`
- **消息**：引用的变量未定义
- **帮助**：检查变量名是否拼写正确，或先定义它
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:2792`、`src/frontend/core/typecheck/inference/statements.rs:2496`、`src/util/diagnostic/error.rs:16` 等 4 处

### E1002：类型不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::type_mismatch(expected, found)`
- **模板**：`期望类型 '{expected}'，实际类型 '{found}'`
- **消息**：期望类型与实际类型不匹配
- **帮助**：使用正确的类型或添加类型转换
- **发射点**：`src/frontend/core/typecheck/inference/assignment.rs:103`、`src/frontend/core/typecheck/inference/assignment.rs:116`、`src/frontend/core/typecheck/inference/bounds.rs:309` 等 83 处

### E1003：未知类型

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unknown_type(type_)`
- **模板**：`未知类型：'{type}'`
- **消息**：引用的类型不存在
- **帮助**：检查类型名是否拼写正确
- **发射点**：`src/frontend/core/typecheck/checker.rs:691`、`src/frontend/core/typecheck/environment.rs:436`、`src/util/test_markers.rs:241`

### E1010：参数数量不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::argument_count_mismatch(func, expected, found)`
- **模板**：`函数 '{func}' 期望 {expected} 个参数，找到 {found} 个`
- **消息**：函数调用参数与定义不匹配
- **帮助**：检查函数调用参数数量
- **发射点**：`src/frontend/core/typecheck/environment.rs:371`、`src/frontend/core/typecheck/inference/expressions.rs:1174`、`src/frontend/core/typecheck/inference/expressions.rs:1279` 等 18 处

### E1011：参数类型不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::parameter_type_mismatch(expected, found)`
- **模板**：`参数类型不匹配：期望 '{expected}'，实际 '{found}'`
- **消息**：参数类型检查失败
- **帮助**：确保参数类型与函数签名匹配
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E1012：返回类型不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::return_type_mismatch(expected, found)`
- **模板**：`返回类型不匹配：期望 '{expected}'，实际 '{found}'`
- **消息**：函数返回类型不正确
- **帮助**：函数体的值与声明的返回类型不符。要么修改函数体，要么修正注解——注意 `h: () -> Int = f` 的含义是「h 返回 Int」，而不是「h 就是 f」；若想给函数起别名请写 `h = f`。
- **发射点**：`src/frontend/core/typecheck/inference/statements.rs:1088`

### E1013：函数未找到

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::function_not_found(func)`
- **模板**：`函数未找到：'{func}'`
- **消息**：调用了未定义的函数
- **帮助**：检查函数名是否拼写正确
- **发射点**：`src/backends/mod.rs:132`、`src/backends/interpreter/executor/debug.rs:145`、`src/backends/interpreter/executor/executor.rs:254` 等 5 处

### E1014：命名参数名未知

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unknown_argument(func, name, params)`
- **模板**：`函数 '{func}' 没有名为 '{name}' 的参数（可用：{params}）`
- **消息**：调用时使用的参数名不在函数的形参表里
- **帮助**：改用形参表里列出的名字，或改为按位置传参
- **发射点**：`src/middle/core/ir_gen.rs:7270`、`src/middle/core/ir_gen.rs:7708`
- **源码注释名**：命名参数名未知（与调用者写的名字不在形参表里）

### E1015：参数重复指定

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::duplicate_argument(func, name)`
- **模板**：`函数 '{func}' 的参数 '{name}' 被重复指定`
- **消息**：同一形参被位置实参与命名实参同时指定
- **帮助**：每个参数只能传一次：去掉多余的命名实参或位置实参
- **发射点**：`src/middle/core/ir_gen.rs:7279`、`src/middle/core/ir_gen.rs:7717`
- **源码注释名**：同一形参被位置实参与命名实参同时指定（或命名两次）

### E1020：无法推断类型

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::cannot_infer_type(expr)`
- **模板**：`无法推断 '{expr}' 的类型`
- **消息**：上下文无法推断类型
- **帮助**：添加类型注解或显式类型参数
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E1021：类型推断冲突

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::type_inference_conflict(reason)`
- **模板**：`类型推断冲突：{reason}`
- **消息**：多个约束导致类型矛盾
- **帮助**：检查类型注解的一致性
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E1030：模式不完整

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::pattern_non_exhaustive(patterns)`
- **模板**：`模式不完整：缺少 {patterns}`
- **消息**：match 表达式没有覆盖所有情况
- **帮助**：为 match 表达式添加缺失的模式
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:1075`、`src/frontend/core/typecheck/inference/expressions.rs:1086`
- **源码注释名**：模式穷举不足

### E1031：不可达模式

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unreachable_pattern(pattern)`
- **模板**：`不可达模式：'{pattern}'`
- **消息**：永远无法匹配的模式
- **帮助**：删除或修改不可达的模式
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:950`、`src/frontend/core/typecheck/inference/expressions.rs:967`、`src/frontend/core/typecheck/inference/expressions.rs:975`

### E1032：模式重复绑定

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::duplicate_pattern_binding(name)`
- **模板**：`模式重复绑定：'{name}' 在同一模式中出现多次`
- **消息**：同一模式内 '{name}' 绑定了多次
- **帮助**：一个模式里每个名字只能绑定一次；要匹配两个相同字段请用不同名字（Rust 语义同）
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:1143`、`src/frontend/core/typecheck/inference/expressions.rs:1220`
- **源码注释名**：模式重复绑定（RFC-010b：同一模式内同名绑定出现多次）

### E1033：或模式绑定不一致

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::or_pattern_binding_mismatch()`
- **模板**：`或模式绑定不一致：\`\|\` 两侧必须绑定相同的名字集`
- **消息**：`\|` 各备选绑定的名字集不同
- **帮助**：或模式 `a(x) \| b(x)` 的每个备选必须引入完全相同的绑定名（Rust 语义同）
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:1118`
- **源码注释名**：或模式绑定不一致（RFC-010b：各备选绑定的名字集不同）

### E1034：结构体模式缺字段

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::struct_pattern_missing_field(struct_, field)`
- **模板**：`结构体模式缺少字段 '{field}'（'{struct}' 的字段须完整覆盖）`
- **消息**：结构体模式未覆盖字段 '{field}'
- **帮助**：结构体模式必须列出全部字段；`..` 略名尚未支持
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:1214`
- **源码注释名**：结构体模式缺字段（RFC-010b：字段须完整覆盖，`..` 略名未支持）

### E1040：操作不支持

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unsupported_operation(op, type_)`
- **模板**：`类型 '{type}' 不支持操作 '{op}'`
- **消息**：类型不支持该操作
- **帮助**：检查该类型支持的操作
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：不支持的操作

### E1041：索引越界

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::index_out_of_bounds(max, index)`
- **模板**：`索引越界：有效范围是 0..{max}，找到 {index}`
- **消息**：数组/列表索引超出范围
- **帮助**：确保索引在集合范围内
- **发射点**：`src/backends/mod.rs:171`、`src/backends/interpreter/executor/ops/elem.rs:225`、`src/frontend/core/typecheck/inference/expressions.rs:2967` 等 5 处
- **源码注释名**：数组越界

### E1042：字段未找到

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::field_not_found(field, struct_)`
- **模板**：`在结构体 '{struct}' 中找不到字段 '{field}'`
- **消息**：访问了不存在的结构体字段
- **帮助**：检查结构体的可用字段
- **发射点**：`src/backends/mod.rs:150`、`src/frontend/core/typecheck/inference/expressions.rs:1228`、`src/frontend/core/typecheck/inference/expressions.rs:1271` 等 9 处

### E1043：模块成员未找到

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::module_has_no_export(module, name, available)`
- **模板**：`模块 '{module}' 没有导出 '{name}'`
- **消息**：访问了模块未导出的成员
- **帮助**：该模块的可用导出：{available}
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3177`
- **源码注释名**：模块成员未找到（模块不是 struct，不得借道 E1042 的 field/struct 语义）

### E1050：需要布尔操作数

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::logical_operand_type_mismatch(left, right)`
- **模板**：`逻辑运算需要布尔操作数，实际为 '{left}' 和 '{right}'`
- **消息**：逻辑运算需要布尔操作数
- **帮助**：将操作数转换为布尔类型或使用其他运算符
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:830`
- **源码注释名**：逻辑运算需要布尔操作数

### E1051：逻辑 NOT 需要布尔操作数

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::logical_not_type_mismatch(type_)`
- **模板**：`逻辑 NOT 需要布尔操作数，实际为 '{type}'`
- **消息**：逻辑 NOT 运算需要布尔操作数
- **帮助**：在应用 NOT 前将操作数转换为布尔类型
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:1764`

### E1052：无效解引用

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_deref(type_)`
- **模板**：`无法解引用类型 '{type}'，期望指针类型`
- **消息**：无法解引用非指针类型
- **帮助**：只有指针类型可以解引用
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:1774`
- **源码注释名**：不能解引用非指针类型

### E1053：非结构体字段访问

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::field_access_on_non_struct(type_)`
- **模板**：`无法在非结构体类型 '{type}' 上访问字段`
- **消息**：无法在非结构体类型上访问字段
- **帮助**：字段访问仅适用于结构体类型
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3179`、`src/frontend/core/typecheck/inference/expressions.rs:3183`
- **源码注释名**：不能在非结构体类型上访问字段

### E1054：条件类型不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::condition_type_mismatch(type_)`
- **模板**：`条件必须是布尔类型，实际为 '{type}'`
- **消息**：条件表达式必须是布尔类型
- **帮助**：使用布尔表达式或转换为布尔类型
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3220`、`src/frontend/core/typecheck/inference/expressions.rs:5049`、`src/frontend/core/typecheck/inference/expressions.rs:5061`
- **源码注释名**：条件必须是布尔类型

### E1055：约束在非泛型上下文中

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::constraint_not_in_generic(type_)`
- **模板**：`约束类型 '{type}' 只能在泛型上下文中使用`
- **消息**：约束类型在非泛型上下文中使用
- **帮助**：使用具体类型代替约束类型
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：约束类型只能在泛型上下文中使用

### E1060：类型参数数量不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::type_argument_count_mismatch(expected, found)`
- **模板**：`期望 {expected} 个类型参数，实际 {found} 个`
- **消息**：提供的类型参数数量错误
- **帮助**：提供正确数量的类型参数
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E1061：无法实例化泛型

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::cannot_instantiate_generic()`
- **模板**：`无法用给定参数实例化泛型类型`
- **消息**：泛型类型无法用给定参数实例化
- **帮助**：确保类型参数满足所有约束
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：无法实例化泛型类型

### E1062：const 泛型约束失败

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::const_constraint_failed(constraint)`
- **模板**：`const 泛型约束失败: \`{constraint}\` 不成立`
- **消息**：const 泛型参数的值约束不满足
- **帮助**：修改 const 参数值使其满足约束
- **发射点**：`src/frontend/core/typecheck/environment.rs:416`

### E1064：绑定位置索引无效

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_binding_position(positions, total)`
- **模板**：`绑定位置索引无效：{positions}（函数共 {total} 个参数）`
- **帮助**：绑定位置必须落在被绑函数的参数范围内（越界或归一化后仍为负）。请检查方法绑定的位置编号。
- **发射点**：`src/frontend/core/typecheck/checker.rs:2222`
- **源码注释名**：绑定位置索引无效（RFC-004）

### E1065：对非函数值调用

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::not_callable(type_)`
- **模板**：`类型 \`{type}\` 不可调用——它不是函数`
- **帮助**：只有函数值才能被调用。请检查该位置表达式的类型；若要定义函数，需写 Fn 注解 `f: () -> T = { ... }`。
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3804`

### E1071：类型定义只能在模块级

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::type_def_only_at_module_level(name)`
- **模板**：`类型定义 '{name}' 只能在模块级（模块顶层）`
- **消息**：类型定义只能出现在模块顶层
- **帮助**：将类型定义移到模块顶层
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:5293`、`src/frontend/core/typecheck/inference/statements.rs:1453`

### E1081：`?` 仅允许在返回可传播类型的函数内使用

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::try_only_allowed_in_result()`
- **模板**：`\`?\` 仅允许在返回实现 \`Try\` 的类型的函数内使用`
- **消息**：`?` 只能在返回实现 Try 的类型的函数内使用
- **帮助**：`?` 的失败值要沿函数返回类型传播——返回类型须实现 `Try` 接口（如 `Result(T, E)`）
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3663`、`src/frontend/core/typecheck/inference/expressions.rs:3687`
- **源码注释名**：`?` 仅允许在返回 Result 的函数内使用

### E1082：`?` 只能用于实现 Try 的类型

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::try_requires_result(type_)`
- **模板**：`'{type}' 未实现 \`Try\` 接口，不能使用 \`?\``
- **消息**：`?` 只能用于实现 Try 接口的类型的表达式
- **帮助**：为该类型实现 `Try` 接口四成员（is_failure/success/residual/from_error），或改用 `Result(T, E)`
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3654`
- **源码注释名**：`?` 只能用于 Result 表达式

### E1083：`?` 的错误类型不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::try_error_type_mismatch(expected, found)`
- **模板**：`\`?\` 的错误类型不匹配：期望 '{expected}'，实际 '{found}'`
- **消息**：`expr?` 的错误类型与外层函数返回类型不匹配
- **帮助**：请确保当前函数返回类型 `Result[_, E]` 的错误类型 `E` 与被解包的 `Result[_, E]` 的错误类型一致。
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3678`

### E1090：✨ 不可言说 ✨

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::type_self_reference_easter_egg()`
- **模板**：`Type: Type = Type`
- **消息**：你已触达 YaoXiang 的哲学边界
- **帮助**：此乃语言之源，编译器在此沉默
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：彩蛋（返回占位符，由 i18n 的 zen_message 提供实际消息）

### E1091：无效的泛型元类型

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_generic_self_reference(decl)`
- **模板**：`泛型元类型自引用不允许：'{decl}'`
- **消息**：泛型元类型自引用是不允许的
- **帮助**：类型构造器如 'Type[T]' 不能引用 Type 自身
- **发射点**：`src/frontend/core/typecheck/checker.rs:3045`
- **源码注释名**：泛型元类型自指错误

### E1092：精化类型实参形态非法

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::refined_arg_not_const(name)`
- **模板**：`'{name}' 要求编译期常量实参，但所给实参无法转换`
- **消息**：精化类型实参无法转换为编译期常量表达式
- **帮助**：谓词/证明函数实参必须是字面量、变量或单参数类型应用，否则精化约束无法生成
- **发射点**：`src/frontend/core/typecheck/checker.rs:3845`、`src/frontend/core/typecheck/checker.rs:3893`、`src/frontend/core/typecheck/checker.rs:3921`
- **源码注释名**：精化类型实参形态非法（RFC-027，#263）

### E1093：精化实参个数不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::refined_arity_mismatch(name, expected, found)`
- **模板**：`'{name}' 期望 {expected} 个实参，实际 {found} 个`
- **消息**：谓词/证明函数实参个数不匹配
- **帮助**：实参个数必须与谓词声明一致，否则精化约束无法生成
- **发射点**：`src/frontend/core/typecheck/checker.rs:3828`、`src/frontend/core/typecheck/checker.rs:3918`
- **源码注释名**：精化类型参数数量不匹配

### E1094：未使用的编译期值参数

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unused_const_param(param, type_)`
- **模板**：`'{param}' 声明为 '{type}' 的编译期值参数，但未在类型体中被引用`
- **消息**：编译期值参数未在类型体中被引用
- **帮助**：请在类型位置（字段类型、内层 Fn 参数类型、Assert/Array 类型实参）引用该参数，或从声明中移除
- **发射点**：`src/frontend/core/typecheck/checker.rs:3240`
- **源码注释名**：编译期值参数未在类型体引用（#297/F）

### E1095：未知接口

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unknown_interface(name)`
- **模板**：`未知接口：'{name}'（类型体引用的名字不是已注册的类型构造器）`
- **消息**：未知接口
- **帮助**：检查名字拼写，或先以 `Iface: (Self: Type) -> Type = &#123;&#123; ... &#125;&#125;` 声明接口
- **发射点**：`src/frontend/core/typecheck/checker.rs:2522`
- **源码注释名**：未知接口（RFC-011a）

### E1096：接口参数数量不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::interface_arity_mismatch(name, expected, found)`
- **模板**：`接口 '{name}' 期望 {expected} 个类型参数，找到 {found} 个`
- **消息**：接口参数数量不匹配
- **帮助**：按接口声明的类型参数个数提供实参
- **发射点**：`src/frontend/core/typecheck/checker.rs:2483`、`src/frontend/core/typecheck/checker.rs:2499`、`src/frontend/core/typecheck/checker.rs:2530`
- **源码注释名**：接口实例化类型实参个数不匹配（RFC-011a）

### E1097：接口成员命名冲突

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::interface_member_conflict(type_, member)`
- **模板**：`类型 '{type}' 的成员 '{member}' 与接口成员冲突（字段与方法共享命名空间，§1.2）`
- **消息**：接口成员命名冲突
- **帮助**：重命名字段或接口成员
- **发射点**：`src/frontend/core/typecheck/checker.rs:2645`
- **源码注释名**：接口成员与类型已有字段共享命名空间（RFC-011a §1.2）

### E1098：接口方法未实现

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::interface_method_missing(type_, interface, method)`
- **模板**：`类型 '{type}' 未实现接口 '{interface}' 的方法 '{method}'`
- **消息**：接口方法未实现
- **帮助**：以 `Type.method: 签名 = ...` 声明该方法，或在类型体内提供绑定
- **发射点**：`src/frontend/core/typecheck/checker.rs:2666`
- **源码注释名**：接口方法未实现（RFC-011a 完整性检查）

### E1099：接口方法签名不匹配

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::interface_method_mismatch(type_, method, expected, found)`
- **模板**：`类型 '{type}' 的方法 '{method}' 与接口签名不符：期望 {expected}，实际 {found}`
- **消息**：接口方法签名不匹配
- **帮助**：让实现签名与接口成员签名完全一致（§3 禁止覆盖，允许重载）
- **发射点**：`src/frontend/core/typecheck/checker.rs:2554`、`src/frontend/core/typecheck/checker.rs:2692`
- **源码注释名**：接口方法签名不匹配（RFC-011a）

### E1100：接口方法重复实现

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::interface_method_duplicate(type_, method)`
- **模板**：`类型 '{type}' 的方法 '{method}' 被重复实现（同签名覆盖被禁止，§3）`
- **消息**：接口方法重复实现
- **帮助**：删除重复实现，保留一份
- **发射点**：`src/frontend/core/typecheck/checker.rs:1935`
- **源码注释名**：同签名接口方法重复实现（RFC-011a §3 覆盖禁止）

### E1101：类型未实现接口

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::type_does_not_implement_interface(type_, interface)`
- **模板**：`类型 '{type}' 未实现接口 '{interface}'，不能进入该存在类型位置`
- **消息**：类型未实现接口
- **帮助**：实现该接口的全部成员并在类型体内实例化（如 `Animal(Dog)`）；RFC-011b 场景（相等/运算符）请为类型实现对应接口，线性令牌字段需先移除
- **发射点**：`src/frontend/core/typecheck/inference/existential.rs:141`、`src/frontend/core/typecheck/inference/expressions.rs:1616`、`src/frontend/core/typecheck/inference/expressions.rs:1633` 等 5 处
- **源码注释名**：具体类型未实现目标接口（RFC-011a §6.3 存在类型成员检查）

### E1102：循环控制语句出现在循环外

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::break_outside_loop(keyword)`
- **模板**：`'{keyword}' 出现在循环外`
- **消息**：break/continue 只能在 while/for 循环体内使用
- **帮助**：把 '{keyword}' 移入 while/for 循环体内
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3294`、`src/frontend/core/typecheck/inference/expressions.rs:3304`
- **源码注释名**：break/continue 出现在循环外（#311：仅 while/for 体内允许循环控制流）

### E1103：类型位置不能使用方括号

- **类别**：`TypeCheck`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::bracket_in_type_position(name)`
- **模板**：`'{name}[...]' 不是类型语法——类型实参要用圆括号`
- **消息**：类型实参用圆括号，方括号是值级下标
- **帮助**：写成 `{name}(...)`。方括号是**值级**下标（`list[0]`），类型位置只能写圆括号：`List(Int)` 而非 `List[Int]`。
- **发射点**：`src/frontend/core/typecheck/checker.rs:689`
- **源码注释名**：类型位置的方括号写法（#371）。

### E1104：接口实现不在类型的定义模块

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::interface_impl_outside_defining_module(type_, interface)`
- **模板**：`类型 '{type}' 不由本模块定义，不能在本模块为其实现接口 '{interface}'`
- **消息**：运算符接口实现必须写在类型的定义模块里
- **帮助**：孤儿规则：实现跟随类型的定义模块。只能为本模块定义的类型写接口实例化（如 `Add(Point, Point, Point)`）；内建类型（Int/List 等）的运算符接口由核心提供，不可补充
- **发射点**：`src/frontend/core/typecheck/checker.rs:2621`
- **源码注释名**：接口实现写在了非定义模块（RFC-011b 孤儿规则：实现跟随类型的

### E1105：变体构造器不可作为字段访问

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::variant_used_as_field(type_, variant)`
- **模板**：`'{variant}' 是和类型 '{type}' 的变体构造器，不是数据字段`
- **消息**：变体构造器只能以 类型.变体(...) 的调用形态使用
- **帮助**：构造值写 `{type}.{variant}(...)`；从和类型值取回载荷用 match 变体解构（RFC-010b）。和类型的记录字段全部升格为变体，运行时值不携带字段表
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3054`
- **源码注释名**：变体构造器被当作字段访问（RFC-010 记录式和类型：变体名升格，

### E1106：约束未满足

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::constraint_unsatisfied(type_, param, interface)`
- **模板**：`类型 '{type}' 未实现 \`{interface}\`——泛型参数 \`{param}\` 的约束 \`{param}: {interface}\` 在此调用点不成立`
- **消息**：类型 '{type}' 未实现接口 '{interface}'（约束 '{param}: {interface}'）
- **帮助**：为该类型实现对应接口（类型体内写接口实例化并定义方法），或改用已实现该接口的实参
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:4191`
- **源码注释名**：约束未满足（RFC-011 §5.2 调用点复检）

### E1107：方法重载歧义

- **类别**：`TypeCheck`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::ambiguous_method_overload(key, count)`
- **模板**：`方法重载歧义：'{key}' 有 {count} 个候选匹配且无法区分`
- **消息**：'{key}' 有 {count} 个候选匹配，无法决议
- **帮助**：为调用结果添加类型注解（如 `x: T = obj.m()`），或让重载候选的参数类型可区分
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:4328`
- **源码注释名**：方法重载歧义（RFC-011a §3：多候选且无期望类型可区分）

### E1108：空块落入容器期望位

- **类别**：`TypeCheck`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::empty_block_as_container()`
- **模板**：`此处期望容器类型，但 \`{}\` 是空块（值 Void）`
- **消息**：`{}` 是空块（值 Void），不是容器字面量
- **帮助**：B 方案语义（SPEC syntax §1.6.4）：`{}` 是空块，不是空字典。空字典用 `dict.new()`，空列表用 `[]`
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:3329`、`src/frontend/core/typecheck/inference/expressions.rs:4084`
- **源码注释名**：空块 `{}` 落入容器期望位（#394：B 方案定案的 `{}` = 空块 Void，
