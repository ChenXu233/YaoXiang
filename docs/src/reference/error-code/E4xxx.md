---
title: 'E4xxx：泛型与特质错误'
description: '泛型约束、特质系统与常量求值相关的错误。'
---

# E4xxx：泛型与特质错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

泛型约束、特质系统与常量求值相关的错误。

本族共 **14** 个码，全部在 `define_codes!` 注册表中，类别为 `Generic`。完整索引见[错误码首页](./index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E4001` | 泛型约束违反 | `Generic` | 否 | `类型 '{type}' 不满足特质约束 '{trait}'` | ✅ 1 处 |
| `E4002` | 特质未找到 | `Generic` | 否 | `特质 '{trait}' 未找到` | ⚠ 暂未发射 |
| `E4003` | 特质实现缺失 | `Generic` | 否 | `类型 '{type}' 缺少特质 '{trait}' 的实现` | ⚠ 暂未发射 |
| `E4004` | 特质实现冲突 | `Generic` | 否 | `特质 '{trait}' 的实现冲突` | ⚠ 暂未发射 |
| `E4005` | 关联类型未找到 | `Generic` | 否 | `在 '{container}' 中找不到关联类型 '{assoc_type}'` | ⚠ 暂未发射 |
| `E4010` | 常量除零 | `Generic` | 否 | `常量表达式除以零` | ✅ 3 处 |
| `E4011` | 常量溢出 | `Generic` | 否 | `常量表达式溢出` | ✅ 4 处 |
| `E4012` | 常量递归过深 | `Generic` | 否 | `常量求值超出最大递归深度 {limit}` | ✅ 1 处 |
| `E4014` | 常量求值失败 | `Generic` | 是 | `无法求值常量表达式：{reason}` | ✅ 10 处 |
| `E4018` | 精化谓词违反 | `Generic` | 是 | `精化谓词违反：{constraint}  反例： {counterexample}` | ✅ 6 处 |
| `E4019` | 类型等式不成立 | `Generic` | 否 | `类型等式不成立：期望 {expected}，实际 {found}` | ✅ 1 处 |
| `E4020` | 需要证明函数 | `Generic` | 否 | `需要证明函数来验证约束` | ✅ 2 处 |
| `E4021` | 循环终止性无法自动证明 | `Generic` | 否 | `无法自动证明循环终止：未找到有效的递减度量` | ✅ 1 处 |
| `E4022` | 测度不成立 | `Generic` | 否 | `测度不成立：此调用点无法证明测度严格递减  反例： {counterexample}` | ✅ 1 处 |

## 逐码说明

### E4001：泛型约束违反

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::trait_bound_not_satisfied(type_, trait_)`
- **模板**：`类型 '{type}' 不满足特质约束 '{trait}'`
- **消息**：类型不满足泛型约束
- **帮助**：确保类型满足所有必需的特质边界
- **发射点**：`src/frontend/core/typecheck/inference/bounds.rs:77`
- **源码注释名**：类型不满足特质约束

### E4002：特质未找到

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::trait_not_found(trait_)`
- **模板**：`特质 '{trait}' 未找到`
- **消息**：引用的特质不存在
- **帮助**：检查特质名或导入正确的特质
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E4003：特质实现缺失

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::missing_trait_impl(trait_, type_)`
- **模板**：`类型 '{type}' 缺少特质 '{trait}' 的实现`
- **消息**：类型未实现必需的特质
- **帮助**：为该类型添加特质实现
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E4004：特质实现冲突

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::conflicting_trait_impls(trait_)`
- **模板**：`特质 '{trait}' 的实现冲突`
- **消息**：多个特质实现冲突
- **帮助**：解决冲突的实现
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E4005：关联类型未找到

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::associated_type_not_found(assoc_type, container)`
- **模板**：`在 '{container}' 中找不到关联类型 '{assoc_type}'`
- **消息**：容器中不存在引用的关联类型
- **帮助**：检查关联类型名和容器定义
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E4010：常量除零

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::const_division_by_zero()`
- **模板**：`常量表达式除以零`
- **帮助**：确保常量表达式中的除数不为零。
- **发射点**：`src/frontend/core/types/eval/const_eval.rs:345`、`src/frontend/core/types/eval/const_eval.rs:352`、`src/frontend/core/types/eval/const_eval.rs:357`

### E4011：常量溢出

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::const_overflow()`
- **模板**：`常量表达式溢出`
- **帮助**：常量表达式的结果超出了类型的范围。
- **发射点**：`src/frontend/core/types/eval/const_eval.rs:376`、`src/frontend/core/types/eval/const_eval.rs:381`、`src/frontend/core/types/eval/const_eval.rs:386` 等 4 处

### E4012：常量递归过深

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::const_recursion_too_deep(limit)`
- **模板**：`常量求值超出最大递归深度 {limit}`
- **帮助**：简化常量表达式以减少递归。
- **发射点**：`src/frontend/core/types/eval/const_eval.rs:540`

### E4014：常量求值失败

- **类别**：`Generic`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::const_eval_failed(reason)`
- **模板**：`无法求值常量表达式：{reason}`
- **帮助**：确保表达式可以在编译期求值。
- **发射点**：`src/frontend/core/types/eval/const_eval.rs:289`、`src/frontend/core/types/eval/const_eval.rs:293`、`src/frontend/core/types/eval/const_eval.rs:314` 等 10 处

### E4018：精化谓词违反

- **类别**：`Generic`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::refinement_violated(constraint)`
- **模板**：`精化谓词违反：{constraint}  反例： {counterexample}`
- **消息**：精化类型约束被证伪
- **帮助**：谓词被证伪——存在使约束为假的赋值。请检查变量值或放宽约束。
- **发射点**：`src/frontend/pipeline.rs:324`、`src/frontend/pipeline.rs:334`、`src/frontend/core/typecheck/checker.rs:4861` 等 6 处

### E4019：类型等式不成立

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::type_mismatch_in_proof(expected, found)`
- **模板**：`类型等式不成立：期望 {expected}，实际 {found}`
- **消息**：类型等式在证明管道内证伪
- **帮助**：检查组件的类型计算或提供证明函数。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:141`
- **源码注释名**：类型等式不成立（证明管道内）

### E4020：需要证明函数

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::proof_function_required()`
- **模板**：`需要证明函数来验证约束`
- **消息**：约束无法在编译期证明，需要提供证明函数
- **帮助**：添加证明函数或提供运行时检查
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:288`
- **历史**：原列出的 `src/frontend/core/typecheck/layers/dispatch.rs:145` 随该模块在 #377-2 删除而消失（零生产调用点，与生产实做的分派分叉）
- **源码注释名**：需要证明函数来验证约束

### E4021：循环终止性无法自动证明

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::loop_termination_unproven()`
- **模板**：`无法自动证明循环终止：未找到有效的递减度量`
- **消息**：编译器无法为该循环合成递减度量，无法自动证明其终止
- **帮助**：这是编译器自动推理的覆盖边界，并非程序错误。请改用带明确递减变量的循环形式（如每轮减小一个有下界的计数变量），或参考 RFC-027 §7 的度量策略与覆盖范围。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:284`

### E4022：测度不成立

- **类别**：`Generic`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::measure_not_decreasing(counterexample)`
- **模板**：`测度不成立：此调用点无法证明测度严格递减  反例： {counterexample}`
- **消息**：该递归调用点的测度未能严格递减，无法证明递归终止（已附反例）
- **帮助**：这是精化类型主张的终止性义务：带 `Terminates` 测度的递归函数，每个递归调用点都必须证明测度严格递减。反例给出了一组使递减不成立的取值。请检查测度选择是否恰当，或补充分支守卫使递归路径上的下降成立。参考 RFC-027a。
- **发射点**：`src/frontend/core/typecheck/proof/verdict.rs:126`
- **源码注释名**：测度不成立（RFC-027a §义务生成）
