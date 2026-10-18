# Dead Code and Empty Design Cleanup

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiered acceptance criteria, and execution phase sequence are in the body of
> RFC-039; the positioning of each companion document is in [this directory's index](index.md).

## Positioning and Scope

This document provides the YaoXiang repository's dead code and empty design inventory, judgment
methods, item-by-item disposition decisions, and execution order. Cleanup targets fall into four
categories:

| Category                                                               | Scale                                                                   |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Files that never participate in compilation                            | 24 files / 2628 lines (of which 23 files / 1081 lines are real orphans) |
| Compiled but zero-call dead code                                       | 1500+ lines                                                             |
| Design documents describing architectures that never existed           | Three documents in `docs/src/dev/design/check/`                         |
| Features hanging on the default entry, with zero tests and zero output | `src/repl/` 1152 lines                                                  |

**The authorized scope is "thorough deletion permitted"** (2026-10-03 decision). However, this
document gives **disposition recommendations opposite to "direct deletion"** for several
items—reasons in [Key Decisions and Rationale](#key-decisions-and-rationale).

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), criterion design (see `07-equivalence-oracle.md`).
B3's `layers/dispatch.rs` whole chain being unwired has its deletion action folded into S2.

## Current State

### Three Liability Forms of Dead Code

**Form 1: Placeholder.** Code exists, looks like it's doing something, actually doesn't.

- The entire `src/frontend/core/typecheck/layers/dispatch.rs` chain (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero calls** in the
  compiler. It is the "runtime check insertion" mechanism from `RFC-027 §4.1`, and is **completely
  unwired**.
- The `check_type_equivalence` (`ProofContext` entry) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is only called by tests. **Note** that
  `is_subtype` in the same file is used in production by `inference/assignment.rs:17`, so **the
  whole file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler; its `pub` surface only
  includes `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend**—the
  `Executor` trait in `backends/mod.rs:356` currently has only one implementer (the interpreter).
  This needs clarification because the directory name is misleading.

**Form 2: Residue.** The upstream decision was made, but deletion was only half-done.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) contains 6 residual types:
  `FunctionInstance` (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), `CaptureVariable`. **Source is clear**: When `RFC-023`
  deprecated the "closure capture model", it stated "**along with the closure model, approximately
  850 lines of code will be deleted**." These 415 lines are the surviving remainder of those 850
  lines—**deletion was only half-done**. **Verification**: doing a whole-repo word-boundary search
  on these 6 types, the hit count outside `instance.rs` is **all 0** (including tests). The
  `pub use passes::mono::instance::*;` at `middle/mod.rs:40` is just a glob relay, not actual
  consumption.

**Form 3: False coverage.** This is the most dangerous kind—**it makes the repository look more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` only has 5 `pub mod` (`literals` / `state` / `symbols` /
`tokenizer` / `tokens`) + one `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`
(`:104-106`). **Nowhere declares `mod tests;`**, so the entire `lexer/tests/` subtree of 13 files /
686 lines **never participates in compilation**.

Among them, 7 are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
`delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totaling 19 lines, 0
assertions). **629 lines / 55 real tests never ran**:

| File              | Lines   | `#[test]` |
| ----------------- | ------- | --------- |
| `literals.rs`     | 222     | 30        |
| `rfc010_lexer.rs` | 167     | 12        |
| `lexer_mod.rs`    | 89      | 5         |
| `rfc004_lexer.rs` | 81      | 5         |
| `symbols.rs`      | 70      | 3         |
| **Total**         | **629** | **55**    |

(`fstring.rs` 90 lines / 11 tests do get compiled, but via the `#[path]` bypass rather than
`mod tests;`; `mod.rs` 38 lines is the declaration file, with no assertions.)

**Only one file, 12 tests, runs in the lexical layer, and all of them only test f-string's
curly-brace raw protocol.**

The insidiousness of this kind of issue is: `cargo test` will not fail (tests that haven't run won't
fail), and CI has no test count baseline or wiring check.

### Empty Designs

The three documents under `docs/src/dev/design/check/` describe a set of **never-existed**
architectures. This is not "designed but not done"—it's that **the documents describe another system
that never existed**, and wrote "possible future designs" as "defects of the implemented system".

## Methodology and Credibility Statement

### Reachability Analysis Method

Starting from the two crate roots `src/lib.rs` / `src/main.rs`, parse all 509 `.rs` files' `mod` /
`pub mod` / `#[path]` declarations, build the module graph and do BFS reachability analysis.
`include!` targets are judged separately (not counted in reachability).

**This method has known misjudgment patterns, which were actually triggered twice during writing:**

1. `Split-Path` returns `\` on Windows, which doesn't match the forward-slash key → once falsely
   reported 499 orphans
2. **For non-`mod.rs` files, the submodule directory is `self-name/` rather than
   `parent-directory/`** → `orchestrator.rs`'s `mod tests` was resolved to
   `src/frontend/module/tests/`, nearly flagging 753 lines of live tests as dead code

Therefore **every conclusion has been double-confirmed via "actually read the file + actually search
for references"**, and items that didn't pass the second confirmation are not included in this
document.

### Line Count Standard

All line counts in this document use `(Get-Content).Count` statistics. **Note that
`Measure-Object -Line` skips blank lines, systematically undercounting by about 5%** (e.g.,
`manifesto.md` reports 658 with `-Line`, actually 893). An earlier analysis round in this repository
gave a lower total due to this standard, which has been corrected here.

### Known Limitations

- **Reachability analysis can only discover "unreachable", not "reachable but meaningless"**. Forms
  other than 2 require item-by-item manual judgment.
- **`pub` items don't trigger `dead_code` warnings in Rust**, so zero-call `pub` types need to be
  discovered by search; this method doesn't cover them.
- This document does not involve running the compiler, so all "never ran" conclusions are static
  determinations.

## A. Files That Never Participated in Compilation

**Total 24 files / 2628 lines.** Among them, `semantic_tokens.rs` is Form 4 (see Section F), the
remaining **23 files / 1081 lines are real orphans**.

| Path                                                          | Scale                   | Content                                                                       | Judgment Basis                                                                                                                                                                |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/tests/` **entire subtree 13 files**  | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` only declares 5 `pub mod`; `:104-106`'s only test declaration is `#[path]` to `tests/fstring.rs`. Directory-wide `grep 'mod tests'` only hits `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 files                        | 14 lines                | All placeholder doc comments, 0 assertions                                    | `src/frontend/pipeline.rs` (740 lines) full text has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration (`Select-String` zero hits)                                |
| `src/package/template/tests/` 3 files                         | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`**                     | `src/package/template/mod.rs` (7 lines) only declares `mod gitignore; mod main_yx;`, no `mod tests;`                                                                          |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden in a **live** directory                                                | `pratt/tests/mod.rs` only declares `led` / `nud` / `precedence`                                                                                                               |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same                                                                          | `passes/tests/mod.rs` only declares `dead_code` / `overload`                                                                                                                  |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same                                                                          | `emitter/tests/mod.rs` only declares `ansi` / `text`                                                                                                                          |

**The last 3 items total 16 tests / 311 lines of code that never ran.** They hide in active
directories, where the `mod.rs` looks complete, so they're more deceptive than an entire orphan
directory.

Among them, two more files in `lexer/tests/` are also in the "declared but missed declaration" form,
but already counted in the 629 lines of the first row, **not double-counted**: `lexer_mod.rs` (89
lines / 5 tests, `lexer/tests/mod.rs` declares 11 `mod` but no `mod lexer_mod;`), `symbols.rs` (70
lines / 3 tests, same as above).

**Real tests that never ran total 1005 lines / 78**: lexer 629/55 + pratt / passes / emitter
311/16 + package/template 65/7.

**Particularly note `precedence_inline.rs`**: it tests the `Precedence` enum + `PrecedenceContext`
in `precedence.rs` (96 lines of dead code) that has **zero production use**. Even if wired up, it
tests a dead path.

Another hidden in the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5 lines, 0
tests): `typecheck/tests/mod.rs` declares 23 test modules, missing only this one—the file has only
one `use` line, pure wreckage. When S1 revives, it will be deleted with the wiring or have
assertions added.

## B. Compiled but Zero-Call Dead Code

| #   | Location                                                                         | Scale               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | -------------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | Whole-repo word-boundary search, hits outside `instance.rs` all 0 (including tests). Source: RFC-023 declared deletion of about 850 lines, only half deleted                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` + 4 pure functions. Only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both are relays). Whole-repo `TypeSystem` hits only 5 places: 2 definitions + 1 re-export + **unrelated** `lexer/symbols.rs:503 TypeSystemValidator` 2. 4 methods have zero call sites. **Because it's `pub`, the compiler never warns**                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| B3  | `src/frontend/core/typecheck/layers/dispatch.rs` entire chain                    | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` zero calls. RFC-027 §4.1's runtime check insertion is unwired. Note it's declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` only called by tests. **Same file's `is_subtype` must be kept** (`inference/assignment.rs:17` production use)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-described as "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests)—and that test file itself is never compiled** (`pratt/tests/mod.rs` only declares `led`/`nud`/`precedence`). Therefore the two **must be deleted together**: deleting just the test leaves 96 lines of dead code, deleting just the enum causes test compilation failure                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`、`:120-122`、`:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, `skip_old_function_syntax` is called at `:725`—**they are running old-syntax rejection gates, not unreferenced dead code**. `skip_old_function_syntax` (`:120-122`) function body has only one line comment `// 旧语法已移除`, but **the function doesn't consume any tokens after being called**—this is an independent behavior issue (whether the rejection path correctly advances parse position), not dead code. **Disposition is to review behavior together with 05-frontend-paradigm.md (Decision D27: naturally deleted with grammar migration), not to delete as dead code directly**                                                                                                                                                                                               |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | Whole repo only has matches, no construction points. `bytecode.rs:2156`'s "no generation point (dead path)" comment is **accurate** (encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| B9  | `Switch` opcode                                                                  | —                   | `backends/common/opcode.rs` defines it, has variants, has `opcode()`/`size()` arms, has decode arm, **but whole repo has zero construction points**. Interpreter `ops/control.rs:90` has active implementation. **Has been explicitly registered as known dead path by tests**: `tests/bytecode.rs:1035-1040`'s `WHITELIST: &[&str] = &["SWITCH"]` (non-empty difference set means failure)                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` **ignoring the `op` field**, decoder `1236-1248` only constructs `Neg`. **Via `opcode()` mapping path silently degrades to I64_NEG** (when `.42` runs directly, `!x` becomes `-x`; interpreter memory path `ops/arith.rs:46-49` is correct). Only used by interpreter `ops/arith.rs:46,49` and `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| B11 | 4 `#[allow(dead_code)]` places                                                   | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (conservative exemption for extern binding, **reasonable to keep**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction** (only construction point is `parser/tests/ast.rs:828` test hand-crafting nodes), but has **12 production consumers + 2 exhaustiveness arms** (full list in 03-type-unification.md §2.6): `spawn/placement.rs:122`、`spawn/analysis.rs:912`、`formatter/handlers/expr.rs:43`、`frontend/module/orchestrator.rs:1492`、`ir_gen.rs:4325/5097`、`checker.rs:1642`、`semantic_tokens.rs:1416`、`inference/expressions.rs:3374`、`inference/existential.rs:55`、`passes/dead_code.rs:305`、`layers/ownership.rs:1207`、`layers/termination.rs:752`, also `ast.rs:1026` `Expr::span()`、`pratt/mod.rs:47` `expr_end_line`. Function definition actually goes through `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion falls to P6 (the 16-item change list in 03 §5.5) |

## C. Empty Designs

| #   | Object                                                                          | Scale                                | Fact                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | ------------------------------------------------------------------------------- | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                             | Describes `CheckSession` (including `:31-42` Rust code draft), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, `HotReloader` with **zero whole-repo hits**. `:46-47`'s "known limitations" self-admits `check_incremental` internally still calls `check_files_with_diagnostics` (full path); `:46` also references `command.rs`'s busy-wait debounce—**`command.rs` has neither `Instant` nor `recv_timeout`** (whole-repo `recv_timeout` only appears in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                             | `:37` claims `traits/` placeholder implementation (coherence / impl_check / object_safety / resolution) is incomplete—**`traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero whole-repo hits. `:38` claims `check_single_module` creates a separate Compiler for each file—**this function does not exist**, it only appears in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                               |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                             | `:14` claims "watch mode only re-checks affected files", `:27` claims "check only does the first two steps". Reality: `check_project` also does vendor consistency, project role classification, dead code analysis, W1006; `main.rs`'s `Check` subcommand **has no `--watch` flag** (all `#[arg]` declarations individually verified)                                                                                                                                                                     |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines design / **0 lines code** | `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` AOT hits are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell. `:861-876` itself leaves a full `- [ ]` implementation checklist, but status is marked `accepted`                                                                                                                                                                                                                                            |
| C5  | `RFC-028` JIT                                                                   | 419 lines design / **0 lines code**  | `028-jit-compiler.md:99-126` requires 6 files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **all do not exist**. Status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                           |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                         | **Physical evidence**: file names precisely correspond to `ModuleCache` and incremental scheduling in C1's design. **Someone built test skeletons according to the design, found no corresponding implementation in the pipeline, and put them aside**                                                                                                                                                                                                                                                     |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                    | `util/diagnostic/tests/mod.rs:4`'s `//! §6.1: CheckSession incremental check` and `mod_tests.rs:4`'s `§4.4` / `§5.2` reference this **gitignored planning document** (`.gitignore` contains `superpowers/`). These § references are **archaeological layer references to deprecated APIs**                                                                                                                                                                                                                 |

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

Therefore the disposition for C1-C3 is **not to delete the capability, but to delete the misleading
documents**, and rewrite `check/index.md`'s boundary table to point to the actual architecture in
`src/frontend/module/`.

### RFC-019 Disposition Needs Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` **is already in the `deprecated/` directory**,
and the residual check result is **clean** (`homoicon` zero whole-repo hits, document ends with an
honest "if restarted in the future, start from proof/prototype experiments, not from the RFC"
conclusion). **It needs no additional disposition**—this item is in the authorization list, but
after verification it's confirmed to be in a terminal state.

## D. Function-Level Dead Code: REPL

`src/repl/` 4 files **1152 lines**, **zero `#[cfg(test)]`, zero `#[test]`**. And `src/main.rs:440`
is `args.command.unwrap_or(Commands::Repl)`—**running `yaoxiang` with no arguments directly enters
REPL**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D1  | **`EvalResult::Value` is never constructed** | The whole `src/repl/` only appears 2 times: definition at `eval.rs:23`, **match arm** at `mod.rs:180`. Therefore `Repl::format_value` (`mod.rs:493-502`) is dead code, **REPL does not echo any evaluation results**                                                                                                                                                                       |
| D2  | **No cross-round state**                     | `extract_definitions` (`eval.rs:345-377`) only calls `define_variable` (stores **type strings**), never calls `define_var` (stores `RuntimeValue`); `wrap_code` doesn't inject historical definitions. `VariableInfo::Value` (`eval.rs:23`) branch is unreachable. `REPLContext.variables` purely serves completion, doesn't participate in evaluation                                     |
| D3  | **Wrapper syntax suspected outdated**        | `eval.rs:314`/`316` produces `main() -> () = () => { code }` (`name() -> Ret` old-style function declaration header), while current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`, RFC-007:94 canonical table is all `name: (a: Type) -> Ret`). **Not verified by running the compiler; if confirmed, REPL would fail to compile on every evaluation** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` only prints two lines of prompt and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                             |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                  |
| D6  | Duplicated with `lib.rs`                     | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent "auto-wrap main" logics**, with different wrap forms                                                                                                                                                                                                                                                            |

## E. Local Cleanup Items

| #   | Location                                                                                           | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                         | Root-level empty directory tree. Almost certainly from somewhere concatenating the string `"undefined"` as a path. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                              |
| E2  | `src/lib.rs:568`、`:582`                                                                           | `dump_type_detail` / `dump_const_detail` each has an `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, panic path is predictable. Should change to meaningful degradation output                                                                                                                                                                                                                                                                                                                                     |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                          | `check_vendor_lock_consistency` and `check_workspace_consistency` are **about 60 lines near-duplicate**, with only dependency source and path-dep skip differences. **Merge**                                                                                                                                                                                                                                                                                                                                                                       |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`             | Old function syntax detection and rejection, about 120 lines. Among them `skip_old_function_syntax` (`:120-122`) function body has only one line comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                          |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                   | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                            | "Tool design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, `check/incremental-checking`. **If executing C1-C3 document deletion, this must be cleaned up synchronously**, otherwise the doc site will have 404 links                                                                                                                                                                                                                                                                                            |
| E7  | Dual `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | 5 variants with the same name (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` has inconsistent bit widths (f32 × f64)**; no `From` / `TryFrom`, hand-written conversions scattered at call sites (`const_eval.rs` / `ir_gen.rs`). `const_data.rs:44` `kind()` for `LibraryRef` / `ExternRef` is live `todo!()`. **Disposition: merge into one + explicit conversion + `todo!()` change to degradation output, falls to P10 (E class local cleanup items)**. 2026-10-06 P0 gate registration (Criterion A, J=0.56)            |
| E8  | `Literal` downstream mapping synonymous duplication (Prohibition 3 A)                              | `Literal → MonoType` same-style arms ≥3 places (`inference/expressions.rs`, `checker.rs`, `inference/patterns.rs`'s `Literal::Int(_) => MonoType::Int(64)` etc.); `Literal → ir::ConstValue` (`ir_gen.rs` 19 hand-written places) and `Literal → const_data::ConstValue` (`const_eval.rs` 4 hand-written places), no shared conversion point. **Disposition: typecheck-side 3 places lifted to shared layer, falls to P6; `ir_gen` / `const_eval` side consolidated together with E7's `ConstValue` merge in P10**. 2026-10-06 P0 gate registration |

### Not Dead Code, but Same Category: Three Hard-Coded Drops

`src/middle/core/bytecode.rs`'s `impl From<BytecodeFile>` (`943-2350`, 1400 lines) has 3 hard-coded
drops:

| Line   | Dropped Content                  | Consequence                                   |
| ------ | -------------------------------- | --------------------------------------------- |
| `2312` | `upvalue_count: 0`               | upvalue meta info lost                        |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifact loses exception table**      |
| `2341` | `globals: Vec::new()`            | **`.42` artifact loses global variable info** |

**These three are not dead code, they are data loss defects**—`.42` artifacts are only complete
under the memory path. **Disposition is different from dead code**: ~~should be tracked as
independent issue~~ **all consolidated into P7 batch e (Decision D52, 2026-10-05), no independent
issue kept**—`2315` (exception table) and `2341` (global variables) are product integrity of
language core semantics, not "postponable" items. All three fixed together, `.42`'s `VERSION`
upgraded from 4 to 5 in one go (format header already has `MAGIC` + `VERSION` fields,
`codegen/bytecode.rs:14-16`). Listed here only to avoid being mistaken as "dead code" for cleanup.

## F. Not Deleted, but to be Marked or Decided

| #   | Object                                                                                                                                                                                                                                                              | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (1547 lines)                                                                                                                                                                                               | **Not dead code**, is `include!`'d via `checker.rs:5618`—**the only `include!` in the whole repo**. This file **is not declared as any module** (`grep 'mod semantic_tokens'` zero whole-repo hits), first line directly is `impl TypeChecker {`, no own `use` header, `checker/` directory has only this one file. Consequences: no module identity, visibility isolation fails, rust-analyzer jump and symbol search fails, toolchain counts it as part of `checker.rs`. **`checker` module's real body is 5618+1547 ≈ 7165 lines**. **Fix: change to real `mod`** (construction steps go to [09](09-execution-wbs.md) §P5 5.1), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| F2  | wasm attributes (same line's `#[cfg(... wasm32 ...)]`, both positive and negative counted): current tree `src/` **29** files have `wasm32` literal, **22** with actual attributes, **7** only mentioned; `orchestrator.rs` **10** entries (4 positive + 6 negative) | **Not dead code.** wasm target is carried by **independent shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`、`:18` `wasm-bindgen = "0.2"`、`:17` depends on main crate; root `Cargo.toml:3`'s `exclude = ["wasm", ...]` exactly indicates it's an independent crate. Only looking at root `Cargo.toml:31`'s `crate-type = ["rlib"]` would misjudge these branches as defensive dead code—**that inference doesn't hold**. CI's wasm build is in `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), callers `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches are load-bearing**: they determine which APIs of the main crate can be called from `wasm/src/lib.rs` (currently **152** lines) under the wasm target (`src/lib.rs:151` / `:170` / `:187` gate `run_file` / `run_project` / `build_bytecode` entirely). **Cleanup range already determined**: 4 wasm arms are unreachable at runtime in playground scenario, but compile-time load-bearing (call sites not gated); 6 `not(wasm32)` arms are load-bearing in native three-stage table ⇒ **no directly deletable low-risk items**—item-by-item judgment and evidence in next section "F2 Judgment" (Decision D35: delete unreachable, keep reachable, falls to P10) |
| F3  | `docs/src/rfc/TRACKING.md` column structure                                                                                                                                                                                                                         | Only has "Status" column (document status), **no "Implementation Status" column**—it **cannot express** the error of "RFC marked complete but code not done". Whole repo only 5/52 RFCs have `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **26 accepted RFCs completely don't record implementation status**, including 009, 009a, 013, 018, 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                      | **Already disposed (4.4.1)**: README now describes the actual call sequence of `checker.rs::check_module_impl` (refinement traversal `predicate` → `termination` → `ownership` → type constraint solving), bracket line numbers taken from current tree, positioning anchored by function name/drive point; no longer declares "lower layer failure upper layer doesn't run" (actually no short-circuit); `equivalence` is described by its real identity as "shared type comparison utility + proof entry used only by tests". This line is only a disposed registration, no longer points to old line range                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                             | `:79` puts RFC-026a in the "Accepted RFCs" table but status column writes "RFCs under Review"; `:48-49`'s "RFCs under Review" section only lists 032 and 027a, missing 026a. Inconsistent with `TRACKING.md`'s 3 reviews                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| F6  | `tools/cargo-dist/` (`dist.exe` 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                          | **Neither tracked by git** (`git ls-files` returns empty), excluded by `.gitignore`. **Not a problem**, only recorded to prevent subsequent audits from misjudging as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` takes to next quote), depends on `("E1001", ...)` tuple syntax, **not a parser**. Its architecture (`parse` + `validate` + build.rs gate + `--fix` cure) is worth reusing for opcode table, but **extractor needs to be rewritten according to opcode form (about 30 lines)**. Falls to 06-cleanup-inventory.md follow-up work or independent RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F8  | `src/backends/common/opcode.rs` gate gap                                                                                                                                                                                                                            | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode fact is expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / decode match (`bytecode.rs:978-2302`) / 48 `translate_*` (`translator.rs:676-1577`). **Compiler only enforces 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + each `ops/*.rs` family function's exhaustive match), missing one in `opcode()` / `size()` / decode arm / encoder **only blows up when running `.42` artifacts**. `size()` table (649-803)'s comment self-description doesn't match actual encoding (`bytecode.rs:2181-2182`), and **has no systematic cross-check test** (existing size tests only cover Nop/Mov/Borrow/Release 4). Operator semantics also take a third parallel enumeration here ( `BinaryOp` / `UnaryOp` / `CompareOp`, `bytecode.rs:70/97/106`, `Rem`/`Xor`/`Sar` another naming set), consolidated together with opcode generation-time gate. Vocabulary target location is `middle/bytecode/opcode.rs` (migrated with bytecode domain merge, eliminating L3→L4 reverse, see 01)                                                                                                                                                              |

### F2 Judgment: `orchestrator.rs` wasm Attribute Reachability (2026-10-18 Judgment, not including cleanup)

**Judgment face** = wasm production build (`wasm-pack` produced shim crate, see F2 row); **judgment
revision** `b5e68b3c9`. This section only registers judgment conclusions, doesn't generate any
cleanup commitments (cleanup goes to P10 / D35).

#### I. Count Standard (Re-taken from Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- Current tree `orchestrator.rs` has **10** wasm attributes in total = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- Standard = **`#[cfg(...)]` attribute text within the same line**, both positive and negative
  counted (this is also the standard used in 02's "12"); runtime macro `cfg!(...)` not
  counted—whole-repo `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'` zero hits.
- "12" is not an error, it's a **fact of the old revision**: `544695ea9` (first commit of RFC-039
  design document) at that time was 5 positive + 7 negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- Difference source is **a pair of attributes moved out as a pair**: `6ec330d22` (WBS 4.2.2,
  `check_project` slimmed to `Program` constructor, that commit net-deleted 252 lines from
  `orchestrator.rs`) moved `role_context` together with its `not(wasm32)` implementation and
  `wasm32` stub **to** `src/driver/helpers.rs:360/394`—`orchestrator.rs` therefore went from 12 to
  10, the mechanism itself didn't disappear.
- Per-revision review (actually executed this time; below is excerpt, `6ec330d22` is the old/new
  boundary):

```console
$ for rev in $(git log --format=%h -20 -- src/frontend/module/orchestrator.rs); do
    pos=$(git show $rev:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]')
    neg=$(git show $rev:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]')
    echo "$rev pos=$pos neg=$neg total=$((pos+neg))"
  done
… b8b3cc3f6 pos=4 neg=6 total=10   # current HEAD side
… 6ec330d22 pos=4 neg=6 total=10
… 5ba52eafb pos=5 neg=7 total=12   # earlier
```

#### II. Three-Question Standard (Without Separation Load-Bearing Arms Will Be Misjudged as Dead Code)

1. **Compile-time existence**—whether that arm participates in compilation under that target;
2. **Runtime reachability**—whether there's a call chain from the playground entry under that
   target;
3. **Deletion cost**—whether deleting it breaks that target's compilation (whether call sites are
   similarly gated).

#### III. Judgment Starting Point (Playground Entry and Stage Table)

- Entry: `wasm/src/lib.rs` (currently 152 lines) `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, only constructs `Pipeline`, doesn't touch
  orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` goes through `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`; `:135`
  where `SingleFile | WasmPlayground` share the table): Parsing / Typecheck / DeadCodeAnalysis /
  ProofExecution / IrGeneration / Monomorphization—**does not contain** VendorConsistency /
  Discovery / Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### IV. Item-by-Item Judgment (Current Tree 10 Items)

| #   | Location (`orchestrator.rs`) | Gated Object                                                            | Compiled in wasm build                 | Playground Runtime Reachable                                                                                                                                                   | Deletion Cost (Whether Call Sites Are Gated)                                  |
| --- | ---------------------------- | ----------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| 1   | `:271` positive              | `ensure_vendor_consistency` (`:267`)'s wasm arm (`Ok(None)`)            | Yes                                    | **No**: only call site `src/driver/arms.rs:1195` (VendorConsistency arm) not in playground stage table                                                                         | High: `arms.rs:1195` not gated, deleting arm alone ⇒ wasm compilation failure |
| 2   | `:418` positive              | `discover_with_used` (`:400`)'s wasm arm (`lock_versions` set to empty) | Yes                                    | **No**: call sites `arms.rs:1221` (Discovery arm), `src/driver/helpers.rs:686` (Check form reference pool `collect_package_refs`, only `arms.rs:521`'s Check branch calls)     | High: same as above                                                           |
| 3   | `:655` positive              | `member_context`'s wasm stub (`:656-659`)                               | Yes                                    | **No**: only call site `:404` (inside #2's function body)                                                                                                                      | High: `:404` not gated                                                        |
| 4   | `:753` positive              | `dep_import_surface`'s wasm stub (`:754-756`)                           | Yes                                    | **No**: call sites `:671` (`resolve_in_package_root`) and `:813` (`resolve_in_vendor_with_lock`), rooted in `:468`'s `resolve_module_path` (inside `discover_with_used`'s BFS) | High: same as above                                                           |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                        | No (doesn't participate in wasm build) | N/A                                                                                                                                                                            | native load-bearing: used by `:678` / `:747`                                  |
| 6   | `:276` negative              | `ensure_vendor_consistency`'s native arm                                | No                                     | N/A                                                                                                                                                                            | native load-bearing: VendorConsistency in `program.rs:61/98/113` three tables |
| 7   | `:413` negative              | `discover_with_used`'s native arm (lock reading)                        | No                                     | N/A                                                                                                                                                                            | native load-bearing: Discovery in `program.rs:62/99/114`                      |
| 8   | `:607` negative              | native `member_context` (workspace resolution)                          | No                                     | N/A                                                                                                                                                                            | native load-bearing: call site `:404`                                         |
| 9   | `:677` negative              | `resolve_in_package_root`'s manifest name reading                       | No                                     | N/A                                                                                                                                                                            | native load-bearing: `:544/:547` ← `:468`                                     |
| 10  | `:745` negative              | native `dep_import_surface`                                             | No                                     | N/A                                                                                                                                                                            | native load-bearing: `:671` / `:813`                                          |

#### V. Conclusions

1. **4 wasm arms are unreachable at runtime in playground scenario** (#1-#4, evidence = stage table
   doesn't contain the stage where their only call site resides), but they are **compile-time
   load-bearing**. Executing D35's literal "delete unreachable ones" directly will fail to
   compile—feasible form is "gate the entire project-mode subtree (orchestrator.rs's
   discovery/resolution functions + driver's VendorConsistency/Discovery two arms + `helpers.rs`'s
   reference pool set) on wasm32", cost and benefit fall to P10 ruling.
2. **6 `not(wasm32)` arms don't exist in playground build** (compile-time excluded), all native side
   load-bearing (MultiFile / Check / Lsp three stage tables). "Playground unreachable ⇒ delete" is
   **not applicable** to them: deleting by that standard would directly break CLI/LSP.
3. ⇒ This directory **has no "directly deletable and low-risk" items**; F2's cleanup benefit only
   falls on "whether wasm artifact becomes smaller"—that item is unverified (see below).

#### VI. Unverified (Must Not Be Used as Conclusions)

- **Wasm artifact size benefit unmeasured**: didn't run `wasm-pack build` and size/symbol table
  tools (`twiggy` / `wasm-objdump`), "how much product can be shrunk by gating project-mode subtree"
  has no data; whether link-time has stripped unreferenced `pub(crate)` items also unverified.
- **Wasm32 consumers outside shim not exhausted**: this round only verified in-repo CI (only wasm32
  compilation is in `_build-wasm.yml:30/77`, via shim crate). If third parties compile main crate
  directly to `wasm32-unknown-unknown` and call project-mode APIs, reachable face will exceed this
  judgment.
- **Native side "load-bearing" = static call chain + stage table**, not verified item-by-item
  end-to-end runtime (didn't run MultiFile/Check/Lsp probes to touch each arm one by one).

#### VII. Appendix: Repo-Level Count Re-verification (Same Revision)

```console
$ git grep -l wasm32 b5e68b3c9 -- '*.rs' | wc -l
30        # includes wasm/ shim crate
$ git grep -l wasm32 b5e68b3c9 -- 'src/*.rs' | wc -l
29
$ git grep -lE '#\[cfg\([^]]*wasm32' b5e68b3c9 -- 'src/*.rs' | wc -l
22        # with actual cfg attributes
$ git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs' | wc -l
0         # no runtime cfg! standard
```

02 § "wasm current state" and F2 row's "27 files / 25 with attributes / 2 only mentioned" are old
revision numbers; current tree is **29 / 22 / 7** (7 only mentioned: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Criterion Discovery Registration (Continued with Each Phase Construction from 2026-10-06)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibition 1 A/B/C/D) first
ran on 2026-10-06. Following the division of labor "gate only reports suspected, human rules"
([08](08-maintenance-mechanism.md) §Known Limitations), this section registers ruling results and
disposition assignment. Unexpanded hits (Criterion A violation layer 25 pairs, Criterion B all
suspected items) are reviewed pair by pair during corresponding phase construction.

| #   | Object                                                                                                                     | Ruling                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Disposition Assignment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                              | **Legitimate structure, not parallel representation**: value/type label pair, maintained 1:1 by exhaustive `value_type()` (`value.rs:326-369`), missing arm modification causes compilation failure. But `ValueType`'s external consumption essentially only remains in `{:?}` formatting on std/os/fs/net error paths and `executor.rs:410`'s constant `Void`, 21 variants far exceed actual needs                                                                                                                                                                                                                                                                                                                                                                                                                  | P10 S5 slimming candidate: downgrade `pub(crate)` and evaluate pruning; S5 construction-time rule together with necessity of `AsyncValue.value_type` field (`value.rs:217`)                                                                                                                                                                                                                                                                                                                                                                                  |
| G2  | Dual `ConstValue` (Criterion A, J=0.56)                                                                                    | **True positive**, includes f32/f64 bit width inconsistency and `kind()` live `todo!()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Already in E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| G3  | Type name string dispatch 11 places (Criterion D true positive)                                                            | **True positive, same root cause**: built-in container names have no single authoritative identification point. List: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), `eval/const_eval.rs:498-511` size table (including `"Uint"` dead arm), `middle/passes/codegen/bytecode.rs:744-756` `to_type_id` (`"Set" => 25` contradicts #300 decision 4), `trait_data.rs:187`'s `"Clone" \| "Dup"`                                                                                                                                                                                                        | **P6 extended verification item**: these points are on typecheck / eval / middle side, not within 03 §5.4's parser data flow coverage; P6 construction-time follow the same principle as 03 §5.3 (`NameKind` set once, match arm no longer does string comparison) to dispose together. Copy table has been corrected to seven places (03 §2.3)                                                                                                                                                                                                              |
| G4  | Criterion D false positive 4 places                                                                                        | **Legitimate lookup / dispatch table, not bridging**: `package/build/requirements.rs:17-25` tool installation guidance, `package/vendor/mod.rs:134-139` build result display name, `repl/mod.rs:241-396` REPL command aliases (8 arms, user interface aliases are legitimate UX), `util/diagnostic/mod.rs:462-468` `parse_runtime_mode` (with comment historical aliases, single boundary point)                                                                                                                                                                                                                                                                                                                                                                                                                     | P9 hardening-time add `// reason:` exemption per place; exemption count reviewed with gate report                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| G5  | `ReleasePlan.drops` span key mismatch (P2 2.4.4 criterion actual test, not P0 hit)                                         | **True positive, contract defect—D20 prophecy confirmed**: ownership records keys by "last use **expression** span" (`ownership.rs:2574 build_release_plan`), `ir_gen.rs:1956` looks up table by "**statement** span", expression-level keys never hit—whole corpus actual test **168 files / 373 keys** not consumed, Drop silently lost (per-case evidence in `src/middle/core/tests/release_plan_spans.rs` file header)                                                                                                                                                                                                                                                                                                                                                                                           | **P4/D20** (Span key → PlanId, behavior face about 168 files, must wait for C2 criterion and corpus diff to be in place); criterion `test_release_plan_spans_consumed` red-state resident (`#[ignore]`), D20 landing removal turns green                                                                                                                                                                                                                                                                                                                     |
| G6  | IR metadata three defects (P2 2.1.2 criterion `verify_loose` corpus green-run process actual test, D38 prophecy fulfilled) | **True positive ×3, all fixed with 2.1** (corpus diff baseline self-compare zero diff certifies behavior equivalence): ① Method IR's locals table truncated to parameter count (`ir_gen.rs` original `take_cur_locals(param_types.len())`)—local_count underestimated, runtime relies on `Frame::set_slot` out-of-bounds resize (`frames.rs:103-105`) silent fault tolerance; ② Lambda / curry middle layer / function path's un-annotated parameters discarded by `filter_map` (three places), params length and frame layout mismatch—aligned with method path's existing convention fill `Int(64)` placeholder; ③ match result slot depends on frame implicit Void pre-initialization (last arm guard failure tolerance landing has undefined path on CFG)—`generate_match_expr_ir` entry explicitly pre-set Void | Already fixed (2.1 same batch). **Judgment face narrowing registration** (verify.rs file header is authoritative): temp slot dominance not checked (Void pre-initialization is frame semantics, ir_gen intentionally depends on); type-consistent trusted source narrowed to parameter slot signature + constants (ir_gen never writes `LocalSlot.ty`, entire table placeholder `Int(64)`—full judgment face falls to P7 value table 04 §262); `init_locals` is #368 debugging-view table not allocation upper limit, init's Local out-of-bounds not checked |

## H. Giant Function Inventory (Pre-Release Audit Standard, from 2026-10-16)

> Standard (2026-10-16 `/pre-release-audit` set): function span > 150 lines is the **initial
> screening threshold**, not a "report every one" command—a full list would turn the inventory into
> line-count piling, drowning this new signal in the stock. Report items must include three numbers:
> **host line count / this insertion line count / line count before insertion**. Two categories:
> **A. Existing giant functions with inserted branches** (>150 lines and this insertion >50 lines);
> **B. New large functions this time** (almost entirely new and >100 lines). Line numbers in the
> table below are from audit revision `1935aab53` (range `v0.8.4..1935aab53`).

### A. Existing Giant Functions with Inserted Branches

| Function                | Location                                                    | Host Lines | This Insertion | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | -------------- | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1594       | 358            | 1236             |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171            | 642              |

### B. New Large Functions This Time

| Function                     | Location                                            | Lines | New This Time |
| ---------------------------- | --------------------------------------------------- | ----- | ------------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177           |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122           |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130           |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107           |

**Existing giant functions not in inventory** (this insertion ≤ 4 lines, only registered not
disposed): `infer_expr` (`expressions.rs:2999`, 955 lines), `collect_function_signature`
(`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`, 441 lines),
`generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), etc.

### Disposition Orientation

- **Class A**: dissolved together with the subsequent splitting tasks of `infer_call_expr` /
  `generate_call_expr_ir`—both are the heaviest hosts in the repo, this named-actual-parameter
  refactor inserted 358 lines into the former.
- **Class B**: `push_joined` is a **local** helper for `Task` work-list style type formatting (its
  host is the 180 lines new), with `mono.rs:123 increment_level_string` (174 lines, does level name
  rewriting) **different responsibilities, not duplicated, don't merge**; `consumer_stage_skipped`
  and `local_count_of` can be split independently.

## Key Decisions and Rationale

The authorization is "thorough deletion permitted", but this document gives **different
recommendations** for several items:

| Category                                                                | Disposition                                  | Rationale                                                                                                                                                                                                                                                                                                           |
| ----------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions, no implementation) | **Delete**                                   | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty functions), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                       |
| **Empty design documents**                                              | **Delete or rewrite**                        | C1, C2 delete then rewrite to point to actual implementation; C3 rewrite boundary table; **must synchronously clean up `config.js:259-268`** (E6)                                                                                                                                                                   |
| **RFCs with status not matching implementation**                        | **Change status**                            | C4 (RFC-018) move back to `draft` or add `impl_status: 'not-started'`; C5 (RFC-028) add `impl_status: 'not-started'`. **Don't delete**—the design itself may have value, the problem is status marking distortion                                                                                                   |
| **Orphan tests with real assertions**                                   | **Revive, not delete**                       | Total **1005 lines / 78 tests**: lexer 629 lines / 55 + 311 lines / 16 hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7. **Revival cost is one `mod` declaration line, value is letting CI regain these coverage**. Deleting them is net loss |
| **Zero-call but semantically important `pub` items**                    | **Downgrade visibility not delete**          | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move to `pub(crate)` to let compiler identify, or add implementation. **B4's `is_subtype` must be kept**                                                                                                                                                           |
| **Opcodes / instructions that can't produce values**                    | **Add construction points or delete opcode** | B8 (`TailCall`), B9 (`Switch`, opcode roundtrip test has explicitly registered with `WHITELIST=["SWITCH"]`), B10 (`UnaryOp::Not` silently degrades). Three choices, can't maintain status quo; D32/D33/D34 have ruled to delete, handled together with `.42` upgrade                                                |
| **Data loss defects**                                                   | ~~Independent issue~~ **P7 batch e (D52)**   | E section's three hard-coded drops—not dead code                                                                                                                                                                                                                                                                    |
| **No module identity but effective**                                    | **Refactor not delete**                      | F1 (`semantic_tokens.rs`) change `include!` → real `mod` (see `02`)                                                                                                                                                                                                                                                 |
| **Needs decision**                                                      | **List as open question**                    | F2's **unreachable wasm branches** cleanup range, layer order correction will expose new diagnostics                                                                                                                                                                                                                |

### Directions Not Adopted

- **All handled as "thorough deletion", including orphan tests**—1005 lines of tests with real
  assertions are assets. Deletion cost is one `mod` declaration line, benefit is CI regaining
  coverage. The authorization is "may delete", not "should delete".
- **Only add gate without moving code**—gates prevent regression, but B1's 415 lines, dispatch
  chain, 96 lines of dead steps **already exist**, gates will only freeze them on the baseline.
- **Keep RFC-018 / RFC-028's `accepted` status and only add documentation**—`rfc/index.md:131`
  defines "accepted = enters implementation phase", readers will reasonably assume implementation
  exists. `TRACKING.md` already has `impl_status` field precedents (5 RFCs using it), cost of adding
  fields is far less than the cost of status distortion.

## Implementation Essentials

**This document does not need equivalence criterion** (belongs to
[C6 pure deletion category](07-equivalence-oracle.md)), but **still needs item-by-item confirmation
of no references**. In RFC-039's global phase sequence, S1 corresponds to P1, S2-S6 correspond to
P10.

| Phase | Content                                                                                                                                | Acceptance                                                                                                                                                                                                                                       |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| S1    | Revive orphan tests with assertions (**1005 lines / 78 tests**)                                                                        | `cargo test` shows test count rise in five modules: lexer / pratt / passes / emitter / template; **expected to expose real defects** (`literals.rs` overflow path, `\x`/`\u` illegal escape paths never tested before), need to reserve fix time |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 with 05 closing/C6/E1)                                                                  | `cargo test` passes; `cargo clippy -D warnings` no new warnings                                                                                                                                                                                  |
| S3    | Empty design document disposition (C1-C3 + E6 synchronous cleanup `config.js`)                                                         | Doc site build no 404; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                   |
| S4    | RFC status correction (C4/C5/F3—add "Implementation Status" column to `TRACKING.md`)                                                   | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` will auto-regenerate, **don't hand-edit**)                                                                                                                                          |
| S5    | `pub` items downgrade visibility (B2/B4)                                                                                               | Compiler can point out real dead code                                                                                                                                                                                                            |
| S6    | Opcode value-less items decision landing (B8/B9/B10/F8, per D32/D33/D34)                                                               | Delete `Switch` / `TailCall` dead paths; B10's `opcode()` distinguishes `op` field; F8 gate landing                                                                                                                                              |
| S7    | `include!` refactor (F1, construction steps go to [09](09-execution-wbs.md) §P5 5.1) + layer order correction (F4, goes to P4's 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                       |

**S1 must be done first, and must reserve defect fix time.** Its value is not "cleanup", but
**letting CI regain 1005 lines / 78 tests that never ran**—before that, any judgment about test
coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. `literals.rs`'s four radix scanners have 4 isomorphic "after
  overflow continue consuming but don't error" branches, the entire `scan_leading_dot` function,
  `\x`/`\u` illegal escape paths, all never tested. After reviving tests, code likely needs to be
  fixed, this will delay subsequent refactoring.
- **B9/B10 involve language behavior decisions**. Whether `Switch` and `UnaryOp::Not` are "add
  construction points" or "delete opcode" needs to consider whether they belong to surfaces depended
  on by released corpus.
- **After C1-C3 deletion, cross-file analysis capability documentation will be briefly absent**,
  until `check/cross-file-analysis.md` is rewritten to point to the actual architecture of
  `src/frontend/module/`.
- **Reachability analysis can only discover "unreachable", not "reachable but meaningless"**.
  Judgments other than Form 2 must be done manually item by item, and **this inventory does not
  constitute exhaustive proof**.
- **`pub` items don't trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  won't be automatically discovered by the compiler, can only be found by search.
- **Deleting RFC-023's half-remainder** (B1) can fulfill the "closure has been deleted from the
  language" decision; **correcting status distortion** (C4/C5/F3) can restore `TRACKING.md`'s
  reference value.

> **The originally listed open questions in this section have all been decided.** Item-by-item
> decisions see [RFC-039 Decision Registration](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document has no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  B1 residue source decision
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — B3 unwired `§4.1` runtime check insertion
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — C1/C6 formal carrier
- [RFC-039 Compiler Function Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — Upper-level master
- [Refactoring Equivalence Criterion](07-equivalence-oracle.md) — C6 category criterion requirements
- `src/frontend/core/typecheck/layers/README.md` — Proof layer's actual call sequence and
  `equivalence`'s real identity (F4 already disposed)
- `build.rs:19-55` — Example of this repo's generation-time gate
