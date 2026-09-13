---
title: 'Cross-file Analysis'
description: 'YaoXiang check cross-file type checking design'
---

# Cross-file Analysis

## Problem Description

In early implementations, `check_files_with_diagnostics` created an independent `Compiler` for each
file, making it impossible to detect cross-file references. `pub` functions defined in fileA could
not be recognized in fileB.

## Solution

Use a shared `TypeEnvironment` and check all modules in dependency order.

## Implementation Flow

```text
1. Parse all .yx files in parallel → Vec<(PathBuf, ModuleId, AST)>
2. Build the dependency graph using ModuleDependencyGraph::build_from_ast
3. detect_cycles() checks for circular dependencies → report errors
4. topological_sort() gets the compilation order
5. Type check in order:
   a. Create a shared TypeEnvironment (including std module)
   b. For each module: register its exports into the shared environment → type check
   c. Collect diagnostics
6. Return CheckResult
```

## Namespace Isolation

Use the `module_name.symbol_name` format to store exported symbols, avoiding conflicts between
symbols with the same name in different modules.

## Known Limitations

- `traits/` placeholder implementations (coherence/impl_check/object_safety/resolution) are
  incomplete
- `check_single_module` still creates an independent Compiler for each module (passing type
  information from the shared env has not been fully implemented)

## Future Work

- T8: End-to-end test for cross-file type checking
- A4: Shared trait_table and native_signatures
