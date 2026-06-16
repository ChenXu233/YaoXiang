---
title: "spawn for 语法正交性悬置"
status: "讨论中"
created: "2026-06-16"
---

# spawn for 语法正交性

## 问题

当前 `spawn for x in items { body }` 是独立的关键词组合（`spawn` + `for`），与其他构造的 `spawn { }` 模式不一致：

```yx
spawn { }              ← spawn 后置绑定 {} 块
spawn for { }          ← spawn 后置绑定 for？这会形成 spawn for 组合关键词
for ... spawn { }      ← spawn 后置绑定 {}，for 的 body = spawn
```

核心问题：**`spawn for` 和 `spawn { for }` 的语义等价性，以及 `for spawn { }` 与 `spawn { for }` 的语义差异。**

## 三种形式的语义

| 形式 | 语义 | DAG 分解 |
|------|------|----------|
| `spawn for x in items { body }` | 每个迭代是独立并发任务（数据并行） | 编译器将 N 个迭代展开为 N 个闭包 |
| `spawn { for x in items { body } }` | for 循环作为一个整体在并发块内顺序执行 | 不分解 for 迭代 |
| `for x in items spawn { body }` | for 顺序迭代，每次迭代 spawn 一个任务 | 每次迭代创建一个 spawn |

三者语义不同，不能随意替换。

## DAG 分解的边界

当前 spawn 的 DAG 分析（`analyze_spawn_body`）识别的是 spawn 块内的**静态直接子表达式**——编译器可见的、数量固定的顶层表达式。for 循环的迭代数是**动态的**（运行时确定），无法在编译期展开为静态的直接子表达式列表。

因此 `spawn { for }` 无法自动等价于 `spawn for`——除非 DAG 分析能处理动态迭代展开，或者语言另有机制标记迭代间独立性。

## 正交性理想

```
spawn 修饰 {} 块 → spawn { ... }
{} 可以包含任何语句 → spawn { for ... { ... } }
```

理想状态下，`spawn { for }` 应该自动识别迭代间无依赖的循环体，展开为数据并行。但这是编译器优化问题（类似自动向量化），不是语法设计问题。

## 悬置项

- [ ] `spawn for` 是否保留为独立语法？还是退化为 `for ... spawn { }` 或 `spawn { for }`？
- [ ] 数据并行的表达是否需要专用语法，还是走标准库（如 `par_iter`）？
- [ ] 如果保留独立语法，`spawn for` 的结合优先级规则是什么？
- [ ] `spawn for` 是否支持 `else spawn` 等更复杂的组合？

## 相关

- RFC-024: 基于 spawn 块的并发模型
- 正交性讨论：`spawn` 与 `{}` 与控制流构造的结合优先性
