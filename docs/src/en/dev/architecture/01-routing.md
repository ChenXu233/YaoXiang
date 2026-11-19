# Feature Routing and Dependency Rules

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). It
> defines the **module boundaries, dependency directions, target directory structure, feature
> routing tables, and anti-regression mechanism** within the four-layer model.
>
> The four-layer model itself, glossary, execution phases, and acceptance criteria are covered in
> the main RFC-039 text. Detailed design for each layer is provided in the remaining documents in
> this directory.

## Dependency Direction Rules

The following rules are enforced by CI checks (see [anti-regression mechanism](#防反弹机制)):

| Rule                                                               | How checked                                                                                                                     | Current state                                                                                                                                         |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `include!` is prohibited                                           | `grep -rn 'include!' src/` should return 0                                                                                      | 0 occurrences (eliminated in 5.1: `checker.rs` now uses a real `mod`)                                                                                 |
| L2 must not `use` L3                                               | Scan `src/frontend/core/{lexer,parser}/` for `use ...typecheck`                                                                 | 2 instances exist and must be eliminated (see Routing Table C)                                                                                        |
| L3/L4 must not `use` L2                                            | Scan `sema/`, `proof/`, `middle/`, and `backends/` for `use ...frontend::{lexer,parser,module}`                                 | Currently 0; use 0 as the baseline (AST is a top-level domain and requires no exemption)                                                              |
| L4 must not reference L1/L2 in reverse                             | Scan `src/backends/` and `src/middle/` for `use ...frontend::core::{lexer,parser}`                                              | A baseline must be established first                                                                                                                  |
| The opcode vocabulary has a single owner: `middle/bytecode/`       | Scan `src/backends/` to ensure it does not define opcode constants (imports only); ensure `sema/` contains no opcode references | The current vocabulary is in `backends/common/opcode.rs` (forming an L3→L4 reverse dependency); migrate it when the bytecode domains are consolidated |
| Cross-layer `pub(crate)` leakage may only decrease, never increase | Count and incorporate the count into the baseline file                                                                          | Cross-layer leakage still exists; the current count is reported by `scripts/ci/check-boundary.py` (total + per-directory counts)                      |
| Hard-coded type/predicate names are prohibited                     | Scan for type-name string literals in `src/frontend/core/parser/`                                                               | `"Terminates"` and similar                                                                                                                            |

## Target Directory Structure (After Completion)

The following is the state after **all implementation phases** are complete, not the target of the
first step. The gap between the current and target states is shown in the mapping table below.

```
src/
├── driver/                        # L1 orchestration: the sole decision point
│   ├── mod.rs                     #   Driver, run()
│   ├── stage.rs                   #   Stage enum (exhaustive) + StageScope
│   ├── program.rs                 #   Program: SingleFile / MultiFile / Check / Lsp / Embedded / WasmPlayground
│   ├── obligations.rs             #   Obligations ledger + assert_drained()
│   ├── unit.rs                    #   Unit: a single compilation unit (path + source + tokens + ast + type_result + ir)
│   └── diagnostics.rs             #   Cross-phase diagnostic aggregation + exit-code decision
│
├── frontend/                      # L2 lexing and parsing
│   ├── lexer/                     #   Declarative: numbers unified in scan_radix(base),
│   │   ├── mod.rs                 #     Sole escape-decoding implementation (merge the three copies in literals.rs)
│   │   ├── number.rs              #     Replaces the four duplicated radix scanners written line by line
│   │   ├── escape.rs              #     Sole escape decoder
│   │   ├── string.rs              #     Strings / multiline strings
│   │   ├── fstring.rs             #     f-string; interpolation now advances into the parser instead of nesting tokenize
│   │   └── token.rs               #     TokenKind (single source)
│   ├── parser/                    #   Grammar-driven: one precedence table, no raw magic numbers
│   │   ├── mod.rs                 #     parse_module
│   │   ├── grammar/               #     Grammar definitions (added after the grammar-driven implementation)
│   │   ├── pattern.rs             #     parse_pattern + expr_to_pattern
│   │   └── error.rs               #     synchronize() + Error placeholder node
│   └── module/                    #   Multi-file: discovery / registry / roles
│       ├── mod.rs
│       ├── registry.rs            #     ModuleRegistry
│       ├── resolver.rs            #     Resolver
│       ├── roles.rs               #     Script / Bin / Lib / Test / Internal
│       ├── consistency.rs         #     lock / vendor consistency (merge the current 60 lines of near-duplicate code)
│       └── discover.rs            #     Discovery strategy that follows use
│
├── ast/                           # Sole AST + sole type representation (top-level domain, peer to parser; see 03)
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
│   │   ├── call/                  #     Split infer_call_expr (see the split axis in 03/04)
│   │   │   ├── mod.rs             #       Dispatch by "callee form"
│   │   │   ├── construct.rs       #       Sum-type construction / generic type construction
│   │   │   ├── generic.rs         #       Explicit type arguments + monomorphization
│   │   │   └── dispatch.rs        #       Overload / method resolution / arity
│   │   ├── expr.rs
│   │   ├── stmt.rs
│   │   ├── pattern.rs
│   │   ├── scope.rs               #     ScopeManager (single three-chain model)
│   │   └── bounds.rs              #     BoundsChecker
│   ├── check/                     #   Module-level checking (after splitting checker.rs)
│   │   ├── mod.rs                 #     TypeChecker orchestration
│   │   ├── annotations.rs         #     Annotation type-name validation + is_predicate_head (proper submodule of the current checker)
│   │   ├── signatures.rs          #     collect_function_signature
│   │   ├── type_defs.rs           #     add_type_definition
│   │   ├── imports.rs             #     Imports/exports / unused imports
│   │   └── semantic_tokens.rs     #     Semantic token collection (proper submodule of the current checker)
│   ├── refine/                    #   Refinement types (current checker/refinement.rs single file, 1107 lines)
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
├── proof/                         # L3: proof and verification support (promoted from typecheck/proof/ to the top level; the driver's obligation ledger holds proof_calls, sema's three layers share the SMT backend, and because it is consumed across layers it is promoted to the top level)
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
│   │   ├── operand.rs             #   Operand (dead Temp/Label/Register variants removed)
│   │   ├── instr.rs               #   Instruction (including Phi)
│   │   ├── block.rs               #   BasicBlock
│   │   ├── fn_.rs                 #   FunctionIR / LocalSlot
│   │   └── verify.rs              #   verify_loose / verify_ssa
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
│   │   └── synth.rs               #     Sole place where constructing ast::Expr is allowed
│   ├── passes/                    #   Passes on IR
│   │   ├── mono/                  #     Monomorphization (delete 415 lines of closure remnants)
│   │   ├── dce/                   #     post-mono dead-code elimination (root-set mark-sweep)
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
│       ├── instr_meta.rs          #     opcode() + size() (same file facilitates differential testing)
│       ├── decode/                #     Decoding (split the current 1400-line From)
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
│   │   ├── debug.rs               #   opcode dispatch table
│   │   ├── ffi.rs
│   │   └── ops/                   #   Split by instruction family
│   └── runtime/                   #   DAG task scheduler (not a second execution backend)
│       ├── engine.rs
│       └── facade.rs
│
├── std/                           # L4 standard library (Rust implementation + .yx hybrid)
├── util/                          # Cross-layer utilities (diagnostics, span, cache, i18n; preserve existing names—the Go "avoid util" reference in 08 is only moderately applicable; renaming would cause net churn without domain benefit, so record it as an exception)
├── formatter/                     # Formatting
├── lsp/                           # Language services
├── repl/                          # REPL (if retained, see Section D of 06)
└── package/                       # Package management
```

### Responsibilities and Boundary Invariants for Each Directory

| Directory                                 | Responsibility                                                                      | Invariants                                                                                                                                                          |
| ----------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/`                                 | Decide which phases to run, how many times to run them, and how failures propagate  | **Must not** be reverse-depended on by L2/L3/L4; the phase set must be an exhaustive enum                                                                           |
| `frontend/lexer`, `frontend/parser`       | Lexing and parsing; construct `ast/` nodes                                          | **Must not** `use` any layer other than `ast` (`sema` / `middle` / `backends`). Predicate names, type names, and operator semantics must all be passed down as data |
| `ast/`                                    | The sole AST and type representation (top-level shared data domain, peer to parser) | **Must not** contain a second definition of the same concept; must not `use` other layers (pure data and pure functions only)                                       |
| `frontend/module/`                        | Multi-file discovery, registry, and roles                                           | Perform discovery and registration only; **do not run analysis phases**; analysis is scheduled by `driver`                                                          |
| `sema/`                                   | Type checking and static analysis                                                   | **Must not** construct IR; must not be aware of opcodes                                                                                                             |
| `proof/`                                  | Proof result types and SMT backends                                                 | The Solver trait is the sole backend abstraction point; **must not** panic via `.expect()`                                                                          |
| `middle/ir/`                              | IR definitions and invariants                                                       | The IR must satisfy SSA discipline; a green `verify_ssa` is a prerequisite for merging                                                                              |
| `middle/lower/`                           | AST → IR                                                                            | The **only** place allowed to construct `ast::Expr` is `synth.rs`                                                                                                   |
| `middle/codegen/`                         | IR → bytecode lowering                                                              | Depend only on IR, not `sema`; do not define the opcode vocabulary                                                                                                  |
| `middle/bytecode/`                        | Bytecode domain: opcode vocabulary + `.42` container + encoding/decoding            | **Sole owner of the opcode vocabulary**; `backends/` consumes but does not define it; `sema` is unaware of opcodes                                                  |
| `backends/`                               | Execution                                                                           | **Must not** reference L1/L2 in reverse; must not be aware of the AST                                                                                               |
| `middle/passes/mono/`                     | Passes on IR                                                                        | Must not reference `frontend`                                                                                                                                       |
| `middle/passes/dce/`                      | Passes on IR (post-mono reachability deletion)                                      | Must not reference `frontend`; the root-set/deletion criterion must have a single implementation in this module                                                     |
| `formatter/`, `lsp/`, `repl/`, `package/` | Peripheral tool layers (outside the L1-L4 model)                                    | Consume only the `pub` APIs of L1-L4; must not touch `pub(crate)` (the leakage count constrains them as well)                                                       |

### Current → Target Mapping

| Current state                                                                                                                                                                               | Target                                                                                                                                                                                                                                                                                                           | Doc     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| `frontend/pipeline.rs` (740 lines, hand-written five-stage if chain)                                                                                                                        | `driver/` + `Program::SingleFile`                                                                                                                                                                                                                                                                                | 02      |
| `frontend/module/orchestrator.rs` (1528 lines, 4 entry points, 60% divergence)                                                                                                              | `frontend/module/` retains only discovery/registration; compilation logic moves to `driver`                                                                                                                                                                                                                      | 02      |
| `frontend/compiler.rs` (267 lines)                                                                                                                                                          | Slim down to a thin wrapper around `driver`                                                                                                                                                                                                                                                                      | 02      |
| `frontend/core/lexer/literals.rs` (1568 lines, 4 radix scanners + 3 escape decoders)                                                                                                        | `frontend/lexer/{number,escape,string,fstring}.rs`                                                                                                                                                                                                                                                               | 05      |
| `frontend/core/parser/pratt/` (hand-written match dispatch, two sets of BP ladders)                                                                                                         | `frontend/parser/grammar/` + a single precedence table                                                                                                                                                                                                                                                           | 05      |
| `frontend/core/parser/ast.rs` (1166 lines: `Expr`/`StmtKind`/`Type`/`Pattern` definitions)                                                                                                  | Split into the top-level `ast/` (`expr`/`stmt`/`type_`/`pattern`)                                                                                                                                                                                                                                                | 03 / D1 |
| `frontend/core/parser/statements/declarations.rs` (1015 lines, 8–10 responsibilities)                                                                                                       | Split `frontend/parser/` + delete 120 lines of dead legacy syntax                                                                                                                                                                                                                                                | 05      |
| `typecheck/checker.rs` (4167 lines + three proper submodules: `checker/annotations.rs` at 470 lines, `checker/refinement.rs` at 1107 lines, and `checker/semantic_tokens.rs` at 1553 lines) | `sema/check/` + `sema/refine/`; convert `include!` to a real `mod` (completed in 5.1), migrate refinement blocks into submodules and centralize the entry point (completed in 5.2: the entry point takes `&self` and returns diagnostics), and migrate the annotation family into a submodule (completed in 5.3) | 02 / 05 |
| `typecheck/checker/semantic_tokens.rs` (1553 lines; already a proper submodule of `checker`)                                                                                                | `sema/check/semantic_tokens.rs`                                                                                                                                                                                                                                                                                  | 02      |
| `typecheck/inference/expressions.rs` (originally a single file)                                                                                                                             | `sema/inference/expressions/{mod,expr}.rs` + `expressions/call/{mod,args,instantiation,method}.rs` (split in 7.0.4; merging `statements.rs` and renaming `inference/` are handled separately)                                                                                                                    | 05 / 04 |
| `typecheck/inference/statements.rs` (2887 lines)                                                                                                                                            | `sema/infer/stmt.rs` + `sema/infer/mod.rs`                                                                                                                                                                                                                                                                       | 05      |
| `typecheck/types/mono.rs` (1077 lines, including a synonym table)                                                                                                                           | `sema/types/{repr,universe}.rs`                                                                                                                                                                                                                                                                                  | 03      |
| `typecheck/types/const_data.rs` (second operator enum)                                                                                                                                      | Align with the operator set of `ast::Type`                                                                                                                                                                                                                                                                       | 03      |
| `typecheck/types/inference/types.rs` (45 lines of dead code)                                                                                                                                | Delete                                                                                                                                                                                                                                                                                                           | 06      |
| `typecheck/layers/dispatch.rs` (entire chain has zero call sites)                                                                                                                           | Delete                                                                                                                                                                                                                                                                                                           | 06      |
| `typecheck/layers/`                                                                                                                                                                         | `sema/layers/` + change `README.md` to the actual layer order                                                                                                                                                                                                                                                    | 06      |
| `typecheck/proof/`                                                                                                                                                                          | Promote to the top-level `proof/`                                                                                                                                                                                                                                                                                | 02      |
| `middle/core/ir_gen.rs` (originally a single file, no tests)                                                                                                                                | `middle/lower/{mod,scope,query,stmt,loops,globals,module_ir,const_eval,curry}.rs` + `lower/expr/{mod,call,access,control,container,lit,op,reference,closure,spawn}.rs` (split in 7.0.1–7.0.3)                                                                                                                    | 04      |
| `middle/core/ir.rs` (905 lines)                                                                                                                                                             | `middle/ir/` + `verify.rs`                                                                                                                                                                                                                                                                                       | 04      |
| `middle/core/bytecode.rs` (2422 lines, including a 1400-line `From`) + `middle/passes/codegen/bytecode.rs` (763 lines, serialization)                                                       | Consolidate into `middle/bytecode/`: the `opcode.rs` vocabulary + the `file.rs` container serialization + `decode/` (10 modules)                                                                                                                                                                                 | 06      |
| `backends/common/opcode.rs` (83-constant vocabulary)                                                                                                                                        | `middle/bytecode/opcode.rs` (eliminate the L3→L4 reverse dependency)                                                                                                                                                                                                                                             | 06      |
| `middle/passes/mono/instance.rs` (415 lines of dead code)                                                                                                                                   | Delete `416-830`; retain 340 lines                                                                                                                                                                                                                                                                               | 06      |
| `std/fs.rs`, `std/net.rs`, and similar (native API bindings)                                                                                                                                | Unchanged                                                                                                                                                                                                                                                                                                        | —       |
| `repl/` (1152 lines, no tests, no echo)                                                                                                                                                     | Retain or delete; see Section D of 06                                                                                                                                                                                                                                                                            | 06      |

**Renaming directories is not the goal.** Changes fall into two categories:

- **Internal structure that must change:** the **internal structure** of the three files
  `middle/core/ir_gen.rs` (8448 lines), `typecheck/checker.rs` (4167 lines + proper submodules of
  470 / 1107 / 1553 lines), and `typecheck/inference/expressions.rs` (5525 lines).
- **Directory hierarchy: renames.** `typecheck/` → `sema/`, `middle/core/` → `middle/ir/`, split
  `parser/ast.rs` into the top-level `ast/` (the `Type` definition moves into `type_.rs`), and
  migrate the opcode vocabulary to `middle/bytecode/`, all completed together with P5/P6 (Decision
  D1). **No phased implementation and no “optional” path.** The rationale is that a directory name
  is the first signal of responsibility: `middle/core/` currently contains three unrelated
  things—`ir.rs`, `ir_gen.rs`, and `bytecode.rs`—so its name no longer reflects its contents. The
  AST is consumed by every layer, so placing it under `frontend/` would mislead downstream consumers
  about its path. CI checks enforce the boundaries, but **the check rules refer to the directory
  responsibility table, and the responsibility table must match the actual directory names**;
  otherwise, `check-boundary.py` cannot determine whether code is in the correct location.

### Naming Conventions

| Rule                                                                                   | Rationale                                                                                                                                                                   |
| -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A concept has exactly one owning module                                                | Prevents parallel definitions such as “three type representations”                                                                                                          |
| The submodule directory of `foo.rs` is named `foo/`                                    | Consistent with Rust 2018 module resolution (`src/frontend/core/typecheck/checker.rs` → `checker/`)                                                                         |
| Cross-layer references must use the `pub` API rather than leaking through `pub(crate)` | The **count** of `pub(crate)` leaks serves as a boundary-erosion metric and may only decrease, never increase (this is a semantic boundary metric, not a line-count metric) |
| Internal implementations do not use `pub` unless they expose a cross-layer API         | Reduces the visibility surface                                                                                                                                              |
| Type file names use `type_.rs` and `fn_.rs` to avoid keywords                          | `type` and `fn` are reserved words starting with Rust 2024                                                                                                                  |
| Do not add `xxx_core` / `xxx_new` / `xxx_v2` directories                               | See Section D of 06: using parallel copies to express a “new version” is a common source of dead code                                                                       |

## Feature Routing Tables

**Answers “Where should I add X?”** This is the most enduringly valuable part of this document
set—it does not become obsolete with any individual refactoring.

### Routing Table A: Language Features

| What to add                          | L2 Lexing                                                                  | L2 Parsing                                                                                                                                                 | L3 Semantics                                                                                                                                                | L3 IR                                           | L4 Execution                                        | Gates/docs                                                                                                            |
| ------------------------------------ | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **Binary operator**                  | Add a `TokenKind` to the keyword table in `lexer/state.rs` and `tokens.rs` | Add a BP constant to `pratt/precedence.rs`, an arm to `infix_info` in `led.rs`, a mapping to `parse_binary` in `led.rs`, and a `BinOp` variant to `ast.rs` | `infer_binary` in `inference/expressions.rs`                                                                                                                | The corresponding lowering in `lower/`          | `opcode.rs` + `bytecode/` + `executor/ops/arith.rs` | RFC-010 table                                                                                                         |
| **Unary operator**                   | Same as above                                                              | Add an arm to `prefix_info` in `nud.rs`, a new parse function, and a `UnOp` variant to `ast.rs`                                                            | `infer_unary`                                                                                                                                               | Same as above                                   | Same as above                                       | RFC-010 table                                                                                                         |
| **Constant-evaluation operators** ⚠️ | —                                                                          | —                                                                                                                                                          | **`types/const_data.rs:234` `BinOp` / `:321` `UnOp` are a second set of enums parallel to `ast::BinOp`/`ast::UnOp`, with no `From`/`TryFrom` between them** | `ir_gen.rs:2317` requires `use ast::BinOp as B` | —                                                   | **No gate**—see Routing Table C                                                                                       |
| **Expression form**                  | —                                                                          | Add an `Expr` variant to `ast.rs` + update `nud.rs`/`led.rs`                                                                                               | Add an infer method to `inference/expressions.rs`                                                                                                           | Add a module under `lower/exprs/`               | `opcode.rs` or reuse an existing opcode             | —                                                                                                                     |
| **Statement form**                   | —                                                                          | Add a `StmtKind` variant to `ast.rs` + update `statements/`                                                                                                | `inference/statements.rs`                                                                                                                                   | `lower/`                                        | Same as above                                       | —                                                                                                                     |
| **Type construction**                | —                                                                          | `Type` in `ast.rs` (see 03)                                                                                                                                | `types/mono.rs` + `solver.rs`                                                                                                                               | The `FunctionCode.params` type                  | Serialization                                       | RFC-011                                                                                                               |
| **Pattern form**                     | —                                                                          | `parse_pattern` + `expr_to_pattern` in `pratt/`                                                                                                            | `inference/patterns.rs`                                                                                                                                     | `compile_pattern` in `lower/`                   | `opcode.rs`                                         | RFC-010b                                                                                                              |
| **Diagnostic code**                  | —                                                                          | —                                                                                                                                                          | —                                                                                                                                                           | —                                               | —                                                   | `codes/eNxxx.rs` + `locales/zh.json` + **RFC-013 code table** (the `build.rs` gate validates all three automatically) |
| **Warning (W prefix)**               | —                                                                          | —                                                                                                                                                          | —                                                                                                                                                           | —                                               | —                                                   | Same as above; note that warnings are not included in the error count                                                 |

> **Current pain point:** adding a binary operator requires **6–7 changes** within the parser/lexer,
> plus changes to the **17 production files** downstream that use `BinOp::`. Whether an omission is
> detected depends on three categories:
>
> - **Caught by the compiler:** locations that perform an **exhaustive `match`** on `ast::BinOp`; an
>   omission directly causes a compilation failure. These make up the majority.
> - **Not caught by the compiler (the true blind spots):** the `matches!` macro, `_` catch-all arms,
>   and three kinds of cross-enum references. An omission in the first two categories appears as
>   “the new operator is handled as a different operator.” An omission in the third category
>   (`ast::BinOp` ↔ `const_data::BinOp`) is completely silent: the two types compile independently.
> - **L4 is not involved:** `backends/interpreter/` has **zero references** to `BinOp` or
>   `ast::`—the interpreter never sees the AST and recognizes only opcodes. The downstream change
>   pressure is therefore concentrated in L3 rather than L4.
>
> Goal: after the grammar-driven refactor, reduce this to 1–2 changes + 1 automatically generated
> change (see 05); resolve the cross-enum portion through common-source generation (see 05), and
> consolidate the bytecode layer’s third operator enum (`BinaryOp`/`UnaryOp`/`CompareOp`) through an
> opcode generation-time gate (see 06 F8).

### Routing Table B: Compiler Mechanisms

| What to add                                                      | Landing point                                                                                                                                                                          | Locations requiring synchronized changes                                                                                                                                                                                                                                                                                     |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compilation phase**                                            | Add a `Stage` variant to `driver/stage.rs`                                                                                                                                             | `Stage::ALL` in `driver/stage.rs`, the corresponding `Program::stages()`, `dispatch` in `driver/mod.rs` (the exhaustive match forces completion), and the CI phase-coverage test                                                                                                                                             |
| **Cross-phase checking layer** (such as ownership / termination) | Add a module under `sema/layers/`                                                                                                                                                      | Register it in the phase table + update `layers/README.md` (**the call-order table must correspond one-for-one with the call sites in `checker.rs`**) + merge its output into `Obligations`                                                                                                                                  |
| **opcode**                                                       | `middle/bytecode/opcode.rs` (currently `backends/common/opcode.rs`; migrate when the bytecode domains are consolidated, bringing all touchpoints under the single `middle/` directory) | **5 locations:** `BytecodeInstr::opcode()`, `size()`, `opcode_name()`, the decoding match for `From<BytecodeFile>`, and the `translate_*` functions in `translator.rs`. Only 2 are enforced by exhaustive compiler matches; the other 3 fail only when running a `.42` artifact. A generation-time gate is required (see 06) |
| **CLI subcommand**                                               | `Commands` enum in `main.rs`                                                                                                                                                           | The `pub` entry points in `lib.rs` (currently 7), and `match command` in `main.rs` (currently 373 lines)                                                                                                                                                                                                                     |
| **LSP capability**                                               | Add a handler under `lsp/handlers/`                                                                                                                                                    | `handle_request` in `lsp/server.rs` (currently 274 lines with 13 instances of repeated boilerplate), and the state in `lsp/world.rs`                                                                                                                                                                                         |
| **Standard-library API**                                         | `src/std/*.rs`                                                                                                                                                                         | **Automatic gates already exist:** byte-for-byte comparison by `gen_interfaces.rs` + marked-region drift detection by `gen_docs.rs`. This is the most mature pattern in the project and should be extended to the opcode and type tables                                                                                     |

### Routing Table C: Known Reverse Dependencies and Parallel Definitions to Eliminate

| Reverse dependency                                              | Location                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Elimination                                                                                                                                                                    | Owning document |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------- |
| parser → typecheck (**only 2 instances, in the same function**) | `parser/statements/declarations.rs:509` and `parser/ast.rs:930`, both of which call `typecheck::operator_interfaces::spec()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Pass operator interface specifications to the parser as data; the parser no longer `use`s typecheck                                                                            | 03              |
| Hard-coded predicate names in the parser                        | `declarations.rs:499` `let is_predicate_app = \|n: &str\| n == "Terminates" \|\| state.is_predicate_name(n);`. The comment at `:494-497` describes this as an unresolved open-set problem: “Future open-set solution: pass predicate names down to the parser, or make them a syntactically decidable form at the grammar layer”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | As above; pass the predicate set down as data                                                                                                                                  | 03              |
| **Duplication created to avoid cross-module dependencies**      | `ast.rs:847 name_used_as_type_in` and `declarations.rs:42 name_used_as_type` are **semantically equivalent but independently implemented**; the comments at `ast.rs:845-846` admit: “This is implemented independently here to avoid an internal parser cross-module dependency”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Merge them into a single implementation through 03’s probe data flow                                                                                                           | 03              |
| checker → internal `layers` functions                           | The traversal family in `checker/refinement.rs` and `revalidate_refined` / `check_return_refinement` call `layers::termination::{negate_guard, substitute_const_expr}` (originally `checker.rs:4857/4986/5246/5384`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Move them to the public API of `sema/refine/` or `sema/layers/termination`                                                                                                     | 02              |
| 3 parallel type representations                                 | `ast::Type` (`ast.rs:427-541`) / `MonoType` (`types/mono.rs`) / `ir::Type` (`ir.rs:3`, re-exported with `pub use`) + serialized `type_table`; bridged by `From<MonoType> for IrType` (`bytecode.rs:2353-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Consolidate into a single representation                                                                                                                                       | 03              |
| Hand-written synonym table                                      | `from_builtin_name` in `types/mono.rs:618-643` maintains branches such as `"Int" \| "int" \| "Int64" \| "int64" \| "i64" => Some(MonoType::Int(64))`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Delete as the type representation is consolidated                                                                                                                              | 03              |
| **3 parallel operator enums**                                   | `ast.rs:191` `pub enum BinOp` (20 variants) / `ast.rs:217` `pub enum UnOp` (4, uniquely containing `Deref`); `types/const_data.rs:234` `pub enum BinOp` (18, using `Ne` rather than `Neq`) / `:321` `pub enum UnOp` (4, uniquely containing `BitNot`); `middle/core/bytecode.rs:70` `BinaryOp` (11, with yet another naming scheme for `Rem`/`Xor`/`Sar`) + `:97` `UnaryOp` + `:106` `CompareOp`. **There is no `From` / `TryFrom` between ast ↔ const_data**, only hand-written lossy free functions in `const_eval.rs:185-216` (`Range`/`Assign`/`BitNot` and others silently return `None`), forcing 5+ files to use import aliases (`AstBinOp` / `AB` / `CEBinOp` / `ConstBinOp` / `B`), as seen in `types/eval/const_eval.rs:19,1061`, `typecheck/layers/tests/ownership.rs:19`, `typecheck/layers/tests/termination.rs:15,226`, and `middle/core/ir_gen.rs:2317` | Converge the two L2 sets through common-source generation (see 05); converge the bytecode-layer set through a generation-time opcode gate and differential testing (see 06 F8) | 03 / 05         |
| `include!`                                                      | Already eliminated (5.1: `checker.rs` now uses a real `mod semantic_tokens`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Complete                                                                                                                                                                       | —               |

## Guidance for Future Extensions

### Checklist When Adding a Language Feature

1. **Consult Routing Table A first** to determine the landing point. Do not choose files by
   intuition.
2. **Types first:** if the feature involves types, follow the single representation in 03; do not
   add a third representation.
3. **Keep gates synchronized:** diagnostic-code changes must simultaneously update `codes/eNxxx.rs`,
   `locales/zh.json`, and the **RFC-013 code table**—`build.rs` enforces consistency among all
   three.
4. **Register the phase:** if the feature introduces a new compile-time obligation, register it in
   the `Stage` enum and `Obligations`.
5. **Check reverse dependencies:** new code must not make L2 depend on L3 or L4 depend on L1/L2.
6. **Wire up tests:** a new test directory must simultaneously have a `mod` declaration in its
   parent module; otherwise, it will not run—the newly added CI check will catch this.

### When Adding a “Layer” (Such as a Future Static Check)

1. Create a module under `sema/layers/` that **does not depend on a layer with a higher number**.
2. Put its output into `Obligations`, where the orchestration layer consumes it uniformly.
3. Update `sema/layers/README.md`—**make it describe the actual layer order** (take line numbers
   from the current tree and anchor locations by function name/driver point).
4. Register it in the `Stage` enum and explicitly declare the modes in which it runs in
   `Program::stages()`.

---

## Where to Find the Maintenance Mechanism

The anti-regression gate (CI checklist, decision procedure, and three prohibitions) has moved to
[08-maintenance-mechanism.md](08-maintenance-mechanism.md).

This document answers only “where things belong”: dependency direction rules, the target directory
structure, feature routing tables, and naming conventions.
