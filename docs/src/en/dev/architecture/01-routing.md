# Feature Routing and Dependency Specifications

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) and
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing tables, and anti-regression mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution stages, and acceptance criteria are covered in
> the main RFC-039 text. Detailed layered design is covered by the other documents in this
> directory.

## Dependency Direction Specifications

The following rules are enforced by CI (see [Anti-Regression Mechanism](#防反弹机制)):

| Rule                                                             | Check Method                                                                                                                                   | Current State                                                                                                                                              |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `include!` is prohibited                                         | `grep -rn 'include!' src/` should return 0                                                                                                     | 0 occurrences (eliminated in 5.1: `checker.rs` was changed to a real `mod`)                                                                                |
| L2 must not `use` L3                                             | Scan `use ...typecheck` in `src/frontend/core/{lexer,parser}/`                                                                                 | 2 occurrences currently exist and must be removed (see Routing Table C)                                                                                    |
| L3/L4 must not `use` L2                                          | Scan `use ...frontend::{lexer,parser,module}` in `sema/`, `proof/`, `middle/`, and `backends/`                                                 | Currently 0; use 0 as the baseline (the AST is a top-level domain and requires no exemptions)                                                              |
| L4 must not reference L1/L2 in reverse                           | Scan `use ...frontend::core::{lexer,parser}` in `src/backends/` and `src/middle/`                                                              | A baseline must first be established                                                                                                                       |
| The opcode vocabulary has a single owner: `middle/bytecode/`     | Scan `src/backends/` to ensure it only imports, rather than defines, opcode constants; scan `sema/` to ensure it contains no opcode references | The current vocabulary is in `backends/common/opcode.rs` (creating a reverse L3→L4 dependency) and will be migrated with the bytecode-domain consolidation |
| Cross-layer `pub(crate)` leaks may only decrease, never increase | Include the count in the baseline file                                                                                                         | Cross-layer leaks still exist; the current count is computed and reported by `scripts/ci/check-boundary.py` (total + per-directory breakdown)              |
| Hard-coded type/predicate names are prohibited                   | Scan for type-name string literals in `src/frontend/core/parser/`                                                                              | `"Terminates"`, etc.                                                                                                                                       |

## Target Directory Structure (After All Implementation Is Complete)

The following is the state **after all implementation phases are complete**, not the target of the
first step. The gaps between the current and target structures are listed in the mapping table
below.

```
src/
├── driver/                        # L1 orchestration: the sole decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustible) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-stage diagnostic aggregation + exit-code decision
│
├── frontend/                      # L2 lexing and parsing
│   ├── lexer/                     #   Declarative: unify number scanning into scan_radix(base),
│   │   ├── mod.rs                 #     sole escape-decoding implementation (merge the 3 copies in literals.rs)
│   │   ├── number.rs              #     replace the 4 line-for-line identical radix scanners
│   │   ├── escape.rs              #     sole escape decoder
│   │   ├── string.rs              #     string / multiline string
│   │   ├── fstring.rs             #     f-string; interpolation now advances to the parser instead of nesting tokenize
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: one precedence table, no bare magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     grammar definitions (added after the grammar-driven implementation)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder node
│   └── module/                    #   Multi-file: discovery / registry / roles
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merge the ~60 lines of near-duplicate code)
│       └── discover.rs            #     discovery strategy that follows use
│
├── ast/                           # Sole AST + sole type carrier (top-level domain, sibling to parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after consolidation; see 03 for how to handle dead variants)
│   ├── stmt.rs                    #   StmtKind (remove signature_params)
│   ├── type_.rs                   #   sole type structure Type + NameKind (26 → consolidated variant set)
│   └── pattern.rs
│
├── sema/                          # L3 upper half: types and static analysis (currently typecheck/)
│   ├── types/                     #   MonoType working representation + solving (see the two roles in 03)
│   │   ├── mod.rs
│   │   ├── repr.rs                #     MonoType definition (working representation)
│   │   ├── universe.rs            #     UniverseLevel (currently inside mono.rs)
│   │   ├── infer.rs               #     inference entry point
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     constant evaluation (including the sole const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     sole ConstExpr; sole source of operators (merge the current 2 sets)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   expression / statement inference
│   │   ├── mod.rs                 #     location of StatementChecker
│   │   ├── call/                  #     split infer_call_expr (see the split axis in 03/04)
│   │   │   ├── mod.rs             #       dispatch by "callee shape"
│   │   │   ├── construct.rs       #       sum-type construction / generic type construction
│   │   │   ├── generic.rs         #       explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (sole three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   module-level checks (after splitting checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     annotation type-name validation + is_predicate_head (now an actual submodule of checker)
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     imports/exports / unused imports
│   │   └── semantic_tokens.rs     #     semantic token collection (now an actual submodule of checker)
│   ├── refine/                    #   refinement types (currently the single checker/refinement.rs file, 1107 lines)
│   │   ├── mod.rs                 #     collect_refined_binding_checks
│   │   ├── binding.rs             #     revalidate_refined
│   │   ├── call_arg.rs            #     check_call_arg_refinements
│   │   ├── return_.rs             #     check_return_refinement
│   │   └── walk.rs                #     refined_walk_* + RefinedWalkCtx
│   └── layers/                    #   cross-cutting analyses
│       ├── README.md              #     must describe the actual layer order, not the intended layer order
│       ├── ownership.rs
│       ├── termination.rs
│       ├── predicate.rs
│       └── equivalence.rs
│
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to the top level; the driver's obligation ledger owns proof_calls, and sema's three layers share the SMT backend, so it is promoted because of cross-layer consumption)
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
│   │   ├── operand.rs             #   Operand (dead Temp/Label/Register variants have been removed)
│   │   ├── instr.rs               #   Instruction (including Phi)
│   │   ├── block.rs               #   BasicBlock
│   │   ├── fn_.rs                 #   FunctionIR / LocalSlot
│   │   └── verify.rs              #   verify_loose / verify_ssa
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
│   │   └── synth.rs               #     the only place permitted to construct ast::Expr
│   ├── passes/                    #   passes on IR
│   │   ├── mono/                  #     monomorphization (remove 415 lines of leftover closure code)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 stack slots (no real register file)
│   └── bytecode/                  #   bytecode domain: opcode vocabulary + container + encoding/decoding
│       ├── opcode.rs              #     sole opcode vocabulary (+ generation-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file makes differential testing easier)
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
│   │   ├── heap.rs                #   L4-internal sharing: heap layout
│   │   └── value.rs               #   L4-internal sharing: runtime value
│   ├── interpreter/               #   interpreter (sole Executor implementation)
│   │   ├── executor.rs
│   │   ├── debug.rs               #   opcode dispatch table
│   │   ├── ffi.rs
│   │   └── ops/                   #   split by instruction family
│   └── runtime/                   #   DAG task scheduler (not a second execution backend)
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 standard library (Rust implementation + .yx mix)
├── util/                          # Cross-layer utilities (diagnostics, span, cache, i18n; retain the existing name—the Go "avoid util" guidance referenced in 08 applies moderately, and renaming causes net churn without domain benefit, so record it as an exception)
├── formatter/                     # Formatting
├── lsp/                           # Language service
├── repl/                          # REPL (if retained, see Section D of 06)
└── package/                       # Package management
```

### Responsibilities and Boundary Invariants for Each Directory

| Directory                                 | Responsibility                                                                           | Invariants                                                                                                                                                          |
| ----------------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Decide which stages to run, how many times to run them, and how failures propagate       | **Must not** be reverse-depended on by L2/L3/L4; the set of stages must be an exhaustible enum                                                                      |
| `frontend/lexer`, `frontend/parser`       | Lexing and parsing; construct `ast/` nodes                                               | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics must all be passed down as data |
| `ast/`                                    | The sole AST and type representation (top-level shared data domain, sibling to `parser`) | **Must not** contain a second definition of the same concept; must not `use` other layers (pure data and pure functions only)                                       |
| `frontend/module/`                        | Multi-file discovery, registry, and roles                                                | Perform discovery and registration only; **do not run analysis stages**. The `driver` schedules analysis                                                            |
| `sema/`                                   | Type checking and static analysis                                                        | **Must not** construct IR; must not be aware of opcodes                                                                                                             |
| `proof/`                                  | Proof result types and SMT backends                                                      | The Solver trait is the sole backend abstraction point; **must not** use `.expect()` panics                                                                         |
| `middle/ir/`                              | IR definitions and invariants                                                            | The IR must satisfy SSA discipline; a green `verify_ssa` is a prerequisite for merging                                                                              |
| `middle/lower/`                           | AST → IR                                                                                 | The **only** place permitted to construct `ast::Expr` is `synth.rs`                                                                                                 |
| `middle/codegen/`                         | IR → bytecode lowering                                                                   | Depend only on IR, not `sema`; do not define the opcode vocabulary                                                                                                  |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + encoding/decoding                 | The **opcode vocabulary has a single owner**; `backends/` consumes but does not define it; `sema` is not aware of opcodes                                           |
| `backends/`                               | Execution                                                                                | **Must not** reference L1/L2 in reverse; must not be aware of the AST                                                                                               |
| `middle/passes/mono/`                     | Passes on IR                                                                             | Must not reference `frontend`                                                                                                                                       |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tool layers (outside the L1-L4 model)                                         | Consume only the `pub` APIs of L1-L4; must not touch `pub(crate)` (the leak count also constrains these layers)                                                     |

### Current → Target Mapping

| Current                                                                                                                                                                                 | Target                                                                                                                                                                                                                                                                                                             | RFC     |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- |
| `frontend/pipeline.rs` (740 lines, hand-written 5-stage if chain)                                                                                                                       | `driver/` + `Program::SingleFile`                                                                                                                                                                                                                                                                                  | 02      |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entry points, 60% branching)                                                                                                           | `frontend/module/` retains only discovery/registration; compilation logic moves to `driver`                                                                                                                                                                                                                        | 02      |
| `frontend/compiler.rs` (267 lines)                                                                                                                                                      | Slimmed down to a thin wrapper around `driver`                                                                                                                                                                                                                                                                     | 02      |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 copies of the radix scanner + 3 copies of escape decoding)                                                                             | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                                                                                                                                                                                                                 | 05      |
| `frontend/core/parser/pratt/` (hand-written match dispatch, 2 sets of BP ladders)                                                                                                       | `frontend/parser/grammar/` + a single precedence table                                                                                                                                                                                                                                                             | 05      |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                                                                              | Split into the top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                                                                                                                                                                                                                  | 03 / D1 |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8-10 responsibilities)                                                                                                   | Split `frontend/parser/` + delete 120 lines of dead legacy syntax                                                                                                                                                                                                                                                  | 05      |
| `typecheck/checker.rs` (4167 lines + 3 actual submodules: `checker/annotations.rs` at 470 lines, `checker/refinement.rs` at 1107 lines, and `checker/semantic_tokens.rs` at 1553 lines) | `sema/check/` + `sema/refine/`; change `include!` to a real `mod` (completed in 5.1), move the refinement block into a submodule and consolidate its entry point (completed in 5.2: the entry point now takes `&self` and returns diagnostics), and move the annotation family into a submodule (completed in 5.3) | 02 / 05 |
| `typecheck/checker/semantic_tokens.rs` (1553 lines, already an actual submodule of `checker`)                                                                                           | `sema/check/semantic_tokens.rs`                                                                                                                                                                                                                                                                                    | 02      |
| `typecheck/inference/expressions.rs` (5525 lines)                                                                                                                                       | `sema/infer/{call/,expr,stmt}.rs`                                                                                                                                                                                                                                                                                  | 05 / 04 |
| `typecheck/inference/statements.rs` (2887 lines)                                                                                                                                        | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                                                                                                                                                                                                                         | 05      |
| `typecheck/types/mono.rs` (1077 lines, including a synonym table)                                                                                                                       | `sema/types/{repr,universe}.rs`                                                                                                                                                                                                                                                                                    | 03      |
| `typecheck/types/const_data.rs` (second operator enum)                                                                                                                                  | Unify with the operators of `ast::Type`                                                                                                                                                                                                                                                                            | 03      |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                                                                                            | Delete                                                                                                                                                                                                                                                                                                             | 06      |
| `typecheck/layers/dispatch.rs` (zero calls across the entire chain)                                                                                                                     | Delete                                                                                                                                                                                                                                                                                                             | 06      |
| `typecheck/layers/`                                                                                                                                                                     | `sema/layers/` + change `README.md` to describe the actual layer order                                                                                                                                                                                                                                             | 06      |
| `typecheck/proof/`                                                                                                                                                                      | Promote to the top-level `proof/`                                                                                                                                                                                                                                                                                  | 02      |
| `middle/core/ir_gen.rs` (8448 lines, no tests)                                                                                                                                          | `middle/lower/` (14 modules)                                                                                                                                                                                                                                                                                       | 04      |
| `middle/core/ir.rs` (905 lines)                                                                                                                                                         | `middle/ir/` + `verify.rs`                                                                                                                                                                                                                                                                                         | 04      |
| `middle/core/bytecode.rs` (2422 lines, 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization)                                                               | Consolidate into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules)                                                                                                                                                                                           | 06      |
| `backends/common/opcode.rs` (83-constant vocabulary)                                                                                                                                    | `middle/bytecode/opcode.rs` (eliminate the reverse L3→L4 dependency)                                                                                                                                                                                                                                               | 06      |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                                                                               | Delete `416-830`; retain 340 lines                                                                                                                                                                                                                                                                                 | 06      |
| `std/fs.rs`, `std/net.rs`, etc. (native API bindings)                                                                                                                                   | Remain in place                                                                                                                                                                                                                                                                                                    | —       |
| `repl/` (1152 lines, no tests, no input echo)                                                                                                                                           | Retain or delete; see Section D of 06                                                                                                                                                                                                                                                                              | 06      |

**Renaming directories is not the goal.** Changes fall into two categories:

- **Internal structures that must change:** the internal structures of the three files
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (4167 lines + actual submodules of
  470 / 1107 / 1553 lines), and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: renaming.** Rename `typecheck/` → `sema/` and `middle/core/` →
  `middle/ir/`; split `parser/ast.rs` into the top-level `ast/` (moving the `Type` definition into
  `type_.rs`); and move the opcode vocabulary into `middle/bytecode/`, completing all of this
  together with P5/P6 (Decision D1). **No phased rollout and no “optional” treatment.** The reason
  is that a directory name is the first signal of its responsibility: `middle/core/` currently
  contains `ir.rs`, `ir_gen.rs`, and `bytecode.rs`—three separate responsibilities—so the name is no
  longer valid. The AST is consumed by every layer, and placing it under `frontend/` would mislead
  downstream paths. CI checks enforce the boundaries, but **the checks reference the directory
  responsibility table, and the responsibility table must match the actual directory names**;
  otherwise, `check-boundary.py` cannot determine whether code is in the correct location.

### Naming Conventions

| Rule                                                                          | Rationale                                                                                                                                                                   |
| ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A concept has exactly one owning module                                       | Prevents parallel definitions such as “3 type representations”                                                                                                              |
| The submodule directory for `foo.rs` is named `foo/`                          | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                         |
| Cross-layer references must use `pub` APIs, not leaked `pub(crate)` items     | The **count** of `pub(crate)` leaks serves as a boundary-erosion metric and may only decrease, never increase (this is a semantic boundary metric, not a line-count metric) |
| Internal implementations should not be `pub` unless they are cross-layer APIs | Reduces the visibility surface                                                                                                                                              |
| Type file names use `type_.rs` and `fn_.rs` to avoid keywords                 | `type` / `fn` have been reserved words since Rust 2024                                                                                                                      |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                      | See Section D of 06: parallel copies used to express a “new version” are a common source of dead code                                                                       |

## Feature Routing Tables

These tables answer **“I want to add X; where should I change it?”** This is the most long-term
valuable part of this document set—it does not become obsolete after any single refactoring.

### Routing Table A: Language Features

| What to Add                         | L2 Lexing                                                         | L2 Parsing                                                                                                                                                         | L3 Semantics                                                                                                                                                                  | L3 IR                                           | L4 Execution                                        | Gates/Documentation                                                                                            |
| ----------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **Binary operator**                 | Keyword table in `lexer/state.rs`; add `TokenKind` to `tokens.rs` | Add a BP constant to `pratt/precedence.rs`; add an arm to `infix_info` in `led.rs`; add a mapping to `parse_binary` in `led.rs`; add a `BinOp` variant to `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                                                  | Corresponding lowering in `lower/`              | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                                  |
| **Unary operator**                  | Same as above                                                     | Add an arm to `prefix_info` in `nud.rs` + a new parse function + a `UnOp` variant to `ast.rs`                                                                      | `infer_unary`                                                                                                                                                                 | Same as above                                   | Same as above                                       | RFC-010 table                                                                                                  |
| **Constant-evaluation operator** ⚠️ | —                                                                 | —                                                                                                                                                                  | **`BinOp` at `types/const_data.rs:234` / `UnOp` at `:321` form a second set of enums parallel to `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` conversion between them** | `ir_gen.rs:2317` requires `use ast::BinOp as B` | —                                                   | **No gate**—see Routing Table C                                                                                |
| **Expression form**                 | —                                                                 | Add an `Expr` variant to `ast.rs` + update `nud.rs`/`led.rs`                                                                                                       | Add an inference method to `inference/expressions.rs`                                                                                                                         | Add a module under `lower/exprs/`               | `opcode.rs` or reuse                                | —                                                                                                              |
| **Statement form**                  | —                                                                 | Add a `StmtKind` variant to `ast.rs` + update `statements/`                                                                                                        | `inference/statements.rs`                                                                                                                                                     | `lower/`                                        | Same as above                                       | —                                                                                                              |
| **Type construction**               | —                                                                 | `Type` in `ast.rs` (see 03)                                                                                                                                        | `types/mono.rs` + `solver.rs`                                                                                                                                                 | `FunctionCode.params` type                      | Serialization                                       | RFC-011                                                                                                        |
| **Pattern form**                    | —                                                                 | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                                    | `inference/patterns.rs`                                                                                                                                                       | `compile_pattern` in `lower/`                   | `opcode.rs`                                         | RFC-010b                                                                                                       |
| **Diagnostic code**                 | —                                                                 | —                                                                                                                                                                  | —                                                                                                                                                                             | —                                               | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (automatically validated by the `build.rs` gate) |
| **Warning (W prefix)**              | —                                                                 | —                                                                                                                                                                  | —                                                                                                                                                                             | —                                               | —                                                   | Same as above; note that warnings do not count toward the error total                                          |

> **Current pain point**: adding one binary operator requires changes in **6-7 places** (within
> parser/lexer) + changes to `BinOp::` in **17 downstream production files**. Whether omissions are
> detected depends on three layers:
>
> - **Caught by the compiler**: places that perform an **exhaustive `match` over `ast::BinOp`** fail
>   to compile if an arm is missing. These constitute the majority.
> - **Not caught by the compiler (the true blind spots)**: the `matches!` macro, `_` fallback arms,
>   and **cross-enum** handling form three categories. An omission in the first two causes the “new
>   operator” to be treated as a different operator. An omission in the third category (`ast::BinOp`
>   ↔ `const_data::BinOp`) is completely silent—the two types compile independently.
> - **L4 is not involved**: `backends/interpreter/` has **zero references** to `BinOp` or
>   `ast::`—the interpreter never sees the AST and recognizes only opcodes. Downstream pressure is
>   therefore concentrated in L3, not L4.
>
> Goal: after the grammar-driven implementation, reduce this to 1-2 locations + 1 autogenerated
> location (see 05); consolidate the cross-enum portion through same-source generation (see 05); and
> consolidate the bytecode layer's third operator enum (`BinaryOp`/`UnaryOp`/`CompareOp`) through
> the opcode generation-time gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| What to Add                                                      | Landing Point                                                                                                                                                                                | Places to Update in Sync                                                                                                                                                                                                                                                                                                                 |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation stage**                                            | Add a `Stage` variant to `driver/stage.rs`                                                                                                                                                   | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, `dispatch` in `driver/mod.rs` (the exhaustive match forces completion), and CI stage-coverage tests                                                                                                                                                            |
| **Cross-stage checking layer** (such as ownership / termination) | Add a module under `sema/layers/`                                                                                                                                                            | Register it in the stage table + update `layers/README.md` (**the call-order table must correspond one-to-one with the call sites in `checker.rs`**) + merge its output into `Obligations`                                                                                                                                               |
| **Opcode**                                                       | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`; migrated with the bytecode-domain consolidation, with all touchpoints consolidated under the single `middle/` directory) | **5 locations**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the decoding `match` in `From<BytecodeFile>`, and `translate_*` in `translator.rs`. Only 2 locations are enforced by compiler exhaustive matches; an omission in the other 3 fails only when running `.42` artifacts. A generation-time gate is required (see 06) |
| **CLI subcommand**                                               | `Commands` enum in `main.rs`                                                                                                                                                                 | The `pub` entry point in `lib.rs` (currently 7) and `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                                   |
| **LSP capability**                                               | Add a handler under `lsp/handlers/`                                                                                                                                                          | `handle_request` in `lsp/server.rs` (currently 274 lines, with 13 instances of duplicated boilerplate) and the state in `lsp/world.rs`                                                                                                                                                                                                   |
| **Standard-library API**                                         | `src/std/*.rs`                                                                                                                                                                               | **An automated gate already exists**: byte-for-byte comparison by `gen_interfaces.rs` + marked-region drift detection by `gen_docs.rs`. This is the most mature pattern in the entire project and should be extended to the opcode and type tables                                                                                       |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions That Must Be Eliminated

| Reverse Dependency                                                | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Elimination Method                                                                                                                                             | Owning Document   |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| parser → typecheck (**only 2 occurrences, in the same function**) | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both of which call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Pass operator-interface specifications to the parser as data; the parser no longer `use`s typecheck                                                            | 03                |
| Hard-coded predicate names in the parser                          | `declarations.rs:499` `let is_predicate_app =                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | n: &str                                                                                                                                                        | n == "Terminates" |         | state.is_predicate_name(n);`. The comment at `:494-497` describes this as an unresolved open-set problem: “The future open-set solution: pass predicate names down to the parser, or change them into a form decidable by the syntax layer” | As above; pass the predicate set down as data        | 03  |
| **Duplication created to avoid a cross-module dependency**        | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` are semantically equivalent but independently implemented; the comments at `ast.rs:845-846` acknowledge: “This is independently implemented here to avoid an internal cross-module dependency within the parser”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Merge them into a single implementation together with the probe data flow from 03                                                                              | 03                |
| checker → internal layer functions                                | The traversal family in `checker/refinement.rs` and `revalidate_refined` / `check_return_refinement` call `layers::termination::{negate_guard, substitute_const_expr}` (originally `checker.rs:4857/4986/5246/5384`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Move them to the public API of `sema/refine/` or `sema/layers/termination`                                                                                     | 02                |
| 3 parallel type representations                                   | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (re-exported from `ir.rs:3` via `pub use`) + serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Consolidate into a single representation                                                                                                                       | 03                |
| Hand-written synonym table                                        | `from_builtin_name` in `types/mono.rs:618-643` maintains branches such as `"Int"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | "int"                                                                                                                                                          | "Int64"           | "int64" | "i64" => Some(MonoType::Int(64))`                                                                                                                                                                                                           | Delete it as the type representation is consolidated | 03  |
| **3 parallel operator enums**                                     | `pub enum BinOp` (`ast.rs:191`, 20 variants) / `pub enum UnOp` (`ast.rs:217`, 4 variants, uniquely including `Deref`); `pub enum BinOp` (`types/const_data.rs:234`, 18 variants, using `Ne` rather than `Neq`) / `pub enum UnOp` (`:321`, 4 variants, uniquely including `BitNot`); `BinaryOp` (`middle/core/bytecode.rs:70`, 11 variants, with yet another naming scheme for `Rem`/`Xor`/`Sar`) + `UnaryOp` (`:97`) + `CompareOp` (`:106`). **There is no `From` / `TryFrom` between ast ↔ const_data**; the only bridge is the hand-written lossy free function in `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot`, etc. return `None` and are silently discarded), forcing 5+ files to assign aliases at import sites (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`), as seen in `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, and `middle/core/ir_gen.rs:2317` | Consolidate the two sets in L2 through same-source generation (see 05); reconcile the bytecode layer's set through the opcode generation-time gate (see 06 F8) | 03 / 05           |
| `include!`                                                        | Already eliminated (5.1: `checker.rs` was changed to a real `mod semantic_tokens`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Complete                                                                                                                                                       | —                 |

## Guidance for Future Extensions

### Checklist for Adding a Language Feature

1. **Consult Routing Table A first** and determine the landing point. Do not choose a file based on
   intuition.
2. **Types first**: if the feature involves types, follow the single representation from 03; do not
   add a third representation.
3. **Synchronize the gates**: diagnostic-code changes must simultaneously update `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table**—`build.rs` enforces consistency among all
   three.
4. **Register the stage**: if the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Check reverse dependencies**: new code must not cause L2 to depend on L3, or L4 to depend on
   L1/L2.
6. **Wire up tests**: a new test directory must also have a `mod` declaration in its parent module;
   otherwise, it will not run. A new CI check will catch this.

### Adding a “Layer” (Such as a Future Static Check)

1. Create a module under `sema/layers/`; it **must not depend on a layer with a higher number**.
2. Put its output into `Obligations`, where the orchestration layer consumes it uniformly.
3. Update `sema/layers/README.md`—make it describe the **actual layer order** (line numbers are
   based on the current tree, with function names/driver points used as location anchors).
4. Register it in the `Stage` enum and explicitly declare in `Program::stages()` which modes run it.

---

## Where to Find the Maintenance Mechanism

The anti-regression gates (CI checklist, decision procedure, and three prohibitions) have moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md).

This document only answers **“Where does something go?”**: dependency direction specifications, the
target directory structure, feature routing tables, and naming conventions.
