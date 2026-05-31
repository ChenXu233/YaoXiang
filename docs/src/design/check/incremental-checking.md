---
title: 增量检查
description: YaoXiang check 增量检查的设计
---

# 增量检查

## 问题描述

watch 模式下，任何文件变更都重新检查所有文件（全量重检），且防抖使用 busy-wait（每 50ms 检查一次），CPU 空转。

## 解决方案

使用 `CheckSession` 管理增量检查状态，利用 `ModuleDependencyGraph::affected_modules` 只重检查受影响的文件。

## 实现流程

```text
首次检查：
  全量检查 → 缓存依赖图 + 每个模块的检查结果

文件变更：
  1. affected_modules(changed_files) → 找出受影响模块
  2. 只重新解析和检查受影响模块
  3. 更新缓存和依赖图
```

## CheckSession

```rust
pub struct CheckSession {
    dep_graph: ModuleDependencyGraph,
    cache: ModuleCache,
    all_files: Vec<PathBuf>,
}

impl CheckSession {
    pub fn check_all(&mut self, files: &[PathBuf]) -> Result<CheckResult>;
    pub fn check_incremental(&mut self, changed_files: &[PathBuf]) -> Result<CheckResult>;
}
```

## 已知限制

- watch 模式仍使用 busy-wait 防抖（`command.rs` 中的 `Instant::now()` + `recv_timeout`）
- `check_incremental` 内部仍调用 `check_files_with_diagnostics`（全量路径），未真正利用增量

## 未来工作

- A2/P1：用 `HotReloader` 替换 busy-wait 防抖
- P2/P3：watch 模式接入 `CheckSession` 实现真正的增量检查
- T9：增量检查正确性测试
