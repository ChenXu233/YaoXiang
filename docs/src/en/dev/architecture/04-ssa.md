# SSA-izing the Intermediate Representation

> **Companion design document**. This article accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution-phase order are defined in the main
> RFC-039 text; the positioning of each companion document is described in the
> [index for this directory](index.md).

## Positioning and Scope

This article addresses L3 IR construction discipline and proposes a “one definition per value”
construction discipline for `src/middle/core/ir_gen.rs` (8,448 lines). **The goal is not to
introduce a new instruction set, but to eliminate three classes of discipline defects that fail
without producing any errors**:

1. Six sets of manually managed register save/restore operations scattered dozens of lines apart;
2. Semantic reordering of `arg_regs` shared by six branches inside `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This article is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, explicit cross-layer contracts, decomposition of
call-expression dispatch, and the integration of the IR static verifier.

This article is not responsible for: convergence of type representations
([03](03-type-unification.md)), stage contracts and the obligations ledger
([02](02-stage-contract.md)), definition of equivalence criteria and gating
([07](07-equivalence-oracle.md)), frontend paradigms ([05](05-frontend-paradigm.md)), or cleanup of
dead code and straw-man designs ([06](06-cleanup-inventory.md)). This article repeatedly cites
conclusions from those documents and does not repeat their arguments.

**The criterion category** follows the **C4 (IR shape changes)** category in
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, rather than exact equality of IR snapshots. SSA transformation necessarily changes the
IR shape; forcing snapshot equality would encourage the team to weaken the criteria—the exact trap
recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, with acceptance requiring behavioral equivalence and a green `verify_ssa`). The
implementation batch numbers within this article (batches a–d) are distinct from the internal
ordering of P7: the former are this article’s incremental delivery strategy, while the latter are
RFC-039’s global phases.

## Feasibility Preconditions

**Verified facts.** The IR already possesses **all structural prerequisites** required for SSA
construction:

| SSA prerequisite             | Current state in this repository                                                                                 | Location                                                               |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                                 | `BasicBlock` in `src/middle/ir/ir.rs`                                  |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                   | `FunctionBody` in `src/middle/ir/ir.rs`                                |
| Scope-nesting information    | `LocalSlot::scope_depth` (“0 = function-parameter layer, used to disambiguate duplicate names in nested scopes”) | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Three-address form           | 76 variants, 54 with `dst: Operand`, and 3 with `dst: Option<Operand>`                                           | `Instruction` in `src/middle/ir/ir.rs`                                 |
| Span on every instruction    | Every variant has a `span: Span` field                                                                           | Same as above                                                          |
| Value types can be annotated | `LocalSlot::ty: MonoType`                                                                                        | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Slot-count upper-bound check | Single table-extraction and E3014 budget gate (`MAX_REGISTERS = 255`)                                            | `finalize_code_body` in `src/middle/lower/scope.rs` (from 7b.4 onward) |

**If these prerequisites were absent, the cost estimates in this article would be completely
different.**

**The two decisive points are as follows:**

**First, SSA transformation does not add a capability; it removes a dependency on one.** In the
current implementation, temporary slots are **deliberately designed to have multiple
definitions**—`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`, explicitly roll `next_temp` back in
three places. The first step of SSA transformation is to **delete these three rollbacks**, making
`next_temp` increase monotonically. Once they are removed, **every temporary slot naturally has
exactly one definition point**—unique definition changes from “a rule that must be checked” into “a
mathematical consequence of the construction method.” Batch a only consolidated the save/restore
surface (see “Defect Class 1”); the three rollbacks were removed with 7b.4 in batch b, when the
allocator took over slot reuse (2026-11-22).

**Second, the slot-count upper bound is a hard check, and deleting the rollbacks briefly caused the
slot count to become uncontrolled (resolved in 7b.4).** The budget gate in
`src/middle/lower/scope.rs` reports E3014 when the count exceeds 255. After removing the three
reclamation points, running the entire corpus produced two corpus files that exceeded the limit and
triggered E3014: `src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; peak slot counts in the
remaining corpus files rose from 51 to 211. **After 7b.4 landed**: the linear-scan allocator reuses
slots according to live intervals, and the two corpus files now use **15 / 24 frame slots** in
practice (344 / 304 values); the remaining corpus files are behaviorally equivalent in all 403/403
cases. E3014 now means only “frame slots after allocation exceed 255.”

**The combined implication of these two points is**: lowering already maintains all the bookkeeping
information required for SSA, but merely **reclaims** that information; 7b.4 makes the allocator
responsible for slot reuse and removes the reclamation. The main SSA workload lies in decomposing
`generate_call_expr_ir` and making cross-layer contracts explicit, not in building IR expressive
capability.

### Prerequisite: Unification of Type Representations

**Verified facts.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The third representation is `ir::Type` on the serialization side,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR currently contains two type representations, and one of them directly
`pub use`s the AST’s type.**

SSA transformation must add a `Phi` variant, record a type for every SSA value (for the “type
consistency” invariant in the 07 verifier), and determine whether the types of two predecessors at a
CFG merge point are compatible. **If type representations have not yet converged, SSA would require
a separate type-compatibility check for every representation**—and the third representation would
grow accordingly.

This is not a question of which task should come first; it is a question of **which order would
produce three sets of type-compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first complete the convergence of
`pub use ast::Type` at `ir.rs:3` and remove the bridge at `bytecode.rs:2353`. Item 6 in the change
list explicitly records that `ir.rs:3` is **not changed by this article**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not merely an “ugly but functional” file. It is a file that **bets correctness on
every line being read carefully**:

- The file contains **0 `#[cfg(test)]` blocks and 0 `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares three modules—`bytecode`, `def_assign`, and
  `local_slots`—with 29 tests in total. `def_assign.rs` and `local_slots.rs` **positively** cover
  ir_gen’s DefId allocation and local-slot naming at a sentinel level, while the four references to
  ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **all negative assertions**—“ir_gen
  does not produce,” “ir_gen also does not construct,” and “the ir_gen frontend does not construct
  the corresponding IR.”

In other words: ir_gen has 29 sentinel-level tests pinning local behavior, but **no IR structural
invariant validation**—dominance, unique definition, jump targets, or type consistency. None of the
three modules declared in `middle/core/tests/mod.rs` positively asserts the overall IR shape.

This file also happens to contain several classes of defects that **compile successfully, pass all
checks, and let the program run, but merely produce incorrect values**. Their common property is:
**failure produces no error**.

This is where the highest risk lies. The problem is not “the function is too long” or “the naming is
confusing”; it is that **there is no machine-executable enforcement mechanism for this file’s core
invariants**.

The contract “save before generating a nested function body and restore afterward; otherwise names
in the inner function leak into the outer scope” is enforced by the type system: `NestedBodyGuard`
in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator, and there is
exactly one implementation each for frame removal and frame restoration (see “Defect Class 1”).

**Why simply “adding a verifier” is not enough**: a verifier can **catch** these defects, but it
cannot catch their **causes**. The cause is not “someone overlooked them while reviewing,” but
rather:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator deliberately relies on this fact.**

Concrete evidence is the three statement-level register reclamations (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`). At the end of every statement, they **roll `next_temp` back**
to the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and the number and
order of writes, as well as the live intervals, are completely determined by the single manual
decision of **when to roll back**.

Under this shape, “unique definition” is not a rule that can be applied incrementally; it **directly
conflicts with the register-allocation strategy**. Therefore, SSA transformation is not adding a
check to existing code; it is **replacing the allocation strategy that depends on multiple
definitions**. The good news is that replacing this strategy is inexpensive—see “Feasibility
Preconditions” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (Three Reclamations Removed in 7b.4)

**Verified facts.** There are seven entry points at which state must be saved/restored before
entering a nested function body. Batch a (7a.2) consolidated them into one RAII guard:
`NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, with
unconditional restoration in `Drop` and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Restoration scope                                   |
| ------------------------------------ | ---------------------------------- | ----------------------------- | --------------------------------------------------- |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (three counters are not restored) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                       |
| Nested function-definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                  |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |

- Frame removal occurs only in the two constructors (`enter` / `enter_decl`), while frame
  restoration occurs only in `Drop`. The guard exclusively holds `&mut` access to the generator for
  its lifetime, leaving no second `&mut` available in scope; `?` and early returns all restore
  through `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`. From 7b.4 onward,
  `next_temp_reg` maintains only the named high-water mark (temporary-value slots belong to the
  linear-scan allocator; `temp_floor` was removed with the three reclamations).
- `enter_decl` does not restore the three counters: after a function or method body returns to the
  module initialization flow, subsequent definitions continue from the child body’s final value
  number. This is the C1 zero-drift constraint and the meaning of “the declaration surface restores
  only the table.”

**Unified accounting and a single budget point (from 7b.4 onward)**: `finalize_code_body` in
`src/middle/lower/scope.rs` is the sole table-extraction and budget point. It extracts the table
using `frame_slots = named_base + max_live`, reports E3014 with the function-body span when the
limit is exceeded, and defines `MAX_REGISTERS = 255` in exactly one place in the repository. All six
function-body shapes—function, method, lambda, anonymous binding, intermediate curry layer, and
innermost curry layer—go through it for finalization, so there is no second definition of “how many
slots a function body should have.”

**The three statement-level reclamations have been removed (landed with 7b.4 as 7a.1, 2026-11-22)**:
the reclamations were hand-copied in `generate_function_ir` / `generate_block_ir` (with a
`result_reg.is_none()` guard) / `generate_lambda_body_ir`. After the allocator took over slot reuse,
reclamation degenerated into an identity operation, so even the guards, the `temp_floor` /
`saved_temp_floor` bookkeeping, and the loop entry/exit bookkeeping in `loops.rs` were removed.
Module initialization and top-level statements still follow the old monotonic slot path with
`value_mode=false` (they do not run the allocator), and their slot numbers and table lengths shift
with the number of named slots in the function body (measured across three snapshots; behavioral
equivalence is covered by 403/403 cases).

**Consequences in the current shape**: the guard exclusively holds `&mut`, restoration has only one
implementation in `Drop`, and there is no writable location where a restore can be omitted. The
temporary-slot dependency on “one slot with multiple definitions” has disappeared with removal of
the reclamations—value slots are reused by the linear-scan allocator according to live intervals,
and `verify_ssa`’s unique-definition criterion now has a construction-side foundation.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified facts (audit-time wording; current locations have been synchronized with the batch-c
decomposition in 7c).** `generate_call_expr_ir` now resides in `src/middle/lower/expr/call.rs`
(split out from `src/middle/core/ir_gen.rs`, which no longer exists) as a single function. Its
signature accepts `func: &Expr` / `args: &[Expr]` / `named_args: &[(String, Expr)]` / `span: &Span`
/ `call_plan_id: PlanId` / `result_reg: Operand` / `instructions` / `constants` (`call_plan_id` is
the decision-consumption key from RFC-039 D20). Internally, multiple branches share the same
`Vec<Operand>` and perform **semantic rewriting** (the line numbers below are from the audit-time
version; after the decomposition, locate them by symbol):

| Line        | Semantics                                                                                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `7395`      | Namespace call: flatten by `slots`, then recollect—named arguments are reordered                                                                             |
| `7724`      | Struct construction: unwrap each slot from `final_args`, then recollect—fields are reordered                                                                 |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **adds a new register with value 0** for each missing slot (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—clone before emitting instructions                                                                        |

The original source of the four reorderings was:

```rust
// 7395
arg_regs = slots.into_iter().flatten().collect();
// 7724
arg_regs = final_args.into_iter().map(|s| s.unwrap()).collect();
// 7835-7848
arg_regs = slots
    .into_iter()
    .map(|s| {
        s.unwrap_or_else(|| {
            let r = self.next_temp_reg();
            instructions.push(Instruction::Load {
                dst: Operand::Local(r),
                src: Operand::Const(ConstValue::Int(0)),
                span: self.cur_span,
            });
            Operand::Local(r)
        })
    })
    .collect();
// 8000
let final_args: Vec<Operand> = arg_regs.clone();
```

The comment at `7835-7848` states: “Uncovered slots: remain absent so that the type-checking stage
reports E1010. As a runtime safety measure, fill in 0 (the normal path never reaches here).”

**Current state of named arguments (after narrowing in 4.9.9)**: before reordering, named arguments
are first bound according to the **declaration-time formal-parameter name table**—the shared
criterion `arg_binding::bind_args_to_params`. The IR-side name table is obtained through
`lookup_param_info`; its sources are this module’s `fn_param_names`, cross-module method candidate
groups (`method_param_info`, queried by the qualified name from `method_dispatch_name`), and the
module export surface. The cross-module method arm without overload resolution does not carry named
arguments (the old behavior silently discarded them; the known gap is recorded in 09 under 4.9.9 →
4.9.10). The “fill in 0” behavior in the table above remains a runtime safety net for positional
arguments and fallback arms; it is unreachable on the normal path.

**This design has two kinds of risk**:

1. **Misalignment does not report an error; it merely produces the wrong value.** Each of the six
   branches independently determines the final order of `arg_regs`, and an incorrect ordering
   assumption in any branch does not panic.
2. **Normalized snapshots are immune to this problem.** 07 already records this limitation: after
   the snapshot tool renames temporary values by “order of first appearance,” “the third argument
   should have used the fifth register” is identical in the snapshot. **This class of defect has no
   snapshot-level criterion at all in the C4 stage.**

**Additional verification: the struct-construction path’s reordering has no duplicate-slot guard.**
`final_args` at `7724` is populated by “placing positional arguments first, then overwriting by
field name for named arguments,” and **there is no duplicate-slot check such as
`slots[idx].is_some()` before overwriting**—unlike the namespace path at `7381`. When a named
argument silently replaces a positional argument, there is no panic or diagnostic; furthermore,
argument **evaluation order** (source order) diverges from the final `arg_regs` order
(field-declaration order). **The assertions added in 7d.2 cover only the three “missing-slot” cases,
not the “duplicate-slot” case**—named arguments in struct construction can still silently overwrite
positional arguments. This branch is reachable only when type checking has already reported an error
while IR is still generated; the type-checking side rejects this through rule ③ of the shared
`bind_record_ctor_args` criterion with E1111. This gap is recorded in the “7d legacy items” section
of [09](09-execution-wbs.md) and is not closed with 7d.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified facts (at audit time).** The repository had **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                       | Production                                                                                                                      | Transfer                                                                              | Consumption                                                                                                                 |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                         | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (statement identity dispatch through `walk_stmt`)          | `typecheck/types.rs:38` ← passed by `checker.rs` ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, sole entry point; settlement-completed marker `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger in one type, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` only passes it through and merges it) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held in an `ir_gen.rs` field    | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; settlement-completed marker `:1097`) |

The audit-time shape (`ir_gen.rs:1942`, **now nonexistent**) was:

```rust
// NLL Release: insert a Drop instruction at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some`—**if the lookup fails, it simply fails; there is no else branch, no assertion, and no
counter**. If either side computes spans differently, **the Drop instruction silently disappears
with zero errors**. The Drop sequence for refinement types is a core carrier of this project’s
ownership semantics (`ownership.rs:26-32` describes an “NLL precise release plan”), and its lowering
was once silent.

The same shape at `7420` / `7493` (`if let Some(mangled) = self.overload_resolutions.get(span)`)
causes overload resolution to fall back to the default binding.

**Remediation (landed in 4.5)**: both keys were changed to the structural identity `PlanId`, and
consumption is **accounted for on a hit** (reading and accounting occur in the same call). A
nonempty difference is reported as a **level-E `W1090`** error at the sole settlement point—“lookup
failure” is no longer silent. See §2 of [02](02-stage-contract.md) and its revision note (4.5) for
the implemented shape and criteria. These line numbers (`1942` / `7420` / `7493`) are retained as
audit-time evidence and are not current-state pointers.

> **`method_def_ordinals` is not span-keyed (**eliminated in 7c.3**).** It was originally a
> `HashMap<String, usize>` that assigned `#N` suffixes according to the traversal order of
> definitions with the same `base_name`. Its risk was **different**: dependence on definition
> order—if generation order differed from type-check registration order, the same function name
> could resolve to a different target. See mechanism three under “Additional Mechanisms for
> Span-Keyed Problems” for the implemented shape: the definition name is produced once by type
> checking from declaration identity, and lowering only consumes it.

### Defect Class 4: Implicit spawn/lambda Handshake (Eliminated in 7c.1)

**Eliminated.** spawn and spawn-for previously required callers to write `pending_env_vars` /
`pending_env_names` before a synthesized Lambda could consume them; closure bodies separately relied
on `closure_captures` to decide whether a variable read an upvalue. Three fields placed correctness
at the mercy of call order and manual cleanup: a nested synthesized closure could mistake an outer
local slot with the same name for a captured value, and early return from closure-body generation
could leak capture state into subsequent unrelated lowering.

7c.1 consolidated synthesized AST into `middle/lower/synth.rs`, replacing the three shared fields
with an explicit `SynthEnv`, and registering named captures as lexical upvalues. Nested-frame exit
restores state according to symbol-table depth, so error paths no longer leave capture state behind.
See “The `synth.rs` Boundary” for the implemented structure.

### What SSA Will Not Solve (Stated Honestly)

| Problem                                                               | Location                                                                                                                                          | Why SSA does not solve it                                                                                                                                                                                                            |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Complexity of `compile_pattern`                                       | `ir_gen.rs:5390-5687`, **298 lines**, with 4 levels of self-recursion (`5473` / `5504` / `5619` / `5640`), external call site `5270`              | Dispatch complexity for pattern matching (union payloads / tuples / structs / literals / nesting) is **intrinsic to the algorithm**. SSA only turns guards’ `JmpIfNot` into `Phi`; it does not reduce the number of guards           |
| Complexity of `eval_const_expr`                                       | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)                           | Constant-folding expression coverage is unrelated to SSA                                                                                                                                                                             |
| Span-keyed failure (**eliminated**)                                   | Audit time: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`); after 4.5, `PlanId` keys + mandatory settlement                    | **SSA does not touch this layer at all** (conclusion unchanged). It was resolved independently through the obligations ledger in [02](02-stage-contract.md); see mechanism two under “Handling Span-Keyed Problems”                  |
| Number of `Instruction` variants                                      | 75 (`Instruction` enum in `ir.rs`; includes `Phi` introduced in 7b.1a)                                                                            | SSA adds `Phi`, for a **net change of +1** (already landed). Variants either map to bytecode opcodes or degrade to NOP and do not decrease because of SSA; under option A, `Phi` is an IR-only pseudo-instruction and adds no opcode |
| Definition-order dependence in `method_def_ordinals` (**eliminated**) | Audit time: `ir_gen.rs:224` / `369` / `1692-1702`; implemented in 7c.3, see mechanism three under “Additional Mechanisms for Span-Keyed Problems” | Similar to span keying but with a different cause; the definition name is now produced by type checking from declaration identity, and lowering only reads it                                                                        |

## Target Design

### Definition of the SSA Shape

**Design judgment.** The target shape is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has 5 variants. **Repository-wide measurements** (counts obtained by
summing `git grep -c` results per file over the full `src/` tree, including tests;
construction-point counts refer to value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                 | Disposition                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 occurrences                                                                                                                                                                  | **Retained**, unchanged                                                                           |
| `Value(ValueId)`    | 7 files / 26 occurrences; **0 production construction points** when introduced in 7b.1a; from 7b.2 onward, `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`) | **Retained** (introduced in 7b.1a)                                                                |
| `Local(usize)`      | 29 files / 417 occurrences                                                                                                                                                                  | **Retained, but semantics split** (see below)                                                     |
| `Arg(usize)`        | 8 files / 11 occurrences                                                                                                                                                                    | **Retained** as the implicit definition at function entry (parameters are definitions in block 0) |
| `Global(usize)`     | 16 files / 26 occurrences                                                                                                                                                                   | **Retained**, unchanged                                                                           |
| `Temp(usize)`       | **0 construction points** (handling arms exist only in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, and are removed with the variant)                               | **Removed** (dead variant, 7b.1a)                                                                 |
| `Label(usize)`      | **0 construction points**                                                                                                                                                                   | **Removed** (dead variant, 7b.1a)                                                                 |
| `Register(u8)`      | **0 construction points** (the original variant declaration said “Added for codegen,” but codegen uses `OperandResolver` in `codegen/operand.rs` and never constructs the variant)          | **Removed** (dead variant, 7b.1a)                                                                 |

`Local(usize)` is split into two variants because it currently **carries two mutually exclusive
semantics**—“storage location of a named variable” (which may be written multiple times) and
“compiler temporary register” (which should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within a function, with a unique definition
    Value(ValueId),
    /// Storage slot for a named local variable: addressable and allowed multiple definitions (non-SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing and never rolled back**. The split
and `ValueId` were landed in 7b.1a (`Operand` enum in `src/middle/ir/ir.rs`); the production side
(`next_temp_reg` changed to produce `Operand::Value`) was landed in 7b.2 (see “Production Form”
under “Value Identification Method”).

#### `Phi` as a New `Instruction` Variant

```rust
/// Merge point: each predecessor basic block supplies one definition of the value
/// incoming is sorted by predecessor block label in ascending order (guaranteed at construction time to ease normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

There are three construction constraints (see the first layer in [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set of `incoming` equals the label set of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. All `incoming` values have the same `MonoType` (**dependent on convergence of the type
   representation in 03**).

The implementation status of `verify_ssa` (the first three constraints were completed in 7b.1b):
constraints 1 and 2 require the value table and block-level CFG, and were implemented in **7b.1b**
as separate criteria for “block-level predecessor count” and “predecessor label set.” The
predecessor set is inferred from `BasicBlock.successors` (`block_predecessors` in
`src/middle/ir/verify.rs`); a count mismatch reports only the count, while a set mismatch reports
the set, so the same defect is not reported twice. Constraint 3’s per-input type consistency was
implemented at the same time. Values are obtained from the value table at `values[v].ty`, **reusing
the existing `types_compatible`** (no third type-compatibility check was introduced; convergence of
type representations in 03 remains “Precondition Two (hard)”; until then, `ValueInfo::ty` remains
`MonoType`). All three criteria use the violation category `InvariantKind::PhiConsistency`.

**Three forms in which a type is unavailable (explicitly registered, not silently permitted)**: an
empty value table means that the basis for judgment does not exist (the constant shape for module
initialization / top-level statements, via the `value_mode=false` path); a nonempty value table with
an out-of-range `ValueId` is a violation; and a non-`Value` operand (`Const` / `Local` / `Arg` /
`Global`) has no value-table type source and is therefore not checked.

**Measured prerequisites (fulfilled in 7b.3)**: before 7b.3, every production construction of
`BasicBlock.successors` in the repository used `Vec::new()` (block-level predecessors were always
empty, so the predecessor-count and predecessor-set criteria were initially covered only by
synthetic fixtures). **From 7b.3 onward, block-level `successors` are populated for real by
`partition_blocks` in `src/middle/ir/ir.rs` at every production construction point** (all 6 in
`lower` plus 1 synthetic constructor body in codegen go through this sole entry point; lowering
products include multi-block loops and branches, with multiple outgoing edges and back edges). The
predecessor-count and predecessor-set criteria therefore have data from real IR, and direct-driver
tests on real lowering products pin the registration-consumption loop. The `Phi` production
construction point is still 0 (production belongs to a later batch); when it is introduced, it will
be directly constrained by this criterion.

7b.1a only introduced the variant: there are 0 lowering production construction points (and there
are still 0; production belongs to a later batch). The consumer side in `translator.rs` was landed
in **7b.3**: option A expands it at block entry into a sequence of `Move` instructions (a group is
placed at the entry of the merge block, existing `JMP` instructions skip between groups, and
incoming-edge jumps are redirected to the group start according to the predecessor). The `Phi` arm
in `translate_instruction` is an unreachable E8001 defensive arm (`executor` never sees `Phi`).

**Handling the guard issue at `3632-3638`**: the current reclamation in `generate_block_ir` has an
`if result_reg.is_none()` guard (`3626-3631` comment: expression-operand blocks may have outer
sibling-argument temporaries that remain live across blocks). After SSA transformation, this guard
is **no longer needed**—because temporary values are no longer rolled back, sibling arguments that
remain live across blocks hold their own `ValueId` and are unaffected by rollback. **This is a place
where SSA transformation genuinely simplifies the code rather than merely renaming things.**

#### Value Identification Method

**Design judgment**: do not perform memory promotion (mem2reg), do not introduce virtual-register
numbering, and **perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots may be promoted to memory” a
  **type-system problem**, not a CFG problem. It has been verified that `LocalSlot::ty: MonoType` in
  `ir.rs` carries complete type information, but determining “whether this `MonoType` may be
  promoted to memory” requires the converged type representation from 03 and coordination with
  refinement-type rules. **Implementing mem2reg before 03 is guessing.**
- Explicit renaming only requires changing the return-value semantics of one `next_temp_reg()` call
  (`next_temp_reg` in `src/middle/lower/scope.rs`) and deleting three rollbacks; it **does not touch
  the control flow of any lowering algorithm**.

Value-table shape (**landed in 7b.1b**): `FunctionBody::Code` gains `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block containing the definition
}
```

The accessors have the same shape as existing accessors (`impl FunctionIR` in
`src/middle/ir/ir.rs`): `values()` is read-only and returns an empty table for TypeDecl (mirroring
`locals()`); `values_mut()` is mutable and `panic!`s for TypeDecl (mirroring `blocks_mut()`).

**Production form (landed in 7b.2)**: `next_temp_reg()` returns `Operand::Value(ValueId)` and
registers one `ValueInfo` entry in the current function body’s value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). The implementation details are:

- **Counter isolation**: `ValueId` is issued by an independent counter, `next_value: u32`, which
  starts at 0 for each function body, increases monotonically, and is never rolled back;
  `NestedBodyGuard` saves and restores it together with the value table (one set per function body).
  The three statement-level reclamations roll back only `next_temp` (slots), not `next_value`; curry
  layers reset it layer by layer (one `FunctionIR` per layer). Module initialization and top-level
  statements have no value table—when `value_mode=false`, `next_temp_reg` degrades to the old
  `Local` slot allocation, leaving the initialization sequence unchanged.
- **Source of `ty`** (uses only information already available in lowering and does not introduce a
  new type classifier): literals use `ConstValue::mono_type()` (converged with the verifier’s
  constant-type mapping into a single source); variables and calls obtain types from the
  type-checking result and bound function return type at the `generate_expr_ir` entry point;
  comparison, logical, string-family, and forward-propagation instructions are filled in according
  to instruction semantics at function-body finalization. When unavailable, the type is explicitly
  recorded as **unknown `MonoType::TypeRef("?")`** (no silent `Int(64)` placeholder; `LocalSlot.ty`
  is never read). Across 232 snapshots, 9,425 values were measured: 7,080 had real types and 2,345
  were explicitly unknown (24.9%), with unknowns concentrated in call, field, index, and
  container-construction shapes.
- **`defining_block`**: takes the current block label at generation time; **block splitting landed
  in 7b.3** (a real lowering `while` body consists of multiple blocks), but this field still uses
  the generation-time `cur_block_label` (currently always 0), which does not yet agree with the real
  defining block. When 7b.4 regenerated the 232 snapshots, this field was not changed (the `@block`
  in the `values` section did not drift); correction must occur in the same batch as regeneration of
  the `values` section and is **explicitly deferred to the `Phi` production batch**.
- **`debug_name`**: always `None` (named variables remain `Local` and do not move to the value
  table).
- **Indexing**: `value_slots: Vec<usize>` (index = `ValueId`) has the same indexing as `values` and
  records the value-to-frame-slot mapping. From 7b.4 onward, it is recomputed by the linear-scan
  allocator in `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`); codegen’s `OperandResolver` only consumes it and does not
  allocate.

`values` and `locals` coexist: `locals` carries addressable named variables (which may have multiple
definitions), while `values` carries single-definition SSA values. **The existence of `locals` does
not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Relationship to `LocalSlot::origin`**: these are **two dimensions** and must not be conflated.
`LocalSlot::origin` (the **slot-identity** field introduced on the ownership side in 2.1.4,
recording the generating `FrameId` of the slot) answers “which frame does this slot belong to,”
while `ValueInfo.defining_block` answers “in which basic block is this SSA value defined?” Value
identity belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`; 7b.1b only added
the value table and **did not change `LocalSlot::origin`** (verified: zero changes).

#### Change-Volume Estimate

| Item                                                         | Estimate                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------ | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Modify `next_temp_reg`                                       | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Delete three statement-level reclamations                    | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Consolidate 6 save/restore sites into RAII                   | Net ±0 lines                          | See “Per-Item Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Structural changes to `ir.rs`                                | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **the first two landed in 7b.1a and the latter two in 7b.1b** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Add `Phi` arm to `translator.rs`                             | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `executor/`                                                  | **Zero changes**                      | Option A: `Phi` is an IR-only pseudo-instruction expanded by `translator.rs` into a Move sequence and does not enter bytecode (Decision Three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Call sites that produce temporary values as `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (`git grep -c` summed per file over the full `src/` tree, including tests): `Operand::Local(` occurs 415 times in 28 files, of which `src/middle/lower/` accounts for 332 occurrences in 14 files; the production point for temporary values is the single `next_temp_reg` site. **7b.2 measurement (baseline-revision remeasurement and net landed change)**: `next_temp_reg()` has 140 call sites in 14 files; the `result_reg: usize` parameter chain spans 35 functions plus 3 `Option<usize>` result slots; `src/middle/lower/` contains 342 `Operand::Local(` occurrences; code-only diff against `src/` totals +23,319/−12,495, including +21,866/−11,950 from regenerating 232 IR snapshots, with a net source-and-test change of +1,453/−545 (mechanical replacements and normalize rendering, mono projection, six direct-driver tests; also including 5 generated error-code artifacts) |

### Per-Item Disposition Table

| Defect class                                                | Eliminated by SSA?         | Disposition                                                                                                                                                                                                                                                                                                                                                                                                  | Criterion                                                                                                           |
| ----------------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore**         | **Yes (resolved in 7b.4)** | The 7 nested function-body entry points are consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and counters), turning “forgot to restore” into a compile error. The three statement-level reclamations were removed with 7b.4 (the allocator reuses slots by live interval), and temporary values no longer occupy `next_temp` | First-layer “inner isolation” invariant in 07 + `verify_loose` / `verify_ssa`                                       |
| **1b. Inconsistent accounting**                             | **Yes (landed)**           | Unified in `finalize_code_body` in `src/middle/lower/scope.rs`: table extraction uses `frame_slots`, the E3014 budget check occurs at the same point as extraction and includes the function-body span, and all six function-body shapes are covered; `MAX_REGISTERS` has one repository-wide definition                                                                                                     | Corpus differential + zero snapshot drift                                                                           |
| **1c. Criteria for the three statement-level reclamations** | **Yes (landed in 7b.4)**   | The three reclamations, the `result_reg.is_none()` guard in `generate_block_ir`, and `temp_floor` bookkeeping were removed; slot reuse is handled by the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                                     | C4: behavioral equivalence (403/403) + invariants                                                                   |
| **2. Semantic reordering of `arg_regs`**                    | **Partial**                | SSA turns “wrong register” into “wrong value ID,” which dominance checking can catch, but **the reordering logic itself remains**. Actual elimination depends on function decomposition—each of the 6 branches performs its own `push`, while ownership of `arg_regs` remains at the top of the function                                                                                                     | First-layer dominance invariant (immune to snapshots)                                                               |
| **3. Span-keyed failure**                                   | **No**                     | SSA does not touch it at all. See “Handling Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                             | `test_release_plan_spans_consumed` in 07                                                                            |
| **4. Implicit order dependence**                            | **No**                     | See “The `synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                                | Guaranteed by code structure and review; the original gate script was cancelled by the user’s 2026-11-24 Decision D |
| `compile_pattern`, 298 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                | —                                                                                                                   |
| `eval_const_expr`, 191 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                | —                                                                                                                   |
| 75 `Instruction` variants                                   | No (+1, landed)            | `Phi` was introduced in 7b.1a; under option A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                         | —                                                                                                                   |

### Register Allocator: In-House or an Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **0 construction points** across the repository—the variant is no
   longer present in the `Operand` enum in `ir.rs` (removed in 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper bound of 255 (exceeding it
   produces E3014). `Operand::Value` is resolved by the allocator introduced in 7b.4 through
   `value_slots` to a frame slot (`>255` produces E3014; no mapping produces E3017 with the
   instruction span).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50
   `translate_*` functions, with function bodies distributed across `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`. Opcodes come from `src/backends/common/opcode.rs`
   (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter’s opcode-dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Assessment of three options**:

| Option                                               | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)    | **Not adopted.** Both take rustc’s `Function` trait and MIR’s `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait (CFG, liveness, spill slots, scratch space). **500+ lines of adapter code for zero benefit**, because there is no real register file to color. Moreover, rustc itself only performs linear scanning at this point (when `enable-llvm` is disabled)                                                                                                              |
| `graph-alloc` family                                 | **Not adopted.** Same reasons, with a less stable API                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| **In-house linear scan (simplified Chaitin-Briggs)** | **Adopted (landed in 7b.4).** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot according to its live interval (`OperandResolver` already has a `u8` contract and the E3014 upper bound). Implement `src/middle/lower/regalloc.rs`: block-level backward dataflow over `successors`, closed intervals, Phi predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`. Exceeding the limit returns `Budget`, and `finalize_code_body` reports E3014 with the declaration span |

**Implementation measurements (7b.4)**: the two corpus files that previously exceeded the limit (354
/ 323 monotonic slots) now use **15 / 24 frame slots** after allocation (344 / 304 values), and the
remaining corpus files are behaviorally equivalent in all 403/403 cases. E3014 now means only “frame
slots after allocation exceed 255.” The counter used as a fallback for back edges is currently not
exposed (`wrap_count` is computed and discarded); the `Phi` production batch must expose and retest
it first (a prerequisite alongside `defining_block`). **Overflow handling**: `u8`/255 remains an
error handled by the budget gate; this article does not solve the case of “not enough registers for
a large function,” which is a downstream consequence of `compile_pattern` recursion depth.

The dev-dependencies at `Cargo.toml:37-39` contain only `criterion` / `proptest`; `[dependencies]`
contains no regalloc, petgraph, or graphlib. Introducing any crate would add a new dependency
surface.

### Decomposition of `generate_call_expr_ir` (Landed in 7d.1)

Before decomposition, this was a single 920-line function (746 lines at audit time; 920 lines after
moving to `middle/lower/expr/call.rs`) handling six forms of call. The implemented shape is:

```
generate_call_expr_ir（call.rs:479-568，function body 78 lines, dispatch only）
├── Return immediately on a match, in this order: variant construction → namespace call → cross-file global-slot indirect call
│                    → method-call reordering → built-in container construction → ordinary/overloaded call
├── CallArgs（call.rs:31-271）— the sole carrier for argument collection / named reordering / slot binding
│    collect_regs / bind_named_slots / collect_regs_in_slot_order / arg_expr
│    Slot determination still belongs exclusively to sema::inference::arg_binding::bind_args_to_params
├── emit_variant_ctor              call.rs:575-621    (RFC-010 variant construction)
├── emit_namespace_call            call.rs:631-684    (namespace call)
├── emit_global_slot_indirect_call call.rs:695-717    (cross-file global slot → CallDyn)
├── emit_method_call_reordered     call.rs:728-853    (RFC-004 binding reordering)
├── emit_builtin_container_ctor    container.rs:175-220（RFC-011 §Vec container construction）
└── emit_plain_call                call.rs:864-1457   (unbound method + function forms)
```

**Corrections to two assumptions in the old sketch**: ① The original design assumed that “SSA has
reduced `arg_regs` to one `ValueId` per argument, and the 6 branches only push without
reordering”—this premise does not hold in the current tree. C1 requires order preservation, so the
implementation retains three reordering paths (`set_regs` / `collect_regs_in_slot_order` /
`bind_named_slots`) and places all of them in `CallArgs`, rather than scattering them across
branches. ② The original names `emit_bound_call` / `emit_struct_ctor` / `emit_curried_call` /
`emit_method_call` do not match the six implemented names; this table is authoritative.

**Consolidated inline copies** (eliminated during the decomposition rather than moved verbatim):
positional-argument evaluation 5→1, named-argument slot insertion 2→1, reconstruction in slot order
2→1, and positional/named index conversion 4→1.

**Assertion of the “fill in 0” fallback in 7d.2**: the three equivalent `unwrap_or_else` branches
that inserted `Load 0` now return the existing **E3005** (`ErrorCodeDefinition::ir_internal_error`).
Its carrier is `CallArgs::require_complete_slots`, not the sketch’s `fill_missing`. The **only
retained** zero-value construction is the field initialization of the empty `Pair(Int)()`
construction, which is legal zero initialization under SPEC §4.3; the corpus file
`generic_constructor.yx` asserts `z.first == 0`.

**Criteria**: for the pure move portion, C1 snapshots have zero diff and all 412 corpus files are
byte-for-byte identical; for the assertion portion, C4 applies (4 new direct-driver cases, with the
old implementation failing and the new implementation passing).

### The `synth.rs` Boundary: Turning an Implicit Callback into an Explicit Synthesized-AST Boundary

**Implemented shape**: `src/middle/lower/synth.rs` is the only L3/L4 production module permitted to
construct `ast::Expr` / `ast::Stmt` / `ast::Type`. Pattern matching, read-only queries, and test
fixtures are not synthesis. The current production synthesis points are spawn’s Lambda/Stmt and
spawn-for’s Lambda, all of which go through this module.

AST nodes do not carry a lowering environment, so `SynthEnv` is not an AST field and is not
pretended to be consumed by an AST helper. It contains runtime environment operands and capture
names, and the caller explicitly passes it to `generate_expr_ir_with_env` together with the
synthesized node:

```rust
pub(super) struct SynthEnv {
    pub(super) env_vars: Vec<Operand>,
    pub(super) env_names: Vec<String>,
}

pub(super) fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    span: Span,
) -> ast::Expr
```

The environment flow is as follows:

- spawn and spawn-for construct `SynthEnv` and pass it explicitly into lowering with the synthesized
  Lambda; they no longer prewrite `pending_env_vars` / `pending_env_names`. The total
  environment-slot count is derived from `env_vars.len()`, and named upvalues start after the
  anonymous-prefix length. Consequently, spawn-for’s anonymous iteration value, outer captures, and
  formal parameters each occupy their proper positions.
- Ordinary lambdas complete the environment using the existing free-variable collector. List
  comprehensions, nested Lambdas, and nested spawns are traversed by the same collector, while
  generator variables and nested formal parameters are excluded by lexical scope.
- Closure bodies register `SynthEnv` as a `BindingLocation::Upvalue` in the existing symbol table.
  Variable reads and namespace-shadowing queries share this lexical binding and no longer read
  `closure_captures`.
- Nested closures first read the outer upvalue through `capture_operand`, then pass the value to the
  inner environment; they no longer mistake an outer local slot with the same name for a capture.
- `NestedBodyGuard` records the symbol-table depth on entry and, on exit (including early `?`
  returns), truncates it to that depth, so capture bindings do not leak into subsequent unrelated
  lowering.
- The sema expression-body function check parameterizes existing inputs with
  `body_stmts + expression_body + span`, directly calls `check_expr`, and reuses tail validation
  instead of synthesizing Block/Stmt/Expr; there is no reverse dependency from sema to middle/lower
  and no additional architectural exception.
- SpawnFromList shares the `env + [anonymous item]` args layout in Embedded and Standard, and
  injects the same environment as upvalues. The anonymous iteration value is fixed at env[0], named
  captures are read after the anonymous prefix, and formal parameters start after the entire
  environment.
- Dual-runtime behavior tests explicitly invoke main() and, inside spawn-for tasks, separately
  assert an item without captures and an item plus an outer capture; the no-capture form also covers
  the regression restoration introduced by 74febd1f. IR assertions and actual execution are recorded
  separately; zero drift across the 403 existing corpus files does not masquerade as behavioral
  coverage of this form.

**Criteria**: D22’s AST-synthesis boundary is guaranteed by the code structure—L3/L4 production code
constructs `ast::Expr` / `ast::Stmt` / `ast::Type` only in `src/middle/lower/synth.rs`; pattern
matching, read-only queries, and test fixtures are not synthesis. Synthesis points outside
`synth.rs` are rejected during review. The original `scripts/ci/check-synth-boundary.py` and the CI
`synth-boundary` job were removed under the user’s 2026-11-24 Decision D and are no longer
maintained as automatic scanning gates: heuristic token scanning cannot cover arbitrary legal use /
alias / re-export forms (even after successive fixes, it remained bypassable through `pub type`
re-exports), and its maintenance cost exceeds its benefit.

D1/D2: `SynthEnv` and `BindingLocation` are both new internal types introduced in this batch, not
reuse of existing types. The former centralizes production-side environment operands, named items,
and slot derivation into an explicit parameter. The latter promotes the original
`SymbolEntry.local_idx` into a discriminable Local/Upvalue location. The alternative—continuing to
pass three shared fields (which cannot prevent nested overwrites or erroneous residual state), or
passing only `Vec<Operand>` (which cannot express the correspondence between the anonymous prefix
and named captures)—was rejected for correctness reasons. The new concepts are not synonymous with
existing AST or consumption-ledger concepts and require neither import aliases nor conversion
tables.

### Handling Span-Keyed Problems (the Part SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism One: Consumption-coverage assertion (mandatory, lowest cost; superseded by a stronger
form on 2026-10-13).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). The
original supplementary proposal in this article was to make it a **runtime counter** rather than
merely a test; the implemented shape is stronger—**a consumption ledger (`ConsumptionLedger<K>`) + a
single settlement point (level-E `W1090`)**. It adds no diagnostic code and does not rely on an
internal ir_gen counter to prove itself:

```rust
// Near ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each hit at :1942, self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are runtime-observable (destructor
side effects), but **“a value that should have been dropped was not dropped” may not immediately
crash under YaoXiang’s current value semantics + Arc/Rc model**—which is exactly why the problem was
silent. The counter turns “silent” into “detectable.”

**Mechanism Two: Change span keys to `DefId` or explicit plan IDs (**landed on 2026-10-13**: RFC-039
D20 / WBS 4.5 implemented this mechanism as a `PlanId(u32)` structural-identity key + mandatory
consumption settlement; see the revision note in §2 of [02](02-stage-contract.md) for the
implemented shape).** The fundamental fix is **not to use `Span` as a key**. `Span` is a source
location and may drift because of macro expansion, `include`, and multi-file merging. `FunctionIR`
already has a `def: Option<DefId>` field (`ir.rs:677`), demonstrating that DefId is available at the
IR layer. **Design judgment**: introduce `PlanId(u32)` for `ReleasePlan` and `overload_resolutions`,
allocated by ownership / overload resolution when producing them, and match IR consumption by
`PlanId`. **This belongs to the obligations ledger (`Obligations`) in 02 and should be implemented
together with [02](02-stage-contract.md), not in this article.**

**Mechanism Three: Definition-order dependence in `method_def_ordinals` (independent small item,
**landed on 2026-11-25**).** The field was originally a `HashMap<String, usize>` whose key was the
unqualified `"{type_name}.{method_name}"`; it assigned `#N` mangled names according to definition
traversal order. If type-check registration order differed from lowering definition order,
identically named methods could resolve to different targets. **Implemented shape**: at the same
point where it registers overload candidates, type checking **produces the names once** as
`method_def_ir_names: HashMap<PlanId, String>` (bare or `#N` mangled names) and passes them to IR
generation through `TypeCheckResult`. The key is the **`PlanId` of the method-declaration
statement** (the same AST structural identity used by D20)—`DefId` is not assigned until
`assign_defs` at the end of lowering and therefore does not exist on the type-checking side.
Lowering reads the name solely by declaration identity, and **a miss fails loudly with E3005** (it
does not fall back to the bare name, because that fallback would disguise “the definition name and
call name came from different sources” as a runnable program). The mutable counter and its write
sites were removed, and definition names no longer have a second derivation path.

### Tracking Hard-Coded Discarded Data

The original `impl From<BytecodeFile> for BytecodeModule` (now `src/middle/bytecode/mod.rs`) had
three hard-coded discards. After the 7e review, these consist of **two genuine losses + one false
positive**:

| Location (old line number) | Field                | Original value                                 | Review conclusion                                                                                                                                                                                                                                                                                                           |
| -------------------------- | -------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312`         | `upvalue_count`      | `0` // Not stored in BytecodeFile              | **Genuine loss**: the closure upvalue-slot count becomes zero after a `.42` round trip; fixed with the v5 persistence work (7e.1)                                                                                                                                                                                           |
| `bytecode.rs:2315`         | `exception_handlers` | `Vec::new()` // Not implemented yet            | **False positive**: the field has no producer or consumer, and the language layer has no throw/try/catch (RFC-030 records `Throw` as a bytecode-layer reservation, while RFC-036 rejects an in-process catch boundary) ⇒ there is no “lost data”; it is a dead field. It is not persisted and its removal belongs to P10 S6 |
| `bytecode.rs:2341`         | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **Genuine loss**: the global table (names + types) becomes zero after a round trip; fixed with the v5 persistence work (7e.3), with the carrier and function signature both using `MonoType` (7e.4)                                                                                                                         |

**Criterion (Decision D52)**: an empty table with no producer is not lost data; it is a dead
path—the same criterion as D32. Only a table with a producer that is not persisted constitutes data
loss, so `2312` / `2341` were included in P7 batch e (7e); `2315` is not persisted, and its removal
as a dead field belongs to P10 S6’s opcode-dead-path cleanup (`THROW` remains a separate item
awaiting a decision). A single version bump covers the two genuine losses (`VERSION` 4→5, D17);
`.42` is a build artifact, and cross-version compatibility is not a goal.

**A third format distortion in the same 7e batch**: the `.42` type-number table was irreversible and
had collisions—`Int(64)` and `Float(32)` were both `9`, while `Float(64)` was read back as
`TypeRef("_")`. The number table was changed to be reversible and collision-free (7e.4), and the
writer now rejects unknown bit widths and compile-time FFI types loudly.

## Detailed Design

### Knock-on Effects of IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                  | Nature                                                                                                     |
| ------------------------------------------------------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add `ValueId(u32)` type                                                   | File header (`ValueId` definition)        | New (**landed in 7b.1a**)                                                                                  |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                 | **Variant additions/removals** (**landed in 7b.1a**)                                                       |
| Add `Phi` variant to `Instruction`                                        | End of `Instruction` enum (`Phi` variant) | **+1 variant** (**landed in 7b.1a**; the enum now has 75 variants)                                         |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                      | +1 field (**landed in 7b.1b**)                                                                             |
| Add `ValueInfo` struct                                                    | `ValueInfo`                               | New (**landed in 7b.1b**)                                                                                  |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                | Add `values()` / `values_mut()` accessors (**landed in 7b.1b**, with the same shape as existing accessors) |

The `pub use ...ast::Type` at the file header is **not changed by this article**—it is in the scope
of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **converged** single type.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                          | Location                              | Nature                                                                                                |
| ------------------------------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or adopt the “expand to Move at runtime” strategy | `127-554` (currently **66** variants) | +1 or +0 variants (see below)                                                                         |
| Add `Phi` arm to `opcode()`                                                     | `558`                                 | Required (`BytecodeInstr` has an exhaustive match)                                                    |
| Add `Phi` arm to `size()`                                                       | `649`                                 | Required                                                                                              |
| Add `Phi` arm to the `From<BytecodeFile>` decoding match                        | `943-2350`                            | Required                                                                                              |
| Correct `upvalue_count: 0`                                                      | `2312`                                | Fixed incidentally by this article (in-memory format; persistence requires a version bump, see below) |

**The bytecode shape of `Phi` requires a design decision**. There are two options (**design
judgment: A is recommended**, for the reasons under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded by `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  selected by predecessor-block index. **No new opcode is added**, and the `.42` format is
  unchanged. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode + predecessor-index encoding, with the interpreter maintaining register
  versions on jumps. **This requires changing the `executor/` state model + the `.42` format + the
  version number**. The only benefit is size reduction.

The decisive reason for choosing A is that changing the `.42` format triggers version-number issues
and a full compatibility discussion, **while A’s size cost is irrelevant in interpreted-execution
scenarios** (the executor runs Move instructions, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions, distributed across `101-1558`)

| Change                                                       | Location                                                                                                                         | Nature                                                                          |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                                 | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                         |
| `Instruction::Phi` translation arm                           | Inside the dispatch match, before `Free`(533)                                                                                    | New branch that expands into a Move sequence (option A)                         |
| Existing 9 NOP degradations **remain in the same positions** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9**: `655-657` covers 2 variants, and `658-660` covers 4 variants |

**SSA transformation does not reduce the NOP count.** These 9 degradations reflect gaps in
expressiveness where “the IR has it, but bytecode does not”; SSA transformation does not touch them.

#### `src/backends/interpreter/executor/`

With option A, `executor/` requires **no changes**—`Phi` disappears during the `translator.rs`
stage, before the interpreter can see it. **This is the second decisive reason for choosing A.**

`BytecodeInstr::Switch` in `executor/ops/control.rs:90` has an active implementation, but **no**
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs` (measured: all three names have zero
matches in `ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions that the IR layer
cannot produce. It does not affect this article, but it is recorded here because it shows that “IR →
bytecode” is not surjective.

The dispatch match in `executor/debug.rs:194` likewise requires no changes (no new opcode under
option A).

### Compiler Change List

**File by file and function by function.** All line numbers are the measured starting points for the
changes:

| No. | File                                                                      | Location                                                                           | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                                                     | `Operand` enum definition                                                          | Add/remove `Operand` variants (**landed in 7b.1a**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 2   | `src/middle/ir/ir.rs`                                                     | End of `Instruction` enum (`Phi` variant)                                          | Add `Instruction::Phi` (**landed in 7b.1a**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 3   | `src/middle/ir/ir.rs`                                                     | `FunctionBody::Code`                                                               | Add `values` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 4   | `src/middle/ir/ir.rs`                                                     | `ValueInfo`                                                                        | Add `ValueInfo` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 5   | `src/middle/ir/ir.rs`                                                     | `values()` / `values_mut()` accessors                                              | Add accessors (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 6   | `src/middle/ir/ir.rs`                                                     | `pub use ast::Type`                                                                | **Leave `pub use ast::Type` unchanged** (scope of [03](03-type-unification.md); recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 7   | `src/middle/core/ir_gen.rs`                                               | `884-891` `next_temp_reg`                                                          | Return `Operand::Value`, increasing monotonically without rollback (**landed in 7b.2**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; add an independent `next_value` counter starting at 0 for each function body, while the three statement-level reclamations roll back only `next_temp`)                                                                                                                                                                                                      |
| 8   | `src/middle/core/ir_gen.rs`                                               | `1954-1959`                                                                        | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 9   | `src/middle/core/ir_gen.rs`                                               | `3632-3638`                                                                        | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 10  | `src/middle/core/ir_gen.rs`                                               | `4737-4742`                                                                        | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 11  | `src/middle/core/ir_gen.rs`                                               | `1745-1759` / `1819-1822`                                                          | Replace with an RAII guard (`generate_method_ir`, entry `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 12  | `src/middle/core/ir_gen.rs`                                               | `1858-1866` / `2026-2029`                                                          | Replace with an RAII guard (`generate_function_ir`, entry `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 13  | `src/middle/core/ir_gen.rs`                                               | `2116` / `2180`                                                                    | Replace with an RAII guard (`generate_curry_innermost_func`, entry `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 14  | `src/middle/core/ir_gen.rs`                                               | `2224-2229` / `2284-2288`                                                          | Replace with an RAII guard (`generate_curry_function_ir`, entry `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 15  | `src/middle/core/ir_gen.rs`                                               | `2768-2777` / `2820-2851`                                                          | Replace with an RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 16  | `src/middle/core/ir_gen.rs`                                               | `4686-4695` / `4767-4774`                                                          | Replace with an RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 17  | `src/middle/core/ir_gen.rs`                                               | `1987-2000`                                                                        | Extract the E3014 check into `check_register_budget()` for shared use by `1798` / `2001` after accounting is unified                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 18  | `src/middle/core/ir_gen.rs`                                               | `1798`                                                                             | Change `take_cur_locals(param_types.len())` to `take_cur_locals(self.temp_high_water)`, eliminating inconsistent accounting                                                                                                                                                                                                                                                                                                                                                                                                            |
| 19  | `src/middle/core/ir_gen.rs`                                               | `248-249`                                                                          | Delete the comment-based contract (now enforced by the type system) and replace it with an explanation pointing to `verify`                                                                                                                                                                                                                                                                                                                                                                                                            |
| 20  | `src/middle/lower/loops.rs`                                               | `generate_spawn_for_ir`                                                            | Call `synth::synth_closure` instead (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 21  | `src/middle/lower/expr/spawn.rs`                                          | `generate_spawn_expr_ir`                                                           | Call `synth::synth_closure`; remove writes/clears of shared pending fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 22  | `src/middle/lower/expr/closure.rs`                                        | `generate_lambda_expr_ir`                                                          | Read `SynthEnv` from the arguments instead of `mem::take`ing shared fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 23  | `src/middle/lower/{mod,scope}.rs`                                         | Symbol table and `NestedBodyGuard`                                                 | Remove the `closure_captures` field; add `BindingLocation::Upvalue` lexical binding and depth-based restoration (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                   |
| 24  | `src/middle/lower/expr/{access,closure}.rs` + `query.rs`                  | Variable and namespace queries                                                     | Replace `closure_captures.contains_key` with a lexical-upvalue lookup; materialize captures exclusively through `capture_operand` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                 |
| 25  | `src/middle/core/ir_gen.rs` (audit-time path)                             | `341` + `1942`                                                                     | ~~Add span-keyed consumption counting~~ → covered by the 4.5 `PlanId` consumption ledger (read and accounting occur together, settlement through W1090); 7c.2 verified zero triggers, with no code changes                                                                                                                                                                                                                                                                                                                             |
| 26  | `src/middle/lower/{mod,stmt}.rs`, semantic side `sema/{checker,types}.rs` | Definition name for `generate_method_ir` + authoritative `TypeCheckResult` mapping | Make `method_def_ordinals` read-only (**landed in 7c.3**: type checking produces the mangled name from the declaration `PlanId`, and lowering only reads it)                                                                                                                                                                                                                                                                                                                                                                           |
| 27  | `src/middle/lower/expr/call.rs` + `container.rs`                          | Audit-time `ir_gen.rs:7269-8014`                                                   | Decompose into `CallArgs` + 6 `emit_*` functions (**landed in 7d.1**: `eb13cac`; the main function at `call.rs:479-568` performs dispatch only)                                                                                                                                                                                                                                                                                                                                                                                        |
| 28  | `src/middle/lower/expr/call.rs`                                           | Audit-time `ir_gen.rs:7838-7846`                                                   | Change the “fill in 0” fallback to return the existing **E3005** (**landed in 7d.2**: `e246d6d`; legal zero-value construction is retained)                                                                                                                                                                                                                                                                                                                                                                                            |
| 29  | `src/middle/lower/synth.rs`                                               | **New file**                                                                       | Synthesized-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 30  | `src/middle/ir/verify.rs`                                                 | **New file** (first layer of [07](07-equivalence-oracle.md))                       | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 31  | `src/middle/passes/codegen/translator.rs`                                 | Dispatch match                                                                     | Translation of `Instruction::Phi` (**landed in 7b.3**: option A expands at **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit Move groups by predecessor, existing `JMP` instructions skip between groups, and incoming-edge jumps are redirected to the group start using `(predecessor, merge-block first instruction)`; the `Phi` arm in `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return site cannot hold multiple Moves distributed by predecessor, so reaching it fails loudly) |
| 32  | `src/middle/passes/codegen/operand.rs`                                    | `36-45` / `72-77`                                                                  | Add the `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`) (**landed in 7b.2**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); removal of the `Temp` arm preceded this in 7b.1a)                                                                                                                                                                                                                                     |
| 33  | `src/middle/core/bytecode.rs`                                             | `127-554`                                                                          | **Unchanged under option A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 34  | `src/middle/core/bytecode.rs`                                             | `558` / `649` / `943-2350`                                                         | **Unchanged under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 35  | `src/middle/core/bytecode.rs`                                             | `2312`                                                                             | Correct `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 36  | `src/backends/interpreter/executor/**`                                    | —                                                                                  | **Zero changes under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 37  | `src/middle/lower/regalloc.rs`                                            | **New file**                                                                       | Linear-scan allocator (**landed in 7b.4**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)` returns `value_slots` and `frame_slots = named_base + max_live`; exceeding the limit returns `Budget`, and `finalize_code_body` reports E3014)                                                                                                                                                                                                                                                                 |
| 38  | `scripts/ci/check-synth-boundary.py`                                      | **Cancelled**                                                                      | The boundary-sentinel script was cancelled under the user’s 2026-11-24 Decision D; do not create a replacement                                                                                                                                                                                                                                                                                                                                                                                                                         |

### Backward Compatibility: Versioning of the `.42` Format

**Verified facts**: the on-disk `.42` format **already has a version
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (“YXBC”) and
`VERSION: u32 = 4`, writes them to the header (`:342-343`), and validates them when reading
(`:454-467`, reporting “unsupported bytecode version” on a mismatch).

**Design judgment (a direct consequence of option A)**:

- `Phi` is expanded into a Move sequence in the `translator.rs` stage and **produces no new
  opcode**.
- The 83 constants in `opcode.rs` have **zero changes**; `opcode_name()` (`120`) is unchanged, and
  the decoding match (`bytecode.rs:943-2350`) is unchanged.
- The `.42` format’s field layout has **zero changes**, and `VERSION` remains 4. Old `.42` files can
  still be read by the new binary, and new `.42` files can still be read by the old binary. **SSA
  transformation itself does not require a version bump.**

**The only scenario requiring a version bump is correction of `2312 upvalue_count`** (Decision D17:
fix it and handle it together with the version bump): adding an `upvalue_count` field to
`BytecodeFunction` and writing it in the encoder is a **new format field**, so `VERSION` is raised
from 4 to 5. The reader’s existing version check rejects old files under its current behavior—`.42`
is a build artifact (`main.rs:671`), cross-version read compatibility is not a goal, and no
additional migration mechanism is needed. Allocated but unused opcode values (D32/D34’s `Switch` /
`TailCall`) are reclaimed together with the version bump.

## Implementation Notes

### Batch Strategy

There are four batches. **Each item within a batch gets an independent commit, independent criteria,
and an independent revert**. “Change item” in the table below refers to the number in the preceding
“Compiler Change List.”

| Batch       | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Change items                 | Criteria                                                                                                                                                           | Rollback point                                                                                                                                       |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Break multiple definitions: delete 3 statement-level reclamations → replace 6 save/restore sites with RAII guards → unify accounting                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 8–10 (first) → 11–16 → 17–18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and accounting unification are pure moves: **C1** snapshot zero-diff | One commit per item; `git revert` rolls it back                                                                                                      |
| **Batch b** | Switch to the SSA shape: `ir.rs` structural changes + temporary values produce `Operand::Value` (mechanical replacement) → `Phi` arm in `translator.rs` (option A) → linear-scan allocator. Within the batch, follow the three-level WBS sequence: 7b.1a (`Operand` variant convergence + introduction of `Instruction::Phi`, **landed**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **landed**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **landed**) → 7b.3 (`Phi` arm in `translator.rs` + population of `BasicBlock.successors`, **landed**) → 7b.4 (linear-scan allocator + deletion of the three statement-level reclamations, together with 7a.1, **landed**) | 1–5, 7 → 31–32 → 37          | **C4**: behavioral equivalence + all `verify_ssa` checks green + `.42` round-trip tests                                                                            | The `ir.rs` changes get **one exclusive commit** (type changes affect compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary → verify the existing consumption ledger (no code changes) → make `method_def_ordinals` read-only (the sentinel script was cancelled under the user’s 2026-11-24 Decision D)                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 20–24, 29 → 25 → 26          | **C4**: behavioral equivalence; the new diagnostic records zero consumption-count triggers (otherwise, a real mismatch exists)                                     | Independent commit                                                                                                                                   |
| **Batch d** | Decompose `generate_call_expr_ir` + assert the “fill in 0” fallback (**landed**: 7d.1 `eb13cac`, 7d.2 `e246d6d`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 27–28                        | Pure-move portion: **C1** snapshot zero-diff. Assertion portion: **C4** corpus differential + first-layer verifier                                                 | Independent commit                                                                                                                                   |

**There are three dependencies whose order within the sequence cannot be exchanged:**

1. Batch a’s “delete reclamations” is scheduled after batch b’s `ir.rs` changes (deletion of the
   three statement-level reclamations landed with 7b.4 and the linear-scan allocator, with 7a.1
   executed together)—first make `next_temp` increase monotonically, so that `ValueId` has the
   semantic basis for “unique definition” (this semantics becomes established only after 7b.2
   changes `next_temp_reg` to produce `Value` and 7b.3 lands the `Phi` arm).
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise the decomposition would copy
   the old `arg_regs` ordering assumptions verbatim into the new file.
3. Batch d must come last—it depends on the preceding batches having converged `arg_regs` semantics
   into “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within RFC-039’s **P7**. The criterion tiers
(`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial** relative to one
another, but items 8–10 / 11–16 / 17–18 within batch a have only ordering constraints and are not
coupled.

### Hard Preconditions

**Precondition One: Criterion baseline.** The first-layer verifier’s `verify_loose` mode in
[07](07-equivalence-oracle.md) **must pass on the existing (non-SSA) IR before batch a may begin**.

Reason: SSA transformation’s criterion is C4 (behavioral equivalence + invariants). If the invariant
verifier itself cannot pass on the IR before the transformation, every acceptance step in batches
a–d lacks an executable criterion and must rely solely on corpus differentials—yet corpus
differentials are **precisely insensitive** to defect classes 1/2/3 (as demonstrated by “Alternative
A” in 07). **This is a real ordering constraint.**

**Precondition Two: Convergence of type representations.** [03](03-type-unification.md) must
complete the convergence of `pub use ast::Type` at `ir.rs:3` and remove the
`From<MonoType> for IrType` bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` would
introduce a third set of type-compatibility judgments.**

### Rollback Strategy

- Each change gets one independent commit; `git revert` granularity equals change-item granularity.
- Batch b’s `ir.rs` structural change is the **only part that cannot be rolled back incrementally**
  (type changes affect compilation throughout the repository). It must therefore **occupy an
  exclusive commit**, and batch a must have passed and established a revertible baseline before it.
- **Do not use a feature flag.** Reason: adding/removing `Operand` variants cannot be isolated with
  a runtime switch; a feature flag would cause CI to stop testing the other path over the long
  term—a repetition of the “test wiring depends on human memory, and silent decay occurs” problem
  recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front; otherwise the implementation will be challenged midway for “not
reducing the line count.”**

| Item                                   | Line-count change                                                                                                                                                                              |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                         | `ir_gen.rs` **8,448**                                                                                                                                                                          |
| Delete 3 statement-level reclamations  | **−15**                                                                                                                                                                                        |
| Replace 6 save/restore sites with RAII | Net ±0 (the RAII guard definition adds about +20 lines, while call sites shrink from 22 lines to about 12)                                                                                     |
| Modify `next_temp_reg`                 | +5                                                                                                                                                                                             |
| Decompose `generate_call_expr_ir`      | **+40–80** (overhead from the signatures and parameter passing of 6 `emit_*` functions; this is the typical cost of a pure-move decomposition, and **decomposition increases the line count**) |
| Span-keyed consumption counter         | +15                                                                                                                                                                                            |
| Make `method_def_ordinals` read-only   | +20                                                                                                                                                                                            |
| New `synth.rs` file                    | +80–120                                                                                                                                                                                        |
| New `verify.rs` file                   | +400–600 (07’s first layer, in this article’s scope but counted independently)                                                                                                                 |
| New `regalloc.rs` file                 | **+500–1,000** (measured at 271 lines in 7b.4)                                                                                                                                                 |
| **Total**                              | **`ir_gen.rs` grows from approximately 8,448 to 8,600–8,800 lines; including new files, the net increase is 900–1,600 lines**                                                                  |

**The early expectation of “8,448 → 5,000–6,000” was incorrect; measured analysis produced the
opposite result.** The reason must be explained:

1. **The core benefit of this article is not reducing line count; it is eliminating an entire class
   of defects that “fail incorrectly without reporting an error.”** Six manual save/restore
   operations (22 lines) become approximately 12 lines plus a guard type under SSA + RAII—**the line
   count is almost unchanged, but “forgot to restore” changes from something possible into a compile
   error**.
2. **A pure-move decomposition necessarily increases line count.** Decomposing the 746-line
   `generate_call_expr_ir` into 7 functions requires repeating signatures (the function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` binding, and error propagation.
   **“Splitting the file to reduce line count” is a misconception—it reduces cognitive load, not
   physical lines.**
3. **The new verifier and allocator are net line-count increases.** These two additions
   (approximately 900–1,600 lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of line count, this article will most likely produce a **net increase**. The benefit lies
> in **verifiability**—turning defects that “fail incorrectly without reporting an error” into
> invariants that “fail the build when violated.” If line count is used as an acceptance criterion,
> this article will fail; the correct acceptance criterion is the one defined in “Batch Strategy.”

**This stage intentionally increases the amount of code in `ir_gen.rs`.** The 2026-10-03 decision
cancelled all line-count / size gates (see Prohibition Two in [08](08-maintenance-mechanism.md));
therefore, **no line-count exemption list is needed**. The legitimacy of the increase is established
by the separation-of-responsibilities argument: SSA transformation separates “register allocation +
definition discipline” from lowering into independent modules, trading increased code volume for the
elimination of two classes of manual discipline that “misalign without reporting an error and merely
produce wrong values”: six manual save/restore operations and the semantic reordering of `arg_regs`.

## Key Decisions and Rationale

### Three Core Decisions

**Decision One: Temporary-value strategy—delete the three statement-level rollbacks so that unique
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, so every temporary slot naturally has exactly one definition point.
Differences in hand-copied guards—such as `3632` having a `result_reg.is_none()` guard while the
other two do not—disappear automatically. The cost is increased slot occupancy—`temp_high_water`
already has a 255-limit check (`1992-2000`) as a backstop.

**Decision Two: Do not perform mem2reg; perform explicit renaming only.** The rationale was given
under “Value Identification Method”: YaoXiang’s `&` / reference / ownership semantics make “which
slots may be promoted to memory” a type-system problem that requires the converged type
representation from [03](03-type-unification.md). **Implementing mem2reg before 03 is guessing.**
Explicit renaming only requires changing the return-value semantics of one `next_temp_reg` site and
deleting three rollbacks; it does not touch the control flow of any lowering algorithm.

**Decision Three: Use option A for `Phi` (IR-only pseudo-instruction expanded into a Move
sequence).** `executor/` and the `.42` format have **zero changes**, limiting the regression surface
to L3. The decisive reason is that changing the `.42` format triggers version-number issues and a
full compatibility discussion, while option A’s size cost is irrelevant under interpreted execution
(the executor runs Move instructions, not Phi). The cost is that `Phi` is invisible in
`dump_bytecode` (when entering the C1/C2 stages, note that 07’s third layer compares it).

### Rejected Alternatives

**A. Split everything at once (SSA + decomposition + `synth` boundary + allocator all in one
change). Rejected.** This violates 07’s core principle: criteria must be tiered. Mixing four batches
into one PR mixes pure-move changes with C4 changes, making snapshot drift impossible to
attribute—if a snapshot changes, it is unclear whether the decomposition moved something incorrectly
or SSA changed it incorrectly. **Rollback granularity degrades from “change item” to “everything.”**

**B. Split the file first, then optimize ordering. Rejected.** Splitting does not eliminate any root
cause: the six manual save/restore operations remain scattered across 10 files, and `arg_regs`
semantic reordering still requires manual reasoning. Worse, **splitting first loses the opportunity
to “build criteria before changing anything”**—the decomposition creates a large line-level diff
that mixes into subsequent SSA diffs and invalidates review. This has the same root cause as the 8
orphan test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests that have never run):
**if the order is wrong, every later step accelerates on the wrong foundation.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as “a third path that was once considered.”** The shape retains `ir_gen.rs` unchanged and adds a
pass that converts its non-SSA IR into SSA IR. Its **advantage** is that lowering remains untouched
and rollback is easy. **Reasons for rejection**: (1) It does not eliminate the root cause of defect
class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the six save/restore
operations remain separated by 59–161 lines. A conversion pass can merge multiple definitions into
`Phi`, but **it cannot fix “names from the inner function leak into the outer scope”**—that is a
`cur_locals` save/restore error occurring before the IR shape, and invisible to the conversion pass.
(2) It turns SSA into a second representation—the repository already has the cautionary example of 3
parallel type representations, and the project’s core lesson is that **“boundaries exist only in
human discipline.”** (3) `verify_loose` becomes a permanent burden.

**D. Keep the current implementation and only add a verifier. Rejected as the sole solution, but it
is a component of this article.** RFC-039’s Alternative B reaches the same judgment: gates can
prevent regression, but cannot repair the current implementation. **However, what D can do must be
stated clearly**: D can **catch instances** of defect classes 1/2/3 (provided that the verifier
covers dominance, type consistency, and inner isolation), but it cannot catch defect class 4
(`closure_captures` leakage), which requires a cross-function liveness assertion beyond the boundary
of `verify(&ModuleIR)`. **Therefore, D and this article are not alternatives; D is a prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not the risk
  point**—the 2026-10-03 decision cancelled line-count / size gates, and the scale problem is
  resolved through separation of responsibilities (see Prohibition Two in
  [08](08-maintenance-mechanism.md)). The legitimacy of the increase lies in replacing two classes
  of manual discipline that “misalign without reporting an error and merely produce wrong values”:
  six manual save/restore operations and `arg_regs` semantic reordering.
- **The “unique definition” check in `verify.rs` is meaningless in the non-SSA shape**
  (`Operand::Local` allows multiple definitions)—07 already records this limitation, and
  `verify_loose` is its degraded mode before this batch.
- **Deleting statement-level reclamation changes the temporary-slot allocation pattern** (executed
  in 7b.4, together with 7a.1): after reclamation is removed, slots are reused by the linear-scan
  allocator according to live intervals, **so the allocation result changes**—this is precisely why
  C4 must run against the full corpus; measured differentials are empty across 403/403 cases.
- **`u8` / 255 slot limit**: the two corpus files that exceeded the limit after reclamation was
  removed (`string_ops.yx` with 354 monotonic slots and `list_comp.yx` with 323 monotonic slots) are
  compressed by the allocator to **15 / 24 frame slots**; the budget gate now reports E3014 only
  when the function body genuinely does not fit after allocation (including the function-body
  declaration span), and no longer falsely rejects based on the “cumulative temporary count.”
- **Benefits must be recorded as well**: turning “errors that cannot be tested” into “invariants
  that can be tested”; no backend changes (`executor/` and `.42` remain unchanged); no changes to
  `Instruction` semantics (all 76 → 77 variants are retained, and all 50 `translate_*` functions are
  retained); incremental delivery is possible (every item can be reverted independently).

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> item-by-item decisions. **This article leaves no pending items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching design; four-layer model, criterion tiers, P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definitions for
  criterion category C4, `verify_loose` / `verify_ssa`, the three criterion layers, and the
  snapshot’s immunity to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the `PlanId`
  transformation for `ReleasePlan`
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; the counterexample of
  “build the directory first, then fail to wire it up”

### Code Locations

**IR Definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — 7 `Operand` variants
- `src/middle/core/ir.rs:47-533` — 76 `Instruction` variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (includes `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (includes `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (includes `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for the `PlanId`
  approach)
- `src/middle/core/ir.rs:689-719` — Accessors such as `all_instructions`

**Defect Class 1 (Multiple Definitions and Save/Restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for the 7 nested function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — sole table-extraction point + E3014 budget
  point (`MAX_REGISTERS = 255`), combining block splitting / slot allocation / table extraction
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — linear-scan allocator (sole producer of `value_slots` and
  `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamation
  removed in 7b.4; slots are reused by the allocator
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — target of 07’s “jump target exists”
  invariant

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  the clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill in 0” fallback

**Defect Class 3 (Span Keying)** — the following are **audit-time evidence pointers** (after the 4.5
key migration, the `release_plan` / `overload_resolutions` keys use `PlanId`, with current-state
pointers in the revision note in §2 of [02](02-stage-contract.md); `method_def_ordinals` was
eliminated in 7c.3 through mechanism three)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write site of
  `method_def_ordinals` (key is the unqualified `"Type.method"`; **the field was removed in 7c.3**,
  and these locations are audit-time evidence)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the transfer structures
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — definition and
  production of `ReleasePlan`
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer transfer
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (Implicit Order Dependence)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — equivalent handshake in `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — two read sites for `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — 3 other `ast::Type` construction sites

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — 66 `BytecodeInstr` variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded discards (compare `2344` /
  `2347`); old audit records, with review conclusions in “Tracking Hard-Coded Discarded Data” above
  (two genuine losses + one false positive)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  bound of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current Test State**

- `src/middle/core/ir_gen.rs` — 8,448 lines, 0 `#[cfg(test)]`, 0 `mod tests`
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no IR structural
  invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example of per-opcode round-trip criteria; the last test in the file; the file contains 1,209
  lines / 23 `#[test]` functions)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies contain only `criterion` / `proptest`; `[dependencies]`
  contains no regalloc, petgraph, or graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans `/rfc/draft`; this
  article is not in that directory and is referenced by RFC-039 and the index for this directory
