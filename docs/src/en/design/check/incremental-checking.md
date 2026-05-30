---
title: Incremental Checking
description: YaoXiang check incremental checking design
---

# Incremental Checking

## Problem Statement

In watch mode, any file change triggers a re-check of all files (full re-check), and debouncing uses busy-wait (checking every 50ms), causing CPU idle spinning.

## Solution

Use `CheckSession` to manage incremental checking state, utilizing `ModuleDependencyGraph::affected_modules` to only re-check affected files.

## Implementation Flow

```text
Initial check:
  Full check → Cache dependency graph + check result for each module

File change:
  1. affected_modules(changed_files) → Find affected modules
  2. Only re-parse and check affected modules
  3. Update cache and dependency graph
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

## Known Limitations

- Watch mode still uses busy-wait debouncing (`Instant::now()` + `recv_timeout` in `command.rs`)
- `check_incremental` internally still calls `check_files_with_diagnostics` (full path), not truly leveraging incremental checking

## Future Work

- A2/P1: Replace busy-wait debouncing with `HotReloader`
- P2/P3: Integrate watch mode with `CheckSession` to achieve true incremental checking
- T9: Incremental checking correctness tests