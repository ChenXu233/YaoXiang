# Feature Routing and Dependency Specification

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) and
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing table, and anti-regression mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution phases, and acceptance criteria are covered in
> the main RFC-039 text. Detailed layer designs are documented in the remaining documents in this
> directory.

## Dependency Direction Specification

The following rules are enforced by CI checks (see [Anti-Regression Mechanism](#防反弹机制)):

| Rule                                                             | Check Method                                                                                                              | Current Status                                                                                                                                               |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `include!` is prohibited                                         | `grep -rn 'include!' src/` must return 0                                                                                  | 0 occurrences (eliminated in 5.1: `checker.rs` was changed to a real `mod`)                                                                                  |
| L2 must not `use` L3                                             | Scan `use ...typecheck` in `src/frontend/core/{lexer,parser}/`                                                            | 2 occurrences exist and must be removed (see Routing Table C)                                                                                                |
| L3/L4 must not `use` L2                                          | Scan `use ...frontend::{lexer,parser,module}` in `sema/`, `proof/`, `middle/`, and `backends/`                            | Currently 0; the baseline is 0 (the AST is a top-level domain, so no exemption clause is needed)                                                             |
| L4 must not reference L1/L2 in reverse                           | Scan `use ...frontend::core::{lexer,parser}` in `src/backends/` and `src/middle/`                                         | A baseline must first be established                                                                                                                         |
| The opcode vocabulary has a single owner: `middle/bytecode/`     | Scan `src/backends/` to ensure it only imports, rather than defines, opcode constants; scan `sema/` for opcode references | The current vocabulary is in `backends/common/opcode.rs` (forming an L3→L4 reverse dependency); it will be migrated when the bytecode domain is consolidated |
| Cross-layer `pub(crate)` leaks may only decrease, never increase | Count them and include the results in the baseline files                                                                  | Cross-layer leaks still exist; the current count is tracked and reported by `scripts/ci/check-boundary.py` (total count + per-directory breakdown)           |
| Hard-coded type/predicate names are prohibited                   | Scan for type-name string literals in `src/frontend/core/parser/`                                                         | `"Terminates"`, etc.                                                                                                                                         |

## Target Directory Structure (After All Implementation Phases Are Complete)

The following is the final form **after all implementation phases are complete**, not the target of
the first step. The gap between the current and target forms is shown in the mapping table below.

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
│   │   ├── mod.rs                 #     sole escape-decoding implementation (merges the 3 copies in literals.rs)
│   │   ├── number.rs              #     replaces 4 identical line-by-line radix scanners
│   │   ├── escape.rs              #     sole escape decoder
│   │   ├── string.rs              #     strings / multiline strings
│   │   ├── fstring.rs             #     f-strings; interpolation forwarded to parser, with no nested tokenization
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: one precedence table, no naked magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     grammar definitions (added after the grammar-driven migration)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder nodes
│   └── module/                    #   Multifile: discovery / registry / roles
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merges the current 60 near-duplicate lines)
│       └── discover.rs            #     discovery strategy that follows use directives
│
├── ast/                           # Sole AST + sole type carrier (top-level domain, peer to parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after consolidation; see 03 for handling dead variants)
│   ├── stmt.rs                    #   StmtKind (removes signature_params)
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
│   │   └── eval/                  #     constant evaluation (including the sole const_data)
│   │       ├── mod.rs
│   │       ├── const_data.rs      #     sole ConstExpr; sole source of operators (merges the current 2 sets)
│   │       ├── eval.rs
│   │       └── dependent.rs
│   ├── infer/                     #   Expression / statement inference
│   │   ├── mod.rs                 #     location of StatementChecker
│   │   ├── call/                  #     split infer_call_expr (see the splitting axis in 03/04)
│   │   │   ├── mod.rs             #       dispatch by "callee shape"
│   │   │   ├── construct.rs       #       type construction / generic type construction
│   │   │   ├── generic.rs         #       explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       overloads / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (sole three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level checks (after splitting checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     annotation type-name validation + is_predicate_head (now a real checker submodule)
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     imports/exports / unused imports
│   │   └── semantic_tokens.rs     #     semantic token collection (now a real checker submodule)
│   ├── refine/                    #   Refinement types (currently checker/refinement.rs, 1107 lines)
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
├── proof/                         # L3: proofs and verification support (promoted from typecheck/proof/ to top level; the driver obligation ledger holds proof_calls; the three sema layers share the SMT backend, so it is promoted because it is consumed across layers)
│   ├── mod.rs                     #   ProofResult / ProofContext / ProofFunctionCall
│   ├── context.rs
│   ├── verdict.rs
│   ├── dep_graph.rs
│   └── smt/                       #   Solver trait + Z3 backend + gates
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
│   │   ├── scope.rs               #     scopes and register frames (RAII)
│   │   ├── module_ir.rs
│   │   ├── const_eval.rs
│   │   ├── globals.rs
│   │   ├── stmt.rs
│   │   ├── curry.rs
│   │   ├── loops.rs
│   │   ├── query.rs               #     pure query utilities
│   │   ├── expr/                  #     expression lowering
│   │   └── synth.rs               #     sole place permitted to construct ast::Expr / ast::Stmt / ast::Type
│   ├── passes/                    #   Passes over IR
│   │   ├── mono/                  #     monomorphization (removes the 415 lines of closure remnants)
│   │   ├── dce/                   #     post-mono dead-code elimination (root-set mark-sweep)
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
│       ├── instr_meta.rs          #     opcode() + size() (same file to facilitate differential testing)
│       ├── decode/                #     decoding (split from the current 1400-line From implementation)
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
├── util/                          # Cross-layer utilities (diagnostics, spans, caches, i18n; retain existing names—the Go "avoid util" guidance cited in 08 is only moderately applicable, and renaming would cause net churn without domain benefit; recorded as an exception)
├── formatter/                     # Formatting
├── lsp/                           # Language services
├── repl/                          # REPL (if retained, see Section D of 06)
└── package/                       # Package management
```

### Responsibilities and Boundary Invariants for Each Directory

| Directory                                 | Responsibility                                                                            | Invariant                                                                                                                                                                               |
| ----------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Decide which stages to run, how many times to run them, and how failures propagate        | It **must not** be depended on in reverse by L2/L3/L4; the set of stages must be an exhaustively matchable enum                                                                         |
| `frontend/lexer`, `frontend/parser`       | Lexing and parsing, constructing `ast/` nodes                                             | They **must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics must all be passed down as data                |
| `ast/`                                    | The sole AST and type representation (a top-level shared data domain, peer to the parser) | It **must not** contain a second definition of the same concept; it does not `use` other layers (pure data and pure functions only)                                                     |
| `frontend/module/`                        | Multifile discovery, registry, and roles                                                  | It performs discovery and registration only and **must not run analysis phases**; the `driver` schedules analyses                                                                       |
| `sema/`                                   | Type checking and static analysis                                                         | It **must not** construct IR and must not be aware of opcodes                                                                                                                           |
| `proof/`                                  | Proof result types and SMT backends                                                       | The Solver trait is the sole backend abstraction point; it **must not** panic via `.expect()`                                                                                           |
| `middle/ir/`                              | IR definitions and invariants                                                             | The IR satisfies SSA discipline; a green `verify_ssa` is a prerequisite for merging                                                                                                     |
| `middle/lower/`                           | AST → IR                                                                                  | The **only** place where L3/L4 production code may construct `ast::Expr` / `ast::Stmt` / `ast::Type` is `synth.rs`; pattern matching and read-only queries do not count as construction |
| `middle/codegen/`                         | IR → bytecode lowering                                                                    | It depends only on IR, not `sema`, and does not define the opcode vocabulary                                                                                                            |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + encoding/decoding                  | The **opcode vocabulary has a single owner**; `backends/` only consumes it and never defines it; `sema` is not aware of opcodes                                                         |
| `backends/`                               | Execution                                                                                 | It **must not** reference L1/L2 in reverse and must not be aware of the AST                                                                                                             |
| `middle/passes/mono/`                     | Passes over IR                                                                            | It must not reference `frontend`                                                                                                                                                        |
| `middle/passes/dce/`                      | Passes over IR (post-mono reachability removal)                                           | It must not reference `frontend`; root-set and removal criteria have a single source in this module                                                                                     |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tooling layers (outside the L1-L4 model)                                       | They consume only the `pub` APIs of L1-L4 and must not touch `pub(crate)` (the leak-count rule applies to them as well)                                                                 |

### Current → Target Mapping

| Current                                                                                                                                                                          | Target                                                                                                                                                                                                                                                                                                | Assigned To |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| `frontend/pipeline.rs` (740 lines, handwritten five-stage if chain)                                                                                                              | `driver/` + `Program::SingleFile`                                                                                                                                                                                                                                                                     | 02          |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entry points, 60% branching)                                                                                                    | `frontend/module/` retains only discovery/registration; compilation logic moves to `driver`                                                                                                                                                                                                           | 02          |
| `frontend/compiler.rs` (267 lines)                                                                                                                                               | Reduced to a thin wrapper around `driver`                                                                                                                                                                                                                                                             | 02          |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                                                                             | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                                                                                                                                                                                                    | 05          |
| `frontend/core/parser/pratt/` (handwritten match dispatch, two BP ladders)                                                                                                       | `frontend/parser/grammar/` + one precedence table                                                                                                                                                                                                                                                     | 05          |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                                                                       | Split into the top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                                                                                                                                                                                                     | 03 / D1     |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8-10 responsibility segments)                                                                                     | Split `frontend/parser/` + remove 120 lines of dead legacy syntax                                                                                                                                                                                                                                     | 05          |
| `typecheck/checker.rs` (4167 lines + three real submodules: `checker/annotations.rs` 470 lines, `checker/refinement.rs` 1107 lines, and `checker/semantic_tokens.rs` 1553 lines) | `sema/check/` + `sema/refine/`; change `include!` to a real `mod` (completed in 5.1), migrate refinement blocks into submodules and consolidate entry points (completed in 5.2: entry point uses `&self` and returns diagnostics), and migrate annotation families into submodules (completed in 5.3) | 02 / 05     |
| `typecheck/checker/semantic_tokens.rs` (1553 lines, already a real `checker` submodule)                                                                                          | `sema/check/semantic_tokens.rs`                                                                                                                                                                                                                                                                       | 02          |
| `typecheck/inference/expressions.rs` (originally a single file)                                                                                                                  | `sema/inference/expressions/{mod,expr}.rs` + `expressions/call/{mod,args,instantiation,method}.rs` (split in 7.0.4; consolidation into `statements.rs` and renaming `inference/` are handled separately)                                                                                              | 05 / 04     |
| `typecheck/inference/statements.rs` (2887 lines)                                                                                                                                 | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                                                                                                                                                                                                            | 05          |
| `typecheck/types/mono.rs` (1077 lines, including a synonym table)                                                                                                                | `sema/types/{repr,universe}.rs`                                                                                                                                                                                                                                                                       | 03          |
| `typecheck/types/const_data.rs` (a second operator enum)                                                                                                                         | Unify its operators with those of `ast::Type`                                                                                                                                                                                                                                                         | 03          |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                                                                                     | Delete                                                                                                                                                                                                                                                                                                | 06          |
| `typecheck/layers/dispatch.rs` (entire chain unused)                                                                                                                             | Delete                                                                                                                                                                                                                                                                                                | 06          |
| `typecheck/layers/`                                                                                                                                                              | `sema/layers/` + change `README.md` to describe the actual layer order                                                                                                                                                                                                                                | 06          |
| `typecheck/proof/`                                                                                                                                                               | Promote to top-level `proof/`                                                                                                                                                                                                                                                                         | 02          |
| `middle/core/ir_gen.rs` (originally a single file with no tests)                                                                                                                 | `middle/lower/{mod,scope,query,stmt,loops,globals,module_ir,const_eval,curry}.rs` + `lower/expr/{mod,call,access,control,container,lit,op,reference,closure,spawn}.rs` (split in 7.0.1-7.0.3)                                                                                                         | 04          |
| `middle/core/ir.rs` (905 lines)                                                                                                                                                  | `middle/ir/` + `verify.rs`                                                                                                                                                                                                                                                                            | 04          |
| `middle/core/bytecode.rs` (2422 lines, 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization)                                                        | Consolidate into `middle/bytecode/`: `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules)                                                                                                                                                                              | 06          |
| `backends/common/opcode.rs` (83-constant vocabulary)                                                                                                                             | `middle/bytecode/opcode.rs` (eliminating the L3→L4 reverse dependency)                                                                                                                                                                                                                                | 06          |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                                                                        | Delete `416-830`; retain 340 lines                                                                                                                                                                                                                                                                    | 06          |
| `std/fs.rs`, `std/net.rs`, etc. (native API bindings)                                                                                                                            | Remain in place                                                                                                                                                                                                                                                                                       | —           |
| `repl/` (1152 lines, no tests, no echo)                                                                                                                                          | Retain or remove, as decided in Section D of 06                                                                                                                                                                                                                                                       | 06          |

**Directory renaming is not the goal.** The changes fall into two categories:

- **Internal structures that must change:** the **internal structures** of the three files
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (4167 lines + real submodules of 470
  / 1107 / 1553 lines), and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: renaming.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`, and
  splitting `parser/ast.rs` into a top-level `ast/` (with the `Type` definition moved into
  `type_.rs`); migrating the opcode vocabulary to `middle/bytecode/`, completed together with P5/P6
  (Decision D1). **No phased rollout and no "optional" treatment.** The reason is that a directory
  name is the first signal of responsibility: `middle/core/` currently contains three unrelated
  things—`ir.rs`, `ir_gen.rs`, and `bytecode.rs`—so its name is no longer meaningful. The AST is
  consumed by every layer, and placing it under `frontend/` would mislead downstream paths. CI
  checks enforce the boundaries, but **the checks reference the directory responsibility table, and
  the responsibility table must match the actual directory names**; otherwise, `check-boundary.py`
  cannot determine whether code is in the correct location.

### Naming Conventions

| Rule                                                                           | Rationale                                                                                                                                                                          |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A concept has exactly one owning module                                        | Prevents parallel definitions such as "3 type representations"                                                                                                                     |
| The submodule directory for `foo.rs` is named `foo/`                           | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                                |
| Cross-layer references must use `pub` APIs, not leaked `pub(crate)` items      | The **count** of `pub(crate)` leaks serves as an indicator of boundary erosion and may only decrease, never increase (this is a semantic-boundary metric, not a line-count metric) |
| Internal implementations do not use `pub` unless they expose a cross-layer API | Reduces the visibility surface                                                                                                                                                     |
| Type file names use `type_.rs` and `fn_.rs` to avoid keywords                  | `type` / `fn` are reserved words starting with Rust 2024                                                                                                                           |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                       | See Section D of 06: parallel copies used to express a "new version" are a common source of dead code                                                                              |

## Feature Routing Tables

**These answer the question: "If I want to add X, where should I change it?"** This is the most
valuable long-term part of this document group, because it remains valid regardless of any
individual refactoring.

### Routing Table A: Language Features

| What to Add                          | L2 Lexing                                                                | L2 Parsing                                                                                                                                                 | L3 Semantics                                                                                                                                                                          | L3 IR                                        | L4 Execution                                        | Gates/Documentation                                                                                            |
| ------------------------------------ | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **Binary operator**                  | Add `TokenKind` to the keyword table in `lexer/state.rs` and `tokens.rs` | Add a BP constant to `pratt/precedence.rs`, an arm to `infix_info` in `led.rs`, a mapping to `parse_binary` in `led.rs`, and a `BinOp` variant to `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                                                          | Corresponding lowering in `lower/`           | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                                  |
| **Unary operator**                   | Same as above                                                            | Add an arm to `prefix_info` in `nud.rs`, add a new parse function, and add a `UnOp` variant to `ast.rs`                                                    | `infer_unary`                                                                                                                                                                         | Same as above                                | Same as above                                       | RFC-010 table                                                                                                  |
| **Constant-evaluation operators** ⚠️ | —                                                                        | —                                                                                                                                                          | **`BinOp` at `types/const_data.rs:234` and `UnOp` at `types/const_data.rs:321` are a second parallel enum alongside `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` between them** | `ir_gen.rs:2317` needs `use ast::BinOp as B` | —                                                   | **No gate**—see Routing Table C                                                                                |
| **Expression form**                  | —                                                                        | Add an `Expr` variant to `ast.rs` + `nud.rs`/`led.rs`                                                                                                      | Add an inference method to `inference/expressions.rs`                                                                                                                                 | Add a module under `lower/exprs/`            | `opcode.rs` or reuse an existing opcode             | —                                                                                                              |
| **Statement form**                   | —                                                                        | Add a `StmtKind` variant to `ast.rs` + `statements/`                                                                                                       | `inference/statements.rs`                                                                                                                                                             | `lower/`                                     | Same as above                                       | —                                                                                                              |
| **Type construction**                | —                                                                        | `Type` in `ast.rs` (see 03)                                                                                                                                | `types/mono.rs` + `solver.rs`                                                                                                                                                         | `FunctionCode.params` type                   | Serialization                                       | RFC-011                                                                                                        |
| **Pattern form**                     | —                                                                        | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                            | `inference/patterns.rs`                                                                                                                                                               | `compile_pattern` in `lower/`                | `opcode.rs`                                         | RFC-010b                                                                                                       |
| **Diagnostic code**                  | —                                                                        | —                                                                                                                                                          | —                                                                                                                                                                                     | —                                            | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (automatically validated by the `build.rs` gate) |
| **Warning (W prefix)**               | —                                                                        | —                                                                                                                                                          | —                                                                                                                                                                                     | —                                            | —                                                   | Same as above; note that warnings are not included in the error count                                          |

> **Current pain point**: adding one binary operator requires changes in **6-7 places** within the
> parser/lexer, plus **17 production files** downstream that use `BinOp::`. Whether omissions are
> detected depends on three categories:
>
> - **Caught by the compiler:** locations that perform an **exhaustive `match`** over `ast::BinOp`
>   fail to compile when a case is missing. Most locations fall into this category.
> - **Not caught by the compiler (the true blind spot):** the `matches!` macro, `_` fallback arms,
>   and the three kinds of **cross-enum** mappings. Missing changes in the first two categories
>   cause the new operator to be treated as a different operator; missing changes in the third
>   category (`ast::BinOp` ↔ `const_data::BinOp`) are completely silent because the two types
>   compile independently.
> - **L4 is not involved:** `backends/interpreter/` has **zero references** to `BinOp` or `ast:`—the
>   interpreter never sees the AST and recognizes only opcodes. Downstream pressure is therefore
>   concentrated in L3, not L4.
>
> Goal: after the grammar-driven migration, reduce the number of changes to 1-2 locations + 1
> automatically generated location (see 05); resolve the cross-enum portion through same-source
> generation (see 05); and consolidate the bytecode layer's third operator enum
> (`BinaryOp`/`UnaryOp`/`CompareOp`) through a generation-time opcode gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| What to Add                                                      | Location                                                                                                                                                                            | Locations That Must Change in Sync                                                                                                                                                                                                                                                                       |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation stage**                                            | Add a `Stage` variant to `driver/stage.rs`                                                                                                                                          | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, `dispatch` in `driver/mod.rs` (exhaustive matching forces additions), and CI stage-coverage tests                                                                                                                              |
| **Cross-stage checking layer** (such as ownership / termination) | Add a module under `sema/layers/`                                                                                                                                                   | Register it in the stage table + update `layers/README.md` (**the call-order table must correspond line by line to the call sites in `checker.rs`**) + merge its outputs into `Obligations`                                                                                                              |
| **Opcode**                                                       | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`; migrate it when the bytecode domain is consolidated so that all touch points reside in one `middle/` directory) | **5 locations**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the decode match in `From<BytecodeFile>`, and `translate_*` in `translator.rs`. Only 2 are forced by exhaustive compiler matches; the other 3 fail only when a `.42` artifact is run. A generation-time gate is required (see 06) |
| **CLI subcommand**                                               | `Commands` enum in `main.rs`                                                                                                                                                        | The `pub` entry point in `lib.rs` (currently 7), and `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                  |
| **LSP capability**                                               | Add a handler under `lsp/handlers/`                                                                                                                                                 | `handle_request` in `lsp/server.rs` (currently 274 lines, with 13 repeated boilerplate sections), and state in `lsp/world.rs`                                                                                                                                                                            |
| **Standard-library API**                                         | `src/std/*.rs`                                                                                                                                                                      | **Automatic gates already exist**: byte-for-byte comparison in `gen_interfaces.rs` + marked-range drift detection in `gen_docs.rs`. This is the most mature pattern in the project and should be extended to the opcode and type tables                                                                  |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions That Must Be Eliminated

| Reverse Dependency                                              | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Elimination Method                                                                                                                                            | Owning Document |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| parser → typecheck (**only 2 locations, in the same function**) | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both calling `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Pass operator interface specifications down to the parser as data; the parser no longer `use`s typecheck                                                      | 03              |
| Hard-coded predicate names in the parser                        | `declarations.rs:499`: `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. The comment at `:494-497` describes this as an unresolved open-set problem: "a future open-set solution would pass predicate names down to the parser, or change them into a form decidable at the syntax layer"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | As above; pass the predicate set down as data                                                                                                                 | 03              |
| **Duplication created to avoid a cross-module dependency**      | `name_used_as_type_in` at `ast.rs:847` and `name_used_as_type` at `declarations.rs:42` are **synonymous but independently implemented**. The comments at `ast.rs:845-846` admit that this is "independently implemented here to avoid an internal parser cross-module dependency"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Merge them into a single implementation through the probe data flow introduced in 03                                                                          | 03              |
| checker → internal layers functions                             | The walk family in `checker/refinement.rs` and `revalidate_refined` / `check_return_refinement` call `layers::termination::{negate_guard, substitute_const_expr}` (formerly `checker.rs:4857/4986/5246/5384`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Move them to the public API of `sema/refine/` or `sema/layers/termination`                                                                                    | 02              |
| 3 parallel type representations                                 | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (`ir.rs:3`, re-exported with `pub use`) + serialized `type_table`; these are bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Consolidate into a single representation                                                                                                                      | 03              |
| Handwritten synonym table                                       | Eliminated—6.1.2 removed 22 lowercase spelling aliases (S11.1 later migrated the DateTime compatibility alias, issue #513); `from_builtin_name` is now `mono.rs:1121-1143` (15 canonical names, with no aliases)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Completed (6.1.2 + S11.1)                                                                                                                                     | 03              |
| **3 parallel operator enums**                                   | `pub enum BinOp` at `ast.rs:191` (20 variants) / `pub enum UnOp` at `ast.rs:217` (4 variants, including the unique `Deref`); `pub enum BinOp` at `types/const_data.rs:234` (18 variants, using `Ne` instead of `Neq`) / `pub enum UnOp` at `types/const_data.rs:321` (4 variants, including the unique `BitNot`); `BinaryOp` at `middle/core/bytecode.rs:70` (11 variants, with yet another naming scheme for `Rem`/`Xor`/`Sar`) + `UnaryOp` + `CompareOp`. **There is no `From` / `TryFrom` between ast ↔ const_data**; the only bridge is the handwritten lossy free functions in `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot`, etc. silently return `None`), forcing 5+ files to alias imports (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`); see `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, and `middle/core/ir_gen.rs:2317` | Consolidate the L2 pair through same-source generation (see 05); check the bytecode enum against generated opcodes through a generation-time gate (see 06 F8) | 03 / 05         |
| `include!`                                                      | Eliminated in 5.1: `checker.rs` was changed to a real `mod semantic_tokens`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Complete                                                                                                                                                      | —               |

## Future Extension Guidelines

### Checklist for Adding a Language Feature

1. **Consult Routing Table A first** to determine the target location. Do not choose files by
   intuition.
2. **Types come first**: if the feature involves types, follow the single representation defined in
   03; do not add a third representation.
3. **Keep gates synchronized**: diagnostic-code changes must simultaneously update `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table**—`build.rs` enforces consistency among all
   three.
4. **Register the stage**: if the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Check reverse dependencies**: new code must not make L2 depend on L3 or L4 depend on L1/L2.
6. **Wire up tests**: every new test directory must also have a `mod` declaration in its parent
   module, or the tests will not run—the newly added CI check will catch this.

### Adding a "Layer" (Such as a Future Static Check)

1. Create a module under `sema/layers/` that **does not depend on any layer with a higher number**.
2. Place its outputs in `Obligations`, where the orchestration layer consumes them uniformly.
3. Update `sema/layers/README.md` so that it **describes the actual layer order** (use line numbers
   from the current tree, with locations anchored by function names or driver points).
4. Register it in the `Stage` enum and explicitly declare the modes in which it runs in
   `Program::stages()`.

---

## Where to Find the Maintenance Mechanism

The anti-regression gate (CI checklist, decision procedure, and three prohibitions) has moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md).

This document answers only the question of **"where things belong"**: dependency direction
specifications, the target directory structure, feature routing tables, and naming conventions.
