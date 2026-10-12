# Feature Routing and Dependency Specification

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). It
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing table, and anti-rebound mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution phases, and acceptance criteria are in the
> RFC-039 main text. See the other documents in this directory for the detailed per-layer design.

## Dependency Direction Specification

The following rules are checked by CI (see [Anti-Rebound Mechanism](#anti-rebound-mechanism)):

| Rule                                                             | Check Method                                                                                                    | Current State                                                                                                                  |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| No `include!`                                                    | `grep -rn 'include!' src/` should be 0                                                                          | 1 occurrence (`checker.rs:5618`)                                                                                               |
| L2 must not `use` L3                                             | Scan `use ...typecheck` in `src/frontend/core/{lexer,parser}/`                                                  | 2 occurrences exist, must be removed (see routing table C)                                                                     |
| L3/L4 must not `use` L2                                          | Scan `use ...frontend::{lexer,parser,module}` in `sema/`, `proof/`, `middle/`, `backends/`                      | Currently 0, baseline 0 (AST is top-level domain, no exemption clause needed)                                                  |
| L4 must not reverse-reference L1/L2                              | Scan `use ...frontend::core::{lexer,parser}` in `src/backends/`, `src/middle/`                                  | Baseline must be established first                                                                                             |
| opcode vocabulary solely belongs to `middle/bytecode/`           | Scan `src/backends/` must not define opcode constants (only import); `sema/` must not contain opcode references | Current vocabulary lives in `backends/common/opcode.rs` (forms L3→L4 reverse), migrating along with the bytecode domain merger |
| `pub(crate)` cross-layer leakage may only decrease, not increase | Stat and merge into baseline file                                                                               | At least 1 occurrence (`collect_used_in_type` used by `inference/statements.rs:16`)                                            |
| No hardcoded type/predicate names                                | Scan type-name string literals inside `src/frontend/core/parser/`                                               | e.g. `"Terminates"`                                                                                                            |

## Target Directory Structure (After All Construction Phases Complete)

The following is the form **after all construction phases are complete**, not the goal of the first
step. See the mapping table below for the gap between the current form and the target form.

```
src/
├── driver/                        # L1 orchestration: the only decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustive) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-stage diagnostic aggregation + exit-code determination
│
├── frontend/                      # L2 lexing and parsing
│   ├── lexer/                     #   Declarative: numbers unified to scan_radix(base),
│   │   ├── mod.rs                 #     escape decoding is the sole implementation (merging the 3 copies in literals.rs)
│   │   ├── number.rs              #     Replaces 4 line-by-line identical radix scanners
│   │   ├── escape.rs              #     Sole escape decoder
│   │   ├── string.rs              #     Strings / multi-line strings
│   │   ├── fstring.rs             #     f-strings; interpolation advances toward parser, no nested tokenize
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: a single precedence table, no naked magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     Grammar definition (added after grammar-driven transition)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder node
│   └── module/                    #   Multi-file: discovery / registry / roles
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merging the current ~60 lines of near-duplicates)
│       └── discover.rs            #     Discovery strategy following use
│
├── ast/                           # The sole AST + sole type carrier (top-level domain, sibling of parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after convergence, see 03 on handling dead variants)
│   ├── stmt.rs                    #   StmtKind (signature_params removed)
│   ├── type_.rs                   #   The sole type structure Type + NameKind (26 → post-convergence variant set)
│   └── pattern.rs
│
├── sema/                          # L3 upper: type and static analysis (current typecheck/)
│   ├── types/                     #   MonoType working representation + solving (see 03 on two roles)
│   │   ├── mod.rs
│   │   ├── repr.rs                #     MonoType definition (working representation)
│   │   ├── universe.rs            #     UniverseLevel (currently inside mono.rs)
│   │   ├── infer.rs               #     Inference entry
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     Constant evaluation (with the sole const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     Sole ConstExpr; sole source of operators (merging the current 2 sets)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   Expression / statement inference
│   │   ├── mod.rs                 #     StatementChecker lives here
│   │   ├── call/                  #     Split infer_call_expr (see 03/04 split axes)
│   │   │   ├── mod.rs             #       Dispatch by "callee shape"
│   │   │   ├── construct.rs       #       Sum-type construction / generic-type construction
│   │   │   ├── generic.rs         #       Explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       Overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (sole three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level checks (after checker.rs split)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     Annotation type-name validation + is_predicate_head
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     Import/export / unused imports
│   │   └── semantic_tokens.rs     #     Semantic-token collection (currently spliced in via include!)
│   ├── refine/                    #   Refinement types (current checker.rs 3862-5616)
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
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to top level; the driver obligations ledger holds proof_calls, the three sema layers share the SMT backend and consume it across layers, hence promoted to top level)
│   ├── mod.rs                     #   ProofResult / ProofContext / ProofFunctionCall
│   ├── context.rs
│   ├── verdict.rs
│   ├── dep_graph.rs
│   └── smt/                       #   Solver trait + Z3 backend + gate
│       ├── backend.rs
│       ├── z3_backend.rs
│       └── translate.rs
│
├── middle/                        # L3 lower: IR
│   ├── ir/                        #   SSA IR (see 04)
│   │   ├── mod.rs
│   │   ├── operand.rs             #   Operand (dead Temp/Label/Register variants removed)
│   │   ├── instr.rs               #   Instruction (with Phi)
│   │   ├── block.rs               #   BasicBlock
│   │   ├── fn_.rs                 #   FunctionIR / LocalSlot
│   │   └── verify.rs              #   verify_loose / verify_ssa
│   ├── lower/                     #   AST → IR (after ir_gen.rs split)
│   │   ├── mod.rs                 #     AstToIrGenerator
│   │   ├── scope.rs               #     Scopes and register frames (RAII)
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
│   │   ├── mono/                  #     Monomorphization (delete 415 lines of closure residue)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #   u8 stack slots (no real register file)
│   └── bytecode/                  #   Bytecode domain: opcode vocabulary + container + codec
│       ├── opcode.rs              #     Sole opcode vocabulary (+ generation-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file for cross-checking convenience)
│       ├── decode/                #     Decoding (after splitting the current 1400-line From)
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
├── std/                           # L4 standard library (Rust implementation + .yx hybrid)
├── util/                          # Cross-layer utilities (diagnostics, span, cache, i18n; keep the existing name — the "avoid util" reference in 08 applies with medium strength, renaming would be pure churn with no domain benefit, treat as an exception)
├── formatter/                     # Formatter
├── lsp/                           # Language service
├── repl/                          # REPL (if kept, see 06 section D)
└── package/                       # Package management
```

### Responsibilities and Boundary Invariants Per Directory

| Directory                                 | Responsibility                                                                         | Invariants                                                                                                                                                      |
| ----------------------------------------- | -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Decides which stages to run, how many times, and how failures propagate                | **Must not** be reverse-depended on by L2/L3/L4; the stage set must be an exhaustive enum                                                                       |
| `frontend/lexer`, `frontend/parser`       | Lexing and parsing, constructing `ast/` nodes                                          | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics are all passed down as data |
| `ast/`                                    | The sole AST and type representation (top-level shared data domain, sibling of parser) | **Must not** contain a second definition of the same concept; does not `use` other layers (pure data and pure functions)                                        |
| `frontend/module/`                        | Multi-file discovery, registry, roles                                                  | Only performs discovery and registration, **does not run analysis stages**; analysis is scheduled by `driver`                                                   |
| `sema/`                                   | Type checking and static analysis                                                      | **Must not** construct IR; not aware of opcode                                                                                                                  |
| `proof/`                                  | Proof result types, SMT backend                                                        | The Solver trait is the sole backend abstraction point; **must not** `.expect()` panic                                                                          |
| `middle/ir/`                              | IR definition and invariants                                                           | IR satisfies SSA discipline; `verify_ssa` green is a prerequisite for merge                                                                                     |
| `middle/lower/`                           | AST → IR                                                                               | The **only** place allowed to construct `ast::Expr` is `synth.rs`                                                                                               |
| `middle/codegen/`                         | IR → bytecode lowering                                                                 | Only depends on IR, not on `sema`; does not define the opcode vocabulary                                                                                        |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + codec                           | **Sole owner of the opcode vocabulary**; `backends/` only consumes, never defines; `sema` is not aware of opcode                                                |
| `backends/`                               | Execution                                                                              | **Must not** reverse-reference L1/L2; not aware of AST                                                                                                          |
| `middle/passes/mono/`                     | Pass on IR                                                                             | Must not reference `frontend`                                                                                                                                   |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tooling layer (not within the L1-L4 model)                                  | Only consumes the `pub` API of L1-L4; must not touch `pub(crate)` (the leakage count constrains them as well)                                                   |

### Current → Target Mapping

| Current State                                                                                                             | Target                                                                                                              | Belongs to |
| ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------- |
| `frontend/pipeline.rs` (740 lines, 5-stage hand-written if-chain)                                                         | `driver/` + `Program::SingleFile`                                                                                   | 02         |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entries, 60% branching)                                                  | `frontend/module/` only keeps discovery/registration; compilation logic goes to `driver`                            | 02         |
| `frontend/compiler.rs` (267 lines)                                                                                        | Slimmed to a thin wrapper around `driver`                                                                           | 02         |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                      | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                  | 05         |
| `frontend/core/parser/pratt/` (hand-written match dispatch, two BP ladders)                                               | `frontend/parser/grammar/` + a single precedence table                                                              | 05         |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr` / `StmtKind` / `Type` / `Pattern` definitions)                          | Demolished into top-level `ast/` (`expr` / `stmt` / `type_` / `pattern`)                                            | 03 / D1    |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8-10 responsibility sections)                              | `frontend/parser/` split + delete 120 lines of dead legacy syntax                                                   | 05         |
| `typecheck/checker.rs` (5618 lines + `include!` 1547 lines)                                                               | `sema/check/` + `sema/refine/`; `include!` becomes a real `mod`                                                     | 02 / 05    |
| `typecheck/checker/semantic_tokens.rs` (1547 lines, no module identity)                                                   | `sema/check/semantic_tokens.rs` (real module)                                                                       | 02         |
| `typecheck/inference/expressions.rs` (5525 lines)                                                                         | `sema/infer/{call/,expr,stmt}.rs`                                                                                   | 05 / 04    |
| `typecheck/inference/statements.rs` (2887 lines)                                                                          | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                          | 05         |
| `typecheck/types/mono.rs` (1077 lines, contains synonym table)                                                            | `sema/types/{repr,universe}.rs`                                                                                     | 03         |
| `typecheck/types/const_data.rs` (the second operator enum set)                                                            | Merged with the operators of `ast::Type`                                                                            | 03         |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                              | Delete                                                                                                              | 06         |
| `typecheck/layers/dispatch.rs` (zero callers in the whole chain)                                                          | Delete                                                                                                              | 06         |
| `typecheck/layers/`                                                                                                       | `sema/layers/` + `README.md` rewritten to the actual layer order                                                    | 06         |
| `typecheck/proof/`                                                                                                        | Promoted to top-level `proof/`                                                                                      | 02         |
| `middle/core/ir_gen.rs` (8448 lines, zero tests)                                                                          | `middle/lower/` (14 modules)                                                                                        | 04         |
| `middle/core/ir.rs` (905 lines)                                                                                           | `middle/ir/` + `verify.rs`                                                                                          | 04         |
| `middle/core/bytecode.rs` (2422 lines, 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization) | Merged into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules) | 06         |
| `backends/common/opcode.rs` (83-constant vocabulary)                                                                      | `middle/bytecode/opcode.rs` (eliminating the L3→L4 reverse)                                                         | 06         |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                 | Delete lines `416-830`, keep 340 lines                                                                              | 06         |
| `std/fs.rs`, `std/net.rs` etc. (native API bindings)                                                                      | Position unchanged                                                                                                  | —          |
| `repl/` (1152 lines, zero tests, no echo)                                                                                 | Keep or delete, see 06 section D                                                                                    | 06         |

**Directory renaming is not the goal.** Changes fall into two categories:

- **Internal structure that must change**: the **internal structure** of the three files
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (7165 lines including `include!`),
  and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: renaming.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`,
  demolishing `parser/ast.rs` into top-level `ast/` (the `Type` definition goes into `type_.rs`),
  the opcode vocabulary migrating to `middle/bytecode/`, all to be completed together with P5/P6
  (decision D1). **No phased rollout, no "optional" clauses.** Reason: directory names are the
  first-glance signal of responsibility — `middle/core/` holds three different things (`ir.rs`,
  `ir_gen.rs`, `bytecode.rs`), the name has already lost its meaning; AST is consumed by all layers,
  putting it under `frontend/` would mislead downstream paths. Boundaries are guaranteed by CI
  checks, but **the check rules reference the directory responsibility table, which must be
  consistent with the actual directory names**, otherwise `check-boundary.py` cannot determine
  whether code is in the right place.

### Naming Conventions

| Rule                                                                                   | Reason                                                                                                                                                           |
| -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| One concept has only one owning module                                                 | Prevents parallel definitions like "3 type representations"                                                                                                      |
| The submodule directory of `foo.rs` is named `foo/`                                    | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                              |
| Cross-layer references must go through the `pub` API, not through `pub(crate)` leakage | The **count** of `pub(crate)` leakage is a boundary-erosion metric, may only decrease not increase (this is a semantic boundary metric, not a line-count metric) |
| Internal implementations do not get `pub` unless they are cross-layer APIs             | Reduce the visibility surface                                                                                                                                    |
| Type file names like `type_.rs`, `fn_.rs` to avoid keywords                            | Starting from Rust 2024, `type` / `fn` are reserved words                                                                                                        |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                               | See 06 section D: expressing "new version" through parallel copies is a common source of dead code                                                               |

## Feature Routing Table

**Answers "I want to add X, where should I change it?"** This is the part of this document group
with the most long-term value — it will not be invalidated by any single refactor.

### Routing Table A: Language Features

| What to add                         | L2 Lex                                                         | L2 Parse                                                                                                                                                 | L3 Semantics                                                                                                                                                | L3 IR                                        | L4 Execution                                        | Gate / Doc                                                                                         |
| ----------------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| **Binary operator**                 | `lexer/state.rs` keyword table, add `TokenKind` to `tokens.rs` | `pratt/precedence.rs` adds a BP constant, `led.rs`'s `infix_info` adds an arm, `led.rs`'s `parse_binary` adds a mapping, `ast.rs` adds a `BinOp` variant | `infer_binary` in `inference/expressions.rs`                                                                                                                | corresponding lowering in `lower/`           | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                      |
| **Unary operator**                  | Same as above                                                  | `nud.rs`'s `prefix_info` adds an arm + new parse function + `ast.rs` adds a `UnOp` variant                                                               | `infer_unary`                                                                                                                                               | Same as above                                | Same as above                                       | RFC-010 table                                                                                      |
| **Constant-evaluation operator** ⚠️ | —                                                              | —                                                                                                                                                        | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` are a second parallel set of enums to `ast::BinOp` / `ast::UnOp`, with no `From`/`TryFrom` whatsoever** | `ir_gen.rs:2317` needs `use ast::BinOp as B` | —                                                   | **No gate** — see routing table C                                                                  |
| **Expression form**                 | —                                                              | Add an `Expr` variant to `ast.rs` + `nud.rs` / `led.rs`                                                                                                  | Add an infer method in `inference/expressions.rs`                                                                                                           | Add a module in `lower/exprs/`               | `opcode.rs` or reuse                                | —                                                                                                  |
| **Statement form**                  | —                                                              | Add a `StmtKind` variant to `ast.rs` + `statements/`                                                                                                     | `inference/statements.rs`                                                                                                                                   | `lower/`                                     | Same as above                                       | —                                                                                                  |
| **Type construction**               | —                                                              | `Type` in `ast.rs` (see 03)                                                                                                                              | `types/mono.rs` + `solver.rs`                                                                                                                               | `FunctionCode.params` type                   | Serialization                                       | RFC-011                                                                                            |
| **Pattern form**                    | —                                                              | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                          | `inference/patterns.rs`                                                                                                                                     | `compile_pattern` in `lower/`                | `opcode.rs`                                         | RFC-010b                                                                                           |
| **Diagnostic code**                 | —                                                              | —                                                                                                                                                        | —                                                                                                                                                           | —                                            | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (the `build.rs` gate auto-validates) |
| **Warnings (W prefix)**             | —                                                              | —                                                                                                                                                        | —                                                                                                                                                           | —                                            | —                                                   | Same as above; note they do not count toward the error count                                       |

> **Current pain point**: a single binary operator requires changing **6-7 places** (within
> parser/lexer) + **17 downstream production files** that mention `BinOp::`. Whether a missed change
> is detected falls into three layers:
>
> - **The compiler catches it**: places that perform an **exhaustive `match`** on `ast::BinOp` — a
>   missed change causes an immediate compile failure. Most cases are like this.
> - **The compiler cannot catch it (the real blind spot)**: the `matches!` macro, the `_` catch-all
>   arm, and **cross-enum** cases. The first two manifest as "the new operator is treated as some
>   other operator"; the third (`ast::BinOp` ↔ `const_data::BinOp`) is completely silent — both
>   types compile independently without issue.
> - **L4 does not participate**: `backends/interpreter/` has **zero references** to `BinOp` and
>   `ast::` — the interpreter never sees the AST, it only knows opcodes. So the downstream pressure
>   concentrates in L3, not L4.
>
> Goal: after grammar-driven transition, drop to 1-2 places + 1 auto-generated spot (see 05); the
> cross-enum part is solved by single-source generation (see 05); the third operator set in the
> bytecode layer (`BinaryOp` / `UnaryOp` / `CompareOp`) is consolidated by the opcode
> generation-time gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| What to add                                                | Landing point                                                                                                                                                                    | Places that need to change in sync                                                                                                                                                                                                                                                                                                           |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation stage**                                      | Add a `Stage` variant in `driver/stage.rs`                                                                                                                                       | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, the `dispatch` in `driver/mod.rs` (exhaustive match forces completeness), CI stage-coverage test                                                                                                                                                                   |
| **Cross-stage check layer** (e.g. ownership / termination) | Add a module in `sema/layers/`                                                                                                                                                   | Stage-table registration + update `layers/README.md` (**the README currently declares a layer order opposite to the actual execution order**) + outputs merged into `Obligations`                                                                                                                                                            |
| **opcode**                                                 | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`, migrating along with the bytecode-domain merger, all touch points consolidated into one `middle/` directory) | **5 places**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the `From<BytecodeFile>` decode match, and the `translate_*` in `translator.rs`. Among these, only 2 are forced by exhaustive match in the compiler, the other 3 silently fail and only blow up by running a `.42` artifact. A generation-time gate is required (see 06) |
| **CLI subcommand**                                         | The `Commands` enum in `main.rs`                                                                                                                                                 | The `pub` entry in `lib.rs` (currently 7), the `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                                            |
| **LSP capability**                                         | Add a handler in `lsp/handlers/`                                                                                                                                                 | `handle_request` in `lsp/server.rs` (currently 274 lines, 13 repeated boilerplate), `lsp/world.rs` state                                                                                                                                                                                                                                     |
| **Standard library API**                                   | `src/std/*.rs`                                                                                                                                                                   | **Automatic gate already in place**: `gen_interfaces.rs` byte-level comparison + `gen_docs.rs` range-drift detection. This is the most mature pattern in the whole project and should be generalized to the opcode table and type table                                                                                                      |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions to Eliminate

| Reverse dependency                                             | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Elimination method                                                                                                                                             | Belongs to |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| parser → typecheck (**only 2 occurrences, the same function**) | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Operator interface specifications are passed down to parser as data, parser no longer `use`s typecheck                                                         | 03         |
| Hardcoded predicate names inside parser                        | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. The comment at `:494-497` self-describes this as an open-set problem to be resolved: "Future open-set solution: pass the predicate names down to parser, or change to a form decidable at the syntax layer"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Same as above, predicate set passed down as data                                                                                                               | 03         |
| **Duplication created to evade cross-module dependencies**     | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` are **synonymous but independently implemented**; the comment at `ast.rs:845-846` self-admits: "Implemented independently here to avoid cross-module dependencies within parser"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Merged into a single implementation along with the probe data flow in 03                                                                                       | 03         |
| checker → internal functions of layers                         | `checker.rs:4857` / `4986` / `5246` / `5384` call `layers::termination::{negate_guard, substitute_const_expr}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Promoted to the public API of `sema/refine/` or `sema/layers/termination`                                                                                      | 02         |
| 3 parallel type representations                                | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (`ir.rs:3` re-exported via `pub use`) + serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Converged to a single representation                                                                                                                           | 03         |
| Hand-written synonym table                                     | `types/mono.rs:618-643` `from_builtin_name` maintains branches like `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Deleted along with the type-representation convergence                                                                                                         | 03         |
| **3 parallel operator enums**                                  | `ast.rs:191` `pub enum BinOp` (20 variants) / `ast.rs:217` `pub enum UnOp` (4, exclusive `Deref`); `types/const_data.rs:234` `pub enum BinOp` (18, uses `Ne` instead of `Neq`) / `:321` `pub enum UnOp` (4, exclusive `BitNot`); `middle/core/bytecode.rs:70` `BinaryOp` (11, `Rem` / `Xor` / `Sar` yet another naming) + `:97` `UnaryOp` + `:106` `CompareOp`. **There is no `From` / `TryFrom` between `ast` and `const_data`**, only the hand-written lossy free functions at `const_eval.rs:185-216` (returning `None` for `Range` / `Assign` / `BitNot` etc. silently drops them), forcing 5+ files to introduce aliases at import (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`), see `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, `middle/core/ir_gen.rs:2317` | The two L2 sets are converged by single-source generation (see 05); the one bytecode-layer set is cross-checked by the opcode generation-time gate (see 06 F8) | 03 / 05    |
| `include!`                                                     | `checker.rs:5618`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Becomes a real `mod`                                                                                                                                           | 02         |

## Future Extension Guide

### Checklist When Adding a Language Feature

1. **Look up routing table A first** to determine the landing point. Don't go by gut feel searching
   for files.
2. **Types first**: if the feature involves types, follow the single representation in 03, do not
   add a third set.
3. **Gate in sync**: diagnostic-code changes must simultaneously modify `codes/eNxxx.rs`,
   `locales/zh.json`, and **the RFC-013 code table** — `build.rs` will force all three to be
   consistent.
4. **Stage registration**: if the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Reverse-dependency check**: new code must not make L2 depend on L3, or L4 depend on L1/L2.
6. **Test wiring**: new test directories must simultaneously add a `mod` declaration in the parent
   module, otherwise they won't run — this will be intercepted by a newly added CI check.

### When Adding a "Layer" (e.g. a Future New Static Check)

1. Create the module in `sema/layers/`, **do not depend on layers with a larger index**.
2. The outputs are merged into `Obligations` and consumed by the orchestration layer in a unified
   way.
3. Update `sema/layers/README.md` — **let it describe the actual layer order**, not the intended
   one. The current file describes an architecture that was never built.
4. Register in the `Stage` enum, and explicitly declare in which modes it runs in
   `Program::stages()`.

---

## Where the Maintenance Mechanism Lives

The anti-rebound gate (CI checklist, decision procedure, three prohibitions) has been moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md). This document only answers "where things
go": dependency direction specification, target directory structure, feature routing table, naming
conventions.
