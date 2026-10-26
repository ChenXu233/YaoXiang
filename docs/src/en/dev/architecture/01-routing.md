# Feature Routing and Dependency Rules

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) and
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing tables, and anti-regression mechanisms** within the four-layer model.
>
> The four-layer model itself, glossary, execution phases, and acceptance criteria are covered in
> the RFC-039 main document. See the remaining documents in this directory for detailed layer
> designs.

## Dependency Direction Rules

The following rules are enforced by CI checks (see [Anti-Regression Mechanism](#防反弹机制)):

| Rule                                                             | How It Is Checked                                                                                                         | Current Status                                                                                                                                                  |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `include!` is prohibited                                         | `grep -rn 'include!' src/` must return 0                                                                                  | 0 occurrences (eliminated in 5.1: `checker.rs` now uses a real `mod`)                                                                                           |
| L2 must not `use` L3                                             | Scan `src/frontend/core/{lexer,parser}/` for `use ...typecheck`                                                           | 2 occurrences exist and must be removed (see routing table C)                                                                                                   |
| L3/L4 must not `use` L2                                          | Scan `sema/`, `proof/`, `middle/`, and `backends/` for `use ...frontend::{lexer,parser,module}`                           | Currently 0; use 0 as the baseline (AST is a top-level domain and requires no exemption)                                                                        |
| L4 must not reference L1/L2 in reverse                           | Scan `src/backends/` and `src/middle/` for `use ...frontend::core::{lexer,parser}`                                        | A baseline must be established first                                                                                                                            |
| The opcode vocabulary belongs exclusively to `middle/bytecode/`  | Scan `src/backends/` to ensure it defines no opcode constants (import only); ensure `sema/` contains no opcode references | The vocabulary currently resides in `backends/common/opcode.rs` (forming an L3→L4 reverse dependency) and will be migrated when the bytecode domains are merged |
| Cross-layer `pub(crate)` leaks may only decrease, never increase | Count and include in the baseline files                                                                                   | At least 1 occurrence (`collect_used_in_type` is used by `inference/statements.rs:16`)                                                                          |
| Hard-coded type and predicate names are prohibited               | Scan for type-name string literals within `src/frontend/core/parser/`                                                     | For example, `"Terminates"`                                                                                                                                     |

## Target Directory Structure (After Completion)

The following is the structure after **all implementation phases have been completed**, not the
target of the first step. See the mapping table below for the differences between the current and
target structures.

```
src/
├── driver/                        # L1 orchestration: the sole decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustively matchable) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-stage diagnostic aggregation + exit-code determination
│
├── frontend/                      # L2 lexing and parsing
│   ├── lexer/                     #   Declarative: numbers unified as scan_radix(base),
│   │   ├── mod.rs                 #     sole escape-decoding implementation (merge the 3 copies in literals.rs)
│   │   ├── number.rs              #     replaces the 4 radix scanners with identical line-by-line logic
│   │   ├── escape.rs              #     sole escape decoder
│   │   ├── string.rs              #     strings / multiline strings
│   │   ├── fstring.rs             #     f-strings; interpolation advances the parser rather than nesting tokenization
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: one precedence table, no bare magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     grammar definitions (added after the grammar-driven implementation)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder node
│   └── module/                    #   Multiple files: discovery / registry / roles
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merge ~60 lines of near-duplicate code)
│       └── discover.rs            #     discovery strategy that follows use statements
│
├── ast/                           # Sole AST + sole type carrier (top-level domain, peer to parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after consolidation; see 03 for handling dead variants)
│   ├── stmt.rs                    #   StmtKind (remove signature_params)
│   ├── type_.rs                   #   Sole type structure Type + NameKind (26 → consolidated variant set)
│   └── pattern.rs
│
├── sema/                          # L3 upper half: types and static analysis (currently typecheck/)
│   ├── types/                     #   MonoType working representation + solving (see the two roles in 03)
│   │   ├── mod.rs
│   │   ├── repr.rs                #     MonoType definition (working representation)
│   │   ├── universe.rs            #     UniverseLevel (currently in mono.rs)
│   │   ├── infer.rs               #     inference entry point
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     constant evaluation (includes the sole const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     sole ConstExpr; sole source of operators (merge the current 2 sets)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   Expression / statement inference
│   │   ├── mod.rs                 #     location of StatementChecker
│   │   ├── call/                  #     split infer_call_expr (see the split axes in 03/04)
│   │   │   ├── mod.rs             #       dispatch by "callee form"
│   │   │   ├── construct.rs       #       sum-type construction / generic type construction
│   │   │   ├── generic.rs         #       explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (sole three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level checks (after splitting checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     annotation type-name validation + is_predicate_head (a genuine submodule of the current checker)
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     imports/exports / unused imports
│   │   └── semantic_tokens.rs     #     semantic token collection (a genuine submodule of the current checker)
│   ├── refine/                    #   Refinement types (currently the single checker/refinement.rs file, 1107 lines)
│   │   ├── mod.rs                 #     collect_refined_binding_checks
│   │   ├── binding.rs             #     revalidate_refined
│   │   ├── call_arg.rs            #     check_call_arg_refinements
│   │   ├── return_.rs             #     check_return_refinement
│   │   └── walk.rs                #     refined_walk_* + RefinedWalkCtx
│   └── layers/                    #   Cross-cutting analyses
│       ├── README.md              #     must describe the actual layer order, not the intended layer order
│       ├── ownership.rs
│       ├── termination.rs
│       ├── predicate.rs
│       └── equivalence.rs
│
├── proof/                         # L3: proofs and verification support (promoted from typecheck/proof/ to top level; the driver obligation ledger holds proof_calls, and three sema layers share the SMT backend, so it is promoted for cross-layer consumption)
│   ├── mod.rs                     #   ProofResult / ProofContext / ProofFunctionCall
│   ├── context.rs
│   ├── verdict.rs
│   ├── dep_graph.rs
│   └── smt/                       #   Solver trait + Z3 backend + gate
│       ├── backend.rs
│       ├── z3_backend.rs
│       └── translate.rs
│
├── middle/                        # L3 lower half: IR
│   ├── ir/                        #   SSA IR (see 04)
│   │   ├── mod.rs
│   │   ├── operand.rs             #     Operand (Temp/Label/Register dead variants removed)
│   │   ├── instr.rs               #     Instruction (including Phi)
│   │   ├── block.rs               #     BasicBlock
│   │   ├── fn_.rs                 #     FunctionIR / LocalSlot
│   │   └── verify.rs              #     verify_loose / verify_ssa
│   ├── lower/                     #   AST → IR (after splitting ir_gen.rs)
│   │   ├── mod.rs                 #     AstToIrGenerator
│   │   ├── scope.rs               #     scopes and register frames (RAII)
│   │   ├── module_ir.rs
│   │   ├── const_eval.rs
│   │   ├── globals.rs
│   │   ├── stmt.rs
│   │   ├── curry.rs
│   │   ├── loops.rs
│   │   ├── query.rs               #     pure query utilities
│   │   ├── expr/                  #     expression lowering
│   │   └── synth.rs               #     sole place permitted to construct ast::Expr
│   ├── passes/                    #   Passes on IR
│   │   ├── mono/                  #     monomorphization (remove 415 lines of residual closure code)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 stack slots (no real register file)
│   └── bytecode/                  #   Bytecode domain: opcode vocabulary + containers + encoding/decoding
│       ├── opcode.rs              #     sole opcode vocabulary (+ generation-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file to facilitate cross-checking)
│       ├── decode/                #     decoding (split from the current 1400-line From)
│       │   ├── mod.rs
│       │   ├── control.rs
│       │   ├── arith.rs
│       │   ├── call.rs
│       │   ├── concurrency.rs
│       │   ├── string.rs
│       │   ├── slot.rs
│       │   ├── aggregate.rs
│       │   ├── closure.rs
│       │   ├── rc.rs
│       │   └── field.rs
│       └── typeconv.rs
│
├── backends/                      # L4 execution
│   ├── common/
│   │   ├── heap.rs                #   Shared within L4: heap layout
│   │   └── value.rs               #   Shared within L4: runtime values
│   ├── interpreter/               #   Interpreter (sole Executor implementation)
│   │   ├── executor.rs
│   │   ├── debug.rs               #   opcode dispatch table
│   │   ├── ffi.rs
│   │   └── ops/                   #   split by instruction family
│   └── runtime/                   #   DAG task scheduler (not a second execution backend)
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 standard library (Rust implementation + mixed .yx)
├── util/                          # Cross-layer utilities (diagnostics, span, cache, i18n; retain existing names—the Go "avoid util" reference in 08 applies moderately, and renaming would add pure churn with no domain benefit, so record it as an exception)
├── formatter/                     # Formatting
├── lsp/                           # Language server
├── repl/                          # REPL (if retained, see section D of 06)
└── package/                       # Package management
```

### Responsibilities and Boundary Invariants of Each Directory

| Directory                                 | Responsibility                                                                         | Invariant                                                                                                                                                           |
| ----------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Determines which stages to run, how many times to run them, and how failures propagate | **Must not** be reverse-depended on by L2/L3/L4; the set of stages must be exhaustively enumerable                                                                  |
| `frontend/lexer`, `frontend/parser`       | Lexing and parsing; constructs `ast/` nodes                                            | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics must all be passed down as data |
| `ast/`                                    | Sole AST and type representation (top-level shared data domain, peer to `parser`)      | **Must not** contain a second definition of the same concept; must not `use` other layers (pure data and pure functions only)                                       |
| `frontend/module/`                        | Multi-file discovery, registry, and roles                                              | Performs discovery and registration only; **does not run analysis stages**; analysis is orchestrated by `driver`                                                    |
| `sema/`                                   | Type checking and static analysis                                                      | **Must not** construct IR; must not be aware of opcodes                                                                                                             |
| `proof/`                                  | Proof result types and SMT backend                                                     | The Solver trait is the sole backend abstraction point; **must not** panic via `.expect()`                                                                          |
| `middle/ir/`                              | IR definitions and invariants                                                          | The IR must satisfy SSA discipline; a green `verify_ssa` is a prerequisite for merging                                                                              |
| `middle/lower/`                           | AST → IR                                                                               | The **only** place permitted to construct `ast::Expr` is `synth.rs`                                                                                                 |
| `middle/codegen/`                         | IR → bytecode lowering                                                                 | Depends only on IR, not `sema`; does not define the opcode vocabulary                                                                                               |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + encoding/decoding               | **Sole ownership of the opcode vocabulary**; `backends/` only consumes it and does not define it; `sema` is unaware of opcodes                                      |
| `backends/`                               | Execution                                                                              | **Must not** reference L1/L2 in reverse; must not be aware of the AST                                                                                               |
| `middle/passes/mono/`                     | Passes on IR                                                                           | Must not reference `frontend`                                                                                                                                       |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tool layers (outside the L1-L4 model)                                       | Consume only the `pub` APIs of L1-L4; must not touch `pub(crate)` (the leak count also constrains them)                                                             |

### Current → Target Mapping

| Current                                                                                                                                                                                      | Target                                                                                                                                                                                                                                                                                                                                                                                                                    | Owning Document |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `frontend/pipeline.rs` (740 lines, hand-written if chain across 5 stages)                                                                                                                    | `driver/` + `Program::SingleFile`                                                                                                                                                                                                                                                                                                                                                                                         | 02              |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entry points, 60% divergent)                                                                                                                | Retain only discovery/registration in `frontend/module/`; move compilation logic to `driver`                                                                                                                                                                                                                                                                                                                              | 02              |
| `frontend/compiler.rs` (267 lines)                                                                                                                                                           | Reduce to a thin wrapper around `driver`                                                                                                                                                                                                                                                                                                                                                                                  | 02              |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                                                                                         | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                                                                                                                                                                                                                                                                                                                        | 05              |
| `frontend/core/parser/pratt/` (hand-written match dispatch, 2 sets of BP ladders)                                                                                                            | `frontend/parser/grammar/` + one precedence table                                                                                                                                                                                                                                                                                                                                                                         | 05              |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                                                                                   | Split into the top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                                                                                                                                                                                                                                                                                                                         | 03 / D1         |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8-10 responsibilities)                                                                                                        | Split `frontend/parser/` + delete 120 lines of obsolete legacy syntax                                                                                                                                                                                                                                                                                                                                                     | 05              |
| `typecheck/checker.rs` (4167 lines + three genuine submodules: `checker/annotations.rs` at 470 lines, `checker/refinement.rs` at 1107 lines, and `checker/semantic_tokens.rs` at 1553 lines) | `sema/check/` + `sema/refine/`; replace `include!` with a real `mod` (completed in 5.1), move refinement blocks into submodules and consolidate the entry point (completed in 5.2: the entry point now takes `&self` and returns diagnostics), and move the annotation family into a submodule (completed in 5.3; `collect_used_in_type` belongs to the shared surface, remains in place, and requires no further export) | 02 / 05         |
| `typecheck/checker/semantic_tokens.rs` (1553 lines, already a genuine submodule of `checker`)                                                                                                | `sema/check/semantic_tokens.rs`                                                                                                                                                                                                                                                                                                                                                                                           | 02              |
| `typecheck/inference/expressions.rs` (5525 lines)                                                                                                                                            | `sema/infer/{call/,expr,stmt}.rs`                                                                                                                                                                                                                                                                                                                                                                                         | 05 / 04         |
| `typecheck/inference/statements.rs` (2887 lines)                                                                                                                                             | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                                                                                                                                                                                                                                                                                                                                | 05              |
| `typecheck/types/mono.rs` (1077 lines, includes a synonym table)                                                                                                                             | `sema/types/{repr,universe}.rs`                                                                                                                                                                                                                                                                                                                                                                                           | 03              |
| `typecheck/types/const_data.rs` (second set of operator enums)                                                                                                                               | Unify the operators with those of `ast::Type`                                                                                                                                                                                                                                                                                                                                                                             | 03              |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                                                                                                 | Delete                                                                                                                                                                                                                                                                                                                                                                                                                    | 06              |
| `typecheck/layers/dispatch.rs` (entire chain has zero callers)                                                                                                                               | Delete                                                                                                                                                                                                                                                                                                                                                                                                                    | 06              |
| `typecheck/layers/`                                                                                                                                                                          | `sema/layers/` + change `README.md` to describe the actual layer order                                                                                                                                                                                                                                                                                                                                                    | 06              |
| `typecheck/proof/`                                                                                                                                                                           | Promote to top-level `proof/`                                                                                                                                                                                                                                                                                                                                                                                             | 02              |
| `middle/core/ir_gen.rs` (8448 lines, zero tests)                                                                                                                                             | `middle/lower/` (14 modules)                                                                                                                                                                                                                                                                                                                                                                                              | 04              |
| `middle/core/ir.rs` (905 lines)                                                                                                                                                              | `middle/ir/` + `verify.rs`                                                                                                                                                                                                                                                                                                                                                                                                | 04              |
| `middle/core/bytecode.rs` (2422 lines, 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization)                                                                    | Merge into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules)                                                                                                                                                                                                                                                                                                        | 06              |
| `backends/common/opcode.rs` (83-constant vocabulary)                                                                                                                                         | `middle/bytecode/opcode.rs` (eliminate the L3→L4 reverse dependency)                                                                                                                                                                                                                                                                                                                                                      | 06              |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                                                                                    | Delete `416-830`; retain 340 lines                                                                                                                                                                                                                                                                                                                                                                                        | 06              |
| `std/fs.rs`, `std/net.rs`, and similar native API bindings                                                                                                                                   | Keep in place                                                                                                                                                                                                                                                                                                                                                                                                             | —               |
| `repl/` (1152 lines, zero tests, no echo)                                                                                                                                                    | Retain or delete; see section D of 06                                                                                                                                                                                                                                                                                                                                                                                     | 06              |

**Renaming directories is not the goal.** Changes fall into two categories:

- **Internal structures that must change**: the **internal structures** of `middle/core/ir_gen.rs`
  (8448 lines), `typecheck/checker.rs` (4167 lines + genuine submodules of 470 / 1107 / 1553 lines),
  and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: renaming.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`, and
  splitting `parser/ast.rs` into the top-level `ast/` (with the `Type` definition moved into
  `type_.rs`), plus moving the opcode vocabulary to `middle/bytecode/`, all completed together with
  P5/P6 (Decision D1). **No phased rollout and no “optional” variants.** The reason is that a
  directory name is the first signal of responsibility: `middle/core/` currently contains three
  unrelated things—`ir.rs`, `ir_gen.rs`, and `bytecode.rs`—so the name no longer conveys its
  purpose. The AST is consumed by every layer, so placing it under `frontend/` would mislead
  downstream paths. CI checks enforce the boundaries, but **the check rules reference the directory
  responsibility table, and the responsibility table must match the actual directory names**;
  otherwise, `check-boundary.py` cannot determine whether code is in the correct location.

### Naming Conventions

| Rule                                                                             | Rationale                                                                                                                                                           |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A concept has exactly one owning module                                          | Prevents parallel definitions such as “3 type representations”                                                                                                      |
| The submodule directory for `foo.rs` is named `foo/`                             | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                 |
| Cross-layer references must use `pub` APIs, not leaked `pub(crate)` items        | The **count** of `pub(crate)` leaks serves as a boundary-erosion metric and may only decrease, never increase (a semantic boundary metric, not a line-count metric) |
| Internal implementations must not use `pub` unless they expose a cross-layer API | Minimizes the visibility surface                                                                                                                                    |
| Type file names use `type_.rs` and `fn_.rs` to avoid keywords                    | `type` and `fn` are reserved words starting with Rust 2024                                                                                                          |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                         | See section D of 06: expressing a “new version” through parallel copies is a common source of dead code                                                             |

## Feature Routing Tables

These tables answer: **“I want to add X; where should I change it?”** They are the most valuable
long-term part of this document set because they do not become obsolete after any single
refactoring.

### Routing Table A: Language Features

| What to Add                         | L2 Lexing                                                         | L2 Syntax                                                                                                                                                        | L3 Semantics                                                                                                                                              | L3 IR                                        | L4 Execution                                        | Gate/Documentation                                                                                             |
| ----------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **Binary operator**                 | Keyword table in `lexer/state.rs`; add `TokenKind` to `tokens.rs` | Add a BP constant to `pratt/precedence.rs`; add an arm to `led.rs`'s `infix_info`; add a mapping to `led.rs`'s `parse_binary`; add a `BinOp` variant to `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                              | Corresponding lowering in `lower/`           | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                                  |
| **Unary operator**                  | Same as above                                                     | Add an arm to `nud.rs`'s `prefix_info` + add a new parse function + add an `UnOp` variant to `ast.rs`                                                            | `infer_unary`                                                                                                                                             | Same as above                                | Same as above                                       | RFC-010 table                                                                                                  |
| **Constant-evaluation operator** ⚠️ | —                                                                 | —                                                                                                                                                                | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` are a second set of enums parallel to `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` whatsoever** | `ir_gen.rs:2317` needs `use ast::BinOp as B` | —                                                   | **No gate**—see routing table C                                                                                |
| **Expression form**                 | —                                                                 | Add an `Expr` variant to `ast.rs` + `nud.rs`/`led.rs`                                                                                                            | Add an infer method to `inference/expressions.rs`                                                                                                         | Add a module under `lower/exprs/`            | `opcode.rs` or reuse an existing opcode             | —                                                                                                              |
| **Statement form**                  | —                                                                 | Add a `StmtKind` variant to `ast.rs` + `statements/`                                                                                                             | `inference/statements.rs`                                                                                                                                 | `lower/`                                     | Same as above                                       | —                                                                                                              |
| **Type construction**               | —                                                                 | `Type` in `ast.rs` (see 03)                                                                                                                                      | `types/mono.rs` + `solver.rs`                                                                                                                             | `FunctionCode.params` type                   | Serialization                                       | RFC-011                                                                                                        |
| **Pattern form**                    | —                                                                 | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                                  | `inference/patterns.rs`                                                                                                                                   | `compile_pattern` in `lower/`                | `opcode.rs`                                         | RFC-010b                                                                                                       |
| **Diagnostic code**                 | —                                                                 | —                                                                                                                                                                | —                                                                                                                                                         | —                                            | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (automatically validated by the `build.rs` gate) |
| **Warning (W prefix)**              | —                                                                 | —                                                                                                                                                                | —                                                                                                                                                         | —                                            | —                                                   | Same as above; note that warnings do not count toward the error total                                          |

> **Current pain point**: adding a binary operator requires changes in **6-7 places** within the
> parser/lexer, plus **17 production files** downstream that use `BinOp::`. Whether a missed change
> is detected falls into three categories:
>
> - **Caught by the compiler**: locations performing an **exhaustive `match`** on `ast::BinOp` fail
>   to compile when a change is missed. This accounts for most locations.
> - **Not caught by the compiler (the real blind spots)**: the `matches!` macro, `_` fallback arms,
>   and **cross-enum** cases. A missed change in the first two categories causes the “new operator”
>   to be treated as a different operator; the third category (`ast::BinOp` ↔ `const_data::BinOp`)
>   is completely silent because the two types compile independently.
> - **L4 is not involved**: `backends/interpreter/` has **zero references** to `BinOp` or
>   `ast::`—the interpreter never sees the AST and recognizes only opcodes. Downstream pressure is
>   therefore concentrated in L3 rather than L4.
>
> Goal: after the grammar-driven implementation, reduce this to 1-2 locations + 1 automatically
> generated location (see 05). The cross-enum portion is resolved through single-source generation
> (see 05), and the bytecode layer's third set of operator enums (`BinaryOp`/`UnaryOp`/`CompareOp`)
> is consolidated through a generation-time gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| What to Add                                                   | Location                                                                                                                                                                       | Places That Must Be Updated                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation stage**                                         | Add a `Stage` variant to `driver/stage.rs`                                                                                                                                     | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, `dispatch` in `driver/mod.rs` (an exhaustive match forces completion), and CI stage coverage tests                                                                                                                                                 |
| **Cross-stage check layer** (such as ownership / termination) | Add a module under `sema/layers/`                                                                                                                                              | Register the stage + update `layers/README.md` (**the call-order table must correspond line by line to the call sites in `checker.rs`**) + merge the output into `Obligations`                                                                                                                                               |
| **Opcode**                                                    | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`; migrate when the bytecode domains are merged and consolidate all touchpoints into one `middle/` directory) | **5 locations**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the decoding match in `From<BytecodeFile>`, and the `translate_*` functions in `translator.rs`. Only 2 are enforced by exhaustive compiler matches; the other 3 fail only after running a `.42` artifact. A generation-time gate is required (see 06) |
| **CLI subcommand**                                            | `Commands` enum in `main.rs`                                                                                                                                                   | The `pub` entry point in `lib.rs` (currently 7) and the `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                   |
| **LSP capability**                                            | Add a handler under `lsp/handlers/`                                                                                                                                            | `handle_request` in `lsp/server.rs` (currently 274 lines with 13 repeated boilerplate blocks) and the state in `lsp/world.rs`                                                                                                                                                                                                |
| **Standard-library API**                                      | `src/std/*.rs`                                                                                                                                                                 | **An automatic gate already exists**: byte-for-byte comparison by `gen_interfaces.rs` + marked-range drift detection by `gen_docs.rs`. This is the most mature pattern in the project and should be extended to the opcode and type tables                                                                                   |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions That Must Be Eliminated

| Reverse Dependency                                          | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Resolution                                                                                                                                            | Owning Document |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| parser → typecheck (**2 occurrences in the same function**) | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both of which call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Pass operator interface specifications down to the parser as data; the parser no longer needs to `use` typecheck                                      | 03              |
| Hard-coded predicate names in the parser                    | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. The comment at `:494-497` identifies this as an unresolved open-set problem: “Future open-set approach: pass predicate names down to the parser, or use a form that can be determined at the syntax layer”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | As above; pass the predicate set down as data                                                                                                         | 03              |
| **Duplication created to avoid cross-module dependencies**  | `name_used_as_type_in` at `ast.rs:847` and `name_used_as_type` at `declarations.rs:42` are synonymous but independently implemented. The comments at `ast.rs:845-846` acknowledge that the implementation is intentionally separate “to avoid internal cross-module dependencies within the parser”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Merge them into a single implementation through 03's probe data flow                                                                                  | 03              |
| checker → internal layers functions                         | The traversal family in `checker/refinement.rs`, along with `revalidate_refined` and `check_return_refinement`, calls `layers::termination::{negate_guard, substitute_const_expr}` (formerly `checker.rs:4857/4986/5246/5384`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Move them to the public API of `sema/refine/` or `sema/layers/termination`                                                                            | 02              |
| 3 parallel type representations                             | `ast::Type` (`ast.rs:427-541`), `MonoType` (`types/mono.rs`), and `ir::Type` (`ir.rs:3`, re-exported with `pub use`) + the serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Consolidate into a single representation                                                                                                              | 03              |
| Hand-written synonym table                                  | `from_builtin_name` in `types/mono.rs:618-643` maintains branches such as `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Delete it as the type representation is consolidated                                                                                                  | 03              |
| **3 parallel operator enums**                               | `pub enum BinOp` in `ast.rs:191` (20 variants) / `pub enum UnOp` in `ast.rs:217` (4, with the unique `Deref`); `pub enum BinOp` in `types/const_data.rs:234` (18, using `Ne` rather than `Neq`) / `pub enum UnOp` in `types/const_data.rs:321` (4, with the unique `BitNot`); and `BinaryOp` in `middle/core/bytecode.rs:70` (11, with yet another naming scheme for `Rem`/`Xor`/`Sar`) + `UnaryOp` at `:97` + `CompareOp` at `:106`. **There is no `From` / `TryFrom` between ast ↔ const_data**; the only bridge is the hand-written, lossy free functions in `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot`, etc. return `None` and are silently discarded). This forces 5+ files to use import aliases (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`), as seen in `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, and `middle/core/ir_gen.rs:2317` | Consolidate the two L2 sets through single-source generation (see 05); cross-check the bytecode set through a generation-time opcode gate (see 06 F8) | 03 / 05         |
| `include!`                                                  | Already eliminated (5.1: `checker.rs` now uses a real `mod semantic_tokens`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Complete                                                                                                                                              | —               |

## Guidance for Future Extensions

### Checklist for Adding a Language Feature

1. **Consult routing table A first** and identify the target location. Do not choose a file by
   intuition.
2. **Types first**: if the feature involves types, follow the single representation described in 03
   and do not introduce a third representation.
3. **Keep gates synchronized**: diagnostic code changes must simultaneously update `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table**—`build.rs` enforces consistency among all
   three.
4. **Register the stage**: if the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Check reverse dependencies**: new code must not cause L2 to depend on L3 or L4 to depend on
   L1/L2.
6. **Wire up tests**: a new test directory must also have a `mod` declaration in its parent module;
   otherwise, it will not run. The new CI check will catch this.

### Adding a “Layer” (Such as a Future Static Check)

1. Create a module under `sema/layers/` that **does not depend on a higher-numbered layer**.
2. Put its output into `Obligations`, where it is consumed uniformly by the orchestration layer.
3. Update `sema/layers/README.md` so that it **describes the actual layer order** (use line numbers
   from the current tree, but anchor locations by function name or driver point).
4. Register the layer in the `Stage` enum and explicitly declare the modes in which it runs in
   `Program::stages()`.

---

## Where to Find the Maintenance Mechanism

The anti-regression gate (CI checklist, decision procedure, and three prohibitions) has moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md).

This document answers only “where things go”: dependency direction rules, the target directory
structure, feature routing tables, and naming conventions.
