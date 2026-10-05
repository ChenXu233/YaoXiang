# Functional Routing and Dependency Specification

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md), defining
> the **module boundaries, dependency directions, target directory structure, functional routing
> table, and anti-rebound mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution phases, and acceptance criteria are in the main
> text of RFC-039. Detailed design for each layer is in the other documents of this directory.

## Dependency Direction Specification

The following rules are enforced by CI checks (see
[Anti-Rebound Mechanism](#anti-rebound-mechanism)):

| Rule                                                             | Check Method                                                                                                     | Current State                                                                                                      |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Forbid `include!`                                                | `grep -rn 'include!' src/` should be 0                                                                           | 1 occurrence (`checker.rs:5618`)                                                                                   |
| L2 must not `use` L3                                             | Scan `use ...typecheck` in `src/frontend/core/{lexer,parser}/`                                                   | 2 existing occurrences, need to be cleared (see Routing Table C)                                                   |
| L3/L4 must not `use` L2                                          | Scan `use ...frontend::{lexer,parser,module}` in `sema/`, `proof/`, `middle/`, `backends/`                       | Currently 0, baseline taken as 0 (AST is top-level domain, no exemption clause needed)                             |
| L4 must not reverse-reference L1/L2                              | Scan `use ...frontend::core::{lexer,parser}` in `src/backends/`, `src/middle/`                                   | Baseline needs to be established first                                                                             |
| Opcode vocabulary exclusively belongs to `middle/bytecode/`      | Scan `src/backends/` should not define opcode constants (only import); `sema/` should not have opcode references | Vocabulary currently at `backends/common/opcode.rs` (forming L3→L4 reverse), migrating with bytecode domain merger |
| `pub(crate)` cross-layer leakage can only decrease, not increase | Count and merge into baseline file                                                                               | At least 1 occurrence (`collect_used_in_type` used by `inference/statements.rs:16`)                                |
| Forbid hardcoded type/predicate names                            | Scan type name string literals within `src/frontend/core/parser/`                                                | `"Terminates"` etc.                                                                                                |

## Target Directory Structure (After All Construction Phases Complete)

The following is the form **after all construction phases complete**, not the goal of the first
step. See the mapping table below for the gap between current and target forms.

```
src/
├── driver/                        # L1 Orchestration: the only decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustive) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-phase diagnostic aggregation + exit code determination
│
├── frontend/                      # L2 Lexical and Syntactic
│   ├── lexer/                     #   Declarative: numbers unified as scan_radix(base),
│   │   ├── mod.rs                 #     single escape decoding implementation (merging current 3 copies in literals.rs)
│   │   ├── number.rs              #     Replaces 4 line-by-line identical radix scanners
│   │   ├── escape.rs              #     Single escape decoder
│   │   ├── string.rs              #     String / multi-line string
│   │   ├── fstring.rs             #     f-string; interpolation changes to feed parser, no nested tokenize
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: single priority table, no bare magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     Grammar definition (added after grammar-driven)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder node
│   └── module/                    #   Multi-file: discovery / registry / role
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merging current 60 lines of near-duplicates)
│       └── discover.rs            #     Discovery strategy along use tracking
│
├── ast/                           # Single AST + single type carrier (top-level domain, parallel to parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after convergence, see 03 on handling of dead variants)
│   ├── stmt.rs                    #   StmtKind (remove signature_params)
│   ├── type_.rs                   #   Single type structure Type + NameKind (26 → converged variant set)
│   └── pattern.rs
│
├── sema/                          # L3 upper half: types and static analysis (current typecheck/)
│   ├── types/                     #   MonoType working representation + solving (see 03 two roles)
│   │   ├── mod.rs
│   │   ├── repr.rs                #     MonoType definition (working representation)
│   │   ├── universe.rs            #     UniverseLevel (currently in mono.rs)
│   │   ├── infer.rs               #     Inference entry point
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     Constant evaluation (including single const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     Single ConstExpr; single source of operators (merging current 2 sets)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   Expression / statement inference
│   │   ├── mod.rs                 #     Where StatementChecker lives
│   │   ├── call/                  #     Split infer_call_expr (see 03/04 split axes)
│   │   │   ├── mod.rs             #       Dispatch by "callee form"
│   │   │   ├── construct.rs       #       Sum type construction / generic type construction
│   │   │   ├── generic.rs         #       Explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       Overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (single three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level check (after splitting current checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     Annotation type name validation + is_predicate_head
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     Import/export / unused imports
│   │   └── semantic_tokens.rs     #     Semantic token collection (currently spliced in via include!)
│   ├── refine/                    #   Refinement types (current checker.rs 3862-5616)
│   │   ├── mod.rs                 #     collect_refined_binding_checks
│   │   ├── binding.rs             #     revalidate_refined
│   │   ├── call_arg.rs            #     check_call_arg_refinements
│   │   ├── return_.rs             #     check_return_refinement
│   │   └── walk.rs                #     refined_walk_* + RefinedWalkCtx
│   └── layers/                    #   Cross-cutting analysis
│       ├── README.md              #     Must be actual layer order, not intended layer order
│       ├── ownership.rs
│       ├── termination.rs
│       ├── predicate.rs
│       └── equivalence.rs
│
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to first level; driver obligations ledger holds proof_calls, sema's three layers share SMT backend, cross-layer consumption so promoted to first level)
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
│   │   ├── operand.rs             #   Operand (Temp/Label/Register dead variants deleted)
│   │   ├── instr.rs               #   Instruction (includes Phi)
│   │   ├── block.rs               #   BasicBlock
│   │   ├── fn_.rs                 #   FunctionIR / LocalSlot
│   │   └── verify.rs              #   verify_loose / verify_ssa
│   ├── lower/                     #   AST → IR (after splitting current ir_gen.rs)
│   │   ├── mod.rs                 #     AstToIrGenerator
│   │   ├── scope.rs               #     Scope and register frames (RAII)
│   │   ├── module_ir.rs
│   │   ├── const_eval.rs
│   │   ├── globals.rs
│   │   ├── stmt.rs
│   │   ├── curry.rs
│   │   ├── loops.rs
│   │   ├── query.rs               #     Pure query utilities
│   │   ├── expr/                  #     Expression lowering
│   │   └── synth.rs               #     The only place allowed to construct ast::Expr
│   ├── passes/                    #   Passes on IR
│   │   ├── mono/                  #     Monomorphization (delete 415 lines of closure remnants)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 stack slots (no real register file)
│   └── bytecode/                  #   Bytecode domain: opcode vocabulary + container + codec
│       ├── opcode.rs              #     Single opcode vocabulary (+ generation-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file for easy comparison)
│       ├── decode/                #     Decoding (split from current 1400-line From)
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
├── backends/                      # L4 Execution
│   ├── common/
│   │   ├── heap.rs                #   L4 internal sharing: heap layout
│   │   └── value.rs               #   L4 internal sharing: runtime value
│   ├── interpreter/               #   Interpreter (single Executor implementation)
│   │   ├── executor.rs
│   │   ├── debug.rs               #   opcode dispatch table
│   │   ├── ffi.rs
│   │   └── ops/                   #   Split by instruction family
│   └── runtime/                   #   DAG task scheduler (not a second execution backend)
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 standard library (Rust implementation + .yx mixed)
├── util/                          # Cross-layer utilities (diagnostics, span, cache, i18n; keeping existing name—08's reference to Go "avoid util" is moderately applicable, renaming is pure churn with no domain benefit, recorded as exception)
├── formatter/                     # Formatter
├── lsp/                           # Language service
├── repl/                          # REPL (if retained, see 06 section D)
└── package/                       # Package management
```

### Responsibility and Boundary Invariants for Each Directory

| Directory                                 | Responsibility                                                                        | Invariant                                                                                                                                              |
| ----------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `driver/`                                 | Decides which phases to run, how many times, how failures propagate                   | **Must not** be reverse-depended on by L2/L3/L4; phase set must be an exhaustive enum                                                                  |
| `frontend/lexer`, `frontend/parser`       | Lexical and syntactic, construct `ast/` nodes                                         | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, operator semantics all flow down via data |
| `ast/`                                    | Single AST and type representation (top-level shared data domain, parallel to parser) | **Must not** have a second definition of the same concept; does not `use` other layers (pure data and pure functions)                                  |
| `frontend/module/`                        | Multi-file discovery, registry, role                                                  | Only does discovery and registration, **does not run analysis phases**; analysis is scheduled by `driver`                                              |
| `sema/`                                   | Type check and static analysis                                                        | **Must not** construct IR; unaware of opcode                                                                                                           |
| `proof/`                                  | Proof result types, SMT backend                                                       | Solver trait is the only backend abstraction point; **must not** `.expect()` panic                                                                     |
| `middle/ir/`                              | IR definition and invariants                                                          | IR satisfies SSA discipline; `verify_ssa` green is a merge prerequisite                                                                                |
| `middle/lower/`                           | AST → IR                                                                              | The **only** place allowed to construct `ast::Expr` is `synth.rs`                                                                                      |
| `middle/codegen/`                         | IR → bytecode lowering                                                                | Only depends on IR, not on `sema`; does not define opcode vocabulary                                                                                   |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + codec                          | **Opcode vocabulary exclusively belongs here**; `backends/` only consumes, does not define; `sema` is unaware of opcode                                |
| `backends/`                               | Execution                                                                             | **Must not** reverse-reference L1/L2; unaware of AST                                                                                                   |
| `middle/passes/mono/`                     | Passes on IR                                                                          | Must not reference `frontend`                                                                                                                          |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tool layers (not in L1-L4 model)                                           | Only consume L1-L4's `pub` API; must not touch `pub(crate)` (leakage count constrains them as well)                                                    |

### Current → Target Mapping

| Current                                                                                                                    | Target                                                                                                             | Attribution |
| -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ----------- |
| `frontend/pipeline.rs` (740 lines, 5-phase handwritten if chain)                                                           | `driver/` + `Program::SingleFile`                                                                                  | 02          |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entries, 60% divergence)                                                  | `frontend/module/` only retains discovery/registration; compilation logic returns to `driver`                      | 02          |
| `frontend/compiler.rs` (267 lines)                                                                                         | Slim down to thin wrapper of `driver`                                                                              | 02          |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                       | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                 | 05          |
| `frontend/core/parser/pratt/` (handwritten match dispatch, two BP ladders)                                                 | `frontend/parser/grammar/` + single priority table                                                                 | 05          |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                 | Demolish to top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                     | 03 / D1     |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8-10 responsibilities)                                      | `frontend/parser/` split + delete 120 lines of dead legacy syntax                                                  | 05          |
| `typecheck/checker.rs` (5618 lines + `include!` 1547 lines)                                                                | `sema/check/` + `sema/refine/`; `include!` changed to real `mod`                                                   | 02 / 05     |
| `typecheck/checker/semantic_tokens.rs` (1547 lines, no module identity)                                                    | `sema/check/semantic_tokens.rs` (real module)                                                                      | 02          |
| `typecheck/inference/expressions.rs` (5525 lines)                                                                          | `sema/infer/{call/,expr,stmt}.rs`                                                                                  | 05 / 04     |
| `typecheck/inference/statements.rs` (2887 lines)                                                                           | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                         | 05          |
| `typecheck/types/mono.rs` (1077 lines, contains synonym table)                                                             | `sema/types/{repr,universe}.rs`                                                                                    | 03          |
| `typecheck/types/const_data.rs` (second set of operator enums)                                                             | Merge with `ast::Type`'s operators                                                                                 | 03          |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                               | Delete                                                                                                             | 06          |
| `typecheck/layers/dispatch.rs` (entire chain zero-called)                                                                  | Delete                                                                                                             | 06          |
| `typecheck/layers/`                                                                                                        | `sema/layers/` + `README.md` changed to actual layer order                                                         | 06          |
| `typecheck/proof/`                                                                                                         | Promoted to first-level `proof/`                                                                                   | 02          |
| `middle/core/ir_gen.rs` (8448 lines, zero tests)                                                                           | `middle/lower/` (14 modules)                                                                                       | 04          |
| `middle/core/ir.rs` (905 lines)                                                                                            | `middle/ir/` + `verify.rs`                                                                                         | 04          |
| `middle/core/bytecode.rs` (2422 lines, 1400 lines `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization) | Merge into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules) | 06          |
| `backends/common/opcode.rs` (83 constant vocabulary)                                                                       | `middle/bytecode/opcode.rs` (eliminate L3→L4 reverse)                                                              | 06          |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                  | Delete `416-830`, keep 340 lines                                                                                   | 06          |
| `std/fs.rs`, `std/net.rs` etc. (native API bindings)                                                                       | Position unchanged                                                                                                 | —           |
| `repl/` (1152 lines, zero tests, no echo)                                                                                  | Retain or delete, see 06 section D                                                                                 | 06          |

**Directory renaming is not the goal.** Changes fall into two categories:

- **Internal structure that must be changed**: the **internal structure** of these three files:
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (7165 lines including `include!`),
  `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: rename.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`,
  `parser/ast.rs` demolished to top-level `ast/` (`Type` definition into `type_.rs`), opcode
  vocabulary migrates to `middle/bytecode/`, completed together with P5/P6 (resolution D1). **No
  phasing, no "optional".** Reason: directory name is the first signal of
  responsibility—`middle/core/` contains `ir.rs`, `ir_gen.rs`, `bytecode.rs` three different things,
  the name has already failed; AST is consumed by all layers, placing it under `frontend/` would
  mislead downstream paths. Boundaries are guaranteed by CI checks, but **check rules reference the
  directory responsibility table, which must be consistent with actual directory names**, otherwise
  `check-boundary.py` cannot determine if code is in the right place.

### Naming Conventions

| Rule                                                                               | Reason                                                                                                                                                           |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| One concept belongs to only one module                                             | Prevent parallel definitions like "3 sets of type representations"                                                                                               |
| `foo.rs`'s submodule directory is called `foo/`                                    | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                              |
| Cross-layer references must go through `pub` API, not through `pub(crate)` leakage | `pub(crate)` leakage **count** as boundary erosion indicator, can only decrease not increase (this is a semantic boundary indicator, not a line count indicator) |
| Internal implementation does not add `pub` unless it is cross-layer API            | Reduce visibility surface                                                                                                                                        |
| Type file names `type_.rs`, `fn_.rs` avoid keywords                                | `type` / `fn` are reserved words starting from Rust 2024                                                                                                         |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                           | See 06 section D: parallel copies expressing "new version" is a common source of dead code                                                                       |

## Functional Routing Table

**Answering "I want to add X, where should I change it".** This is the most long-term valuable part
of this document group—it will not be invalidated by any refactoring.

### Routing Table A: Language Features

| To add                              | L2 Lexical                                                   | L2 Syntactic                                                                                                                                    | L3 Semantic                                                                                                                                            | L3 IR                                        | L4 Execution                                        | Gate/Doc                                                                                         |
| ----------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **Binary operator**                 | `lexer/state.rs` keyword table, `tokens.rs` adds `TokenKind` | `pratt/precedence.rs` adds BP constant, `led.rs`'s `infix_info` adds arm, `led.rs`'s `parse_binary` adds mapping, `ast.rs` adds `BinOp` variant | `inference/expressions.rs`'s `infer_binary`                                                                                                            | Corresponding `lower/` lowering              | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                    |
| **Unary operator**                  | Same as above                                                | `nud.rs`'s `prefix_info` adds arm + new parse function + `ast.rs` adds `UnOp` variant                                                           | `infer_unary`                                                                                                                                          | Same as above                                | Same as above                                       | RFC-010 table                                                                                    |
| **Constant evaluation operator** ⚠️ | —                                                            | —                                                                                                                                               | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` are parallel second set of enums with `ast::BinOp`/`ast::UnOp`, no `From`/`TryFrom` between them** | `ir_gen.rs:2317` needs `use ast::BinOp as B` | —                                                   | **No gate**—see Routing Table C                                                                  |
| **Expression form**                 | —                                                            | `ast.rs` adds `Expr` variant + `nud.rs`/`led.rs`                                                                                                | `inference/expressions.rs` adds infer method                                                                                                           | `lower/exprs/` adds module                   | `opcode.rs` or reuse                                | —                                                                                                |
| **Statement form**                  | —                                                            | `ast.rs` adds `StmtKind` variant + `statements/`                                                                                                | `inference/statements.rs`                                                                                                                              | `lower/`                                     | Same as above                                       | —                                                                                                |
| **Type construction**               | —                                                            | `ast.rs`'s `Type` (see 03)                                                                                                                      | `types/mono.rs` + `solver.rs`                                                                                                                          | `FunctionCode.params` type                   | Serialization                                       | RFC-011                                                                                          |
| **Pattern form**                    | —                                                            | `pratt/`'s `parse_pattern` + `expr_to_pattern`                                                                                                  | `inference/patterns.rs`                                                                                                                                | `lower/`'s `compile_pattern`                 | `opcode.rs`                                         | RFC-010b                                                                                         |
| **Diagnostic code**                 | —                                                            | —                                                                                                                                               | —                                                                                                                                                      | —                                            | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (`build.rs` gate will auto-verify) |
| **Warning (W prefix)**              | —                                                            | —                                                                                                                                               | —                                                                                                                                                      | —                                            | —                                                   | Same as above; note not counted in error count                                                   |

> **Current pain point**: adding a binary operator requires changing **6-7 places** (within
> parser/lexer) + downstream `BinOp::` in **17 production files**. Whether missed changes can be
> detected falls into three layers:
>
> - **Compiler catches it**: positions doing **exhaustive `match`** on `ast::BinOp`, missed changes
>   directly fail to compile. This category is the majority.
> - **Compiler cannot catch (true blind spots)**: `matches!` macro, `_` catch-all arm, and
>   **cross-enum** three categories. First two missed changes manifest as "new operator treated as
>   another operator"; third category (`ast::BinOp` ↔ `const_data::BinOp`) missed changes are
>   completely silent—two types compile independently fine.
> - **L4 not involved**: `backends/interpreter/` has **zero references** to `BinOp` and `ast::`—the
>   interpreter never sees AST, it only recognizes opcode. Therefore downstream pressure
>   concentrates at L3, not L4.
>
> Goal: after grammar-driven, reduce to 1-2 places + 1 auto-generation (see 05); cross-enum part
> resolved by same-source generation (see 05), third set of operator enums at bytecode layer
> (`BinaryOp`/`UnaryOp`/`CompareOp`) merged via opcode generation-time gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| To add                                                     | Location                                                                                                                                                          | Places needing synchronous change                                                                                                                                                                                                                                                                  |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation phase**                                      | `driver/stage.rs` adds `Stage` variant                                                                                                                            | `driver/stage.rs`'s `Stage::ALL`, corresponding `Program::stages()`, `driver/mod.rs`'s `dispatch` (exhaustive match forces completion), CI phase coverage test                                                                                                                                     |
| **Cross-phase check layer** (e.g. ownership / termination) | `sema/layers/` adds module                                                                                                                                        | Phase table registration + `layers/README.md` update (**currently this README declares layer order opposite to actual execution order**) + output merged into `Obligations`                                                                                                                        |
| **opcode**                                                 | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`, migrates with bytecode domain merger, all touchpoints collected into one `middle/` directory) | **5 places**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, `From<BytecodeFile>` decoding match, `translator.rs`'s `translate_*`. Of these, only 2 are forced by compiler's exhaustive match, the other 3 only blow up when running `.42` products. Requires generation-time gate (see 06) |
| **CLI subcommand**                                         | `main.rs`'s `Commands` enum                                                                                                                                       | `lib.rs`'s `pub` entry (currently 7), `main.rs`'s `match command` (currently 373 lines)                                                                                                                                                                                                            |
| **LSP capability**                                         | `lsp/handlers/` adds handler                                                                                                                                      | `lsp/server.rs`'s `handle_request` (currently 274 lines, 13 duplicate boilerplates), `lsp/world.rs` state                                                                                                                                                                                          |
| **Standard library API**                                   | `src/std/*.rs`                                                                                                                                                    | **Automatic gate already exists**: `gen_interfaces.rs` byte-by-byte comparison + `gen_docs.rs` mark range drift detection. This is the most mature pattern in the project, should be extended to opcode table and type table                                                                       |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions to Eliminate

| Reverse dependency                                       | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Elimination method                                                                                                                        | Attribution doc |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| parser → typecheck (**only 2 places, same function**)    | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Operator interface specification flows down to parser as data, parser no longer `use`s typecheck                                          | 03              |
| Hardcoded predicate names within parser                  | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. ` :494-497` comment states this is an open-set problem to solve: 「future open-set solution: flow predicate names down to parser, or change to syntax-layer decidable form」                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Same as above, predicate set flows down with data                                                                                         | 03              |
| **Duplication created to avoid cross-module dependency** | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` **synonymous but independently implemented**, `ast.rs:845-846` comment self-admits "independent implementation here to avoid parser internal cross-module dependency"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Merged into single implementation along with 03's probe data flow                                                                         | 03              |
| checker → layers internal functions                      | `checker.rs:4857` / `4986` / `5246` / `5384` call `layers::termination::{negate_guard, substitute_const_expr}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Promoted to public API of `sema/refine/` or `sema/layers/termination`                                                                     | 02              |
| 3 parallel type representations                          | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (`ir.rs:3` re-exported via `pub use`) + serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Converged to single representation                                                                                                        | 03              |
| Handwritten synonym table                                | `types/mono.rs:618-643` `from_builtin_name` maintains `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))` etc. branches                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Deleted along with type representation convergence                                                                                        | 03              |
| **3 parallel operator enums**                            | `ast.rs:191` `pub enum BinOp` (20 variants) / `ast.rs:217` `pub enum UnOp` (4, unique `Deref`); `types/const_data.rs:234` `pub enum BinOp` (18, `Ne` not `Neq`) / `:321` `pub enum UnOp` (4, unique `BitNot`); `middle/core/bytecode.rs:70` `BinaryOp` (11, `Rem`/`Xor`/`Sar` another naming) + `:97` `UnaryOp` + `:106` `CompareOp`. **No `From` / `TryFrom` between ast ↔ const_data**, only handwritten lossy free functions in `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot` etc. return `None` silently discarded), forcing 5+ files to alias at import (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`), see `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, `middle/core/ir_gen.rs:2317` | L2 two sets converged by same-source generation (see 05); bytecode layer one set cross-checked by opcode generation-time gate (see 06 F8) | 03 / 05         |
| `include!`                                               | `checker.rs:5618`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Changed to real `mod`                                                                                                                     | 02              |

## Future Extension Guide

### Checklist When Adding a Language Feature

1. **Check Routing Table A first**, determine landing point. Don't find files by intuition.
2. **Type first**: if the feature involves types, follow 03's single representation, don't add a
   third representation.
3. **Gate sync**: diagnostic code changes must simultaneously change `codes/eNxxx.rs`,
   `locales/zh.json`, **RFC-013 code table**—`build.rs` will enforce consistency among the three.
4. **Phase registration**: if the feature introduces a new compile-time obligation, register in
   `Stage` enum and `Obligations`.
5. **Reverse dependency check**: new code must not make L2 depend on L3, or L4 depend on L1/L2.
6. **Test wiring**: new test directory must simultaneously add `mod` declaration in parent module,
   otherwise it will not run—this will be intercepted by newly added CI check.

### When Adding a "Layer" (e.g. Future New Static Check)

1. Create module in `sema/layers/`, **do not depend on layers with larger sequence numbers**.
2. Outputs go into `Obligations`, consumed uniformly by orchestration layer.
3. Update `sema/layers/README.md`—**let it describe actual layer order**, not intended layer order.
   Currently this file describes an architecture that has not been built.
4. Register in `Stage` enum, explicitly declare in which modes to run in `Program::stages()`.

---

## Where to Look for Maintenance Mechanism

Anti-rebound gate (CI check list, decision procedure, three prohibitions) has been moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md). This article only answers "where things
go": dependency direction specification, target directory structure, functional routing table,
naming conventions.
