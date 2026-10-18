# Feature Routing and Dependency Specification

> **Companion Design Document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). It
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing tables, and anti-rebound mechanisms** within the four-layer model.
>
> The four-layer model itself, the glossary, execution stages, and acceptance criteria are described
> in the main text of RFC-039. The detailed design of each layer is covered in the rest of the
> documents in this directory.

## Dependency Direction Specification

The following rules are enforced by CI checks (see
[Anti-Rebound Mechanism](#anti-rebound-mechanism)):

| Rule                                                                    | Check Method                                                                                                      | Current Status                                                                                                                             |
| ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `include!` is prohibited                                                | `grep -rn 'include!' src/` should be 0                                                                            | 1 occurrence (`checker.rs:5618`)                                                                                                           |
| L2 must not `use` L3                                                    | Scan `use ...typecheck` in `src/frontend/core/{lexer,parser}/`                                                    | 2 occurrences exist, must be cleared (see Routing Table C)                                                                                 |
| L3/L4 must not `use` L2                                                 | Scan `use ...frontend::{lexer,parser,module}` in `sema/`, `proof/`, `middle/`, `backends/`                        | Currently 0; baseline taken as 0 (AST is a top-level domain, no exemption clause needed)                                                   |
| L4 must not back-reference L1/L2                                        | Scan `use ...frontend::core::{lexer,parser}` in `src/backends/`, `src/middle/`                                    | Baseline must first be established                                                                                                         |
| opcode vocabulary belongs exclusively to `middle/bytecode/`             | Scan `src/backends/` — must not define opcode constants (only import); `sema/` must not contain opcode references | Current vocabulary is in `backends/common/opcode.rs` (forming an L3→L4 reverse dependency), to be migrated with the bytecode domain merger |
| `pub(crate)` cross-layer leakage only allowed to decrease, not increase | Count and add to the baseline file                                                                                | At least 1 occurrence (`collect_used_in_type` used by `inference/statements.rs:16`)                                                        |
| Hardcoded type/predicate names prohibited                               | Scan type-name string literals inside `src/frontend/core/parser/`                                                 | `"Terminates"`, etc.                                                                                                                       |

## Target Directory Structure (After All Construction Phases Complete)

The following is the form **after all construction phases are complete**, not the goal of the first
step. The gap between the current form and the target form is shown in the mapping table below.

```
src/
├── driver/                        # L1 orchestration: the sole decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustive) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-stage diagnostic aggregation + exit-code determination
│
├── frontend/                      # L2 lexical and syntactic
│   ├── lexer/                     #   Declarative: numbers unified as scan_radix(base),
│   │   ├── mod.rs                 #     single escape-decoding implementation (merge the 3 copies in literals.rs)
│   │   ├── number.rs              #     replaces the 4 line-by-line identical radix scanners
│   │   ├── escape.rs              #     single escape decoder
│   │   ├── string.rs              #     strings / multi-line strings
│   │   ├── fstring.rs             #     f-string; interpolation advances to parser rather than nested tokenize
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: a single precedence table, no raw magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     grammar definition (newly added after grammar-driven refactor)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder node
│   └── module/                    #   Multi-file: discovery / registry / role
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merge the current ~60 lines of near-duplication)
│       └── discover.rs            #     discovery strategy that follows `use` traces
│
├── ast/                           # The sole AST + sole type carrier (top-level domain, parallel to parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after convergence; see 03 for handling of dead variants)
│   ├── stmt.rs                    #   StmtKind (remove signature_params)
│   ├── type_.rs                   #   The sole type structure Type + NameKind (26 → converged variant set)
│   └── pattern.rs
│
├── sema/                          # L3 upper half: type and static analysis (currently typecheck/)
│   ├── types/                     #   MonoType working representation + solving (see 03 for the two roles)
│   │   ├── mod.rs
│   │   ├── repr.rs                #     MonoType definition (working representation)
│   │   ├── universe.rs            #     UniverseLevel (currently inside mono.rs)
│   │   ├── infer.rs               #     Inference entry point
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     constant evaluation (includes the sole const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     Sole ConstExpr; single source of operators (merge the current 2 sets)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   Expression / statement inference
│   │   ├── mod.rs                 #     StatementChecker lives here
│   │   ├── call/                  #     Split infer_call_expr (see 03/04 for the split axes)
│   │   │   ├── mod.rs             #       Dispatch by "callee form"
│   │   │   ├── construct.rs       #       Sum-type construction / generic-type construction
│   │   │   ├── generic.rs         #       Explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       Overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (sole three-link model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level checks (after checker.rs is split)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     Annotation type-name validation + is_predicate_head
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     Import/export / unused imports
│   │   └── semantic_tokens.rs     #     Semantic token collection (currently pasted in via include!)
│   ├── refine/                    #   Refinement types (currently checker.rs 3862-5616)
│   │   ├── mod.rs                 #     collect_refined_binding_checks
│   │   ├── binding.rs             #     revalidate_refined
│   │   ├── call_arg.rs            #     check_call_arg_refinements
│   │   ├── return_.rs             #     check_return_refinement
│   │   └── walk.rs                #     refined_walk_* + RefinedWalkCtx
│   └── layers/                    #   Cross-cutting analysis
│       ├── README.md              #     Must reflect the actual layer order, not the intended one
│       ├── ownership.rs
│       ├── termination.rs
│       ├── predicate.rs
│       └── equivalence.rs
│
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to top level; the driver obligations ledger holds proof_calls, the three sema layers share the SMT backend, so it is promoted to top level because it is consumed across layers)
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
│   │   ├── operand.rs             #   Operand (Temp/Label/Register dead variants already removed)
│   │   ├── instr.rs               #   Instruction (includes Phi)
│   │   ├── block.rs               #   BasicBlock
│   │   ├── fn_.rs                 #   FunctionIR / LocalSlot
│   │   └── verify.rs              #   verify_loose / verify_ssa
│   ├── lower/                     #   AST → IR (after ir_gen.rs is split)
│   │   ├── mod.rs                 #     AstToIrGenerator
│   │   ├── scope.rs               #     Scope and register frame (RAII)
│   │   ├── module_ir.rs
│   │   ├── const_eval.rs
│   │   ├── globals.rs
│   │   ├── stmt.rs
│   │   ├── curry.rs
│   │   ├── loops.rs
│   │   ├── query.rs               #     Pure query utilities
│   │   ├── expr/                  #     Expression lowering
│   │   └── synth.rs               #     The sole place allowed to construct ast::Expr
│   ├── passes/                    #   Passes on IR
│   │   ├── mono/                  #     Monomorphization (remove 415 lines of leftover closure code)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 stack slot (no real register file)
│   └── bytecode/                  #   Bytecode domain: opcode vocabulary + container + codec
│       ├── opcode.rs              #     Sole opcode vocabulary (+ build-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file for easy diffing)
│       ├── decode/                #     Decoding (split from the current 1400-line From)
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
│   │   ├── heap.rs                #   Shared inside L4: heap layout
│   │   └── value.rs               #   Shared inside L4: runtime value
│   ├── interpreter/               #   Interpreter (the sole Executor implementation)
│   │   ├── executor.rs
│   │   ├── debug.rs               #   opcode dispatch table
│   │   ├── ffi.rs
│   │   └── ops/                   #   Split by instruction family
│   └── runtime/                   #   DAG task scheduler (not a second execution backend)
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 standard library (Rust implementation + .yx mixed)
├── util/                          # Cross-layer utilities (diagnostics, span, cache, i18n; keep the existing name — the Go "avoid util" cited in 08 is a moderate fit; renaming would be net churn with no domain benefit, so it is recorded as an exception)
├── formatter/                     # Formatter
├── lsp/                           # Language service
├── repl/                          # REPL (if kept, see Section D of 06)
└── package/                       # Package management
```

### Responsibility and Boundary Invariants for Each Directory

| Directory                                 | Responsibility                                                                          | Invariants                                                                                                                                                         |
| ----------------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `driver/`                                 | Decide which stages to run, how many times, and how failures propagate                  | **Must not** be reverse-depended on by L2/L3/L4; the stage set must be an exhaustive enum                                                                          |
| `frontend/lexer`, `frontend/parser`       | Lexical and syntactic; construct `ast/` nodes                                           | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics are always passed down as data |
| `ast/`                                    | The sole AST and type representation (top-level shared data domain, parallel to parser) | **Must not** contain a second definition of the same concept; must not `use` other layers (pure data and pure functions)                                           |
| `frontend/module/`                        | Multi-file discovery, registry, role                                                    | Only performs discovery and registration, **does not run analysis stages**; analysis is scheduled by `driver`                                                      |
| `sema/`                                   | Type checking and static analysis                                                       | **Must not** construct IR; unaware of opcodes                                                                                                                      |
| `proof/`                                  | Proof result types, SMT backend                                                         | The Solver trait is the sole backend abstraction point; **must not** `.expect()` panic                                                                             |
| `middle/ir/`                              | IR definition and invariants                                                            | IR obeys SSA discipline; `verify_ssa` green is a merge prerequisite                                                                                                |
| `middle/lower/`                           | AST → IR                                                                                | The **sole** place allowed to construct `ast::Expr` is `synth.rs`                                                                                                  |
| `middle/codegen/`                         | IR → bytecode lowering                                                                  | Depends only on IR, not on `sema`; does not define the opcode vocabulary                                                                                           |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + codec                            | **opcode vocabulary belongs exclusively here**; `backends/` only consumes and does not define; `sema` is unaware of opcodes                                        |
| `backends/`                               | Execution                                                                               | **Must not** back-reference L1/L2; unaware of AST                                                                                                                  |
| `middle/passes/mono/`                     | Passes on IR                                                                            | Must not reference `frontend`                                                                                                                                      |
| `formatter/`, `lsp/`, `repl/`, `package/` | Outer tool layers (not inside the L1–L4 model)                                          | Only consume `pub` APIs from L1–L4; must not touch `pub(crate)` (the leakage count also constrains them)                                                           |

### Current → Target Mapping

| Current                                                                                                                   | Target                                                                                                              | Belongs To |
| ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------- |
| `frontend/pipeline.rs` (740 lines, 5-stage hand-written if chain)                                                         | `driver/` + `Program::SingleFile`                                                                                   | 02         |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entry points, 60% divergence)                                            | `frontend/module/` keeps only discovery/registration; compilation logic goes to `driver`                            | 02         |
| `frontend/compiler.rs` (267 lines)                                                                                        | Slimmed down to a thin wrapper around `driver`                                                                      | 02         |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 copies of radix scanner + 3 copies of escape decoder)                    | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                  | 05         |
| `frontend/core/parser/pratt/` (hand-written match dispatch, two BP ladders)                                               | `frontend/parser/grammar/` + a single precedence table                                                              | 05         |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                | Demolished into top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                  | 03 / D1    |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8-10 segments of responsibility)                           | `frontend/parser/` split + delete 120 lines of dead old grammar                                                     | 05         |
| `typecheck/checker.rs` (5618 lines + 1547 lines via `include!`)                                                           | `sema/check/` + `sema/refine/`; `include!` becomes a real `mod`                                                     | 02 / 05    |
| `typecheck/checker/semantic_tokens.rs` (1547 lines, no module identity)                                                   | `sema/check/semantic_tokens.rs` (real module)                                                                       | 02         |
| `typecheck/inference/expressions.rs` (5525 lines)                                                                         | `sema/infer/{call/,expr,stmt}.rs`                                                                                   | 05 / 04    |
| `typecheck/inference/statements.rs` (2887 lines)                                                                          | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                          | 05         |
| `typecheck/types/mono.rs` (1077 lines, includes synonym table)                                                            | `sema/types/{repr,universe}.rs`                                                                                     | 03         |
| `typecheck/types/const_data.rs` (second operator enum set)                                                                | Merge with `ast::Type` operators                                                                                    | 03         |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                              | Delete                                                                                                              | 06         |
| `typecheck/layers/dispatch.rs` (entire chain has zero calls)                                                              | Delete                                                                                                              | 06         |
| `typecheck/layers/`                                                                                                       | `sema/layers/` + `README.md` changed to the actual layer order                                                      | 06         |
| `typecheck/proof/`                                                                                                        | Promoted to top-level `proof/`                                                                                      | 02         |
| `middle/core/ir_gen.rs` (8448 lines, zero tests)                                                                          | `middle/lower/` (14 modules)                                                                                        | 04         |
| `middle/core/ir.rs` (905 lines)                                                                                           | `middle/ir/` + `verify.rs`                                                                                          | 04         |
| `middle/core/bytecode.rs` (2422 lines, 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization) | Merged into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules) | 06         |
| `backends/common/opcode.rs` (83 constant vocabulary)                                                                      | `middle/bytecode/opcode.rs` (eliminate the L3→L4 reverse dependency)                                                | 06         |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                 | Delete `416-830`, keep the 340 lines                                                                                | 06         |
| `std/fs.rs`, `std/net.rs`, etc. (native API bindings)                                                                     | Position unchanged                                                                                                  | —          |
| `repl/` (1152 lines, zero tests, no echo)                                                                                 | Keep or delete; see Section D of 06                                                                                 | 06         |

**Renaming directories is not the goal.** The changes fall into two categories:

- **Internal structure that must change**: the **internal structure** of three files —
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (7165 lines including `include!`),
  and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: rename.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`,
  `parser/ast.rs` demolished into top-level `ast/` (the `Type` definition goes into `type_.rs`), the
  opcode vocabulary migrates to `middle/bytecode/`, to be completed together with P5/P6 (resolution
  D1). **No phasing, no "optional".** The rationale: directory names are the first-glance signal of
  responsibility — `middle/core/` currently contains three different things (`ir.rs`, `ir_gen.rs`,
  `bytecode.rs`); the name is already failing. AST is consumed by all layers, so placing it under
  `frontend/` would mislead downstream paths. Boundaries are guaranteed by CI checks, but **the
  check rules reference the directory responsibility table, which must match the actual directory
  names**, otherwise `check-boundary.py` cannot determine whether code is placed correctly.

### Naming Conventions

| Rule                                                                        | Rationale                                                                                                                                                                            |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A concept has exactly one owning module                                     | Prevents parallel definitions like "3 sets of type representations"                                                                                                                  |
| Submodules of `foo.rs` are named `foo/`                                     | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                                  |
| Cross-layer references must go through `pub` APIs, not `pub(crate)` leakage | The **count** of `pub(crate)` leakages is used as a boundary-erosion indicator, only allowed to decrease, not increase (this is a semantic-boundary metric, not a line-count metric) |
| Internal implementations do not add `pub` unless they are cross-layer APIs  | Reduces the visibility surface                                                                                                                                                       |
| Type filenames `type_.rs`, `fn_.rs` avoid keywords                          | `type` / `fn` are reserved words starting in Rust 2024                                                                                                                               |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                    | See Section D of 06: parallel copies as a way to express "new version" are a common source of dead code                                                                              |

## Feature Routing Table

**Answers the question: "I want to add X — where should I change?"** This is the part of the
document set with the most long-term value — it does not become invalid with any single refactor.

### Routing Table A: Language Features

| What to Add                         | L2 Lexical                                                        | L2 Syntactic                                                                                                                                            | L3 Semantic                                                                                                                                                       | L3 IR                                           | L4 Execution                                        | Gate / Documentation                                                                                        |
| ----------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **Binary operator**                 | Keyword table in `lexer/state.rs`, add `TokenKind` in `tokens.rs` | Add BP constant in `pratt/precedence.rs`, add arm in `led.rs`'s `infix_info`, add mapping in `led.rs`'s `parse_binary`, add `BinOp` variant in `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                                      | Corresponding lowering in `lower/`              | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                               |
| **Unary operator**                  | Same as above                                                     | Add arm to `nud.rs`'s `prefix_info` + new parse function + add `UnOp` variant in `ast.rs`                                                               | `infer_unary`                                                                                                                                                     | Same as above                                   | Same as above                                       | RFC-010 table                                                                                               |
| **Constant-evaluation operator** ⚠️ | —                                                                 | —                                                                                                                                                       | **`BinOp` at `types/const_data.rs:234` / `UnOp` at `:321` are a second set of enums parallel to `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` between them** | `ir_gen.rs:2317` requires `use ast::BinOp as B` | —                                                   | **No gate** — see Routing Table C                                                                           |
| **Expression form**                 | —                                                                 | Add `Expr` variant in `ast.rs` + `nud.rs`/`led.rs`                                                                                                      | Add infer method in `inference/expressions.rs`                                                                                                                    | Add module in `lower/exprs/`                    | `opcode.rs` or reuse                                | —                                                                                                           |
| **Statement form**                  | —                                                                 | Add `StmtKind` variant in `ast.rs` + `statements/`                                                                                                      | `inference/statements.rs`                                                                                                                                         | `lower/`                                        | Same as above                                       | —                                                                                                           |
| **Type construction**               | —                                                                 | `Type` in `ast.rs` (see 03)                                                                                                                             | `types/mono.rs` + `solver.rs`                                                                                                                                     | `FunctionCode.params` type                      | Serialization                                       | RFC-011                                                                                                     |
| **Pattern form**                    | —                                                                 | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                         | `inference/patterns.rs`                                                                                                                                           | `compile_pattern` in `lower/`                   | `opcode.rs`                                         | RFC-010b                                                                                                    |
| **Diagnostic code**                 | —                                                                 | —                                                                                                                                                       | —                                                                                                                                                                 | —                                               | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (the `build.rs` gate automatically validates) |
| **Warning (W prefix)**              | —                                                                 | —                                                                                                                                                       | —                                                                                                                                                                 | —                                               | —                                                   | Same as above; note that warnings are not counted as errors                                                 |

> **Current pain point**: A binary operator requires changes in **6-7 places** (inside
> parser/lexer) + the **17 production files** that have `BinOp::` downstream. Whether a missed
> change is caught falls into three layers:
>
> - **The compiler will catch it**: any location that does an **exhaustive `match`** on `ast::BinOp`
>   will fail to compile if a change is missed. This is the majority of cases.
> - **The compiler cannot catch it (the real blind spot)**: the `matches!` macro, the `_` fallback
>   arm, and **cross-enum** cases. The first two manifest as "the new operator is treated as a
>   different operator"; the third (`ast::BinOp` ↔ `const_data::BinOp`) is completely silent — both
>   types compile independently.
> - **L4 is not involved**: `backends/interpreter/` has **zero references** to `BinOp` or `ast::` —
>   the interpreter never sees the AST, only opcodes. So downstream pressure is concentrated in L3,
>   not L4.
>
> Goal: after grammar-driven refactor, reduce to 1-2 places + 1 auto-generated place (see 05); the
> cross-enum part is solved by same-source generation (see 05); the third set of operator enums in
> the bytecode layer (`BinaryOp`/`UnaryOp`/`CompareOp`) is consolidated by the opcode build-time
> gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| What to Add                                                | Landing Point                                                                                                                                                             | Places That Need Synchronous Updates                                                                                                                                                                                                                                                                              |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation stage**                                      | Add `Stage` variant in `driver/stage.rs`                                                                                                                                  | `Stage::ALL` in `driver/stage.rs`, corresponding `Program::stages()`, `dispatch` in `driver/mod.rs` (exhaustive match forces completion), CI stage coverage tests                                                                                                                                                 |
| **Cross-stage check layer** (e.g. ownership / termination) | Add module in `sema/layers/`                                                                                                                                              | Stage table registration + `layers/README.md` update (**the call-order table must correspond line-by-line to the call sites in `checker.rs`**) + output integrated into `Obligations`                                                                                                                             |
| **opcode**                                                 | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`; migrates with the bytecode domain merger; all touch points consolidated into one `middle/` directory) | **5 places**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the decode `match` in `From<BytecodeFile>`, and `translate_*` in `translator.rs`. Only 2 of these are forced by the compiler's exhaustive match; the other 3 only fail when running the `.42` product. A build-time gate is required (see 06) |
| **CLI subcommand**                                         | `Commands` enum in `main.rs`                                                                                                                                              | `pub` entry in `lib.rs` (currently 7), the `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                     |
| **LSP capability**                                         | Add handler in `lsp/handlers/`                                                                                                                                            | `handle_request` in `lsp/server.rs` (currently 274 lines, 13 instances of repeated boilerplate), state in `lsp/world.rs`                                                                                                                                                                                          |
| **Standard library API**                                   | `src/std/*.rs`                                                                                                                                                            | **An automatic gate already exists**: `gen_interfaces.rs` does byte-for-byte comparison + `gen_docs.rs` detects range drift. This is the most mature pattern in the entire project, and should be extended to the opcode table and type table                                                                     |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions to Eliminate

| Reverse Dependency                                         | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Elimination Method                                                                                                                        | Owning Document |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| parser → typecheck (**only 2 places, the same function**)  | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | The operator-interface spec is passed down as data to the parser; the parser no longer `use`s typecheck                                   | 03              |
| Hardcoded predicate name in parser                         | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. The comment at `:494-497` self-declares that this is an open-set problem to be solved: "Future open-set solution: pass predicate names down to the parser, or change to a form decidable at the syntax layer"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Same as above, the predicate set is passed down as data                                                                                   | 03              |
| **Duplication created to avoid cross-module dependencies** | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` are **synonymous but independently implemented**; the comment at `ast.rs:845-846` self-admits: "This is independently implemented to avoid cross-module dependencies inside the parser"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Merge into a single implementation alongside the probe data flow in 03                                                                    | 03              |
| checker → layers internal functions                        | `checker.rs:4857` / `4986` / `5246` / `5384` call `layers::termination::{negate_guard, substitute_const_expr}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Promote to a public API in `sema/refine/` or `sema/layers/termination`                                                                    | 02              |
| 3 parallel type representations                            | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (re-exported via `pub use` at `ir.rs:3`) + the serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Converge into a single representation                                                                                                     | 03              |
| Hand-written synonym table                                 | `types/mono.rs:618-643` `from_builtin_name` maintains branches like `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Deleted along with the type-representation convergence                                                                                    | 03              |
| **3 parallel operator enums**                              | `ast.rs:191` `pub enum BinOp` (20 variants) / `ast.rs:217` `pub enum UnOp` (4, with the unique `Deref`); `types/const_data.rs:234` `pub enum BinOp` (18, `Ne` instead of `Neq`) / `:321` `pub enum UnOp` (4, with the unique `BitNot`); `middle/core/bytecode.rs:70` `BinaryOp` (11, yet another naming set with `Rem`/`Xor`/`Sar`) + `:97` `UnaryOp` + `:106` `CompareOp`. **There is no `From` / `TryFrom` between ast ↔ const_data**, only a hand-written lossy free function at `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot` etc. return `None` and silently drop), forcing 5+ files to alias at the import site (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`), see `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, `middle/core/ir_gen.rs:2317` | The L2 two sets converge via same-source generation (see 05); the bytecode-layer set is diffed via the opcode build-time gate (see 06 F8) | 03 / 05         |
| `include!`                                                 | `checker.rs:5618`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Change to a real `mod`                                                                                                                    | 02              |

## Future Extension Guidelines

### Checklist When Adding a Language Feature

1. **Check Routing Table A first** to determine the landing point. Don't rely on intuition to find
   files.
2. **Type-first**: if the feature involves types, follow the single representation in 03, don't add
   a third one.
3. **Gate in sync**: diagnostic-code changes must simultaneously update `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table** — `build.rs` will enforce consistency among the
   three.
4. **Stage registration**: if the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Reverse-dependency check**: new code must not let L2 depend on L3, or L4 depend on L1/L2.
6. **Test wiring**: new test directories must also add a `mod` declaration in the parent module,
   otherwise they will not run — this will be caught by a newly added CI check.

### When Adding a "Layer" (e.g. a future new static check)

1. Create the module in `sema/layers/`, **do not depend on layers with a higher sequence number**.
2. Outputs go into `Obligations` and are consumed uniformly by the orchestration layer.
3. Update `sema/layers/README.md` — **let it describe the actual layer order** (line numbers are
   taken from the current tree, and the anchor is the function name / driver point).
4. Register in the `Stage` enum, and explicitly declare in `Program::stages()` under which modes it
   runs.

---

## Where to Find the Maintenance Mechanism

The anti-rebound gate (CI check list, decision procedure, the three prohibitions) has been moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md). This document only answers "where things
go": dependency direction specification, target directory structure, feature routing tables, and
naming conventions.
