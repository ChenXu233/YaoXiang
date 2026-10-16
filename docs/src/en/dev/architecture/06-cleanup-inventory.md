# Dead Code and Empty Design Cleanup

> **Companion design document**. This document is a companion to
> [RFC-039: Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution phase order are in the main RFC-039
> body; the positioning of each companion document is in the [directory index](index.md).

## Positioning and Scope

This document provides the YaoXiang repository's dead code and empty design inventory, determination
methods, item-by-item disposition decisions, and execution order. Cleanup targets fall into four
categories:

| Category                                                            | Scale                                                                   |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Files that never participated in compilation                        | 24 files / 2628 lines (of which 23 files / 1081 lines are true orphans) |
| Compiled but with zero call sites                                   | 1500+ lines                                                             |
| Design documents describing an architecture that never existed      | Three documents under `docs/src/dev/design/check/`                      |
| Features hung on the default entry, with zero tests and zero output | `src/repl/` 1152 lines                                                  |

**The authorization scope is "complete deletion permitted"** (2026-10-03 decision). However, this
document gives **disposition recommendations opposite to "direct deletion"** for several items — see
[Key Decisions and Rationale](#key-decisions-and-rationale) for reasoning.

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), criteria design (see `07-equivalence-oracle.md`). The
B3-recorded `layers/dispatch.rs` entire chain being unwired — its deletion action is assigned to S2.

## Current State

### Three Liability Forms of Dead Code

**Form 1: Placeholder.** Code exists, looks like it's doing something, but actually does not.

- The entire `src/frontend/core/typecheck/layers/dispatch.rs` chain (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the "runtime check insertion" mechanism from `RFC-027 §4.1`, and is
  **completely unwired**.
- `check_type_equivalence` (the `ProofContext` entry) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is only called by tests. **Note**: the
  `is_subtype` in the same file is used in production by `inference/assignment.rs:17`, so **the file
  cannot be deleted in its entirety**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler. Its `pub` surface
  consists only of `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend** — the
  `Executor` trait at `backends/mod.rs:356` currently has only one implementor (the interpreter).
  This point needs clarification, as the directory name is misleading.

**Form 2: Residue.** The upstream decision has been made, but deletion was only half-done.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) leaves 6 types: `FunctionInstance`
  (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), `CaptureVariable`. **Source is clear**: when `RFC-023`
  deprecated the "closure capture model", it stated "**approximately 850 lines of code deleted along
  with the closure model**". These 415 lines are the survivors of those 850 — **deletion was only
  half-done**. **Verification**: doing a repository-wide word-boundary search on these 6 types, the
  hit count **outside `instance.rs` is 0** (including tests). The
  `pub use passes::mono::instance::*;` at `middle/mod.rs:40` is just a glob relay and does not count
  as consumption.

**Form 3: False coverage.** This is the most dangerous form — **it makes the repository look more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only 5 `pub mod`s (`literals` / `state` / `symbols` /
`tokenizer` / `tokens`) + one `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`
(`:104-106`). **No place declares `mod tests;`**, therefore the entire `lexer/tests/` subtree — 13
files, 686 lines — **never participated in compilation**.

Among these, 7 are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
`delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totalling 19 lines, 0
assertions). **629 lines / 55 real tests have never run**:

| File | Lines | `#[test]` | | --- | --- | | `literals.rs` | 222 | 30 | | `rfc010_lexer.rs` | 167 |
12 | | `lexer_mod.rs` | 89 | 5 | | `rfc004_lexer.rs` | 81 | 5 | | `symbols.rs` | 70 | 3 | |
**Total** | **629** | **55** |

(`fstring.rs` 90 lines / 11 tests do compile, but via the `#[path]` bypass rather than `mod tests;`;
`mod.rs` 38 lines is a declaration file, no assertions.)

**The lexer layer actually runs only one file, 12 tests, all testing only the f-string curly-brace
raw protocol.**

The insidiousness of this type of problem: `cargo test` does not fail (tests that didn't run don't
fail), and CI has no test count baseline or wiring check.

### Empty Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not "designed but not built" — the documents describe **a different system that
never existed**, and frame "potential future designs" as "defects of an implemented system".

## Method and Credibility Statement

### Reachability Analysis Method

Starting from the two crate roots `src/lib.rs` / `src/main.rs`, parse the `mod` / `pub mod` /
`#[path]` declarations of all 509 `.rs` files, build a module graph and perform BFS reachability
analysis. `include!` targets are adjudicated separately (not counted in reachability).

**This method has known misjudgment patterns, and was actually triggered twice during writing:**

1. `Split-Path` returns `\` on Windows, which doesn't match forward-slash keys → once misreported
   499 orphans
2. **A submodule directory of a non-`mod.rs` file is `itself-name/` rather than
   `parent-directory/`** → `orchestrator.rs`'s `mod tests` was parsed as
   `src/frontend/module/tests/`, nearly classifying 753 lines of live tests as dead code

Therefore **every conclusion has undergone double confirmation via "actually reading the file +
actually searching for references"**, and items that did not pass secondary confirmation do not
enter this document.

### Line Count Calibration

All line counts in this document are measured with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines, systematically undercounting by about 5%** (example:
`manifesto.md` reports 658 with `-Line`, actually 893). An earlier round of analysis in this
repository gave a lower total due to this calibration; that has been corrected here.

### Known Limitations

- **Reachability analysis can only discover "unreachable", not "reachable but meaningless"**. Forms
  beyond Form 2 require manual item-by-item judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types need to be
  discovered by search; this method does not cover them.
- This document does not involve running the compiler, so all "never run" conclusions are static
  determinations.

## A. Files That Never Participated in Compilation

**Total: 24 files / 2628 lines.** Of these, `semantic_tokens.rs` belongs to Form 4 (see section F);
the remaining **23 files / 1081 lines are true orphans**.

| Path                                                          | Scale                   | Content                                                                       | Determination Basis                                                                                                                                                                 |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/tests/` **entire subtree, 13 files** | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only 5 `pub mod`s; the only test declaration at `:104-106` is `#[path]` to `tests/fstring.rs`. Directory-wide `grep 'mod tests'` only hits `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 files                        | 14 lines                | All placeholder doc comments, 0 assertions                                    | `src/frontend/pipeline.rs` (740 lines) has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration in the entire text (zero `Select-String` hits)                             |
| `src/package/template/tests/` 3 files                         | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`s**                    | `src/package/template/mod.rs` (7 lines) declares only `mod gitignore; mod main_yx;`, no `mod tests;`                                                                                |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden in a **live** directory                                                | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                     |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same                                                                          | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                        |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same                                                                          | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                |

**The last 3 items total 16 tests / 311 lines of code that have never run.** They hide inside active
directories, where the `mod.rs` looks complete, making them more deceptive than whole orphan
directories.

Within `lexer/tests/`, there are also two more files in the "declared but declaration missed" form,
but they are already counted in the 629 lines of the first row above, **not double-counted**:
`lexer_mod.rs` (89 lines / 5 tests, `lexer/tests/mod.rs` declares 11 `mod`s but no
`mod lexer_mod;`), `symbols.rs` (70 lines / 3 tests, same).

**Total real tests that have never run: 1005 lines / 78 tests**: lexer 629/55 + pratt / passes /
emitter 311/16 + package/template 65/7.

**Particularly note `precedence_inline.rs`**: it tests the `Precedence` enum + `PrecedenceContext`
in `precedence.rs` that have **zero production use** (96 lines of dead code). Even if it were wired
up, it would test a dead path.

Another item hidden within the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, 0 tests): `typecheck/tests/mod.rs` declares 23 test modules, except missing it — the file is
left with only one `use` line, pure residue. When S1 revives, it is deleted along with the wiring,
or assertions are added.

## B. Compiled but with Zero Call Sites

| #   | Location                                                                         | Scale               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --- | -------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | Repository-wide word-boundary search, hits outside `instance.rs` all 0 (including tests). Source: RFC-023 declared deletion of about 850 lines, only half deleted                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` + 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both relays). Repository-wide `TypeSystem` hits only 5 places: 2 definitions + 1 re-export + **unrelated** `lexer/symbols.rs:503 TypeSystemValidator` 2. The 4 methods have zero call sites. **Because it's `pub`, the compiler never warns**                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| B3  | `src/frontend/core/typecheck/layers/dispatch.rs` entire chain                    | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` zero calls. RFC-027 §4.1's runtime check insertion is unwired. Note it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` only called by tests. **The same file's `is_subtype` must be preserved** (production use at `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-described as "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests) — and that test file itself has never been compiled** (`pratt/tests/mod.rs` only declares `led`/`nud`/`precedence`). Therefore the two **must be deleted together**: deleting the test alone leaves 96 lines of dead code, deleting the enum alone causes test compilation failure                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`、`:120-122`、`:145-167` | about 120 lines     | `is_old_function_syntax` is called at `:719`, `skip_old_function_syntax` is called at `:725` — **they are running old-syntax rejection gates, not unreferenced dead code**. The `skip_old_function_syntax` function body (`:120-122`) has only a one-line comment `// 旧语法已移除`, but **the function consumes no tokens after being called** — this is an independent behavioral issue (whether the rejection path correctly advances the parse position), not dead code. **Disposition is to review behavior along with 05-frontend-paradigm.md (resolution D27: naturally deleted with the grammar migration), not delete directly as dead code**                                                                                                                                                                               |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | Repository-wide only match, no construction sites. The "no generation point (dead path)" comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| B9  | `Switch` opcode                                                                  | —                   | Defined in `backends/common/opcode.rs`, has variants, has `opcode()`/`size()` arms, has decode arms, **but repository-wide zero construction sites**. The interpreter `ops/control.rs:90` has an active implementation. **Explicitly registered by tests as a known dead path**: `tests/bytecode.rs:1035-1040`'s `WHITELIST: &[&str] = &["SWITCH"]` (fails if the set difference is non-empty)                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` **ignoring the `op` field**, and the decoder at `1236-1248` also only constructs `Neg`. **Via the `opcode()` mapping path it silently degrades to I64_NEG** (when `.42` is run directly, `!x` becomes `-x`; the interpreter's in-memory path `ops/arith.rs:46-49` is actually correct). Only used by interpreter `ops/arith.rs:46,49` and `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                        |
| B11 | `#[allow(dead_code)]` 4 places                                                   | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (conservative exemption for extern binding, **reasonable to keep**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction** (the only construction point is `parser/tests/ast.rs:828` test manual node), but has **12 production consumers + 2 exhaustive arms** (full list in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()`, `pratt/mod.rs:47` `expr_end_line`. Function definitions actually go through `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion is assigned to P6 (03 §5.5's 16-item change list) |

## C. Empty Designs

| #   | Object                                                                          | Scale                                      | Fact                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                   | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, `HotReloader` all have **zero repository hits**. The "known limitations" at `:46-47` self-admits that `check_incremental` internally still calls `check_files_with_diagnostics` (full path); `:46` also references busy-wait debouncing in `command.rs` — **but `command.rs` has neither `Instant` nor `recv_timeout`** (repository-wide `recv_timeout` only appears in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                   | `:37` claims that `traits/` placeholder implementations (coherence / impl_check / object_safety / resolution) are incomplete — **the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero repository hits. `:38` claims that `check_single_module` creates a separate Compiler for each file — **this function does not exist**, and only appears in a comment in `util/diagnostic/tests/mod_tests.rs:248`                                                                                                 |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                   | `:14` claims "watch mode only re-checks affected files", `:27` claims "check only does the first two steps". Actual: `check_project` also does vendor consistency, project role determination, dead code analysis, W1006; the `Check` subcommand in `main.rs` **does not have a `--watch` flag** (each `#[arg]` declaration has been individually verified)                                                                                                                                                                                       |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines of design / **0 lines of code** | The AOT hits at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell. `:861-876` left itself an all `- [ ]` implementation checklist, but the status is marked `accepted`                                                                                                                                                                                                                                                                          |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**  | `028-jit-compiler.md:99-126` requires 6 files at `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **none of which exist**. Status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                                                                  |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                               | **Physical evidence**: the filenames precisely match `ModuleCache` and incremental scheduling in the C1 design. **Someone built test skeletons according to the design, found that the pipeline had no corresponding implementation, and stopped**                                                                                                                                                                                                                                                                                                |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                          | The `//! §6.1: CheckSession incremental check` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` reference this **gitignored planning document** (`.gitignore` contains `superpowers/`). These § references are **archaeological layer references to a deprecated API**                                                                                                                                                                                                                                  |

### Real Implementation Location of Cross-File Analysis

The cross-file analysis capability described in C1-C3 **really exists**, just at a completely
different path:

| Component           | Location                              | Lines          |
| ------------------- | ------------------------------------- | -------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439            |
| Resolver            | `src/frontend/module/resolver.rs`     | 195            |
| Role classification | `src/frontend/module/roles.rs`        | 388            |
| Consistency check   | `src/frontend/module/consistency.rs`  | 206            |
| Orchestrator        | `src/frontend/module/orchestrator.rs` | 1528           |
| **Total**           |                                       | **about 2670** |

Therefore, the disposition of C1-C3 is **not deleting the capability, but deleting misleading
documentation**, and rewriting the boundary table in `check/index.md` to point to the actual
architecture at `src/frontend/module/`.

### Clarification Needed for Disposition of RFC-019

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already in the `deprecated/` directory**,
and the residual check result is **clean** (`homoicon` has zero repository hits, and the document
ends with an honest "if restarted in the future, start from proof/prototype experiments, not from
the RFC" conclusion). **It needs no additional disposition** — it's in the authorization list, but
verification confirms it's already at terminal state.

## D. Feature-Level Dead Code: REPL

`src/repl/` 4 files **1152 lines**, **zero `#[cfg(test)]`, zero `#[test]`**. And `src/main.rs:440`
is `args.command.unwrap_or(Commands::Repl)` — **executing `yaoxiang` directly without arguments
enters the REPL**.

| #   | Fact                                            | Evidence                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed**    | The entire `src/repl/` only appears 2 times: the definition at `eval.rs:23`, and a **match arm** at `mod.rs:180`. Therefore `Repl::format_value` (`mod.rs:493-502`) is dead code, **the REPL does not echo any evaluation result**                                                                                                                                                           |
| D2  | **No cross-round state**                        | `extract_definitions` (`eval.rs:345-377`) only calls `define_variable` (storing a **type string**), never calls `define_var` (storing `RuntimeValue`); `wrap_code` also doesn't inject historical definitions. The `VariableInfo::Value` branch at `eval.rs:23` is unreachable. `REPLContext.variables` purely serves completion, not evaluation                                             |
| D3  | **Wrapping syntax suspected of being outdated** | `eval.rs:314`/`316` produces `main() -> () = () => { code }` (the `name() -> Ret` old-style function declaration header), while the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`, all the canonical forms in RFC-007:94 are `name: (a: Type) -> Ret`). **Compiler not actually run to verify; if true, every REPL evaluation fails to compile** |
| D4  | `:debug` is a stub                              | `mod.rs:454-466` just prints two lines of hints and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                                |
| D5  | `:history` is a stub                            | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                    |
| D6  | Duplicates `lib.rs`                             | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent "automatic main wrapping" logics**, and the wrapping forms are different                                                                                                                                                                                                                                          |

## E. Local Cleanup Items

| #   | Location                                                                                           | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | -------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                         | Root-level empty directory tree. Almost certainly a place where the string `"undefined"` was concatenated as a path somewhere. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                          |
| E2  | `src/lib.rs:568`、`:582`                                                                           | `dump_type_detail` / `dump_const_detail` each have a `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, the panic path is predictable. Should be changed to a meaningful fallback output                                                                                                                                                                                                                                                                                                                                      |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                          | `check_vendor_lock_consistency` and `check_workspace_consistency` have **about 60 lines of near-duplication**, differing only in the dependency source and the path-dep skip. **Merge**                                                                                                                                                                                                                                                                                                                                                                     |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`             | Old function syntax detection and rejection, about 120 lines. Among them, `skip_old_function_syntax`(`:120-122`) function body has only a one-line comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                   | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                            | The "Tooling Design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, `check/incremental-checking`. **If executing the document deletion of C1-C3, this must be cleaned up synchronously**, otherwise the documentation site will have 404 links                                                                                                                                                                                                                                                                            |
| E7  | Dual `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | 5 variants with the same names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` bit-width is inconsistent (f32 × f64)**; no `From` / `TryFrom`, conversions are hand-written and scattered at call sites (`const_eval.rs` / `ir_gen.rs`). `kind()` at `const_data.rs:44` is a live `todo!()` for `LibraryRef` / `ExternRef`. **Disposition: merge into one + explicit conversion + change `todo!()` to fallback output, assigned to P10 (E category local cleanup)**. Registered in 2026-10-06 P0 gate (criterion A, J=0.56)          |
| E8  | `Literal` downstream mapping synonymous duplication (prohibition three A)                          | Same `Literal → MonoType` arm in ≥3 places (`inference/expressions.rs`, `checker.rs`, `Literal::Int(_) => MonoType::Int(64)` in `inference/patterns.rs`, etc.); `Literal → ir::ConstValue` (19 places hand-written in `ir_gen.rs`) and `Literal → const_data::ConstValue` (4 places hand-written in `const_eval.rs`), no shared conversion point. **Disposition: typecheck side 3 places extracted to shared layer, assigned to P6; `ir_gen` / `const_eval` side consolidated along with E7's `ConstValue` merge in P10**. Registered in 2026-10-06 P0 gate |

### Not Dead Code, but Same Category of Problem: Three Hard-Coded Drops

The `impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1400 lines) has 3
hard-coded drops:

| Line   | Dropped Content                  | Consequence                                        |
| ------ | -------------------------------- | -------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | upvalue metadata lost                              |
| `2315` | `exception_handlers: Vec::new()` | **`.42` output loses exception table**             |
| `2341` | `globals: Vec::new()`            | **`.42` output loses global variable information** |

**These three are not dead code, they are data loss defects** — `.42` output is complete only under
the in-memory path. **Disposition is different from dead code**: ~~should be tracked as a separate
issue~~ **all already collected into P7 batch e (resolution D52, 2026-10-05), no independent issue
retained** — `2315` (exception table) and `2341` (global variables) are language core semantic
output completeness, not "deferrable" items. All three fixed together, `.42`'s `VERSION` goes from 4
to 5 in one shot (the format header already has `MAGIC` + `VERSION` fields,
`codegen/bytecode.rs:14-16`). Listed here only to avoid being mistakenly cleaned up as "dead code".

## F. Don't Delete, but Mark or Decide

| #   | Object                                                                                     | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (1547 lines)                      | **Not dead code**, it's spliced in via `include!` by `checker.rs:5618` — **the only `include!` in the entire repository**. This file **is not declared as any module** (`grep 'mod semantic_tokens'` zero hits repository-wide), the first line directly is `impl TypeChecker {`, no `use` header of its own, only one file in the `checker/` directory. Consequence: no module identity, visibility isolation fails, rust-analyzer jump and symbol search fail, the toolchain counts it as part of `checker.rs`. **The real `checker` module size is 5618+1547 ≈ 7165 lines**. **Fix: change to a real `mod`** (construction steps in [09](09-execution-wbs.md) §P5 5.1), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| F2  | `#[cfg(target_arch = "wasm32")]` branches in 27 files                                      | **Not dead code.** The wasm target is carried by a **separate shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, `:17` depends on the main crate; the root `Cargo.toml:3`'s `exclude = ["wasm", ...]` precisely indicates it as a separate crate. Only looking at root `Cargo.toml:31`'s `crate-type = ["rlib"]` would misjudge these branches as defensive dead code — **that inference does not hold**. CI has 4 build places (`_build-wasm.yml:75`, called by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`). **These 27 files' branches are load-bearing** — they determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm target (`lib.rs:139/153/170` gates off `run_file` / `run_project` / `build_bytecode` as a whole). **The real cleanup target** is the unreachable part — `wasm/src/lib.rs` only calls `Compiler::new` / `compile_with_source` / `std::io::wasm_output` / `CodegenContext` / `Interpreter`. Which of orchestrator.rs's 12 wasm attributes are unreachable in the playground scenario needs to be judged one by one (resolution D35: delete unreachable, keep reachable, assigned to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                             | Has only a "Status" column (document status), **no "Implementation Status" column** — it **cannot express** the error "RFC marked complete but code isn't". Repository-wide only 5/52 RFCs have an `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **26 accepted RFCs have no implementation status record at all**, including 009, 009a, 013, 018, 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| F4  | `src/frontend/core/typecheck/layers/README.md:1-16`                                        | Declares layer order equivalence → ownership → termination → predicate, **exactly opposite to actual execution order** (actually predicate → termination → ownership), and **no short-circuit at all**. The README describes an unbuilt (or partially abandoned) intended architecture                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| F5  | `docs/src/rfc/index.md`                                                                    | `:79` puts RFC-026a in the "Accepted RFCs" table but the status column says "RFCs Under Review"; the "RFCs Under Review" subsection at `:48-49` only lists 032 and 027a, missing 026a. Inconsistent with the 3 review entries in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F6  | `tools/cargo-dist/` (`dist.exe` 21,108,736 bytes) and `benches/shootout/out/` (8 binaries) | **Neither is tracked by git** (`git ls-files` returns empty), excluded by `.gitignore`. **Not a problem**, just recorded to prevent subsequent audits from misjudging as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                       | `extract_code_from_entry_line` is a **line-start prefix matcher** (`trim_start().strip_prefix("(\"")` takes up to the next quote), depends on the `("E1001", ...)` tuple syntax, **not a parser**. Its architecture (`parse` + `validate` + build.rs gate + `--fix` healing) is worth reusing for the opcode table, but **the extractor needs to be rewritten according to opcode form (about 30 lines)**. Belongs to 06-cleanup-inventory.md's follow-up work or a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode fact is expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / decode match (`bytecode.rs:978-2302`) / 48 `translate_*` (`translator.rs:676-1577`). **The compiler only enforces 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + each `ops/*.rs` family function's exhaustive match). Missing one of `opcode()` / `size()` / decode arm / encoder **only blows up when running `.42` output**. The `size()` table (`649-803`) comments are inconsistent with actual encoding (`bytecode.rs:2181-2182`), and **no systematic cross-check test exists** (existing size tests only cover Nop/Mov/Borrow/Release 4). Operator semantics here also go through a third parallel enum (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`, `Rem`/`Xor`/`Sar` another set of names), consolidated along with the opcode generation-time gate. The vocabulary target location is `middle/bytecode/opcode.rs` (migrated along with the bytecode domain merge, eliminating L3→L4 reverse, see 01)                |

## G. Gate and Criterion Discovery Registration (registered from 2026-10-06 with each phase's construction)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, prohibition one A/B/C/D) first
run on 2026-10-06. Following the division of labor "gate only reports suspected, human adjudicates"
([08](08-maintenance-mechanism.md) §Known Limitations), this section registers adjudication results
and disposition assignment. Unexpanded hits (criterion A violation layer 25 pairs, criterion B all
suspected items) are reviewed pair-by-pair as corresponding phase construction proceeds.

| #   | Object                                                                                                                     | Adjudication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Disposition Assignment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                              | **Legitimate structure, not parallel representation**: value / type label pair, maintained 1:1 by exhaustive `value_type()` (`value.rs:326-369`), missing an arm means compilation failure. But `ValueType`'s external consumption is essentially limited to `{:?}` formatting in std/os/fs/net error paths and `executor.rs:410` always-setting `Void`, 21 variants far exceed actual need                                                                                                                                                                                                                                                                                                                                                                                                                                                        | P10 S5 slimming candidate: downgrade to `pub(crate)` and evaluate trimming; during S5 construction, judge together with the necessity of `AsyncValue.value_type` field (`value.rs:217`)                                                                                                                                                                                                                                                                                                                                                                                                    |
| G2  | Dual `ConstValue` (criterion A, J=0.56)                                                                                    | **True positive**, includes f32/f64 bit-width inconsistency and live `todo!()` in `kind()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Already assigned to E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| G3  | Type name string dispatch 11 places (criterion D true positive)                                                            | **True positive, same root cause**: no single authoritative identification point for built-in container names. Inventory: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), large table at `eval/const_eval.rs:498-511` (contains `"Uint"` dead arm), `to_type_id` at `middle/passes/codegen/bytecode.rs:744-756` (`"Set" => 25` contradicts decision 4 of #300), `"Clone" \| "Dup"` at `trait_data.rs:187`                                                                                                                                                                                                                        | **P6 extended verification item**: these points are on the typecheck / eval / middle side, not within the parser data flow coverage of 03 §5.4; during P6 construction, follow the same principle of 03 §5.3 (`NameKind` fixed once, match arms no longer do string comparison). The copy table has been synchronously corrected to seven places (03 §2.3)                                                                                                                                                                                                                                 |
| G4  | Criterion D false positives 4 places                                                                                       | **Legitimate lookup / dispatch table, not bridging**: `package/build/requirements.rs:17-25` tool installation guide, `package/vendor/mod.rs:134-139` build result display name, `repl/mod.rs:241-396` REPL command aliases (8 arms, user interface aliases are legitimate UX), `util/diagnostic/mod.rs:462-468` `parse_runtime_mode` (with comment on historical aliases, a single boundary point)                                                                                                                                                                                                                                                                                                                                                                                                                                                 | When P9 transitions to hard, add `// reason:` exemption place by place; exemption count is reviewed as the gate report is processed                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| G5  | `ReleasePlan.drops` span key mismatch (P2 2.4.4 criterion actual test, not P0 hit)                                         | **True positive, contract defect — D20 prophecy empirically confirmed**: ownership keys by "last used **expression** span" (`ownership.rs:2574 build_release_plan`), `ir_gen.rs:1956` looks up by "**statement** span", expression-level keys never hit — full corpus actual test **168 files / 373 keys** not consumed, Drop silently lost (per-case evidence in `src/middle/core/tests/release_plan_spans.rs` file header)                                                                                                                                                                                                                                                                                                                                                                                                                       | **P4/D20** (Span key → PlanId, behavioral surface about 168 files, must wait for C2 criterion and corpus diff to be in place); criterion `test_release_plan_spans_consumed` stays in red state (`#[ignore]`), turns green when D20 lands and is removed                                                                                                                                                                                                                                                                                                                                    |
| G6  | Three IR metadata defects (P2 2.1.2 criterion `verify_loose` corpus run green process actual test, D38 prophecy fulfilled) | **True positive ×3, all fixed with 2.1** (corpus diff baseline self-compare zero diff proves behavioral equivalence): ① Method IR's locals table truncated to parameter count (`ir_gen.rs` original `take_cur_locals(param_types.len())`) — local_count underestimated, runtime relies on `Frame::set_slot` out-of-bounds resize (`frames.rs:103-105`) silent tolerance; ② unannotated parameters in lambda / curry middle layer / function path are dropped by `filter_map` (three places), params length out of sync with frame layout — align with method path existing convention to fill `Int(64)` placeholder; ③ match result slot relies on frame implicit Void pre-initialization (the fallback point for the last arm's guard failure has a path with no definition on the CFG) — explicit Void pre-set at `generate_match_expr_ir` entry | Already fixed (same batch as 2.1). **Determination surface narrowing registration** (verify.rs file header is authoritative): temp slot dominance not checked (Void pre-initialization is frame semantics, ir_gen intentionally depends on); type-consistent trusted source narrowed to parameter slot signature + constants (ir_gen never writes `LocalSlot.ty`, entire table is `Int(64)` placeholder — complete determination surface goes to P7 value table 04 §262); `init_locals` is #368 debug perspective table not allocation upper bound, init's Local out-of-bounds not checked |

## H. Giant Function Inventory (pre-release audit calibration, from 2026-10-16)

> Calibration (set at 2026-10-16 `/pre-release-audit`): function span > 150 lines is the **initial
> screening threshold**, not an "report each one" command — listing them all would turn the
> inventory into a line count pile, drowning the new signals of this round in the existing ones.
> Report entries must contain three numbers: **host line count / this insertion line count / line
> count before insertion**. Two categories: **A. existing giant function has branch inserted** (>150
> lines and this insertion >50 lines); **B. new large function this round** (almost entirely new
> and >100 lines). The line numbers in the table below are from audit revision `1935aab53`
> (`v0.8.4..1935aab53` range).

### A. Existing Giant Function Has Branch Inserted

| Function                | Location                                                    | Host Lines | This Insertion | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | -------------- | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1594       | 358            | 1236             |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171            | 642              |

### B. New Large Functions This Round

| Function                     | Location                                            | Lines | This Round New |
| ---------------------------- | --------------------------------------------------- | ----- | -------------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177            |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122            |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130            |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107            |

**Existing giant functions not in the inventory** (this round insertion ≤ 4 lines, register only, do
not dispose): `infer_expr` (`expressions.rs:2999`, 955 lines), `collect_function_signature`
(`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`, 441 lines),
`generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), etc.

### Disposition Orientation

- **Category A**: Dissolved along with the subsequent split tasks of `infer_call_expr` /
  `generate_call_expr_ir` — these are the heaviest hosts in the repository; the named argument
  refactoring this round inserted 358 lines into the former.
- **Category B**: `push_joined` is a **local** helper for `Task` worklist-style type formatting (its
  host is the 180 lines of new code), and has **different responsibilities from
  `mono.rs:123 increment_level_string` (174 lines, does level name rewriting), does not constitute
  duplication, do not merge**; `consumer_stage_skipped` and `local_count_of` can be independently
  split.

## Key Decisions and Rationale

The authorization is "complete deletion permitted", but this document gives **different
recommendations** for several items:

| Category                                                                | Disposition                                        | Rationale                                                                                                                                                                                                                                                                                                               |
| ----------------------------------------------------------------------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions, no implementation) | **Delete**                                         | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty function), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                            |
| **Empty design documents**                                              | **Delete or rewrite**                              | Delete C1, C2 then rewrite to point to actual implementation; rewrite C3's boundary table; **must synchronously clean up `config.js:259-268`** (E6)                                                                                                                                                                     |
| **RFCs with status not matching implementation**                        | **Change status**                                  | C4 (RFC-018) move back to `draft` or add `impl_status: 'not-started'`; C5 (RFC-028) add `impl_status: 'not-started'`. **Don't delete** — the design itself may have value, the problem is status mark distortion                                                                                                        |
| **Orphan tests with real assertions**                                   | **Revive, don't delete**                           | Total **1005 lines / 78 tests**: lexer 629 lines / 55 + 311 lines / 16 hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7. **Revival cost is one line of `mod` declaration, value is letting CI regain this coverage**. Deleting them is a net loss |
| **`pub` items with zero calls but semantically important**              | **Downgrade visibility, don't delete**             | B2 (`TypeSystem`), B4 (`check_type_equivalence`) — move to `pub(crate)` to let the compiler identify, or supplement implementation. **B4's `is_subtype` must be preserved**                                                                                                                                             |
| **Opcodes / instructions that produce no value**                        | **Supplement construction point or delete opcode** | B8 (`TailCall`), B9 (`Switch`, opcode round-trip test has been explicitly registered as `WHITELIST=["SWITCH"]`), B10 (`UnaryOp::Not` silently degrades). Three options, cannot maintain status quo; D32/D33/D34 have already adjudicated to delete, handled along with `.42` version upgrade                            |
| **Data loss defects**                                                   | ~~independent issue~~ **P7 batch e (D52)**         | The three hard-coded drops in section E — not dead code                                                                                                                                                                                                                                                                 |
| **No module identity but effective**                                    | **Transform, don't delete**                        | F1 (`semantic_tokens.rs`) change `include!` → real `mod` (see `02`)                                                                                                                                                                                                                                                     |
| **Needs decision**                                                      | **List as open issues**                            | F2's **unreachable wasm branch** cleanup scope, layer order correction will expose new diagnostics                                                                                                                                                                                                                      |

### Unadopted Directions

- **All processed as "complete deletion", orphan tests also deleted** — 1005 lines of tests with
  real assertions are an asset. Deletion cost is one line of `mod` declaration, benefit is CI
  regaining coverage. The authorization is "can delete", not "should delete".
- **Only add gates, don't touch code** — gates can prevent regression, but B1's 415 lines, the
  dispatch chain, and 96 lines of dead staircase **already exist**; gates will only freeze them at
  the baseline.
- **Keep `accepted` status of RFC-018 / RFC-028, only supplement documentation** —
  `rfc/index.md:131` defines "accepted = enter implementation phase", readers will reasonably assume
  implementation exists. `TRACKING.md` already has precedent for the `impl_status` field (5 RFCs use
  it), cost of supplementing the field is far lower than the cost of status distortion.

## Implementation Points

**This document does not need equivalence criteria** (belongs to
[C6 pure deletion category](07-equivalence-oracle.md)), but **still needs item-by-item confirmation
of no references**. In RFC-039's global phase sequence, S1 corresponds to P1, S2-S6 correspond to
P10.

| Phase | Content                                                                                                                              | Acceptance                                                                                                                                                                                                                                        |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphan tests with assertions (**1005 lines / 78 tests**)                                                                      | `cargo test` shows test count increase in lexer / pratt / passes / emitter / template five modules; **expected to expose real defects** (`literals.rs` overflow path, `\x`/`\u` illegal escape previously never tested), need to reserve fix time |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 closed with 05/C6/E1)                                                                 | `cargo test` passes; `cargo clippy -D warnings` no new warnings                                                                                                                                                                                   |
| S3    | Empty design document disposition (C1-C3 + E6 synchronously clean up `config.js`)                                                    | Documentation site build has no 404; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                      |
| S4    | RFC status correction (C4/C5/F3 — add "Implementation Status" column to `TRACKING.md`)                                               | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` will be auto-regenerated, **don't manually edit**)                                                                                                                                   |
| S5    | `pub` items downgrade visibility (B2/B4)                                                                                             | Compiler can point out real dead code                                                                                                                                                                                                             |
| S6    | Opcode produces no value item decision lands (B8/B9/B10/F8, execute per D32/D33/D34)                                                 | Delete `Switch` / `TailCall` dead paths; B10's `opcode()` distinguishes `op` field; F8 gate lands                                                                                                                                                 |
| S7    | `include!` transformation (F1, construction steps in [09](09-execution-wbs.md) §P5 5.1) + layer order correction (F4, to P4's 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                        |

**S1 must be done first, and defect fix time must be reserved.** Its value is not "cleanup", but
**letting CI regain 1005 lines / 78 tests that have never run** — before that, any judgment about
test coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners of `literals.rs` have 4 isomorphic
  "continue consuming after overflow without error" branches, the entire `scan_leading_dot`
  function, the `\x`/`\u` illegal escape path — all have never been tested. After reviving the
  tests, code will most likely need to be fixed, which will delay subsequent refactoring.
- **B9/B10 involve language behavior decisions**. Whether to "supplement construction points" or
  "delete opcodes" for `Switch` and `UnaryOp::Not` needs to consider whether they belong to the
  surface depended on by released corpora.
- **After C1-C3 deletion, the capability documentation for cross-file analysis will be briefly
  missing**, until `check/cross-file-analysis.md` is rewritten to point to the actual architecture
  of `src/frontend/module/`.
- **Reachability analysis can only discover "unreachable", not "reachable but meaningless"**.
  Judgments beyond Form 2 must be done manually item by item, and **this list does not constitute
  exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  cannot be automatically discovered by the compiler and can only be found by search.
- **Deleting RFC-023's half-done residue** (B1) can fulfill the decision "closures have been removed
  from the language"; **correcting status distortion** (C4/C5/F3) can restore the reference value of
  `TRACKING.md`.

> **All open issues originally listed in this section have been adjudicated.** Item-by-item
> decisions are in [RFC-039 Resolution Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  source decision for B1 residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — B3's unwired `§4.1` runtime check insertion
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — formal carrier of C1/C6
- [RFC-039 Compiler Feature Routing Directory Design](../../rfc/accepted/039-compiler-architecture.md)
  — upper-level master
- [Refactoring Equivalence Criteria](07-equivalence-oracle.md) — criteria requirements for C6
  category
- `src/frontend/core/typecheck/layers/README.md` — F4 layer order declaration distortion
- `build.rs:19-55` — example of generation-time gate in this repository
