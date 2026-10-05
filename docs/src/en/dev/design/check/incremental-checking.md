---
title: 'Incremental Checking'
description: 'YaoXiang check Incremental Checking Design'
---

# Incremental Checking

## Problem Description

In watch mode, any file change triggers a full re-check of all files (full re-check), and the
debounce uses busy-wait (checking every 50ms), causing the CPU to spin idle.

## Solution

Use `CheckSession` to manage incremental checking state, leveraging
`ModuleDependencyGraph::affected_modules` to only re-check affected files.

## Implementation Flow

```text
First check:
  Full check → Cache dependency graph + check results for each module

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

- watch mode still uses busy-wait debounce (`Instant::now()` + `recv_timeout` in `command.rs`)
- `check_incremental` internally still calls `check_files_with_diagnostics` (full-check path), not
  truly utilizing incremental checking

## Future Work

- A2/P1: Replace busy-wait debounce with `HotReloader`
- P2/P3: Integrate watch mode with `CheckSession` to achieve true incremental checking
- T9: Correctness tests for incremental checking
