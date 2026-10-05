# Dead Code and Empty Design Cleanup

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, verification criterion grading, and execution phase ordering are in the RFC-039
> main text; the positioning of each companion document is in [this directory's index](index.md).

## Positioning and Scope

This document provides the YaoXiang repository's dead code and empty design inventory, determination
methods, per-item disposition decisions, and execution order. Cleanup targets fall into four
categories:

| Category                                                        | Scale                                                                   |
| --------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Files never participating in compilation                        | 24 files / 2628 lines (of which 23 files / 1081 lines are true orphans) |
| Compiled but zero-call dead code                                | 1500+ lines                                                             |
| Design documents describing an architecture that never existed  | Three documents in `docs/src/dev/design/check/`                             |
| Features attached to the default entry, zero tests, zero output | `src/repl/` 1152 lines                                                  |

**The authorized scope is "full deletion permitted"** (decision of 2026-10-03). However, this
document gives **disposition recommendations contrary to "direct deletion"** for several
items—rationale in [Key Decisions and Rationale](#key-decisions-and-rationale).

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), criterion design (see `07-equivalence-oracle.md`). The
entire `layers/dispatch.rs` chain recorded under B3 is unwired; its deletion action falls under S2.

## Current State

### Three Forms of Dead-Code Liability

**Form 1: Placeholder.** Code exists, looks like it is doing something, but actually does nothing.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the "runtime check insertion" mechanism designed in `RFC-027 §4.1`,
  **completely unwired**.
- `check_type_equivalence` (`ProofContext` entry) in
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is only called by tests. **Note** that the
  same file's `is_subtype` is used in production by `inference/assignment.rs:17`, so **the whole
  file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler; its `pub` surface has
  only `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` / `RuntimeStats`
  / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend**—the `Executor` trait
  at `backends/mod.rs:356` currently has only one implementor (the interpreter). This point needs
  clarification because the directory name can mislead.

**Form 2: Residue.** An upstream decision has been made, but deletion was only half-done.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) retains 6 types: `FunctionInstance`
  (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), `CaptureVariable`. **Clear source**: When `RFC-023` deprecated
  the "closure capture model" it declared "**roughly 850 lines of code will be deleted along with
  the closure model**". These 415 lines are the survivors of those 850—**deletion was only
  half-done**. **Verification**: doing a whole-repo word-boundary search for these 6 types, the hit
  count **outside `instance.rs` is 0** (including tests). The `pub use passes::mono::instance::*;`
  at `middle/mod.rs:40` is just a glob passthrough and does not constitute consumption.

**Form 3: False coverage.** This is the most dangerous kind—**it makes the repository look more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only 5 `pub mod`s (`literals` / `state` / `symbols` /
`tokenizer` / `tokens`) + a `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`
(`:104-106`). **Nowhere declares `mod tests;`**, so the entire `lexer/tests/` subtree of 13 files /
686 lines **has never participated in compilation**.

Among them, 7 are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
`delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totaling 19 lines, 0
assertions). **629 lines / 55 real tests have never run**:

| File              | Lines   | `#[test]` |
| ----------------- | ------- | --------- |
| `literals.rs`     | 222     | 30        |
| `rfc010_lexer.rs` | 167     | 12        |
| `lexer_mod.rs`    | 89      | 5         |
| `rfc004_lexer.rs` | 81      | 5         |
| `symbols.rs`      | 70      | 3         |
| **Total**         | **629** | **55**    |

(`fstring.rs` 90 lines / 11 tests does compile, but via the `#[path]` bypass rather than
`mod tests;`; `mod.rs` 38 lines is a declaration file with no assertions.)

**Only one file / 12 tests is actually running at the lexical layer, and they all test only the
f-string brace raw protocol.**

The stealth of this kind of problem lies in: `cargo test` will not fail (tests that have not been
run cannot fail), and CI has no test-count baseline or wiring check.

### Empty Design

The three documents under `docs/src/dev/design/check/` describe an architecture that **has never
existed**. This is not "designed but not done"; rather, **the documents describe a different system
that has never existed**, and write "possible future design" as "defects of an implemented system".

## Methods and Credibility Statement

### Reachability Analysis Method

Starting from the two crate roots `src/lib.rs` / `src/main.rs`, parse all 509 `.rs` files' `mod` /
`pub mod` / `#[path]` declarations, build the module graph and run BFS reachability analysis.
`include!` targets are judged separately (not counted toward reachability).

**This method has known false-positive patterns; during the writing process it actually triggered
twice:**

1. `Split-Path` on Windows returns `\`, which doesn't match the forward-slash key → once falsely
   reported 499 orphans
2. **The submodule directory for a non-`mod.rs` file is `its own name/` rather than
   `parent directory/`** → `orchestrator.rs`'s `mod tests` was parsed as
   `src/frontend/module/tests/`, nearly classifying 753 lines of living tests as dead code

Therefore **every conclusion has been double-confirmed via "actually read the file + actually search
references"**, and entries that did not pass secondary confirmation are not admitted into this
document.

### Line Count Calibration

All line counts in this document are counted with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines, systematically undercounting by about 5%** (e.g.,
`manifesto.md` reports 658 with `-Line`, actually 893). An earlier analysis in this repo gave a low
total because of this calibration, corrected here.

### Known Limitations

- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Forms
  beyond Form 2 require manual judgment on a per-item basis.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types must be
  found via search, which this method does not cover.
- This document does not involve running the compiler, so all "never run" conclusions are static
  determinations.

## A. Files Never Participating in Compilation

**Total 24 files / 2628 lines.** Of these, `semantic_tokens.rs` is Form 4 (see section F); the
remaining **23 files / 1081 lines are true orphans**.

| Path                                                          | Scale                   | Content                                                                       | Determination Basis                                                                                                                                                                                   |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/tests/` **entire subtree, 13 files** | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only 5 `pub mod`s; the only test declaration at `:104-106` is `#[path]` to `tests/fstring.rs`. Running `grep 'mod tests'` over the whole directory hits only `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 files                        | 14 lines                | All placeholder doc comments, 0 assertions                                    | `src/frontend/pipeline.rs` (740 lines) has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declarations in the entire text (zero `Select-String` hits)                                              |
| `src/package/template/tests/` 3 files                         | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`s**                    | `src/package/template/mod.rs` (7 lines) declares only `mod gitignore; mod main_yx;`, no `mod tests;`                                                                                                  |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden in a **live** directory                                                | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                                       |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                          |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                                  |

**The last 3 items total 16 tests / 311 lines of code that have never run.** They are hidden inside
active directories; the directory's `mod.rs` looks complete, so they are more deceptive than entire
orphan directories.

Inside `lexer/tests/` there are another two files of the same "declared but missing declaration"
form, but they are already counted within the 629 lines in the first row of the table, **not
double-counted**: `lexer_mod.rs` (89 lines / 5 tests, `lexer/tests/mod.rs` declares 11 `mod`s but no
`mod lexer_mod;`), `symbols.rs` (70 lines / 3 tests, same situation).

**Real tests that have never run total 1005 lines / 78 tests**: lexer 629/55 + pratt / passes /
emitter 311/16 + package/template 65/7.

**Special attention to `precedence_inline.rs`**: it tests a `Precedence` enum + `PrecedenceContext`
in `precedence.rs` that is **zero-used in production** (96 lines of dead code). Even if it were
wired up, it would test a dead path.

Another hidden in the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5 lines, 0
tests): `typecheck/tests/mod.rs` declares 23 test modules, but specifically misses it—the file is
left with a single `use` line, pure debris. During S1 revival it should be deleted along with the
wiring, or have assertions added.

## B. Compiled but Zero-Call Dead Code

| #   | Location                                                                         | Scale               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | -------------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | Whole-repo word-boundary search: hits outside `instance.rs` all 0 (including tests). Source: RFC-023 declared deletion of roughly 850 lines, only half was done                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` + 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both passthroughs). Whole-repo `TypeSystem` hits only 5: definition 2 + re-export 1 + **unrelated** `lexer/symbols.rs:503 TypeSystemValidator` 2. The 4 methods have zero call sites. **Because it is `pub`, the compiler will never warn**                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| B3  | `src/frontend/core/typecheck/layers/dispatch.rs` entire chain                    | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` zero call sites. RFC-027 §4.1's runtime check insertion is unwired. Note it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` only called by tests. **Same file's `is_subtype` must be kept** (used in production by `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-described as "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests)—and that test file itself has never been compiled** (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore **both must be deleted together**: deleting only the tests leaves 96 lines of dead code, deleting only the enum would break test compilation                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, `skip_old_function_syntax` at `:725`—**they are live old-syntax rejection gates, not unreferenced dead code**. The `skip_old_function_syntax` (`:120-122`) function body has only one comment line `// 旧语法已移除`, but **the function consumes no token after being called**—this is an independent behavior issue (whether the rejection path correctly advances the parse position), not dead code. **Disposition: review behavior together with 05-frontend-paradigm.md (decision D27: naturally deleted with the grammar migration), not treated as dead code for direct deletion**                                                                                                                                                                                                           |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | Whole repo has only `match`, no construction sites. The "no generation site (dead path)" comment at `bytecode.rs:2156` is **accurate** (encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| B9  | `Switch` opcode                                                                  | —                   | `backends/common/opcode.rs` defines it, has a variant, has `opcode()`/`size()` arms, has a decode arm, **but the whole repo has zero construction sites**. The interpreter has live implementation at `ops/control.rs:90`. **Explicitly registered by tests as a known dead path**: `WHITELIST: &[&str] = &["SWITCH"]` at `tests/bytecode.rs:1035-1040` (fails if the difference is non-empty)                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` **ignoring the `op` field**, and the decoder at `1236-1248` also only constructs `Neg`. **Silently degrades to I64_NEG via the `opcode()` mapping path** (`.42` direct run: `!x` becomes `-x`; the interpreter's memory path at `ops/arith.rs:46-49` is actually correct). Used only by the interpreter at `ops/arith.rs:46,49` and `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                                            |
| B11 | 4 `#[allow(dead_code)]` sites                                                    | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (conservative exemption for extern binding, **reasonable to keep**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction** (the only construction site is the test at `parser/tests/ast.rs:828` hand-crafting a node), but has **12 production consumers + 2 exhaustive arms** (full list in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`; plus `ast.rs:1026` `Expr::span()`, `pratt/mod.rs:47` `expr_end_line`. Function definitions actually go through `Expr::Lambda`(`nud.rs:425-429`) + `StmtKind::Assign`(`declarations.rs:462-479`). Deletion belongs to P6 (the 16-item change list in 03 §5.5) |

## C. Empty Design

| #   | Object                                                                          | Scale                                      | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                                 | 53 lines                                   | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, `HotReloader` **all have zero hits in the whole repo**. The "known limits" at `:46-47` itself admits that `check_incremental` internally still calls `check_files_with_diagnostics` (full-quantity path); `:46` also references the busy-wait debouncing in `command.rs`—**`command.rs` has neither `Instant` nor `recv_timeout`** (whole-repo `recv_timeout` appears only in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                                  | 43 lines                                   | `:37` claims placeholder implementations in `traits/` (coherence / impl_check / object_safety / resolution) are incomplete—**the `traits/` directory does not exist** (`Test-Path` = False), and those identifiers have zero hits in the whole repo. `:38` claims `check_single_module` creates a separate Compiler for each file—**that function does not exist**, appearing only in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                                             |
| C3  | `docs/src/dev/design/check/index.md`                                                | 29 lines                                   | `:14` claims "watch mode only re-checks affected files", `:27` claims "check only does the first two steps". Reality: `check_project` also does vendor consistency, project role determination, dead-code analysis, W1006; the `Check` subcommand in `main.rs` **has no `--watch` flag** (all `#[arg]` declarations have been checked one by one)                                                                                                                                                                                                       |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines of design / **0 lines of code** | AOT hits in `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell. `:861-876` itself left an implementation checklist with all `- [ ]`, but status is `accepted`                                                                                                                                                                                                                                                                                          |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**  | `028-jit-compiler.md:99-126` requires 6 files in `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **none exist**. Status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                                                                                 |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                               | **Physical-evidence-level proof**: the filenames precisely correspond to `ModuleCache` and the incremental scheduler in C1's design. **Someone built test skeletons according to the design, found no corresponding implementation in the pipeline, and put it down**                                                                                                                                                                                                                                                                                   |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                          | The `//! §6.1: CheckSession incremental checking` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` point to this **gitignored planning document** (`.gitignore` includes `superpowers/`). These § references are **archaeological-layer references to a deprecated API**                                                                                                                                                                                                                                      |

### Actual Implementation Location of Cross-File Analysis

The cross-file analysis capability that C1-C3 describe **does exist**; the path is completely
different:

| Component            | Location                              | Lines          |
| -------------------- | ------------------------------------- | -------------- |
| Module registry      | `src/frontend/module/registry.rs`     | 439            |
| Resolver             | `src/frontend/module/resolver.rs`     | 195            |
| Role classification  | `src/frontend/module/roles.rs`        | 388            |
| Consistency checking | `src/frontend/module/consistency.rs`  | 206            |
| Orchestrator         | `src/frontend/module/orchestrator.rs` | 1528           |
| **Total**            |                                       | **about 2670** |

Therefore the disposition of C1-C3 is **not to delete the capability, but to delete the misleading
documents**, and rewrite the boundary table in `check/index.md` to point at the actual architecture
in `src/frontend/module/`.

### RFC-019 Disposition Needs Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already in the `deprecated/`
directory**, and the residual check result is **clean** (`homoicon` has zero hits in the whole repo;
the end of the document has an honest "if restarted in the future, start from proof/prototype
experiments, not from the RFC" conclusion). **It needs no additional disposition**—it is in the
authorized list, but verification confirms it is already at terminal state.

## D. Function-Level Dead Code: REPL

`src/repl/` 4 files **1152 lines**, **zero `#[cfg(test)]`, zero `#[test]`**. Meanwhile
`src/main.rs:440` is `args.command.unwrap_or(Commands::Repl)`—**executing `yaoxiang` without
arguments directly enters REPL**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D1  | **`EvalResult::Value` is never constructed** | Across all `src/repl/` it appears only 2 times: definition at `eval.rs:23`, and a **match arm** at `mod.rs:180`. Therefore `Repl::format_value` (`mod.rs:493-502`) is dead code, **REPL does not echo any evaluation result**                                                                                                                                                              |
| D2  | **No cross-round state**                     | `extract_definitions` (`eval.rs:345-377`) only calls `define_variable` (stores **type string**), never `define_var` (stores `RuntimeValue`); `wrap_code` also doesn't inject historical definitions. The `VariableInfo::Value` branch at `eval.rs:23` is unreachable. `REPLContext.variables` purely serves completion, not evaluation                                                     |
| D3  | **Wrapping syntax suspected outdated**       | `eval.rs:314`/`316` produces `main() -> () = () => { code }` (old-style `name() -> Ret` function declaration header), while the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`, RFC-007:94's canonical table is all `name: (a: Type) -> Ret`). **Compiler not actually run to verify; if correct, every REPL evaluation would fail to compile** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` only prints two hint lines and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                                  |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                  |
| D6  | Duplicate of `lib.rs`                        | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent "automatic wrap-main" logics**, and the wrap forms differ                                                                                                                                                                                                                                                       |

## E. Local Cleanup Items

| #   | Location                                                                               | Disposition                                                                                                                                                                                                                                    |
| --- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                             | Root-level empty directory tree. Almost certainly some place string-concatenated `"undefined"` as a path. **Delete directly**                                                                                                                  |
| E2  | `src/lib.rs:568`, `:582`                                                               | `dump_type_detail` / `dump_const_detail` each have a `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related; the panic path is predictable. Should change to meaningful fallback output                               |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                              | `check_vendor_lock_consistency` and `check_workspace_consistency` are **about 60 lines of near-duplicates**, differing only in two places: the dependency source and the path-dep skip. **Merge**                                              |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167` | Old function syntax detection and rejection, about 120 lines. Among them, `skip_old_function_syntax`(`:120-122`) function body has only one comment line. **Delete together with 05-frontend-paradigm.md**                                     |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                       | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                               |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                | "Tool design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, `check/incremental-checking`. **If executing C1-C3's document deletion, this must be cleaned up in sync**, otherwise the doc site has 404 links |

### Not Dead Code, but Same-Class Issue: Three Hard-Coded Drops

The `impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1400 lines) has 3
hard-coded drops:

| Line   | Dropped Content                  | Consequence                                          |
| ------ | -------------------------------- | ---------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | upvalue metadata lost                                |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifact loses exception table**             |
| `2341` | `globals: Vec::new()`            | **`.42` artifact loses global variable information** |

**These three are not dead code, but data-loss defects**—the `.42` artifact is complete only on the
in-memory path. **Disposition is different from dead code**: should be tracked as a separate issue.
`2312` is fixed with P7 and bumps `.42`'s `VERSION` from 4 to 5 per D17 (the format header already
has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`); `2315` / `2341` follow the same
version-bump path, registered as a separate issue in 04-ssa.md. Listed here only to avoid being
mistakenly cleaned up as "dead code".

## F. Don't Delete, but Mark or Decide

| #   | Object                                                                                     | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (1547 lines)                      | **Not dead code**; it is included by `checker.rs:5618` via `include!`—**the only `include!` in the entire repository**. This file **is not declared as any module** (whole-repo `grep 'mod semantic_tokens'` is zero hits); the first line is directly `impl TypeChecker {`, with no `use` header of its own, and the `checker/` directory has only this one file in it. Consequences: no module identity, visibility isolation fails, rust-analyzer jump-to and symbol search fail, the toolchain counts it as part of `checker.rs`. **The real size of the `checker` module is 5618+1547 ≈ 7165 lines**. **Fix: change to a real `mod`** (construction steps belong to [09](09-execution-wbs.md) §P5 5.1), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| F2  | 27 files' `#[cfg(target_arch = "wasm32")]` branches                                        | **Not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, `:17` depends on the main crate; the root `Cargo.toml:3` `exclude = ["wasm", ...]` precisely indicates it is an independent crate. Only looking at the root `Cargo.toml:31` `crate-type = ["rlib"]` would mistakenly judge these branches as defensive dead code—**this inference does not hold**. CI has 4 build sites (`_build-wasm.yml:75`, callers `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`). **These 27 files' branches are load-bearing**; they determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm target (`lib.rs:139/153/170` fully gate out `run_file` / `run_project` / `build_bytecode`). **The real cleanup target** is the unreachable part—`wasm/src/lib.rs` only calls `Compiler::new` / `compile_with_source` / `std::io::wasm_output` / `CodegenContext` / `Interpreter`; which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the playground scenario needs to be judged one by one (decision D35: delete unreachable, keep reachable, belongs to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                      | Has only a "Status" column (document status), **no "Implementation Status" column**—it **cannot express** the error of "RFC marked complete but code is not". Only 5/52 RFCs in the whole repo have an `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **26 accepted RFCs do not record implementation status at all**, including 009, 009a, 013, 018, 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| F4  | `src/frontend/core/typecheck/layers/README.md:1-16`                                        | Declares layer order equivalence → ownership → termination → predicate, **exactly the reverse of actual execution order** (actually predicate → termination → ownership), and **has no short-circuit at all**. The README describes a not-yet-built (or partially abandoned) intended architecture                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| F5  | `docs/src/rfc/index.md`                                                             | `:79` places RFC-026a in the "Accepted RFCs" table but the status column says "RFCs Under Review"; the "RFCs Under Review" subsection at `:48-49` only lists 032 and 027a, missing 026a. Inconsistent with the 3 review items in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| F6  | `tools/cargo-dist/` (`dist.exe` 21,108,736 bytes) and `benches/shootout/out/` (8 binaries) | **Neither is tracked by git** (`git ls-files` returns empty), excluded by `.gitignore`. **Not a problem**, recorded only to prevent later audit from mistaking as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                       | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` takes until the next quote), depending on the `("E1001", ...)` tuple syntax, **not a parser**. Its architecture (`parse` + `validate` + build.rs gate + `--fix` healing) is worth replicating to the opcode table, but **the extractor needs to be rewritten for opcode form (about 30 lines)**. Belongs to follow-up work for 06-cleanup-inventory.md or a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode fact is expressed in **5 places**: `BytecodeInstr::opcode()`(`bytecode.rs:558-646`)/ `size()`(`649-803`)/ `opcode::opcode_name()`(`opcode.rs:120-206`)/ decode match(`bytecode.rs:978-2302`)/ 48 `translate_*`(`translator.rs:676-1577`). **The compiler enforces only 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + the exhaustive `match` of each `ops/*.rs` family function); `opcode()` / `size()` / decode arm / encoder miss one **only blows up when running the `.42` artifact**. The comment of the `size()` table (649-803) describes itself inconsistently with the actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic pairwise test** (existing size tests cover only Nop/Mov/Borrow/Release 4). Operator semantics here also use a third parallel enum (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`, another naming set for `Rem`/`Xor`/`Sar`), closed up together with the opcode generation-time gate. The vocabulary target location is `middle/bytecode/opcode.rs` (migrated with the bytecode domain merge, eliminating the L3→L4 inversion, see 01)       |

## Key Decisions and Rationale

The authorization is "full deletion permitted", but this document gives **different
recommendations** for several items:

| Category                                                               | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons without assertions or implementation) | **Delete**                                 | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty functions), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                               |
| **Empty design documents**                                             | **Delete or rewrite**                      | C1, C2 deleted then rewritten to point at actual implementation; C3's boundary table rewritten; **`config.js:259-268` must be cleaned up in sync** (E6)                                                                                                                                                                     |
| **RFCs whose status does not match implementation**                    | **Change status**                          | C4 (RFC-018) moved back to `draft` or add `impl_status: 'not-started'`; C5 (RFC-028) add `impl_status: 'not-started'`. **Don't delete**—the design itself may have value; the problem is status-mark distortion                                                                                                             |
| **Orphan tests with real assertions**                                  | **Revive, not delete**                     | Total **1005 lines / 78 tests**: lexer 629 lines / 55 + 311 lines / 16 hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7. **Revival cost is one line of `mod` declaration; the value is letting CI regain this coverage**. Deleting them is a net loss |
| **Zero-call but semantically important `pub` items**                   | **Demote visibility, not delete**          | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move to `pub(crate)` to let the compiler point them out, or add implementations. **B4's `is_subtype` must be kept**                                                                                                                                                        |
| **Opcodes / instructions that cannot produce values**                  | **Add construction site or delete opcode** | B8 (`TailCall`), B9 (`Switch`, opcode round-trip test explicitly registered with `WHITELIST=["SWITCH"]`), B10 (`UnaryOp::Not` silently degrades). Three choices, can't maintain the status quo; D32/D33/D34 have ruled to delete, handled together with `.42` version bump                                                  |
| **Data-loss defects**                                                  | **Separate issue**                         | Three hard-coded drops in section E—not dead code                                                                                                                                                                                                                                                                           |
| **No module identity but valid**                                       | **Refactor, not delete**                   | F1 (`semantic_tokens.rs`) change `include!` → real `mod` (see `02`)                                                                                                                                                                                                                                                         |
| **Needs decision**                                                     | **Listed as open question**                | Scope of unreachable wasm branch cleanup in F2, layer-order fix may expose new diagnostics                                                                                                                                                                                                                                  |

### Directions Not Adopted

- **Process all as "full deletion", even orphan tests**—1005 lines of real-assertion tests are an
  asset. The deletion cost is one line of `mod` declaration, the gain is CI regaining coverage. The
  authorization is "may delete", not "should delete".
- **Only add gates, don't touch code**—gates can prevent regression, but the 415 lines of B1, the
  dispatch chain, and the 96 dead rungs **already exist**; gates only freeze them at baseline.
- **Keep `accepted` status of RFC-018 / RFC-028 and only supplement
  documentation**—`rfc/index.md:131` defines "accepted = entering implementation phase"; readers
  will reasonably assume there is implementation. `TRACKING.md` already has a precedent of the
  `impl_status` field (5 RFCs use it); the cost of adding the field is far less than the cost of
  status distortion.

## Implementation Points

**This document does not need equivalence criteria** (belongs to
[C6 pure deletion category](07-equivalence-oracle.md)), but **still requires per-item confirmation
of no references**. In RFC-039's global phase sequence, S1 corresponds to P1, S2-S6 correspond to
P10.

| Phase | Content                                                                                                                                | Verification                                                                                                                                                                                                                                                 |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| S1    | Revive orphan tests with assertions (**1005 lines / 78 tests**)                                                                        | `cargo test` shows the test count for lexer / pratt / passes / emitter / template five modules rise; **expected to expose real defects** (overflow paths in `literals.rs`, `\x`/`\u` illegal escape sequences never tested before), need to reserve fix time |
| S2    | Delete pure-placeholder dead code (B1/B3/B6/B7 closed with 05/C6/E1)                                                                   | `cargo test` passes; `cargo clippy -D warnings` has no new warnings                                                                                                                                                                                          |
| S3    | Empty design document disposition (C1-C3 + E6 sync cleanup of `config.js`)                                                             | Doc site build has no 404; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                           |
| S4    | RFC status correction (C4/C5/F3—`TRACKING.md` adds "Implementation Status" column)                                                     | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` will be regenerated automatically, **don't manually edit**)                                                                                                                                     |
| S5    | Demote `pub` item visibility (B2/B4)                                                                                                   | Compiler can point out the real dead code                                                                                                                                                                                                                    |
| S6    | Opcode cannot-produce-value items decided (B8/B9/B10/F8, executed per D32/D33/D34)                                                     | Delete `Switch` / `TailCall` dead paths; B10's `opcode()` distinguishes `op` field; F8 gate landed                                                                                                                                                           |
| S7    | `include!` refactor (F1, construction steps belong to [09](09-execution-wbs.md) §P5 5.1) + layer-order fix (F4, belongs to P4's 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                                   |

**S1 must be done first, and defect fix time must be reserved.** Its value is not "cleanup" but
**letting CI regain 1005 lines / 78 tests that have never run**—before this, any judgment about test
coverage is built on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners in `literals.rs` have 4 isomorphic
  "continue consuming after overflow without error" branches, the entire `scan_leading_dot`
  function, the `\x`/`\u` illegal escape paths—none of them has been tested. After revival, it is
  very likely code needs to be fixed, which will delay subsequent refactoring.
- **B9/B10 involve language behavior decisions**. Whether `Switch` and `UnaryOp::Not` are "add
  construction site" or "delete opcode" requires considering whether they belong to a surface that
  released test corpora depend on.
- **After C1-C3 are deleted, the cross-file analysis capability documentation will be briefly
  absent** until `check/cross-file-analysis.md` is rewritten to point at the actual architecture in
  `src/frontend/module/`.
- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Judgments
  beyond Form 2 must be done manually on a per-item basis, and **this list does not constitute
  exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  will not be automatically found by the compiler, only via search.
- **Deleting RFC-023's half-done residue** (B1) can fulfill the decision "closures have been removed
  from the language"; **fixing status distortion** (C4/C5/F3) can restore `TRACKING.md`'s reference
  value.

> **The open questions originally listed in this section have all been decided.** Per-item decisions
> are in [RFC-039 Decision Register](../../rfc/draft/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  source decision of B1's residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — B3's unwired `§4.1` runtime check insertion
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — formal carrier of C1/C6
- [RFC-039 Compiler Function Routing Directory Design](../../rfc/draft/039-compiler-architecture.md) —
  upper-level master document
- [Refactor Equivalence Criteria](07-equivalence-oracle.md) — criteria requirements for the C6
  category
- `src/frontend/core/typecheck/layers/README.md` — F4 layer-order declaration distortion
- `build.rs:19-55` — example of this repo's generation-time gates
