# Dead Code and Phantom Design Cleanup

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance-criterion grading, and execution phase sequence are in the RFC-039
> main text; the positioning of each companion document is in the [directory index](index.md).

## Positioning and Scope

This document provides the YaoXiang repository's inventory of dead code and phantom designs, the
determination methodology, the per-item disposition decisions, and the execution order. Four
categories of cleanup targets:

| Category                                                         | Scale                                                                   |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Files that never participated in compilation                     | 24 files / 2628 lines (of which 23 files / 1081 lines are real orphans) |
| Compiled but zero-call dead code                                 | 1500+ lines                                                             |
| Design documents describing architectures that never existed     | Three files in `docs/src/dev/design/check/`                             |
| Features hung on the default entry, with zero tests, zero output | `src/repl/` 1152 lines                                                  |

**The authorized scope is "permitted to be completely deleted"** (2026-10-03 decision). But this
document gives **disposition recommendations opposite to "direct deletion"** for several items — see
the [Key Decisions and Rationale](#key-decisions-and-rationale) section for reasons.

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), criterion design (see `07-equivalence-oracle.md`). The
entire chain of `layers/dispatch.rs` recorded under B3 is unwired, and its deletion action is placed
under S2.

## Current State

### Three Liability Forms of Dead Code

**Form 1: Placeholder.** Code exists, looks like it's doing something, but doesn't actually do it.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero calls** in the
  compiler. It is the "runtime check insertion" mechanism designed in `RFC-027 §4.1`, and is
  **completely unwired**.
- `check_type_equivalence` in `src/frontend/core/typecheck/layers/equivalence.rs:216` (the
  `ProofContext` entry) is only called by tests. **Note** that `is_subtype` in the same file is used
  in production at `inference/assignment.rs:17`, so **the entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler; its `pub` surface
  contains only `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend** — the
  `Executor` trait in `backends/mod.rs:356` currently has only one implementer (the interpreter).
  This point needs to be clarified because the directory name is misleading.

**Form 2: Residual.** Upstream decisions have been made, but deletion was only half-done.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) retains 6 types: `FunctionInstance`
  (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), `CaptureVariable`. **Source is clear**: when `RFC-023`
  deprecated the "closure capture model", it stated that "**approximately 850 lines of code will be
  deleted along with the closure model**". These 415 lines are the surviving remnants of those 850
  lines — **deletion was only half-done**. **Verification**: doing a whole-repo word-boundary search
  for these 6 types, the hit count outside `instance.rs` is **all zero** (including tests). The
  `pub use passes::mono::instance::*;` in `middle/mod.rs:40` is just a glob re-export, which does
  not constitute consumption.

**Form 3: False coverage.** This is the most dangerous form — **it makes the repository look more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` only has 5 `pub mod`s (`literals` / `state` / `symbols` /
`tokenizer` / `tokens`) + one `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`
(`:104-106`). **Nowhere declares `mod tests;`**, so the entire subtree of 13 files / 686 lines in
`lexer/tests/` **never participated in compilation**.

Among them, 7 are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
`delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totaling 19 lines, 0
assertions). **629 lines / 55 real tests have never run**:

| File | Lines | `#[test]` | | --- | --- | | `literals.rs` | 222 | 30 | | `rfc010_lexer.rs` | 167 |
12 | | `lexer_mod.rs` | 89 | 5 | | `rfc004_lexer.rs` | 81 | 5 | | `symbols.rs` | 70 | 3 | |
**Total** | **629** | **55** |

(`fstring.rs` 90 lines / 11 tests do compile, but via the `#[path]` bypass rather than `mod tests;`;
`mod.rs` 38 lines is a declaration file, with no assertions.)

**At the lexer layer, only one file is actually running, with 12 tests, and they only test the
f-string brace raw protocol.**

The insidiousness of this kind of problem is that `cargo test` will not fail (tests that didn't run
don't fail), and CI has no test-count baseline or wiring checks.

### Phantom Designs

The three documents under `docs/src/dev/design/check/` describe a set of architectures that **never
existed**. This is not "designed but not done" — **the documents describe another system that never
existed**, and write "possible future designs" as "defects of the implemented system".

## Methodology and Credibility Statement

### Reachability Analysis Method

Starting from the two crate roots `src/lib.rs` / `src/main.rs`, parse all 509 `.rs` files' `mod` /
`pub mod` / `#[path]` declarations, build a module graph and perform BFS reachability analysis.
`include!` targets are judged separately (not counted in reachability).

**This method has known misjudgment patterns, which were actually triggered twice during writing:**

1. `Split-Path` returns `\` on Windows, which doesn't match forward-slash keys → at one point
   misreported 499 orphans
2. **The submodule directory of a non-`mod.rs` file is `self-name/` rather than `parent-dir/`** →
   the `mod tests` in `orchestrator.rs` was parsed as `src/frontend/module/tests/`, almost
   classifying 753 lines of live tests as dead code

Therefore, **every conclusion has been double-confirmed by "actually reading the file + actually
searching for references"**, and items that did not pass secondary confirmation do not enter this
document.

### Line Count Convention

All line counts in this document are counted by `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines, systematically undercounting by about 5%** (e.g.,
`manifesto.md` reports 658 with `-Line`, actually 893). An earlier analysis round in this repository
gave an undercounted total due to this convention, which is corrected here.

### Known Limitations

- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Forms
  other than 2 require item-by-item human judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types need to be
  found by search, and this method does not cover them.
- This document does not involve running the compiler, so all "never run" conclusions are static
  determinations.

## A. Files That Never Participated in Compilation

**Total 24 files / 2628 lines.** Among them, `semantic_tokens.rs` belongs to Form 4 (see section F),
and the remaining **23 files / 1081 lines are real orphans**.

| Path                                                                   | Scale                   | Content                                                                       | Basis for Determination                                                                                                                                                                   |
| ---------------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **The entire subtree of 13 files in `src/frontend/core/lexer/tests/`** | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` only declares 5 `pub mod`s; `:104-106` is the only test declaration, `#[path]` to `tests/fstring.rs`. `grep 'mod tests'` in the whole directory only hits `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 files                                 | 14 lines                | All placeholder documentation comments, 0 assertions                          | `src/frontend/pipeline.rs` (740 lines) has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration in the entire file (zero `Select-String` hits)                                   |
| `src/package/template/tests/` 3 files                                  | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`s**                    | `src/package/template/mod.rs` (7 lines) only declares `mod gitignore; mod main_yx;`, no `mod tests;`                                                                                      |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`            | 95 lines / **6 tests**  | Hidden inside a **live** directory                                            | `pratt/tests/mod.rs` only declares `led` / `nud` / `precedence`                                                                                                                           |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs`          | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` only declares `dead_code` / `overload`                                                                                                                              |
| `src/util/diagnostic/emitter/tests/json.rs`                            | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` only declares `ansi` / `text`                                                                                                                                      |

**The last 3 items total 16 tests / 311 lines of code that have never run.** They are hidden inside
active directories, where `mod.rs` looks complete, so they are more deceptive than whole orphan
directories.

Inside `lexer/tests/`, two other files belong to the "declared but declaration missing" form, but
they are already counted in the 629 lines of the first row above and **not double-counted**:
`lexer_mod.rs` (89 lines / 5 tests, `lexer/tests/mod.rs` declares 11 `mod`s but no
`mod lexer_mod;`), `symbols.rs` (70 lines / 3 tests, same situation).

**The total of real tests that have never run is 1005 lines / 78**: lexer 629/55 + pratt / passes /
emitter 311/16 + package/template 65/7.

**Special attention to `precedence_inline.rs`**: it tests the **`Precedence` enum +
`PrecedenceContext` (96 lines of dead code) in `precedence.rs` that are zero-used in production**.
Even if it gets wired up, what it tests is a dead path.

Another hidden one in the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, 0 tests): `typecheck/tests/mod.rs` declares 23 test modules, but misses this one — the file
has only one line of `use` left, which is pure debris. When S1 revives the tests, this one will be
deleted along with the wiring or get assertions added.

## B. Compiled but Zero-Call Dead Code

| #   | Location                                                                         | Scale               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | -------------------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | Whole-repo word-boundary search, hits outside `instance.rs` are all 0 (including tests). Source: RFC-023 declared deletion of about 850 lines, only half was deleted                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` + 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both are re-exports). Whole-repo `TypeSystem` hits are only 5: 2 definitions + 1 re-export + **unrelated** `lexer/symbols.rs:503 TypeSystemValidator` 2. 4 methods have zero call sites. **Because it's `pub`, the compiler will never warn**                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| B3  | The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs`             | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` zero calls. The runtime check insertion of RFC-027 §4.1 is unwired. Note that it is declared in `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` is only called by tests. **`is_subtype` in the same file must be kept** (used in production at `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-described as "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external reference is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests) — and that test file itself has never been compiled** (`pratt/tests/mod.rs` only declares `led`/`nud`/`precedence`). Therefore the two **must be deleted together**: deleting only the test leaves 96 lines of dead code, deleting only the enum causes the test to fail to compile                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, `skip_old_function_syntax` is called at `:725` — **they are running old-syntax rejection gates, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) has only a single-line comment `// Old syntax has been removed`, but **the function does not consume any tokens after being called** — this is an independent behavioral issue (whether the rejection path correctly advances the parse position), not dead code. **The disposition is to review the behavior together with 05-frontend-paradigm.md (Decision D27: naturally deleted along with grammar migration), not to be deleted directly as dead code**                                                                                                                                                         |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | Whole repo only has match, no construction site. The comment "no generation site (dead path)" in `bytecode.rs:2156` is **accurate** (encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B9  | `Switch` opcode                                                                  | —                   | `backends/common/opcode.rs` is already defined, has variants, has `opcode()`/`size()` arms, has decoding arms, **but zero construction sites repo-wide**. The interpreter has an active implementation at `ops/control.rs:90`. **Has been explicitly registered as a known dead path by tests**: `WHITELIST: &[&str] = &["SWITCH"]` at `tests/bytecode.rs:1035-1040` (fails if the set difference is non-empty)                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` to **ignore the `op` field**, and the decoder at `1236-1248` only constructs `Neg`. **Silently degrades to I64_NEG via the `opcode()` mapping path** (when running `.42` directly, `!x` becomes `-x`; the interpreter in-memory path `ops/arith.rs:46-49` is actually correct). Only used by the interpreter `ops/arith.rs:46,49` and `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                                             |
| B11 | 4 occurrences of `#[allow(dead_code)]`                                           | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (conservative exemption for extern binding, **reasonable to keep**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction** (the only construction site is `parser/tests/ast.rs:828` where tests manually build the node), but has **12 production consumers + 2 exhaustive arms** (full list in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()`, `pratt/mod.rs:47` `expr_end_line`. Function definitions actually go through `Expr::Lambda`(`nud.rs:425-429`) + `StmtKind::Assign`(`declarations.rs:462-479`). Deletion belongs to P6 (the 16-item change list in 03 §5.5) |

## C. Phantom Designs

| #   | Object                                                                          | Scale                                      | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                   | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, `HotReloader` have **zero hits repo-wide**. The "known limitations" at `:46-47` self-admits that `check_incremental` internally still calls `check_files_with_diagnostics` (the full-quantity path); `:46` also references the busy-wait debounce in `command.rs` — **neither `Instant` nor `recv_timeout` exists in `command.rs`** (whole-repo `recv_timeout` only appears in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                   | `:37` claims that the `traits/` placeholder implementations (coherence / impl_check / object_safety / resolution) are not finished — **the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero hits repo-wide. `:38` claims that `check_single_module` creates an independent Compiler for each file — **this function does not exist**, it only appears in a comment in `util/diagnostic/tests/mod_tests.rs:248`                                                                                                |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                   | `:14` claims "watch mode only re-checks affected files", `:27` claims "check only does the first two steps". Actual: `check_project` also does vendor consistency, project role determination, dead code analysis, W1006; the `Check` subcommand in `main.rs` **does not have a `--watch` flag** (every `#[arg]` declaration has been checked one by one)                                                                                                                                                                                                |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines of design / **0 lines of code** | The AOT hits in `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell. `:861-876` even left a checklist full of `- [ ]`, but the status is marked `accepted`                                                                                                                                                                                                                                                                                               |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**  | `028-jit-compiler.md:99-126` requires 6 files in `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **all do not exist**. Status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                                                                            |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                               | **Physical-evidence level**: the filenames precisely correspond to `ModuleCache` and incremental scheduling in the C1 design. **Someone built test skeletons per the design, found there was no corresponding implementation in the pipeline, and left it**                                                                                                                                                                                                                                                                                              |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                          | The `//! §6.1: CheckSession incremental check` in `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references in `mod_tests.rs:4` refer to this **gitignored planning document** (`.gitignore` contains `superpowers/`). These § references are **archaeological references to a deprecated API**                                                                                                                                                                                                                                                |

### Real Implementation Location of Cross-File Analysis

The cross-file analysis capabilities described in C1-C3 **really exist**, just in completely
different paths:

| Component           | Location                              | Lines          |
| ------------------- | ------------------------------------- | -------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439            |
| Resolver            | `src/frontend/module/resolver.rs`     | 195            |
| Role classification | `src/frontend/module/roles.rs`        | 388            |
| Consistency check   | `src/frontend/module/consistency.rs`  | 206            |
| Orchestrator        | `src/frontend/module/orchestrator.rs` | 1528           |
| **Total**           |                                       | **About 2670** |

Therefore, the disposition of C1-C3 is **not to delete the capabilities, but to delete the
misleading documents**, and to rewrite the boundary table in `check/index.md` to point to the actual
architecture of `src/frontend/module/`.

### The Disposition of RFC-019 Needs Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already in the `deprecated/` directory**,
and the residual check result is **clean** (`homoicon` has zero hits repo-wide, the document ends
with an honest conclusion of "if restarting in the future, start from proof/prototype experiments,
not from the RFC"). **It does not need additional disposition** — this item is in the authorization
list, but after verification it is confirmed to be in its final state.

## D. Function-Level Dead Code: REPL

`src/repl/` 4 files **1152 lines**, **zero `#[cfg(test)]`, zero `#[test]`**. Meanwhile,
`src/main.rs:440` is `args.command.unwrap_or(Commands::Repl)` — **running `yaoxiang` without any
arguments directly enters the REPL**.

| #   | Fact                                               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` has never been constructed** | Throughout `src/repl/`, it only appears twice: the definition in `eval.rs:23`, and a **match arm** in `mod.rs:180`. Therefore `Repl::format_value` (`mod.rs:493-502`) is dead code — **the REPL does not echo any evaluation result**                                                                                                                                                                          |
| D2  | **No cross-round state**                           | `extract_definitions` (`eval.rs:345-377`) only calls `define_variable` (storing a **type string**), never calls `define_var` (storing `RuntimeValue`); `wrap_code` also doesn't inject historical definitions. The branch of `VariableInfo::Value` (`eval.rs:23`) is unreachable. `REPLContext.variables` purely serves completion and does not participate in evaluation                                      |
| D3  | **Wrapped syntax appears outdated**                | `eval.rs:314`/`316` produces `main() -> () = () => { code }` (the `name() -> Ret` old-style function declaration header), whereas the current normative form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`, the normative table in RFC-007:94 is all `name: (a: Type) -> Ret`). **The compiler was not actually run to test; if true, every evaluation in the REPL will fail to compile** |
| D4  | `:debug` is a stub                                 | `mod.rs:454-466` only prints two lines of hints and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                                                  |
| D5  | `:history` is a stub                               | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                                      |
| D6  | Duplicate with `lib.rs`                            | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent "auto-wrap main" logics**, and the wrap forms are different                                                                                                                                                                                                                                                                         |

## E. Local Cleanup Items

| #   | Location                                                                               | Disposition                                                                                                                                                                                                                                                                        |
| --- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                             | Empty directory tree at the root. Almost certainly a case of concatenating the string `"undefined"` as a path somewhere. **Delete directly**                                                                                                                                       |
| E2  | `src/lib.rs:568`, `:582`                                                               | `dump_type_detail` / `dump_const_detail` each have a `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, the panic path is predictable. Should be changed to meaningful fallback output                                                               |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                              | `check_vendor_lock_consistency` and `check_workspace_consistency` are **about 60 lines of near-duplicate code**, differing only in the dependency source and path-dep skipping in two places. **Merge**                                                                            |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167` | Old function syntax detection and rejection, about 120 lines. Among them, `skip_old_function_syntax` (`:120-122`) has a function body with only a single-line comment. **Delete together with 05-frontend-paradigm.md**                                                            |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                       | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                   |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                | The "Tool Design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, `check/incremental-checking`. **If executing the document deletion in C1-C3, this must be cleaned up synchronously**, otherwise 404 links will appear on the documentation site |

### Not Dead Code, but Similar Issues: Three Hardcoded Discards

The `impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1400 lines) has 3
hardcoded discards:

| Line   | Discarded Content                | Consequence                                          |
| ------ | -------------------------------- | ---------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | Upvalue metadata is lost                             |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifacts lose the exception table**         |
| `2341` | `globals: Vec::new()`            | **`.42` artifacts lose global variable information** |

**These three places are not dead code, they are data-loss defects** — `.42` artifacts are only
complete under the in-memory path. **The disposition is different from dead code**: ~~should be
tracked as a separate issue~~ **All consolidated into P7 batch e (Decision D52, 2026-10-05), no
independent issue is left** — `2315` (exception table) and `2341` (global variables) are about the
integrity of language core semantics in artifacts, not "deferrable" items. All three are fixed
together, and `.42`'s `VERSION` is bumped from 4 to 5 in one go (the format header already has
`MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`). They are listed here only to avoid being
mistakenly cleaned up as "dead code".

## F. Not Deleted, but to be Marked or Decided

| #   | Object                                                                                     | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (1547 lines)                      | **Not dead code**, it is concatenated via `include!` at `checker.rs:5618` — **the only `include!` in the entire repository**. This file **is not declared as any module** (`grep 'mod semantic_tokens'` has zero hits repo-wide), the first line is directly `impl TypeChecker {`, with no own `use` header, and is the only file in the `checker/` directory. Consequences: no module identity, visibility isolation fails, rust-analyzer jump and symbol search fails, and the toolchain counts it as part of `checker.rs`. **The real size of the `checker` module is 5618+1547 ≈ 7165 lines**. **The fix: change to a real `mod`** (construction steps in [09](09-execution-wbs.md) §P5 5.1), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| F2  | `#[cfg(target_arch = "wasm32")]` branches in 27 files                                      | **Not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, `:17` depends on the main crate; the root `Cargo.toml:3`'s `exclude = ["wasm", ...]` exactly indicates it's an independent crate. Just looking at the root `Cargo.toml:31`'s `crate-type = ["rlib"]` would misjudge these branches as defensive dead code — **this inference does not hold**. CI has 4 build places (`_build-wasm.yml:75`, called from `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`). **These branches in those 27 files are load-bearing**, they determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm target (`lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` entirely). **The real cleanup target** is the unreachable parts of it — `wasm/src/lib.rs` only calls `Compiler::new` / `compile_with_source` / `std::io::wasm_output` / `CodegenContext` / `Interpreter`, and which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the playground scenario needs to be determined one by one (Decision D35: delete unreachable ones, keep reachable ones, attributed to P10)               |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                             | Only has the "Status" column (document status), **no "Implementation Status" column** — it **cannot express** the error "RFC marked as complete but code is missing". Only 5/52 RFCs repo-wide have an `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **26 accepted RFCs do not record implementation status at all**, including 009, 009a, 013, 018, 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F4  | `src/frontend/core/typecheck/layers/README.md:1-16`                                        | Declares layer order equivalence → ownership → termination → predicate, **exactly the opposite of the actual execution order** (actual predicate → termination → ownership), and **has no short-circuit**. The README describes a set of intended architecture that was not built (or has been partially abandoned)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| F5  | `docs/src/rfc/index.md`                                                                    | `:79` puts RFC-026a in the "Accepted RFCs" table but writes "Reviewing RFCs" in the status column; the "Reviewing RFCs" subsection at `:48-49` only lists 032 and 027a, missing 026a. Inconsistent with the 3 review items in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| F6  | `tools/cargo-dist/` (`dist.exe` 21,108,736 bytes) and `benches/shootout/out/` (8 binaries) | **None are tracked by git** (`git ls-files` returns empty), excluded by `.gitignore`. **Not a problem**, recorded only to prevent subsequent audits from misjudging as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                       | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` to the next quote), depends on the `("E1001", ...)` tuple syntax, **not a parser**. Its architecture (`parse` + `validate` + build.rs gate + `--fix` healing) is worth replicating to the opcode table, but **the extractor needs to be rewritten per the opcode form (about 30 lines)**. Attribution: follow-up work in 06-cleanup-inventory.md or an independent RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode fact is expressed in **5 places**: `BytecodeInstr::opcode()`(`bytecode.rs:558-646`)/ `size()`(`649-803`)/ `opcode::opcode_name()`(`opcode.rs:120-206`)/ decoding match(`bytecode.rs:978-2302`)/ 48 `translate_*`(`translator.rs:676-1577`). **The compiler only enforces 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + exhaustive matches in each `ops/*.rs` family function), missing one place in `opcode()` / `size()` / decoding arms / encoders **only blows up when running `.42` artifacts**. The comments in the `size()` table (649-803) self-describe inconsistently with actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic cross-check test** (existing size tests only cover Nop/Mov/Borrow/Release 4). Operator semantics here also go through a third set of parallel enums (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`, `Rem`/`Xor`/`Sar` have yet another set of names), to be consolidated together with the opcode-generation-phase gate. The target location of the vocabulary table is `middle/bytecode/opcode.rs` (migrated along with the bytecode domain merge, eliminating the L3→L4 reverse, see 01) |

## Key Decisions and Rationale

The authorization is "permitted to be completely deleted", but this document gives **different
recommendations** for several items:

| Category                                                                | Disposition                                     | Rationale                                                                                                                                                                                                                                                                                                                              |
| ----------------------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions, no implementation) | **Delete**                                      | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty function), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                                           |
| **Phantom design documents**                                            | **Delete or rewrite**                           | C1, C2 are deleted then rewritten to point to the actual implementation; C3 has its boundary table rewritten; **`config.js:259-268` must be cleaned up synchronously** (E6)                                                                                                                                                            |
| **RFCs whose status does not match implementation**                     | **Change status**                               | C4 (RFC-018) moved back to `draft` or supplement with `impl_status: 'not-started'`; C5 (RFC-028) supplement with `impl_status: 'not-started'`. **Do not delete** — the design itself may have value, the problem is the status-mark distortion                                                                                         |
| **Orphan tests with real assertions**                                   | **Revive, do not delete**                       | Total **1005 lines / 78 tests**: lexer 629 lines / 55 + 311 lines / 16 hidden inside live directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7. **The cost of revival is one line of `mod` declaration, the value is letting CI regain this coverage**. Deleting them is a net loss |
| **Zero-call but semantically important `pub` items**                    | **Lower visibility, do not delete**             | B2 (`TypeSystem`), B4 (`check_type_equivalence`) — move to `pub(crate)` to let the compiler point them out, or supplement the implementation. **B4's `is_subtype` must be kept**                                                                                                                                                       |
| **Opcodes/instructions that produce no value**                          | **Add construction sites or delete the opcode** | B8 (`TailCall`), B9 (`Switch`, the opcode round-trip test has already been explicitly registered with `WHITELIST=["SWITCH"]`), B10 (`UnaryOp::Not` silently degrades). Choose one of three, the status quo cannot be maintained; D32/D33/D34 have already been adjudicated as deletion, handled together with the `.42` version bump   |
| **Data-loss defects**                                                   | ~~Independent issue~~ **P7 batch e (D52)**      | The three hardcoded discards in section E — not dead code                                                                                                                                                                                                                                                                              |
| **No module identity but valid**                                        | **Refactor, do not delete**                     | F1 (`semantic_tokens.rs`) changes `include!` → real `mod` (see `02`)                                                                                                                                                                                                                                                                   |
| **Needs decision**                                                      | **List as open issues**                         | The **unreachable wasm branch** cleanup scope in F2, layer-order correction will expose new diagnostics                                                                                                                                                                                                                                |

### Directions Not Adopted

- **Process everything per "complete deletion", orphan tests also deleted** — 1005 lines of
  real-assertion tests are assets. The deletion cost is one line of `mod` declaration, the benefit
  is CI regaining coverage. The authorization is "may delete", not "should delete".
- **Only add gates, do not move code** — gates prevent regression, but B1's 415 lines, the dispatch
  chain, the 96 lines of dead ladder **already exist**, gates will only freeze them at baseline.
- **Keep RFC-018 / RFC-028's `accepted` status, only supplement documentation** — `rfc/index.md:131`
  defines "accepted = enters the implementation phase", readers will reasonably assume there is
  already an implementation. `TRACKING.md` has precedents for the `impl_status` field (5 RFCs are
  using it), the cost of supplementing the field is much lower than the cost of status distortion.

## Implementation Points

**This document does not require an equivalence criterion** (belongs to the
[C6 pure-deletion category](07-equivalence-oracle.md)), but **each item still needs to be confirmed
to have no references**. In the global phase sequence of RFC-039, S1 corresponds to P1, S2-S6
correspond to P10.

| Phase | Content                                                                                                                         | Acceptance                                                                                                                                                                                                                                                              |
| ----- | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphan tests with assertions (**1005 lines / 78 tests**)                                                                 | Test counts in the five modules lexer / pratt / passes / emitter / template rise in `cargo test`; **real defects are expected to be exposed** (`literals.rs` overflow paths, illegal `\x`/`\u` escapes have never been tested before), defect-fix time must be reserved |
| S2    | Delete pure-placeholder dead code (B1/B3/B6/B7 with 05 closed out / C6 / E1)                                                    | `cargo test` passes; `cargo clippy -D warnings` has no new warnings                                                                                                                                                                                                     |
| S3    | Phantom design document disposition (C1-C3 + E6 synchronously cleans up `config.js`)                                            | Documentation site builds without 404; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                          |
| S4    | RFC status correction (C4/C5/F3 — add "Implementation Status" column to `TRACKING.md`)                                          | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` will be regenerated automatically, **do not edit manually**)                                                                                                                                               |
| S5    | Lower visibility of `pub` items (B2/B4)                                                                                         | The compiler can point out real dead code                                                                                                                                                                                                                               |
| S6    | Opcode no-value item decisions land (B8/B9/B10/F8, executed per D32/D33/D34)                                                    | Delete `Switch` / `TailCall` dead paths; B10's `opcode()` distinguishes the `op` field; F8 gate lands                                                                                                                                                                   |
| S7    | `include!` refactor (F1, construction steps in [09](09-execution-wbs.md) §P5 5.1) + layer-order correction (F4, in 4.4.1 of P4) | See `02-stage-contract.md`                                                                                                                                                                                                                                              |

**S1 must be done first, and defect-fix time must be reserved.** Its value is not "cleanup", but
**letting CI regain 1005 lines / 78 tests that have never run** — before that, any judgment about
test coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four base scanners in `literals.rs` have 4 isomorphic
  "continue consuming after overflow without reporting" branches, the entire `scan_leading_dot`
  function, and the illegal `\x`/`\u` escape paths, all have never been tested. After reviving the
  tests, the code will most likely need to be fixed, which will delay subsequent refactoring.
- **B9/B10 involve language behavior decisions**. Whether `Switch` and `UnaryOp::Not` are "add
  construction sites" or "delete the opcode" requires considering whether they belong to surfaces
  depended on by published corpora.
- **After C1-C3 are deleted, the capability documentation for cross-file analysis will be briefly
  missing**, until `check/cross-file-analysis.md` is rewritten to point to the actual architecture
  of `src/frontend/module/`.
- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Judgments
  beyond form 2 must be done manually item by item, and **this inventory does not constitute an
  exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  will not be automatically discovered by the compiler, only by search.
- **Deleting the half-cut residual of RFC-023** (B1) can fulfill the decision that "closures have
  been removed from the language"; **correcting the status distortion** (C4/C5/F3) can restore the
  reference value of `TRACKING.md`.

> **The open issues originally listed in this section have all been adjudicated.** Per-item
> decisions are in the [RFC-039 Decision Registry](../../rfc/draft/039-compiler-architecture.md)
> (D1–D50). **This document does not leave any pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  Source decision for B1's residual
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — The unwired `§4.1` runtime check insertion of B3
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — The formal vehicle for C1/C6
- [RFC-039 Compiler Function Routing Directory Design](../../rfc/draft/039-compiler-architecture.md)
  — The overall master plan
- [Refactoring Equivalence Criteria](07-equivalence-oracle.md) — Criterion requirements for the C6
  category
- `src/frontend/core/typecheck/layers/README.md` — Layer-order declaration distortion of F4
- `build.rs:19-55` — An example of this repository's generation-phase gate
