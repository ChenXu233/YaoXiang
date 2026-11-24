# Functional Routing and Dependency Standards

> **Supporting design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) and
> defines the **module boundaries, dependency directions, target directory structure, functional
> routing tables, and anti-regression mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution phases, and acceptance criteria are covered in
> the main RFC-039 text. See the other documents in this directory for detailed layer designs.

## Dependency Direction Standards

The following rules are enforced by CI checks (see [Anti-Regression Mechanism](#防反弹机制)):

| Rule                                                               | Enforcement Method                                                                                             | Current Status                                                                                                                                                       |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `include!` is prohibited                                           | `grep -rn 'include!' src/` must return 0                                                                       | 0 occurrences (eliminated in 5.1: `checker.rs` was converted to a real `mod`)                                                                                        |
| L2 must not `use` L3                                               | Scan `src/frontend/core/{lexer,parser}/` for `use ...typecheck`                                                | 2 occurrences exist and must be removed (see Routing Table C)                                                                                                        |
| L3/L4 must not `use` L2                                            | Scan `sema/`, `proof/`, `middle/`, and `backends/` for `use ...frontend::{lexer,parser,module}`                | Currently 0, with 0 as the baseline (AST is a top-level domain and needs no exemption clause)                                                                        |
| L4 must not reference L1/L2 in reverse                             | Scan `src/backends/` and `src/middle/` for `use ...frontend::core::{lexer,parser}`                             | A baseline must first be established                                                                                                                                 |
| The opcode vocabulary has a single owner: `middle/bytecode/`       | Scan `src/backends/` for opcode constant definitions (imports only); `sema/` must contain no opcode references | The vocabulary currently resides in `backends/common/opcode.rs` (creating an L3→L4 reverse dependency) and will be migrated when the bytecode domain is consolidated |
| Cross-layer `pub(crate)` leakage may only decrease, never increase | Include the count in the baseline file                                                                         | Cross-layer leakage still exists; the current count is tracked and reported by `scripts/ci/check-boundary.py` (total and per-directory counts)                       |
| Hard-coded type and predicate names are prohibited                 | Scan type-name string literals in `src/frontend/core/parser/`                                                  | `"Terminates"`, etc.                                                                                                                                                 |

## Target Directory Structure (After Implementation Is Complete)

The following is the shape **after all implementation phases are complete**, not the target of the
first phase. See the mapping table below for the differences between the current and target shapes.

```
src/
├── driver/                        # L1 orchestration: the sole decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustively matchable) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-stage diagnostic aggregation + exit-code decision
│
├── frontend/                      # L2 lexing and parsing
│   ├── lexer/                     #   Declarative: numbers unified as scan_radix(base),
│   │   ├── mod.rs                 #     sole escape-decoding implementation (merge the 3 copies in literals.rs)
│   │   ├── number.rs              #     replaces 4 line-for-line identical radix scanners
│   │   ├── escape.rs              #     sole escape decoder
│   │   ├── string.rs              #     strings / multiline strings
│   │   ├── fstring.rs             #     f-string; interpolation advances to the parser instead of nesting tokenize
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
│       ├── consistency.rs         #     lock / vendor consistency (merge the current 60 near-duplicate lines)
│       └── discover.rs            #     discovery strategy that follows use
│
├── ast/                           # Sole AST + sole type carrier (top-level domain, peer to parser; see 03)
│   ├── mod.rs
│   ├── expr.rs                    #   Expr (after convergence; see 03 for handling dead variants)
│   ├── stmt.rs                    #   StmtKind (remove signature_params)
│   ├── type_.rs                   #   Sole type structure Type + NameKind (26 → converged variant set)
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
│   │   ├── mod.rs                 #     StatementChecker location
│   │   ├── call/                  #     split infer_call_expr (see the split axes in 03/04)
│   │   │   ├── mod.rs             #       dispatch by "callee form"
│   │   │   ├── construct.rs       #       type construction / generic type construction
│   │   │   ├── generic.rs         #       explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (sole three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   module-level checks (after splitting checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     annotation type-name validation + is_predicate_head (now a real submodule of checker)
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     import/export / unused imports
│   │   └── semantic_tokens.rs     #     semantic token collection (now a real submodule of checker)
│   ├── refine/                    #   refinement types (currently the single file checker/refinement.rs, 1107 lines)
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
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to top level; the driver obligations ledger holds proof_calls, the three sema layers share the SMT backend, and it is promoted for cross-layer consumption)
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
│   │   └── synth.rs               #     the only place permitted to construct ast::Expr
│   ├── passes/                    #   passes on IR
│   │   ├── mono/                  #     monomorphization (delete 415 lines of closure remnants)
│   │   ├── dce/                   #     post-mono dead-code elimination (root-set mark-sweep)
│   │   └── ...
│   ├── codegen/                   #   IR → bytecode lowering
│   │   ├── mod.rs                 #     CodegenContext
│   │   ├── translator.rs
│   │   └── operand.rs             #     u8 stack slots (no real register file)
│   └── bytecode/                  #   bytecode domain: opcode vocabulary + containers + encoding/decoding
│       ├── opcode.rs              #     sole opcode vocabulary (+ generation-time gate)
│       ├── file.rs                #     BytecodeFile serialization (MAGIC / VERSION)
│       ├── mod.rs                 #     BytecodeModule facade + Display
│       ├── instr.rs               #     BytecodeInstr
│       ├── instr_meta.rs          #     opcode() + size() (same file for differential testing)
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
├── std/                           # L4 standard library (Rust implementation + .yx hybrid)
├── util/                          # Cross-layer utilities (diagnostics, spans, cache, i18n); keep the existing name—the Go "avoid util" reference in 08 is only moderately applicable, and renaming would create net churn without domain benefit, so record it as an exception
├── formatter/                     # Formatting
├── lsp/                           # Language server
├── repl/                          # REPL (if retained, see Section D in 06)
└── package/                       # Package management
```

### Responsibilities and Boundary Invariants for Each Directory

| Directory                                 | Responsibility                                                                        | Invariants                                                                                                                                                          |
| ----------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Decide which phases to run, how many times to run them, and how failures propagate    | **Must not** be reverse-depended on by L2/L3/L4; the set of phases must be represented by an exhaustively matchable enum                                            |
| `frontend/lexer`, `frontend/parser`       | Lexing and parsing; construct `ast/` nodes                                            | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics must all be passed down as data |
| `ast/`                                    | The sole AST and type representation (top-level shared data domain, peer to `parser`) | **Must not** contain a second definition of the same concept; must not `use` other layers (pure data and pure functions only)                                       |
| `frontend/module/`                        | Multi-file discovery, registry, and roles                                             | Perform discovery and registration only; **do not run analysis phases**. The `driver` schedules analyses                                                            |
| `sema/`                                   | Type checking and static analysis                                                     | **Must not** construct IR; must not be aware of opcodes                                                                                                             |
| `proof/`                                  | Proof result types and SMT backends                                                   | The Solver trait is the sole backend abstraction point; **must not** panic through `.expect()`                                                                      |
| `middle/ir/`                              | IR definitions and invariants                                                         | The IR must satisfy SSA discipline; a green `verify_ssa` is a prerequisite for merging                                                                              |
| `middle/lower/`                           | AST → IR                                                                              | The **only** place permitted to construct `ast::Expr` is `synth.rs`                                                                                                 |
| `middle/codegen/`                         | IR → bytecode lowering                                                                | Depend only on IR, not `sema`; do not define the opcode vocabulary                                                                                                  |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` containers + encoding/decoding             | **Sole ownership of the opcode vocabulary**; `backends/` only consumes it and never defines it; `sema` is not aware of opcodes                                      |
| `backends/`                               | Execution                                                                             | **Must not** reference L1/L2 in reverse; must not be aware of the AST                                                                                               |
| `middle/passes/mono/`                     | Passes on IR                                                                          | Must not reference `frontend`                                                                                                                                       |
| `middle/passes/dce/`                      | Passes on IR (post-mono reachability deletion)                                        | Must not reference `frontend`; the root-set and deletion criteria must have a single owner in this module                                                           |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tooling layers (outside the L1–L4 model)                                   | Consume only the `pub` APIs of L1–L4; must not touch `pub(crate)` (the leakage count constrains them as well)                                                       |

### Current → Target Mapping

| Current                                                                                                                                                                          | Target                                                                                                                                                                                                                                                                                                            | Area    |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| `frontend/pipeline.rs` (740 lines, five-stage handwritten if chain)                                                                                                              | `driver/` + `Program::SingleFile`                                                                                                                                                                                                                                                                                 | 02      |
| `frontend/module/orchestrator.rs` (1528 lines, four entry points, 60% divergence)                                                                                                | Leave only discovery/registration in `frontend/module/`; move compilation logic to `driver`                                                                                                                                                                                                                       | 02      |
| `frontend/compiler.rs` (267 lines)                                                                                                                                               | Slim it down to a thin `driver` wrapper                                                                                                                                                                                                                                                                           | 02      |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                                                                             | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                                                                                                                                                                                                                | 05      |
| `frontend/core/parser/pratt/` (hand-written match dispatch, two sets of BP ladders)                                                                                              | `frontend/parser/grammar/` + a single precedence table                                                                                                                                                                                                                                                            | 05      |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                                                                       | Split into the top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                                                                                                                                                                                                                 | 03 / D1 |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8–10 responsibility sections)                                                                                     | Split `frontend/parser/` and delete 120 lines of dead legacy syntax                                                                                                                                                                                                                                               | 05      |
| `typecheck/checker.rs` (4167 lines + three real submodules: `checker/annotations.rs` 470 lines, `checker/refinement.rs` 1107 lines, and `checker/semantic_tokens.rs` 1553 lines) | `sema/check/` + `sema/refine/`; convert `include!` to a real `mod` (completed in 5.1), migrate the refinement block to a submodule and consolidate its entry point (completed in 5.2: the entry point takes `&self` and returns diagnostics), and migrate the annotation family to a submodule (completed in 5.3) | 02 / 05 |
| `typecheck/checker/semantic_tokens.rs` (1553 lines, already a real submodule of `checker`)                                                                                       | `sema/check/semantic_tokens.rs`                                                                                                                                                                                                                                                                                   | 02      |
| `typecheck/inference/expressions.rs` (originally a single file)                                                                                                                  | `sema/inference/expressions/{mod,expr}.rs` + `expressions/call/{mod,args,instantiation,method}.rs` (split in 7.0.4; merging `statements.rs` and renaming `inference/` are handled separately)                                                                                                                     | 05 / 04 |
| `typecheck/inference/statements.rs` (2887 lines)                                                                                                                                 | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                                                                                                                                                                                                                        | 05      |
| `typecheck/types/mono.rs` (1077 lines, includes a synonym table)                                                                                                                 | `sema/types/{repr,universe}.rs`                                                                                                                                                                                                                                                                                   | 03      |
| `typecheck/types/const_data.rs` (second operator enum)                                                                                                                           | Align its operators with those of `ast::Type`                                                                                                                                                                                                                                                                     | 03      |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                                                                                     | Delete                                                                                                                                                                                                                                                                                                            | 06      |
| `typecheck/layers/dispatch.rs` (entire chain has zero calls)                                                                                                                     | Delete                                                                                                                                                                                                                                                                                                            | 06      |
| `typecheck/layers/`                                                                                                                                                              | `sema/layers/` + change `README.md` to describe the actual layer order                                                                                                                                                                                                                                            | 06      |
| `typecheck/proof/`                                                                                                                                                               | Promote to the top-level `proof/`                                                                                                                                                                                                                                                                                 | 02      |
| `middle/core/ir_gen.rs` (originally a single file, with zero tests)                                                                                                              | `middle/lower/{mod,scope,query,stmt,loops,globals,module_ir,const_eval,curry}.rs` + `lower/expr/{mod,call,access,control,container,lit,op,reference,closure,spawn}.rs` (split in 7.0.1–7.0.3)                                                                                                                     | 04      |
| `middle/core/ir.rs` (905 lines)                                                                                                                                                  | `middle/ir/` + `verify.rs`                                                                                                                                                                                                                                                                                        | 04      |
| `middle/core/bytecode.rs` (2422 lines, including a 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization)                                            | Consolidate into `middle/bytecode/`: the `opcode.rs` vocabulary + `file.rs` container serialization + `decode/` (10 modules)                                                                                                                                                                                      | 06      |
| `backends/common/opcode.rs` (83-constant vocabulary)                                                                                                                             | `middle/bytecode/opcode.rs` (eliminate the L3→L4 reverse dependency)                                                                                                                                                                                                                                              | 06      |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                                                                        | Delete `416-830` and retain 340 lines                                                                                                                                                                                                                                                                             | 06      |
| `std/fs.rs`, `std/net.rs`, etc. (native API bindings)                                                                                                                            | Remain in place                                                                                                                                                                                                                                                                                                   | —       |
| `repl/` (1152 lines, zero tests, no echo)                                                                                                                                        | Retain or delete; see Section D in 06                                                                                                                                                                                                                                                                             | 06      |

**Renaming directories is not the goal.** Changes fall into two categories:

- **Internal structures that must change:** the internal structures of the three files
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (4167 lines + real submodules of 470
  / 1107 / 1553 lines), and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: renaming.** Rename `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`,
  split `parser/ast.rs` into the top-level `ast/` (with the `Type` definition moved into
  `type_.rs`), and migrate the opcode vocabulary to `middle/bytecode/`, all as part of P5/P6
  (Decision D1). **No phased rollout and no “optional” option.** The reason is that a directory name
  is the first signal of responsibility—`middle/core/` currently contains three unrelated things,
  `ir.rs`, `ir_gen.rs`, and `bytecode.rs`, so the name no longer conveys its purpose. The AST is
  consumed by every layer, and placing it under `frontend/` would mislead downstream paths. CI
  checks enforce the boundaries, but **the checks reference the directory responsibility table, and
  that table must match the actual directory names**; otherwise, `check-boundary.py` cannot
  determine whether code is in the correct location.

### Naming Conventions

| Rule                                                                               | Rationale                                                                                                                                                                  |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A concept has exactly one owning module                                            | Prevents parallel definitions such as “3 type representations”                                                                                                             |
| The submodule directory for `foo.rs` is named `foo/`                               | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                        |
| Cross-layer references must use `pub` APIs, not leaked `pub(crate)` items          | The **count** of `pub(crate)` leaks serves as a boundary-erosion metric and may only decrease, never increase; this is a semantic-boundary metric, not a line-count metric |
| Internal implementations must not be made `pub` unless they form a cross-layer API | Reduces the visible surface                                                                                                                                                |
| Type file names use `type_.rs` and `fn_.rs` to avoid keywords                      | `type` / `fn` are reserved words as of Rust 2024                                                                                                                           |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                           | See Section D in 06: using parallel copies to represent a “new version” is a common source of dead code                                                                    |

## Functional Routing Table

**Answers the question: “I want to add X; where should I change it?”** This is the most valuable
long-term part of this document group—it does not become obsolete after any single refactoring.

### Routing Table A: Language Features

| Thing to Add                        | L2 Lexing                                                         | L2 Parsing                                                                                                                                                             | L3 Semantics                                                                                                                                                | L3 IR                                        | L4 Execution                                        | Gate/Documentation                                                                                           |
| ----------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| **Binary operator**                 | Keyword table in `lexer/state.rs`; add `TokenKind` to `tokens.rs` | Add a BP constant to `pratt/precedence.rs`, add an arm to `infix_info` in `led.rs`, add a mapping to `parse_binary` in `led.rs`, and add a `BinOp` variant to `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                                | Corresponding lowering in `lower/`           | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                                |
| **Unary operator**                  | Same as above                                                     | Add an arm to `prefix_info` in `nud.rs` + a new parse function + a `UnOp` variant to `ast.rs`                                                                          | `infer_unary`                                                                                                                                               | Same as above                                | Same as above                                       | RFC-010 table                                                                                                |
| **Constant-evaluation operator** ⚠️ | —                                                                 | —                                                                                                                                                                      | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` are a second set of enums parallel to `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` between them** | `ir_gen.rs:2317` needs `use ast::BinOp as B` | —                                                   | **No gate**—see Routing Table C                                                                              |
| **Expression form**                 | —                                                                 | Add an `Expr` variant to `ast.rs` + `nud.rs`/`led.rs`                                                                                                                  | Add an infer method to `inference/expressions.rs`                                                                                                           | Add a module under `lower/exprs/`            | `opcode.rs` or reuse                                | —                                                                                                            |
| **Statement form**                  | —                                                                 | Add a `StmtKind` variant to `ast.rs` + `statements/`                                                                                                                   | `inference/statements.rs`                                                                                                                                   | `lower/`                                     | Same as above                                       | —                                                                                                            |
| **Type construction**               | —                                                                 | `Type` in `ast.rs` (see 03)                                                                                                                                            | `types/mono.rs` + `solver.rs`                                                                                                                               | `FunctionCode.params` type                   | Serialization                                       | RFC-011                                                                                                      |
| **Pattern form**                    | —                                                                 | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                                        | `inference/patterns.rs`                                                                                                                                     | `compile_pattern` in `lower/`                | `opcode.rs`                                         | RFC-010b                                                                                                     |
| **Diagnostic code**                 | —                                                                 | —                                                                                                                                                                      | —                                                                                                                                                           | —                                            | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (automatically checked by the `build.rs` gate) |
| **Warning (W prefix)**              | —                                                                 | —                                                                                                                                                                      | —                                                                                                                                                           | —                                            | —                                                   | Same as above; note that warnings are not included in the error count                                        |

> **Current pain point**: Adding a binary operator requires changes in **6–7 locations** (within the
> parser/lexer) + **17 production files** downstream that use `BinOp::`. Whether an omission is
> detected falls into three categories:
>
> - **Caught by the compiler**: Locations that perform an **exhaustive `match`** on `ast::BinOp`
>   fail to compile if an omission is not corrected. These make up the majority.
> - **Not caught by the compiler (the real blind spots)**: The `matches!` macro, `_` fallback arms,
>   and **cross-enum** code form three categories. Omissions in the first two cause the “new
>   operator” to be treated as a different operator. Omissions in the third category (`ast::BinOp` ↔
>   `const_data::BinOp`) are completely silent—the two types compile independently.
> - **L4 is not involved**: `backends/interpreter/` has **zero references** to `BinOp` or
>   `ast::`—the interpreter never sees the AST and recognizes only opcodes. Downstream pressure is
>   therefore concentrated in L3, not L4.
>
> Target: after the grammar-driven implementation, reduce this to 1–2 locations + 1 autogenerated
> location (see 05); resolve the cross-enum portion through same-source generation (see 05); and
> consolidate the third operator-enum set in the bytecode layer (`BinaryOp`/`UnaryOp`/`CompareOp`)
> through the opcode generation-time gate (see 06 F8).

### Routing Table B: Compilation Mechanisms

| Thing to Add                                                  | Landing Point                                                                                                                                                                          | Locations That Must Change in Sync                                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation phase**                                         | Add a `Stage` variant to `driver/stage.rs`                                                                                                                                             | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, `dispatch` in `driver/mod.rs` (the exhaustive match forces completion), and CI phase-coverage tests                                                                                                                                          |
| **Cross-stage check layer** (such as ownership / termination) | Add a module under `sema/layers/`                                                                                                                                                      | Register it in the phase table + update `layers/README.md` (**the call-order table must correspond one by one to the call sites in `checker.rs`**) + merge its outputs into `Obligations`                                                                                                                              |
| **Opcode**                                                    | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`; migrate it when the bytecode domain is consolidated and bring all touchpoints into the single `middle/` directory) | **5 locations**: `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the `From<BytecodeFile>` decoding match, and `translate_*` in `translator.rs`. Only 2 are forced by an exhaustive compiler match; omissions in the other 3 fail only when running a `.42` artifact. A generation-time gate is required (see 06) |
| **CLI subcommand**                                            | `Commands` enum in `main.rs`                                                                                                                                                           | The `pub` entry point in `lib.rs` (currently 7), and `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                |
| **LSP capability**                                            | Add a handler under `lsp/handlers/`                                                                                                                                                    | `handle_request` in `lsp/server.rs` (currently 274 lines, with 13 instances of duplicated boilerplate), and state in `lsp/world.rs`                                                                                                                                                                                    |
| **Standard-library API**                                      | `src/std/*.rs`                                                                                                                                                                         | **An automatic gate already exists**: byte-for-byte comparison in `gen_interfaces.rs` + marked-range drift detection in `gen_docs.rs`. This is the most mature pattern in the entire project and should be extended to opcode and type tables                                                                          |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions

| Reverse Dependency                                                | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | How to Eliminate It                                                                                                                                                      | Owning Document   |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------- |
| parser → typecheck (**only 2 occurrences, in the same function**) | `parser/statements/declarations.rs:509` and `parser/ast.rs:930` both call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Pass operator-interface specifications down to the parser as data; the parser no longer `use`s typecheck                                                                 | 03                |
| Hard-coded predicate names in the parser                          | `declarations.rs:499` `let is_predicate_app =                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | n: &str                                                                                                                                                                  | n == "Terminates" |     | state.is_predicate_name(n);`. The comment at `:494-497` describes this as an unresolved open-set problem: “a future open-set solution: pass predicate names down to the parser, or change them to a form decidable at the syntax layer” | As above; pass the predicate set down as data | 03  |
| **Duplication created to avoid cross-module dependencies**        | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` are synonymous but independent implementations. The comment at `ast.rs:845-846` admits that “this is independently implemented to avoid cross-module dependencies within the parser”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Merge them into a single implementation through 03's probe data flow                                                                                                     | 03                |
| checker → internal functions in layers                            | The traversal family in `checker/refinement.rs` and `revalidate_refined` / `check_return_refinement` call `layers::termination::{negate_guard, substitute_const_expr}` (formerly `checker.rs:4857/4986/5246/5384`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Move them into the public API of `sema/refine/` or `sema/layers/termination`                                                                                             | 02                |
| 3 parallel type representations                                   | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (`ir.rs:3`, re-exported with `pub use`) + the serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Converge on a single representation                                                                                                                                      | 03                |
| Hand-written synonym table                                        | Eliminated—6.1.2 removed 22 lowercase spelling aliases (S11.1 subsequently moved the DateTime compatibility alias; issue #513). `from_builtin_name` is now `mono.rs:1121-1143` (15 canonical names, with no aliases)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Completed (6.1.2 + S11.1)                                                                                                                                                | 03                |
| **3 parallel operator enums**                                     | `ast.rs:191` `pub enum BinOp` (20 variants) / `ast.rs:217` `pub enum UnOp` (4, with the unique `Deref`); `types/const_data.rs:234` `pub enum BinOp` (18, using `Ne` instead of `Neq`) / `:321` `pub enum UnOp` (4, with the unique `BitNot`); `middle/core/bytecode.rs:70` `BinaryOp` (11, with yet another naming scheme for `Rem`/`Xor`/`Sar`) + `:97` `UnaryOp` + `:106` `CompareOp`. **There is no `From` / `TryFrom` between ast ↔ const_data**; only hand-written lossy free functions exist in `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot`, etc. return `None` and are silently discarded), forcing 5+ files to alias imports (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`); see `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, and `middle/core/ir_gen.rs:2317` | Converge the 2 L2 sets through same-source generation (see 05); converge the bytecode set through differential validation by the opcode generation-time gate (see 06 F8) | 03 / 05           |
| `include!`                                                        | Eliminated (5.1: `checker.rs` was converted to a real `mod semantic_tokens`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Complete                                                                                                                                                                 | —                 |

## Future Extension Guidance

### Checklist When Adding a Language Feature

1. **Consult Routing Table A first** and determine the landing point. Do not choose a file based on
   intuition.
2. **Types first**: If the feature involves types, follow 03's single-representation design and do
   not add a third representation.
3. **Synchronize the gate**: Diagnostic-code changes must simultaneously update `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table**—`build.rs` enforces consistency among all
   three.
4. **Register the phase**: If the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Check reverse dependencies**: New code must not make L2 depend on L3 or L4 depend on L1/L2.
6. **Wire up tests**: A new test directory must also have a `mod` declaration in its parent module;
   otherwise, it will not run. A newly added CI check will catch this.

### When Adding a “Layer” (Such as a Future Static Check)

1. Create a module under `sema/layers/`; it **must not depend on any layer with a higher number**.
2. Put its outputs into `Obligations`, where the orchestration layer consumes them uniformly.
3. Update `sema/layers/README.md` so that it **describes the actual layer order** (use line numbers
   from the current tree, but anchor locations by function name or driver point).
4. Register the layer in the `Stage` enum, and explicitly declare the modes in which it runs in
   `Program::stages()`.

---

## Where to Find the Maintenance Mechanism

The anti-regression gate (CI checklist, decision procedure, and three prohibitions) has moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md).

This document only answers “where things go”: dependency direction standards, target directory
structure, functional routing tables, and naming conventions.
