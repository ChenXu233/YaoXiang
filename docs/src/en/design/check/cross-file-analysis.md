---
title: Cross-file Analysis
description: Design of YaoXiang check's cross-file type checking
---

# Cross-file Analysis

## Problem Description

In the early implementation, `check_files_with_diagnostics` created an independent `Compiler` for each file, making it impossible to detect cross-file references. The `pub` function defined in fileA could not be recognized in fileB.

## Solution

Use a shared `TypeEnvironment` to check all modules in dependency order.

## Implementation Flow

```text
1. Parse all .yx files in parallel → Vec<(PathBuf, ModuleId, AST)>
2. Build dependency graph using ModuleDependencyGraph::build_from_ast
3. detect_cycles() check for circular dependencies → report error
4. topological_sort() get compilation order
5. Type check in order:
   a. Create shared TypeEnvironment (containing std module)
   b. For each module: register its exports to shared environment → type check
   c. Collect diagnostics
6. Return CheckResult
```

## Namespace Isolation

Use `module_name.symbol_name` format to store exported symbols, avoiding name conflicts for symbols with the same name in different modules.

## Known Limitations

- `traits/` placeholder implementation (coherence/impl_check/object_safety/resolution) not yet completed
- `check_single_module` still creates an independent Compiler for each module (type information propagation through shared environment not fully implemented)

## Future Work

- T8: End-to-end testing for cross-file type checking
- A4: Shared trait_table and native_signatures
