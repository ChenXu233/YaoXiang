---
title: '警告码'
description: '编译器警告码及说明'
---

# 警告码

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

警告码以 `W` 开头，不会阻止编译，但提示代码中可能存在的问题。当前注册表共 **8** 个警告码，完整索引见[错误码首页](../error-code/index.md)。

## 配置

可以通过 `yaoxiang.toml` 配置死代码警告的行为：

```toml
[lint]
# 死代码警告级别：off | warn | deny
dead-code = "warn"
```

- `off`：禁用警告
- `warn`：显示警告（默认）
- `deny`：将警告视为错误，阻止编译

## 码一览

| 码号 | 中文名 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- |
| `W1001` | 未使用的私有函数 | 否 | `未使用的函数：'{name}'` | ✅ 1 处 |
| `W1002` | 未使用的私有类型 | 否 | `未使用的类型：'{name}'` | ✅ 1 处 |
| `W1003` | 未使用的导入 | 否 | `未使用的导入：'{name}'` | ✅ 1 处 |
| `W1004` | 未使用的私有变量 | 否 | `未使用的变量：'{name}'` | ✅ 1 处 |
| `W1005` | 未使用的私有方法 | 否 | `未使用的方法：'{name}'` | ✅ 1 处 |
| `W1006` | 本地模块遮蔽依赖包 | 否 | `本地模块 '{module}' 遮蔽了依赖包 '{dependency}'` | ✅ 2 处 |
| `W1063` | const 泛型约束无法求值 | 否 | `const 泛型约束无法求值: \`{constraint}\`` | ✅ 1 处汇聚发射 |
| `W1081` | 终止性义务未判定 | 否 | `{count} 条终止性测度义务未判定（SMT 求解器不可用）` | ✅ 1 处汇聚发射 |
| `W1080` | 编译期证明降级 | 否 | `编译期无法证明约束，已降级为运行时检查` | ⚠ 暂未发射 |

## 逐码说明

### W1001：未使用的函数

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的函数：'{name}'`
- **帮助**：函数从未被引用，考虑移除或添加调用；被包内其它文件引用时不报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:558`

### W1002：未使用的类型

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的类型：'{name}'`
- **帮助**：类型从未被引用，考虑移除或使用该类型；被包内其它文件引用时不报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:559`

### W1003：未使用的导入

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的导入：'{name}'`
- **帮助**：考虑移除未使用的导入
- **发射点**：`src/frontend/core/typecheck/checker.rs:3000`

### W1004：未使用的变量

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的变量：'{name}'`
- **帮助**：变量从未被引用，考虑移除或使用该变量；被包内其它文件引用时不报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:560`

### W1005：未使用的方法

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的方法：'{name}'`
- **帮助**：方法从未被引用，考虑移除或添加调用；被包内其它文件引用时不报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:561`

### W1006：本地模块遮蔽依赖包

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`本地模块 '{module}' 遮蔽了依赖包 '{dependency}'`
- **帮助**：本地模块解析优先级最高（RFC-014）：'./src/{module}' 覆盖 .yaoxiang/vendor/ 中的依赖包 '{dependency}'。若遮蔽非本意请重命名本地模块；可用 --deny-shadowing 将其升级为错误
- **发射点**：`src/frontend/module/orchestrator.rs:340`、`src/util/diagnostic/command.rs:78`

### W1063：const 泛型约束无法求值

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`const 泛型约束无法求值: \`{constraint}\``
- **帮助**：确保 const 参数是编译期常量
- **发射点**：✅ WBS 3.4.1（2026-10-07）接线——事实链 `environment.rs`（Unproven 臂收集）→ `ExpressionInferrer.unevaluable_const_constraints` → `StatementChecker` 回收 → `checker.rs` 汇聚发射。模板尾段 `({var} = {value})` 原为无注册参数的占位，接线时收敛为单 `{constraint}` 参数（约束描述携带 binder 与实参信息）

### W1081：终止性义务未判定

- **类别**：`Warning`
- **span 豁免**：是（模块级汇总诊断，无单点 span）
- **模板**：`{count} 条终止性测度义务未判定（SMT 求解器不可用）`
- **帮助**：安装 Z3 求解器后重新检查，以获得终止性判定；「未判定」不等于「不终止」，但义务的判定被跳过了
- **发射点**：✅ WBS 3.4.2（2026-10-07）接线——`TerminationChecker` 的 Unjudged 判定（无求解器时「只记录不发射」的静默通道）经 `count_unjudged_obligations` 计数，`checker.rs` 汇聚发射。求解器在场时的 NotProved（判不出）不触发本码——它是求解器**缺失**的专属信号

### W1080：编译期证明降级

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`编译期无法证明约束，已降级为运行时检查`
- **帮助**：考虑添加证明函数以提高安全性
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处）
- **历史**：唯一构造点原在 `src/frontend/core/typecheck/layers/dispatch.rs:232`。该模块与生产
  已实现的调用点义务分叉（生产在 `checker.rs` 直接调 `check_predicate`，`Unproven` 一律报错而非
  降级为警告），零生产调用点，已于 #377-2 整体删除，故本码在用户流程中不可达。保留注册与
  翻译，避免码位被后续功能重用。

## 警告级别

| 级别 | 效果 |
| --- | --- |
| `off` | 完全禁用此类警告 |
| `warn` | 显示警告但继续编译（默认） |
| `deny` | 将警告视为错误，阻止编译 |

## 与错误码的区别

- **错误（`E` 前缀）**：阻止编译，必须修复。
- **警告（`W` 前缀）**：提示潜在问题，可选择修复。

## 死代码告警的判定口径

`W1001`–`W1005` 报的是**未被引用的顶层绑定与类型定义**。RFC-029g 已删除 `pub` 关键字，没有任何修饰符能豁免这些告警；唯一豁免口是**包内引用池**（RFC-029f）：项目内任一文件引用到的名字视为活——跨文件消费者在本文件不可见，这是「宁漏报」方向。单文件（Script）路径没有包视角，未引用的顶层绑定一律报告。
