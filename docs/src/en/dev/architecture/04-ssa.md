# Intermediate Representation SSA Transformation

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main text of RFC-039 for the four-layer model, tiered acceptance criteria, and execution phase
> order; see the [index of this directory](index.md) for the positioning of the companion documents.

## Positioning and Scope

This document addresses L3 IR construction discipline and proposes a construction approach that
enforces “one definition per value” for `src/middle/core/ir_gen.rs` (8,448 lines). **The goal is not
to introduce a new instruction set, but to eliminate three categories of discipline defects that
produce no errors when they fail**:

1. Six hand-written register save/restore pairs separated by dozens of lines;
2. Semantic reordering of `arg_regs` shared by six branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, explicit cross-layer contracts, decomposition of
call-expression dispatch, and the wiring of the IR static verifier.

This document is not responsible for: convergence of type representations
([03](03-type-unification.md)), phase contracts and the obligation ledger
([02](02-stage-contract.md)), the definition and gating of equivalence criteria
([07](07-equivalence-oracle.md)), the frontend paradigm ([05](05-frontend-paradigm.md)), or cleanup
of dead code and promissory designs left unimplemented ([06](06-cleanup-inventory.md)). This
document repeatedly references the conclusions of those documents without repeating their arguments.

**Criterion categories** are applied according to **C4 (IR shape changes)** in
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, not exact equality of IR snapshots. SSA transformation necessarily changes the IR
shape, and forcing snapshot equality would induce the team to relax the criterion—the exact trap
recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, with acceptance requiring behavioral equivalence and a green `verify_ssa`). The
implementation batch numbers within this document (batches a–d) are distinct from the internal
ordering of P7: the former are this document’s batching strategy, while the latter is RFC-039’s
global phase sequence.

## Feasibility Preconditions

**Verified fact.** The IR already possesses **all structural prerequisites** needed to construct
SSA:

| SSA prerequisite             | Current repository state                                                                                         | Location                                                               |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                                 | `BasicBlock` in `src/middle/ir/ir.rs`                                  |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                   | `FunctionBody` in `src/middle/ir/ir.rs`                                |
| Scope nesting information    | `LocalSlot::scope_depth` (“0 = function-parameter level, used to disambiguate duplicate names in nested scopes”) | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Three-address form           | 76 variants, 54 with `dst: Operand`, and 3 with `dst: Option<Operand>`                                           | `Instruction` in `src/middle/ir/ir.rs`                                 |
| Per-instruction span         | Every variant has a `span: Span` field                                                                           | Same as above                                                          |
| Value types can be annotated | `LocalSlot::ty: MonoType`                                                                                        | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Slot-count upper-bound check | Table extraction and the E3014 budget gate share a single point (`MAX_REGISTERS = 255`)                          | `finalize_code_body` in `src/middle/lower/scope.rs` (from 7b.4 onward) |

**If these prerequisites were absent, the cost estimate in this document would be completely
different.**

**The two decisive points are as follows:**

**First, SSA transformation is not a new capability; it removes a capability dependency.** Temporary
slots are currently **deliberately designed to have multiple definitions**—`generate_function_ir`
and `generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`, explicitly roll `next_temp` back in three places. The first step
of SSA transformation is to **delete these three rollbacks**, making `next_temp` increase
monotonically. Once they are deleted, **every temporary slot naturally has exactly one definition
point**—unique definition changes from “a rule that must be checked” into “a mathematical
consequence of the construction method.” Batch a only converged the save/restore surface (see
“Defect Class 1”); the three rollbacks were removed together with 7b.4 in batch b (the allocator
took over slot reuse; 2026-11-22).

**Second, the slot-count upper bound is a hard check, and deleting the rollbacks briefly caused the
slot count to become uncontrolled (resolved in 7b.4).** The budget gate in
`src/middle/lower/scope.rs` reports E3014 when the count exceeds 255. After removing all three
reclamation points and running the full corpus, two corpus cases crossed the limit and triggered
E3014: `src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; peak slot counts in the
remaining corpus rose from 51 to 211. **After 7b.4 landed:** the linear-scan allocator reuses slots
according to live intervals; the two corpus cases now use **15 / 24** frame slots (344 / 304
values), and the remaining corpus is behaviorally equivalent at 403/403. E3014 now means only “frame
slots after allocation exceed 255.”

**Taken together, these points mean:** lowering already maintains all the bookkeeping information
SSA requires, but merely **throws it away**. 7b.4 makes the allocator responsible for slot reuse and
removes the reclamation. The main SSA work lies in decomposing `generate_call_expr_ir` and making
cross-layer contracts explicit, not in building new IR expressive power.

### Prerequisite: Unification of Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The third representation is `ir::Type` on the serialization side,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **two type representations coexist in the IR, and one of them directly `pub use`s
the AST’s type.**

SSA transformation must add a `Phi` variant, record a type for every SSA value (for the validator’s
“type consistency” invariant), and determine whether the types of two predecessors are compatible at
a CFG convergence point. **If type representations have not yet converged, SSA would require a
separate type-compatibility check for every representation**—and the third representation would grow
accordingly.

This is not a question of which task should come first; it is a question of **which task would first
produce three sets of type-compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first converge `pub use ast::Type`
at `ir.rs:3` and remove the bridge at `bytecode.rs:2353`. Item 6 in the change list explicitly
records that **this document does not modify** `ir.rs:3`.

## Current State: Four Classes of Defects

`ir_gen.rs` is not “an ugly file that nevertheless works.” It is a file whose **correctness depends
on humans reading it line by line**:

- The file contains **0 `#[cfg(test)]` blocks and 0 `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares three modules—`bytecode`, `def_assign`, and
  `local_slots`—with 29 tests in total. `def_assign.rs` and `local_slots.rs` **positively** cover
  ir_gen’s DefId allocation and local-slot naming at a sentinel level; all four ir_gen references in
  `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **negative assertions**—“ir_gen does not
  emit,” “ir_gen does not construct,” and “the ir_gen frontend does not construct the corresponding
  IR.”

In other words: ir_gen has 29 sentinel-level tests pinning local behavior, but **no IR structural
invariant validation** (dominance, unique definition, jump targets, or type consistency)—none of the
three modules declared in `middle/core/tests/mod.rs` makes a positive assertion about the overall IR
shape.

This file also happens to contain several classes of defects that **compile successfully, pass every
check, and let the program run normally, but produce wrong values**. Their common property is:
**failure produces no error**.

This is where the highest risk lies. The problem is not that “the function is too long,” nor that
“the naming is confusing,” but that **the file’s core invariants have no machine-enforced
mechanism**.

The contract that “the nested function body must be saved before generation and restored afterward,
otherwise inner function names will leak into the outer scope” is enforced by the type system:
`NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator,
and frame removal and restoration each have only one implementation (see “Defect Class 1”).

**Why adding a verifier alone is not enough**: a verifier can **catch** these defects, but it cannot
catch **what causes them**. The cause is not “someone missed it while reviewing,” but:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator deliberately depends on that fact.**

The concrete evidence is three statement-level register reclamations (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`)—at the end of every statement, they **roll `next_temp` back** to
the greater of “the last named-slot watermark” and `temp_floor`
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and the number and
order of writes and the live intervals are determined entirely by the single manual decision of
**when to roll back**.

Under this shape, “unique definition” is not a rule that can be introduced incrementally; it
**directly conflicts with the register-allocation strategy**. SSA transformation therefore is not a
check added to existing code; it is a **replacement of the allocation strategy that depends on
multiple definitions**. The good news is that replacing this strategy is inexpensive—see
“Feasibility Preconditions” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (the Three Reclamation Points Were Removed in 7b.4)

**Verified fact.** There are seven entry points that require save/restore when entering a nested
function body. Batch a (7a.2) converged them into one RAII guard: `NestedBodyGuard<'a>` in
`src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, with unconditional restoration in
`Drop` and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Restoration scope                                       |
| ------------------------------------ | ---------------------------------- | ----------------------------- | ------------------------------------------------------- |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (the three counters are not restored) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                           |
| Nested function definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                      |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                           |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                           |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                           |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                           |

- Frame removal occurs only in the two constructors (`enter` / `enter_decl`), while restoration
  occurs only in `Drop`; the guard exclusively holds `&mut` access to the generator for its
  lifetime, so no second `&mut` is available in scope. All `?`/early-return paths restore through
  `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`; from 7b.4 onward,
  `next_temp_reg` maintains only the named high-water mark (temporary-value slots belong to the
  linear-scan allocator; `temp_floor` was removed together with the three reclamation points).
- `enter_decl` does not restore the three counters: after a function body or method body returns to
  the module-initialization flow, subsequent definitions continue from the child body’s final value
  number. This is a C1 zero-snapshot-drift constraint and the meaning of “the declaration surface
  restores only tables.”

**Unified accounting and a single budget point (from 7b.4 onward):** `finalize_code_body` in
`src/middle/lower/scope.rs` is the sole table-extraction and budget point—it extracts the table
using `frame_slots = named_base + max_live`, reports E3014 with the function-body span on overflow,
and defines `MAX_REGISTERS = 255` in exactly one place in the repository. All six function-body
forms (function / method / lambda / anonymous binding / middle curry layer / innermost curry layer)
are finalized through it, so there is no second definition of “how many slots a function body should
have.”

**The three statement-level reclamations were removed (7a.1 landed together with 7b.4,
2026-11-22):** the reclamations were hand-copied in `generate_function_ir` / `generate_block_ir`
(with a `result_reg.is_none()` guard) / `generate_lambda_body_ir`; after the allocator took over
slot reuse, reclamation degraded to an identity operation, so even the guard, `temp_floor` /
`saved_temp_floor`, and the loop-entry/loop-exit bookkeeping in `loops.rs` were removed. Module
initialization and top-level statements still follow the old monotonic-slot path with
`value_mode=false` and do not run the allocator; their slot numbers and table lengths shift with the
number of named slots in function bodies (three snapshots measured; behavioral equivalence is
covered by 403/403).

**Consequences in the current form:** the guard exclusively holds `&mut`, and restoration has only
one implementation, in `Drop`; there is nowhere to write a missing restoration. The temporary-slot
dependency on multiple definitions has disappeared together with the reclamations—value slots are
reused by the linear-scan allocator according to live intervals, so `verify_ssa`’s unique-definition
criterion now has a construction-side basis.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified fact (audit-time snapshot; current locations have been synchronized with the batch-c
decomposition in 7c).** `generate_call_expr_ir` is now located in `src/middle/lower/expr/call.rs`
(split out of `src/middle/core/ir_gen.rs`, which no longer exists), as a single function. Its
signature accepts `func: &Expr` / `args: &[Expr]` / `named_args: &[(String, Expr)]` / `span: &Span`
/ `call_plan_id: PlanId` / `result_reg: Operand` / `instructions` / `constants` (`call_plan_id` is
the decision-consumption key from RFC-039 D20). Internally, multiple branches share the same
`Vec<Operand>` and perform **semantic rewriting** (the line numbers below are from the audit-time
version; after decomposition, locate by symbol):

| Line        | Semantics                                                                                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `7395`      | Namespace call: flattened by `slots` and then re-`collect`ed—named arguments are reordered                                                                   |
| `7724`      | Struct construction: each slot in `final_args` is `unwrap`ed and then re-`collect`ed—fields are reordered                                                    |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **adds a new register containing 0** (`next_temp_reg` + `Instruction::Load`) for each missing slot |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—the vector is cloned before instructions are emitted                                                      |

The original form of the four reorderings is:

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
reports E1010. For runtime safety, fill in 0 (the normal path never reaches here).”

**Current state of named arguments (after narrowing in 4.9.9):** before reordering, named arguments
are bound through the **declaration-time formal-parameter name table**—using the shared criterion
`arg_binding::bind_args_to_params`. The IR-side name table is obtained through `lookup_param_info`,
with sources including this unit’s `fn_param_names`, the cross-unit method candidate group
(`method_param_info`, queried by the qualified name in `method_dispatch_name`), and the module
export surface. A cross-unit method arm without overload resolution does not carry named arguments
(the old behavior silently discarded them; the known gap is recorded in 09 under 4.9.9 → 4.9.10).
The “fill in 0” behavior in the table above remains a runtime safety net for positional arguments
and fallback arms and is unreachable on the normal path.

**This design has two kinds of risk**:

1. **Misalignment reports no error and merely produces a wrong value.** Each of the six branches
   determines the final order of `arg_regs`; an incorrect ordering assumption in any branch does not
   cause a panic.
2. **Normalized snapshots are immune to this problem.** 07 already records this limitation: after
   the snapshot tool renames temporary values by order of first appearance, “the third argument
   should have used the fifth register” is identical in the snapshot. **At the C4 stage, this class
   of defect has no snapshot criterion at all.**

**Additional verification: the struct-construction path’s reordering has no duplicate-slot guard.**
`final_args` at `7724` is populated by “put positional arguments in their slots first, then
overwrite by field name for named arguments,” but **there is no check such as `slots[idx].is_some()`
before overwriting** (unlike the namespace path at `7381`). When a named argument silently replaces
a positional argument, there is no panic and no diagnostic. Moreover, argument **evaluation order**
(source order) diverges from the final `arg_regs` order (field-declaration order) on this path. The
`fill_missing` assertion in batch d must cover the “duplicate slot” check as well; otherwise, the
silent replacement on the struct-construction path will survive unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified fact (audit-time snapshot).** The repository has **two** Span-keyed cross-layer
contracts:

| Contract               | Declaration                                                                                                                    | Production                                                                                                                 | Transfer                                                                              | Consumption                                                                                                          |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                      | `ownership.rs:31` (the `ReleasePlan::drops` field), `build_release_plan` (statement identity when dispatching `walk_stmt`) | `typecheck/types.rs:38` ← passed by `checker.rs` ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, the sole entry point; completion marker at `:1095`)                        |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger combined; `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` only passes through and merges)  | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held in the `ir_gen.rs` field   | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; completion marker at `:1097`) |

The audit-time form (`ir_gen.rs:1942`, **now nonexistent**) was:

```rust
// NLL Release: 在语句边界插入 Drop 指令
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

The `if let Some` means that **if the lookup fails, it simply fails—there is no else branch,
assertion, or counter**. If the two sides compute spans differently in any way, **the Drop
instruction silently disappears, with zero errors**. The Drop sequence for refinement types is a
central carrier of this project’s ownership semantics (`ownership.rs:26-32` describes the “NLL
precise release plan”), and its lowering was once silent.

The same pattern at `7420` / `7493` (`if let Some(mangled) = self.overload_resolutions.get(span)`)
causes overload resolution to fall back to the default binding.

**Disposition (landed in 4.5):** both keys were changed to the structural identity `PlanId`, and
consumption **records a hit in the same operation** (the read and accounting occur in one call). A
non-empty difference is reported at the sole settlement point as an **E-level `W1090`**—“not found”
is no longer silent. See the revision note in §2 of [02](02-stage-contract.md) (4.5) for the
implemented form and criteria; these line numbers (`1942` / `7420` / `7493`) are retained as
audit-time evidence and are not current-state pointers.

> **`method_def_ordinals` is not Span-keyed (**eliminated in 7c.3**).** It was originally a
> `HashMap<String, usize>` that assigned `#N` suffixes according to the traversal order of
> definitions keyed by `base_name`. Its risk is **different**: definition-order dependence—if
> generation order differs from typecheck registration order, the same function name may resolve to
> a different target. See mechanism three in “Additional Mechanisms for Span-Keyed Problems” for the
> implemented form: typecheck produces the definition name once according to declaration identity,
> and lowering only consumes it.

### Defect Class 4: Implicit spawn/lambda Handshake (eliminated in 7c.1)

**Eliminated.** spawn and spawn-for previously required the caller to write `pending_env_vars` /
`pending_env_names` first, after which a synthesized Lambda would consume them; closure bodies
separately used `closure_captures` to determine whether a variable reads an upvalue. Three fields
tied correctness to call order and manual cleanup: a nested synthesized closure could mistake an
outer local slot with the same name for a captured value, and early return from closure-body
generation could leak capture state into later, unrelated lowering.

7c.1 converged synthesized ASTs into `middle/lower/synth.rs`, replacing the three shared fields with
an explicit `SynthEnv` and registering named captures as lexical upvalues. Nested-frame exit
restores state according to symbol-table depth, so error paths no longer leave capture state behind.
See the “`synth.rs` Boundary” section for the resulting structure.

### What SSA Will Not Solve (Stated Honestly)

| Problem                                                               | Location                                                                                                                                       | Why SSA does not solve it                                                                                                                                                                                                                         |
| --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                                          | `ir_gen.rs:5390-5687`, **298 lines**, with 4 levels of recursion (`5473` / `5504` / `5619` / `5640`), and an external call site at `5270`      | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **inherent to the algorithm**. SSA only turns guard-generated `JmpIfNot` instructions into `Phi`; it does not reduce the number of guards |
| `eval_const_expr` complexity                                          | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)                        | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                                                   |
| Span-keyed failure (**eliminated**)                                   | Audit-time: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`); after 4.5, `PlanId` keys + mandatory settlement                 | **SSA does not touch this layer at all** (the conclusion remains unchanged). It is independently resolved by the obligation-ledger mechanism in [02](02-stage-contract.md); see mechanism two in “Disposition of Span-Keyed Problems”             |
| Number of `Instruction` variants                                      | 75 (the `Instruction` enum in `ir.rs`; including the `Phi` introduced by 7b.1a)                                                                | SSA adds `Phi`, a **net change of +1** (already landed). A variant maps to a bytecode opcode or is degraded to NOP; it will not decrease because of SSA. Under option A, `Phi` is an IR-only pseudo-instruction and adds no opcode                |
| Definition-order dependence in `method_def_ordinals` (**eliminated**) | Audit-time: `ir_gen.rs:224` / `369` / `1692-1702`; implemented in 7c.3, see mechanism three in “Additional Mechanisms for Span-Keyed Problems” | Similar to Span keying but with a different cause; the definition name is now produced by typecheck according to declaration identity, and lowering only reads it                                                                                 |

## Target Design

### Definition of the SSA Shape

**Design judgment.** The target shape is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has 5 variants. **Repository-wide measurements** (hit counts summed by
file using `git grep -c`, over all of `src/` including tests; construction points refer to
value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                          | Disposition                                                                                        |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                                  | **Retained**, unchanged                                                                            |
| `Value(ValueId)`    | 7 files / 26 hits; **0 production construction points** when introduced by 7b.1a; from 7b.2 onward, `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`)                 | **Retained** (introduced in 7b.1a)                                                                 |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                                  | **Retained but semantics split** (see below)                                                       |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                    | **Retained**, as the implicit definition at function entry (parameters are definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                                   | **Retained**, unchanged                                                                            |
| `Temp(usize)`       | **0 construction points** (the processing arm only existed in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, and was removed with the variant)                                 | **Removed** (dead variant, 7b.1a)                                                                  |
| `Label(usize)`      | **0 construction points**                                                                                                                                                                            | **Removed** (dead variant, 7b.1a)                                                                  |
| `Register(u8)`      | **0 construction points** (the comment at the original variant declaration said "Added for codegen", but codegen uses `OperandResolver` in `codegen/operand.rs` and does not construct this variant) | **Removed** (dead variant, 7b.1a)                                                                  |

`Local(usize)` is split into two variants because it currently **serves two mutually exclusive
semantics**—“the storage location of a named variable” (which may be written multiple times) and “a
compiler temporary register” (which should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA 值：函数内稠密编号，唯一定义
    Value(ValueId),
    /// 具名局部变量的存储槽：可寻址、允许多次定义（非 SSA 部分）
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing and never rolled back**. The split
and `ValueId` were introduced by 7b.1a (the `Operand` enum in `src/middle/ir/ir.rs`); the production
side (`next_temp_reg` now producing `Operand::Value`) was implemented by 7b.2 (see “Production Form”
under “Value Identification”).

#### `Phi` as a New `Instruction` Variant

```rust
/// 汇合点：每个前驱基本块提供该值的一个定义
/// incoming 按前驱 block label 升序排列（构造时保证，便于规范化）
Phi {
    dst: Operand,                                  // 必为 Operand::Value
    incoming: Vec<(usize, Operand)>,               // (前驱 label, 来自该前驱的值)
    span: Span,
},
```

There are three construction constraints (see the first layer of [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set of `incoming` equals the label set of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. All `incoming` values have the same `MonoType` (**dependent on the type representations having
   converged in 03**).

The implemented state of `verify_ssa` (7b.1b completed the first three constraints): constraints 1
and 2 require the value table and block-level CFG, and were implemented in **7b.1b** as two
criteria: “block-level predecessor count” and “predecessor label set.” The predecessor set is
inferred from `BasicBlock.successors` (`block_predecessors` in `src/middle/ir/verify.rs`); a count
mismatch reports only a count error, and a set mismatch reports only a set error, so the same defect
is not reported twice. The per-input type-consistency criterion for constraint 3 was implemented at
the same time, obtaining values from the value table `values[v].ty` and **reusing the existing
`types_compatible`** (no third type-compatibility check was introduced; convergence of the type
representations in 03 remains “Precondition Two (Hard)”; even then, `ValueInfo::ty` remains
`MonoType`). The violation category for all three criteria is `InvariantKind::PhiConsistency`.

**Three forms for which the type is unavailable (explicitly recorded, never silently allowed)**: an
empty value table means that the basis for judgment does not exist (the constant shape of module
initialization / top-level statements, on the `value_mode=false` path); a non-empty value table with
an out-of-range `ValueId` is a violation; a non-`Value` operand (`Const` / `Local` / `Arg` /
`Global`) has no value-table type source and is not judged.

**Measured prerequisite (fulfilled in 7b.3):** before 7b.3, every repository-wide production
construction of `BasicBlock.successors` used `Vec::new()` (block-level predecessors were always
empty, and the predecessor-count and predecessor-set criteria were then exercised only by synthetic
fixtures). **From 7b.3 onward, block-level `successors` are populated for real by `partition_blocks`
in `src/middle/ir/ir.rs` at every production construction point** (all six `lower` sites and the one
synthetic function-body constructor in codegen go through this sole entry point; loop and branch
blocks produced by `lower` include multiple blocks, dual edges, and back edges). Consequently, the
predecessor-count and predecessor-set criteria operate on real data—the direct-driving test on true
lowering output pins down this registration-to-consumption loop. The `Phi` production construction
count remains 0 (production belongs to a later batch), and its eventual implementation will be
directly constrained by this criterion.

7b.1a only introduced the variant: the lowering production construction count was 0 (and remains 0;
production belongs to a later batch). The consumption side in `translator.rs` was implemented in
**7b.3**: option A expands it at block entry into a sequence of `Move` instructions (each group is
placed at the convergence block’s entry, existing `JMP` instructions skip between groups, and
incoming-edge jumps are redirected to the group’s starting point). The `Phi` arm of
`translate_instruction` is an unreachable E8001 defensive arm (`executor` never sees `Phi`).

**Disposition of the guard problem at `3632-3638`:** the reclamation in the current
`generate_block_ir` has an `if result_reg.is_none()` guard (the comment at `3626-3631` explains that
the block may hold sibling argument temporaries that remain live across blocks). After SSA
transformation, this guard is **no longer needed**—temporary values are no longer rolled back, and
sibling arguments that remain live across blocks hold their own `ValueId`s and cannot be affected by
a rollback. **This is where SSA transformation can genuinely simplify the code, rather than merely
rename things.**

#### Value Identification

**Design judgment:** do not perform memory promotion (mem2reg), do not introduce virtual-register
numbering, and **perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots can be promoted to registers” a
  **type-system problem**, not a CFG problem. It has been verified that `LocalSlot::ty: MonoType` in
  `ir.rs` carries complete type information, but determining “whether this `MonoType` can be
  promoted to a register” requires the type representations converged in 03 and cooperation with
  refinement-type rules. **Performing mem2reg before 03 means guessing.**
- Explicit renaming requires changing only the return-value semantics of `next_temp_reg()`
  (`next_temp_reg` in `src/middle/lower/scope.rs`) and deleting the three rollbacks; it **does not
  touch the control flow of any lowering algorithm**.

The value-table form (**implemented in 7b.1b**): `FunctionBody::Code` gains
`values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// 调试名（来自被消除的具名槽位），None 表示纯临时值
    pub debug_name: Option<String>,
    pub defining_block: usize,   // 定义所在基本块的 label
}
```

The accessors have the same shape as existing accessors (`impl FunctionIR` in
`src/middle/ir/ir.rs`): `values()` is read-only and returns an empty table for TypeDecl (mirroring
`locals()`); `values_mut()` is mutable and panics for TypeDecl (mirroring `blocks_mut()`).

**Production form (implemented in 7b.2):** `next_temp_reg()` returns `Operand::Value(ValueId)` and
registers one `ValueInfo` entry in the current function body’s value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). The implemented form is as follows:

- **Counter isolation:** `ValueId`s are issued by an independent counter, `next_value: u32`. Each
  function body starts at 0, increases monotonically, and is never rolled back. `NestedBodyGuard`
  saves and restores it together with the value table (one set per function body). The three
  statement-level reclamations only roll back `next_temp` (slots) and do not touch `next_value`.
  Each curry layer is reset separately (one `FunctionIR` per layer). Module initialization and
  top-level statements have no value table—when `value_mode=false`, `next_temp_reg` degrades to the
  old `Local` slot allocation, leaving the initialization sequence unchanged.
- **Type source** (only existing lowering information; no new type inference mechanism): literals
  use `ConstValue::mono_type()` (converged with the verifier’s constant-type mapping into a single
  source); variables and calls query the type-check result and bound function return type at the
  `generate_expr_ir` entry point; comparison, logical, string-family, and forward-propagation forms
  are filled in at function-body finalization according to instruction semantics. When unavailable,
  an **explicit unknown `MonoType::TypeRef("?")`** is used (not a silent `Int(64)` placeholder;
  `LocalSlot.ty` is never read). Across 232 snapshots, there are 9,425 values: 7,080 have real types
  and 2,345 have explicitly unknown types (24.9%), concentrated in call, field, index, and
  container-construction forms.
- **defining_block:** takes the current block label during generation. **Block splitting landed in
  7b.3** (a true lowering `while` body consists of multiple blocks), but this field still uses the
  generation-time `cur_block_label` (currently always 0), so it does not yet match the actual
  defining block. 7b.4 did not change this field when regenerating the 232 snapshots (the `@block`
  entry in the `values` section did not drift). Correcting it must be done in the same batch as
  regenerating the `values` section and is **explicitly handed over to the `Phi` production batch**.
- **debug_name:** always `None` (named variables remain `Local` and are not migrated into the value
  table).
- **Indexing:** `value_slots: Vec<usize>` has the same indexing as `values` (index = `ValueId`) and
  records the value-to-frame-slot mapping. From 7b.4 onward, it is recomputed by the linear-scan
  allocator in `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`); codegen’s `OperandResolver` only consumes it and never
  allocates.

`values` and `locals` coexist: `locals` carries addressable named variables (which may have multiple
definitions), while `values` carries single-definition SSA values. **The existence of `locals` does
not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Separation from `LocalSlot::origin`:** the two are **different dimensions** and must not be
conflated. `LocalSlot::origin` (the **slot-identity** field introduced by origin-side 2.1.4,
recording the generating `FrameId` of the slot) answers “which frame does this slot belong to”;
`ValueInfo.defining_block` answers “in which basic block is this SSA value defined?” Value identity
belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`; 7b.1b only added the value
table and **did not change `LocalSlot::origin`** (measured: zero changes).

#### Change-Size Estimate

| Item                                                         | Estimate                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------ | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Change to `next_temp_reg`                                    | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Delete the three statement-level reclamations                | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Consolidate 6 save/restore pairs into RAII                   | Net ±0 lines                          | See the “Itemized Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `ir.rs` structural changes                                   | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **7b.1a landed the first two, and 7b.1b landed the last two** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Add a `Phi` arm to `translator.rs`                           | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `executor/`                                                  | **No changes**                        | Option A: `Phi` is an IR-only pseudo-instruction expanded by `translator.rs` into Move instructions and does not enter bytecode (Decision Three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Call sites that produce temporary values as `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (hit counts summed by file using `git grep -c`, over all of `src/` including tests): `Operand::Local(` appears 415 times in 28 files, including 332 times in 14 files under `src/middle/lower/`; the production point for temporary values is the single `next_temp_reg`. **7b.2 measurement (baseline-revision remeasurement and net landed change):** `next_temp_reg()` has 140 call sites in 14 files, the `result_reg: usize` parameter chain spans 35 functions plus 3 `Option<usize>` result slots, and `Operand::Local(` appears 342 times under `src/middle/lower/`. The code-only diff across `src/` totals +23319/−12495, of which regenerating 232 IR snapshots accounts for +21866/−11950; the net source-and-test change is +1453/−545 (mechanical replacements, normalization rendering, mono projection, and six direct-driving tests; five generated error-code artifacts were also committed alongside the code changes) |

### Itemized Disposition Table

| Defect class                                                | Eliminated by SSA?         | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                     | Criterion                                                                                                          |
| ----------------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **1. Multiple definitions and manual save/restore**         | **Yes (resolved in 7b.4)** | The seven nested-function-body entry points are consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the counters), turning “forgot to restore” into a compile error. The three statement-level reclamations were removed with 7b.4 (the allocator reuses slots according to live intervals), and temporary values no longer occupy `next_temp` | First-layer “inner isolation” invariant from 07 + `verify_loose`/`verify_ssa`                                      |
| **1b. Inconsistent accounting**                             | **Yes (landed)**           | Unified in `finalize_code_body` in `src/middle/lower/scope.rs`: the table is extracted using `frame_slots`; the E3014 budget check occurs at the same point and includes the function-body span; all six function-body forms are covered; `MAX_REGISTERS` has one definition in the repository                                                                                                                                  | Corpus differential + zero snapshot drift                                                                          |
| **1c. Criteria for the three statement-level reclamations** | **Yes (landed in 7b.4)**   | The three reclamations, the `result_reg.is_none()` guard in `generate_block_ir`, and the `temp_floor` bookkeeping were removed; slot reuse is now handled by the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                                                | C4: behavioral equivalence (403/403) + invariants                                                                  |
| **2. Semantic reordering of `arg_regs`**                    | **Partially**              | SSA turns “using the wrong register” into “using the wrong value ID,” which dominance checking can catch, but **the reordering logic itself remains**. Actual elimination depends on function decomposition—each of the six branches performs `push`, while ownership of `arg_regs` remains at the top of the function                                                                                                          | First-layer dominance invariant (snapshots are immune)                                                             |
| **3. Span-keyed failure**                                   | **No**                     | SSA does not touch it. See “Additional Mechanisms for Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                                      | `test_release_plan_spans_consumed` from 07                                                                         |
| **4. Implicit ordering dependency**                         | **No**                     | See the “`synth.rs` Boundary” section                                                                                                                                                                                                                                                                                                                                                                                           | Guaranteed by code structure and review; the original gate script was canceled by the user’s 2026-11-24 Decision D |
| `compile_pattern`, 298 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                                   | —                                                                                                                  |
| `eval_const_expr`, 191 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                                   | —                                                                                                                  |
| 75 `Instruction` variants                                   | No (+1, landed)            | `Phi` was introduced by 7b.1a; under option A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                                            | —                                                                                                                  |

### Register Allocator: Custom Implementation or Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **0 construction points throughout the repository**—the variant is no
   longer present in the `Operand` enum in `ir.rs` (removed in 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper bound of 255 (exceeding it
   reports E3014); `Operand::Value` is resolved by the 7b.4 allocator’s `value_slots` to a frame
   slot (`>255` reports E3014; no mapping reports E3017 with the instruction span).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50
   `translate_*` functions distributed across `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`, with opcodes from `src/backends/common/opcode.rs`
   (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter’s opcode-dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Assessment of three options**:

| Option                                             | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both take rustc’s `Function` trait and MIR’s `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would have to implement the complete trait (CFG, liveness, spill slots, scratch), requiring **500+ lines of adapter code for zero benefit**—because there is no real register file to color. Moreover, rustc itself only uses linear scan at this point (when `enable-llvm` is disabled)                                                                                                       |
| `graph-alloc` family                               | **Not adopted.** The same reasons apply, and the API is less stable                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| **Custom linear scan (simplified Chaitin-Briggs)** | **Adopted (landed in 7b.4).** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot according to its live interval (`OperandResolver` already has the `u8` contract and E3014 upper bound). Implement `src/middle/lower/regalloc.rs`: block-level `successors` backward dataflow, closed intervals, Phi predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`. On overflow, return `Budget` so that `finalize_code_body` reports E3014 with the declaration span |

**Landed measurement (7b.4):** the two corpus cases that previously exceeded the limit (354 / 323
monotonic slots) now use **15 / 24** frame slots after allocation (344 / 304 values), and the
remaining corpus is behaviorally equivalent at 403/403. E3014 now means only “frame slots after
allocation exceed 255.” The counter used for the back-edge fallback is currently not exposed
(`wrap_count` is calculated and discarded); the `Phi` production batch must expose and retest it
first (alongside `defining_block` as a precondition). **Overflow handling:** `u8`/255 is still
handled as an error by the budget gate—this document does not solve “the function has too few
registers,” which is a downstream consequence of `compile_pattern` recursion depth.

The dev-dependencies at `Cargo.toml:37-39` contain only `criterion` / `proptest`; `[dependencies]`
contains no regalloc / petgraph / graphlib. Introducing any of these crates would add a new
dependency surface.

### Decomposition Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA shape and criteria must come first**. The reason is that
decomposition itself is C1 (a pure move, judged by zero snapshot diff), but decomposition will carry
the old ordering assumptions into the new file unchanged unless the semantics of `arg_regs` have
first been reduced by SSA to “one independent `ValueId` per argument.”

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

The “fill in 0” logic at `7835-7848` (`7838-7846`) is moved down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the comment
claiming that “the normal path never reaches here” is replaced by an assertion**: if any slot is
missing and typecheck has not reported E1010, return a diagnostic directly. **This turns a silent
fallback into a hard error and is the most direct improvement in this document for Defect Class 2.**

**Criteria:** the pure-move portion (with the SSA shape unchanged) uses a zero-diff snapshot; making
`fill_missing` assertive belongs to C4 and uses a corpus differential plus the first-layer
validator.

### `synth.rs` Boundary: Turning an Implicit Callback into an Explicit Synthesized-AST Boundary

**Implemented form:** `src/middle/lower/synth.rs` is the only L3/L4 production module permitted to
construct `ast::Expr` / `ast::Stmt` / `ast::Type`. Pattern matching, read-only queries, and test
fixtures are not synthesis. The current production synthesis points are spawn’s Lambda/Stmt and
spawn-for’s Lambda, all of which pass through this module.

AST nodes do not carry a lowering environment, so `SynthEnv` is not an AST field and is not
pretended to be consumed by an AST helper. It contains runtime environment operands and capture
names, and the caller explicitly passes it to `generate_expr_ir_with_env` alongside the synthesized
node:

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

- spawn and spawn-for construct `SynthEnv` and explicitly pass it into lowering alongside the
  synthesized Lambda; they no longer prewrite `pending_env_vars` / `pending_env_names`. The total
  environment-slot count is derived from `env_vars.len()`, and named upvalues begin after the
  anonymous prefix. Consequently, spawn-for’s anonymous iteration value, outer captures, and
  parameters each occupy their proper positions.
- Ordinary Lambdas complete the environment from the existing free-variable collector. List
  comprehensions, nested Lambdas, and nested spawns traverse the same collector, while generator
  variables and nested parameters are excluded by lexical scope.
- Closure bodies register `SynthEnv` as `BindingLocation::Upvalue` in the existing symbol table.
  Variable reads and namespace-shadowing queries share this lexical binding and no longer read
  `closure_captures`.
- A nested closure first reads the outer upvalue through `capture_operand`, then passes the value
  into the inner environment; it cannot mistake an outer local slot with the same name for a
  captured value.
- `NestedBodyGuard` records the symbol-table depth on entry and truncates to that depth on exit,
  including early return through `?`; capture bindings cannot leak into later, unrelated lowering.
- Sema’s expression-body function check parameterizes the existing inputs with
  `body_stmts + expression_body + span`, directly invokes `check_expr`, and reuses tail validation
  instead of synthesizing a Block/Stmt/Expr. There is no reverse dependency from sema to
  middle/lower and no additional architectural exception.
- SpawnFromList shares the `env + [anonymous item]` argument layout in Embedded and Standard and
  injects the same environment into upvalues. The anonymous iteration value is fixed at env[0],
  named captures are read after the anonymous prefix, and parameters begin after the entire
  environment.
- Dual-runtime behavior tests explicitly invoke main() and assert, within a spawn-for task, both the
  absence of a captured item value and the presence of an item plus an outer capture. The no-capture
  form also covers the regression restored by 74febd1f. IR assertions and actual execution are
  recorded separately; zero drift across the existing 403 corpus cases does not masquerade as
  behavioral coverage for this form.

**Criteria:** D22’s AST-synthesis boundary is guaranteed by code structure—L3/L4 production code
constructs `ast::Expr` / `ast::Stmt` / `ast::Type` only in `src/middle/lower/synth.rs`; pattern
matching, read-only queries, and test fixtures are not synthesis; synthesis points outside
`synth.rs` are rejected by review. The original `scripts/ci/check-synth-boundary.py` and the CI
`synth-boundary` job were deleted under the user’s 2026-11-24 Decision D and are no longer
maintained as an automated scanning gate: heuristic token scanning cannot cover arbitrary legal use
/ alias / re-export forms (successive attempts to close gaps were still bypassed by `pub type`
re-exports), and the maintenance cost exceeds the benefit.

D1/D2: `SynthEnv` and `BindingLocation` are both new internal types introduced by this batch, not
reused existing types. The former centralizes production-side environment operands, named items, and
slot derivation into one explicit parameter. The latter promotes the former `SymbolEntry.local_idx`
into a discriminable Local/Upvalue location. The alternatives—continuing to pass three shared
fields, which cannot prevent nested overwrites or erroneous residue, or passing only `Vec<Operand>`,
which cannot express the correspondence between the anonymous prefix and named captures—are both
rejected for correctness reasons. The new concepts are not synonymous with existing ASTs or
consumption ledgers and require neither import aliases nor conversion tables.

### Disposition of Span-Keyed Problems (the Part SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism One: consumption-coverage assertion (mandatory, lowest cost; superseded by a stronger
form on 2026-10-13).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). The
original supplement to this document proposed making it a **runtime counter** rather than merely a
test. The implemented form is stronger: a **consumption ledger (`ConsumptionLedger<K>`) + a single
settlement point (E-level `W1090`)**, adding no diagnostic code and not relying on an internal
ir_gen counter to prove itself:

```rust
// ir_gen.rs:341 附近
// 记下 typecheck 交了几条计划
self.release_plan_total = type_result.release_plan.drops.len();
// 每次 :1942 命中时 self.release_plan_hit += 1;
// generate_module_ir 尾部（assign_defs 之后）：
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* 新诊断码：E3xxx NLL 释放计划未被 IR 消费 */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a value that should have been dropped was not dropped” may not immediately
crash under YaoXiang’s current value semantics + Arc/Rc model**—which is precisely why it was
silent. Counting turns “silent” into “detectable.”

**Mechanism Two: change Span keys to `DefId` or explicit plan IDs (**landed on 2026-10-13**: RFC-039
D20 / WBS 4.5 implemented this mechanism as a `PlanId(u32)` structural-identity key + mandatory
consumption settlement; see the revision note in §2 of [02](02-stage-contract.md)).** The root fix
is to **not use Span as a key**. `Span` is a source location and may drift because of macro
expansion, `include`, or multi-file merging. `FunctionIR` already has a `def: Option<DefId>` field
(`ir.rs:677`), demonstrating that DefId is available at the IR layer. **Design judgment:** introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, assigned by ownership / overload
resolution when produced, and match IR consumption by `PlanId`. **This belongs to the obligation
ledger (`Obligations`) in 02 and should be implemented together with [02](02-stage-contract.md), not
in this document.**

**Mechanism Three: definition-order dependence in `method_def_ordinals` (independent small item,
**landed on 2026-11-25**).** The field was originally a `HashMap<String, usize>`, with an
unqualified `"{type_name}.{method_name}"` key, assigning `#N` mangled names according to
definition-traversal order. If typecheck’s registration order differs from lowering’s definition
order, methods with the same name may resolve to different targets. **Implemented form:** typecheck
**produces `method_def_ir_names: HashMap<PlanId, String>` once**, at the same point where it
registers overload candidates (bare names or `#N` mangled names), and passes it to IR generation
through `TypeCheckResult`. The key is the **`PlanId` of the method declaration statement** (an AST
structural identity, the same model as D20)—`DefId` is not allocated until the end of lowering in
`assign_defs` and therefore does not exist on the typecheck side. Lowering only reads the name by
declaration identity; **a miss fails loudly with E3005** (it does not fall back to the bare name,
because that would disguise “the definition name and call name did not come from the same source” as
a runnable program). The mutable counter and its write sites were removed, and definition names no
longer have a second derivation entry point.

### Tracking the Three Hard-Coded Discards

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hard-coded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                                                     |
| ------------------ | -------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | The closure upvalue count becomes zero after a `.42` round trip                                                                                 |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **The entire exception table is lost from `.42` artifacts**                                                                                     |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **Global-variable information is lost from `.42` artifacts** (note that `global_names` at `2347` **is** populated, so the two are inconsistent) |

**Design judgment (revised on 2026-10-05, Decision D52): all three are brought into this document’s
scope as P7 batch e (7e), and no separate “issue” is created.** The original judgment—and the reason
it was overturned—is recorded below:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields) and unrelated
   to SSA.~~ This is true, but it is not grounds for exclusion—P7 already modifies `bytecode.rs`,
   and fixing all three together requires only one `VERSION` bump (4→5); splitting them into two
   bumps would be more expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → bumping the version → expanding scope
   from “IR shape” to “artifact format.”~~ The version mechanism already exists (`MAGIC` + `VERSION`
   header, with the reader rejecting mismatches), and `.42` is a build artifact; cross-version
   compatibility is not a goal. The expansion is in effort, not risk.
3. ~~`2315` / `2341` are actually low-risk, and running `.42` directly is a secondary path.~~
   **Overturned**: losing the exception table makes throw/try behavior incorrect when `.42` is run
   directly—throw/try is a core language semantic. A secondary path does not mean silent errors are
   acceptable. This refactoring’s principle is to **leave no core functionality as an unimplemented
   legacy** (`Vec::new() // Not implemented yet` is exactly such a legacy).

**Also retained:** all three must still be recorded in the `Obligations` ledger as “produced but not
serialized” until 7e is complete, to prevent them from becoming a fourth category of “something
nobody knows about.” `2312` lands together with the version bump (D17), and the allocated but unused
opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Knock-On Effects of the IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                                                     |
| ------------------------------------------------------------------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add the `ValueId(u32)` type                                               | File header (`ValueId` definition)            | New (**landed in 7b.1a**)                                                                                  |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**landed in 7b.1a**)                                                       |
| Add a `Phi` variant to `Instruction`                                      | End of the `Instruction` enum (`Phi` variant) | **Variant +1** (**landed in 7b.1a**; the enum now has 75 variants)                                         |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | Field +1 (**landed in 7b.1b**)                                                                             |
| Add the `ValueInfo` struct                                                | `ValueInfo`                                   | New (**landed in 7b.1b**)                                                                                  |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                    | Add `values()` / `values_mut()` accessors (**landed in 7b.1b**, with the same shape as existing accessors) |

The `pub use ...ast::Type` at the file header is **not modified by this document**—it belongs to the
scope of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **converged** single
type.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                                     | Location                              | Nature                                                                                                     |
| ------------------------------------------------------------------------------------------ | ------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or use the “expand to Move instructions at runtime” strategy | `127-554` (currently **66** variants) | Variant +1 or +0 (see below)                                                                               |
| Add a `Phi` arm to `opcode()`                                                              | `558`                                 | Required (`BytecodeInstr` has an exhaustive match)                                                         |
| Add a `Phi` arm to `size()`                                                                | `649`                                 | Required                                                                                                   |
| Add a `Phi` arm to the `From<BytecodeFile>` decoding match                                 | `943-2350`                            | Required                                                                                                   |
| Correct `upvalue_count: 0`                                                                 | `2312`                                | Fixed incidentally by this document (in-memory format; persisting it requires a `VERSION` bump, see below) |

**The bytecode shape of `Phi` requires a design decision**. There are two options (**design
judgment: A is recommended**, for the reasons given in “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded by `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the start of the block,
  selected by predecessor block index. **No new opcode is added**, and the `.42` format does not
  change. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode and predecessor-index encoding, with the interpreter maintaining register
  versions on jumps. **This requires changes to the `executor/` state model + the `.42` format + the
  version number**. The only benefit is size.

The decisive reason for choosing A is that changing the `.42` format triggers version-number issues
and a full compatibility discussion, **whereas A’s size cost is irrelevant under interpretation**
(the interpreter executes Move instructions, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions distributed across `101-1558`)

| Change                                              | Location                                                                                                                                 | Nature                                                                                |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                        | `OperandResolver` (`codegen/operand.rs`)                                                                                                 | Add an arm                                                                            |
| Translation arm for `Instruction::Phi`              | Inside the dispatch match, before `Free` (533)                                                                                           | New branch that expands into a Move sequence (option A)                               |
| The existing 9 NOP degradations **remain in place** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart + UnsafeBlockEnd / `658-660` PtrFromRef + PtrDeref + PtrStore + PtrLoad | **Verified as 9 items**: `655-657` covers 2 variants, and `658-660` covers 4 variants |

**SSA transformation does not reduce the number of NOPs.** These 9 degradations reflect gaps between
IR and bytecode expressiveness, and SSA transformation does not touch them.

#### `src/backends/interpreter/executor/`

With option A, `executor/` requires **no changes**—`Phi` disappears during the `translator.rs` stage
and is never visible to the interpreter. **This is the second decisive reason for choosing A.**

`BytecodeInstr::Switch` at `executor/ops/control.rs:90` has an active implementation, but **no**
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs` (measured: all three names have zero
hits in `ir.rs`)—**this is a reverse gap**: the bytecode layer has an instruction that the IR layer
cannot produce. It does not affect this document, but is recorded here because it shows that “IR →
bytecode” is not surjective.

The dispatch match at `executor/debug.rs:194` likewise requires no changes (under option A, there is
no new opcode).

### Compiler Change List

**File by file and function by function.** All line numbers are change **starting points** (verified
by measurement):

| No. | File                                                                      | Location                                                                           | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                                                     | `Operand` enum definition                                                          | Add/remove `Operand` variants (**landed in 7b.1a**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 2   | `src/middle/ir/ir.rs`                                                     | End of the `Instruction` enum (`Phi` variant)                                      | Add `Instruction::Phi` (**landed in 7b.1a**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 3   | `src/middle/ir/ir.rs`                                                     | `FunctionBody::Code`                                                               | Add `values` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 4   | `src/middle/ir/ir.rs`                                                     | `ValueInfo`                                                                        | Add `ValueInfo` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 5   | `src/middle/ir/ir.rs`                                                     | `values()` / `values_mut()` accessors                                              | Add accessors (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 6   | `src/middle/ir/ir.rs`                                                     | `pub use ast::Type`                                                                | **Do not modify `pub use ast::Type`** (scope of [03](03-type-unification.md); recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 7   | `src/middle/core/ir_gen.rs`                                               | `next_temp_reg` at `884-891`                                                       | Return `Operand::Value`, increasing monotonically without rollback (**landed in 7b.2**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; an independent `next_value` counter was added, starting at 0 for each function body; the three statement-level reclamations only roll back `next_temp` and do not touch it)                                                                                                                                                                                                          |
| 8   | `src/middle/core/ir_gen.rs`                                               | `1954-1959`                                                                        | **Delete** the statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 9   | `src/middle/core/ir_gen.rs`                                               | `3632-3638`                                                                        | **Delete** the statement-level reclamation, including the `result_reg.is_none()` guard                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 10  | `src/middle/core/ir_gen.rs`                                               | `4737-4742`                                                                        | **Delete** the statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 11  | `src/middle/core/ir_gen.rs`                                               | `1745-1759` / `1819-1822`                                                          | Replace with an RAII guard (`generate_method_ir`, entry at `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 12  | `src/middle/core/ir_gen.rs`                                               | `1858-1866` / `2026-2029`                                                          | Replace with an RAII guard (`generate_function_ir`, entry at `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 13  | `src/middle/core/ir_gen.rs`                                               | `2116` / `2180`                                                                    | Replace with an RAII guard (`generate_curry_innermost_func`, entry at `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 14  | `src/middle/core/ir_gen.rs`                                               | `2224-2229` / `2284-2288`                                                          | Replace with an RAII guard (`generate_curry_function_ir`, entry at `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 15  | `src/middle/core/ir_gen.rs`                                               | `2768-2777` / `2820-2851`                                                          | Replace with an RAII guard (`generate_anon_binding_ir`, entry at `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 16  | `src/middle/core/ir_gen.rs`                                               | `4686-4695` / `4767-4774`                                                          | Replace with an RAII guard (`generate_lambda_body_ir`, entry at `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 17  | `src/middle/core/ir_gen.rs`                                               | `1987-2000`                                                                        | Extract the E3014 check into `check_register_budget()`, shared after the accounting at `1798`/`2001` is unified                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 18  | `src/middle/core/ir_gen.rs`                                               | `1798`                                                                             | Change `take_cur_locals(param_types.len())` to `take_cur_locals(self.temp_high_water)`, eliminating inconsistent accounting                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 19  | `src/middle/core/ir_gen.rs`                                               | `248-249`                                                                          | Delete the contract comment (now enforced by the type system) and replace it with a pointer to `verify`                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 20  | `src/middle/lower/loops.rs`                                               | `generate_spawn_for_ir`                                                            | Change the call to `synth::synth_closure` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 21  | `src/middle/lower/expr/spawn.rs`                                          | `generate_spawn_expr_ir`                                                           | Change the call to `synth::synth_closure` and remove the writes/clears of the shared pending fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 22  | `src/middle/lower/expr/closure.rs`                                        | `generate_lambda_expr_ir`                                                          | Read `SynthEnv` from the arguments instead of using `mem::take` on shared fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 23  | `src/middle/lower/{mod,scope}.rs`                                         | Symbol table and `NestedBodyGuard`                                                 | Remove the `closure_captures` field; use `BindingLocation::Upvalue` lexical binding and depth-based restoration (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 24  | `src/middle/lower/expr/{access,closure}.rs` + `query.rs`                  | Variable and namespace queries                                                     | Change `closure_captures.contains_key` to query the lexical upvalue; materialize captures uniformly through `capture_operand` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                              |
| 25  | `src/middle/core/ir_gen.rs` (audit-time path)                             | `341` + `1942`                                                                     | ~~Add Span-keyed consumption counting~~ → covered by 4.5’s `PlanId` consumption ledger (read and accounting in the same operation, settled with W1090); 7c.2 verifies zero triggers, with no code change                                                                                                                                                                                                                                                                                                                                                        |
| 26  | `src/middle/lower/{mod,stmt}.rs`, semantic side `sema/{checker,types}.rs` | Definition name for `generate_method_ir` + authoritative `TypeCheckResult` mapping | Make `method_def_ordinals` read-only (**landed in 7c.3**: typecheck produces the mangled name according to the declaration `PlanId`, and lowering only reads it)                                                                                                                                                                                                                                                                                                                                                                                                |
| 27  | `src/middle/core/ir_gen.rs`                                               | `7269-8014`                                                                        | **Do last**: decompose into a top-level `CallArgs` + 6 `emit_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 28  | `src/middle/core/ir_gen.rs`                                               | `7838-7846`                                                                        | Replace the “fill in 0” fallback with a returned diagnostic                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 29  | `src/middle/lower/synth.rs`                                               | **New**                                                                            | Synthesized-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 30  | `src/middle/ir/verify.rs`                                                 | **New** (first layer of [07](07-equivalence-oracle.md))                            | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 31  | `src/middle/passes/codegen/translator.rs`                                 | Dispatch match                                                                     | Translation of `Instruction::Phi` (**landed in 7b.3**: option A expands it at the **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit Move groups per predecessor, existing `JMP` instructions skip between groups, and incoming-edge jumps are redirected to `(predecessor, convergence block’s first instruction)` at the group’s starting point; the `Phi` arm of `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return slot cannot hold multiple Moves distributed across predecessors, so arrival fails loudly) |
| 32  | `src/middle/passes/codegen/operand.rs`                                    | `36-45` / `72-77`                                                                  | Add an `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`) (**landed in 7b.2**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); removal of the `Temp` arm landed first in 7b.1a)                                                                                                                                                                                                                                                                |
| 33  | `src/middle/core/bytecode.rs`                                             | `127-554`                                                                          | **Unchanged under option A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 34  | `src/middle/core/bytecode.rs`                                             | `558` / `649` / `943-2350`                                                         | **Unchanged under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 35  | `src/middle/core/bytecode.rs`                                             | `2312`                                                                             | Correct `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 36  | `src/backends/interpreter/executor/**`                                    | —                                                                                  | **No changes under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 37  | `src/middle/lower/regalloc.rs`                                            | **New**                                                                            | Linear-scan allocator (**landed in 7b.4**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)` returns `value_slots` and `frame_slots = named_base + max_live`; overflow returns `Budget`, and `finalize_code_body` reports E3014)                                                                                                                                                                                                                                                                                                     |
| 38  | `scripts/ci/check-synth-boundary.py`                                      | **Canceled**                                                                       | The boundary-sentinel script was canceled by the user’s 2026-11-24 Decision D; no replacement is created                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

### Backward Compatibility: Versioning the `.42` Format

**Verified fact:** the `.42` on-disk format **already has a versioning
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (“YXBC”) and
`VERSION: u32 = 4`, writes them in the header (`:342-343`), and validates them when reading
(`:454-467`, reporting "unsupported bytecode version" on a mismatch).

**Design judgment (a direct consequence of option A)**:

- `Phi` is expanded by `translator.rs` into a Move sequence and **does not produce a new opcode**.
- The 83 constants in `opcode.rs` **do not change**, `opcode_name()` (`120`) does not change, and
  the decoding match (`bytecode.rs:943-2350`) does not change.
- The `.42` format’s field layout **does not change**, and `VERSION` remains 4. Old `.42` files can
  still be read by the new binary, and new `.42` files can still be read by the old binary. **SSA
  transformation itself does not require a version bump.**

**The only scenario requiring a version bump is the correction to `2312 upvalue_count`** (Decision
D17: fix it and handle it together with the version bump): add an `upvalue_count` field to
`BytecodeFunction` and have the encoder write it. This **adds a field to the format**, raising
`VERSION` from 4 to 5. The existing reader-side version check rejects old files according to
existing behavior—`.42` is a build artifact (`main.rs:671`), and cross-version read compatibility is
not a goal, so the version bump requires no additional migration mechanism. Allocated but unused
opcode values (`Switch` / `TailCall` in D32/D34) are reclaimed in the same batch.

## Implementation Notes

### Batch Strategy

There are four batches. **Every item within a batch has an independent commit, independent criteria,
and an independent revert.** “Change items” in the table refer to the item numbers in the “Compiler
Change List” above.

| Batch       | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Change items                 | Criteria                                                                                                                                                                   | Rollback point                                                                                                                                                       |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Break multiple definitions: delete the three statement-level reclamations → replace 6 save/restore pairs with RAII guards → unify accounting                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 8–10 (first) → 11–16 → 17–18 | Deleting the reclamations: **C4** (behavioral equivalence + first-layer dominance invariants). RAII and accounting unification are pure moves: **C1**, zero-diff snapshots | One item per commit; `git revert` rolls back an item                                                                                                                 |
| **Batch b** | Switch to the SSA shape: structural changes to `ir.rs` + mechanically change temporary values to produce `Operand::Value` → add the `Phi` arm to `translator.rs` (option A) → linear-scan allocator. Within the batch, proceed serially through the three-level WBS tasks: 7b.1a (`Operand` variant convergence + introduction of the `Instruction::Phi` variant, **landed**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **landed**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **landed**) → 7b.3 (`Phi` arm in `translator.rs` + population of `BasicBlock.successors`, **landed**) → 7b.4 (linear-scan allocator + deletion of the three statement-level reclamations, together with 7a.1, **landed**) | 1–5, 7 → 31–32 → 37          | **C4**: behavioral equivalence + all-green `verify_ssa` + `.42` round-trip tests                                                                                           | The `ir.rs` changes **must occupy one commit by themselves** (the type changes affect compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary → verify the existing consumption ledger (no code change) → make `method_def_ordinals` read-only (the sentinel script was canceled by the user’s 2026-11-24 Decision D)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | 20–24, 29 → 25 → 26          | **C4**: behavioral equivalence; the consumption counter must produce zero triggers on the new diagnostic (otherwise, a real mismatch exists)                               | Independent commit                                                                                                                                                   |
| **Batch d** | Decompose `generate_call_expr_ir` + make the “fill in 0” fallback assertive                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 27–28                        | Pure-move portion: **C1**, zero-diff snapshots. Assertive portion: **C4**, corpus differential + first-layer validator                                                     | Independent commit                                                                                                                                                   |

**There are three dependencies whose order within the batches cannot be changed:**

1. Batch a’s “delete the reclamations” must follow the `ir.rs` changes in batch b (deletion of the
   three statement-level reclamations landed with 7b.4 and the linear-scan allocator, with 7a.1
   executed in the same batch)—first make `next_temp` increase monotonically, and only then does
   `ValueId` have the semantic basis for “unique definition” (that semantic property holds after
   7b.2 changes `next_temp_reg` to produce `Value` and 7b.3 lands the `Phi` arm).
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise, the decomposition will carry
   the old `arg_regs` ordering assumptions into the new file unchanged.
3. Batch d must come last—it depends on the previous two batches having converged the semantics of
   `arg_regs` to “one independent `ValueId` per argument.”

**Position relative to RFC-039:** all four batches fall within **P7** of RFC-039. The criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial**, but there
is no coupling between items 8–10, 11–16, and 17–18 within batch a beyond their ordering.

### Hard Preconditions

**Precondition One: criterion baseline.** The `verify_loose` mode of the first-layer validator in
[07](07-equivalence-oracle.md) **must pass on the existing non-SSA IR** before batch a may begin.

The reason is that SSA transformation uses C4 criteria (behavioral equivalence + invariants). If the
invariant validator itself cannot pass on the existing IR before the refactoring, every acceptance
step in batches a–d lacks executable criteria and can rely only on corpus differentials—yet corpus
differentials are **precisely insensitive** to Defect Classes 1/2/3 (as demonstrated by “Alternative
A” in 07). **This is a real ordering constraint.**

**Precondition Two: convergence of type representations.** [03](03-type-unification.md) must
converge `pub use ast::Type` at `ir.rs:3` and remove the `From<MonoType> for IrType` bridge at
`bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` introduces a third set of type-compatibility
checks.**

### Rollback Strategy

- Each change is one independent commit, and `git revert` granularity equals change-item
  granularity.
- Batch b’s `ir.rs` structural changes are **the only changes that cannot be rolled back
  incrementally** (the type changes affect compilation throughout the repository). Therefore, they
  must **occupy a commit by themselves**, and batch a must already have passed and established a
  revertible baseline before that commit.
- **Do not use a feature flag.** The reason is that adding/removing `Operand` variants cannot be
  isolated by a runtime switch; a feature flag would leave CI permanently untested on one path—a
  recurrence of exactly what RFC-039 records as “test wiring depends on people remembering, and
  silent rot occurs.”

### Expectation Management: Net Increase of 900–1600 Lines

**This must be stated up front, or implementation will be challenged midway with “the line count did
not decrease.”**

| Item                                          | Line-count change                                                                                                                                                                       |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                                | `ir_gen.rs` **8448**                                                                                                                                                                    |
| Delete the three statement-level reclamations | **−15**                                                                                                                                                                                 |
| Replace 6 save/restore pairs with RAII        | Net ±0 (the RAII guard definition adds about +20 lines, while the use sites fall from 22 lines to about 12)                                                                             |
| Change `next_temp_reg`                        | +5                                                                                                                                                                                      |
| Decompose `generate_call_expr_ir`             | **+40–80** (the signature and parameter-passing overhead of the 6 `emit_*` functions; this is the typical cost of a pure-move decomposition—**decomposition increases the line count**) |
| Span-keyed consumption counting               | +15                                                                                                                                                                                     |
| Make `method_def_ordinals` read-only          | +20                                                                                                                                                                                     |
| New `synth.rs` file                           | +80–120                                                                                                                                                                                 |
| New `verify.rs` file                          | +400–600 (the first layer of 07, although within this document’s scope, is counted separately)                                                                                          |
| New `regalloc.rs` file                        | **+500–1000** (measured at 271 lines in 7b.4)                                                                                                                                           |
| **Total**                                     | **`ir_gen.rs` grows from approximately 8448 to 8600–8800; including the new files, the net increase is 900–1600 lines**                                                                 |

**The early expectation that “8448 → 5000–6000” would result was wrong**, and the measured
projection is the opposite. The reason is as follows:

1. **The core benefit of this document is not fewer lines; it is eliminating an entire class of
   defects that “produce wrong values without reporting errors.”** The 6 manual save/restore pairs
   (22 lines) become about 12 lines plus a guard type under SSA + RAII—**the line count is almost
   unchanged, but “forgot to restore” changes from a possibility into a compile error**.
2. **A pure-move decomposition necessarily increases the line count.** Decomposing the 746-line
   `generate_call_expr_ir` into 7 functions repeats the signature in each one (the function already
   has `#[allow(clippy::too_many_arguments)]`, `7268`), along with `&mut self` bindings and error
   propagation. **“Splitting the file to reduce line count” is a misunderstanding**—it reduces
   cognitive load, not physical line count.
3. **The new verifier and allocator are net line increases.** These two components (approximately
   900–1600 lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of line count, this document will most likely produce a **net increase**. The benefit
> lies in **verifiability**—turning defects that “produce wrong values without reporting errors”
> into invariants that **fail when violated**. If line count were used as the acceptance criterion,
> this document would fail; the correct acceptance criteria are those in the “Batch Strategy”
> section.

**This stage will intentionally increase the amount of code in `ir_gen.rs`.** The 2026-10-03
decision canceled all line-count and size gates (see Prohibition Two in
[08](08-maintenance-mechanism.md)), so **no line-count exemption list is needed**. The legitimacy of
the increase is established by the separation-of-responsibilities argument: SSA transformation
extracts “register allocation + definition discipline” from lowering into standalone modules,
trading increased code size for the elimination of the two classes of manual discipline—6 manual
save/restore pairs and semantic reordering of `arg_regs`—that produce “misalignment without errors
and only wrong values.”

## Key Decisions and Rationale

### Three Core Decisions

**Decision One: temporary-value strategy—delete the three statement-level rollbacks so that unique
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, and every temporary slot naturally has exactly one definition point. **The
criteria are consistent, but the guards differ** (`3632` has a `result_reg.is_none()` guard, while
the other two do not), so these hand-copied differences disappear. The cost is increased slot
occupancy—`temp_high_water` already has a 255 upper-bound check (`1992-2000`) as a backstop.

**Decision Two: do not perform mem2reg; perform explicit renaming only.** The reasons are given
under “Value Identification”: YaoXiang’s `&` / reference / ownership semantics make “which slots can
be promoted to registers” a type-system problem that requires the converged type representations
from [03](03-type-unification.md). **Performing mem2reg before 03 means guessing.** Explicit
renaming requires changing only the return-value semantics of `next_temp_reg` and deleting the three
rollbacks, without touching the control flow of any lowering algorithm.

**Decision Three: use option A for `Phi` (IR-only pseudo-instruction expanded into a Move
sequence).** `executor/` and the `.42` format have **zero changes**, limiting the regression surface
to L3. The decisive reason is that changing the `.42` format triggers version-number issues and a
full compatibility discussion, whereas A’s size cost is irrelevant under interpretation (the
interpreter executes Move instructions, not Phi). The cost is that `Phi` is invisible in
`dump_bytecode` (this must be kept in mind when entering the C1/C2 stage, where the third layer in
07 compares it).

### Rejected Alternatives

**A. Perform the complete decomposition at once (SSA + decomposition + `synth` boundary + allocator
simultaneously). Rejected.** This violates 07’s core principle: criteria must be tiered by category.
Mixing all four batches into one PR combines pure-move changes with C4 changes, and snapshot drift
cannot be attributed—when a snapshot changes, it is impossible to tell whether the decomposition
moved something incorrectly or SSA changed it incorrectly. **Rollback granularity degrades from
“change item” to “everything.”**

**B. Split the files first and optimize ordering later. Rejected.** Splitting does not eliminate any
root cause: the 6 manual save/restore pairs remain scattered across 10 files, and the semantic
reordering of `arg_regs` still requires manual reasoning. Worse, **splitting first loses the
opportunity to “establish criteria before making changes”**—the decomposition produces a large
line-level diff that mixes into the diffs of later SSA changes, invalidating review. This has the
same root cause as the 8 orphan test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78
tests that have never run): **with the wrong order, every subsequent step accelerates on an
incorrect foundation.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as “a third path once considered.”** Its form is to leave `ir_gen.rs` unchanged and add a pass that
converts its non-SSA IR into SSA IR. **The advantage** is that lowering remains untouched and
rollback is easy. **The reasons for rejection are**: (1) it does not eliminate the root cause of
Defect Class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the 6 save/restore
pairs remain separated by 59–161 lines. A conversion pass can merge multiple definitions into `Phi`,
but **it cannot repair “inner function names leaking into the outer scope”**—that is an error in the
`cur_locals` save/restore logic that occurs before the IR shape, and a conversion pass cannot see
it. (2) It turns SSA into a second representation—the repository already has the cautionary example
of 3 parallel type representations, and this project’s core lesson is that **“boundaries exist only
in people’s awareness.”** (3) `verify_loose` becomes a permanent burden.

**D. Keep the current state and only add a verifier. Rejected as the sole solution, but it is a
component of this document.** RFC-039’s Alternative B reaches the same conclusion: gates can prevent
regression but cannot repair the existing state. **However, it must be clear what D can
accomplish**: D can **catch instances** of Defect Classes 1/2/3 (provided that the verifier covers
dominance, type consistency, and inner isolation), but it cannot catch Defect Class 4
(`closure_captures` leakage)—that requires a cross-function liveness assertion beyond the function
boundary of `verify(&ModuleIR)`. **Therefore, D is not an alternative to this document; it is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1600 lines** (new `verify.rs` / `regalloc.rs`). **This is not itself
  the risk**—the 2026-10-03 decision canceled line-count and size gates, and scale is addressed
  through separation of responsibilities (see Prohibition Two in [08](08-maintenance-mechanism.md)).
  The legitimacy of the increase is that it replaces the two classes of manual discipline—6 manual
  save/restore pairs and semantic reordering of `arg_regs`—that produce “misalignment without errors
  and only wrong values.”
- **The “unique definition” check in `verify.rs` is meaningless in non-SSA form** (`Operand::Local`
  permits multiple definitions)—07 already records this limitation, and `verify_loose` is its
  degraded mode for this batch.
- **Deleting the statement-level reclamations changes the temporary-slot allocation pattern**
  (executed in 7b.4, together with 7a.1): after deleting the reclamations, slots are reused by the
  linear-scan allocator according to live intervals, so **the allocation result changes**—this is
  exactly why C4 must genuinely run the full corpus; measured, the 403/403 differential is empty.
- **`u8` / 255 slot limit:** the two corpus cases that exceeded the limit after deleting the
  reclamations (`string_ops.yx` with 354 monotonic slots, `list_comp.yx` with 323 monotonic slots)
  are compressed by the allocator to **15 / 24** frame slots; the budget gate now reports E3014 only
  when the post-allocation frame genuinely does not fit (including the function-body declaration
  span), and no longer rejects based on the “cumulative temporary count.”
- **Benefits must also be recorded**: turning “errors that cannot be tested” into “invariants that
  can be tested”; no backend changes (`executor/` and `.42` remain unchanged); no changes to
  `Instruction` semantics (all 76 → 77 variants are retained, as are all 50 `translate_*`
  functions); incremental implementation (each item can be reverted independently).

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> decisions (D1–D50). **This document leaves no pending items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) — the
  overarching architecture; four-layer model, criterion tiers, P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definition of C4
  criteria, `verify_loose` / `verify_ssa`, three criterion layers, and snapshots’ immunity to
  `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the
  `ReleasePlan` `PlanId` refactoring
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; a cautionary example of
  “creating the directory first and then failing to wire it up”

### Code Locations

**IR definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — 7 `Operand` variants
- `src/middle/core/ir.rs:47-533` — 76 `Instruction` variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (the feasibility basis for the
  `PlanId` approach)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect Class 1 (multiple definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for the seven nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — the sole table-extraction point + E3014
  budget point (`MAX_REGISTERS = 255`), combining block splitting / slot allocation / table
  extraction
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — linear-scan allocator (the sole producer of `value_slots` and
  `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamations
  removed in 7b.4; slot reuse is now handled by the allocator
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant
  referenced by 07

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  the clone
- `src/middle/core/ir_gen.rs:7838-7846` — the silent “fill in 0” fallback

**Defect Class 3 (Span keying)** — the following are **audit-time** evidence pointers (after the 4.5
key migration: the `release_plan` / `overload_resolutions` keys have been converted to `PlanId`; see
the revision note in §2 of [02](02-stage-contract.md) for current-state pointers;
`method_def_ordinals` was eliminated in 7c.3 through mechanism three)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of the three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write site of
  `method_def_ordinals` (the key is the unqualified `"Type.method"`; **the field was deleted in
  7c.3**, and this is audit-time evidence)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the transfer structures
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — definition and
  production of `ReleasePlan`
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer transfer
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (implicit ordering dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — the equivalent handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — the two read sites of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — three other `ast::Type` constructions

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — 66 `BytecodeInstr` variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded discards (compare `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (the bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  bound of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current testing state**

- `src/middle/core/ir_gen.rs` — 8,448 lines, 0 `#[cfg(test)]` blocks, 0 `mod tests` modules
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefIds and slot naming, with no IR
  structural invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example per-opcode round-trip criterion; the last test in the file; 1,209 lines / 23 `#[test]`
  functions in total)

**Engineering configuration**

- `Cargo.toml:37-39` — dev-dependencies contain only `criterion` / `proptest`; `[dependencies]`
  contains no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans the `/rfc/draft`
  directory; this document is not under that directory and is referenced by RFC-039 and this
  directory’s index
