---
title: "spawn for Syntax Orthogonality Suspension"
status: "Under Discussion"
created: "2026-06-16"
---

# spawn for Syntax Orthogonality

## Problem

The current `spawn for x in items { body }` is an independent keyword combination (`spawn` + `for`), which is inconsistent with the `spawn { }` pattern of other constructs:

```yx
spawn { }              ← spawn with {} block bound after
spawn for { }          ← spawn with for bound after? This forms a spawn for compound keyword
for ... spawn { }      ← spawn with {} bound after, for's body = spawn
```

Core question: **the semantic equivalence between `spawn for` and `spawn { for }`, and the semantic difference between `for spawn { }` and `spawn { for }`.**

## Semantics of the Three Forms

| Form | Semantics | DAG Decomposition |
|------|-----------|-------------------|
| `spawn for x in items { body }` | Each iteration is an independent concurrent task (data parallel) | Compiler unrolls N iterations into N closures |
| `spawn { for x in items { body } }` | The for loop as a whole executes sequentially inside the concurrent block | Does not unroll for iterations |
| `for x in items spawn { body }` | for iterates sequentially, each iteration spawns a task | Each iteration creates a spawn |

The three forms have different semantics and cannot be substituted freely.

## Boundaries of DAG Decomposition

The current DAG analysis of spawn (`analyze_spawn_body`) identifies the **statically direct sub-expressions** inside the spawn block—top-level expressions that are visible to the compiler and have a fixed quantity. The number of iterations of a for loop is **dynamic** (determined at runtime), and cannot be unrolled at compile-time into a static list of direct sub-expressions.

Therefore, `spawn { for }` cannot automatically be equivalent to `spawn for`—unless the DAG analysis can handle dynamic iteration unrolling, or the language provides another mechanism to mark inter-iteration independence.

## The Orthogonality Ideal

```
spawn modifies {} block → spawn { ... }
{} can contain any statement → spawn { for ... { ... } }
```

Ideally, `spawn { for }` should automatically identify loops whose iterations have no inter-iteration dependencies and unroll them into data-parallel form. But this is a compiler optimization problem (similar to auto-vectorization), not a syntax design problem.

## Suspended Items

- [ ] Should `spawn for` be kept as standalone syntax? Or should it degrade to `for ... spawn { }` or `spawn { for }`?
- [ ] Does the expression of data parallelism require dedicated syntax, or should it go through the standard library (e.g., `par_iter`)?
- [ ] If standalone syntax is kept, what are the binding precedence rules for `spawn for`?
- [ ] Does `spawn for` support more complex combinations like `else spawn`?

## Related

- RFC-024: Concurrent Model Based on spawn Blocks
- Orthogonality Discussion: Binding Precedence between `spawn`, `{}`, and Control Flow Constructs