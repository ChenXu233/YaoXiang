# Dead Code and Unused Design Cleanup

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, grading of acceptance criteria, and execution stage order are in the RFC-039
> main text; the positioning of each subsidiary document is in the [directory index](index.md).

## Positioning and Scope

This document provides the YaoXiang repository's dead code and unused design inventory,
determination methods, per-item disposition decisions, and execution order. There are four cleanup
target categories:

| Category                                                                     | Scale                                                                   |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Files that never participated in compilation                                 | 24 files / 2628 lines (of which 23 files / 1081 lines are real orphans) |
| Compiled but zero-call dead code                                             | 1500+ lines                                                             |
| Design documents describing an architecture that never existed               | 3 documents under `docs/src/dev/design/check/`                          |
| Functionality hooked into the default entry, with zero tests and zero output | `src/repl/` 1152 lines                                                  |

**The authorization scope is "deletion is permitted"** (decision of 2026-10-03). However, this
document provides **disposition recommendations opposite to "delete directly"** for several
items—see [Key Decisions and Rationale](#key-decisions-and-rationale) for reasons.

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), oracle design (see `07-equivalence-oracle.md`). The
entire `layers/dispatch.rs` chain recorded in B3 is unwired; its deletion action belongs to S2.

## Current State

### Three Liability Forms of Dead Code

**Form 1: Placeholder.** Code exists, looks like it's doing something, but doesn't actually do it.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero calls** in the
  compiler. It is the "runtime check insertion" mechanism designed in `RFC-027 §4.1`, **completely
  unwired**.
- `check_type_equivalence` (`src/frontend/core/typecheck/layers/equivalence.rs:216`, `ProofContext`
  entry) is only called by tests. **Note** that the same file's `is_subtype` is used in production
  by `inference/assignment.rs:17`, so the **entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler; its `pub` surface only
  has `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` / `RuntimeStats`
  / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend**—the `Executor` trait
  at `backends/mod.rs:356` currently has only one implementer (the interpreter). This needs to be
  clarified because the directory name is misleading.

**Form 2: Residue.** The upstream decision has been made, but deletion was only half-done.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) leaves 6 types: `FunctionInstance`
  (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), `CaptureVariable`. **Source is clear**: when `RFC-023`
  deprecated the "closure capture model", it stated "**approximately 850 lines of code to be deleted
  along with the closure model**". These 415 lines are the surviving residue from those 850
  lines—**deletion was only half-done**. **Verification**: doing a whole-repo word-boundary search
  for these 6 types, the hit count **outside `instance.rs` is 0** (including tests). The
  `pub use passes::mono::instance::*;` at `middle/mod.rs:40` is just a glob relay and does not
  constitute consumption.

**Form 3: False coverage.** This is the most dangerous kind—**it makes the repository look more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only 5 `pub mod` declarations (`literals` / `state` / `symbols`
/ `tokenizer` / `tokens`) + one `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`
(`:104-106`). **Nowhere is `mod tests;` declared**, so the entire `lexer/tests/` subtree of 13 files
/ 686 lines **never participated in compilation**.

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

(`fstring.rs` 90 lines / 11 tests does compile, but via the `#[path]` bypass rather than
`mod tests;`; `mod.rs` 38 lines is a declaration file with no assertions.)

**The lexer layer has only one file actually running, 12 tests, all testing only f-string's brace
raw protocol.**

The sneakiness of this kind of problem is: `cargo test` will not fail (tests that never ran won't
fail), and CI has no test count baseline or wiring check.

### Phantom Design

The three documents under `docs/src/dev/design/check/` describe a **never-existed** architecture.
This is not "designed but not built", but rather **the documents describe another system that never
existed**, and present "possible future designs" as "defects of an already-implemented system".

## Methods and Credibility Statement

### Reachability Analysis Method

Starting from the two crate roots `src/lib.rs` / `src/main.rs`, parse all 509 `.rs` files' `mod` /
`pub mod` / `#[path]` declarations, build the module graph and perform BFS reachability analysis.
`include!` targets are judged separately (not counted in reachability).

**This method has known misjudgment patterns, which actually triggered twice during writing:**

1. `Split-Path` returns `\` on Windows, not matching the forward-slash keys → once falsely reported
   499 orphans
2. **The submodule directory of a non-`mod.rs` file is `self_name/` rather than `parent_dir/`** →
   `orchestrator.rs`'s `mod tests` was parsed as `src/frontend/module/tests/`, almost misjudging 753
   lines of living tests as dead code

Therefore **every conclusion has been double-confirmed via "actually read file + actually search
references"**, and items that did not pass secondary confirmation do not enter this document.

### Line Count Convention

All line counts in this document are counted with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines, systematically undercounting by about 5%** (e.g.,
`manifesto.md` reports 658 with `-Line`, actual 893). An earlier round of analysis in this
repository gave an undercounted total due to this convention, which has been corrected here.

### Known Limitations

- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Forms
  beyond Form 2 require manual judgment item by item.
- **`pub` items in Rust do not trigger `dead_code` warnings**, so zero-call `pub` types need to be
  discovered by search; this method does not cover them.
- This document does not involve running the compiler, so all "never ran" conclusions are static
  determinations.

## A. Files That Never Participated in Compilation

**Total 24 files / 2628 lines.** Among them, `semantic_tokens.rs` belongs to Form 4 (see Section F),
and the remaining **23 files / 1081 lines are real orphans**.

| Path                                                          | Scale                   | Content                                                                       | Determination Basis                                                                                                                                                                       |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/tests/` **entire subtree 13 files**  | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` only declares 5 `pub mod`; `:104-106` is the only test declaration, `#[path]` to `tests/fstring.rs`. `grep 'mod tests'` in the entire directory only hits `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 files                        | 14 lines                | All placeholder doc comments, 0 assertions                                    | `src/frontend/pipeline.rs` (740 lines) has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declarations (zero `Select-String` hits)                                                     |
| `src/package/template/tests/` 3 files                         | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`**                     | `src/package/template/mod.rs` (7 lines) only declares `mod gitignore; mod main_yx;`, no `mod tests;`                                                                                      |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden in a **living** directory                                              | `pratt/tests/mod.rs` only declares `led` / `nud` / `precedence`                                                                                                                           |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` only declares `dead_code` / `overload`                                                                                                                              |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` only declares `ansi` / `text`                                                                                                                                      |

**The last 3 items total 16 tests / 311 lines of code that never ran.** They hide in active
directories, whose `mod.rs` looks complete, so they are more deceptive than entire orphan subtrees.

Two more files inside `lexer/tests/` also belong to the "declared but missing declaration" form, but
have been counted in the 629 lines of the first row above, **not double-counted**: `lexer_mod.rs`
(89 lines / 5 tests, `lexer/tests/mod.rs` declares 11 `mod` but no `mod lexer_mod;`), `symbols.rs`
(70 lines / 3 tests, same as above).

**Real tests that never ran total 1005 lines / 78**: lexer 629/55 + pratt / passes / emitter
311/16 + package/template 65/7.

**Special attention to `precedence_inline.rs`**: it tests the **production-zero-use** `Precedence`
enum + `PrecedenceContext` (96 lines of dead code) in `precedence.rs`. Even if it's wired up, it
tests a dead path.

Another item hidden in the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, 0 tests): `typecheck/tests/mod.rs` declares 23 test modules, but missed this one—the file is
left with only one `use` line, pure wreckage. When S1 revives it, delete along with wiring or add
assertions.

## B. Compiled but Zero-Call Dead Code

| #   | Location                                                                         | Scale               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | -------------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | Whole-repo word-boundary search, hits outside `instance.rs` are all 0 (including tests). Source: RFC-023 declared deletion of about 850 lines, only half was deleted                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` + 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both relays). Whole-repo `TypeSystem` hits only 5: 2 definitions + 1 re-export + **unrelated** `lexer/symbols.rs:503 TypeSystemValidator` 2. 4 methods have zero call sites. **Because it's `pub`, the compiler never warns**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B3  | Entire `src/frontend/core/typecheck/layers/dispatch.rs` chain                    | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` have zero calls. RFC-027 §4.1's runtime check insertion is unwired. Note it's declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` is only called by tests. **The same file's `is_subtype` must be kept** (production use at `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-described as "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests)—and that test file itself is never compiled** (`pratt/tests/mod.rs` only declares `led`/`nud`/`precedence`). So the two **must be deleted together**: deleting only the test leaves 96 lines of dead code; deleting only the enum causes test compilation failure                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, `skip_old_function_syntax` is called at `:725`—**they are running old-syntax rejection gates, not unreferenced dead code**. The `skip_old_function_syntax` function body (`:120-122`) has only one line of comment `// 旧语法已移除`, but **the function does not consume any tokens after being called**—this is an independent behavior issue (whether the rejection path correctly advances the parse position), not dead code. **Disposition is to review behavior together with 05-frontend-paradigm.md (decision D27: naturally deleted with grammar migration), not to delete directly as dead code**                                                                                                                                                                                               |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | Whole repo only has match, no construction site. The "no generation point (dead path)" comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B9  | `Switch` opcode                                                                  | —                   | `backends/common/opcode.rs` has it defined, has variants, has `opcode()`/`size()` arms, has decode arm, **but whole repo has zero construction sites**. Interpreter `ops/control.rs:90` has active implementation. **Has been explicitly registered as a known dead path by tests**: `tests/bytecode.rs:1035-1040` `WHITELIST: &[&str] = &["SWITCH"]` (non-empty difference fails)                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` **ignoring the `op` field**, decoder at `1236-1248` also only constructs `Neg`. **Silently degenerates to I64_NEG via the `opcode()` mapping path** (when running `.42` directly, `!x` becomes `-x`; interpreter memory path `ops/arith.rs:46-49` is actually correct). Only used by interpreter `ops/arith.rs:46,49` and `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B11 | 4 occurrences of `#[allow(dead_code)]`                                           | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (conservative exemption for extern binding, **reasonable to keep**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction** (the only construction site is `parser/tests/ast.rs:828` where tests manually build nodes), but has **12 production consumers + 2 exhaustive arms** (complete list in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()`, `pratt/mod.rs:47` `expr_end_line`. Function definitions actually go through `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion belongs to P6 (the 16-item change list in 03 §5.5) |

## C. Phantom Design

| #   | Object                                                                          | Scale                                      | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                   | Describes `CheckSession` (with Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, `HotReloader` **all zero hits in the whole repo**. The "known limitations" at `:46-47` admits that `check_incremental` still calls `check_files_with_diagnostics` internally (full path); `:46` also references busy-wait debouncing from `command.rs`—**`command.rs` has neither `Instant` nor `recv_timeout`** (whole-repo `recv_timeout` only appears in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                   | `:37` claims placeholder implementations in `traits/` (coherence / impl_check / object_safety / resolution) are not done—**the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero hits in the whole repo. `:38` claims `check_single_module` creates a separate Compiler for each file—**this function does not exist**, only appearing in a comment in `util/diagnostic/tests/mod_tests.rs:248`                                                                                |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                   | `:14` claims "watch mode only rechecks affected files", `:27` claims "check only does the first two steps". Reality: `check_project` also does vendor consistency, project role determination, dead code analysis, W1006; the `Check` subcommand in `main.rs` **has no `--watch` flag** (every `#[arg]` declaration has been verified one by one)                                                                                                                                                                        |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines of design / **0 lines of code** | AOT hits in `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell. `:861-876` itself leaves a full `- [ ]` implementation checklist, but status is marked `accepted`                                                                                                                                                                                                                                                       |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**  | `028-jit-compiler.md:99-126` requires 6 files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **all do not exist**. Status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                                         |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                               | **Physical evidence level**: the file names precisely match the `ModuleCache` and incremental scheduler in the C1 design. **Someone built test skeletons according to the design, found no corresponding implementation in the pipeline, and stopped**                                                                                                                                                                                                                                                                   |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                          | The `//! §6.1: CheckSession incremental check` in `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references in `mod_tests.rs:4` refer to this **gitignored planning document** (`.gitignore` contains `superpowers/`). These § references are **archaeological-layer references to a deprecated API**                                                                                                                                                                                                          |

### Real Implementation Location of Cross-File Analysis

The cross-file analysis capabilities described in C1-C3 **really do exist**, just in a completely
different path:

| Component           | Location                              | Lines          |
| ------------------- | ------------------------------------- | -------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439            |
| Resolver            | `src/frontend/module/resolver.rs`     | 195            |
| Role classification | `src/frontend/module/roles.rs`        | 388            |
| Consistency check   | `src/frontend/module/consistency.rs`  | 206            |
| Orchestration       | `src/frontend/module/orchestrator.rs` | 1528           |
| **Total**           |                                       | **about 2670** |

So the disposition for C1-C3 is **not to delete the capability, but to delete the misleading
documents**, and rewrite the boundary table in `check/index.md` to point to the actual architecture
of `src/frontend/module/`.

### Disposition of RFC-019 Needs Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already in the `deprecated/` directory**,
and the residual check result is **clean** (`homoicon` zero hits in the whole repo, the document has
an honest conclusion at the end: "if restarting in the future, start from proof/prototype
experiments, not from the RFC"). **It does not need additional disposition**—this item is in the
authorization list, but verification confirmed it is already in a terminal state.

## D. Feature-Level Dead Code: REPL

`src/repl/` 4 files **1152 lines**, **zero `#[cfg(test)]`, zero `#[test]`**. And `src/main.rs:440`
is `args.command.unwrap_or(Commands::Repl)`—**executing `yaoxiang` without arguments directly enters
REPL**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed** | In the entire `src/repl/`, it only appears 2 times: definition at `eval.rs:23`, **match arm** at `mod.rs:180`. So `Repl::format_value` (`mod.rs:493-502`) is dead code, **REPL does not echo any evaluation results**                                                                                                                                                                          |
| D2  | **No cross-round state**                     | `extract_definitions` (`eval.rs:345-377`) only calls `define_variable` (stores **type string**), never calls `define_var` (stores `RuntimeValue`); `wrap_code` also doesn't inject historical definitions. The branch of `VariableInfo::Value` (`eval.rs:23`) is unreachable. `REPLContext.variables` purely serves completion, does not participate in evaluation                             |
| D3  | **Wrapping syntax possibly outdated**        | `eval.rs:314`/`316` produces `main() -> () = () => { code }` (`name() -> Ret` old-style function declaration header), while the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`, the canonical tables in RFC-007:94 are all `name: (a: Type) -> Ret`). **Compiler not actually run to test; if true, REPL's every evaluation would fail to compile** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` only prints two lines of hints and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries**                                                                                                                                                                                                                                                       |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                      |
| D6  | Duplicates with `lib.rs`                     | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent "auto-wrap main" logics**, with different wrapping forms                                                                                                                                                                                                                                                            |

## E. Local Cleanup Items

| #   | Location                                                                               | Disposition                                                                                                                                                                                                                                                      |
| --- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                             | Root-level empty directory tree. Almost certainly some place concatenated the string `"undefined"` as a path. **Delete directly**                                                                                                                                |
| E2  | `src/lib.rs:568`, `:582`                                                               | `dump_type_detail` / `dump_const_detail` each have one `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, panic path is predictable. Should be changed to meaningful degraded output                                               |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                              | `check_vendor_lock_consistency` and `check_workspace_consistency` are **about 60 lines of near-duplicate**, differing only in dependency source and path-dep skip. **Merge**                                                                                     |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167` | Old function syntax detection and rejection, about 120 lines. The `skip_old_function_syntax` (`:120-122`) function body has only one line of comment. **Delete along with 05-frontend-paradigm.md**                                                              |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                       | 96 lines of dead code (see B6). **Delete along with 05-frontend-paradigm.md**                                                                                                                                                                                    |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                | The "Tool Design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, `check/incremental-checking`. **If executing the C1-C3 document deletion, this must be cleaned up synchronously**, otherwise the doc site will have 404 links |

### Not Dead Code, but the Same Class of Problem: Three Hardcoded Discards

The `impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1400 lines) has 3
hardcoded discards:

| Line   | Discarded Content                | Consequence                                          |
| ------ | -------------------------------- | ---------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | upvalue metadata lost                                |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifacts lose exception table**             |
| `2341` | `globals: Vec::new()`            | **`.42` artifacts lose global variable information** |

**These three are not dead code, they are data loss defects**—`.42` artifacts are only complete
under the memory path. **The disposition is different from dead code**: ~~should be tracked as a
separate issue~~ **all incorporated into P7 batch e (decision D52, 2026-10-05), no separate
issue**—`2315` (exception table) and `2341` (global variables) belong to language core semantic
artifact completeness, not the "deferrable" category. Fix all three together, `.42`'s `VERSION`
upgrades from 4 to 5 in one go (the format header already has `MAGIC` + `VERSION` fields,
`codegen/bytecode.rs:14-16`). Listed here only to avoid being mistakenly cleaned up as "dead code".

## F. Not Deleted, but to be Flagged or Decided

| #   | Object                                                                                     | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (1547 lines)                      | **Not dead code**, it's spliced in via `include!` by `checker.rs:5618`—**the only `include!` in the entire repository**. This file **is not declared as any module** (`grep 'mod semantic_tokens'` zero hits in the whole repo), the first line is directly `impl TypeChecker {`, with no own `use` header, and it's the only file under the `checker/` directory. Consequence: no module identity, visibility isolation fails, rust-analyzer jumping and symbol search fail, the toolchain counts it as part of `checker.rs`. **The real size of the `checker` module is 5618+1547 ≈ 7165 lines**. **Fix: change to a real `mod`** (construction steps go to [09](09-execution-wbs.md) §P5 5.1), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| F2  | `#[cfg(target_arch = "wasm32")]` branches in 27 files                                      | **Not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, `:17` depends on the main crate; root `Cargo.toml:3` `exclude = ["wasm", ...]` precisely indicates it's an independent crate. Only looking at root `Cargo.toml:31` `crate-type = ["rlib"]` would misjudge these branches as defensive dead code—**that inference does not hold**. CI has 4 build locations (`_build-wasm.yml:75`, called by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`). **These 27 files' branches are load-bearing**, they determine which APIs in the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm target (`lib.rs:139/153/170` gates `run_file` / `run_project` / `build_bytecode` entirely). **The real cleanup target** is the unreachable part—`wasm/src/lib.rs` only calls `Compiler::new` / `compile_with_source` / `std::io::wasm_output` / `CodegenContext` / `Interpreter`, which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the playground scenario needs to be determined one by one (decision D35: delete unreachable, keep reachable, goes to P10)                                    |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                             | Only has the "status" column (document status), **no "implementation status" column**—it **cannot express** the error "RFC marked complete but code doesn't have it". Only 5/52 RFCs in the whole repo have the `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **26 accepted RFCs completely don't record implementation status**, including 009, 009a, 013, 018, 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| F4  | `src/frontend/core/typecheck/layers/README.md:1-16`                                        | Declares layer order equivalence → ownership → termination → predicate, **exactly opposite to the actual execution order** (actual: predicate → termination → ownership), and **has no short-circuit**. The README describes a set of intended architecture that was never built (or has been partially abandoned)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| F5  | `docs/src/rfc/index.md`                                                                    | `:79` puts RFC-026a in the "Accepted RFC" table but the status column says "RFCs Under Review"; the "RFCs Under Review" subsection at `:48-49` only lists 032 and 027a, missing 026a. Inconsistent with the 3 review items in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| F6  | `tools/cargo-dist/` (`dist.exe` 21,108,736 bytes) and `benches/shootout/out/` (8 binaries) | **Neither is tracked by git** (`git ls-files` returns empty), excluded by `.gitignore`. **Not a problem**, recorded only to prevent future audit from misjudging as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                       | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` takes until the next quote), depending on `("E1001", ...)` tuple syntax, **not a parser**. Its architecture (`parse` + `validate` + build.rs gate + `--fix` healing) is worth reusing for the opcode table, but **the extractor needs to be rewritten according to opcode form (about 30 lines)**. Belongs to follow-up work of 06-cleanup-inventory.md or an independent RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode fact is expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / decode match (`bytecode.rs:978-2302`) / 48 `translate_*` (`translator.rs:676-1577`). **The compiler only enforces 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + exhaustive match in each `ops/*.rs` family function), missing one of `opcode()` / `size()` / decode arm / encoder **only explodes when running `.42` artifacts**. The comment of the `size()` table (649-803) self-described is inconsistent with actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic differential test** (existing size tests only cover Nop/Mov/Borrow/Release 4). Operator semantics also go through a third parallel enum here ( `BinaryOp` / `UnaryOp` / `CompareOp`, `bytecode.rs:70/97/106`, another set of naming for `Rem`/`Xor`/`Sar`), resolved together with the opcode generation-phase gate. The vocabulary target location is `middle/bytecode/opcode.rs` (migrated along with bytecode domain merger, eliminating the L3→L4 reverse direction, see 01) |

## Key Decisions and Rationale

The authorization is "deletion is permitted", but this document provides **different
recommendations** for several items:

| Category                                                                | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions, no implementation) | **Delete**                                 | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty functions), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                   |
| **Phantom design documents**                                            | **Delete or rewrite**                      | C1, C2 delete then rewrite to point to actual implementation; C3 rewrite boundary table; **must synchronously clean up `config.js:259-268`** (E6)                                                                                                                                                               |
| **RFCs with status-implementation mismatch**                            | **Change status**                          | C4 (RFC-018) move back to `draft` or add `impl_status: 'not-started'`; C5 (RFC-028) add `impl_status: 'not-started'`. **Don't delete**—the design itself may have value, the problem is status marking distortion                                                                                               |
| **Orphan tests with real assertions**                                   | **Revive, not delete**                     | Total **1005 lines / 78 tests**: lexer 629 lines / 55 + 311 lines / 16 hidden in living directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7. **Revival cost is one `mod` declaration line, value is CI regains this coverage**. Deleting them is a net loss |
| **Zero-call but semantically important `pub` items**                    | **Reduce visibility rather than delete**   | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move to `pub(crate)` to let the compiler identify, or add implementation. **B4's `is_subtype` must be kept**                                                                                                                                                   |
| **Opcodes / instructions that produce no value**                        | **Add construction site or delete opcode** | B8 (`TailCall`), B9 (`Switch`, opcode roundtrip test has been explicitly registered with `WHITELIST=["SWITCH"]`), B10 (`UnaryOp::Not` silently degenerates). Three choices, cannot maintain status quo; D32/D33/D34 have been adjudicated as delete, handled together with `.42` version upgrade                |
| **Data loss defects**                                                   | ~~Separate issue~~ **P7 batch e (D52)**    | E section three hardcoded discards—not dead code                                                                                                                                                                                                                                                                |
| **No module identity but effective**                                    | **Refactor don't delete**                  | F1 (`semantic_tokens.rs`) change `include!` → real `mod` (see `02`)                                                                                                                                                                                                                                             |
| **Items requiring decision**                                            | **List as open issues**                    | Unreachable wasm branch cleanup scope in F2, layer order correction will expose new diagnostics                                                                                                                                                                                                                 |

### Directions Not Adopted

- **Process all as "deletion", orphan tests also deleted**—1005 lines of tests with real assertions
  are assets. Deletion cost is one `mod` declaration, benefit is CI regains coverage. Authorization
  is "may delete", not "should delete".
- **Only add gates without touching code**—gates prevent regression, but B1's 415 lines, dispatch
  chain, 96 lines of dead ladder **already exist**, gates will only freeze them at the baseline.
- **Keep `accepted` status of RFC-018 / RFC-028, only supplement documentation**—`rfc/index.md:131`
  defines "accepted = entering implementation phase", readers will reasonably assume implementation
  exists. `TRACKING.md` already has the `impl_status` field precedent (5 RFCs in use), the cost of
  adding the field is far less than the cost of status distortion.

## Implementation Points

**This document does not need an equivalence oracle** (belongs to
[C6 pure deletion category](07-equivalence-oracle.md)), but **still needs to confirm no references
item by item**. In RFC-039's global stage sequence, S1 corresponds to P1, S2-S6 correspond to P10.

| Stage | Content                                                                                                                              | Acceptance                                                                                                                                                                                                                                             |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| S1    | Revive orphan tests with assertions (**1005 lines / 78 tests**)                                                                      | Test count of lexer / pratt / passes / emitter / template five modules rises in `cargo test`; **expected to expose real defects** (overflow paths in `literals.rs`, `\x`/`\u` illegal escapes have never been tested before), need to reserve fix time |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 closed with 05/C6/E1)                                                                 | `cargo test` passes; `cargo clippy -D warnings` no new warnings                                                                                                                                                                                        |
| S3    | Phantom design document disposition (C1-C3 + E6 synchronous cleanup of `config.js`)                                                  | Doc site builds without 404; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                   |
| S4    | RFC status correction (C4/C5/F3—`TRACKING.md` adds "implementation status" column)                                                   | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` will auto-regenerate, **don't hand-edit**)                                                                                                                                                |
| S5    | `pub` item visibility reduction (B2/B4)                                                                                              | Compiler can point out real dead code                                                                                                                                                                                                                  |
| S6    | Opcode value-not-produced item decisions landed (B8/B9/B10/F8, executed per D32/D33/D34)                                             | Delete `Switch` / `TailCall` dead paths; B10's `opcode()` distinguishes `op` field; F8 gate landed                                                                                                                                                     |
| S7    | `include!` refactor (F1, construction steps go to [09](09-execution-wbs.md) §P5 5.1) + layer order correction (F4, goes to P4 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                             |

**S1 must be done first, and defect fix time must be reserved.** Its value is not "cleanup", but
**letting CI regain 1005 lines / 78 tests that never ran**—before that, any judgment about test
coverage is built on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners in `literals.rs` have 4 isomorphic
  "continue consuming after overflow but not report error" branches, the entire `scan_leading_dot`
  function, `\x`/`\u` illegal escape paths, all have never been tested. After reviving tests, code
  fixes are very likely needed, which will delay subsequent refactoring.
- **B9/B10 involve language behavior decisions**. Whether `Switch` and `UnaryOp::Not` should "add
  construction sites" or "delete opcode" needs to consider whether they belong to the surface that
  released corpus depends on.
- **After C1-C3 are deleted, the capability documentation for cross-file analysis will be briefly
  absent**, until `check/cross-file-analysis.md` is rewritten to point to the actual architecture of
  `src/frontend/module/`.
- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Judgments
  beyond Form 2 must be done manually item by item, and **this list does not constitute exhaustive
  proof**.
- **`pub` items in Rust do not trigger `dead_code` warnings**, so zero-call `pub` types (B2, B4)
  will not be automatically discovered by the compiler, only by search.
- **Deleting RFC-023's half-done residue** (B1) can deliver on the decision "closures have been
  removed from the language"; **correcting status distortion** (C4/C5/F3) can restore
  `TRACKING.md`'s reference value.

> **The open questions originally listed in this section have all been adjudicated.** For each
> decision, see [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  Source decision of B1 residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Unwired runtime check insertion from B3, §4.1
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — Formal carrier of C1/C6
- [RFC-039 Compiler Function Routing Directory Design](../../rfc/accepted/039-compiler-architecture.md)
  — Upper-level master
- [Refactoring Equivalence Oracle](07-equivalence-oracle.md) — Oracle requirements for the C6
  category
- `src/frontend/core/typecheck/layers/README.md` — F4 layer order declaration distortion
- `build.rs:19-55` — Example of this repository's generation-phase gate
