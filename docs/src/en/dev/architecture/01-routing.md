# Functional Routing and Dependency Specifications

> **Supplementary Design Document**. This document is a supplement to
> [RFC-039: Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md); it
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing guide, and anti-regression mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution stages, and acceptance criteria are described in
> the body of RFC-039. Layered detailed design is in the rest of the documents in this directory.

## Dependency Direction Specifications

The following rules are enforced by CI checks (see
[Anti-regression Mechanism](#anti-regression-mechanism)):

| Rule                                                             | Check Method                                                                                       | Current Status                                                                                                                                 |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `include!` forbidden                                             | `grep -rn 'include!' src/` should be 0                                                             | 1 occurrence (`checker.rs:5618`)                                                                                                               |
| L2 must not `use` L3                                             | Scan `use ...typecheck` in `src/frontend/core/{lexer,parser}/`                                     | 2 occurrences exist; must be cleared (see Routing Guide C)                                                                                     |
| L3/L4 must not `use` L2                                          | Scan `use ...frontend::{lexer,parser,module}` in `sema/`, `proof/`, `middle/`, `backends/`         | Currently 0; baseline set to 0 (AST is a top-level domain; no exemption clause needed)                                                         |
| L4 must not reverse-reference L1/L2                              | Scan `use ...frontend::core::{lexer,parser}` in `src/backends/`, `src/middle/`                     | Baseline must first be established                                                                                                             |
| Opcode vocabulary belongs solely to `middle/bytecode/`           | `src/backends/` must not define opcode constants (only import); `sema/` must not reference opcodes | Currently the vocabulary lives in `backends/common/opcode.rs` (forming L3→L4 reverse direction), to be migrated with the bytecode domain merge |
| `pub(crate)` cross-layer leakage may only decrease, not increase | Count and roll into baseline file                                                                  | At least 1 occurrence (`collect_used_in_type` used by `inference/statements.rs:16`)                                                            |
| Hardcoded type/predicate names forbidden                         | Scan type-name string literals in `src/frontend/core/parser/`                                      | e.g. `"Terminates"`                                                                                                                            |

## Target Directory Structure (After All Construction Is Complete)

What follows is the form **after all construction phases are complete**, not the target of the first
step. The gap between current form and target form is in the mapping table below.

```
src/
├── driver/                        # L1 orchestration: the only decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustively enumerable) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-stage diagnostic aggregation + exit-code determination
│
├── frontend/                      # L2 lexical and syntax
│   ├── lexer/                     #   Declarative: numbers unified as scan_radix(base),
│   │   ├── mod.rs                 #     single escape-decoding implementation (the 3 copies in literals.rs merged into one)
│   │   ├── number.rs              #     replaces the 4 line-by-line identical radix scanners
│   │   ├── escape.rs              #     the sole escape decoder
│   │   ├── string.rs              #     strings / multi-line strings
│   │   ├── fstring.rs             #     f-string; interpolation advances toward parser rather than nesting tokenize
│   │   └── token.rs               #     TokenKind (single source of truth)
│   ├── parser/                    #   Grammar-driven: a single precedence table, no bare magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     grammar definition (added after grammar-driven refactor)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder nodes
│   └── module/                    #   Multi-file: discovery / registry / roles
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (the current 60 lines of near-duplication merged)
│       └── discover.rs            #     discovery strategy that follows `use` traces
│
├── ast/                           # Sole AST + sole type carrier (top-level domain, peer to parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after consolidation; see 03 for handling of dead variants)
│   ├── stmt.rs                    #   StmtKind (remove signature_params)
│   ├── type_.rs                   #   Sole type structure Type + NameKind (26 → consolidated variant set)
│   └── pattern.rs
│
├── sema/                          # L3 upper half: typing and static analysis (current typecheck/)
│   ├── types/                     #   MonoType working representation + solving (see 03 for two roles)
│   │   ├── mod.rs
│   │   ├── repr.rs                #     MonoType definition (working representation)
│   │   ├── universe.rs            #     UniverseLevel (currently inside mono.rs)
│   │   ├── infer.rs               #     inference entry
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     constant evaluation (includes the sole const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     sole ConstExpr; sole source of operators (the current 2 sets merged)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   expression / statement inference
│   │   ├── mod.rs                 #     StatementChecker lives here
│   │   ├── call/                  #     split-out infer_call_expr (see 03/04 for the split axis)
│   │   │   ├── mod.rs             #       dispatch by callee shape
│   │   │   ├── construct.rs       #       sum-type construction / generic-type construction
│   │   │   ├── generic.rs         #       explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (the sole three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level checks (after splitting current checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     annotation type-name validation + is_predicate_head
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     import/export / unused imports
│   │   └── semantic_tokens.rs     #     semantic-token collection (currently stitched in via include!)
│   ├── refine/                    #   refinement types (current checker.rs 3862-5616)
│   │   ├── mod.rs                 #     collect_refined_binding_checks
│   │   ├── binding.rs             #     revalidate_refined
│   │   ├── call_arg.rs            #     check_call_arg_refinements
│   │   ├── return_.rs             #     check_return_refinement
│   │   └── walk.rs                #     refined_walk_* + RefinedWalkCtx
│   └── layers/                    #   cross-cutting analysis
│       ├── README.md              #     must be the actual layer order, not the intended one
│       ├── ownership.rs
│       ├── termination.rs
│       ├── predicate.rs
│       └── equivalence.rs
│
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to top level; the driver obligations ledger holds proof_calls, the three sema layers share the SMT backend, and since it is consumed across layers it is promoted to top level)
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
│   │   ├── operand.rs             #   Operand (Temp/Label/Register dead variants removed)
│   │   ├── instr.rs               #   Instruction (including Phi)
│   │   ├── block.rs               #   BasicBlock
│   │   ├── fn_.rs                 #   FunctionIR / LocalSlot
│   │   └── verify.rs              #   verify_loose / verify_ssa
│   ├── lower/                     #   AST → IR (after splitting current ir_gen.rs)
│   │   ├── mod.rs                 #     AstToIrGenerator
│   │   ├── scope.rs               #     scope and register frame (RAII)
│   │   ├── module_ir.rs
│   │   ├── const_eval.rs
│   │   ├── globals.rs
│   │   ├── stmt.rs
│   │   ├── curry.rs
│   │   ├── loops.rs
│   │   ├── query.rs               #     pure query utilities
│   │   ├── expr/                  #     expression lowering
│   │   └── synth.rs               #     the only place allowed to construct ast::Expr
│   ├── passes/                    #   passes over IR
│   │   ├── mono/                  #     monomorphization (delete 415 lines of closure remnants)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 stack slot (no real register file)
│   └── bytecode/                  #   bytecode domain: opcode vocabulary + container + codec
│       ├── opcode.rs              #     sole opcode vocabulary (+ build-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file for easy cross-check)
│       ├── decode/                #     decode (split-out from the 1400-line From)
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
│   │   ├── heap.rs                #   L4 internal sharing: heap layout
│   │   └── value.rs               #   L4 internal sharing: runtime value
│   ├── interpreter/               #   interpreter (the sole Executor implementation)
│   │   ├── executor.rs
│   │   ├── debug.rs               #   opcode dispatch table
│   │   ├── ffi.rs
│   │   └── ops/                   #   split by instruction family
│   └── runtime/                   #   DAG task scheduler (not a second execution backend)
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 standard library (Rust implementation + .yx mixed)
├── util/                          # cross-layer utilities (diagnostics, span, cache, i18n; keep the existing name — the Go "avoid util" cited in 08 is moderately applicable, renaming would be pure churn with no domain gain, logged as an exception)
├── formatter/                     # formatting
├── lsp/                           # language service
├── repl/                          # REPL (if retained, see 06 section D)
└── package/                       # package management
```

### Per-Directory Responsibilities and Boundary Invariants

| Directory                                 | Responsibility                                                                  | Invariants                                                                                                                                                      |
| ----------------------------------------- | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Decides which stages to run, how many times, how failure propagates             | **Must not** be reverse-depended on by L2/L3/L4; the set of stages must be an exhaustively-enumerable enum                                                      |
| `frontend/lexer`, `frontend/parser`       | Lexical and syntactic; constructs `ast/` nodes                                  | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics are all passed down as data |
| `ast/`                                    | Sole AST and type representation (top-level shared data domain, peer to parser) | **Must not** contain a second definition of the same concept; does not `use` other layers (pure data and pure functions)                                        |
| `frontend/module/`                        | Multi-file discovery, registry, roles                                           | Only does discovery and registration; **does not run analysis stages**; analysis is scheduled by `driver`                                                       |
| `sema/`                                   | Type checking and static analysis                                               | **Must not** construct IR; not aware of opcodes                                                                                                                 |
| `proof/`                                  | Proof result types, SMT backend                                                 | Solver trait is the sole backend abstraction point; **must not** `.expect()` panic                                                                              |
| `middle/ir/`                              | IR definition and invariants                                                    | IR satisfies SSA discipline; `verify_ssa` green is the prerequisite for merge                                                                                   |
| `middle/lower/`                           | AST → IR                                                                        | **The only** place allowed to construct `ast::Expr` is `synth.rs`                                                                                               |
| `middle/codegen/`                         | IR → bytecode lowering                                                          | Depends only on IR, not on `sema`; does not define the opcode vocabulary                                                                                        |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + codec                    | **Sole home of the opcode vocabulary**; `backends/` only consumes, never defines; `sema` is unaware of opcodes                                                  |
| `backends/`                               | Execution                                                                       | **Must not** reverse-reference L1/L2; not aware of AST                                                                                                          |
| `middle/passes/mono/`                     | Passes over IR                                                                  | Must not reference `frontend`                                                                                                                                   |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tool layers (outside the L1–L4 model)                                | Only consume the `pub` API of L1–L4; must not touch `pub(crate)` (the leakage count constrains them as well)                                                    |

### Current → Target Mapping

| Current                                                                                                                   | Target                                                                                                              | Belongs To |
| ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------- |
| `frontend/pipeline.rs` (740 lines, 5-stage hand-written if-chain)                                                         | `driver/` + `Program::SingleFile`                                                                                   | 02         |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entries, 60% branching)                                                  | `frontend/module/` keeps only discovery/registration; compilation logic goes to `driver`                            | 02         |
| `frontend/compiler.rs` (267 lines)                                                                                        | Reduced to a thin wrapper around `driver`                                                                           | 02         |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                      | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                  | 05         |
| `frontend/core/parser/pratt/` (hand-written match dispatch, two BP ladders)                                               | `frontend/parser/grammar/` + a single precedence table                                                              | 05         |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                | Demolished to top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                    | 03 / D1    |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8–10 sections of responsibility)                           | split `frontend/parser/` + remove 120 lines of dead old syntax                                                      | 05         |
| `typecheck/checker.rs` (5618 lines + `include!` 1547 lines)                                                               | `sema/check/` + `sema/refine/`; `include!` replaced by real `mod`                                                   | 02 / 05    |
| `typecheck/checker/semantic_tokens.rs` (1547 lines, no module identity)                                                   | `sema/check/semantic_tokens.rs` (a real module)                                                                     | 02         |
| `typecheck/inference/expressions.rs` (5525 lines)                                                                         | `sema/infer/{call/,expr,stmt}.rs`                                                                                   | 05 / 04    |
| `typecheck/inference/statements.rs` (2887 lines)                                                                          | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                          | 05         |
| `typecheck/types/mono.rs` (1077 lines, includes a synonym table)                                                          | `sema/types/{repr,universe}.rs`                                                                                     | 03         |
| `typecheck/types/const_data.rs` (a second operator enum)                                                                  | Merged with the operator enum on `ast::Type`                                                                        | 03         |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                              | Delete                                                                                                              | 06         |
| `typecheck/layers/dispatch.rs` (entire chain has zero calls)                                                              | Delete                                                                                                              | 06         |
| `typecheck/layers/`                                                                                                       | `sema/layers/` + `README.md` rewritten to reflect the actual layer order                                            | 06         |
| `typecheck/proof/`                                                                                                        | Promoted to top-level `proof/`                                                                                      | 02         |
| `middle/core/ir_gen.rs` (8448 lines, zero tests)                                                                          | `middle/lower/` (14 modules)                                                                                        | 04         |
| `middle/core/ir.rs` (905 lines)                                                                                           | `middle/ir/` + `verify.rs`                                                                                          | 04         |
| `middle/core/bytecode.rs` (2422 lines, 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization) | Merged into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules) | 06         |
| `backends/common/opcode.rs` (83 constants)                                                                                | `middle/bytecode/opcode.rs` (eliminates L3→L4 reverse direction)                                                    | 06         |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                 | delete `416-830`, keep 340 lines                                                                                    | 06         |
| `std/fs.rs`, `std/net.rs`, etc. (native API bindings)                                                                     | location unchanged                                                                                                  | —          |
| `repl/` (1152 lines, zero tests, no echo)                                                                                 | Retain or delete, see 06 section D                                                                                  | 06         |

**Renaming directories is not the goal.** The changes fall into two categories:

- **Internal structures that must be changed**: the internal structure of the three files
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (7165 lines including `include!`),
  and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: rename.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`,
  `parser/ast.rs` demolished to top-level `ast/` (the `Type` definition into `type_.rs`), opcode
  vocabulary migrated to `middle/bytecode/`, completed together with P5/P6 (Decision D1). **Not
  split into phases, no "optional"**. The reason: the directory name is the first-glance signal of
  responsibility — `middle/core/` currently holds `ir.rs`, `ir_gen.rs`, and `bytecode.rs` for three
  different things, so the name has already failed; AST is consumed by all layers, placing it under
  `frontend/` would mislead downstream paths. Boundaries are guaranteed by CI checks, but **the
  check rules reference the directory-responsibility table, and that table must be consistent with
  the actual directory names**, otherwise `check-boundary.py` has no way to judge whether code is in
  the right place.

### Naming Conventions

| Rule                                                                               | Reason                                                                                                                                                                             |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A concept has exactly one home module                                              | Prevents parallel definitions like "3 sets of type representations"                                                                                                                |
| The submodule directory of `foo.rs` is named `foo/`                                | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                                |
| Cross-layer references must go through the `pub` API, not via `pub(crate)` leakage | The `pub(crate)` leakage **count** is used as a boundary-erosion indicator and may only decrease, not increase (this is a semantic-boundary indicator, not a line-count indicator) |
| Internal implementations should not have `pub` unless they are cross-layer API     | Reduce the visibility surface                                                                                                                                                      |
| Type file names like `type_.rs`, `fn_.rs` avoid keywords                           | `type` / `fn` are reserved words from Rust 2024 onward                                                                                                                             |
| Do not add new `xxx_core` / `xxx_new` / `xxx_v2` directories                       | See 06 section D: parallel copies as a way to express "new version" is a common source of dead code                                                                                |

## Feature Routing Guide

**Answering "I want to add X, where do I change?"** This is the part of this group of documents that
has the most long-term value — it will not be invalidated by any single refactor.

### Routing Guide A: Language Features

| What to Add                         | L2 Lexical                                                        | L2 Parser                                                                                                                                                 | L3 Semantic                                                                                                                                                 | L3 IR                                           | L4 Execution                                        | Gate/Docs                                                                                      |
| ----------------------------------- | ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| **Binary operator**                 | keyword table in `lexer/state.rs`, add `TokenKind` in `tokens.rs` | add BP constant in `pratt/precedence.rs`, add arm in `infix_info` in `led.rs`, add mapping in `parse_binary` in `led.rs`, add `BinOp` variant in `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                                | corresponding lowering in `lower/`              | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                  |
| **Unary operator**                  | same as above                                                     | add arm in `prefix_info` in `nud.rs` + new parse function + add `UnOp` variant in `ast.rs`                                                                | `infer_unary`                                                                                                                                               | same as above                                   | same as above                                       | RFC-010 table                                                                                  |
| **Constant-evaluation operator** ⚠️ | —                                                                 | —                                                                                                                                                         | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` are a second set of enums parallel to `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` between them** | `ir_gen.rs:2317` requires `use ast::BinOp as B` | —                                                   | **No gate** — see Routing Guide C                                                              |
| **Expression form**                 | —                                                                 | add `Expr` variant in `ast.rs` + `nud.rs`/`led.rs`                                                                                                        | add infer method in `inference/expressions.rs`                                                                                                              | add module in `lower/exprs/`                    | `opcode.rs` or reuse                                | —                                                                                              |
| **Statement form**                  | —                                                                 | add `StmtKind` variant in `ast.rs` + `statements/`                                                                                                        | `inference/statements.rs`                                                                                                                                   | `lower/`                                        | same as above                                       | —                                                                                              |
| **Type construction**               | —                                                                 | `Type` in `ast.rs` (see 03)                                                                                                                               | `types/mono.rs` + `solver.rs`                                                                                                                               | types of `FunctionCode.params`                  | serialization                                       | RFC-011                                                                                        |
| **Pattern form**                    | —                                                                 | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                           | `inference/patterns.rs`                                                                                                                                     | `compile_pattern` in `lower/`                   | `opcode.rs`                                         | RFC-010b                                                                                       |
| **Diagnostic code**                 | —                                                                 | —                                                                                                                                                         | —                                                                                                                                                           | —                                               | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (`build.rs` gate auto-validates) |
| **Warning (W prefix)**              | —                                                                 | —                                                                                                                                                         | —                                                                                                                                                           | —                                               | —                                                   | same as above; note that warnings are not counted in the error count                           |

> **Current pain point**: a binary operator requires changes in **6–7 places** (inside parser/lexer)
> plus the **17 production files** downstream of `BinOp::`. Whether a missed change is detected
> falls into three tiers:
>
> - **What the compiler catches**: anywhere there is an **exhaustive `match`** over `ast::BinOp`,
>   missing a change directly causes a compile error. This is the majority of cases.
> - **What the compiler cannot catch (the true blind spot)**: the `matches!` macro, `_` fallback
>   arms, and **cross-enum** cases. The first two missed changes manifest as "the new operator is
>   treated as some other operator"; the third (`ast::BinOp` ↔ `const_data::BinOp`) missed change is
>   completely silent — the two types each compile independently.
> - **L4 is not involved**: `backends/interpreter/` has **zero references** to `BinOp` and `ast::` —
>   the interpreter never sees AST, it only knows opcodes. So downstream pressure concentrates in
>   L3, not L4.
>
> Goal: after the grammar-driven refactor, drop to 1–2 places + 1 generated place (see 05); the
> cross-enum portion is solved by single-source generation (see 05); the third operator enum at the
> bytecode layer (`BinaryOp`/`UnaryOp`/`CompareOp`) is closed off by the opcode build-time gate (see
> 06 F8).

### Routing Guide B: Compilation Mechanisms

| What to Add                                                | Landing Point                                                                                                                                                                | Places That Must Be Changed in Sync                                                                                                                                                                                                                                                                                                |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation stage**                                      | add `Stage` variant in `driver/stage.rs`                                                                                                                                     | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, the `dispatch` (exhaustive match) in `driver/mod.rs`, the CI stage coverage test                                                                                                                                                                         |
| **Cross-stage check layer** (e.g. ownership / termination) | add module in `sema/layers/`                                                                                                                                                 | stage-table registration + `layers/README.md` update (**currently the layer order declared in that README is the reverse of the actual execution order**) + output merged into `Obligations`                                                                                                                                       |
| **opcode**                                                 | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`, to be migrated with the bytecode-domain merge; all touch points concentrated in one `middle/` directory) | **5 places**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the decode `match` in `From<BytecodeFile>`, and `translate_*` in `translator.rs`. Only 2 of these are forced by exhaustive match in the compiler; the other 3 missed changes only explode when running a `.42` product. A build-time gate is required (see 06) |
| **CLI subcommand**                                         | `Commands` enum in `main.rs`                                                                                                                                                 | the `pub` entry in `lib.rs` (currently 7), the `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                                  |
| **LSP capability**                                         | add handler in `lsp/handlers/`                                                                                                                                               | `handle_request` in `lsp/server.rs` (currently 274 lines, 13 repeated boilerplate), state in `lsp/world.rs`                                                                                                                                                                                                                        |
| **Standard library API**                                   | `src/std/*.rs`                                                                                                                                                               | **Automatic gate already in place**: `gen_interfaces.rs` byte-by-byte diff + `gen_docs.rs` interval-drift detection. This is the most mature pattern in the project and should be extended to the opcode table and type table                                                                                                      |

### Routing Guide C: Known Reverse Dependencies and Parallel Definitions to Eliminate

| Reverse Dependency                                         | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Elimination Method                                                                                                                                   | Belongs To |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| parser → typecheck (**only 2 places, same function**)      | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Pass the operator-interface spec down to parser as data; parser no longer `use`s typecheck                                                           | 03         |
| Hardcoded predicate names in parser                        | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. The comment at `:494-497` self-describes this as an open-set problem to be solved: "future open-set solution: pass predicate names down to the parser, or change to a syntactically-decidable shape"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Same as above, the predicate set is passed down as data                                                                                              | 03         |
| **Duplication created to evade cross-module dependencies** | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` are **synonymous but independently implemented**; the comment at `ast.rs:845-846` confesses "this is independently implemented here to avoid cross-module dependencies inside parser"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Merged into a single implementation along with the probe data flow in 03                                                                             | 03         |
| checker → internal functions in layers                     | `checker.rs:4857` / `4986` / `5246` / `5384` call `layers::termination::{negate_guard, substitute_const_expr}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Promoted to the public API of `sema/refine/` or `sema/layers/termination`                                                                            | 02         |
| 3 parallel type representations                            | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (`ir.rs:3` re-exported via `pub use`) + the serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Consolidated into a single representation                                                                                                            | 03         |
| Hand-written synonym table                                 | `types/mono.rs:618-643` `from_builtin_name` maintains branches like `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Deleted along with the type-representation consolidation                                                                                             | 03         |
| **3 parallel operator enums**                              | `ast.rs:191` `pub enum BinOp` (20 variants) / `ast.rs:217` `pub enum UnOp` (4, with the unique `Deref`); `types/const_data.rs:234` `pub enum BinOp` (18, `Ne` rather than `Neq`) / `:321` `pub enum UnOp` (4, with the unique `BitNot`); `middle/core/bytecode.rs:70` `BinaryOp` (11, yet another naming with `Rem`/`Xor`/`Sar`) + `:97` `UnaryOp` + `:106` `CompareOp`. **No `From` / `TryFrom` between ast and const_data**, only the hand-written lossy free functions at `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot` etc. return `None` and silently drop), forcing 5+ files to alias at import (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`), see `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, `middle/core/ir_gen.rs:2317` | The L2 two sets consolidated by single-source generation (see 05); the bytecode-layer set is cross-checked by the opcode build-time gate (see 06 F8) | 03 / 05    |
| `include!`                                                 | `checker.rs:5618`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Replaced by a real `mod`                                                                                                                             | 02         |

## Future Extension Guide

### Checklist When Adding a Language Feature

1. **First consult Routing Guide A** to determine the landing point. Do not rely on intuition to
   find files.
2. **Types first**: if the feature involves types, follow the single representation in 03; do not
   introduce a third representation.
3. **Gate in sync**: diagnostic-code changes must simultaneously change `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table** — `build.rs` enforces consistency among the
   three.
4. **Stage registration**: if the feature introduces new compile-time obligations, register them in
   the `Stage` enum and `Obligations`.
5. **Reverse-dependency check**: new code must not let L2 depend on L3, or L4 depend on L1/L2.
6. **Test wiring**: a new test directory must also add a `mod` declaration in the parent module,
   otherwise it will not run — this will be intercepted by the newly-added CI check.

### When Adding a "Layer" (Such as a Future New Static Check)

1. Create a module under `sema/layers/`, **not depending on layers with larger ordinals**.
2. Output goes into `Obligations`, consumed uniformly by the orchestration layer.
3. Update `sema/layers/README.md` — **let it describe the actual layer order**, not the intended
   one. The current file describes an architecture that was never built.
4. Register in the `Stage` enum, and explicitly declare in which modes it runs in
   `Program::stages()`.

---

## Where to Find the Maintenance Mechanism

The anti-regression gate (CI check list, decision procedure, three prohibitions) has moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md). This document only answers "where does
things go": dependency-direction specifications, target directory structure, feature routing guide,
naming conventions.
