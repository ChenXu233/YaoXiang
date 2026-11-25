# SSA Conversion of the Intermediate Representation

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main RFC-039 text for the four-layer model, acceptance-criterion tiers, and execution-phase
> order; see the [index for this directory](index.md) for the positioning of each companion
> document.

## Positioning and Scope

This document addresses IR-construction discipline at L3 and proposes a construction model based on
“one definition per value” for `src/middle/core/ir_gen.rs` (8,448 lines). **The goal is not to
introduce a new instruction set, but to eliminate three classes of discipline defects that produce
no errors when they fail**:

1. Six hand-written register save/restore pairs separated by dozens of lines;
2. Semantic reordering of shared `arg_regs` across six branches inside `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: changes to IR shape (`Operand` / `Instruction`), temporary-value
allocation discipline, explicit cross-layer contracts, decomposition of call-expression dispatch,
and the wiring of the IR static verifier.

This document is not responsible for: type-representation convergence
([03](03-type-unification.md)), stage contracts and the obligations ledger
([02](02-stage-contract.md)), the definition and enforcement of equivalence criteria
([07](07-equivalence-oracle.md)), frontend paradigms ([05](05-frontend-paradigm.md)), or cleanup of
dead code and placeholder designs ([06](06-cleanup-inventory.md)). This document repeatedly cites
the conclusions of those documents and does not repeat their arguments.

**Acceptance-criterion category** follows **C4 (IR shape changes)** from
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, without requiring full IR snapshot equality. SSA conversion necessarily changes IR
shape; forcing snapshot equality would encourage the team to weaken the criterion, which is
precisely the trap recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, accepted through behavioral equivalence plus a green `verify_ssa`). The
implementation-batch numbers within this document (batches a–d) are distinct from the ordering
within RFC-039 P7: the former are this document’s batching strategy, while the latter are RFC-039’s
global phases.

## Feasibility Premises

**Verified fact.** The IR already has **all structural prerequisites** needed for SSA construction:

| SSA prerequisite             | Current state in this repository                                                                                   | Location                                                               |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                                   | `BasicBlock` in `src/middle/ir/ir.rs`                                  |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                     | `FunctionBody` in `src/middle/ir/ir.rs`                                |
| Scope nesting information    | `LocalSlot::scope_depth` (`"0 = function-parameter layer, used to disambiguate duplicate names in nested scopes"`) | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                                 | `Instruction` in `src/middle/ir/ir.rs`                                 |
| Per-instruction span         | Every variant has a `span: Span` field                                                                             | Same as above                                                          |
| Annotatable value types      | `LocalSlot::ty: MonoType`                                                                                          | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Slot-count upper-bound check | Single point for table extraction and the E3014 budget gate (`MAX_REGISTERS = 255`)                                | `finalize_code_body` in `src/middle/lower/scope.rs` (from 7b.4 onward) |

**If these prerequisites were absent, the cost estimates in this document would be entirely
different.**

**The two decisive points are the following:**

**First, SSA conversion is not a new capability; it removes a dependency on one.** Temporary slots
are currently **deliberately designed to have multiple definitions**—`generate_function_ir` and
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`, explicitly roll `next_temp` back in three places. The first step
of SSA conversion is to **remove these three rollbacks**, allowing `next_temp` to increase
monotonically. Once they are removed, **every temporary slot naturally has exactly one definition
point**—unique definition changes from “a rule that must be checked” into “a mathematical
consequence of the construction method.” Batch a only consolidated the save/restore surface (see
“Defect Class 1”); the three rollbacks were removed together with 7b.4 in batch b (allocator
takeover of slot reuse) on 2026-11-22.

**Second, the slot-count upper bound is a hard check, and removing the rollbacks briefly caused slot
counts to become unmanageable (resolved in 7b.4).** The budget gate in `src/middle/lower/scope.rs`
reports E3014 when the count exceeds 255. In practice, after removing the three reclamations and
running the full corpus, two corpus cases crossed the limit and triggered
E3014—`src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; the peak slot count for all
other corpus cases rose from 51 to 211. **After 7b.4 landed:** the linear-scan allocator reuses
slots by live interval, and the two corpus cases now use **15 / 24 frame slots** (344 / 304 values),
while the remaining 403/403 corpus cases remain behaviorally equivalent. From this point onward,
E3014 means only “frame slots after allocation exceed 255.”

**The combined meaning of these two points is:** lowering already maintains all the bookkeeping
information needed for SSA, but merely **reclaims** it; 7b.4 makes the allocator responsible for
slot reuse and removes the reclamation. Most of the SSA-conversion effort therefore lies in
decomposing `generate_call_expr_ir` and making cross-layer contracts explicit, not in building IR
expressive power.

### Prerequisite: Unification of Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. A third representation on the serialization side is `ir::Type`,
bridged through `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR contains two type representations simultaneously, and one of them directly
`pub use`s the AST type.**

SSA conversion must add a `Phi` variant, record a type for every SSA value (to support the
verifier’s “type consistency” invariant from 07), and determine whether the types of two
predecessors at a CFG merge point are compatible. **If type representations have not yet been
unified, SSA will require one type-compatibility implementation for each representation**—and the
third representation will grow accordingly.

This is not a question of which task should come first; it is a question of **which approach would
produce three separate type-compatibility implementations**.
[03 Type-Representation Unification](03-type-unification.md) must first complete the convergence of
`pub use ast::Type` at `ir.rs:3` and the removal of the bridge at `bytecode.rs:2353`. Item 6 in this
document’s change list explicitly records that `ir.rs:3` is **not changed by this document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not merely an “ugly but working” file. It is a file that **places correctness in the
hands of line-by-line human review**:

- The file contains **zero `#[cfg(test)]` blocks and zero `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares the `bytecode`, `def_assign`, and `local_slots` modules,
  with 29 tests in total. `def_assign.rs` and `local_slots.rs` provide **positive** coverage of
  ir_gen’s DefId allocation and local-slot naming (at sentinel level), while all four references to
  ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **negative assertions**—“ir_gen does
  not emit,” “ir_gen also does not construct,” or “the ir_gen frontend does not construct the
  corresponding IR.”

In other words, ir_gen has 29 sentinel-level tests pinning local behavior, but **has no IR
structural-invariant validation** (dominance, unique definition, jump targets, type
consistency)—none of the three modules declared in `middle/core/tests/mod.rs` makes positive
assertions about the overall shape of the IR.

This file also happens to contain several classes of defects that **compile successfully, pass all
checks, and run correctly, but merely produce wrong values**. Their common property is: **failure
produces no error**.

This is where the highest risk lies. The problem is not that “the function is too long,” nor that
“the naming is confusing,” but that **the core invariants of this file have no machine-executable
enforcement mechanism**.

The contract “save before generating a nested function body, restore afterward; otherwise, inner
function names leak into the outer scope” is enforced by the type system: `NestedBodyGuard` in
`src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator, and frame removal and
restoration each have only one implementation (see “Defect Class 1”).

**Why adding a verifier is not sufficient**: a verifier can **detect** these defects, but it cannot
detect their **causes**. The cause is not that “a reviewer missed something,” but that:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator deliberately depends on this property.**

Concrete evidence consists of the three statement-level register reclamations
(`generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`)—at the end of every statement, they
**roll back** `next_temp` to the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and the number and
order of writes, along with live intervals, are determined entirely by one manual decision: **when
to roll back**.

Under this model, “unique definition” is not a rule that can be applied incrementally; it **directly
conflicts with the register-allocation strategy**. SSA conversion is therefore not a matter of
adding a check to existing code; it means **replacing the allocation strategy that depends on
multiple definitions**. The good news is that the replacement cost is low—see “Feasibility Premises”
above.

### Defect Class 1: Multiple Definitions and Manual save/restore (the Three Reclamations Were Removed in 7b.4)

**Verified fact.** There are seven entry points that require saving/restoring state before entering
a nested function body. Batch a (7a.2) consolidated them into one RAII guard: `NestedBodyGuard<'a>`
in `src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, unconditional restoration in
`Drop`, and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Rollback scope                                         |
| ------------------------------------ | ---------------------------------- | ----------------------------- | ------------------------------------------------------ |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (three counters are not rolled back) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                          |
| Nested function-definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                     |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                          |

- Frame removal occurs only in the two constructors (`enter` / `enter_decl`), and restoration occurs
  only in `Drop`; the guard exclusively holds `&mut` access to the generator for its lifetime, so no
  second `&mut` is available in the scope, and all `?`/early-return paths restore through `Drop`.
- The affected fields are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`; from 7b.4 onward,
  `next_temp_reg` maintains only the named-variable high-water mark (temporary-value slots belong to
  the linear-scan allocator; `temp_floor` was removed with the three reclamations).
- `enter_decl` does not roll back the three counters: after a function body or method body returns
  to the module-initialization flow, subsequent definitions continue from the child body’s final
  value number. This is the constraint required for C1 snapshot zero drift and the meaning of “the
  declaration surface rolls back only the table.”

**Unified Semantics and a Single Budget Point (from 7b.4 onward):** `finalize_code_body` in
`src/middle/lower/scope.rs` is the sole table-extraction and budget point—it extracts the table
using `frame_slots = named_base + max_live`, reports E3014 with the function-body span if the limit
is exceeded, and defines `MAX_REGISTERS = 255` in exactly one place in the repository. All six
function-body forms (function / method / lambda / anonymous binding / intermediate curry layer /
innermost curry layer) are finalized through it, so there is no second definition of “how many slots
a function body should have.”

**The three statement-level reclamations have been removed (implemented together in 7a.1/7b.4 on
2026-11-22):** the reclamations were hand-copied in `generate_function_ir` / `generate_block_ir`
(with a `result_reg.is_none()` guard) / `generate_lambda_body_ir`; after the allocator took over
slot reuse, the reclamations degenerated into identity operations, so the guards, the `temp_floor` /
`saved_temp_floor` bookkeeping, and the loop-entry/loop-exit bookkeeping in `loops.rs` were removed
as well. Module init and top-level statements still use the old monotonic-slot path with
`value_mode=false` and do not run the allocator; their slot numbers and table lengths shift with the
number of named slots in function bodies (three snapshots measured; behavioral equivalence is
covered by 403/403).

**Consequences (current form):** the guard exclusively holds `&mut`, restoration has only one
implementation in `Drop`, and there is no writable location where a restore could be omitted; the
temporary-slot dependency on “one slot with multiple definitions” has disappeared with the
reclamations—value slots are now reused by the linear-scan allocator according to live intervals,
giving `verify_ssa`’s unique-definition criterion a construction-side foundation.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified fact (audit-time wording; the current location has been synchronized with the batch-c
decomposition from 7c).** `generate_call_expr_ir` is now located in `src/middle/lower/expr/call.rs`
(split out of `src/middle/core/ir_gen.rs`, which no longer exists) and is a single function. Its
signature accepts `func: &Expr` / `args: &[Expr]` / `named_args: &[(String, Expr)]` / `span: &Span`
/ `call_plan_id: PlanId` / `result_reg: Operand` / `instructions` / `constants` (`call_plan_id` is
the decision-consumption key from RFC-039 D20). Multiple internal branches share the same
`Vec<Operand>` and perform **semantic rewriting** (the line numbers below are from the audit-time
version; after decomposition, locate the relevant symbols):

| Line        | Semantics                                                                                                                                                      |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten `slots`, then recollect them—named arguments are reordered                                                                             |
| `7724`      | Struct construction: unwrap each slot in `final_args`, then recollect them—fields are reordered                                                                |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **fills missing slots with a new register whose value is 0** (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—clone before emitting instructions                                                                          |

The original source of the four reorderings is:

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

The comment in `7835-7848` states: “Uncovered slots: keep them absent so that the type-checking
stage reports E1010. For runtime safety, fill with 0 (the normal path never reaches here).”

**Additional facts about named arguments (from 4.9.9 onward):** before reordering, named arguments
are bound according to the **declaration-time formal-parameter name table**—through the shared
criterion `arg_binding::bind_args_to_params` (the IR-side name table is obtained through
`lookup_param_info` / `method_param_info`, whose sources are this unit’s `fn_param_names` and
cross-unit method candidate groups). If the name table cannot be obtained and the call is not within
an existing exemption category (closure value binding / top-level function binding in this unit /
named nested-function declaration), E1014 is reported immediately (the empty-table exit), and silent
reordering no longer occurs. The “fill with 0” behavior in the table above remains a runtime safety
net for positional arguments and fallback branches; it is unreachable on the normal path.

**This design carries two risks**:

1. **Misalignment produces no error, only wrong values.** Each of the six branches independently
   determines the final order of `arg_regs`, and an incorrect ordering assumption in any one branch
   will not cause a panic.
2. **Normalized snapshots are immune to this defect.** Document 07 records this limitation: after
   the snapshot tool renames temporary values by “first-occurrence order,” “the third argument
   should have used the fifth register” is identical in the snapshot. **This class of defect does
   not even have a snapshot-level criterion in phase C4.**

**Additional verification: the struct-construction path’s reordering has no duplicate-slot guard.**
The `final_args` at `7724` are populated by “placing positional arguments first, then overwriting by
field name for named arguments,” and **there is no duplicate-slot check such as
`slots[idx].is_some()` before overwriting** (unlike the namespace path at `7381`)—when a named
argument silently replaces a positional argument, there is neither a panic nor a diagnostic.
Furthermore, the arguments’ **evaluation order** (source order) diverges from the final `arg_regs`
order (field-declaration order) on this path. Batch d’s assertion-based `fill_missing` must also
cover the “duplicate slot” check; otherwise, silent replacement in the struct-construction path will
survive unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated** on 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified fact (at audit time).** The repository has **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                                 | Production                                                                                                               | Propagation                                                                           | Consumption                                                                                                                      |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                                   | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatches `walk_stmt` by statement identity)      | `typecheck/types.rs:38` ← passed by `checker.rs` ← read and cloned at `ir_gen.rs:360` | `ir_gen.rs:3103` (`ReleasePlan::consume`, the sole entry point; settlement-completion marker `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger in the same structure, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` merely forwards and merges it) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held in an `ir_gen.rs` field    | `ir_gen.rs:7697` (with a binding), `7772` (without a binding) (`OverloadResolutions::get`; settlement-completion marker `:1097`) |

The audit-time form (`ir_gen.rs:1942`, **now nonexistent**) was:

```rust
// NLL Release: insert a Drop instruction at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some` means that **a failed lookup simply produces no result: there is no else branch,
assertion, or counter**. If the two sides compute spans differently in any respect, the **Drop
instruction silently disappears, with zero errors**. The Drop sequence for refinement types is a
core carrier of this project’s ownership semantics (`ownership.rs:26-32` describes the “precise NLL
release plan”), yet its lowering was once silent.

The same pattern at `7420` / `7493` (`if let Some(mangled) = self.overload_resolutions.get(span)`)
caused overload resolution to fall back to the default binding.

**Disposition (implemented in 4.5):** both keys were changed to the structural identity `PlanId`,
and consumption is **accounted for on a hit** (reading and accounting occur in the same call). A
nonempty difference is reported at the sole settlement point as an **E-grade `W1090`**—“not found”
is no longer silent. The implemented form and criterion are documented in §2 of
[02](02-stage-contract.md) (revision note for 4.5); these line numbers (`1942` / `7420` / `7493`)
are retained as audit-time evidence and are not pointers to the current implementation.

> **`method_def_ordinals` is not span-keyed (**eliminated in 7c.3**).** It was originally a
> `HashMap<String, usize>`, assigning `#N` suffixes according to the traversal order of definitions
> with `base_name`. Its risk was **different**: definition-order dependence—if generation order
> differed from typecheck registration order, the same function name would resolve to a different
> target. See mechanism three under “Handling of Span-Keyed Problems” for the implemented form: the
> definition name is produced once by typecheck based on declaration identity, and lowering only
> consumes it.

### Defect Class 4: Implicit spawn/lambda Handshake (Eliminated in 7c.1)

**Eliminated.** spawn and spawn-for previously required the caller to write `pending_env_vars` /
`pending_env_names` before a synthesized Lambda consumed them; closure bodies separately relied on
`closure_captures` to determine whether a variable read an upvalue. These three fields placed
correctness in the hands of call order and manual cleanup: a nested synthesized closure could
mistake an outer local slot with the same name for a captured value, and an early return from
closure-body generation could leak capture state into subsequent unrelated lowering.

7c.1 consolidated synthesized AST into `middle/lower/synth.rs`, replacing the three shared fields
with an explicit `SynthEnv` and registering named captures as lexical upvalues. Nested-frame exit
restores state according to symbol-table depth, so error paths no longer leave capture state behind.
See the “`synth.rs` Boundary” section for the implemented structure.

### What SSA Will Not Solve (Listed Honestly)

| Problem                                                               | Location                                                                                                                               | Why SSA does not solve it                                                                                                                                                                                                                    |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                                          | `ir_gen.rs:5390-5687`, **298 lines**, four levels of direct recursion (`5473` / `5504` / `5619` / `5640`), external call site `5270`   | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **intrinsic to the algorithm**. SSA merely turns guards’ `JmpIfNot` instructions into `Phi`; it does not reduce the number of guards |
| `eval_const_expr` complexity                                          | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming direct recursion)              | Constant-folding expression coverage is unrelated to SSA                                                                                                                                                                                     |
| Span-keyed failure (**eliminated**)                                   | At audit time: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)—after 4.5, `PlanId` keys + mandatory settlement       | **SSA does not touch this layer at all** (the conclusion remains unchanged). This was independently resolved by the obligations-ledger mechanism in [02](02-stage-contract.md); see mechanism two under “Handling of Span-Keyed Problems”    |
| Number of `Instruction` variants                                      | 75 (`Instruction` enum in `ir.rs`; includes `Phi` introduced in 7b.1a)                                                                 | SSA adds `Phi`, for a **net change of +1** (implemented). Variants correspond to bytecode opcodes or degrade to NOP; they do not decrease because of SSA. Under scheme A, `Phi` is an IR-only pseudo-instruction and adds no opcode          |
| Definition-order dependence in `method_def_ordinals` (**eliminated**) | At audit time: `ir_gen.rs:224` / `369` / `1692-1702`; implemented in 7c.3, see mechanism three under “Handling of Span-Keyed Problems” | Similar to span keying but with a different cause; definition names are now produced by typecheck based on declaration identity, and lowering only consumes them                                                                             |

## Target Design

### Definition of the SSA Form

**Design judgment.** The target form is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has five variants. **Repository-wide measurements** (hit counts
obtained by summing `git grep -c` per file, across all of `src/` including tests; constructor counts
refer to value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                           | Disposition                                                                                             |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                                   | **Retained**, unchanged                                                                                 |
| `Value(ValueId)`    | 7 files / 26 hits; introduced in 7b.1a with **zero production construction sites**, and from 7b.2 onward `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`)             | **Retained** (introduced in 7b.1a)                                                                      |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                                   | **Retained, but semantics split** (see below)                                                           |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                     | **Retained**, as the implicit definitions at function entry (parameters are the definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                                    | **Retained**, unchanged                                                                                 |
| `Temp(usize)`       | **Zero construction sites** (the only handling arms are in `codegen/operand.rs`’s `OperandResolver::to_reg` and `is_register`, and were removed with the variant)                                     | **Removed** (dead variant, 7b.1a)                                                                       |
| `Label(usize)`      | **Zero construction sites**                                                                                                                                                                           | **Removed** (dead variant, 7b.1a)                                                                       |
| `Register(u8)`      | **Zero construction sites** (the original variant declaration’s comment said “Added for codegen,” but codegen uses the `OperandResolver` in `codegen/operand.rs` and does not construct this variant) | **Removed** (dead variant, 7b.1a)                                                                       |

`Local(usize)` is split into two variants because it currently **simultaneously serves two mutually
exclusive semantics**—“storage location for a named variable” (which may be written multiple times)
and “compiler temporary register” (which should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within a function, uniquely defined
    Value(ValueId),
    /// Storage slot for a named local variable: addressable, multiple definitions allowed (non-SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing and never rolled back**. The split
and `ValueId` were implemented in 7b.1a (`Operand` enum in `src/middle/ir/ir.rs`); the production
side (`next_temp_reg` changed to produce `Operand::Value`) was implemented in 7b.2 (see “Value
Identification Method” below for the production form).

#### `Phi` as a New `Instruction` Variant

```rust
/// Merge point: each predecessor basic block provides one definition of this value
/// incoming is sorted by predecessor block label in ascending order (guaranteed at construction for normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (see the first layer of [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set of `incoming` == the set of labels of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. The `MonoType` of all `incoming` values is identical (**dependent on the type representations
   from 03 having been unified**).

**Implemented status of `verify_ssa` (the first three constraints were completed in 7b.1b):**
constraints 1 and 2 require the value table and block-level CFG and were implemented in **7b.1b** as
separate “block-level predecessor count” and “predecessor label set” criteria—the predecessor set is
derived from `BasicBlock.successors` (`block_predecessors` in `src/middle/ir/verify.rs`); a count
mismatch reports only the count, while a set mismatch reports the set, so the same defect is not
reported twice. The per-input type-consistency constraint was implemented concurrently, with values
obtained from the value table at `values[v].ty`, **reusing the existing `types_compatible`** (no
third type-compatibility implementation was introduced; unification of the type representations from
03 remains the “hard prerequisite · prerequisite two,” and `ValueInfo::ty` will still be `MonoType`
by then). All three criteria use the violation category `InvariantKind::PhiConsistency`.

**Three forms in which the type is unavailable (explicitly registered, never silently permitted):**
an empty value table means that no basis for judgment exists (the invariant form of module init /
top-level statements on the `value_mode=false` path); a nonempty value table with an out-of-range
`ValueId` is a violation; a non-`Value` operand (`Const` / `Local` / `Arg` / `Global`) has no
value-table type source and is not judged.

**Verified prerequisites (fulfilled in 7b.3):** before 7b.3, every repository-wide production
construction of `BasicBlock.successors` used `Vec::new()` (block-level predecessor sets were always
empty, and the predecessor-count and predecessor-set criteria were then exercised by synthetic
fixtures). **From 7b.3 onward, block-level `successors` are populated for real by `partition_blocks`
in `src/middle/ir/ir.rs` at every production construction site** (six in `lower` plus one synthetic
function-body constructor in codegen all go through this sole entry point; lowering products include
multi-block loops and branches, double edges, and back edges), so the predecessor-count and
predecessor-set criteria have data from real IRs—direct-drive tests on real lowering products pin
down this registration-to-consumption closed loop. The number of `Phi` production construction sites
remains zero (production belongs to a later batch); when implemented, it will be directly subject to
these criteria.

7b.1a introduced only the variant: there were zero lowering production construction sites (and there
remain zero; production belongs to a later batch). The consumer in `translator.rs` was implemented
in **7b.3**: under scheme A, it expands each `Phi` into a chain of `Move` instructions at block
entry (each group lands at the merge block’s entry, existing `JMP`s between groups skip over them,
and incoming-edge jumps are redirected to the group start according to the predecessor). The `Phi`
arm of `translate_instruction` is an unreachable E8001 defensive arm (`executor` never sees `Phi`).

**Disposition of the guard issue at `3632-3638`:** the reclamation in the current
`generate_block_ir` has an `if result_reg.is_none()` guard (the `3626-3631` comment: a block for
expression operands may have sibling argument temporaries that remain live across blocks). After SSA
conversion, this guard is **no longer needed**—temporary values are no longer rolled back, so
sibling arguments that live across blocks retain their own `ValueId` and are unaffected by
rollbacks. **This is a place where SSA conversion can genuinely simplify the code, rather than
merely renaming things.**

#### Value Identification Method

**Design judgment:** do not perform memory promotion (mem2reg), do not use virtual-register numbers,
and **perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots may be memory-promoted” a
  **type-system problem**, not a CFG problem. It has been verified that `ir.rs`’s
  `LocalSlot::ty: MonoType` carries complete type information, but determining “whether this
  `MonoType` can be memory-promoted” requires the unified type representation from 03 and
  cooperation with refinement-type rules. **Performing mem2reg before 03 is guessing.**
- Explicit renaming only requires changing the return-value semantics of one function
  (`next_temp_reg()` in `src/middle/lower/scope.rs`) and removing the three rollbacks; **it does not
  touch the control flow of any lowering algorithm**.

Value-table form (**implemented in 7b.1b**): `FunctionBody::Code` gains `values: Vec<ValueInfo>`,
where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (inherited from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block in which the value is defined
}
```

The accessors have the same shape as existing accessors (`impl FunctionIR` in
`src/middle/ir/ir.rs`): `values()` is read-only and returns an empty table for TypeDecl (mirroring
`locals()`); `values_mut()` is mutable and panics on TypeDecl (mirroring `blocks_mut()`).

**Production form (implemented in 7b.2):** `next_temp_reg()` returns `Operand::Value(ValueId)` and
registers one `ValueInfo` in the current function body’s value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). The implemented form is as follows:

- **Counter isolation:** `ValueId`s are issued by an independent `next_value: u32` counter, starting
  from 0 for each function body, increasing monotonically, and never being rolled back;
  `NestedBodyGuard` saves and restores it together with the value table (one set per function body);
  the three statement-level reclamations roll back only `next_temp` (slots) and do not touch
  `next_value`; curry layers reset it layer by layer (one `FunctionIR` per layer). Module init /
  top-level statements have no value table—when `value_mode=false`, `next_temp_reg` degrades to the
  old `Local` slot allocation, leaving the init sequence unchanged.
- **Type source** (uses only information already available to lowering; no new type adjudicator is
  introduced): literals use `ConstValue::mono_type()` (the constant-type mapping is unified with the
  verify mapping as a single source); variables/calls use the type-check result and bound function
  return type at the `generate_expr_ir` entry point;
  comparison/logical/string-family/forward-propagation forms are filled in at function-body
  finalization according to instruction semantics. When unavailable, the type is an **explicit
  unknown `MonoType::TypeRef("?")`** (no silent `Int(64)` placeholder; `LocalSlot.ty` is never
  read). Across 232 snapshots, 9,425 values were measured: 7,080 had real types and 2,345 had
  explicit unknown types (24.9%), concentrated in call/field/index/container-construction forms.
- **`defining_block`:** takes the current block label during generation; **block partitioning was
  implemented in 7b.3** (a real lowering `while` body consists of multiple blocks), but the field is
  still taken from the generation-time `cur_block_label` (currently always 0), so it does not yet
  match the actual defining block. When 7b.4 regenerated the 232 snapshots, this field was not
  changed (the `@block` section under `values` did not drift). Its correction must occur in the same
  batch as regeneration of the `values` section and is **explicitly handed over to the `Phi`
  production batch**.
- **`debug_name`:** always `None` (named variables remain `Local` and are not migrated to the value
  table).
- **Indexing:** `value_slots: Vec<usize>` (index = ValueId) has the same indexing as `values` and
  records the value-to-frame-slot mapping; from 7b.4 onward, it is recomputed by the linear-scan
  allocator in `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`), and codegen’s `OperandResolver` only consumes it rather
  than allocating.

`values` and `locals` coexist: `locals` carries addressable named variables (which may have multiple
definitions), while `values` carries single-definition SSA values. **The existence of `locals` does
not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Normalization with `LocalSlot::origin`:** the two are **different dimensions** and must not be
mixed—`LocalSlot::origin` (the **slot-identity** field introduced on the origin side in 2.1.4,
recording the `FrameId` of the frame that owns the slot) answers “which frame does this slot belong
to,” whereas `ValueInfo.defining_block` answers “in which basic block is this SSA value defined?”
Value identity belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`; 7b.1b only
adds the value table and **does not change `LocalSlot::origin`** (measured as zero changes).

#### Change-Volume Estimate

| Item                                              | Estimate                          | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------- | --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` modification                      | ~15 lines                         | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Removal of three statement-level reclamations     | −15 lines                         | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Consolidation of six save/restore pairs into RAII | Net ±0 lines                      | See “Item-by-Item Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `ir.rs` structural changes                        | +120–180 lines                    | `ValueId` / `Phi` / `ValueInfo` / value table; **7b.1a implemented the first two and 7b.1b the latter two** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `Phi` arm in `translator.rs`                      | +25–40 lines (for 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `executor/`                                       | **Zero changes**                  | Scheme A: `Phi` is an IR-only pseudo-instruction expanded into a `Move` chain in `translator.rs`, and never enters bytecode (decision three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Call sites changed to produce `Operand::Value`    | Mechanical replacement (for 7b.2) | **Measured** (`git grep -c` summed per file, across all of `src/` including tests): `Operand::Local(` occurs 415 times in 28 files, including 332 times in 14 files under `src/middle/lower/`; the single production point for temporary values is `next_temp_reg`. **7b.2 measurement (retested against the baseline revision, net implemented amount):** `next_temp_reg()` call sites: 140 in 14 files; `result_reg: usize` parameter chains: 35 functions plus 3 `Option<usize>` result slots; `Operand::Local(` under `src/middle/lower/`: 342 occurrences. By pen, changes under `src/` total +23,319/−12,495, of which regeneration of 232 IR snapshots accounts for +21,866/−11,950; source and tests net +1,453/−545 (mechanical replacements and normalize rendering, mono projection, six direct-drive tests; also includes five error-code artifacts checked in with the changes) |

### Item-by-Item Disposition Table

| Defect class                                                | Eliminated by SSA?            | Disposition                                                                                                                                                                                                                                                                                                                                                                                                          | Criterion                                                                                                         |
| ----------------------------------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore**         | **Yes (resolved in 7b.4)**    | The seven nested-function-body entry points are consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the counters), turning “forgot to restore” into a compile error; the three statement-level reclamations were removed with 7b.4 (the allocator reuses slots by live interval), and temporary values no longer occupy `next_temp` | First-layer “inner isolation” invariant from 07 + `verify_loose`/`verify_ssa`                                     |
| **1b. Inconsistent semantics**                              | **Yes (implemented)**         | Unified in `finalize_code_body` in `src/middle/lower/scope.rs`: the table is extracted using `frame_slots`, and the E3014 budget check occurs at the same point with the function-body span; all six function-body forms are covered; `MAX_REGISTERS` is defined in exactly one place in the repository                                                                                                              | Corpus differential + snapshot zero drift                                                                         |
| **1c. Criteria for the three statement-level reclamations** | **Yes (implemented in 7b.4)** | The three reclamations, the `result_reg.is_none()` guard in `generate_block_ir`, and the `temp_floor` bookkeeping were removed; slot reuse belongs to the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                                            | C4: behavioral equivalence (403/403) + invariants                                                                 |
| **2. `arg_regs` semantic reordering**                       | **Partial**                   | SSA turns “using the wrong register” into “using the wrong value ID,” which dominance checking can detect, but **the reordering logic itself remains**. Actual elimination depends on function decomposition—each of the six branches performs `push`, while ownership of `arg_regs` remains at the top of the function                                                                                              | First-layer dominance invariant (snapshots are immune)                                                            |
| **3. Span-keyed failure**                                   | **No**                        | SSA does not touch it at all. See “Handling of Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                                  | `test_release_plan_spans_consumed` from 07                                                                        |
| **4. Implicit ordering dependency**                         | **No**                        | See “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                                            | Guaranteed by code structure and review; the original gate script was canceled by the user’s 2026-11-24 ruling 丁 |
| `compile_pattern`, 298 lines                                | No                            | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                        | —                                                                                                                 |
| `eval_const_expr`, 191 lines                                | No                            | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                        | —                                                                                                                 |
| 75 `Instruction` variants                                   | No (+1, implemented)          | `Phi` was introduced in 7b.1a; under scheme A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                                 | —                                                                                                                 |

### Register Allocator: Custom Implementation or Existing Crate?

**Verified facts (decisive):**

1. `Operand::Register(u8)` has **zero construction sites** in the repository—the variant is no
   longer in the `Operand` enum in `ir.rs` (removed in 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper limit of 255 (exceeding it
   reports E3014); from 7b.4 onward, `Operand::Value` is resolved by the allocator’s `value_slots`
   output into a frame slot (`>255` reports E3014; no mapping reports E3017 with the instruction
   span).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50
   `translate_*` functions, with function bodies distributed across `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`, where opcodes come from
   `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` contains the interpreter’s opcode-dispatch family.

**Conclusion:** the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Evaluation of the three approaches:**

| Approach                                           | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both take rustc’s `Function` trait plus MIR’s `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait set (CFG, liveness, spill slots, scratch space): **500+ lines of adapter code for zero benefit**, because there is no real register file to color. Moreover, rustc itself uses only linear scan at this stage when `enable-llvm` is disabled                                                                                                                         |
| `graph-alloc` family                               | **Not adopted.** Same reason, with an even less stable API                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| **Custom linear scan (simplified Chaitin-Briggs)** | **Adopted (implemented in 7b.4).** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot according to its live interval (`OperandResolver` already has the `u8` contract and E3014 limit). Implement `src/middle/lower/regalloc.rs`: block-level `successors` backward dataflow, closed intervals, `Phi` predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`, and an out-of-range result returns `Budget`, which `finalize_code_body` reports as E3014 with the declaration span |

**Implementation measurements (7b.4):** the two corpus cases that previously exceeded the limit (354
/ 323 monotonic slots) now use **15 / 24 frame slots** after allocation (344 / 304 values), while
the remaining 403/403 corpus cases are behaviorally equivalent. From this point onward, E3014 means
only “frame slots after allocation exceed 255.” The counter used for the back-edge fallback is
currently not exposed (`wrap_count` is computed and discarded); the `Phi` production batch must
expose and retest it first (alongside `defining_block` in the prerequisite list). **Overflow
handling:** `u8`/255 is still handled as an error by the budget gate—this document does not solve “a
function has too few registers” (a downstream consequence of `compile_pattern` recursion depth).

The dev-dependencies at `Cargo.toml:37-39` contain only `criterion` / `proptest`; `[dependencies]`
contains no regalloc / petgraph / graphlib. Introducing any such crate would add a new dependency
surface.

### Decomposition Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA form and criteria must exist first**. The reason is that
decomposition itself is C1 (a pure move, judged by snapshot zero-diff), but only after `arg_regs`
semantics have been reduced by SSA to “one independent `ValueId` per argument” can decomposition
avoid carrying the old ordering assumptions into the new file unchanged.

**Core design constraint: ownership of `arg_regs` remains at the top of the function; the six
branches only `push` and never reorder.**

```
generate_call_expr_ir (746 行)
├── 参数求值：按源码顺序求值，push 到 arg_regs —— 6 个分支共用，唯一允许 append 的地方
├── CallArgs（值对象）
│   └── arg_regs: Vec<Operand>  —— 只在顶部创建，6 个分支只 push，绝不重新 collect/reorder
├── 分支 1 → emit_namespace_call(args: CallArgs)         [7395 分支抽出]
├── 分支 2 → emit_bound_call(args: CallArgs)              [7406+ 分支抽出]
├── 分支 3 → emit_struct_ctor(args: CallArgs)             [7724 分支抽出]
├── 分支 4 → emit_plain_call(args: CallArgs)              [7835-7848 分支抽出]
├── 分支 5 → emit_curried_call(args: CallArgs)
└── 分支 6 → emit_method_call(args: CallArgs)             [7420 / 7493 分支抽出]
```

The “fill with 0” logic in `7835-7848` (`7838-7846`) is moved down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the
self-description stating that “the normal path never reaches here” is replaced with an assertion**:
if any slot is missing and typecheck has not reported E1010, a diagnostic is returned directly.
**This turns a silent fallback into a hard error and is this document’s most direct improvement for
Defect Class 2.**

**Criterion:** the pure-move portion (where the SSA form is unchanged) uses snapshot zero-diff;
assertion-based `fill_missing` belongs to C4 and uses corpus differential testing plus the
first-layer verifier.

### `synth.rs` Boundary: Turning an Implicit Callback into an Explicit Synthesized-AST Boundary

**Implemented form:** `src/middle/lower/synth.rs` is the only module in L3/L4 production code
permitted to construct `ast::Expr` / `ast::Stmt` / `ast::Type`. Pattern matching, read-only queries,
and test fixtures do not constitute synthesis. The current production synthesis points are spawn’s
Lambda/Stmt and spawn-for’s Lambda, all of which go through this module.

AST nodes do not carry a lowering environment, so `SynthEnv` is not an AST field, and AST helpers do
not pretend to consume it. It contains runtime-environment operands and capture names, and the
caller passes it explicitly to `generate_expr_ir_with_env` alongside the synthesized node:

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

The environment flows as follows:

- spawn and spawn-for construct a `SynthEnv` and pass it explicitly into lowering alongside the
  synthesized Lambda; they no longer pre-write `pending_env_vars` / `pending_env_names`. The total
  environment slot count is derived from `env_vars.len()`, and named upvalues are offset from the
  length of the anonymous prefix; consequently, spawn-for’s anonymous iteration value, outer
  captures, and parameters each occupy their proper positions.
- Ordinary lambdas complete their environment from the existing free-variable collector. List
  comprehensions, nested Lambdas, and nested spawns traverse the same collector, while generator
  variables and nested parameters are excluded according to lexical scope.
- Closure bodies register `SynthEnv` as `BindingLocation::Upvalue` in the existing symbol table.
  Variable reads and namespace-shadowing queries share this lexical binding and no longer read
  `closure_captures`.
- A nested closure first reads the outer upvalue through `capture_operand`, then passes the value to
  the inner environment; an outer local slot with the same name is never mistaken for a capture.
- `NestedBodyGuard` records the symbol-table depth on entry and truncates back to that depth on
  exit, including early returns through `?`; capture bindings do not leak into subsequent unrelated
  lowering.
- Sema’s expression-body function checking parameterizes the existing inputs with
  `body_stmts + expression_body + span`, directly calls `check_expr`, and reuses the tail validation
  rather than synthesizing a Block/Stmt/Expr; there is no reverse dependency from sema to
  middle/lower or additional architectural exception.
- SpawnFromList shares the `env + [anonymous item]` argument layout between Embedded and Standard
  and injects upvalues from the same environment. The anonymous iteration value is fixed at
  `env[0]`, named captures are read after the anonymous prefix, and parameters begin after the
  complete environment.
- Dual-runtime behavior tests explicitly call `main()` and, within a spawn-for task, separately
  assert the no-capture item value and the item-plus-outer-capture value; the no-capture form also
  covers the regression restoration introduced by 74febd1f. IR assertions and actual execution are
  recorded separately, and zero drift across the 403 existing corpus cases is not presented as
  behavioral coverage of this form.

**Criterion:** the D22 AST-synthesis boundary is guaranteed by code structure—L3/L4 production code
constructs `ast::Expr` / `ast::Stmt` / `ast::Type` only in `src/middle/lower/synth.rs`; pattern
matching, read-only queries, and test fixtures are exempt; synthesis sites outside `synth.rs` are
rejected during review. The original `scripts/ci/check-synth-boundary.py` and the CI
`synth-boundary` job were removed under the user’s 2026-11-24 ruling 丁 and no replacement scanner
is maintained: a heuristic token scan cannot cover arbitrary legal use / alias / re-export forms
(even after successive fixes, it remained bypassable through a `pub type` re-export), and its
maintenance cost exceeds its benefit.

D1/D2: `SynthEnv` and `BindingLocation` are both new internal types introduced in this batch, not
reused existing types. The former centralizes the producer-side environment operands, named items,
and slot derivation into one explicit parameter; the latter promotes the original
`SymbolEntry.local_idx` to a distinguishable Local/Upvalue location. The alternatives—continuing to
pass three shared fields (which cannot prevent nested overwrites and erroneous residual state), or
passing only `Vec<Operand>` (which cannot express the correspondence between the anonymous prefix
and named captures)—were rejected for correctness reasons. The new concepts are not synonymous with
existing AST or consumption-ledger concepts, and require neither import aliases nor conversion
tables.

### Handling of Span-Keyed Problems (the Part SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism one: consumption-coverage assertion (mandatory, lowest cost; superseded by a stronger
form on 2026-10-13).** Document 07 defines `test_release_plan_spans_consumed` (now settled by
`PlanId`). This document’s original supplement proposed turning it into a **runtime counter** rather
than merely a test; the implemented form is stronger—**a consumption ledger
(`ConsumptionLedger<K>`) + a single settlement point (E-grade `W1090`)**—without introducing a new
diagnostic code or relying on an internal ir_gen counter to prove itself:

```rust
// Around ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each hit at :1942, self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan was not consumed by IR */);
//   }
```

**Why this works:** a `drop()` instruction for a non-`ref` local is observable at runtime through
destructor side effects, but **“a value that should have been dropped was not dropped” may not crash
immediately under YaoXiang’s current value semantics and Arc/Rc model**—which is precisely why it
was silent. Counting turns “silent” into “detectable.”

**Mechanism two: change span keys to `DefId` or explicit plan IDs (**implemented on 2026-10-13**:
RFC-039 D20 / WBS 4.5 implemented this mechanism as a `PlanId(u32)` structural-identity key +
mandatory consumption settlement; see the revision note in §2 of [02](02-stage-contract.md)).** The
fundamental fix is **not to use `Span` as a key**. `Span` is a source location and may drift due to
macro expansion, `include`, or multi-file merging. `FunctionIR` already has a `def: Option<DefId>`
field (`ir.rs:677`), demonstrating that DefId is available at the IR layer. **Design judgment:**
introduce `PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, assign it when ownership /
overload resolution produces the plan, and match by `PlanId` during IR consumption. **This belongs
to the obligations ledger (`Obligations`) in 02 and should be implemented together with
[02](02-stage-contract.md), not in this document.**

**Mechanism three: definition-order dependence in `method_def_ordinals` (independent small item,
**implemented on 2026-11-25**).** The field was originally a `HashMap<String, usize>`, with an
unqualified `"{type_name}.{method_name}"` key, assigning `#N` mangled names according to
definition-traversal order. If typecheck registration order differed from lowering definition order,
same-named methods could resolve to different targets. **Implemented form:** at the same point where
it registers overload candidates, typecheck produces `method_def_ir_names: HashMap<PlanId, String>`
once (a bare name or `#N` mangled name) and passes it to IR generation through `TypeCheckResult`.
The key is the **method-declaration statement’s `PlanId`** (an AST structural identity, the same
model as D20)—`DefId` is not allocated until the lowering tail’s `assign_defs`, so it does not exist
on the typecheck side. Lowering reads the name solely by declaration identity, and **a miss fails
loudly with E3005** (it does not fall back to the bare name, because that would disguise “the
definition name and call name have different sources” as a runnable program). The mutable counter
and its write site have been removed, and definition names no longer have a second derivation entry
point.

### Tracking the Three Hard-Coded Discards

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hard-coded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                                               |
| ------------------ | -------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count becomes zero after a `.42` round trip                                                                               |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **The `.42` artifact loses the entire exception table**                                                                                   |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **The `.42` artifact loses global-variable information** (note that `global_names` **is** populated at `2347`, creating an inconsistency) |

**Design judgment (revised on 2026-10-05, decision D52): all three are brought into this document’s
scope as P7 batch e (7e), and no separate “independent issue” is created.** The original
judgment—the latter two would become independent issues—and the reason it was overturned are
recorded below:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields), unrelated to
   SSA.~~ This is true, but does not justify exclusion—P7 already modifies `bytecode.rs`, and fixing
   all three together requires only one `VERSION` bump (4→5), whereas splitting them into two bumps
   is more expensive.
2. ~~Fixing them requires changing the on-disk `.42` format → bumping the version → expanding scope
   from “IR shape” to “artifact format.”~~ The version-bump mechanism already exists (`MAGIC` +
   `VERSION` header, rejected by the reader when they do not match), and `.42` is a build artifact
   for which cross-version compatibility is not a goal—the expansion is in effort, not risk.
3. ~~The actual risk levels of `2315` / `2341` are low, and running `.42` directly is a secondary
   path.~~ **Overturned:** losing the exception table makes throw/try behave incorrectly when `.42`
   is run directly—throw/try is core language semantics; a secondary path does not imply that errors
   may remain silent. The principle of this refactoring is **not to leave core functionality as an
   unimplemented remnant** (`Vec::new() // Not implemented yet` is precisely such a remnant).

**Retained as well:** all three must still be registered in the `Obligations` ledger as “produced
but not serialized” until 7e is complete, to prevent them from becoming a fourth category of
“something nobody knows about.” `2312` is implemented together with the version bump (D17); the
assigned but unused opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Knock-on Effects of IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                                                          |
| ------------------------------------------------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Add `ValueId(u32)` type                                                   | File header (`ValueId` definition)            | New (**implemented in 7b.1a**)                                                                                  |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**implemented in 7b.1a**)                                                       |
| Add `Phi` variant to `Instruction`                                        | End of the `Instruction` enum (`Phi` variant) | **+1 variant** (**implemented in 7b.1a**; the enum now has 75 variants)                                         |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | **+1 field** (**implemented in 7b.1b**)                                                                         |
| Add `ValueInfo` struct                                                    | `ValueInfo`                                   | New (**implemented in 7b.1b**)                                                                                  |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                    | Add `values()` / `values_mut()` accessors (**implemented in 7b.1b**, with the same shape as existing accessors) |

The file-header `pub use ...ast::Type` is **not changed by this document**—it belongs to the scope
of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **unified** single type.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                        | Location                              | Nature                                                                                                         |
| ----------------------------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or use the “expand to Move at runtime” strategy | `127-554` (currently **66** variants) | +1 or +0 variants (see below)                                                                                  |
| Add a `Phi` arm to `opcode()`                                                 | `558`                                 | Mandatory (`BytecodeInstr` has an exhaustive match)                                                            |
| Add a `Phi` arm to `size()`                                                   | `649`                                 | Mandatory                                                                                                      |
| Add a `Phi` arm to the `From<BytecodeFile>` decode match                      | `943-2350`                            | Mandatory                                                                                                      |
| Fix `upvalue_count: 0`                                                        | `2312`                                | Incidentally fixed by this document (in-memory format; on-disk persistence requires a version bump; see below) |

**The bytecode form of `Phi` requires a design decision.** There are two options (**design judgment:
A is recommended**, for the reasons under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  chain of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  selecting the value according to the predecessor block index. **No opcode is added**, and the
  `.42` format is unchanged. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode + predecessor-index encoding, and have the interpreter maintain register
  versions on jumps. **This requires changing the `executor/` state model + the `.42` format + the
  version number.** The only benefit is size.

The decisive reason for choosing A is that **a `.42` format change triggers version-number issues
and full compatibility discussions, while A’s size cost is irrelevant in interpreted execution**
(the interpreter executes Moves, not Phis).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions distributed across `101-1558`)

| Change                                                       | Location                                                                                                                                 | Nature                                                                                               |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                                 | `OperandResolver` (`codegen/operand.rs`)                                                                                                 | Add an arm                                                                                           |
| Translate the `Instruction::Phi` arm                         | In the dispatch match, before `Free` (533)                                                                                               | New branch that expands into a Move chain (scheme A)                                                 |
| Existing nine NOP-degradation locations **remain unchanged** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart + UnsafeBlockEnd / `658-660` PtrFromRef + PtrDeref + PtrStore + PtrLoad | **Verified as nine instructions:** `655-657` covers two variants, and `658-660` covers four variants |

**SSA conversion does not reduce the number of NOPs.** These nine degradations reflect gaps where
“the IR has it, but bytecode does not”; SSA conversion does not touch them.

#### `src/backends/interpreter/executor/`

Under scheme A, `executor/` requires **no changes**—`Phi` disappears during the `translator.rs`
phase and is never visible to the interpreter. **This is the second decisive reason for choosing
A.**

`BytecodeInstr::Switch` at `executor/ops/control.rs:90` has an active implementation, but `ir.rs`
contains **no** `Switch` / `BrTable` / `JumpTable` variant (all three names have zero hits in
`ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions that the IR layer cannot
produce. It does not affect this document, but it is recorded here because it shows that “IR →
bytecode” is not surjective.

The dispatch match at `executor/debug.rs:194` likewise requires no changes (there is no new opcode
under scheme A).

### Compiler Change List

**File by file and function by function.** All line numbers are the **starting points** of the
changes (verified by measurement):

| No. | File                                                                      | Location                                                                       | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                                                     | `Operand` enum definition                                                      | Add/remove `Operand` variants (**implemented in 7b.1a**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2   | `src/middle/ir/ir.rs`                                                     | End of the `Instruction` enum (`Phi` variant)                                  | Add `Instruction::Phi` (**implemented in 7b.1a**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 3   | `src/middle/ir/ir.rs`                                                     | `FunctionBody::Code`                                                           | Add `values` (**implemented in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 4   | `src/middle/ir/ir.rs`                                                     | `ValueInfo`                                                                    | Add `ValueInfo` (**implemented in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 5   | `src/middle/ir/ir.rs`                                                     | `values()` / `values_mut()` accessors                                          | Add accessors (**implemented in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 6   | `src/middle/ir/ir.rs`                                                     | `pub use ast::Type`                                                            | **Do not change `pub use ast::Type`** (scope of [03](03-type-unification.md); recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 7   | `src/middle/core/ir_gen.rs`                                               | `884-891` `next_temp_reg`                                                      | Return `Operand::Value`, increasing monotonically without rollback (**implemented in 7b.2**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; add an independent `next_value` counter starting from 0 for each function body; the three statement-level reclamations roll back only `next_temp` and do not touch it)                                                                                                                                                                                                |
| 8   | `src/middle/core/ir_gen.rs`                                               | `1954-1959`                                                                    | **Remove** the statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 9   | `src/middle/core/ir_gen.rs`                                               | `3632-3638`                                                                    | **Remove** the statement-level reclamation, including the `result_reg.is_none()` guard                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 10  | `src/middle/core/ir_gen.rs`                                               | `4737-4742`                                                                    | **Remove** the statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 11  | `src/middle/core/ir_gen.rs`                                               | `1745-1759` / `1819-1822`                                                      | Replace with an RAII guard (`generate_method_ir`, entry at `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 12  | `src/middle/core/ir_gen.rs`                                               | `1858-1866` / `2026-2029`                                                      | Replace with an RAII guard (`generate_function_ir`, entry at `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 13  | `src/middle/core/ir_gen.rs`                                               | `2116` / `2180`                                                                | Replace with an RAII guard (`generate_curry_innermost_func`, entry at `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 14  | `src/middle/core/ir_gen.rs`                                               | `2224-2229` / `2284-2288`                                                      | Replace with an RAII guard (`generate_curry_function_ir`, entry at `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 15  | `src/middle/core/ir_gen.rs`                                               | `2768-2777` / `2820-2851`                                                      | Replace with an RAII guard (`generate_anon_binding_ir`, entry at `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 16  | `src/middle/core/ir_gen.rs`                                               | `4686-4695` / `4767-4774`                                                      | Replace with an RAII guard (`generate_lambda_body_ir`, entry at `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 17  | `src/middle/core/ir_gen.rs`                                               | `1987-2000`                                                                    | Extract the E3014 check into `check_register_budget()`, shared by `1798`/`2001` after their semantics are unified                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 18  | `src/middle/core/ir_gen.rs`                                               | `1798`                                                                         | Change `take_cur_locals(param_types.len())` to `take_cur_locals(self.temp_high_water)`, eliminating the inconsistent semantics                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 19  | `src/middle/core/ir_gen.rs`                                               | `248-249`                                                                      | Remove the comment-based contract (now enforced by the type system), replacing it with a reference to `verify`                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 20  | `src/middle/lower/loops.rs`                                               | `generate_spawn_for_ir`                                                        | Change to call `synth::synth_closure` (**implemented in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 21  | `src/middle/lower/expr/spawn.rs`                                          | `generate_spawn_expr_ir`                                                       | Change to call `synth::synth_closure`; remove writes/clears of shared pending fields (**implemented in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 22  | `src/middle/lower/expr/closure.rs`                                        | `generate_lambda_expr_ir`                                                      | Read `SynthEnv` from the arguments instead of using `mem::take` on shared fields (**implemented in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 23  | `src/middle/lower/{mod,scope}.rs`                                         | Symbol table and `NestedBodyGuard`                                             | Remove the `closure_captures` field; use `BindingLocation::Upvalue` lexical bindings and depth-based restoration (**implemented in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                            |
| 24  | `src/middle/lower/expr/{access,closure}.rs` + `query.rs`                  | Variable and namespace queries                                                 | Replace `closure_captures.contains_key` with lexical-upvalue lookup; materialize captures exclusively through `capture_operand` (**implemented in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                             |
| 25  | `src/middle/core/ir_gen.rs` (audit-time path)                             | `341` + `1942`                                                                 | ~~Add a span-keyed consumption counter~~ → covered by 4.5’s `PlanId` consumption ledger (reading and accounting occur together; settlement uses W1090); 7c.2 verified zero triggers, with no code changes                                                                                                                                                                                                                                                                                                                                             |
| 26  | `src/middle/lower/{mod,stmt}.rs`, semantic side `sema/{checker,types}.rs` | `generate_method_ir` definition name + authoritative `TypeCheckResult` mapping | Make `method_def_ordinals` read-only (**implemented in 7c.3**: typecheck produces mangled names by declaration `PlanId`, and lowering only reads them)                                                                                                                                                                                                                                                                                                                                                                                                |
| 27  | `src/middle/core/ir_gen.rs`                                               | `7269-8014`                                                                    | **Last:** decompose into a top-level `CallArgs` + six `emit_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 28  | `src/middle/core/ir_gen.rs`                                               | `7838-7846`                                                                    | Replace the “fill with 0” fallback with a returned diagnostic                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 29  | `src/middle/lower/synth.rs`                                               | **New**                                                                        | Synthesized-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 30  | `src/middle/ir/verify.rs`                                                 | **New** (first layer of [07](07-equivalence-oracle.md))                        | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 31  | `src/middle/passes/codegen/translator.rs`                                 | Dispatch match                                                                 | Translate `Instruction::Phi` (**implemented in 7b.3**: under scheme A, expansion occurs at **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit Move groups by predecessor, existing `JMP`s between groups skip over them, and incoming-edge jumps are redirected by `(predecessor, merge-block first instruction)` to the group start; the `Phi` arm of `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return location cannot hold multiple Moves distributed by predecessor, so reaching it fails loudly) |
| 32  | `src/middle/passes/codegen/operand.rs`                                    | `36-45` / `72-77`                                                              | Add an `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`) (**implemented in 7b.2**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); removal of the `Temp` arm was implemented first in 7b.1a)                                                                                                                                                                                                                                        |
| 33  | `src/middle/core/bytecode.rs`                                             | `127-554`                                                                      | **Unchanged under scheme A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 34  | `src/middle/core/bytecode.rs`                                             | `558` / `649` / `943-2350`                                                     | **Unchanged under scheme A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 35  | `src/middle/core/bytecode.rs`                                             | `2312`                                                                         | Fix `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 36  | `src/backends/interpreter/executor/**`                                    | —                                                                              | **Zero changes under scheme A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 37  | `src/middle/lower/regalloc.rs`                                            | **New**                                                                        | Linear-scan allocator (**implemented in 7b.4**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)`, returning `value_slots` and `frame_slots = named_base + max_live`; an out-of-range result returns `Budget`, which `finalize_code_body` reports as E3014 with the declaration span)                                                                                                                                                                                                                                      |
| 38  | `scripts/ci/check-synth-boundary.py`                                      | **Canceled**                                                                   | The boundary-sentinel script was canceled by the user’s 2026-11-24 ruling 丁; no replacement is created                                                                                                                                                                                                                                                                                                                                                                                                                                               |

### Backward Compatibility: Version Number for the `.42` Format

**Verified fact:** the on-disk `.42` format **already has a version-number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (`"YXBC"`)
and `VERSION: u32 = 4`, writes them in the header (`:342-343`), and validates them when reading
(`:454-467`, reporting “unsupported bytecode version” on a mismatch).

**Design judgment (a direct consequence of scheme A):**

- `Phi` is expanded into a chain of `Move` instructions in the `translator.rs` phase and **does not
  produce a new opcode**.
- The 83 constants in `opcode.rs` have **zero changes**, `opcode_name()` (`120`) is unchanged, and
  the decode match (`bytecode.rs:943-2350`) is unchanged.
- The field layout of the `.42` format has **zero changes**, and `VERSION` remains 4. Old `.42`
  files can still be read by the new binary, and new `.42` files can still be read by the old
  binary. **SSA conversion itself does not require a version bump.**

**The only scenario requiring a version bump is the fix to `2312 upvalue_count`** (decision D17: fix
it and handle it together with the version bump): add an `upvalue_count` field to `BytecodeFunction`
and have the encoder write it. This adds a field to the format and bumps `VERSION` from 4 to 5. The
reader’s existing version check rejects old files according to its existing behavior—`.42` is a
build artifact (`main.rs:671`), and cross-version read compatibility is not a goal, so the version
bump requires no additional migration mechanism. The assigned but unused opcode values (D32/D34’s
`Switch` / `TailCall`) are reclaimed in the same batch.

## Implementation Essentials

### Batch Strategy

There are four batches. **Every item within a batch gets an independent commit, independent
criterion, and independent revert.** The “change items” column below refers to the numbers in the
preceding “Compiler Change List.”

| Batch       | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Change items               | Criterion                                                                                                                                                             | Rollback point                                                                                                                                                       |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Eliminate multiple definitions: remove the three statement-level reclamations → replace six save/restore pairs with RAII guards → unify semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 8–10 first → 11–16 → 17–18 | Removing the reclamations: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and semantic unification are pure moves: **C1** snapshot zero-diff | One commit per item; `git revert` is sufficient                                                                                                                      |
| **Batch b** | Switch to the SSA form: `ir.rs` structural changes + mechanically change temporary-value production to `Operand::Value` → add the `Phi` arm in `translator.rs` (scheme A) → linear-scan allocator. Within the batch, follow the WBS third-level task order serially: 7b.1a (`Operand` variant consolidation + introduction of `Instruction::Phi`, **implemented**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **implemented**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **implemented**) → 7b.3 (`Phi` arm in `translator.rs` + population of `BasicBlock.successors`, **implemented**) → 7b.4 (linear-scan allocator + removal of the three statement-level reclamations, together with 7a.1, **implemented**) | 1–5, 7 → 31–32 → 37        | **C4**: behavioral equivalence + `verify_ssa` fully green + `.42` round-trip test                                                                                     | The `ir.rs` changes **must occupy one commit by themselves** (the type changes affect compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary → verify the existing consumption ledger (no code changes) → make `method_def_ordinals` read-only (the sentinel script was canceled by the user’s 2026-11-24 ruling 丁)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | 20–24, 29 → 25 → 26        | **C4**: behavioral equivalence; the consumption counter has zero triggers on the new diagnostic (otherwise, a real mismatch exists)                                   | Independent commit                                                                                                                                                   |
| **Batch d** | Decompose `generate_call_expr_ir` + turn the “fill with 0” fallback into an assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | 27–28                      | Pure-move portion: **C1** snapshot zero-diff. Assertion portion: **C4** corpus differential + first-layer verifier                                                    | Independent commit                                                                                                                                                   |

**Three ordering dependencies within the batches are non-interchangeable:**

1. Batch a’s “remove reclamations” must follow the `ir.rs` changes in batch b—the removal of the
   three statement-level reclamations landed together with 7b.4 and the linear-scan allocator, with
   7a.1 executed alongside it—because `next_temp` must increase monotonically before `ValueId` can
   have the semantic foundation of “unique definition” (this semantic only becomes established after
   7b.2 changes `next_temp_reg` to produce `Value` and 7b.3 implements the `Phi` arm).
2. Batch b’s `Phi` arm must precede the decomposition in batch d—otherwise, the decomposition would
   carry the old `arg_regs` ordering assumptions into the new file unchanged.
3. Batch d must come last—it depends on the previous two batches having already converged `arg_regs`
   semantics to “one independent `ValueId` per argument.”

**Position relative to RFC-039:** all four batches fall within RFC-039’s **P7**, and the criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial** relative to
one another, but there is no coupling between items 8–10, 11–16, and 17–18 within batch a beyond
their ordering constraints.

### Hard Prerequisites

**Prerequisite one: criterion baseline.** The first-layer verifier’s `verify_loose` mode from
[07](07-equivalence-oracle.md) **must first run green against the existing non-SSA IR** before batch
a may begin.

The reason is that the SSA-conversion criterion is C4 (behavioral equivalence + invariants). If the
invariant verifier itself cannot run green before the refactoring, then every acceptance gate in
batches a–d lacks an executable criterion and can rely only on corpus differentials—yet corpus
differentials are **precisely insensitive** to Defect Classes 1/2/3 (as established in 07’s
“Alternative A”). **This is a genuine ordering constraint.**

**Prerequisite two: type-representation unification.** [03](03-type-unification.md) must complete
the convergence of `pub use ast::Type` at `ir.rs:3` and remove the `From<MonoType> for IrType`
bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` will introduce a third type-compatibility
implementation.**

### Rollback Strategy

- Every change receives an independent commit, making the `git revert` granularity equal to the
  change-item granularity.
- The batch-b `ir.rs` structural changes are **the only part that cannot be rolled back
  incrementally** (type changes affect compilation throughout the repository). Therefore, they must
  **occupy one commit by themselves**, and batch a must already have run green before them,
  providing a revertible baseline.
- **Do not use a feature flag.** The reason is that adding/removing `Operand` variants cannot be
  isolated with a runtime switch; a feature flag would cause CI to stop testing the other path over
  the long term—recreating exactly the “test wiring depends on human memory, and silent decay
  follows” problem recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front; otherwise, the work will be challenged midway through with “the line
count did not go down.”**

| Item                                      | Line-count change                                                                                                                                                                  |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                            | `ir_gen.rs` **8,448**                                                                                                                                                              |
| Remove three statement-level reclamations | **−15**                                                                                                                                                                            |
| Replace six save/restore pairs with RAII  | Net ±0 (the RAII guard definition is about +20 lines, while use sites fall from 22 lines to about 12)                                                                              |
| Modify `next_temp_reg`                    | +5                                                                                                                                                                                 |
| Decompose `generate_call_expr_ir`         | **+40–80** (signature and parameter-passing overhead for six `emit_*` functions; this is the typical cost of a pure-move decomposition—**decomposition increases the line count**) |
| Span-keyed consumption counter            | +15                                                                                                                                                                                |
| Make `method_def_ordinals` read-only      | +20                                                                                                                                                                                |
| New `synth.rs` file                       | +80–120                                                                                                                                                                            |
| New `verify.rs` file                      | +400–600 (the first layer of 07, although within this document’s scope, is counted separately)                                                                                     |
| New `regalloc.rs` file                    | **+500–1,000** (measured at 271 lines in 7b.4)                                                                                                                                     |
| **Total**                                 | **`ir_gen.rs` from about 8,448 to 8,600–8,800; including new files, a net increase of 900–1,600 lines**                                                                            |

**The early expectation of “8,448 → 5,000–6,000” was incorrect**; measured extrapolation produced
the opposite result. The reason must be explained:

1. **The core benefit of this document is not fewer lines, but eliminating an entire class of
   defects that “fail without reporting errors.”** The six manual save/restore pairs (22 lines)
   become about 12 lines plus a guard type after SSA + RAII—**the line count is almost unchanged,
   but “forgot to restore” changes from a possibility into a compile error**.
2. **A pure-move decomposition necessarily increases the line count.** Decomposing the 746-line
   `generate_call_expr_ir` into seven functions requires repeating the signature (the function
   already has `#[allow(clippy::too_many_arguments)]`, `7268`), the `&mut self` binding, and error
   propagation for each function. **“Splitting the file reduces the line count” is a
   misconception—it reduces cognitive load, not physical line count.**
3. **The new verifier and allocator increase the line count.** These two components (about 900–1,600
   lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit:**

> In terms of lines, this document will most likely produce a **net increase**. The benefit lies in
> **verifiability**—turning defects that “fail without reporting errors” into invariants that **fail
> the build**. If line count is used as an acceptance criterion, this document will fail; the
> correct acceptance criterion is the one in the “Batch Strategy” section.

**This phase will intentionally increase the amount of code in `ir_gen.rs`.** The 2026-10-03
decision canceled all line-count and size gates (see prohibition two in
[08](08-maintenance-mechanism.md)); therefore, **no line-count exemption list is needed**—the
legitimacy of the increase is established by the separation-of-responsibilities argument: SSA
conversion removes “register allocation + definition discipline” from lowering into independent
modules, trading increased code size for the elimination of the two classes of manual discipline in
which “misalignment does not report an error and merely produces wrong values”: six manual
save/restore pairs and the semantic reordering of `arg_regs`.

## Key Decisions and Rationale

### Three Core Decisions

**Decision one: temporary-value strategy—remove the three statement-level rollbacks so that unique
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Removing them makes `next_temp`
increase monotonically, so every temporary slot naturally has exactly one definition point. **The
criterion is consistent but the guards differ** (`3632` has a `result_reg.is_none()` guard, while
the other two do not), and these hand-copied differences disappear with them. The cost is increased
slot occupancy—the existing 255 upper-limit check on `temp_high_water` (`1992-2000`) provides the
safety net.

**Decision two: do not perform mem2reg; perform explicit renaming only.** The reasons are given
under “Value Identification Method”: YaoXiang’s `&` / reference / ownership semantics make “which
slots may be memory-promoted” a type-system problem that requires the type representation unified by
[03](03-type-unification.md). **Performing mem2reg before 03 is guessing.** Explicit renaming only
requires changing the return-value semantics of one `next_temp_reg` and removing the three
rollbacks; it does not touch the control flow of any lowering algorithm.

**Decision three: adopt scheme A for `Phi` (IR-only pseudo-instruction expanded into a chain of
Moves).** `executor/` and the `.42` format have **zero changes**, and the regression surface is
limited to L3. The decisive reason is that a `.42` format change triggers version-number issues and
full compatibility discussions, while A’s size cost is irrelevant under interpreted execution (the
interpreter executes Moves, not Phis). The cost is that `Phi` is not visible in `dump_bytecode`
(this must be kept in mind when entering the C1/C2 phases, where the third layer in 07 performs the
comparison).

### Rejected Alternatives

**A. Decompose everything at once (SSA + decomposition + `synth` boundary + allocator
simultaneously). Rejected.** This violates 07’s core principle: criteria must be tiered by category.
Mixing all four batches into one PR mixes pure-move changes with C4 changes, so snapshot drift
cannot be attributed—when a snapshot changes, it is unclear whether the decomposition moved
something incorrectly or the SSA change was incorrect. **Rollback granularity degrades from “change
item” to “everything.”**

**B. Split the file first, then optimize ordering. Rejected.** Decomposition does not eliminate any
root cause: the six manual save/restore pairs remain scattered across 10 files, and `arg_regs`
semantic reordering still requires manual reasoning. Worse, **splitting first loses the opportunity
to “build criteria before changing anything”**—decomposition produces a large line-level diff that
mixes into the diff of subsequent SSA changes and invalidates review. This has the same root cause
as the eight orphaned test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests that
never ran): **with the wrong order, every subsequent step accelerates along an incorrect
foundation.**

**C. Introduce an intermediate SSA layer without changing the existing lowering. Rejected, but
recorded as “a third option that was once considered.”** The form is to leave `ir_gen.rs` unchanged
and add a pass that converts its non-SSA IR into SSA IR. **The advantage** is that lowering remains
untouched and rollback is easy. **Reasons for rejection:** (1) It does not eliminate the root cause
of Defect Class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the six
save/restore pairs remain separated by 59–161 lines. A conversion pass can merge multiple
definitions into `Phi`, but **it cannot fix “an inner function’s name leaks into the outer
scope”**—that is a `cur_locals` save/restore error that occurs before the IR form and is invisible
to the conversion pass. (2) It turns SSA into a second representation—the repository already bears
the lesson of three parallel type representations, and this project’s core lesson is that
**“boundaries exist only in people’s awareness.”** (3) `verify_loose` would become a permanent
burden.

**D. Keep the current state and only add a verifier. Rejected as the sole solution, but it is a
component of this document.** RFC-039 Alternative B reaches the same conclusion: a gate can prevent
regressions but cannot repair the current state. **However, what D can accomplish must be stated
clearly:** D can **detect instances** of Defect Classes 1/2/3 (provided that the verifier covers
dominance, type consistency, and inner isolation), but it cannot detect Defect Class 4
(`closure_captures` leakage)—that requires a cross-function liveness assertion beyond the boundary
of `verify(&ModuleIR)`. **Therefore, D is not an alternative to this document; it is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point**—the 2026-10-03 decision canceled line-count and size gates, and the scale issue is
  addressed through separation of responsibilities (see prohibition two in
  [08](08-maintenance-mechanism.md)). The legitimacy of the increase lies in replacing two classes
  of manual discipline in which “misalignment does not report an error and merely produces wrong
  values”: six manual save/restore pairs and the semantic reordering of `arg_regs`.
- **The “unique definition” check in `verify.rs` is meaningless in the non-SSA form**
  (`Operand::Local` permits multiple definitions)—document 07 records this limitation, and
  `verify_loose` is its degraded mode for use before this batch.
- **Removing statement-level reclamations changes the temporary-slot allocation pattern**
  (implemented in 7b.4, together with 7a.1): after removing the reclamations, slots are reused by
  the linear-scan allocator according to live intervals, **so the allocation result changes**—this
  is precisely why C4 must genuinely run against the full corpus; the measured 403/403 differential
  was empty.
- **`u8` / 255 slot limit:** the two corpus cases that exceeded the limit after removing the
  reclamations (`string_ops.yx` with 354 monotonic slots and `list_comp.yx` with 323 monotonic
  slots) are compressed by the allocator to **15 / 24 frame slots**; the budget gate now reports
  E3014 only when the post-allocation frame truly does not fit, with the function-body declaration
  span, rather than incorrectly rejecting based on “cumulative temporary count.”
- **Benefits must be recorded as well:** turning “errors that cannot be tested” into “testable
  invariants”; no backend changes (`executor/` and `.42` have zero changes); no changes to
  `Instruction` semantics (all 76 → 77 variants are retained, and all 50 `translate_*` functions are
  retained); incremental delivery is possible (each item can be reverted independently).

> **All open questions originally listed in this section have been resolved.** See
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for each
> decision. **This document leaves no pending items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching architecture; four-layer model, criterion tiers, P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definition of C4
  criteria, `verify_loose` / `verify_ssa`, three-layer criteria, and the limitation that snapshots
  are immune to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of three
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the
  `ReleasePlan` `PlanId` conversion
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — eight orphaned test trees; the counterexample
  of “creating a directory first, then failing to wire it up”

### Code Locations

**IR definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — `Operand`, 7 variants
- `src/middle/core/ir.rs:47-533` — `Instruction`, 76 variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (the feasibility basis for the
  `PlanId` scheme)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect Class 1 (multiple definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for the seven nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — sole table-extraction point + E3014 budget
  point (`MAX_REGISTERS = 255`), combining block partitioning / slot allocation / table extraction
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only from within `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named-variable high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — linear-scan allocator (sole producer of `value_slots` and
  `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamations
  removed (7b.4); slots are now reused by the allocator
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant
  referenced by 07

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  a clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill with 0” fallback

**Defect Class 3 (span keying)** — the following are **audit-time** evidentiary pointers (after the
4.5 key migration, both `release_plan` and `overload_resolutions` are keyed by `PlanId`; see the
revision note in §2 of [02](02-stage-contract.md) for current pointers; `method_def_ordinals` was
eliminated in 7c.3 through mechanism three):

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of the three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — `release_plan` write and consumption
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write site of
  `method_def_ordinals` (key is the unqualified `"Type.method"`; **the field was removed in 7c.3**,
  and this is audit-time evidence)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the propagation structures
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — definition and
  production of `ReleasePlan`
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer propagation
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (implicit ordering dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — the equivalent handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — two read sites of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — three other `ast::Type` construction sites

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr`, 66 variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded discards (compare `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (the bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — nine NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, `u8` limit of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; 83 constants in the file
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current test state**

- `src/middle/core/ir_gen.rs` — 8,448 lines, zero `#[cfg(test)]`, zero `mod tests`
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, 29 tests in
  total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no IR
  structural-invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — four references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example opcode-round-trip criterion; the last test in the file; the file contains 1,209 lines / 23
  `#[test]` functions)

**Engineering configuration**

- `Cargo.toml:37-39` — dev-dependencies contain only `criterion` / `proptest`; `[dependencies]`
  contains no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans `/rfc/draft`; this
  document is not under that directory and is referenced by RFC-039 and this directory’s index
