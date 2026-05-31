---
title: Cross-File Analysis
description: YaoXiang check cross-file type checking design
---

# Cross-File Analysis

## Problem Description

In the early implementation, `check_files_with_diagnostics` created an independent `Compiler` for each file, making it impossible to detect cross-file references. A `pub` function defined in fileA could not be recognized in fileB.

## Solution

Use a shared `TypeEnvironment` and check all modules in dependency order.

## Implementation Flow

```text
1. Parse all .yx files in parallel → Vec<(PathBuf, ModuleId, AST)>
2. Build dependency graph with ModuleDependencyGraph::build_from_ast
3. detect_cycles() checks for circular dependencies → report error
4. topological_sort() obtains compilation order
5. Type check in order:
   a. Create shared TypeEnvironment (including std module)
   b. For each module: register its exports to shared environment → type check
   c. Collect diagnostic information
6. Return CheckResult
```

## Namespace Isolation

Use `module_name.symbol_name` format to store exported symbols, avoiding name conflicts between symbols with the same name in different modules.

## Known Limitations

- `traits/` placeholder implementation (coherence/impl_check/object_safety/resolution) is incomplete
- `check_single_module` still creates an independent Compiler for each module (full implementation of type information propagation via shared env is not yet complete)

## Future Work

- T8: End-to-end test for cross-file type checking
- A4: Shared trait_table and native_signatures