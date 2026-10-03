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
| `W1063` | const 泛型约束无法求值 | 否 | `const 泛型约束无法求值: \`{constraint}\` ({var} = {value})` | ⚠ 暂未发射 |
| `W1080` | 编译期证明降级 | 否 | `编译期无法证明约束，已降级为运行时检查` | ⚠ 暂未发射 |

## 逐码说明

### W1001：未使用的私有函数

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的函数：'{name}'`
- **帮助**：函数从未被引用，考虑移除或添加调用；pub 函数是对外接口，不会报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:603`

### W1002：未使用的私有类型

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的类型：'{name}'`
- **帮助**：类型从未被引用，考虑移除或使用该类型；pub 类型是对外接口，不会报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:604`

### W1003：未使用的导入

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的导入：'{name}'`
- **帮助**：考虑移除未使用的导入
- **发射点**：`src/frontend/core/typecheck/checker.rs:3000`

### W1004：未使用的私有变量

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的变量：'{name}'`
- **帮助**：变量从未被引用，考虑移除或使用该变量；pub 变量是对外接口，不会报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:605`

### W1005：未使用的私有方法

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`未使用的方法：'{name}'`
- **帮助**：方法从未被引用，考虑移除或添加调用；pub 方法是对外接口，不会报此警告
- **发射点**：`src/frontend/core/typecheck/passes/dead_code.rs:606`

### W1006：本地模块遮蔽依赖包

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`本地模块 '{module}' 遮蔽了依赖包 '{dependency}'`
- **帮助**：本地模块解析优先级最高（RFC-014）：'./src/{module}' 覆盖 .yaoxiang/vendor/ 中的依赖包 '{dependency}'。若遮蔽非本意请重命名本地模块；可用 --deny-shadowing 将其升级为错误
- **发射点**：`src/frontend/module/orchestrator.rs:340`、`src/util/diagnostic/command.rs:78`

### W1063：const 泛型约束无法求值

- **类别**：`Warning`
- **span 豁免**：否
- **模板**：`const 泛型约束无法求值: \`{constraint}\` ({var} = {value})`
- **帮助**：确保 const 参数是编译期常量
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处）

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

`W1001`–`W1005` 报的是**未使用的私有**（非 `pub`）函数 / 类型 / 变量 / 方法。`pub` 声明是对外接口，编译器无从得知外部是否有消费者，因此永不触发这些告警（#321 定案 B）。若看到「未使用」的告警，去掉对应的 `pub` 即可消除；反过来，需要被外部调用的声明请保留 `pub`。
