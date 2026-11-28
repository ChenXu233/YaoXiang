# SSA-ification of the Intermediate Representation

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main RFC-039 text for the four-layer model, tiering of acceptance criteria, and
> execution-stage ordering; see the [index for this directory](index.md) for the role of each
> companion document.

## Purpose and Scope

This article addresses the construction discipline for the L3 IR. For `src/middle/core/ir_gen.rs`
(8,448 lines), it proposes a construction method based on **a single definition for every value**.
**The goal is not to introduce a new instruction set, but to eliminate three classes of discipline
defects that fail without producing any errors**:

1. Six manual register save/restore sites separated from their related code by dozens of lines;
2. Semantic reordering of the shared `arg_regs` value across six branches within
   `generate_call_expr_ir` (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of `Span`-keyed cross-layer contracts.

This article is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, explicit cross-layer contracts, decomposition of
call-expression dispatch, and integration of the static IR verifier.

This article is not responsible for: convergence of type representations
([03](03-type-unification.md)), stage contracts and the obligation ledger
([02](02-stage-contract.md)), definition of equivalence criteria and their enforcement gates
([07](07-equivalence-oracle.md)), frontend paradigms ([05](05-frontend-paradigm.md)), or cleanup of
dead code and speculative designs ([06](06-cleanup-inventory.md)). This article repeatedly relies on
conclusions from those documents without repeating their arguments.

**Criterion category** follows **C4 (IR shape changes)** in
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, rather than byte-for-byte IR snapshot equality. SSA-ification necessarily changes the
IR shape, and forcing snapshot equality would encourage the team to weaken the criteria—the exact
trap recorded in 07.

**Its position in the RFC-039 stage sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, with acceptance requiring behavioral equivalence and a green `verify_ssa`). The
implementation batch numbers within this article (batches a–d) are distinct from the ordering within
P7: the former are this article's batching strategy, while the latter are RFC-039's global stages.

## Feasibility Preconditions

**Verified facts.** The IR already has **all structural prerequisites** needed to construct SSA:

| SSA prerequisite             | Current repository state                                                                                           | Location                                                               |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                                   | `BasicBlock` in `src/middle/ir/ir.rs`                                  |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                     | `FunctionBody` in `src/middle/ir/ir.rs`                                |
| Nested scope information     | `LocalSlot::scope_depth` (`"0 = function parameter layer, used to disambiguate duplicate names in nested scopes"`) | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                                 | `Instruction` in `src/middle/ir/ir.rs`                                 |
| Per-instruction span         | Every variant has a `span: Span` field                                                                             | Same as above                                                          |
| Annotatable value types      | `LocalSlot::ty: MonoType`                                                                                          | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Slot-count upper-bound check | Table extraction and E3014 budget gate share one point (`MAX_REGISTERS = 255`)                                     | `finalize_code_body` in `src/middle/lower/scope.rs` (from 7b.4 onward) |

**If these prerequisites were absent, the cost estimate in this article would be completely
different.**

**The two decisive points are:**

**First, SSA-ification is not a new capability; it removes a capability dependency.** In the current
implementation, temporary slots are **deliberately designed to be defined multiple
times**—`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` explicitly roll `next_temp` back in
three places. The first step of SSA-ification is to **delete these three rollbacks**, making
`next_temp` increase monotonically. Once they are removed, **every temporary slot naturally has
exactly one definition point**—single definition changes from “a rule that must be checked” to “a
mathematical consequence of the construction method.” Batch a only converged the save/restore
surface (see “Defect Class 1”); the three rollbacks were removed with batch b's 7b.4 (when the
allocator took over slot reuse) on 2026-11-22.

**Second, the slot-count upper bound is a hard check, and deleting the rollbacks briefly caused slot
counts to become unbounded (resolved by 7b.4).** The budget gate in `src/middle/lower/scope.rs`
reports E3014 when the count exceeds 255. After removing the three reclamations and running the full
corpus, two corpus entries crossed the limit and triggered E3014: `src/std/tests/string_ops.yx` with
354 slots and `tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; peak slot counts
for the remaining corpus entries rose from 51 to 211. **After 7b.4 landed:** the linear-scan
allocator reuses slots according to live intervals, and the two corpus entries now use **15 / 24
frame slots** (344 / 304 values), while the other 403/403 corpus entries remain behaviorally
equivalent. From that point onward, E3014 means only “frame slots after allocation exceed 255.”

**Taken together, these two points mean:** lowering already maintains all the bookkeeping
information required for SSA, but **discards it**; 7b.4 makes the allocator responsible for slot
reuse and removes the reclamation. The main SSA-ification effort therefore lies in decomposing
`generate_call_expr_ir` and making cross-layer contracts explicit, not in building IR
expressiveness.

### Prerequisite: Unification of Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The serialization side has a third representation, `ir::Type`,
bridged through `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR contains two type representations simultaneously, and one of them directly
`pub use`s the AST type.**

SSA-ification requires a new `Phi` variant, a type for every SSA value (to support the verifier's
“type consistency” invariant from 07), and a way to determine whether the types of the two
predecessors at a CFG join point are compatible. **If type representations have not converged, SSA
will require a separate type-compatibility check for each representation**—and the third
representation will grow accordingly.

This is not a question of which task to perform first, but of **which task would produce three sets
of type-compatibility logic**. [03 Type Representation Unification](03-type-unification.md) must
first converge `ir.rs:3`'s `pub use ast::Type` and remove the bridge at `bytecode.rs:2353`. Item 6
in the change list explicitly records that `ir.rs:3` is **not changed by this article**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not a file that is merely “ugly but functional.” It is a file that **bets correctness
on human line-by-line review**:

- It contains **zero `#[cfg(test)]` blocks and zero `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares the `bytecode`, `def_assign`, and `local_slots` modules,
  with 29 tests in total. `def_assign.rs` and `local_slots.rs` **positively** cover ir_gen's DefId
  allocation and local-slot naming (at sentinel level), while all four ir_gen references in
  `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **negative assertions**—“ir_gen does not
  emit,” “ir_gen does not construct either,” and “the ir_gen frontend does not construct the
  corresponding IR.”

In other words: ir_gen has 29 sentinel-level tests pinning local behavior, but **has no verification
of any whole-IR structural invariant** (dominance, single definition, jump targets, or type
consistency)—none of the three modules declared by `middle/core/tests/mod.rs` makes positive
assertions about the overall IR shape.

This file also happens to contain several classes of defect that **compile successfully, pass all
checks, and run normally, but produce incorrect values**. Their common property is: **failure
produces no error**.

This is where the highest risk lies. The problem is not “the function is too long” or “the naming is
confusing,” but that **there is no machine-enforced mechanism for this file's core invariants**.

The contract “save before generating a nested function body, then restore it afterward; otherwise
names from the inner function will leak into the outer function” is enforced by the type system:
`NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively borrows the generator as `&mut`; frame
removal and restoration each have exactly one implementation (see “Defect Class 1”).

**Why adding a verifier alone is not enough**: a verifier can **catch** these defects, but it cannot
catch **their causes**. The cause is not “a reviewer missed them,” but:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen's
> temporary-register allocator deliberately depends on that property.**

The concrete evidence is three statement-level register reclamations (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`). At the end of every statement, they **roll `next_temp` back**
to the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and the number and
order of writes and the live intervals are determined entirely by a single manual decision: **when
to roll back**.

Under this shape, “single definition” is not a rule that can be applied incrementally; it **directly
conflicts with the register-allocation strategy**. SSA-ification is therefore not a matter of adding
a check to existing code, but **replacing the allocation strategy that depends on multiple
definitions**. Fortunately, the cost of replacing it is low—see “Feasibility Preconditions” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (The Three Reclamations Were Removed in 7b.4)

**Verified facts.** There are seven entry points at which entering a nested function body requires
save/restore. Batch a (7a.2) converged them into one RAII guard: `NestedBodyGuard<'a>` in
`src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, unconditional restoration in
`Drop`, and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Rollback scope                                         |
| ------------------------------------ | ---------------------------------- | ----------------------------- | ------------------------------------------------------ |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (three counters are not rolled back) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                          |
| Nested function definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                     |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                          |

- Frame removal occurs only in the two constructors (`enter` / `enter_decl`), and restoration occurs
  only in `Drop`. The guard exclusively borrows the generator as `&mut` for its lifetime, so no
  second `&mut` is available in scope; every `?`/early return passes through `Drop` for restoration.
- The fields involved are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`; from 7b.4 onward,
  `next_temp_reg` maintains only the named high-water mark (temporary-value slots belong to the
  linear-scan allocator; `temp_floor` was removed with the three reclamations).
- `enter_decl` does not roll back the three counters: after a function or method body returns to the
  module-initialization flow, subsequent definitions continue from the child body's final value
  number. This is the constraint for zero drift in C1 snapshots and the meaning of “the declaration
  surface rolls back only the table.”

**Unified Accounting and a Single Budget Point (from 7b.4 onward)**: `finalize_code_body` in
`src/middle/lower/scope.rs` is the only table-extraction and budget point—it extracts the table
using `frame_slots = named_base + max_live`, reports E3014 with the function-body span if the limit
is exceeded, and defines `MAX_REGISTERS = 255` in exactly one place repository-wide. All six
function-body forms (function / method / lambda / anonymous binding / intermediate curry layer /
innermost curry layer) are finalized through it, so there is no second definition of “how many slots
a function body should have.”

**The three statement-level reclamations were removed (7a.1 landed with 7b.4 on 2026-11-22)**: the
reclamation had previously been copied by hand into `generate_function_ir` / `generate_block_ir`
(with a `result_reg.is_none()` guard) / `generate_lambda_body_ir`; after the allocator took over
slot reuse, reclamation degraded to an identity operation, so the guard, `temp_floor` /
`saved_temp_floor`, and the loop-entry/loop-exit bookkeeping in `loops.rs` were all removed. Module
initialization and top-level statements still use the old monotonic-slot path with
`value_mode=false` and do not run the allocator; their slot numbers and table lengths shift with the
number of named slots in function bodies (three snapshots measured; behavioral equivalence is
covered by 403/403).

**Consequences (current form)**: the guard exclusively borrows `&mut`; restoration has only one
implementation in `Drop`; and there is no writable location at which a restore can be omitted. The
temporary-slot dependency on “one slot with multiple definitions” has disappeared along with the
reclamations: value slots are reused by the linear-scan allocator according to live intervals,
giving the value-single-definition criterion in `verify_ssa` a construction-side foundation.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified facts (as of the audit; the current location reflects the 7c batch-c split).**
`generate_call_expr_ir` is now in `src/middle/lower/expr/call.rs` (split out from
`src/middle/core/ir_gen.rs`, which no longer exists), and is a single function. Its signature
accepts `func: &Expr` / `args: &[Expr]` / `named_args: &[(String, Expr)]` / `span: &Span` /
`call_plan_id: PlanId` / `result_reg: Operand` / `instructions` / `constants` (`call_plan_id` is the
decision-consumption key from RFC-039 D20). Multiple internal branches share the same `Vec<Operand>`
and perform **semantic rewriting** (the line numbers below are from the audited version; after the
split, locate them by symbol):

| Line        | Semantics                                                                                                                                              |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `7395`      | Namespace call: flatten by `slots`, then re-`collect` — reorders named arguments                                                                       |
| `7724`      | Struct construction: `unwrap` each slot from `final_args`, then re-`collect` — reorders fields                                                         |
| `7835-7848` | Function call: reorder named arguments; `7838-7846` **adds a new register with value 0** for each missing slot (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();` — clone before emitting instructions                                                                |

The original source of the four reorderings:

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
reports E1010. For runtime safety, fill with 0 (the normal path should never reach this).”

**Current handling of named arguments (after narrowing in 4.9.9)**: before reordering, named
arguments are bound using the **declaration-time formal-parameter name table**—the shared criterion
`arg_binding::bind_args_to_params`. The IR-side name table is obtained through `lookup_param_info`,
sourced from this unit's `fn_param_names`, cross-unit method candidate groups (`method_param_info`,
queried using the qualified name from `method_dispatch_name`), and the module export surface.
Cross-unit method arms without overload resolution do not carry named arguments; the old behavior
silently discarded them, and that known gap is recorded in 09 under 4.9.9 → 4.9.10. The “fill with
0” behavior in the table above remains a runtime safety net for positional arguments and fallback
arms; it is unreachable on the normal path.

**This design has two risks**:

1. **Misalignment reports no error and merely produces the wrong value.** Each of the six branches
   independently determines the final order of `arg_regs`, so an incorrect ordering assumption in
   any branch does not panic.
2. **Normalized snapshots are immune to this defect.** This limitation is recorded in 07: after the
   snapshot tool renames temporary values by “first-occurrence order,” “the third argument should
   have used the fifth register” is identical in the snapshot. **This class of defect has no
   snapshot-level criterion at all during the C4 stage.**

**Additional verification: the struct-construction path's reordering has no duplicate-slot guard.**
`final_args` at `7724` is populated according to “positional arguments occupy their slots first,
then named arguments overwrite by field name,” but **there is no duplicate-slot check such as
`slots[idx].is_some()` before overwriting** (unlike the namespace path at `7381`)—when a named
argument silently replaces a positional argument, there is no panic or diagnostic. Furthermore, the
argument **evaluation order** (source order) diverges from the final `arg_regs` order
(field-declaration order) on this path. **7d.2's assertion-based handling covers only the three
“missing slot” cases, not “duplicate slots”**—named arguments in struct construction can still
silently overwrite positional arguments. That branch is reachable only in the state “type checking
has reported an error, but IR is still generated”; the shared criterion rule ③ in
`bind_record_ctor_args` on the type-checking side already rejects this with E1111. The gap is
recorded under “7d legacy items” in [09](09-execution-wbs.md) and is not closed with 7d.

### Defect Class 3: Silent Failure of `Span`-Keyed Cross-Layer Contracts (**Eliminated** on 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified facts (at audit time).** The repository has **two** `Span`-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                       | Producer                                                                                                                 | Propagation                                                                             | Consumer                                                                                                                    |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                         | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatching the statement identity of `walk_stmt`) | `typecheck/types.rs:38` ← `checker.rs` propagation ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, sole entry point; settlement-completed marker `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger in one type, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` only propagates and merges)    | `typecheck/types.rs:62` ← `checker.rs` assembly ← `ir_gen.rs` field ownership           | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; settlement-completed marker `:1097`) |

The form at audit time (`ir_gen.rs:1942`, **now nonexistent**) was:

```rust
// NLL Release: insert a Drop instruction at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some`—**if the lookup fails, it simply fails: there is no else branch, assertion, or
counter**. If the two sides compute spans differently in any respect, **the Drop instruction
silently disappears, with zero errors**. The Drop sequence for refinement types is a central carrier
of this project's ownership semantics (`ownership.rs:26-32` describes the “NLL precise release
plan”), and its implementation was silent.

The same form appears at `7420` / `7493`
(`if let Some(mangled) = self.overload_resolutions.get(span)`), causing overload resolution to fall
back to the default binding.

**Disposition (landed in 4.5)**: both keys were changed to the structural identity `PlanId`, and
consumption is **accounted for on a hit** (the read and accounting occur in the same call). A
non-empty difference is reported at the sole settlement point as **E-level `W1090`**—“not found” is
no longer silent. See the revision note in §2 of [02](02-stage-contract.md) (4.5) for the
implemented form and criteria. The line numbers here (`1942` / `7420` / `7493`) are retained as
audit-time evidence and are not current pointers.

> **`method_def_ordinals` was not `Span`-keyed (**eliminated in 7c.3**).** It was originally a
> `HashMap<String, usize>` that assigned `#N` suffixes according to definition traversal order over
> `base_name`. Its risk was **different**: dependence on definition order—if generation order
> differed from type-check registration order, the same function name would resolve to a different
> target. See mechanism three under “Additional Span-Keyed Mechanisms” for the implemented form: the
> definition name is produced once by type checking from declaration identity, and lowering only
> consumes it.

### Defect Class 4: Implicit spawn/lambda Handshake (Eliminated in 7c.1)

**Eliminated.** spawn and spawn-for previously required the caller to write `pending_env_vars` /
`pending_env_names` first, after which a synthesized Lambda consumed them; closure bodies separately
used `closure_captures` to determine whether a variable was read as an upvalue. Three fields placed
correctness at the mercy of call order and manual cleanup: a nested synthesized closure could
mistake an outer local slot with the same name for a captured value, and an early return while
generating a closure body could leak capture state into later, unrelated lowering.

7c.1 converged synthesized AST into `middle/lower/synth.rs`, replacing the three shared fields with
an explicit `SynthEnv` and registering named captures as lexical upvalues. Nested frame exit
restores state according to symbol-table depth, so error paths no longer leave capture state behind.
See “`synth.rs` Boundary” for the implemented structure.

### What SSA Will Not Solve (Stated Explicitly)

| Issue                                                                 | Location                                                                                                                                     | Why SSA Does Not Solve It                                                                                                                                                                                                                           |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Complexity of `compile_pattern`                                       | `ir_gen.rs:5390-5687`, **298 lines**, four levels of self-recursion (`5473` / `5504` / `5619` / `5640`), external call site `5270`           | Dispatch complexity for pattern matching (union payloads / tuples / structs / literals / nesting) is **inherent to the algorithm**. SSA only changes `JmpIfNot` instructions produced by guards into `Phi`; it does not reduce the number of guards |
| Complexity of `eval_const_expr`                                       | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)                      | Constant-folding expression coverage is unrelated to SSA                                                                                                                                                                                            |
| `Span`-keyed failure (**eliminated**)                                 | At audit time: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)—after 4.5, `PlanId` keys + mandatory consumption settlement | **SSA does not touch this layer at all** (the conclusion is unchanged). It was independently resolved by the obligation-ledger mechanism in [02](02-stage-contract.md); see mechanism two under “Disposition of the `Span`-Keying Problem”          |
| Number of `Instruction` variants                                      | 75 (`Instruction` enum in `ir.rs`; includes `Phi` introduced by 7b.1a)                                                                       | SSA adds `Phi`, a **net change of +1** (already landed). Variants map to bytecode opcodes or degrade to NOP and do not decrease because of SSA; under plan A, `Phi` is an IR-only pseudo-instruction and adds no opcode                             |
| Definition-order dependence of `method_def_ordinals` (**eliminated**) | At audit time: `ir_gen.rs:224` / `369` / `1692-1702`; 7c.3 is described under mechanism three in “Additional `Span`-Keyed Mechanisms”        | Similar to `Span` keying but with a different cause; the definition name is now produced by type checking from declaration identity, and lowering only consumes it                                                                                  |

## Target Design

### Definition of the SSA Shape

**Design judgment.** The target shape is as follows.

#### Changes to the `Operand` Variants

The `Operand` enum in `ir.rs` has five variants. **Repository-wide measurements** (hit counts summed
by file using `git grep -c`, covering all of `src/`, including tests; construction-point counts
refer to value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                               | Disposition                                                                                       |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                                       | **Retained**, unchanged                                                                           |
| `Value(ValueId)`    | 7 files / 26 hits; introduced by 7b.1a with **zero production construction points**, and from 7b.2 onward `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`)                | **Retained** (introduced by 7b.1a)                                                                |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                                       | **Retained, but semantics split** (see below)                                                     |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                         | **Retained** as the implicit definition at function entry (parameters are definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                                        | **Retained**, unchanged                                                                           |
| `Temp(usize)`       | **Zero construction points** (the handling arm exists only in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, and is removed with the variant)                                       | **Removed** (dead variant, 7b.1a)                                                                 |
| `Label(usize)`      | **Zero construction points**                                                                                                                                                                              | **Removed** (dead variant, 7b.1a)                                                                 |
| `Register(u8)`      | **Zero construction points** (the comment at the original variant declaration said "Added for codegen", but codegen uses the `OperandResolver` in `codegen/operand.rs` and never constructs this variant) | **Removed** (dead variant, 7b.1a)                                                                 |

`Local(usize)` is split into two variants because it currently **serves two mutually exclusive
semantics**—“the storage location of a named variable” (which may be written multiple times) and “a
compiler temporary register” (which should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within a function, single definition
    Value(ValueId),
    /// Storage slot for a named local variable: addressable, multiple definitions allowed (non-SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing and never rolled back**. The split
and `ValueId` landed in 7b.1a (`Operand` enum in `src/middle/ir/ir.rs`); the producer side
(`next_temp_reg` changed to produce `Operand::Value`) landed in 7b.2 (see “Value Identification”
below for the production form).

#### `Phi` as a New `Instruction` Variant

```rust
/// Join point: each predecessor basic block provides one definition of this value
/// incoming is sorted by predecessor block label in ascending order (guaranteed at construction, for normalization)
Phi {
    dst: Operand,                                  // Must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (see the first layer in [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set of `incoming` == the set of labels of all blocks in `blocks` whose `successors`
   contain this block's label.
3. The `MonoType` of all `incoming` values is identical (**dependent on type representations having
   converged in 03**).

The implementation status of `verify_ssa` (the first three constraints were completed in 7b.1b):
constraints 1 and 2 require the value table and block-level CFG, and landed in **7b.1b** as the two
criteria “block-level predecessor count” and “predecessor label set”—the predecessor set is derived
from `BasicBlock.successors` (`block_predecessors` in `src/middle/ir/verify.rs`); a count mismatch
reports only the count, while a set mismatch reports the set, so the same defect is not reported
twice. Constraint 3's per-input type consistency landed at the same time, taking values from the
value table `values[v].ty` and **reusing the existing `types_compatible`** (no third
type-compatibility check was introduced; convergence of type representations in 03 remains the hard
“Precondition Two”; at that point, `ValueInfo::ty` will still be `MonoType`). All three criteria use
the violation category `InvariantKind::PhiConsistency`.

**Three forms for which the type is unavailable (explicitly recorded; never silently allowed)**: an
empty value table means that no basis for judgment exists (the constant shape of module
initialization / top-level statements, the `value_mode=false` path); a non-empty value table with an
out-of-range `ValueId` is a violation; and a non-`Value` operand (`Const` / `Local` / `Arg` /
`Global`) has no value-table type source and therefore is not judged.

**Measured precondition (fulfilled by 7b.3)**: before 7b.3, every repository-wide production
construction of `BasicBlock.successors` used `Vec::new()` (block-level predecessors were always
empty, so the predecessor-count and predecessor-set criteria were initially exercised through
synthetic fixtures). **From 7b.3 onward, block-level `successors` are populated for real by
`partition_blocks` in `src/middle/ir/ir.rs` at every production construction point** (all six
`lower` sites plus the one codegen-synthesized function body use this sole entry point; `lower`
products include loop and branch blocks with multiple blocks, double edges, and back edges). The
predecessor-count and predecessor-set criteria therefore have data on real IR—direct-driver tests
pin the registration-to-consumption loop on genuine lowering products. `Phi` still has zero
production construction points; its producer belongs to a later batch and will be directly
constrained by these criteria.

7b.1a introduced only the variant: there were zero lowering production construction points (and
there remain zero; production belongs to a later batch). The consumer in `translator.rs` landed in
**7b.3**: under plan A, it is expanded at block entry into a sequence of `Move` instructions (each
group begins at a join block, existing `JMP`s skip between groups, and incoming-edge jumps are
redirected to the group start according to the predecessor). The `Phi` arm of
`translate_instruction` is an unreachable E8001 defensive arm (`executor` never sees `Phi`).

**Handling of the guard issue at `3632-3638`**: the reclamation in the current `generate_block_ir`
has an `if result_reg.is_none()` guard (`3626-3631` comment: “operand-bearing blocks; the outer
scope may hold sibling-argument temporaries that live across blocks”). After SSA-ification, this
guard is **no longer necessary**—temporary values are no longer rolled back, so a sibling argument
that lives across a block holds its own `ValueId` and cannot be affected by a rollback. **This is
one area where SSA-ification genuinely simplifies the code rather than merely renaming it.**

#### Value Identification

**Design judgment**: do not perform memory promotion (mem2reg), do not use virtual-register
numbering, and **perform explicit renaming only**. The reasons are:

- YaoXiang's `&` / reference / ownership semantics make “which slots can be promoted to memory” a
  **type-system problem**, not a CFG problem. It has been verified that `LocalSlot::ty: MonoType` in
  `ir.rs` carries complete type information, but deciding whether a `MonoType` can be promoted
  requires the converged type representation from 03 and coordination with refinement-type rules.
  **Performing mem2reg before 03 is guessing.**
- Explicit renaming requires changing only the return-value semantics of `next_temp_reg()`
  (`next_temp_reg` in `src/middle/lower/scope.rs`) and deleting the three rollbacks; it **does not
  touch the control flow of any lowering algorithm**.

Value-table form (**landed in 7b.1b**): `FunctionBody::Code` gained `values: Vec<ValueInfo>`, where:

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    /// Basic-block label where the definition occurs
    pub defining_block: usize,
}
```

The accessors follow the existing accessor pattern (`impl FunctionIR` in `src/middle/ir/ir.rs`):
`values()` is read-only and returns an empty table for TypeDecl (mirroring `locals()`);
`values_mut()` is mutable and panics for TypeDecl (mirroring `blocks_mut()`).

**Production form (landed in 7b.2)**: `next_temp_reg()` returns `Operand::Value(ValueId)` and
registers a `ValueInfo` entry in the current function body's value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). Item-by-item production form:

- **Counter isolation**: `ValueId` is issued by an independent counter, `next_value: u32`; each
  function body starts at 0, increases monotonically, and is never rolled back. `NestedBodyGuard`
  saves and restores it together with the value table (one set per function body); the three
  statement-level reclamations roll back only `next_temp` (slots) and do not touch `next_value`;
  each curry layer resets them (one `FunctionIR` per layer). Module initialization and top-level
  statements have no value table—when `value_mode=false`, `next_temp_reg` degrades to the old
  `Local` slot allocation, leaving the init sequence unchanged.
- **Type source** (only information already available to lowering; no new type inference mechanism):
  literals use `ConstValue::mono_type()` (converging with the verifier's constant-type mapping into
  a single source); variables and calls obtain types from the type-check result and binding function
  return type at the `generate_expr_ir` entry point; comparisons, logical operations, string
  families, and forward propagation are filled in at function-body finalization according to
  instruction semantics. When unavailable, an **explicit unknown `MonoType::TypeRef("?")`** is used
  (not a silent `Int(64)` placeholder, and `LocalSlot.ty` is never read for this purpose). Across
  232 snapshots, 9,425 values were measured: 7,080 had real types and 2,345 were explicitly unknown
  (24.9%), with unknowns concentrated in call, field, index, and container-construction forms.
- **`defining_block`**: uses the current block label during generation; **block partitioning landed
  in 7b.3** (a real lowering `while` body consists of multiple blocks), but the field still takes
  `cur_block_label` during generation (currently always 0), so it does not yet match the real
  defining block. When 7b.4 regenerated the 232 snapshots, this field was unchanged (the `@block` in
  the `values` section did not drift). Correcting it must happen in the same batch as regenerating
  the `values` section and is **explicitly deferred to the `Phi` production batch**.
- **`debug_name`**: always `None` (named variables remain `Local` and are not migrated to the value
  table).
- **Indexing**: `value_slots: Vec<usize>` has the same indexing as `values` (`ValueId` is the index)
  and records the value-to-frame-slot mapping; from 7b.4 onward, it is recomputed by the linear-scan
  allocator in `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`), and codegen's `OperandResolver` only consumes it; it does
  not allocate.

`values` and `locals` coexist: `locals` carries addressable named variables (which may be defined
multiple times), while `values` carries single-definition SSA values. **The existence of `locals`
does not violate SSA**—it is the “storage” portion outside SSA (LLVM's alloca model).

**Division of responsibility with `LocalSlot::origin`**: these are **two dimensions** and must not
be conflated—`LocalSlot::origin` (the **slot-identity** field introduced on the origin side in
2.1.4, recording the generating frame `FrameId` of that slot) answers “which frame does this slot
belong to,” while `ValueInfo.defining_block` answers “in which basic block is this SSA value
defined?” Value identity belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`;
7b.1b only adds the value table and **does not change `LocalSlot::origin`** (verified as zero
changes).

#### Change-Size Estimate

| Item                                                     | Estimate                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Modify `next_temp_reg`                                   | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Delete three statement-level reclamations                | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Converge six save/restore sites into RAII                | Net ±0 lines                          | See “Per-Item Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Changes to `ir.rs`                                       | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **the first two landed in 7b.1a and the latter two in 7b.1b** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Add `Phi` arm to `translator.rs`                         | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `executor/`                                              | **No changes**                        | Plan A: `Phi` is an IR-only pseudo-instruction expanded by `translator.rs` into a sequence of Moves and never enters bytecode (Decision Three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Call sites changing temporary values to `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (`git grep -c` summed by file, all of `src/`, including tests): 415 `Operand::Local(` occurrences across 28 files, including 332 across 14 files in `src/middle/lower/`; the single production point for temporary values is `next_temp_reg`. **7b.2 measurement (retested against the baseline revision and net landed change)**: 140 `next_temp_reg()` call sites across 14 files, 35 functions with a `result_reg: usize` parameter chain and 3 `Option<usize>` result positions, 342 `Operand::Local(` occurrences within `src/middle/lower/`; the code commit changed `src/` by +23,319/−12,495, of which regenerating 232 IR snapshots accounts for +21,866/−11,950, while source and tests net +1,453/−545 (mechanical replacements and normalize rendering, mono projection, six direct-driver tests; also includes five error-code artifacts committed with the change) |

### Per-Item Disposition Table

| Defect class                                        | Eliminated by SSA?         | Disposition                                                                                                                                                                                                                                                                                                                                                                                                            | Criterion                                                                                                        |
| --------------------------------------------------- | -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore** | **Yes (resolved by 7b.4)** | Seven nested-function-body entry points are consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the counters), turning “forgot to restore” into a compile error; the three statement-level reclamations were removed with 7b.4 because the allocator reuses slots by live interval, and temporary values no longer occupy `next_temp` | First-layer “inner-scope isolation” invariant in 07 + `verify_loose`/`verify_ssa`                                |
| **1b. Inconsistent accounting**                     | **Yes (landed)**           | Unified in `finalize_code_body` in `src/middle/lower/scope.rs`: the table is extracted according to `frame_slots`; the E3014 budget check and table extraction occur at the same point with the function-body span; all six function-body forms are covered; `MAX_REGISTERS` is defined in exactly one place repository-wide                                                                                           | Corpus differential + zero snapshot drift                                                                        |
| **1c. Three statement-level reclamation criteria**  | **Yes (landed in 7b.4)**   | The three reclamations, the `result_reg.is_none()` guard in `generate_block_ir`, and `temp_floor` bookkeeping were removed; slot reuse is delegated to the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                                             | C4: behavioral equivalence (403/403) + invariants                                                                |
| **2. Semantic reordering of `arg_regs`**            | **Partially**              | SSA turns “using the wrong register” into “using the wrong value ID,” which dominance checking can catch, but **the reordering logic itself remains**. Actual elimination requires function decomposition—each of the six branches performs its own `push`, while `arg_regs` ownership remains at the top of the function                                                                                              | First-layer dominance invariant (immune to snapshots)                                                            |
| **3. `Span`-keyed failure**                         | **No**                     | SSA does not touch it. See “Disposition of the `Span`-Keying Problem”                                                                                                                                                                                                                                                                                                                                                  | `test_release_plan_spans_consumed` in 07                                                                         |
| **4. Implicit order dependence**                    | **No**                     | See “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                                              | Guaranteed by code structure and review; the original gate script was canceled under User Ruling D on 2026-11-24 |
| `compile_pattern`, 298 lines                        | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                          | —                                                                                                                |
| `eval_const_expr`, 191 lines                        | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                          | —                                                                                                                |
| 75 `Instruction` variants                           | No (+1, landed)            | `Phi` was introduced by 7b.1a; under plan A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                                     | —                                                                                                                |

### Register Allocator: Custom Implementation or Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **zero construction points repository-wide**—the variant is no longer
   in the `Operand` enum in `ir.rs` (removed by 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper limit of 255 (exceeding it
   reports E3014); `Operand::Value` is resolved by the allocator introduced in 7b.4 through
   `value_slots` to a frame slot (`>255` reports E3014; no mapping reports E3017 with the
   instruction span).
3. `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50 `translate_*`
   functions, with function bodies distributed across `101-1558`) outputs
   `BytecodeInstruction::new(opcode, operands)`, where opcodes come from
   `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter's opcode-dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine has **no finite register file, no register pressure, and no need for
spilling or splitting**.

**Evaluation of three approaches**:

| Approach                                           | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both use rustc's `Function` trait and its MIR `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait (CFG, liveness, spill slots, scratch), requiring **500+ lines of adapter code for zero benefit**—there is no real register file to color. Moreover, rustc itself uses only linear scan at this point (when `enable-llvm` is disabled)                                                                                                              |
| `graph-alloc` family                               | **Not adopted.** Same reason, with an even less stable API                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| **Custom linear scan (simplified Chaitin–Briggs)** | **Adopted (landed in 7b.4).** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot according to live intervals (`OperandResolver` already has a `u8` contract and the E3014 limit). Implemented in `src/middle/lower/regalloc.rs`: block-level `successors` backward dataflow, closed intervals, Phi predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`, with overflow returning `Budget` for `finalize_code_body` to report E3014 with the declaration span |

**Landing measurements (7b.4)**: the two corpus entries that previously exceeded the limit (354 /
323 monotonic slots) now use **15 / 24 frame slots** after allocation (344 / 304 values), while the
other 403/403 corpus entries remain behaviorally equivalent; E3014 now means only “frame slots after
allocation exceed 255.” The fallback counter for back edges is currently not exposed (`wrap_count`
is computed and discarded); the `Phi` production batch must expose and retest it first (a
prerequisite alongside `defining_block`). **Overflow handling**: the `u8`/255 limit remains an error
handled by the budget gate—this article does not solve “a large function does not have enough
registers” (a downstream consequence of `compile_pattern` recursion depth).

The dev-dependencies at `Cargo.toml:37-39` contain only `criterion` / `proptest`; `[dependencies]`
contains no regalloc / petgraph / graphlib—introducing any crate would add a new dependency surface.

### Decomposition of `generate_call_expr_ir` (Landed in 7d.1)

Before decomposition, it was a single 920-line function (746 lines at audit time; 920 lines after
moving to `middle/lower/expr/call.rs`) handling six call forms. The landed form is:

```
generate_call_expr_ir（call.rs:479-568，函数体 78 行，只做分派）
├── 命中即返回，按序：变体构造 → 命名空间调用 → 跨文件全局槽位间接调用
│                    → 方法调用重排 → 内置容器构造 → 普通/重载调用
├── CallArgs（call.rs:31-271）——实参收集 / 命名重排 / 槽位绑定的唯一载体
│    collect_regs / bind_named_slots / collect_regs_in_slot_order / arg_expr
│    槽位判定仍唯一归 sema::inference::arg_binding::bind_args_to_params
├── emit_variant_ctor              call.rs:575-621    （RFC-010 变体构造）
├── emit_namespace_call            call.rs:631-684    （命名空间调用）
├── emit_global_slot_indirect_call call.rs:695-717    （跨文件全局槽位 → CallDyn）
├── emit_method_call_reordered     call.rs:728-853    （RFC-004 绑定重排）
├── emit_builtin_container_ctor    container.rs:175-220（RFC-011 §Vec 容器构造）
└── emit_plain_call                call.rs:864-1457   （无绑定方法 + 函数形态）
```

**Two corrections to assumptions in the earlier draft**: ① The original design assumed that “SSA had
reduced `arg_regs` to one `ValueId` per argument, so the six branches would only push and not
reorder.” That premise does not hold in the current tree; C1 requires order preservation, so the
implementation retains three reordering paths (`set_regs` / `collect_regs_in_slot_order` /
`bind_named_slots`), all consolidated into `CallArgs` rather than scattered across branches. ② The
original names `emit_bound_call` / `emit_struct_ctor` / `emit_curried_call` / `emit_method_call` do
not match the six landed names; this table is authoritative.

**Consolidated inline copies** (eliminated during the split, not copied unchanged):
positional-argument evaluation 5→1, named-argument slot insertion 2→1, reconstruction in slot order
2→1, and positional/named index conversion 4→1.

**Assertion-based handling of “fill with 0” in 7d.2**: the three equivalent `unwrap_or_else`
branches that inserted `Load 0` now return the existing `E3005`
(`ErrorCodeDefinition::ir_internal_error`). The carrier is `CallArgs::require_complete_slots`, not
the draft's `fill_missing`; the **only retained** zero fill is the legal zero initialization for the
empty construction `Pair(Int)()`, required by SPEC §4.3 and asserted by the corpus entry
`generic_constructor.yx` (`z.first == 0`).

**Criteria**: the pure-move portion has C1 zero-diff snapshots and byte-for-byte equality across 412
corpus entries; the assertion-based portion uses C4 (four new direct-driver cases, red with the old
implementation and green with the new one).

### `synth.rs` Boundary: Turning Implicit Callbacks into an Explicit Synthesized-AST Boundary

**Landed form**: `src/middle/lower/synth.rs` is the only module in L3/L4 production code permitted
to construct `ast::Expr` / `ast::Stmt` / `ast::Type`. Pattern matching, read-only queries, and test
fixtures are not synthesis. The current production synthesis points are spawn's Lambda/Stmt and
spawn-for's Lambda, all routed through this module.

AST nodes do not carry the lowering environment, so `SynthEnv` is not an AST field, and AST helpers
do not pretend to consume it. It contains runtime environment operands and capture names, and the
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

The environment flow is as follows:

- spawn and spawn-for construct `SynthEnv` and pass it explicitly into lowering with the synthesized
  Lambda; they no longer prewrite `pending_env_vars` / `pending_env_names`. The total environment
  slot count is derived from `env_vars.len()`, and named upvalues are offset from the
  anonymous-prefix length; therefore spawn-for's anonymous iteration value, outer captures, and
  parameters each occupy their intended positions.
- Ordinary lambdas complete the environment using the existing free-variable collector. List
  comprehensions, nested Lambdas, and nested spawns all traverse the same collector; generator
  variables and nested parameters are excluded according to lexical scope.
- Closure bodies register `SynthEnv` as `BindingLocation::Upvalue` in the existing symbol table.
  Variable reads and namespace-shadowing queries share this lexical binding and no longer read
  `closure_captures`.
- Nested closures use `capture_operand` to read the outer upvalue first, then pass the value to the
  inner environment; an outer local slot with the same name is never mistaken for a capture.
- `NestedBodyGuard` records the symbol-table depth on entry, and exit—including early return through
  `?`—truncates it to that depth, so capture bindings do not leak into later unrelated lowering.
- Sema's expression-body function check parameterizes the existing inputs with
  `body_stmts + expression_body + span`, directly calls `check_expr`, and reuses the tail
  validation; it no longer synthesizes a Block/Stmt/Expr and introduces no reverse dependency from
  sema to middle/lower or additional architectural exception.
- SpawnFromList shares the `env + [anonymous item]` argument layout between Embedded and Standard
  and injects the same environment as upvalues. The anonymous iteration value is fixed at env[0],
  named captures are read after the anonymous prefix, and parameters start after the complete
  environment.
- Dual-runtime behavior tests explicitly invoke main() and assert, within a spawn-for task, both the
  no-capture item value and the item-plus-outer-capture value; the no-capture form also covers the
  regression restoration introduced by 74febd1f. IR assertions and actual execution are recorded
  separately, and zero drift across the existing 403 corpus entries is not presented as behavioral
  coverage of this form.

**Criteria**: D22's synthesized-AST boundary is guaranteed by code structure—L3/L4 production code
constructs `ast::Expr` / `ast::Stmt` / `ast::Type` only in `src/middle/lower/synth.rs`; pattern
matching, read-only queries, and test fixtures are not synthesis; synthesis points outside
`synth.rs` are rejected by review. The original `scripts/ci/check-synth-boundary.py` and the CI
`synth-boundary` job were removed under User Ruling D on 2026-11-24; heuristic token scanning is no
longer maintained because it cannot cover arbitrary legal use / alias / re-export forms—successive
attempts to close gaps were still bypassed by `pub type` re-exports—and its maintenance cost exceeds
its benefit.

D1/D2: `SynthEnv` and `BindingLocation` are new internal types introduced by this batch, not reuse
of existing types. The former centralizes production-side environment operands, named items, and
slot derivation as one explicit parameter. The latter promotes the former `SymbolEntry.local_idx`
into a distinguishable Local/Upvalue location. The alternatives—continuing to pass three shared
fields, which cannot prevent nested overwrites and erroneous residue, or passing only
`Vec<Operand>`, which cannot express the correspondence between the anonymous prefix and named
captures—were rejected for correctness reasons. The new concepts are not synonymous with existing
AST or consumption-ledger concepts and require no import aliases or conversion tables.

### Disposition of the `Span`-Keying Problem (the Part SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism One: Consumption-Coverage Assertion (Mandatory, Lowest Cost; Replaced by a Stronger Form
on 2026-10-13).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). This
article's original supplement proposed making it a **runtime count** rather than merely a test. The
landed form is stronger than that proposal—a **consumption ledger (`ConsumptionLedger<K>`) + a
single settlement point (E-level `W1090`)**—without adding a new diagnostic code or relying on an
internal ir_gen counter to prove itself:

```rust
// Near ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each hit at :1942, self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* New diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a value that should have been dropped was not dropped” may not immediately
crash under YaoXiang's current value semantics and Arc/Rc model**—which is precisely why it was
silent. Counting turns “silent” into “detectable.”

**Mechanism Two: Change `Span` Keys to `DefId` or Explicit Plan IDs (**Landed on 2026-10-13**:
RFC-039 D20 / WBS 4.5 implemented this mechanism as a `PlanId(u32)` structural-identity key +
mandatory consumption settlement; see the revision note in §2 of [02](02-stage-contract.md)).** The
fundamental fix is to **not use `Span` as a key**. `Span` is a source location and may drift due to
macro expansion, `include`, or multi-file merging. `FunctionIR` already has a `def: Option<DefId>`
field (`ir.rs:677`), demonstrating that DefId is available at the IR layer. **Design judgment**:
introduce `PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, assign it when
ownership/overload decisions are produced, and match it by `PlanId` when the IR consumes the plan.
**This belongs to the obligation-ledger (`Obligations`) scope of 02 and should be implemented
together with [02](02-stage-contract.md), not in this article.**

**Mechanism Three: Definition-Order Dependence of `method_def_ordinals` (Independent Minor Item,
**Landed on 2026-11-25**).** The field was originally a `HashMap<String, usize>` whose key was the
module-unqualified `"{type_name}.{method_name}"`, assigning `#N` disambiguated names according to
definition traversal order. If type-check registration order differed from lowering definition
order, methods with the same name could resolve to different targets. **Landed form**: at the same
point where it registers overload candidates, type checking **produces**
`method_def_ir_names: HashMap<PlanId, String>` once (a bare name or a `#N` disambiguated name) and
passes it to IR generation through `TypeCheckResult`. The key is the **method declaration
statement's `PlanId`** (AST structural identity, the same model as D20)—`DefId` is not assigned
until lowering's final `assign_defs`, so it does not exist on the type-check side. Lowering only
reads the name according to declaration identity, and **a miss reports E3005 loudly** (it does not
fall back to the bare name, because that would disguise “the definition name and call name have
different sources” as a runnable program). The mutable counter and its write site have been removed,
and definition names no longer have a second derivation entry point.

### Tracking of Hard-Coded Discards

`impl From<BytecodeFile> for BytecodeModule` (now `src/middle/bytecode/mod.rs`) originally had three
hard-coded discards; after the 7e review, there were **two genuine losses + one false positive**:

| Location (old line number) | Field                | Original value                                 | Review conclusion                                                                                                                                                                                                                                                                                                          |
| -------------------------- | -------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312`         | `upvalue_count`      | `0` // Not stored in BytecodeFile              | **Genuine loss**: the closure upvalue slot count became zero after a `.42` round trip; persisted with v5 (7e.1)                                                                                                                                                                                                            |
| `bytecode.rs:2315`         | `exception_handlers` | `Vec::new()` // Not implemented yet            | **False positive**: the field has no producer or consumer, and the language layer has no throw/try/catch (RFC-030 records `Throw` as reserved at the bytecode layer, and RFC-036 rejects an in-process catch boundary) ⇒ there is no “lost data”; it is a dead field and is not persisted. **Removed (6753bbe, P10 S6.1)** |
| `bytecode.rs:2341`         | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **Genuine loss**: the global table (names + types) became zero after a round trip; persisted with v5 (7e.3), with the carrier and function signature both using `MonoType` (7e.4)                                                                                                                                          |

**Criterion (Decision D52)**: an empty table with no producer is not lost data; it is a dead
path—the same criterion as D32. Only a table with a producer whose container is not persisted
represents data loss. Therefore, `2312` / `2341` were included in P7 batch e (7e); `2315` is not
persisted and was **removed as a dead field with P10 S6.1 (6753bbe)** (`THROW` remains a separate
pending decision). A single version bump covers the two genuine losses (`VERSION` 4→5, D17); `.42`
is a build artifact, and cross-version compatibility is not a goal.

**A third format distortion in the same 7e batch**: the `.42` type-number table is irreversible and
has collisions—`Int(64)` and `Float(32)` are both `9`, while `Float(64)` is read back as
`TypeRef("_")`. The number table was changed to be reversible and collision-free (7e.4), and the
writer now rejects unknown bit widths and compile-time FFI types loudly.

## Detailed Design

### Knock-on Effects of IR Structure Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                  | Nature                                                                                                   |
| ------------------------------------------------------------------------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Add `ValueId(u32)` type                                                   | File header (`ValueId` definition)        | New (**landed in 7b.1a**)                                                                                |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                 | **Variant additions/removals** (**landed in 7b.1a**)                                                     |
| Add `Phi` variant to `Instruction`                                        | End of `Instruction` enum (`Phi` variant) | **+1 variant** (**landed in 7b.1a**; the enum now has 75 variants)                                       |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                      | **+1 field** (**landed in 7b.1b**)                                                                       |
| Add `ValueInfo` structure                                                 | `ValueInfo`                               | New (**landed in 7b.1b**)                                                                                |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Same-named accessors                      | Add `values()` / `values_mut()` accessors (**landed in 7b.1b**, following the existing accessor pattern) |

The file-header `pub use ...ast::Type` is **not changed by this article**—it belongs to the scope of
[03](03-type-unification.md). However, `ValueInfo::ty` must use the **converged** single type
representation.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                          | Location                              | Nature                                                                                                |
| ------------------------------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or use the “expand at runtime into Move” strategy | `127-554` (currently **66** variants) | +1 or +0 variants (see below)                                                                         |
| Add `Phi` arm to `opcode()`                                                     | `558`                                 | Required (`BytecodeInstr` exhaustive match)                                                           |
| Add `Phi` arm to `size()`                                                       | `649`                                 | Required                                                                                              |
| Add `Phi` arm to the `From<BytecodeFile>` decode match                          | `943-2350`                            | Required                                                                                              |
| Correct `upvalue_count: 0`                                                      | `2312`                                | Fixed incidentally by this article (in-memory format; persistence requires a version bump, see below) |

**The bytecode form of `Phi` requires a design decision**. There are two options (**design judgment:
recommend A**, with reasons under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded by `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  selecting by predecessor block index. **No new opcode is added**, and the `.42` format remains
  unchanged. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode + predecessor-index encoding, with the interpreter maintaining register
  versions at jumps. **This requires changing the `executor/` state model + the `.42` format + the
  version number.** The only benefit is size.

The decisive reason for choosing A: changing the `.42` format triggers version-number issues and
full compatibility discussions, while **A's size cost is irrelevant in interpreted execution** (the
interpreter executes Moves, not Phis).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions, distributed across `101-1558`)

| Change                                             | Location                                                                                                                         | Nature                                                                                  |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                       | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                                 |
| `Instruction::Phi` translation arm                 | Dispatch match, before `Free` (533)                                                                                              | New branch that expands into a Move sequence (Plan A)                                   |
| Existing nine NOP degradations **remain in place** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as nine**: `655-657` covers two variants, and `658-660` covers four variants |

**SSA-ification does not reduce the NOP count.** These nine degradations reflect gaps between IR
expressiveness and bytecode expressiveness, and SSA-ification does not touch them.

#### `src/backends/interpreter/executor/`

With Plan A, `executor/` requires **no changes**—`Phi` disappears during the `translator.rs` stage
and is never visible to the interpreter. **This is the second decisive reason for choosing A.**

`BytecodeInstr::Switch` has an active implementation at `executor/ops/control.rs:90`, but **no**
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs` (all three names have zero matches in
`ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions that the IR layer cannot
produce. It does not affect this article, but is recorded here because it demonstrates that “IR →
bytecode” is not surjective.

The dispatch match in `executor/debug.rs:194` likewise requires no changes (there is no new opcode
under Plan A).

### Compiler Change List

**File by file and function by function.** All line numbers are the starting points for the changes
(verified by measurement):

| No. | File                                                                      | Location                                                                            | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                                                     | `Operand` enum definition                                                           | Add/remove `Operand` variants (**landed in 7b.1a**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 2   | `src/middle/ir/ir.rs`                                                     | End of `Instruction` enum (`Phi` variant)                                           | Add `Instruction::Phi` (**landed in 7b.1a**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 3   | `src/middle/ir/ir.rs`                                                     | `FunctionBody::Code`                                                                | Add `values` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 4   | `src/middle/ir/ir.rs`                                                     | `ValueInfo`                                                                         | Add `ValueInfo` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 5   | `src/middle/ir/ir.rs`                                                     | `values()` / `values_mut()` accessors                                               | Add accessors (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 6   | `src/middle/ir/ir.rs`                                                     | `pub use ast::Type`                                                                 | **Do not change `pub use ast::Type`** (scope of [03](03-type-unification.md); recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 7   | `src/middle/core/ir_gen.rs`                                               | `884-891` `next_temp_reg`                                                           | Return `Operand::Value`, increasing monotonically without rollback (**landed in 7b.2**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; a separate `next_value` counter was added, starting at 0 per function body, and the three statement-level reclamations roll back only `next_temp`, not it)                                                                                                                                                                                                    |
| 8   | `src/middle/core/ir_gen.rs`                                               | `1954-1959`                                                                         | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 9   | `src/middle/core/ir_gen.rs`                                               | `3632-3638`                                                                         | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 10  | `src/middle/core/ir_gen.rs`                                               | `4737-4742`                                                                         | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 11  | `src/middle/core/ir_gen.rs`                                               | `1745-1759` / `1819-1822`                                                           | Replace with RAII guard (`generate_method_ir`, entry `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 12  | `src/middle/core/ir_gen.rs`                                               | `1858-1866` / `2026-2029`                                                           | Replace with RAII guard (`generate_function_ir`, entry `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 13  | `src/middle/core/ir_gen.rs`                                               | `2116` / `2180`                                                                     | Replace with RAII guard (`generate_curry_innermost_func`, entry `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 14  | `src/middle/core/ir_gen.rs`                                               | `2224-2229` / `2284-2288`                                                           | Replace with RAII guard (`generate_curry_function_ir`, entry `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 15  | `src/middle/core/ir_gen.rs`                                               | `2768-2777` / `2820-2851`                                                           | Replace with RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 16  | `src/middle/core/ir_gen.rs`                                               | `4686-4695` / `4767-4774`                                                           | Replace with RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 17  | `src/middle/core/ir_gen.rs`                                               | `1987-2000`                                                                         | Extract the E3014 check into `check_register_budget()`, shared by `1798` / `2001` after their accounting is unified                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 18  | `src/middle/core/ir_gen.rs`                                               | `1798`                                                                              | Change `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)` to eliminate inconsistent accounting                                                                                                                                                                                                                                                                                                                                                                                                               |
| 19  | `src/middle/core/ir_gen.rs`                                               | `248-249`                                                                           | Remove the contract comment (now enforced by the type system) and replace it with a pointer to `verify`                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 20  | `src/middle/lower/loops.rs`                                               | `generate_spawn_for_ir`                                                             | Change to call `synth::synth_closure` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 21  | `src/middle/lower/expr/spawn.rs`                                          | `generate_spawn_expr_ir`                                                            | Change to call `synth::synth_closure`; remove writes/clears of shared pending fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 22  | `src/middle/lower/expr/closure.rs`                                        | `generate_lambda_expr_ir`                                                           | Read `SynthEnv` from the arguments instead of `mem::take`-ing shared fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 23  | `src/middle/lower/{mod,scope}.rs`                                         | Symbol table and `NestedBodyGuard`                                                  | Remove the `closure_captures` field; use `BindingLocation::Upvalue` lexical bindings and depth-based restoration (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                    |
| 24  | `src/middle/lower/expr/{access,closure}.rs` + `query.rs`                  | Variable and namespace queries                                                      | Replace `closure_captures.contains_key` with a lexical-upvalue lookup; materialize captures uniformly through `capture_operand` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                     |
| 25  | `src/middle/core/ir_gen.rs` (audit-time path)                             | `341` + `1942`                                                                      | ~~Add `Span`-keyed consumption counting~~ → covered by the 4.5 `PlanId` consumption ledger (read and accounting in one operation, settled by W1090); 7c.2 verified zero triggers, with no code change                                                                                                                                                                                                                                                                                                                                    |
| 26  | `src/middle/lower/{mod,stmt}.rs`, semantic side `sema/{checker,types}.rs` | Definition names for `generate_method_ir` + authoritative `TypeCheckResult` mapping | Make `method_def_ordinals` read-only (**landed in 7c.3**: type checking produces disambiguated names from declaration `PlanId`, and lowering only reads them)                                                                                                                                                                                                                                                                                                                                                                            |
| 27  | `src/middle/lower/expr/call.rs` + `container.rs`                          | Audit-time `ir_gen.rs:7269-8014`                                                    | Decompose into `CallArgs` + six `emit_*` functions (**landed in 7d.1**: `eb13cac`; the main function `call.rs:479-568` only dispatches)                                                                                                                                                                                                                                                                                                                                                                                                  |
| 28  | `src/middle/lower/expr/call.rs`                                           | Audit-time `ir_gen.rs:7838-7846`                                                    | Change the “fill with 0” fallback to return the existing `E3005` (**landed in 7d.2**: `e246d6d`; legal empty-construction zero values remain)                                                                                                                                                                                                                                                                                                                                                                                            |
| 29  | `src/middle/lower/synth.rs`                                               | **New**                                                                             | Synthesized-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 30  | `src/middle/ir/verify.rs`                                                 | **New** (first layer of [07](07-equivalence-oracle.md))                             | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 31  | `src/middle/passes/codegen/translator.rs`                                 | Dispatch match                                                                      | Translate `Instruction::Phi` (**landed in 7b.3**: Plan A expands it at **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit a Move group per predecessor, existing `JMP`s skip between groups, and incoming-edge jumps are redirected to the group start according to `(predecessor, join-block first instruction)`; the `Phi` arm of `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return slot cannot hold multiple Moves distributed across predecessors, so reaching it is a loud failure) |
| 32  | `src/middle/passes/codegen/operand.rs`                                    | `36-45` / `72-77`                                                                   | Add the `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`) (**landed in 7b.2**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); removal of the `Temp` arm landed first in 7b.1a)                                                                                                                                                                                                                                        |
| 33  | `src/middle/core/bytecode.rs`                                             | `127-554`                                                                           | **Unchanged under Plan A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 34  | `src/middle/core/bytecode.rs`                                             | `558` / `649` / `943-2350`                                                          | **Unchanged under Plan A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 35  | `src/middle/core/bytecode.rs`                                             | `2312`                                                                              | Correct `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 36  | `src/backends/interpreter/executor/**`                                    | —                                                                                   | **No changes under Plan A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 37  | `src/middle/lower/regalloc.rs`                                            | **New**                                                                             | Linear-scan allocator (**landed in 7b.4**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)` returns `value_slots` and `frame_slots = named_base + max_live`; overflow returns `Budget` for `finalize_code_body` to report E3014)                                                                                                                                                                                                                                                                             |
| 38  | `scripts/ci/check-synth-boundary.py`                                      | **Canceled**                                                                        | The boundary sentinel script was canceled under User Ruling D on 2026-11-24; it is not created                                                                                                                                                                                                                                                                                                                                                                                                                                           |

### Backward Compatibility: Version Number for the `.42` Format

**Verified facts**: the `.42` persisted format **already has a version-number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (`"YXBC"`)
and `VERSION: u32 = 4`, writes them to the header (`:342-343`), and validates them when reading
(`:454-467`, reporting "unsupported bytecode version" on a mismatch).

**Design judgment (a direct consequence of Plan A)**:

- `Phi` is expanded into a sequence of `Move` instructions during the `translator.rs` stage and
  **produces no new opcode**.
- The 83 constants in `opcode.rs` undergo **zero changes**, `opcode_name()` (`120`) remains
  unchanged, and the decode match (`bytecode.rs:943-2350`) remains unchanged.
- The `.42` field layout undergoes **zero changes**, and `VERSION` remains 4. Old `.42` files can
  still be read by the new binary, and new `.42` files can still be read by the old binary.
  **SSA-ification itself does not require a version bump.**

**The only scenario requiring a version bump is correcting `2312 upvalue_count`** (Decision D17: fix
it, and handle it with the version bump): adding an `upvalue_count` field to `BytecodeFunction` and
having the encoder write it is a **new format field**, so `VERSION` is raised from 4 to 5. The
existing reader-side version check rejects old files according to existing behavior—`.42` is a build
artifact (`main.rs:671`), and cross-version read compatibility is not a goal, so the version bump
requires no additional migration mechanism. The allocated but unused opcode values (`Switch` /
`TailCall` under D32/D34) are reclaimed with the version bump.

## Implementation Notes

### Batch Strategy

There are four batches. **Every item within a batch gets an independent commit, independent
criterion, and independent revert.** “Change item” in the table below refers to the corresponding
number in “Compiler Change List.”

| Batch       | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Change items                 | Criterion                                                                                                                                                            | Rollback point                                                                                                                                                  |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Break multiple definitions: delete the three statement-level reclamations → replace six save/restore sites with an RAII guard → unify accounting                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 8–10 (first) → 11–16 → 17–18 | Deleting reclamations: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and accounting unification are pure moves: **C1** zero-diff snapshots | One commit per item; `git revert` rolls it back                                                                                                                 |
| **Batch b** | Switch to the SSA shape: `ir.rs` structure changes + temporary values produce `Operand::Value` (mechanical replacement) → `Phi` arm in `translator.rs` (Plan A) → linear-scan allocator. Within the batch, WBS level-three tasks run serially: 7b.1a (`Operand` variant convergence + introduction of `Instruction::Phi`, **landed**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **landed**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **landed**) → 7b.3 (`Phi` arm in `translator.rs` + population of `BasicBlock.successors`, **landed**) → 7b.4 (linear-scan allocator + deletion of the three statement-level reclamations, in the same batch as 7a.1, **landed**) | 1–5, 7 → 31–32 → 37          | **C4**: behavioral equivalence + all `verify_ssa` checks green + `.42` round-trip tests                                                                              | The `ir.rs` change **must occupy one commit by itself** (the type change affects compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary → verify the existing consumption ledger (no code change) → make `method_def_ordinals` read-only (the sentinel script was canceled under User Ruling D on 2026-11-24)                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 20–24, 29 → 25 → 26          | **C4**: behavioral equivalence; the consumption count triggers zero new diagnostics (otherwise a real mismatch exists)                                               | Independent commit                                                                                                                                              |
| **Batch d** | Decompose `generate_call_expr_ir` + make the “fill with 0” fallback assertion-based (**landed**: 7d.1 `eb13cac`, 7d.2 `e246d6d`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 27–28                        | Pure-move portion: **C1** zero-diff snapshots. Assertion-based portion: **C4** corpus differential + first-layer verifier                                            | Independent commit                                                                                                                                              |

**Three ordering dependencies cannot be exchanged within the batches:**

1. Batch a's “delete reclamations” comes after batch b's `ir.rs` change (the three statement-level
   reclamations were removed with 7b.4 and the linear-scan allocator in the same batch, with 7a.1
   executed alongside it)—`next_temp` must increase monotonically first, before `ValueId` can have
   the semantics of “single definition” (which becomes effective only after 7b.2 changes
   `next_temp_reg` to produce `Value` and 7b.3 lands the `Phi` arm).
2. Batch b's `Phi` arm must precede batch d's decomposition—otherwise the decomposition would simply
   copy the old `arg_regs` ordering assumptions into the new file.
3. Batch d must come last—it depends on the preceding batches having converged `arg_regs` semantics
   to “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within **P7** of RFC-039. The criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial** relative to
one another, but within batch a, only ordering constraints—and no coupling—exist among 8–10 / 11–16
/ 17–18.

### Hard Preconditions

**Precondition One: Criterion Baseline.** The first-layer verifier's `verify_loose` mode from
[07](07-equivalence-oracle.md) **must first pass on the existing non-SSA IR** before batch a may
begin.

Reason: the SSA-ification criterion is C4 (behavioral equivalence + invariants). If the invariant
verifier itself cannot pass on the existing IR before the transformation, then every acceptance step
in batches a–d lacks an executable criterion and can rely only on corpus differentials—yet corpus
differentials are **precisely insensitive** to defect classes 1/2/3 (as demonstrated by “Alternative
A” in 07). **This is a real ordering constraint.**

**Precondition Two: Type-Representation Convergence.** [03](03-type-unification.md) must complete
the convergence of `ir.rs:3`'s `pub use ast::Type` and remove the `From<MonoType> for IrType` bridge
at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` would introduce a third set of
type-compatibility checks.**

### Rollback Strategy

- Every change is an independent commit, with `git revert` granularity equal to change-item
  granularity.
- Batch b's `ir.rs` structure change is the **only part that cannot be rolled back incrementally**
  (the type change affects compilation throughout the repository). It must therefore **occupy a
  commit by itself**, and batch a must already have passed and established a revertible baseline
  before it.
- **Do not use a feature flag.** Reason: adding/removing `Operand` variants cannot be isolated with
  a runtime switch; a feature flag would cause CI to stop testing the other path over the long
  term—a replay of the “test wiring depends on human memory, and corruption occurs silently” problem
  recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front; otherwise the implementation will be challenged midway with “the
line count did not decrease.”**

| Item                                      | Line-count change                                                                                                                                                                  |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                            | `ir_gen.rs` **8,448**                                                                                                                                                              |
| Delete three statement-level reclamations | **−15**                                                                                                                                                                            |
| Replace six save/restore sites with RAII  | Net ±0 (the RAII guard definition is about +20 lines, while use sites fall from 22 lines to about 12)                                                                              |
| Modify `next_temp_reg`                    | +5                                                                                                                                                                                 |
| Decompose `generate_call_expr_ir`         | **+40–80** (signature and parameter-passing overhead for six `emit_*` functions; this is the typical cost of a pure-move decomposition—**decomposition increases the line count**) |
| `Span`-keyed consumption counting         | +15                                                                                                                                                                                |
| Make `method_def_ordinals` read-only      | +20                                                                                                                                                                                |
| New `synth.rs`                            | +80–120                                                                                                                                                                            |
| New `verify.rs`                           | +400–600 (first layer of 07; within this article's scope but counted separately)                                                                                                   |
| New `regalloc.rs`                         | **+500–1,000** (measured at 271 lines, 7b.4)                                                                                                                                       |
| **Total**                                 | **`ir_gen.rs` approximately 8,448 → 8,600–8,800; including new files, net increase of 900–1,600 lines**                                                                            |

**The early expectation of “8,448 → 5,000–6,000” was wrong**; the measured projection produced the
opposite result. The reason must be explained:

1. **The core benefit of this article is not fewer lines; it is eliminating an entire class of
   defects that “are wrong without reporting an error.”** Six manual save/restore sites (22 lines)
   become about 12 lines plus a guard type after SSA + RAII—**the line count is almost unchanged,
   but “forgot to restore” changes from a possibility into a compile error**.
2. **A pure-move decomposition necessarily increases the line count.** Decomposing the 746-line
   `generate_call_expr_ir` into seven functions requires repeating signatures (the function already
   has `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` bindings, and error propagation.
   **“Splitting the file reduces the line count” is a misconception—it reduces cognitive load, not
   physical lines.**
3. **The new verifier and allocator increase the line count.** These two additions (approximately
   900–1,600 lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of lines, this article will most likely produce a **net increase**. The benefit lies in
> **verifiability**—turning defects that are “wrong without reporting an error” into invariants that
> “fail with an error.” If line count is used as the acceptance criterion, this article will fail;
> the correct acceptance criteria are those in “Batch Strategy.”

**This stage will intentionally increase the amount of code in `ir_gen.rs`.** The 2026-10-03
decision canceled all line-count and size gates (see Prohibition Two in
[08](08-maintenance-mechanism.md)), so **no line-count exemption list is necessary**—the increase is
justified by separation of responsibilities: SSA-ification extracts “register allocation +
definition discipline” from lowering into independent modules, trading increased code size for
elimination of the manual discipline in six manual save/restore sites and the `arg_regs` semantic
reordering—two classes of “misalignment reports no error and merely produces the wrong value.”

## Key Decisions and Rationale

### Three Core Decisions

**Decision One: Temporary-Value Strategy—Delete the Three Statement-Level Rollbacks So That Single
Definition Becomes a Mathematical Consequence of Construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, so every temporary slot naturally has exactly one definition point. **The
criteria are consistent but the guards differ** (`3632` has an `result_reg.is_none()` guard, while
the other two do not), and such hand-copied differences disappear as a result. The cost is increased
slot occupancy—the existing 255 upper-limit check on `temp_high_water` (`1992-2000`) provides a
backstop.

**Decision Two: Do Not Perform mem2reg; Perform Explicit Renaming Only.** The reasons were given
under “Value Identification”: YaoXiang's `&` / reference / ownership semantics make “which slots can
be promoted to memory” a type-system problem that requires the converged type representation from
[03](03-type-unification.md). **Performing mem2reg before 03 is guessing.** Explicit renaming
requires changing only the return-value semantics of one `next_temp_reg` call and deleting the three
rollbacks; it does not touch the control flow of any lowering algorithm.

**Decision Three: Use Plan A for `Phi` (IR-only pseudo-instruction, expanded into a sequence of
Moves).** `executor/` and the `.42` format undergo **zero changes**, confining the regression
surface to L3. The decisive reason is that changing the `.42` format would trigger version-number
issues and full compatibility discussions, while Plan A's size cost is irrelevant under interpreted
execution (the interpreter executes Moves, not Phis). The cost is that `Phi` is not visible in
`dump_bytecode` (when entering the C1/C2 stages, 07's third layer must account for this).

### Rejected Alternatives

**A. Decompose Everything at Once (SSA + decomposition + `synth` boundary + allocator
simultaneously). Rejected.** This violates 07's core principle: criteria must be tiered. Mixing four
batches into one PR mixes pure-move changes with C4 changes, so snapshot drift cannot be
attributed—when a snapshot changes, it is impossible to tell whether the decomposition moved
something incorrectly or whether SSA changed it incorrectly. **Rollback granularity degrades from
“change item” to “everything.”**

**B. Split the File First, Then Optimize Ordering. Rejected.** Splitting does not eliminate any root
cause: the six manual save/restore sites remain scattered across 10 files, and `arg_regs` semantic
reordering still requires manual reasoning. Worse, **splitting first loses the opportunity to
“establish criteria before making changes”**—the split produces a large line-level diff that mixes
into subsequent SSA diffs and invalidates review. This has the same root cause as the eight orphan
test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests that never ran): **with the
wrong ordering, every subsequent step accelerates work based on an incorrect foundation.**

**C. Introduce an Intermediate SSA Layer Without Changing Existing Lowering. Rejected, but recorded
as “a third path once considered.”** The form would retain `ir_gen.rs` unchanged and add a pass that
converts its non-SSA IR into SSA IR. **The advantage** is that lowering remains untouched and
rollback is easy. **Reasons for rejection**: (1) It does not eliminate the root cause of defect
class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the six save/restore sites
remain separated by 59–161 lines. A conversion pass could merge multiple definitions into `Phi`, but
it **cannot fix “an inner function's name leaks into the outer function”**—that is a `cur_locals`
save/restore error occurring before the IR shape, invisible to the conversion pass. (2) It turns SSA
into a second representation—the repository already has the cautionary example of three parallel
type representations—and this project's core lesson is that **“boundaries exist only in people's
awareness.”** (3) `verify_loose` would become a permanent burden.

**D. Keep the Current Implementation and Only Add a Verifier. Rejected as the sole approach, but it
is a component of this article.** RFC-039 Alternative B reaches the same conclusion: a gate can
prevent regression but cannot repair the current implementation. **However, D's capabilities must be
stated precisely**: D can **catch instances** of defect classes 1/2/3 (provided the verifier covers
dominance, type consistency, and inner-scope isolation), but it cannot catch defect class 4
(`closure_captures` leakage)—that requires a cross-function liveness assertion beyond the boundary
of `verify(&ModuleIR)`. **D is therefore not an alternative to this article; it is a prerequisite
for it.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not the risk
  point**—the 2026-10-03 decision canceled line-count and size gates, and the scale concern is
  resolved through separation of responsibilities (see Prohibition Two in
  [08](08-maintenance-mechanism.md)). The increase is justified because it replaces the manual
  discipline in six manual save/restore sites and the `arg_regs` semantic reordering—two classes of
  “misalignment reports no error and merely produces the wrong value.”
- **The “single definition” check in `verify.rs` is meaningless in the non-SSA shape**
  (`Operand::Local` permits multiple definitions)—this limitation is recorded in 07, and
  `verify_loose` is its degraded mode before this batch.
- **Deleting statement-level reclamations changes the temporary-slot allocation pattern** (executed
  in 7b.4, alongside 7a.1): after deletion, slots are reused by the linear-scan allocator according
  to live intervals, so **allocation results change**—this is precisely why C4 must run against the
  complete corpus; the measured 403/403 differential was empty.
- **`u8` / 255 slot limit**: the two corpus entries that exceeded the limit after reclamation was
  removed (`string_ops.yx` with 354 monotonic slots and `list_comp.yx` with 323 monotonic slots) are
  compressed by the allocator to **15 / 24 frame slots**; the budget gate now reports E3014 only
  when the post-allocation frame genuinely does not fit (with the function-body declaration span),
  rather than falsely rejecting based on the cumulative temporary count.
- **Advantages must also be recorded**: turning “errors that cannot be tested” into “invariants that
  can be tested”; no backend changes (`executor/` and `.42` remain unchanged); no changes to
  `Instruction` semantics (all 76 → 77 variants are retained, and all 50 `translate_*` functions are
  retained); incremental delivery (each item can be reverted independently).

> **All open questions originally listed in this section have been resolved.** The item-by-item
> decisions are recorded in
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> article leaves no pending items.**

## See Also

### Documentation

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching framework; four-layer model, criterion tiers, P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definition of C4
  criteria, `verify_loose` / `verify_ssa`, three criterion layers, and the snapshot tool's immunity
  to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of three
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the
  `ReleasePlan` `PlanId` migration
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — eight orphan test trees; the cautionary
  example of “creating a directory first and then failing to wire it up”

### Code Locations

**IR Definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — 7 `Operand` variants
- `src/middle/core/ir.rs:47-533` — 76 `Instruction` variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for the `PlanId`
  approach)
- `src/middle/core/ir.rs:689-719` — `all_instructions` and other accessors

**Defect Class 1 (Multiple Definitions and Save/Restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for seven nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — sole table-extraction + E3014 budget point
  (`MAX_REGISTERS = 255`), combining block partitioning / slot allocation / table extraction
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — linear-scan allocator (sole producer of `value_slots` and
  `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamations
  removed (7b.4); slots are reused by the allocator
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — target of 07's “jump target exists”
  invariant

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  a clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill with 0” fallback

**Defect Class 3 (`Span` Keying)** — The following are **audit-time** evidence pointers (after the
4.5 key migration: the `release_plan` / `overload_resolutions` keys have been converted to `PlanId`;
see the revision note in §2 of [02](02-stage-contract.md) for current pointers;
`method_def_ordinals` was eliminated by 7c.3 under mechanism three):

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write point of
  `method_def_ordinals` (key was the module-unqualified `"Type.method"`; **the field was removed in
  7c.3**, and this is audit-time evidence)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the propagation structures
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — definition and
  production of `ReleasePlan`
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer propagation
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (Implicit Order Dependence)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — equivalent handshake in `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — three other `ast::Type` constructions

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — 66 `BytecodeInstr` variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded discards (compare `2344` /
  `2347`); old audit record, with review conclusions in “Tracking of Hard-Coded Discards” above (two
  genuine losses + one false positive)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge pending removal in
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — nine NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  limit of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active `BytecodeInstr::Switch`
  implementation (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current Test State**

- `src/middle/core/ir_gen.rs` — 8,448 lines, zero `#[cfg(test)]`, zero `mod tests`
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no whole-IR
  structural-invariant verification
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — four references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example per-opcode round-trip criterion; the last test in the file; 1,209 lines / 23 `#[test]`
  functions in total)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies contain only `criterion` / `proptest`; `[dependencies]`
  contains no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans `/rfc/draft`; this
  article is not in that directory and is referenced by RFC-039 and this directory's index
