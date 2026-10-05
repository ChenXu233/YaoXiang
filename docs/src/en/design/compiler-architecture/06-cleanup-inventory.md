# Dead Code and Empty Design Cleanup

> **Ancillary design document.** This document is ancillary to
> [RFC-039 Compiler Architecture Refactoring](../rfc/draft/039-compiler-architecture.md). The
> four-layer model, the tiered acceptance criteria, and the execution phase sequence are in the body
> of RFC-039; the positioning of each ancillary document is in the [directory index](index.md).

## Positioning and Scope

This document provides the YaoXiang repository's inventory of dead code and empty designs, the
determination methods, item-by-item disposition decisions, and execution order. The cleanup targets
fall into four categories:

| Category                                                       | Scale                                                                   |
| -------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Files that never participate in compilation                    | 24 files / 2628 lines (of which 23 files / 1081 lines are real orphans) |
| Compiled but zero-call dead code                               | 1500+ lines                                                             |
| Design documents describing an architecture that never existed | three files under `docs/src/design/check/`                              |
| Features hanging on the default entry, zero tests, zero output | `src/repl/` 1152 lines                                                  |

**The authorization scope is "deletion is permitted"** (decision of 2026-10-03). However, for
several items this document gives **disposition recommendations opposite to "delete directly"**—see
rationale in [Key Decisions and Rationale](#key-decisions-and-rationale).

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), and criterion design (see `07-equivalence-oracle.md`).
The unwired chain in `layers/dispatch.rs` recorded under B3 has its deletion grouped under S2.

## Current State

### Three Liability Forms of Dead Code

**Form 1: Placeholder.** Code exists, looks like it's doing something, but doesn't.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the "runtime check insertion" mechanism designed by `RFC-027 §4.1`, **entirely
  unwired**.
- `check_type_equivalence` (`ProofContext` entry) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is called only by tests. **Note** that
  `is_subtype` in the same file is used in production at `inference/assignment.rs:17`, so **the
  entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler; its `pub` surface only
  exposes `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It **is not a second execution backend**—the
  `Executor` trait at `backends/mod.rs:356` currently has only one implementor (the interpreter).
  This point needs to be clarified because the directory name is misleading.

**Form 2: Remnant.** An upstream decision was made, but the deletion was only half-done.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) is remnant: 6 types remain:
  `FunctionInstance` (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), `CaptureVariable`. **Source is clear**: when `RFC-023`
  deprecated the "closure capture model", it stated "**approximately 850 lines of code to be deleted
  together with the closure model**". These 415 lines are the remnants that survived from those 850
  lines—**deletion was only half-done**. **Verification**: performing whole-repo word-boundary
  search for these 6 types, hit count outside `instance.rs` is **all 0** (including tests).
  `pub use passes::mono::instance::*;` at `middle/mod.rs:40` is just a glob re-export, not
  consumption.

**Form 3: False Coverage.** This is the most dangerous kind—**it makes the repository look more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only 5 `pub mod`s (`literals` / `state` / `symbols` /
`tokenizer` / `tokens`) + one `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`
(`:104-106`). **No place declares `mod tests;`**, therefore the entire subtree `lexer/tests/`—13
files, 686 lines—**never participates in compilation**.

Of these, 7 are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 / `delimiters.rs`
3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totaling 19 lines, 0 assertions). **629
lines / 55 real tests never run**:

| File | Lines | `#[test]` | | --- | --- | | `literals.rs` | 222 | 30 | | `rfc010_lexer.rs` | 167 |
12 | | `lexer_mod.rs` | 89 | 5 | | `rfc004_lexer.rs` | 81 | 5 | | `symbols.rs` | 70 | 3 | |
**Total** | **629** | **55** |

(`fstring.rs` 90 lines / 11 tests does indeed compile, but via the `#[path]` bypass rather than
`mod tests;`; `mod.rs` 38 lines is the declaration file, no assertions.)

**The lexical layer actually running is only one file with 12 tests, all of which only test
f-string's curly-brace raw protocol.**

The hidden danger of this kind of problem is: `cargo test` does not fail (tests that haven't been
run cannot fail), and CI has no test-count baseline or wiring check either.

### Empty Designs

The three documents under `docs/src/design/check/` describe an architecture that **never existed**.
This is not "designed but not built", but rather **the documents describe another system that never
existed, and write "designs that might happen in the future" as "defects of the implemented
system"**.

## Methodology and Credibility Statement

### Reachability Analysis Method

Starting from the two crate roots `src/lib.rs` / `src/main.rs`, parse all 509 `.rs` files' `mod` /
`pub mod` / `#[path]` declarations, build the module graph and perform BFS reachability analysis.
`include!` targets are adjudicated separately (not counted in reachability).

**This method has known misjudgment patterns, two of which were actually triggered during writing:**

1. `Split-Path` on Windows returns `\`, which doesn't match forward-slash keys → once misreported
   499 orphans
2. **The submodule directory of a non-`mod.rs` file is `self_name/` rather than `parent_dir/`** →
   `orchestrator.rs`'s `mod tests` was parsed as `src/frontend/module/tests/`, almost classifying
   753 lines of living tests as dead code

Therefore **every conclusion is double-confirmed through "actually reading the file + actually
searching for references"**, and items that did not pass secondary confirmation are not included in
this document.

### Line Count Calibration

All line counts in this document are computed using `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines and systematically undercounts by about 5%** (e.g.,
`manifesto.md` reports 658 with `-Line`, actually 893). An earlier analysis in this repository gave
an undercounted total due to this calibration; it has been corrected here.

### Known Limitations

- **Reachability analysis can only discover "unreachable", not "reachable but meaningless"**. Forms
  other than Form 2 require per-item human judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types need to be
  discovered by search, and this method does not cover them.
- This document does not involve running the compiler, so all "never run" conclusions are static
  judgments.

## A. Files That Never Participate in Compilation

**Totaling 24 files / 2628 lines.** Of these, `semantic_tokens.rs` belongs to Form 4 (see Section
F), the remaining **23 files / 1081 lines are real orphans**.

| Path                                                            | Scale                   | Content                                                                       | Basis for Determination                                                                                                                                                                     |
| --------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/tests/` **entire subtree of 13 files** | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only 5 `pub mod`s; the only test declaration at `:104-106` is `#[path]` to `tests/fstring.rs`. `grep 'mod tests'` in the whole directory hits only `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 files                          | 14 lines                | All placeholder doc comments, 0 assertions                                    | `src/frontend/pipeline.rs` (740 lines) has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration anywhere (`Select-String` zero hits)                                               |
| `src/package/template/tests/` 3 files                           | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`s**                    | `src/package/template/mod.rs` (7 lines) declares only `mod gitignore; mod main_yx;`, no `mod tests;`                                                                                        |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`     | 95 lines / **6 tests**  | Hidden in a **living** directory                                              | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                             |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs`   | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                |
| `src/util/diagnostic/emitter/tests/json.rs`                     | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                        |

**The last 3 items total 16 tests / 311 lines of code that never ran.** They are hidden in active
directories, where the `mod.rs` looks complete, making them more deceptive than entire orphan
subtrees.

Of these, two other files inside `lexer/tests/` are also in the "declared but missing declaration"
form, but they are already counted in the 629 lines in the first row of the table above, **not
double-counted**: `lexer_mod.rs` (89 lines / 5 tests, `lexer/tests/mod.rs` declares 11 `mod`s but no
`mod lexer_mod;`), `symbols.rs` (70 lines / 3 tests, same situation).

**Total real tests that never ran: 1005 lines / 78**: lexer 629/55 + pratt / passes / emitter
311/16 + package/template 65/7.

**Special attention to `precedence_inline.rs`**: it tests the `Precedence` enum +
`PrecedenceContext` (96 lines of dead code) in `precedence.rs` that has **zero production uses**.
Even if wired up, it would test a dead path.

Another one hidden in the 23rd orphan: `src/frontend/core/typecheck/tests/semantic_db.rs` (5 lines,
0 tests): `typecheck/tests/mod.rs` declares 23 test modules but misses this one—the file has only
one line of `use`, pure wreckage. When S1 revives, delete along with wiring or add assertions.

## B. Compiled but Zero-Call Dead Code

| #   | Location                                                                         | Scale               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | -------------------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | Whole-repo word-boundary search, hits outside `instance.rs` are all 0 (including tests). Source: RFC-023 declared deletion of approximately 850 lines, only half was deleted                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` + 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both re-exports). Whole-repo `TypeSystem` hits only 5 places: 2 definitions + 1 re-export + **unrelated** `lexer/symbols.rs:503 TypeSystemValidator` × 2. The 4 methods have zero call sites. **Because it is `pub`, the compiler will never warn**                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| B3  | Entire chain in `src/frontend/core/typecheck/layers/dispatch.rs`                 | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` zero calls. RFC-027 §4.1's runtime check insertion is unwired. Note it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` is called only by tests. **`is_subtype` in the same file must be kept** (used in production at `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-described as "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests)—and that test file itself is never compiled** (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore the two **must be deleted together**: deleting the test alone leaves 96 lines of dead code, deleting the enum alone causes test compilation failure                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, `skip_old_function_syntax` is called at `:725`—**they are running old-syntax rejection gates, not unreferenced dead code**. The `skip_old_function_syntax` function body (`:120-122`) has only a one-line comment `// 旧语法已移除`, but **after this function is called it does not consume any token**—this is an independent behavior issue (whether the rejection path correctly advances parse position), not dead code. **Disposition is to review behavior along with 05-frontend-paradigm.md (resolution D27: naturally deleted with the grammar migration), not to delete it as dead code**                                                                                                                                                                                                          |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | Only matches in the whole repo, no construction site. The "no generation site (dead path)" comment at `bytecode.rs:2156` is **accurate** (encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| B9  | `Switch` opcode                                                                  | —                   | Defined in `backends/common/opcode.rs`, has variant, has `opcode()`/`size()` arms, has decode arm, **but zero construction sites in the whole repo**. The interpreter has a live implementation at `ops/control.rs:90`. **Explicitly registered as a known dead path by tests**: `WHITELIST: &[&str] = &["SWITCH"]` at `tests/bytecode.rs:1035-1040` (non-empty difference means failure)                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` to **ignore the `op` field**, and the decoder at `1236-1248` also only constructs `Neg`. **The `opcode()` mapping path silently degrades to I64_NEG** (when running `.42` directly, `!x` becomes `-x`; the interpreter's memory path `ops/arith.rs:46-49` is actually correct). Used only by interpreter `ops/arith.rs:46,49` and `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B11 | 4 `#[allow(dead_code)]`                                                          | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (conservative exemption for extern binding, **reasonable to keep**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| B12 | `Expr::FnDef` in `src/frontend/core/parser/ast.rs:37-43`                         | —                   | **Zero production construction** (the only construction site is the test at `parser/tests/ast.rs:828` that manually crafts a node), but has **12 production consumers + 2 exhaustive arms** (full list in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `Expr::span()` at `ast.rs:1026`, `expr_end_line` at `pratt/mod.rs:47`. Function definition actually goes through `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion goes to P6 (16-item change list in 03 §5.5) |

## C. Empty Designs

| #   | Object                                                                          | Scale                                      | Fact                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/design/check/incremental-checking.md`                                 | 53 lines                                   | The `CheckSession` it describes (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, `HotReloader` all have **zero hits in the whole repo**. The "known limitations" at `:46-47` self-admits that `check_incremental` internally still calls `check_files_with_diagnostics` (full path); `:46` also references the busy-wait debounce in `command.rs`—**`command.rs` has neither `Instant` nor `recv_timeout`** (whole-repo `recv_timeout` appears only in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/design/check/cross-file-analysis.md`                                  | 43 lines                                   | `:37` claims that the `traits/` placeholder implementations (coherence / impl_check / object_safety / resolution) are incomplete—**the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero hits in the whole repo. `:38` claims that `check_single_module` creates a separate Compiler for each file—**this function does not exist**, it only appears in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                            |
| C3  | `docs/src/design/check/index.md`                                                | 29 lines                                   | `:14` claims "watch mode only re-checks affected files", `:27` claims "check only does the first two steps". Actually: `check_project` also does vendor consistency, project role determination, dead code analysis, W1006; the `Check` subcommand in `main.rs` has **no `--watch` flag** (every `#[arg]` declaration has been checked one by one)                                                                                                                                                                                                 |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines of design / **0 lines of code** | AOT hits at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell. `:861-876` itself leaves an all `- [ ]` implementation checklist, but the status is marked `accepted`                                                                                                                                                                                                                                                                             |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**  | `028-jit-compiler.md:99-126` requires 6 files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **all do not exist**. Status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                                                                   |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                               | **Physical-evidence level**: the filenames correspond exactly to the `ModuleCache` and incremental scheduler in the C1 design. **Someone built the test skeleton according to the design, found no corresponding implementation in the pipeline, and shelved it**                                                                                                                                                                                                                                                                                  |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                          | The `//! §6.1: CheckSession incremental check` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` point to this **gitignored planning document** (`.gitignore` includes `superpowers/`). These § references are **archaeological references to deprecated APIs**                                                                                                                                                                                                                                           |

### The Real Implementation Location of Cross-File Analysis

The cross-file analysis capability described by C1-C3 **does exist**, just on an entirely different
path:

| Component           | Location                              | Lines          |
| ------------------- | ------------------------------------- | -------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439            |
| Resolver            | `src/frontend/module/resolver.rs`     | 195            |
| Role classification | `src/frontend/module/roles.rs`        | 388            |
| Consistency check   | `src/frontend/module/consistency.rs`  | 206            |
| Orchestrator        | `src/frontend/module/orchestrator.rs` | 1528           |
| **Total**           |                                       | **about 2670** |

Therefore the disposition of C1-C3 is **not to delete the capability, but to delete the misleading
documents**, and rewrite the boundary table in `check/index.md` to point to the actual architecture
in `src/frontend/module/`.

### Disposition of RFC-019 Needs Clarification

`docs/src/design/rfc/deprecated/019-typed-homoiconicity.md` is **already in the `deprecated/`
directory**, and the residual check result is **clean** (`homoicon` has zero hits in the whole repo,
and the document ends with an honest "if restarting in the future, start from proof/prototype
experiments, not from the RFC" conclusion). **It needs no additional disposition**—it is in the
authorization list, but after verification it is confirmed to be in its terminal state.

## D. Feature-Level Dead Code: REPL

`src/repl/` 4 files **1152 lines**, **zero `#[cfg(test)]`, zero `#[test]`**. And `src/main.rs:440`
is `args.command.unwrap_or(Commands::Repl)`—**running `yaoxiang` directly without arguments enters
REPL**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                            |
| --- | -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed** | In the whole `src/repl/` it appears only twice: the definition at `eval.rs:23`, the **match arm** at `mod.rs:180`. Therefore `Repl::format_value` (`mod.rs:493-502`) is dead code—**REPL does not echo any evaluation result**                                                                                                                                                      |
| D2  | **No cross-round state**                     | `extract_definitions` (`eval.rs:345-377`) only calls `define_variable` (stores the **type string**), never calls `define_var` (stores `RuntimeValue`); `wrap_code` also does not inject historical definitions. The `VariableInfo::Value` branch at `eval.rs:23` is unreachable. `REPLContext.variables` purely serves completion, not evaluation                                   |
| D3  | **Wrapper syntax appears outdated**          | `eval.rs:314`/`316` produces `main() -> () = () => { code }` (`name() -> Ret` old-style function declaration header), while the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`, RFC-007:94's canonical table is all `name: (a: Type) -> Ret`). **Not actually run in compiler; if it holds, every REPL evaluation will fail to compile** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` only prints two lines of hint and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                        |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                           |
| D6  | Duplicated with `lib.rs`                     | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent "auto-wrap main" logics**, with different wrapper forms                                                                                                                                                                                                                                                  |

## E. Local Cleanup Items

| #   | Location                                                                               | Disposition                                                                                                                                                                                                                                               |
| --- | -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                             | Empty directory tree at root level. Almost certainly some place concatenated the string `"undefined"` as a path. **Delete directly**                                                                                                                      |
| E2  | `src/lib.rs:568`, `:582`                                                               | `dump_type_detail` / `dump_const_detail` each have a `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, the panic path is predictable. Should be changed to a meaningful degraded output                                    |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                              | `check_vendor_lock_consistency` and `check_workspace_consistency` are **about 60 lines of near-duplication**, differing only in the source of dependencies and the path-dep skip. **Merge**                                                               |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167` | Old function syntax detection and rejection, about 120 lines. Among these, `skip_old_function_syntax` (`:120-122`) function body has only one line of comment. **Delete together with 05-frontend-paradigm.md**                                           |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                       | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                          |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                | The "Tool Design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, `check/incremental-checking`. **If the document deletion of C1-C3 is executed, this must be cleaned up in sync**, otherwise the doc site has 404 links |

### Not Dead Code, but the Same Kind of Problem: Three Hardcoded Discards

`impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1400 lines) has 3 hardcoded
discards:

| Line   | Discarded Content                | Consequence                                          |
| ------ | -------------------------------- | ---------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | upvalue meta-information lost                        |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifact loses exception table**             |
| `2341` | `globals: Vec::new()`            | **`.42` artifact loses global variable information** |

**These three are not dead code, they are data loss defects**—the `.42` artifact is complete only in
the memory path. **The handling is different from dead code**: ~~should be tracked as a separate
issue~~ **all absorbed into P7 batch e (resolution D52, 2026-10-05), no independent issue**—`2315`
(exception table) and `2341` (global variables) belong to the artifact integrity of language core
semantics, not "deferrable" items. Fixing all three at once, `.42`'s `VERSION` goes from 4 to 5 in
one go (the format header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`). This
is listed here only to avoid being mistakenly treated as "dead code" to clean up.

## F. Do Not Delete, but Mark or Decide

| #   | Object                                                                                     | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (1547 lines)                      | **Not dead code**, it is spliced in by `checker.rs:5618` via `include!`—**the only `include!` in the entire repo**. This file **is not declared as any module** (`grep 'mod semantic_tokens'` has zero hits in the whole repo), the first line is directly `impl TypeChecker {`, without its own `use` header, and it is the only file in the `checker/` directory. Consequences: no module identity, broken visibility isolation, broken rust-analyzer jump and symbol search, toolchain counts it as part of `checker.rs`. **The real size of the `checker` module is 5618+1547 ≈ 7165 lines**. **Fix: change to a real `mod`** (construction steps go to [09](09-execution-wbs.md) §P5 5.1), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F2  | `#[cfg(target_arch = "wasm32")]` branches in 27 files                                      | **Not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, `:17` depends on the main crate; the root `Cargo.toml:3` `exclude = ["wasm", ...]` precisely says it is an independent crate. Looking only at the root `Cargo.toml:31` `crate-type = ["rlib"]` would misjudge these branches as defensive dead code—**this inference does not hold**. CI has 4 builds (`_build-wasm.yml:75`, called by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`). **The branches in these 27 files are load-bearing**, they determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call in the wasm target (`lib.rs:139/153/170` gates `run_file` / `run_project` / `build_bytecode` as a whole). **The real cleanup target** is the unreachable part—`wasm/src/lib.rs` only calls `Compiler::new` / `compile_with_source` / `std::io::wasm_output` / `CodegenContext` / `Interpreter`, of orchestrator.rs's 12 wasm attributes, which are unreachable in the playground scenario, needs to be judged one by one (resolution D35: delete the unreachable, keep the reachable, go to P10)                    |
| F3  | Column structure of `docs/src/design/rfc/TRACKING.md`                                      | Only the "Status" column (document status), **no "Implementation Status" column**—it **cannot express** the error "RFC marked as complete but the code does not exist". Only 5/52 RFCs in the whole repo have an `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **26 accepted RFCs do not record implementation status at all**, including 009, 009a, 013, 018, 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| F4  | `src/frontend/core/typecheck/layers/README.md:1-16`                                        | Declares layer order equivalence → ownership → termination → predicate, **exactly the opposite of the actual execution order** (actual: predicate → termination → ownership), and **there is no short-circuit at all**. The README describes an unbuilt (or partially abandoned) intended architecture                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| F5  | `docs/src/design/rfc/index.md`                                                             | `:79` puts RFC-026a in the "Accepted RFC" table but the status column says "Reviewing RFC"; the "Reviewing RFC" subsection at `:48-49` only lists 032 and 027a, missing 026a. Inconsistent with TRACKING.md's 3 reviews                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| F6  | `tools/cargo-dist/` (`dist.exe` 21,108,736 bytes) and `benches/shootout/out/` (8 binaries) | **None tracked by git** (`git ls-files` returns empty), excluded by `.gitignore`. **Not a problem**, just recorded to prevent subsequent audit from misjudging as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                       | `extract_code_from_entry_line` is a **line-start prefix matcher** (`trim_start().strip_prefix("(\"")` to the next quote), depends on the `("E1001", ...)` tuple syntax, **not a parser**. Its architecture (`parse` + `validate` + build.rs gate + `--fix` remediation) is worth replicating to the opcode table, but **the extractor needs to be rewritten for the opcode form (about 30 lines)**. Goes to follow-up of 06-cleanup-inventory.md or a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode fact is expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / decode match (`bytecode.rs:978-2302`) / 48 `translate_*` (`translator.rs:676-1577`). **The compiler only enforces 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + each `ops/*.rs` family function's exhaustive match); `opcode()` / `size()` / decode arms / encoder missing one place **only blows up when running `.42` artifact**. The `size()` table's (649-803) comment self-description is inconsistent with actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic cross-check test** (existing size tests cover only Nop/Mov/Borrow/Release 4). Operator semantics here also go through a third parallel enum (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`, `Rem`/`Xor`/`Sar` yet another set of names), converged together with the opcode generation-phase gate. The target location of the word table is `middle/bytecode/opcode.rs` (migrated with the bytecode domain merge, eliminating L3→L4 reverse, see 01) |

## Key Decisions and Rationale

The authorization is "deletion is permitted", but for several items this document gives **different
recommendations**:

| Category                                                                    | Disposition                                  | Rationale                                                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholder** (skeletons without assertions, without implementation) | **Delete**                                   | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty function), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                                |
| **Empty design documents**                                                  | **Delete or rewrite**                        | C1, C2 delete and rewrite to point to actual implementation; C3 rewrite the boundary table; **`config.js:259-268` must be cleaned up in sync** (E6)                                                                                                                                                                         |
| **RFCs with status inconsistent with implementation**                       | **Change status**                            | C4 (RFC-018) move back to `draft` or add `impl_status: 'not-started'`; C5 (RFC-028) add `impl_status: 'not-started'`. **Do not delete**—the design itself may be valuable, the problem is the status marker is distorted                                                                                                    |
| **Orphan tests with real assertions**                                       | **Revive, not delete**                       | Total **1005 lines / 78 tests**: lexer 629 lines / 55 + 311 lines / 16 hidden in living directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7. **The cost of revival is one `mod` declaration, the value is that CI regains these coverage**. Deleting them is a net loss |
| **Zero-call but semantically important `pub` items**                        | **Reduce visibility rather than delete**     | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move to `pub(crate)` to let the compiler identify, or supplement the implementation. **`is_subtype` in B4 must be kept**                                                                                                                                                   |
| **Opcodes / instructions that cannot produce values**                       | **Add construction sites or delete opcodes** | B8 (`TailCall`), B9 (`Switch`, the opcode roundtrip test has been explicitly registered with `WHITELIST=["SWITCH"]`), B10 (`UnaryOp::Not` silently degrades). Choose one of three, cannot maintain the status quo; D32/D33/D34 have already been adjudicated to delete, handled together with the `.42` version upgrade     |
| **Data loss defects**                                                       | ~~Independent issue~~ **P7 batch e (D52)**   | The three hardcoded discards in Section E—not dead code                                                                                                                                                                                                                                                                     |
| **Valid but lacking module identity**                                       | **Refactor, not delete**                     | F1 (`semantic_tokens.rs`) change `include!` → real `mod` (see `02`)                                                                                                                                                                                                                                                         |
| **Needing decision**                                                        | **Listed as open questions**                 | Cleanup scope of **unreachable wasm branches** in F2, layer order correction will expose new diagnostics                                                                                                                                                                                                                    |

### Directions Not Adopted

- **Handle all according to "deletion", orphan tests also deleted**—1005 lines of tests with real
  assertions are assets. The cost of deletion is one `mod` declaration, the benefit is that CI
  regains coverage. The authorization is "may delete", not "should delete".
- **Only add gates, don't move code**—gates can prevent regression, but B1's 415 lines, the dispatch
  chain, 96 lines of dead ladder **already exist**, gates will only freeze them at the baseline.
- **Keep `accepted` status of RFC-018 / RFC-028, only supplement documentation**—`rfc/index.md:131`
  defines "accepted = enters implementation phase", readers will reasonably assume implementation
  exists. `TRACKING.md` already has a precedent for the `impl_status` field (5 RFCs are using it),
  the cost of supplementing the field is far less than the cost of status distortion.

## Implementation Points

**This document does not need an equivalence oracle** (belongs to the
[C6 pure deletion category](07-equivalence-oracle.md)), but **each item still needs confirmation of
no references**. In RFC-039's global phase sequence, S1 corresponds to P1, S2-S6 correspond to P10.

| Phase | Content                                                                                                                              | Acceptance                                                                                                                                                                                                                                                             |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphan tests with assertions (**1005 lines / 78 tests**)                                                                      | The test count of five modules lexer / pratt / passes / emitter / template rises in `cargo test`; **expected to expose real defects** (overflow path in `literals.rs`, `\x`/`\u` illegal escape paths have never been tested before), time for repair must be reserved |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 closed with 05/C6/E1)                                                                 | `cargo test` passes; `cargo clippy -D warnings` no new warnings                                                                                                                                                                                                        |
| S3    | Empty design document disposition (C1-C3 + E6 synchronously clean up `config.js`)                                                    | Doc site build has no 404; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                                     |
| S4    | RFC status correction (C4/C5/F3—`TRACKING.md` add "Implementation Status" column)                                                    | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` will be regenerated automatically, **do not edit by hand**)                                                                                                                                               |
| S5    | Reduce visibility of `pub` items (B2/B4)                                                                                             | Compiler can point out real dead code                                                                                                                                                                                                                                  |
| S6    | Opcode no-value-producing item decisions land (B8/B9/B10/F8, execute according to D32/D33/D34)                                       | Delete `Switch` / `TailCall` dead paths; B10's `opcode()` distinguishes `op` field; F8 gate lands                                                                                                                                                                      |
| S7    | `include!` refactor (F1, construction steps go to [09](09-execution-wbs.md) §P5 5.1) + layer order correction (F4, go to P4's 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                                             |

**S1 must be done first, and defect repair time must be reserved.** Its value is not "cleanup", but
**letting CI regain 1005 lines / 78 tests that have never run**—before this, any judgment about test
coverage is built on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects.** The four radix scanners in `literals.rs` have 4 isomorphic
  "continue consuming without erroring after overflow" branches, the entire `scan_leading_dot`
  function, and the `\x`/`\u` illegal escape paths, none of which have ever been tested. After
  reviving the tests, it is very likely that the code needs to be fixed, which will delay subsequent
  refactoring.
- **B9/B10 involve language behavior decisions.** Whether `Switch` and `UnaryOp::Not` are "add
  construction sites" or "delete opcodes" needs to consider whether they belong to the surface that
  published corpus depends on.
- **After C1-C3 are deleted, the capability documentation of cross-file analysis will be briefly
  absent**, until `check/cross-file-analysis.md` is rewritten to point to the actual architecture in
  `src/frontend/module/`.
- **Reachability analysis can only discover "unreachable", not "reachable but meaningless"**.
  Judgments beyond Form 2 must be done item by item manually, and **this inventory does not
  constitute exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  will not be automatically discovered by the compiler, only by search.
- **Deleting RFC-023's half-done remnant** (B1) fulfills the decision "closure has been removed from
  the language"; **correcting status distortion** (C4/C5/F3) can restore the reference value of
  `TRACKING.md`.

> **The open questions originally listed in this section have all been adjudicated.** See
> item-by-item decisions in [RFC-039 Decision Registry](../rfc/draft/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../rfc/deprecated/023-closure-capture-model.md) — the
  source decision of B1's remnant
- [RFC-027 Compile-Time Evaluation and Types](../rfc/accepted/027-compile-time-evaluation-types.md)
  — B3's unwired `§4.1` runtime check insertion
- [RFC-018 LLVM AOT](../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../rfc/draft/029a-module-cache-incremental.md)
  — the formal carrier of C1/C6
- [RFC-039 Compiler Function Routing Directory Design](../rfc/draft/039-compiler-architecture.md) —
  top-level master plan
- [Refactoring Equivalence Oracle](07-equivalence-oracle.md) — oracle requirements for the C6
  category
- `src/frontend/core/typecheck/layers/README.md` — F4 layer order declaration distortion
- `build.rs:19-55` — example of generation-phase gate in this repository
