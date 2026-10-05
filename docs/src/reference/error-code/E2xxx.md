---
title: 'E2xxx：语义分析错误'
description: '语义分析阶段产生的错误，涵盖作用域、变量生命周期、所有权与函数签名解析等。'
---

# E2xxx：语义分析错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

语义分析阶段产生的错误，涵盖作用域、变量生命周期、所有权与函数签名解析等。

本族共 **23** 个码，全部在 `define_codes!` 注册表中，类别为 `Semantic`。完整索引见[错误码首页](index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E2001` | 作用域错误 | `Semantic` | 否 | `变量 '{name}' 不在作用域中` | ⚠ 暂未发射 |
| `E2002` | 重复定义 | `Semantic` | 否 | `重复定义：'{name}' 已在当前作用域中定义` | ✅ 4 处 |
| `E2003` | 所有权错误 | `Semantic` | 否 | `所有权约束违反：{reason}` | ✅ 1 处 |
| `E2010` | 不可变赋值 | `Semantic` | 否 | `无法给不可变变量 '{name}' 赋值` | ✅ 2 处 |
| `E2011` | 使用未初始化变量 | `Semantic` | 否 | `使用未初始化的变量 '{name}'` | ⚠ 暂未发射 |
| `E2012` | 可变性冲突 | `Semantic` | 否 | `可变性冲突：无法在不可变上下文中使用可变引用` | ⚠ 暂未发射 |
| `E2013` | 变量遮蔽 | `Semantic` | 否 | `Cannot shadow existing variable '{name}'` | ✅ 3 处 |
| `E2014` | 使用已移动的值 | `Semantic` | 否 | `'{name}' 已被移动，无法再次使用` | ✅ 1 处 |
| `E2016` | 不可变赋值 | `Semantic` | 否 | `无法给不可变变量 '{name}' 赋值` | ✅ 1 处 |
| `E2018` | 可变/不可变借用冲突 | `Semantic` | 否 | `无法可变借用 '{name}'（已被不可变借用）` | ✅ 1 处 |
| `E2019` | 双重释放 | `Semantic` | 否 | `值 '{name}' 被释放了两次` | ✅ 1 处 |
| `E2020` | 释放后使用 | `Semantic` | 否 | `'{name}' 在释放后被使用` | ✅ 1 处 |
| `E2027` | unsafe 解引用 | `Semantic` | 否 | `无法在 unsafe 块外解引用裸指针` | ✅ 1 处 |
| `E2029` | spawn 内引用循环 | `Semantic` | 否 | `spawn 内引用循环: {cycle}` | ✅ 1 处 |
| `E2030` | 精化类型约束违反 | `Semantic` | 否 | `对 '{assigned}' 赋值后，'{var}' 不再满足精化类型约束：{constraint}（反例：{counterexample}）` | ✅ 1 处 |
| `E2031` | 精化约束无法证明 | `Semantic` | 否 | `对 '{assigned}' 赋值后，'{var}' 的精化类型约束：{constraint} 在证明内核内无法静态证明` | ✅ 3 处 |
| `E2090` | 无效签名 | `Semantic` | 否 | `无效签名：{reason}` | ✅ 3 处 |
| `E2091` | 签名未知类型 | `Semantic` | 否 | `无效签名：未知类型 '{type_name}'` | ⚠ 暂未发射 |
| `E2092` | 签名缺少箭头 | `Semantic` | 否 | `无效签名：缺少 '->'` | ✅ 1 处 |
| `E2093` | 重复参数名 | `Semantic` | 否 | `无效签名：重复参数名 '{name}'` | ✅ 2 处 |
| `E2094` | 泛型参数遮蔽 | `Semantic` | 否 | `无效签名：泛型参数 '{name}' 遮蔽了外层泛型参数` | ⚠ 暂未发射 |
| `E2095` | 参数名遮蔽泛型 | `Semantic` | 否 | `无效签名：参数名 '{name}' 遮蔽了泛型参数` | ✅ 1 处 |
| `E2096` | 签名裸容器类型 | `Semantic` | 否 | `无效签名：容器类型 '{name}' 缺少类型实参` | ✅ 1 处 |

## 逐码说明

### E2001：作用域错误

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::variable_not_in_scope(name)`
- **模板**：`变量 '{name}' 不在作用域中`
- **消息**：变量不在当前作用域
- **帮助**：检查变量的作用域，确保它在这里可访问
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：变量不在作用域中

### E2002：重复定义

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::duplicate_definition(name)`
- **模板**：`重复定义：'{name}' 已在当前作用域中定义`
- **消息**：变量在同一作用域中定义了多次
- **帮助**：重命名或删除重复的定义
- **发射点**：`src/frontend/core/typecheck/checker.rs:1507`、`src/frontend/core/typecheck/inference/expressions.rs:5197`、`src/frontend/core/typecheck/inference/statements.rs:1545` 等 4 处

### E2003：所有权错误

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::ownership_violation(reason)`
- **模板**：`所有权约束违反：{reason}`
- **消息**：所有权约束未满足
- **帮助**：检查值的所有权语义
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:215`
- **源码注释名**：所有权约束违反

### E2010：不可变赋值

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::immutable_assignment(name)`
- **模板**：`无法给不可变变量 '{name}' 赋值`
- **消息**：试图修改不可变变量
- **帮助**：使用 'mut' 声明可变变量
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:5192`、`src/frontend/core/typecheck/inference/statements.rs:2218`

### E2011：使用未初始化变量

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::uninitialized_variable(name)`
- **模板**：`使用未初始化的变量 '{name}'`
- **消息**：使用了未初始化的变量
- **帮助**：使用前初始化变量
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E2012：可变性冲突

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::mutability_conflict()`
- **模板**：`可变性冲突：无法在不可变上下文中使用可变引用`
- **消息**：在不可变上下文中使用可变引用
- **帮助**：确保引用的可变性匹配使用上下文
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E2013：变量遮蔽

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::variable_shadowing(name)`
- **模板**：`Cannot shadow existing variable '{name}'`
- **消息**：不能遮蔽已有变量
- **帮助**：使用不同的变量名，或在不同的作用域中声明变量
- **发射点**：`src/frontend/core/typecheck/inference/expressions.rs:5202`、`src/frontend/core/typecheck/inference/statements.rs:2232`、`src/frontend/core/typecheck/inference/statements.rs:2308`

### E2014：使用已移动的值

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::use_after_move(name)`
- **模板**：`'{name}' 已被移动，无法再次使用`
- **帮助**：该值已被移动到新的所有者。如需再次使用，请使用引用或克隆。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:163`
- **源码注释名**：使用已移动的变量

### E2016：不可变赋值

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::immutable_assign(name)`
- **模板**：`无法给不可变变量 '{name}' 赋值`
- **帮助**：使用 `mut` 声明变量以允许赋值。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:193`
- **源码注释名**：不可变赋值（所有权检查器用）

### E2018：可变/不可变借用冲突

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::mutable_immutable_borrow_conflict(name)`
- **模板**：`无法可变借用 '{name}'（已被不可变借用）`
- **帮助**：确保在创建可变引用前没有不可变引用存在。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:153`

### E2019：双重释放

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::double_drop(name)`
- **模板**：`值 '{name}' 被释放了两次`
- **帮助**：确保值只被释放一次。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:183`

### E2020：释放后使用

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::use_after_drop(name)`
- **模板**：`'{name}' 在释放后被使用`
- **帮助**：在值被释放前使用它，或重构代码。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:173`

### E2027：unsafe 解引用

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unsafe_deref()`
- **模板**：`无法在 unsafe 块外解引用裸指针`
- **帮助**：将解引用操作包裹在 `unsafe` 块中。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:200`

### E2029：spawn 内引用循环

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::spawn_ref_cycle(cycle)`
- **模板**：`spawn 内引用循环: {cycle}`
- **消息**：spawn 块内检测到引用形成循环
- **帮助**：spawn 捕获的引用不能形成循环依赖，请检查捕获列表
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:208`
- **源码注释名**：spawn 内 ref 循环

### E2030：精化类型约束违反

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::refined_constraint_violated(assigned, var, constraint, counterexample)`
- **模板**：`对 '{assigned}' 赋值后，'{var}' 不再满足精化类型约束：{constraint}（反例：{counterexample}）`
- **帮助**：精化类型标注要求依赖变量在相关赋值之后仍然满足谓词约束。请调整赋值使约束成立，或放宽类型标注。
- **发射点**：`src/frontend/core/typecheck/checker.rs:4874`
- **源码注释名**：精化类型约束违反（赋值后依赖变量 VC 被 SMT 证伪——

### E2031：精化约束无法证明

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::refined_unproven(assigned, var, constraint)`
- **模板**：`对 '{assigned}' 赋值后，'{var}' 的精化类型约束：{constraint} 在证明内核内无法静态证明`
- **帮助**：RFC-027 要求所有精化约束在编译期得到证明——证不出即编译错误。请改写约束使其实参可静态取值（字面量或字面量可折叠的表达式），或提供返回 Type 的证明函数参与编译期执行。
- **发射点**：`src/frontend/core/typecheck/checker.rs:4895`、`src/frontend/core/typecheck/checker.rs:5033`、`src/frontend/core/typecheck/checker.rs:5143`
- **源码注释名**：精化约束无法证明（RFC-027 §4/§9：Unproven → 编译错误 + 未解命题，

### E2090：无效签名

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_signature(reason)`
- **模板**：`无效签名：{reason}`
- **消息**：函数签名解析失败
- **帮助**：检查函数签名格式是否正确
- **发射点**：`src/frontend/core/typecheck/signature.rs:58`、`src/frontend/core/typecheck/signature.rs:63`、`src/frontend/core/typecheck/signature.rs:459`
- **源码注释名**：签名解析失败（通用）

### E2091：签名未知类型

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_signature_unknown_type(type_name)`
- **模板**：`无效签名：未知类型 '{type_name}'`
- **消息**：签名中包含未知类型
- **帮助**：使用有效类型，如 '(T) -> T' 表示函数参数
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：未知类型

### E2092：签名缺少箭头

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_signature_missing_arrow()`
- **模板**：`无效签名：缺少 '->'`
- **消息**：签名中缺少 '->'
- **帮助**：确保签名格式为 '(参数列表) -> 返回类型'
- **发射点**：`src/frontend/core/typecheck/signature.rs:71`
- **源码注释名**：缺少箭头

### E2093：重复参数名

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_signature_duplicate_param(name)`
- **模板**：`无效签名：重复参数名 '{name}'`
- **消息**：签名中存在重复的参数名
- **帮助**：确保每个参数名是唯一的
- **发射点**：`src/frontend/core/typecheck/signature.rs:51`、`src/frontend/core/typecheck/signature.rs:84`

### E2094：泛型参数遮蔽

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_signature_generic_shadows(name)`
- **模板**：`无效签名：泛型参数 '{name}' 遮蔽了外层泛型参数`
- **消息**：泛型参数遮蔽了外层泛型参数
- **帮助**：为内层函数类型使用不同的泛型参数名
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E2095：参数名遮蔽泛型

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_signature_param_shadows_generic(name)`
- **模板**：`无效签名：参数名 '{name}' 遮蔽了泛型参数`
- **消息**：参数名与泛型参数同名
- **帮助**：参数名不能与泛型参数同名
- **发射点**：`src/frontend/core/typecheck/signature.rs:92`

### E2096：签名裸容器类型

- **类别**：`Semantic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_signature_bare_container(name)`
- **模板**：`无效签名：容器类型 '{name}' 缺少类型实参`
- **消息**：签名中的容器类型缺少类型实参
- **帮助**：容器类型是泛型类型构造器（SPEC type-system §4.1.1），裸名不是合法类型表达式。写成参数化形态：List(T)、Dict(K, V)、Vec(T)、Array(T, N)、Set(T)、Tuple(T, ...)
- **发射点**：`src/frontend/core/typecheck/signature.rs:452`
- **源码注释名**：签名裸容器类型（#391：容器是泛型构造器，SPEC §4.1.1，构造期拒绝）
