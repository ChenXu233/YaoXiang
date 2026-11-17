# Feature Routing and Dependency Guidelines

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). It
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing table, and anti-regression mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution phases, and acceptance criteria are covered in
> the main RFC-039 text. Detailed layer designs are covered by the other documents in this
> directory.

## Dependency Direction Guidelines

The following rules are enforced by CI checks (see [anti-regression mechanism](#防反弹机制)):

| Rule                                                                | How it is checked                                                                                                   | Current state                                                                                                                                                  |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `include!` is prohibited                                            | `grep -rn 'include!' src/` should return 0                                                                          | 0 occurrences (eliminated in 5.1: `checker.rs` now uses a real `mod`)                                                                                          |
| L2 must not `use` L3                                                | Scan for `use ...typecheck` in `src/frontend/core/{lexer,parser}/`                                                  | 2 occurrences exist and must be removed (see Routing Table C)                                                                                                  |
| L3/L4 must not `use` L2                                             | Scan for `use ...frontend::{lexer,parser,module}` in `sema/`, `proof/`, `middle/`, and `backends/`                  | Currently 0; baseline is set to 0 (AST is a top-level domain and requires no exemption clause)                                                                 |
| L4 must not reference L1/L2 in reverse                              | Scan for `use ...frontend::core::{lexer,parser}` in `src/backends/` and `src/middle/`                               | A baseline must first be established                                                                                                                           |
| The opcode vocabulary has a single owner: `middle/bytecode/`        | Scan `src/backends/` for opcode constant definitions (imports only are allowed); scan `sema/` for opcode references | The vocabulary currently resides in `backends/common/opcode.rs` (creating an L3→L4 reverse dependency); migrate it together with bytecode-domain consolidation |
| Cross-layer `pub(crate)` leaks may decrease but must never increase | Count them and include the counts in the baseline file                                                              | Cross-layer leaks still exist; the current count is generated and reported by `scripts/ci/check-boundary.py` (total + per-directory)                           |
| Hard-coded type/predicate names are prohibited                      | Scan for type-name string literals in `src/frontend/core/parser/`                                                   | `"Terminates"` and similar                                                                                                                                     |

## Target Directory Structure (After All Construction Phases)

The following is the state **after all construction phases are complete**, not the target of the
first step. The gap between the current and target forms is shown in the mapping table below.

```
src/
├── driver/                        # L1 orchestration: the sole decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustively matchable) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-phase diagnostic aggregation + exit-code determination
│
├── frontend/                      # L2 lexing and parsing
│   ├── lexer/                     #   Declarative: numbers unified as scan_radix(base),
│   │   ├── mod.rs                 #     Sole implementation of escape decoding (merge the three copies currently in literals.rs)
│   │   ├── number.rs              #     Replaces four identical radix scanners
│   │   ├── escape.rs              #     Sole escape decoder
│   │   ├── string.rs              #     Strings / multiline strings
│   │   ├── fstring.rs             #     f-string; interpolation advances toward the parser instead of nesting tokenize
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: a single precedence table, no bare magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     Grammar definitions (added after the grammar-driven implementation)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder node
│   └── module/                    #   Multi-file: discovery / registry / roles
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merge approximately 60 lines of near-duplicate code)
│       └── discover.rs            #     Discovery strategy following use references
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
│   │   ├── universe.rs            #     UniverseLevel (currently inside mono.rs)
│   │   ├── infer.rs               #     Inference entry point
│   │   ├── solver.rs              #     unify / substitute
│   │   └── eval/                  #     Constant evaluation (including the sole const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     Sole ConstExpr; sole source of operators (merge the current two sets)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   Expression / statement inference
│   │   ├── mod.rs                 #     Location of StatementChecker
│   │   ├── call/                  #     Split infer_call_expr (see split axes in 03/04)
│   │   │   ├── mod.rs             #       Dispatch by "callee form"
│   │   │   ├── construct.rs       #       Homogeneous type construction / generic type construction
│   │   │   ├── generic.rs         #       Explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       Overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (sole three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level checks (after splitting checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     Annotation type-name validation + is_predicate_head (an actual submodule of the current checker)
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     Imports/exports / unused imports
│   │   └── semantic_tokens.rs     #     Semantic token collection (an actual submodule of the current checker)
│   ├── refine/                    #   Refined types (currently checker/refinement.rs, a single 1107-line file)
│   │   ├── mod.rs                 #     collect_refined_binding_checks
│   │   ├── binding.rs             #     revalidate_refined
│   │   ├── call_arg.rs            #     check_call_arg_refinements
│   │   ├── return_.rs             #     check_return_refinement
│   │   └── walk.rs                #     refined_walk_* + RefinedWalkCtx
│   └── layers/                    #   Cross-cutting analyses
│       ├── README.md              #     Must describe the actual layer order, not the intended layer order
│       ├── ownership.rs
│       ├── termination.rs
│       ├── predicate.rs
│       └── equivalence.rs
│
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to top level; the driver obligation ledger holds proof_calls; the three sema layers share the SMT backend, so it is promoted for cross-layer consumption)
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
│   │   ├── operand.rs             #     Operand (dead Temp/Label/Register variants removed)
│   │   ├── instr.rs               #     Instruction (including Phi)
│   │   ├── block.rs               #     BasicBlock
│   │   ├── fn_.rs                 #     FunctionIR / LocalSlot
│   │   └── verify.rs              #     verify_loose / verify_ssa
│   ├── lower/                     #   AST → IR (after splitting ir_gen.rs)
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
│   │   └── synth.rs               #     Sole place allowed to construct ast::Expr
│   ├── passes/                    #   Passes on IR
│   │   ├── mono/                  #     Monomorphization (remove 415 lines of closure remnants)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 stack slots (no real register file)
│   └── bytecode/                  #   Bytecode domain: opcode vocabulary + containers + encoding/decoding
│       ├── opcode.rs              #     Sole opcode vocabulary (+ generation-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file to facilitate differential testing)
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
│   │   ├── heap.rs                #   Shared within L4: heap layout
│   │   └── value.rs               #   Shared within L4: runtime value
│   ├── interpreter/               #   Interpreter (sole Executor implementation)
│   │   ├── executor.rs
│   │   ├── debug.rs               #   Opcode dispatch table
│   │   ├── ffi.rs
│   │   └── ops/                   #   Split by instruction family
│   └── runtime/                   #   DAG task scheduler (not a second execution backend)
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 standard library (Rust implementation + .yx mixture)
├── util/                          # Cross-layer utilities (diagnostics, span, cache, i18n; retain the existing name—the Go guidance to "avoid util" referenced in 08 is only moderately applicable, and renaming would create net churn without domain benefit, so it is recorded as an exception)
├── formatter/                     # Formatting
├── lsp/                           # Language server
├── repl/                          # REPL (if retained; see Section D of 06)
└── package/                       # Package management
```

### Responsibilities and Boundary Invariants for Each Directory

| Directory                                 | Responsibility                                                                         | Invariants                                                                                                                                                          |
| ----------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Determines which phases to run, how many times to run them, and how failures propagate | **Must not** be reverse-depended on by L2/L3/L4; the phase set must be an exhaustively enumerable set                                                               |
| `frontend/lexer`, `frontend/parser`       | Lexing and parsing; construct `ast/` nodes                                             | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics must all be passed down as data |
| `ast/`                                    | The sole AST and type representation (top-level shared data domain, peer to `parser`)  | **Must not** contain a second definition of the same concept; does not `use` other layers (pure data and pure functions only)                                       |
| `frontend/module/`                        | Multi-file discovery, registry, and roles                                              | Performs discovery and registration only; **must not** run analysis phases; the `driver` schedules analysis                                                         |
| `sema/`                                   | Type checking and static analysis                                                      | **Must not** construct IR; has no awareness of opcodes                                                                                                              |
| `proof/`                                  | Proof result types and SMT backend                                                     | The Solver trait is the sole backend abstraction point; **must not** panic through `.expect()`                                                                      |
| `middle/ir/`                              | IR definitions and invariants                                                          | The IR obeys SSA discipline; a passing `verify_ssa` is a prerequisite for merge                                                                                     |
| `middle/lower/`                           | AST → IR                                                                               | The **only** place allowed to construct `ast::Expr` is `synth.rs`                                                                                                   |
| `middle/codegen/`                         | IR → bytecode lowering                                                                 | Depends only on the IR, not on `sema`; does not define the opcode vocabulary                                                                                        |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + encoding/decoding               | **Sole ownership of the opcode vocabulary**; `backends/` only consumes it and never defines it; `sema` has no awareness of opcodes                                  |
| `backends/`                               | Execution                                                                              | **Must not** reference L1/L2 in reverse; has no awareness of the AST                                                                                                |
| `middle/passes/mono/`                     | Passes on the IR                                                                       | Must not reference `frontend`                                                                                                                                       |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tooling layers (outside the L1-L4 model)                                    | Consume only the `pub` APIs of L1-L4; must not touch `pub(crate)` (the same leak-count constraint applies to them)                                                  |

### Current State → Target Mapping

| Current state                                                                                                                                                                  | Target                                                                                                                                                                                                                                                                                                | Owning document |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `frontend/pipeline.rs` (740 lines, hand-written 5-stage if chain)                                                                                                              | `driver/` + `Program::SingleFile`                                                                                                                                                                                                                                                                     | 02              |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entry points, 60% branching)                                                                                                  | Leave only discovery/registration in `frontend/module/`; move compilation logic to `driver`                                                                                                                                                                                                           | 02              |
| `frontend/compiler.rs` (267 lines)                                                                                                                                             | Reduce to a thin wrapper around `driver`                                                                                                                                                                                                                                                              | 02              |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                                                                           | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                                                                                                                                                                                                    | 05              |
| `frontend/core/parser/pratt/` (hand-written match dispatch, 2 sets of BP ladders)                                                                                              | `frontend/parser/grammar/` + a single precedence table                                                                                                                                                                                                                                                | 05              |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                                                                     | Split into top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                                                                                                                                                                                                         | 03 / D1         |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8-10 responsibility sections)                                                                                   | Split `frontend/parser/` + remove 120 lines of dead legacy syntax                                                                                                                                                                                                                                     | 05              |
| `typecheck/checker.rs` (4167 lines + three actual submodules: `checker/annotations.rs` 470 lines, `checker/refinement.rs` 1107 lines, `checker/semantic_tokens.rs` 1553 lines) | `sema/check/` + `sema/refine/`; change `include!` to a real `mod` (completed in 5.1), move the refinement block into a submodule and consolidate its entry point (completed in 5.2: entry point uses `&self` and returns diagnostics), move the annotation family into a submodule (completed in 5.3) | 02 / 05         |
| `typecheck/checker/semantic_tokens.rs` (1553 lines, already an actual submodule of `checker`)                                                                                  | `sema/check/semantic_tokens.rs`                                                                                                                                                                                                                                                                       | 02              |
| `typecheck/inference/expressions.rs` (originally a single file)                                                                                                                | `sema/inference/expressions/{mod,expr}.rs` + `expressions/call/{mod,args,instantiation,method}.rs` (split in 7.0.4; merging `statements.rs` and renaming `inference/` are separate matters)                                                                                                           | 05 / 04         |
| `typecheck/inference/statements.rs` (2887 lines)                                                                                                                               | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                                                                                                                                                                                                            | 05              |
| `typecheck/types/mono.rs` (1077 lines, including a synonym table)                                                                                                              | `sema/types/{repr,universe}.rs`                                                                                                                                                                                                                                                                       | 03              |
| `typecheck/types/const_data.rs` (second operator enum set)                                                                                                                     | Unify with the operators of `ast::Type`                                                                                                                                                                                                                                                               | 03              |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                                                                                   | Delete                                                                                                                                                                                                                                                                                                | 06              |
| `typecheck/layers/dispatch.rs` (entire chain has zero calls)                                                                                                                   | Delete                                                                                                                                                                                                                                                                                                | 06              |
| `typecheck/layers/`                                                                                                                                                            | `sema/layers/` + change `README.md` to describe the actual layer order                                                                                                                                                                                                                                | 06              |
| `typecheck/proof/`                                                                                                                                                             | Promote to top-level `proof/`                                                                                                                                                                                                                                                                         | 02              |
| `middle/core/ir_gen.rs` (originally a single file with zero tests)                                                                                                             | `middle/lower/{mod,scope,query,stmt,loops,globals,module_ir,const_eval,curry}.rs` + `lower/expr/{mod,call,access,control,container,lit,op,reference,closure,spawn}.rs` (split in 7.0.1–7.0.3)                                                                                                         | 04              |
| `middle/core/ir.rs` (905 lines)                                                                                                                                                | `middle/ir/` + `verify.rs`                                                                                                                                                                                                                                                                            | 04              |
| `middle/core/bytecode.rs` (2422 lines, including a 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization)                                          | Consolidate into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules)                                                                                                                                                                              | 06              |
| `backends/common/opcode.rs` (83-constant vocabulary)                                                                                                                           | `middle/bytecode/opcode.rs` (eliminate the L3→L4 reverse dependency)                                                                                                                                                                                                                                  | 06              |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                                                                      | Delete `416-830`; retain 340 lines                                                                                                                                                                                                                                                                    | 06              |
| `std/fs.rs`, `std/net.rs`, and similar native API bindings                                                                                                                     | Keep in place                                                                                                                                                                                                                                                                                         | —               |
| `repl/` (1152 lines, zero tests, no echo)                                                                                                                                      | Retain or delete; see Section D of 06                                                                                                                                                                                                                                                                 | 06              |

**Directory renaming is not the goal.** Changes fall into two categories:

- **Internal structures that must change:** the **internal structures** of `middle/core/ir_gen.rs`
  (8448 lines), `typecheck/checker.rs` (4167 lines + actual submodules of 470 / 1107 / 1553 lines),
  and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: renaming.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`, split
  `parser/ast.rs` into top-level `ast/` (with the `Type` definition moved into `type_.rs`), and
  migrate the opcode vocabulary to `middle/bytecode/`, all completed together with P5/P6 (Decision
  D1). **There is no phased rollout and no “optional” option.** The rationale is that a directory
  name is the first signal of responsibility: `middle/core/` currently contains three unrelated
  things—`ir.rs`, `ir_gen.rs`, and `bytecode.rs`—so its name is no longer valid. The AST is consumed
  by every layer, and placing it under `frontend/` would give downstream code a misleading path. CI
  checks enforce the boundaries, but **the check rules reference the directory responsibility table,
  which must match the actual directory names**; otherwise, `check-boundary.py` cannot determine
  whether code is in the correct location.

### Naming Conventions

| Rule                                                                            | Rationale                                                                                                                                                          |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A concept has exactly one owning module                                         | Prevents parallel definitions such as “3 type representations”                                                                                                     |
| The submodule directory of `foo.rs` is named `foo`                              | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                |
| Cross-layer references must use `pub` APIs, not leak through `pub(crate)`       | The **count** of `pub(crate)` leaks is a boundary-erosion metric and may only decrease, never increase (it is a semantic boundary metric, not a line-count metric) |
| Do not add `pub` to internal implementations unless they form a cross-layer API | Reduces the visible API surface                                                                                                                                    |
| Type file names use `type_.rs` and `fn_.rs` to avoid keywords                   | `type` and `fn` are reserved keywords starting with Rust 2024                                                                                                      |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                        | See Section D of 06: using parallel copies to express a “new version” is a common source of dead code                                                              |

## Feature Routing Tables

**These answer the question, “I want to add X; where should I change it?”** This is the most
enduringly valuable part of this document set—it does not become obsolete with any single
refactoring.

### Routing Table A: Language Features

| What to add                          | L2 Lexing                                                        | L2 Syntax                                                                                                                                                        | L3 Semantics                                                                                                                                                         | L3 IR                                        | L4 Execution                                        | Gates/Documentation                                                                                              |
| ------------------------------------ | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **Binary operator**                  | Keyword list in `lexer/state.rs`; add `TokenKind` to `tokens.rs` | Add a BP constant to `pratt/precedence.rs`; add an arm to `led.rs`'s `infix_info`; add a mapping to `led.rs`'s `parse_binary`; add a `BinOp` variant to `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                                         | Corresponding lowering in `lower/`           | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                                    |
| **Unary operator**                   | Same as above                                                    | Add an arm to `nud.rs`'s `prefix_info` + a new parse function + a `UnOp` variant to `ast.rs`                                                                     | `infer_unary`                                                                                                                                                        | Same as above                                | Same as above                                       | RFC-010 table                                                                                                    |
| **Constant-evaluation operators** ⚠️ | —                                                                | —                                                                                                                                                                | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` are a second set of enums parallel to `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` conversion whatsoever** | `ir_gen.rs:2317` needs `use ast::BinOp as B` | —                                                   | **No gate**—see Routing Table C                                                                                  |
| **Expression form**                  | —                                                                | Add an `Expr` variant to `ast.rs` + `nud.rs`/`led.rs`                                                                                                            | Add an infer method to `inference/expressions.rs`                                                                                                                    | Add a module under `lower/exprs/`            | `opcode.rs` or reuse                                | —                                                                                                                |
| **Statement form**                   | —                                                                | Add a `StmtKind` variant to `ast.rs` + `statements/`                                                                                                             | `inference/statements.rs`                                                                                                                                            | `lower/`                                     | Same as above                                       | —                                                                                                                |
| **Type construction**                | —                                                                | `Type` in `ast.rs` (see 03)                                                                                                                                      | `types/mono.rs` + `solver.rs`                                                                                                                                        | `FunctionCode.params` type                   | Serialization                                       | RFC-011                                                                                                          |
| **Pattern form**                     | —                                                                | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                                  | `inference/patterns.rs`                                                                                                                                              | `compile_pattern` in `lower/`                | `opcode.rs`                                         | RFC-010b                                                                                                         |
| **Diagnostic code**                  | —                                                                | —                                                                                                                                                                | —                                                                                                                                                                    | —                                            | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (the `build.rs` gate validates them automatically) |
| **Warning (W prefix)**               | —                                                                | —                                                                                                                                                                | —                                                                                                                                                                    | —                                            | —                                                   | Same as above; note that it does not count toward the error total                                                |

> **Current pain point:** adding a binary operator requires changes in **6-7 places** within
> parser/lexer, plus the **17 downstream production files** that use `BinOp::`. Whether a missed
> change is detected falls into three categories:
>
> - **Caught by the compiler:** places that perform an **exhaustive `match`** on `ast::BinOp` fail
>   to compile if a change is missed. These make up the majority.
> - **Not caught by the compiler (the real blind spots):** the `matches!` macro, `_` fallback arms,
>   and cross-enum cases. A missed change in the first two categories causes the “new operator” to
>   be treated as another operator. A missed change in the third category (`ast::BinOp` ↔
>   `const_data::BinOp`) is completely silent—the two types compile independently.
> - **L4 does not participate:** `backends/interpreter/` has zero references to `BinOp` or `ast::`.
>   The interpreter never sees the AST; it recognizes only opcodes. Downstream pressure is therefore
>   concentrated in L3 rather than L4.
>
> Goal: after the grammar-driven implementation, reduce this to 1-2 places + 1 automatically
> generated location (see 05). Resolve the cross-enum portion through common-source generation (see
> 05). Consolidate the third operator enum set in the bytecode layer
> (`BinaryOp`/`UnaryOp`/`CompareOp`) through an opcode generation-time gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| What to add                                                      | Location                                                                                                                                                                                 | Places that must be updated in sync                                                                                                                                                                                                                                                                                          |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation phase**                                            | Add a `Stage` variant to `driver/stage.rs`                                                                                                                                               | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, `dispatch` in `driver/mod.rs` (the exhaustive match forces completion), and CI phase-coverage tests                                                                                                                                                |
| **Cross-stage checking layer** (such as ownership / termination) | Add a module under `sema/layers/`                                                                                                                                                        | Register it in the phase table + update `layers/README.md` (**the call-order table must correspond one-for-one with the call sites in `checker.rs`**) + fold its outputs into `Obligations`                                                                                                                                  |
| **Opcode**                                                       | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`; migrate it together with bytecode-domain consolidation and consolidate all touchpoints into one `middle/` directory) | **5 locations**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the `From<BytecodeFile>` decode match, and `translate_*` in `translator.rs`. Only 2 are enforced by compiler exhaustive matches; a missed change in the other 3 fails only when running a `.42` artifact. A generation-time gate is required (see 06) |
| **CLI subcommand**                                               | `Commands` enum in `main.rs`                                                                                                                                                             | `pub` entry points in `lib.rs` (currently 7), `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                             |
| **LSP capability**                                               | Add a handler under `lsp/handlers/`                                                                                                                                                      | `handle_request` in `lsp/server.rs` (currently 274 lines with 13 instances of repeated boilerplate), state in `lsp/world.rs`                                                                                                                                                                                                 |
| **Standard-library API**                                         | `src/std/*.rs`                                                                                                                                                                           | **An automated gate already exists**: byte-for-byte comparison by `gen_interfaces.rs` + marked-region drift detection by `gen_docs.rs`. This is the project's most mature pattern and should be extended to the opcode and type tables                                                                                       |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions That Must Be Eliminated

| Reverse dependency                                                      | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | How to eliminate it                                                                                                                                                 | Owning document |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| parser → typecheck (**only 2 occurrences, in the same function**)       | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both calling `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Pass operator-interface specifications down to the parser as data; the parser no longer `use`s typecheck                                                            | 03              |
| Hard-coded predicate names in the parser                                | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. The comment at `:494-497` identifies this as an unresolved open-set issue: 「未来的开集方案：把谓词名下传给 parser，或改成语法层可判定的形态」                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | As above; pass the predicate set down as data                                                                                                                       | 03              |
| **Duplicate implementation created to avoid a cross-module dependency** | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` are equivalent but independent implementations. The comment at `ast.rs:845-846` acknowledges: 「此处独立实现以避免 parser 内部跨模块依赖」                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Merge them into a single implementation through 03's probe data flow                                                                                                | 03              |
| checker → internal layers functions                                     | The walk family in `checker/refinement.rs`, together with `revalidate_refined` / `check_return_refinement`, calls `layers::termination::{negate_guard, substitute_const_expr}` (formerly `checker.rs:4857/4986/5246/5384`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Promote them to the public API of `sema/refine/` or `sema/layers/termination`                                                                                       | 02              |
| 3 parallel type representations                                         | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (`ir.rs:3`, re-exported with `pub use`) + serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Consolidate into a single representation                                                                                                                            | 03              |
| Hand-written synonym table                                              | `types/mono.rs:618-643`: `from_builtin_name` maintains branches such as `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Remove it as the type representations are consolidated                                                                                                              | 03              |
| **3 parallel operator enum sets**                                       | `ast.rs:191` `pub enum BinOp` (20 variants) / `ast.rs:217` `pub enum UnOp` (4, uniquely including `Deref`); `types/const_data.rs:234` `pub enum BinOp` (18, using `Ne` instead of `Neq`) / `:321` `pub enum UnOp` (4, uniquely including `BitNot`); `middle/core/bytecode.rs:70` `BinaryOp` (11, introducing yet another naming scheme with `Rem`/`Xor`/`Sar`) + `:97` `UnaryOp` + `:106` `CompareOp`. **There is no `From` / `TryFrom` whatsoever between ast and const_data**; the only bridge is the handwritten lossy free function in `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot`, etc. return `None` and are silently discarded). This forces 5+ files to alias imports (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`); see `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, and `middle/core/ir_gen.rs:2317` | Consolidate the two L2 sets through common-source generation (see 05); compare and consolidate the bytecode set through the opcode generation-time gate (see 06 F8) | 03 / 05         |
| `include!`                                                              | Already eliminated (5.1: `checker.rs` now uses a real `mod semantic_tokens`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Complete                                                                                                                                                            | —               |

## Guidance for Future Extensions

### Checklist When Adding a Language Feature

1. **Consult Routing Table A first** and determine the target location. Do not choose files by
   intuition.
2. **Types first:** if the feature involves types, follow 03's single representation and do not
   introduce a third representation.
3. **Synchronize the gates:** diagnostic-code changes must simultaneously update `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table**—`build.rs` enforces consistency among all
   three.
4. **Register the phase:** if the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Check reverse dependencies:** new code must not make L2 depend on L3 or make L4 depend on
   L1/L2.
6. **Wire up tests:** a new test directory must also have a `mod` declaration in its parent module;
   otherwise, it will not run. The newly added CI check will block this.

### When Adding a “Layer” (Such as a Future Static Check)

1. Create a module under `sema/layers/`; it **must not depend on any layer with a higher number**.
2. Put its outputs in `Obligations`, where the orchestration layer consumes them uniformly.
3. Update `sema/layers/README.md` so that it **describes the actual layer order** (use line numbers
   from the current tree, with locations anchored to function names or driver points).
4. Register it in the `Stage` enum and explicitly declare in `Program::stages()` which modes run it.

---

## Where to Find the Maintenance Mechanism

The anti-regression gate (CI checklist, decision procedure, and three prohibitions) has moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md).

This document answers only “where things go”: dependency direction guidelines, target directory
structure, feature routing tables, and naming conventions.
