# Intermediate Representation SSA-ification

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main RFC-039 text for the four-layer model, grading of acceptance criteria, and execution
> phase order; see the [index in this directory](index.md) for the positioning of each companion
> document.

## Positioning and Scope

This document addresses IR construction discipline at L3. For `src/middle/core/ir_gen.rs` (8,448
lines), it proposes a construction model based on “exactly one definition per value.” **The goal is
not to introduce a new instruction set, but to eliminate three kinds of discipline defects that
produce no errors when they fail**:

1. Six manual register save/restore sites scattered dozens of lines apart;
2. Semantic reordering of `arg_regs` shared by six branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, making cross-layer contracts explicit, splitting
call-expression dispatch, and wiring the IR static verifier.

This document is not responsible for: converging type representations
([03](03-type-unification.md)), stage contracts and the obligation ledger
([02](02-stage-contract.md)), defining and gating equivalence criteria
([07](07-equivalence-oracle.md)), frontend paradigms ([05](05-frontend-paradigm.md)), or cleaning up
dead code and empty-umbrella designs ([06](06-cleanup-inventory.md)). This document repeatedly cites
conclusions from those documents without re-arguing them.

**The criterion category** follows **C4 (IR shape changes)** from
[07 Equivalence Oracle](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, without requiring full equality of IR snapshots. SSA-ification necessarily changes the
IR shape, and forcing snapshot equality would lead the team to weaken the criteria—the exact trap
recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`), with acceptance requiring behavioral equivalence and a green `verify_ssa`. The
internal implementation batch numbers (batches a–d) are distinct from the internal ordering of P7:
the former are this document’s batching strategy, while the latter are RFC-039’s global phases.

## Feasibility Prerequisites

**Verified fact.** The IR already possesses **all structural prerequisites** required for SSA
construction:

| SSA prerequisite             | Current repository state                                                                     | Location                                                               |
| ---------------------------- | -------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                             | `BasicBlock` in `src/middle/ir/ir.rs`                                  |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                               | `FunctionBody` in `src/middle/ir/ir.rs`                                |
| Scope nesting information    | `LocalSlot::scope_depth` (`"0 = 函数参数层，供嵌套作用域重名消歧"`)                          | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Three-address form           | 76 variants, 54 with `dst: Operand`, and 3 with `dst: Option<Operand>`                       | `Instruction` in `src/middle/ir/ir.rs`                                 |
| Per-instruction span         | Every variant has a `span: Span` field                                                       | Same as above                                                          |
| Value types can be annotated | `LocalSlot::ty: MonoType`                                                                    | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Slot-count upper-bound check | Slot-table extraction and the E3014 budget gate share a single point (`MAX_REGISTERS = 255`) | `finalize_code_body` in `src/middle/lower/scope.rs` (from 7b.4 onward) |

**Without these prerequisites, the cost estimate in this document would be completely different.**

**The two decisive points are as follows.**

**First, SSA-ification is not a new capability; it removes a capability dependency.** In the current
design, temporary slots are **deliberately designed to be defined multiple
times**—`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`, explicitly rewind `next_temp` in
three places. The first step of SSA-ification is to **delete these three rewinds**, allowing
`next_temp` to increase monotonically. Once they are deleted, **every temporary slot naturally has
only one definition point**—unique definition changes from “a rule that must be checked” to “a
mathematical consequence of the construction model.” Batch a only converged the save/restore surface
(see “Defect Class 1”); the three rewinds were removed together with 7b.4 in batch b (the allocator
took over slot reuse) on 2026-11-22.

**Second, the slot-count upper bound is a hard check; deleting the rewinds briefly caused slot
counts to become unmanageable (resolved by 7b.4).** The budget gate in `src/middle/lower/scope.rs`
reports E3014 when the count exceeds 255. Running the full corpus after deleting the three
reclamation sites produced two corpus cases that crossed the limit and triggered
E3014—`src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; peak slot counts for the
remaining corpus rose from 51 to 211. **After 7b.4 landed:** the linear-scan allocator reuses slots
according to live intervals, and the two corpus cases now use **15 / 24 frame slots** (344 / 304
values), while the remaining 403/403 corpus cases remain behaviorally equivalent. From this point
onward, E3014 means only “frame slots after allocation exceed 255.”

**The combined meaning of these two points is:** lowering already maintains all the bookkeeping
information required for SSA, but merely **discards it**; 7b.4 makes the allocator responsible for
slot reuse and removes reclamation. The main SSA-ification work lies in splitting
`generate_call_expr_ir` and making cross-layer contracts explicit, not in building new IR
expressiveness.

### Prerequisite: Unification of Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, while `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The third representation on the serialization side is `ir::Type`,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR contains two type representations simultaneously, and one of them directly
`pub use`s the AST type.**

SSA-ification must add a `Phi` variant, record a type for every SSA value (for the “type
consistency” invariant in 07’s verifier), and determine whether the types of two predecessors are
compatible at a CFG convergence point. **If type representations have not yet converged, SSA will
require a type-compatibility check for every representation**—and the third representation will grow
accordingly.

This is not a question of which task should come first; it is a question of **which order will avoid
producing three sets of type-compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first complete the convergence of
`pub use ast::Type` at `ir.rs:3` and remove the bridge at `bytecode.rs:2353`. Item 6 in the change
list explicitly records that `ir.rs:3` **is not changed by this document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not merely “an ugly file that happens to work.” It is a file whose **correctness
depends on people reviewing every line**:

- The file contains **0 `#[cfg(test)]` blocks and 0 `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares the `bytecode`, `def_assign`, and `local_slots` modules,
  with 29 tests in total. `def_assign.rs` and `local_slots.rs` **positively** cover ir_gen’s DefId
  allocation and local-slot naming (at sentinel level), while the four references to ir_gen in
  `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **entirely negative assertions**—“ir_gen does
  not produce,” “ir_gen also does not construct,” and “the ir_gen frontend does not construct the
  corresponding IR.”

In other words: ir_gen has 29 sentinel-level tests pinning local behavior, but **no IR structural
invariant validation** (dominance, unique definition, jump targets, type consistency)—none of the
three modules declared by `middle/core/tests/mod.rs` positively asserts the overall IR shape.

This file also happens to contain several classes of defects that **compile successfully, pass every
check, and let the program continue running with incorrect values**. Their common property is:
**failure produces no error**.

This is where the highest risk lies. The problem is not “the function is too long” or “the naming is
confusing,” but that **there is no machine-enforceable mechanism for the core invariants of this
file**.

The contract “save state before generating a nested function body, then restore it afterward;
otherwise names from the inner function will leak into the outer function” is enforced by the type
system: `NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the
generator, and frame removal and restoration each have exactly one implementation (see “Defect Class
1”).

**Why adding a verifier is not enough**: a verifier can **catch** these defects, but it cannot catch
their **causes**. The cause is not “someone missed something during review,” but:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator deliberately depends on this property.**

The concrete evidence is three statement-level register reclamation sites (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`). At the end of every statement, they **rewind** `next_temp` to
the greater of the last named slot and `temp_floor`
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and the number and
order of writes and the live intervals are entirely determined by a single manual decision: when to
rewind.

Under this shape, “unique definition” is not a rule that can be introduced incrementally; it
**directly conflicts with the register-allocation strategy**. SSA-ification is therefore not a
matter of adding a check to existing code, but of **replacing the allocation strategy that depends
on multiple definitions**. The good news is that the replacement cost is low—see “Feasibility
Prerequisites” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (the Three Reclamation Sites Were Removed in 7b.4)

**Verified fact.** There are seven entry points that require save/restore when entering a nested
function body. Batch a (7a.2) converged them into one RAII guard: `NestedBodyGuard<'a>` in
`src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, with `Drop` unconditionally
restoring state and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Restore scope                                       |
| ------------------------------------ | ---------------------------------- | ----------------------------- | --------------------------------------------------- |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (three counters are not restored) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                       |
| Nested function-definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                  |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |

- Frame removal occurs only in the two constructors (`enter` / `enter_decl`), and restoration occurs
  only in `Drop`. The guard exclusively holds `&mut` access to the generator for its lifetime,
  leaving no second `&mut` available in scope; `?` and early returns all restore state through
  `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`. From 7b.4 onward,
  `next_temp_reg` maintains only the named high-water mark; temporary-value slots belong to the
  linear-scan allocator, and `temp_floor` was removed together with the three reclamation sites.
- `enter_decl` does not restore the three counters: after a function or method body returns to the
  module-initialization flow, subsequent definitions continue from the child body’s final value
  number. This is the constraint for zero drift in C1 snapshots and the meaning of “declaration
  scope restores only the table.”

**Unified convention and single budget point (from 7b.4 onward):** `finalize_code_body` in
`src/middle/lower/scope.rs` is the only slot-table extraction and budget point. It derives the table
from `frame_slots = named_base + max_live`; an out-of-range result reports E3014 with the
function-body span; and `MAX_REGISTERS = 255` is defined in exactly one place in the repository. All
six function-body forms (function / method / lambda / anonymous binding / middle curry layer /
innermost curry layer) finish through it, so there is no second convention for “how many slots a
function body should have.”

**The three statement-level reclamation sites have been removed (landed with 7b.4 as part of 7a.1,
2026-11-22):** the reclamation logic had been copied manually into `generate_function_ir` /
`generate_block_ir` (with a `result_reg.is_none()` guard) / `generate_lambda_body_ir`. After the
allocator took over slot reuse, reclamation degenerated into an identity operation, so even the
guard, the `temp_floor` / `saved_temp_floor` bookkeeping, and the loop-entry/loop-exit bookkeeping
in `loops.rs` were removed. Module init / top-level statements still follow the old monotonically
increasing slot path with `value_mode=false` and do not run the allocator; their slot numbers and
table lengths shift with the number of named slots in the function body (three snapshots measured;
behavioral equivalence is covered by 403/403).

**Consequences in the current shape:** the guard exclusively holds `&mut`, and restoration has only
one implementation in `Drop`, leaving no place to write a missed restore. The temporary-slot
dependency on defining one slot multiple times has disappeared with removal of reclamation—value
slots are reused by the linear-scan allocator according to live intervals, providing the
construction-side foundation for `verify_ssa`’s unique-definition criterion.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified fact.** `generate_call_expr_ir` is located at `ir_gen.rs:7269-8014`, spans **746 lines**,
and is a single function. Its signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (along with `_expr` / `result_reg` / `instructions`
/ `constants`). Internally, six branches share the same `Vec<Operand>` and perform **semantic
rewriting**:

| Line        | Semantics                                                                                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `7395`      | Namespace call: flattened by `slots` and then collected again—named arguments are reordered                                                                  |
| `7724`      | Struct construction: each slot in `final_args` is unwrapped and then collected again—fields are reordered                                                    |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **adds a new register containing 0** for each missing slot (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—cloned before emitting instructions                                                                       |

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
reports E1010. For runtime safety, fill in 0 (the normal path should never reach here).”

**This design has two kinds of risk**:

1. **Misalignment produces no error, only incorrect values.** Each of the six branches independently
   determines the final order of `arg_regs`, and an incorrect ordering assumption anywhere will not
   cause a panic.
2. **Normalized snapshots are immune to this defect.** 07 already records this limitation: after the
   snapshot tool renames temporary values by “first occurrence order,” “the third argument should
   have used the fifth register” is identical in the snapshot. **This class of defect does not even
   have snapshot coverage at the C4 stage.**

**Additional verification: the struct-construction path has no duplicate-slot guard when
reordering.** `final_args` at `7724` is filled by “positional arguments occupy their slots first,
then named arguments overwrite them by field name,” but **there is no duplicate-slot check such as
`slots[idx].is_some()` before overwriting**—unlike the namespace path at `7381`. When a named
argument silently replaces a positional argument, there is no panic or diagnostic. Furthermore, the
**evaluation order of arguments** (source order) diverges from the final `arg_regs` order
(field-declaration order) here. The `fill_missing` assertion work in batch d must also cover the
duplicate-slot check; otherwise, the silent replacement in the struct-construction path will survive
unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified fact at audit time.** The repository had **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                                 | Production                                                                                                                      | Transfer                                                                              | Consumption                                                                                                                    |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                                   | `ownership.rs:31` (the `ReleasePlan::drops` field), `build_release_plan` (dispatches `walk_stmt` by statement identity)         | `typecheck/types.rs:38` ← passed by `checker.rs` ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, the sole entry point; settlement-complete marker `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger in the same structure, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` only passes it through and merges it) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held in an `ir_gen.rs` field    | `ir_gen.rs:7697` (with a binding), `7772` (without a binding) (`OverloadResolutions::get`; settlement-complete marker `:1097`) |

The form at audit time (`ir_gen.rs:1942`, **now absent**) was:

```rust
// NLL Release: insert a Drop instruction at the statement boundary
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some`—**if the lookup fails, that is all there is to it: no else branch, no assertion, and
no count**. If the two sides compute spans differently anywhere, the **Drop instruction silently
disappears with zero errors**. The Drop sequence of refinement types is the core carrier of this
project’s ownership semantics (`ownership.rs:26-32` describes itself as the “NLL precise release
plan”), yet its lowering had been silent.

`7420` / `7493` have the same form (`if let Some(mangled) = self.overload_resolutions.get(span)`),
causing overload resolution to fall back to the default binding.

**Disposition (landed in 4.5):** both keys were changed to the structural identity `PlanId`, and
consumption now **records a hit at the same time it is read**. A non-empty difference set is
reported at the sole settlement point as **E-level `W1090`**—“not found” is no longer silent. See
[02](02-stage-contract.md) §2 revision note (4.5) for the implemented form and criteria. The line
numbers here (`1942` / `7420` / `7493`) are retained as audit-time evidence and are not pointers to
the current implementation.

> **`method_def_ordinals` is not span-keyed (**eliminated in 7c.3**).** It was originally a
> `HashMap<String, usize>` that assigned `#N` suffixes according to the traversal order of
> definitions by `base_name`. Its risk was **different**: order-dependent definitions—if generation
> order differs from typecheck registration order, the same function name may resolve to a different
> target. See mechanism three under “Additional Mechanisms for Span-Keyed Problems” for the
> implemented form: typecheck produces the definition name once according to declaration identity,
> and lowering only consumes it.

### Defect Class 4: Implicit spawn/lambda Handshake (Eliminated in 7c.1)

**Eliminated.** spawn and spawn-for previously required callers to write `pending_env_vars` /
`pending_env_names` first, after which a synthesized Lambda would consume them; closure bodies
separately used `closure_captures` to decide whether a variable read an upvalue. Three fields placed
correctness at the mercy of call order and manual cleanup: a nested synthesized closure could
mistake an outer local slot with the same name for a captured value, and an early return while
generating a closure body could leak capture state into unrelated later lowering.

7c.1 converged synthesized AST into `middle/lower/synth.rs`, replaced the three shared fields with
explicit `SynthEnv`, and registered named captures as lexical upvalues. When exiting a nested frame,
state is restored according to symbol-table depth, and error paths no longer leave capture state
behind. See “`synth.rs` Boundary” for the implemented structure.

### What SSA Will Not Resolve (Listed Honestly)

| Problem                                                            | Location                                                                                                                                                   | Why SSA does not resolve it                                                                                                                                                                                                                               |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                                       | `ir_gen.rs:5390-5687`, **298 lines**, recursively calls itself 4 times (`5473` / `5504` / `5619` / `5640`), with external call sites at `5270`             | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **intrinsic to the algorithm**. SSA only turns the `JmpIfNot` instructions produced by guards into `Phi`; it does not reduce the number of guards |
| `eval_const_expr` complexity                                       | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)                                    | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                                                           |
| Span-keyed failure (**eliminated**)                                | Audit-time evidence: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`); after 4.5, `PlanId` keys + mandatory settlement                    | **SSA does not touch this layer at all** (the conclusion remains unchanged). It was independently resolved by the obligation ledger in [02](02-stage-contract.md); see mechanism two under “Disposition of Span-Keyed Problems”                           |
| Number of `Instruction` variants                                   | 75 (the `Instruction` enum in `ir.rs`; includes the `Phi` introduced by 7b.1a)                                                                             | SSA adds `Phi`, a **net change of +1** (already landed). A variant maps to a bytecode opcode or is degraded to NOP; SSA does not reduce it. Under option A, `Phi` is an IR-only pseudo-instruction and adds no opcode                                     |
| `method_def_ordinals` order-dependent definitions (**eliminated**) | Audit-time evidence: `ir_gen.rs:224` / `369` / `1692-1702`; implemented in 7c.3, see mechanism three under “Additional Mechanisms for Span-Keyed Problems” | Similar to span keying but with a different cause; definition names are now produced by typecheck according to declaration identity, and lowering only consumes them                                                                                      |

## Target Design

### Definition of the SSA Shape

**Design judgment.** The target shape is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has 5 variants. **Repository-wide measurements** (hit counts obtained
by summing `git grep -c` per file over the full `src/` tree, including tests; constructor counts are
value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                      | Disposition                                                                                           |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                              | **Retained**, unchanged                                                                               |
| `Value(ValueId)`    | 7 files / 26 hits; introduced by 7b.1a with **0 production constructor sites**, and from 7b.2 onward `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`)            | **Retained** (introduced in 7b.1a)                                                                    |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                              | **Retained but split semantically** (see below)                                                       |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                | **Retained** as the implicit definition at function entry (parameters are the definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                               | **Retained**, unchanged                                                                               |
| `Temp(usize)`       | **0 constructor sites** (its match arm only exists in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, and the variant was removed with it)                                  | **Removed** (dead variant, 7b.1a)                                                                     |
| `Label(usize)`      | **0 constructor sites**                                                                                                                                                                          | **Removed** (dead variant, 7b.1a)                                                                     |
| `Register(u8)`      | **0 constructor sites** (the comment at the original variant declaration said “Added for codegen,” but codegen uses `OperandResolver` in `codegen/operand.rs` and never constructs this variant) | **Removed** (dead variant, 7b.1a)                                                                     |

`Local(usize)` is split into two variants because it currently carries **two mutually exclusive
semantics**—“the storage location of a named variable” (which may be written multiple times) and “a
compiler temporary register” (which should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within a function, defined exactly once
    Value(ValueId),
    /// Named local variable storage slot: addressable, multiple definitions allowed (non-SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing and never rewound**. The split and
`ValueId` were landed by 7b.1a (the `Operand` enum in `src/middle/ir/ir.rs`). The production side
(`next_temp_reg` changed to produce `Operand::Value`) was landed by 7b.2; see “Production Form”
under “Value Identification Method” for details.

#### `Phi` as a New `Instruction` Variant

```rust
/// Convergence point: each predecessor basic block provides one definition of this value
/// incoming is sorted in ascending order of predecessor block label (guaranteed during construction to ease normalization)
Phi {
    dst: Operand,                                  // Must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

There are three construction constraints (see the first layer in [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of the block`.
2. The label set of `incoming` equals the set of labels of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. All `incoming` values have the same `MonoType` (**depends on 03’s type representations having
   converged**).

The implementation status of `verify_ssa` (the first three constraints were completed in 7b.1b):
constraints 1 and 2 require the value table and block-level CFG and were landed in **7b.1b** as
separate criteria for “block-level predecessor count” and “predecessor label set.” The predecessor
set is inferred from `BasicBlock.successors` (`block_predecessors` in `src/middle/ir/verify.rs`); a
count mismatch reports only the count, while a set mismatch reports the set, so the same defect is
not reported twice. The per-input type-consistency constraint for constraint 3 landed concurrently,
taking types from the value table entry `values[v].ty` and **reusing the existing
`types_compatible`** (no third type-compatibility check was introduced; convergence of 03’s type
representations remains “prerequisite two (hard),” at which point `ValueInfo::ty` will still be
`MonoType`). All three criteria use the violation category `InvariantKind::PhiConsistency`.

**Three forms in which the type is unavailable (explicitly registered, never silently allowed):** an
empty value table means the basis for judgment does not exist (the constant form used by module init
/ top-level statements, the `value_mode=false` path); a non-empty value table with an out-of-range
`ValueId` is a violation; and a non-`Value` operand (`Const` / `Local` / `Arg` / `Global`) has no
value-table type source, so no judgment is made.

**Measured prerequisite (fulfilled in 7b.3):** before 7b.3, every production construction of
`BasicBlock.successors` in the repository used `Vec::new()` (block-level predecessors were always
empty, so the predecessor-count and predecessor-set criteria were initially covered only by
synthetic fixtures). **From 7b.3 onward, block-level `successors` are genuinely populated by
`partition_blocks` in `src/middle/ir/ir.rs` at every production construction site** (all 6 in
`lower` + 1 synthesized constructor body in codegen pass through this sole entry point; loop and
branch blocks produced by `lower` have multiple blocks, multiple outgoing edges, and back edges).
Consequently, the predecessor-count and predecessor-set criteria operate on real data from real
IR—the direct-driver tests on true lowering output pin down this registration-to-consumption loop.
`Phi` still has 0 production constructor sites (production belongs to a later batch), and when
implemented will be directly constrained by this criterion.

7b.1a only introduced the variant: there were 0 lowering production constructor sites (and there
remain 0; production belongs to a later batch). The consumer side in `translator.rs` was landed in
**7b.3**: under option A, each predecessor is expanded at block entry into a sequence of `Move`
instructions (the group is placed at the convergence block’s entry, existing `JMP`s skip between
groups, and incoming-edge jumps are redirected to the group’s start according to the predecessor).
The `Phi` arm of `translate_instruction` is an unreachable E8001 defensive arm (`executor` never
sees `Phi`).

**Disposition of the guard issue at `3632-3638`:** the reclamation in the current
`generate_block_ir` has an `if result_reg.is_none()` guard (the comment at `3626-3631` explains that
a block containing only expression operands may have outer sibling argument temporaries that live
across the block). After SSA-ification, this guard is **no longer needed**—temporary values are no
longer rewound, and a sibling argument that lives across blocks holds its own `ValueId`, which
cannot be affected by a rewind. **This is one place where SSA-ification can genuinely simplify the
code rather than merely rename it.**

#### Value Identification Method

**Design judgment:** do not perform mem2reg, do not introduce virtual-register numbering, and
**perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots can be promoted to registers” a
  **type-system problem**, not a CFG problem. It has been verified that `LocalSlot::ty: MonoType` in
  `ir.rs` carries complete type information, but determining “whether this `MonoType` can be
  promoted” requires the converged type representation from 03 and coordination with refinement-type
  rules. **Performing mem2reg before 03 is guessing.**
- Explicit renaming requires changing only the return-value semantics of `next_temp_reg()`
  (`next_temp_reg` in `src/middle/lower/scope.rs`) and deleting the three rewinds; it **does not
  touch the control flow of any lowering algorithm**.

Value-table form (**landed in 7b.1b**): `FunctionBody::Code` gains `values: Vec<ValueInfo>`, where:

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // Label of the basic block containing the definition
}
```

The accessors have the same shape as existing accessors (`impl FunctionIR` in
`src/middle/ir/ir.rs`): `values()` is read-only and returns an empty table for TypeDecl (mirroring
`locals()`); `values_mut()` is mutable and panics for TypeDecl (mirroring `blocks_mut()`).

**Production form (landed in 7b.2):** `next_temp_reg()` returns `Operand::Value(ValueId)` and
registers one `ValueInfo` entry in the current function body’s value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). The implemented details are:

- **Counter isolation:** `ValueId` is issued by an independent `next_value: u32` counter. Each
  function body starts at 0, increases monotonically, and is never rewound; `NestedBodyGuard` saves
  and restores it together with the value table (one set per function body). The three
  statement-level reclamation sites only rewind `next_temp` (slots) and do not touch `next_value`;
  each curry layer resets independently (one `FunctionIR` per layer). Module init / top-level
  statements have no value table—when `value_mode=false`, `next_temp_reg` falls back to the old
  `Local` slot allocation, leaving the init sequence unchanged.
- **Type source** (only information already available to lowering; no new type decider is
  introduced): literals use `ConstValue::mono_type()` (converged with the verifier’s constant-type
  mapping into a single source); variables/calls query the typecheck result and bound-function
  return type at the `generate_expr_ir` entry point;
  comparison/logical/string-family/forward-propagation forms are filled in according to instruction
  semantics when the function body is finalized. When unavailable, the type is **explicitly unknown
  `MonoType::TypeRef("?")`** (no silent `Int(64)` placeholder; `LocalSlot.ty` is never read). Across
  232 snapshots, 9,425 values were measured: 7,080 had real types and 2,345 were explicitly unknown
  (24.9%), with unknown types concentrated in call, field, index, and container-construction forms.
- **`defining_block`:** takes the current block label at generation time. **Block splitting landed
  in 7b.3** (a true lowering `while` body now consists of multiple blocks), but this field still
  takes the generation-time `cur_block_label` (currently always 0), so it does not yet agree with
  the actual defining block. When 7b.4 regenerated the 232 snapshots, this field was not changed
  (the `@block` values in the `values` section did not drift). Its correction must occur in the same
  batch as regeneration of the `values` section and is **explicitly deferred to the `Phi` production
  batch**.
- **`debug_name`:** always `None` (named variables remain `Local` and are not migrated to the value
  table).
- **Indexing:** `value_slots: Vec<usize>` (index = `ValueId`) is parallel to `values` and records
  the value-to-frame-slot mapping. From 7b.4 onward, it is recomputed by the linear-scan allocator
  in `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`); codegen’s `OperandResolver` only consumes it and does not
  allocate.

`values` and `locals` coexist: `locals` holds addressable named variables (which may be defined
multiple times), while `values` holds single-definition SSA values. **The existence of `locals` does
not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Alignment with `LocalSlot::origin`:** these are **two dimensions** and must not be conflated.
`LocalSlot::origin` (the **slot-identity** field added by origin work 2.1.4, recording the
generation frame `FrameId` to which the slot belongs) answers “which frame owns this slot,” while
`ValueInfo.defining_block` answers “in which basic block is this SSA value defined?” Value identity
belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`; 7b.1b only adds the value
table and **does not change `LocalSlot::origin`** (measured as zero changes).

#### Change-Size Estimate

| Item                                                             | Estimate                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` refactoring                                      | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Deleting the three statement-level reclamation sites             | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Converging 6 save/restore sites into RAII                        | Net ±0 lines                          | See “Itemized Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `ir.rs` structural changes                                       | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **the first two landed in 7b.1a and the latter two in 7b.1b** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Adding a `Phi` arm to `translator.rs`                            | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `executor/`                                                      | **Zero changes**                      | Option A: `Phi` is an IR-only pseudo-instruction expanded into a `Move` sequence by `translator.rs` and never enters bytecode (decision three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Call sites changing temporary values to produce `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (`git grep -c` summed per file over the full `src/` tree, including tests): `Operand::Local(` has 415 occurrences / 28 files, of which `src/middle/lower/` has 332 occurrences / 14 files; the single production point for temporary values is `next_temp_reg`. **7b.2 measurement (baseline revision retest and net landed change):** `next_temp_reg()` has 140 call sites / 14 files; the `result_reg: usize` parameter chain spans 35 functions + 3 `Option<usize>` result slots; `src/middle/lower/` contains 342 `Operand::Local(` occurrences; the code-pen diff for `src/` totals +23319/−12495, of which regeneration of 232 IR snapshots accounts for +21866/−11950, and source plus tests total +1453/−545 (mechanical replacement, normalize rendering, mono projection, and six direct-driver tests; also includes 5 generated error-code artifacts committed with the pen) |

### Itemized Disposition Table

| Defect class                                                     | Eliminated by SSA          | Disposition                                                                                                                                                                                                                                                                                                                                                                                                        | Criterion                                                                                                       |
| ---------------------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore**              | **Yes (resolved in 7b.4)** | The 7 nested-function-body entry points are converged into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the counters), turning “forgot to restore” into a compile error; the three statement-level reclamation sites were removed with 7b.4 (the allocator reuses slots by live interval), and temporary values no longer occupy `next_temp` | First-layer “inner isolation” invariant in 07 + `verify_loose`/`verify_ssa`                                     |
| **1b. Inconsistent convention**                                  | **Yes (landed)**           | Unified in `finalize_code_body` in `src/middle/lower/scope.rs`: derive the table from `frame_slots`; the E3014 budget check and table extraction share one point and carry the function-body span; all six function-body forms are covered; `MAX_REGISTERS` is defined in exactly one place in the repository                                                                                                      | Corpus differential + zero snapshot drift                                                                       |
| **1c. Criteria for the three statement-level reclamation sites** | **Yes (landed in 7b.4)**   | The three reclamation sites + the `result_reg.is_none()` guard in `generate_block_ir` + the `temp_floor` bookkeeping have been removed; slot reuse is delegated to the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                             | C4: behavioral equivalence (403/403) + invariants                                                               |
| **2. `arg_regs` semantic reordering**                            | **Partially**              | SSA turns “using the wrong register” into “using the wrong value ID,” which can be caught by dominance checking, but **the reordering logic itself remains**. Actual elimination depends on function decomposition: each of the six branches pushes independently, while ownership of `arg_regs` remains at the top of the function                                                                                | First-layer dominance invariant (snapshots are immune)                                                          |
| **3. Span-keyed failure**                                        | **No**                     | SSA does not touch it. See “Additional Mechanisms for Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                         | `test_release_plan_spans_consumed` in 07                                                                        |
| **4. Implicit order dependency**                                 | **No**                     | See “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                                          | Guaranteed by code structure and review; the original gate script was canceled by user decision D on 2026-11-24 |
| `compile_pattern` at 298 lines                                   | No                         | See “What SSA Will Not Resolve”                                                                                                                                                                                                                                                                                                                                                                                    | —                                                                                                               |
| `eval_const_expr` at 191 lines                                   | No                         | See “What SSA Will Not Resolve”                                                                                                                                                                                                                                                                                                                                                                                    | —                                                                                                               |
| 75 `Instruction` variants                                        | No (+1, landed)            | `Phi` was introduced by 7b.1a; under option A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                               | —                                                                                                               |

### Register Allocator: Build Our Own or Use an Existing Crate

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **0 constructor sites throughout the repository**—the variant is no
   longer in the `Operand` enum in `ir.rs` (removed by 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper limit of 255 (anything larger
   reports E3014). `Operand::Value` is resolved by the allocator introduced in 7b.4 via
   `value_slots` to a frame slot (`>255` reports E3014; no mapping reports E3017 with the
   instruction span).
3. `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50 `translate_*`
   functions distributed over `101-1558`) emits `BytecodeInstruction::new(opcode, operands)`, with
   opcodes from `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter’s opcode-dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Assessment of three options**:

| Option                                               | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)    | **Not adopted.** Both take rustc’s `Function` trait plus its MIR `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would have to implement the complete trait (CFG, liveness, spill slots, scratch space). **500+ lines of adapter code buy zero benefit**, because there is no real register file to color. Moreover, rustc itself uses only linear scan at this point (`enable-llvm` disabled)                                                                                                                          |
| `graph-alloc` family                                 | **Not adopted.** Same reason, and the API is less stable                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| **In-house linear scan (simplified Chaitin-Briggs)** | **Adopted (landed in 7b.4).** YaoXiang actually needs a mapping from each `ValueId` to a non-conflicting `u8` stack slot according to its live interval (the `OperandResolver` already has a `u8` contract and the E3014 limit). Implemented in `src/middle/lower/regalloc.rs`: block-level `successors` backward dataflow, closed intervals, Phi predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`, with overflow returning `Budget`, which `finalize_code_body` reports as E3014 with the declaration span |

**Landing measurement (7b.4):** the two corpus cases that previously exceeded the limit (354 / 323
monotonic slots) now use **15 / 24 frame slots** after allocation (344 / 304 values), and the
remaining 403/403 corpus cases are behaviorally equivalent. From this point onward, E3014 means only
“frame slots after allocation exceed 255.” The back-edge fallback counter is not currently exposed
(`wrap_count` is calculated and discarded); the `Phi` production batch must expose and retest it
first (alongside `defining_block`). **Overflow handling**: `u8`/255 remains an error handled by the
budget gate—this document does not resolve the case of “a large function has insufficient registers”
(a downstream consequence of `compile_pattern` recursion depth).

The dev-dependencies at `Cargo.toml:37-39` include only `criterion` / `proptest`; `[dependencies]`
contains no regalloc / petgraph / graphlib. Introducing any such crate would add a new dependency
surface.

### Decomposition Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA shape and criteria must exist first**. The reason is that
decomposition itself is C1 (pure movement, judged by zero-diff snapshots), but only after `arg_regs`
semantics have been eliminated by SSA to the point where “each argument has an independent
`ValueId`” will decomposition avoid carrying the old ordering assumptions into the new files.

**Core design constraint: ownership of `arg_regs` remains at the top of the function, and the six
branches only `push`; they never reorder.**

```
generate_call_expr_ir (746 lines)
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

The “fill in 0” logic at `7835-7848` (`7838-7846`) moves down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the comment
claiming that “the normal path should never reach here” is replaced with an assertion**: if any slot
is missing and typecheck has not reported E1010, return a diagnostic directly. **This turns a silent
fallback into a hard error and is the most direct improvement this document makes to Defect
Class 2.**

**Criterion:** the pure-movement portion (where the SSA shape is unchanged) uses zero-diff
snapshots; assertion-based `fill_missing` is C4 and uses corpus differentials + the first-layer
verifier.

### `synth.rs` Boundary: Turning an Implicit Callback into an Explicit Synthesized-AST Boundary

**Implemented form:** `src/middle/lower/synth.rs` is the only module in L3/L4 production code
permitted to construct `ast::Expr` / `ast::Stmt` / `ast::Type`. Pattern matching, read-only queries,
and test fixtures are not synthesis. The current production synthesis points are spawn’s Lambda/Stmt
and spawn-for’s Lambda, all of which pass through this module.

AST nodes do not carry lowering environments, so `SynthEnv` is not an AST field, and AST helpers do
not pretend to consume it. It contains runtime environment operands and named capture names, which
the caller passes explicitly with the synthesized node to `generate_expr_ir_with_env`:

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

- spawn and spawn-for construct `SynthEnv` and pass it explicitly with the synthesized Lambda into
  lowering; they no longer prewrite `pending_env_vars` / `pending_env_names`. The total number of
  environment slots is derived from `env_vars.len()`, and named upvalues start at the offset after
  the anonymous prefix; consequently, spawn-for’s anonymous iteration value, outer captures, and
  parameters each occupy their intended position.
- Ordinary Lambdas complete the environment from the existing free-variable collector. List
  comprehensions, nested Lambdas, and nested spawns are traversed by the same collector, while
  generator variables and nested parameters are excluded by lexical scope.
- Closure bodies register `SynthEnv` as a `BindingLocation::Upvalue` in the existing symbol table.
  Variable reads and namespace-shadowing queries share this lexical binding and no longer read
  `closure_captures`.
- A nested closure first reads the outer upvalue through `capture_operand`, then passes the value
  into the inner environment; it cannot mistake an outer local slot with the same name for a
  captured value.
- `NestedBodyGuard` records the symbol-table depth on entry and truncates back to that depth on exit
  (including early return through `?`), so capture bindings do not leak into later unrelated
  lowering.
- Sema’s expression-body function check parameterizes the existing inputs with
  `body_stmts + expression_body + span`, directly calls `check_expr`, and reuses the tail validation
  rather than synthesizing a Block/Stmt/Expr; there is no sema→middle/lower reverse dependency or
  additional architectural exception.
- SpawnFromList shares the `env + [anonymous item]` argument layout in Embedded and Standard and
  injects upvalues from the same environment. The anonymous iteration value is fixed at env[0],
  named captures are read after the anonymous prefix, and parameters start after the entire
  environment.
- The dual-runtime behavior test explicitly invokes main() and separately asserts, inside the
  spawn-for task, both the no-capture item value and the item-plus-outer-capture form; the
  no-capture form also covers the regression restored by 74febd1f. IR assertions and actual
  execution are recorded separately, and zero drift across the existing 403 corpus cases does not
  masquerade as behavioral coverage of this form.

**Criterion:** D22’s synthesized-AST boundary is guaranteed by code structure—L3/L4 production code
constructs `ast::Expr` / `ast::Stmt` / `ast::Type` only in `src/middle/lower/synth.rs`; pattern
matching, read-only queries, and test fixtures are not synthesis; and review rejects synthesis sites
outside `synth.rs`. The original `scripts/ci/check-synth-boundary.py` and the CI `synth-boundary`
job were deleted by user decision D on 2026-11-24, and no automatic scanning gate is maintained:
heuristic token scanning cannot cover arbitrary legal use / alias / re-export forms (even after
successive patches, it remained bypassable through `pub type` re-exports), and its maintenance cost
exceeds its benefit.

D1/D2: `SynthEnv` and `BindingLocation` are both new internal types introduced in this batch, not
reused existing types. The former centralizes the production-side environment operands, named items,
and slot derivation into one explicit parameter; the latter promotes the original
`SymbolEntry.local_idx` into a discriminable Local/Upvalue location. The alternatives—continuing to
pass three shared fields, which cannot prevent nested overwrites and erroneous residue, or passing
only `Vec<Operand>`, which cannot express the correspondence between the anonymous prefix and named
captures—were rejected for correctness reasons. The new concepts are not synonymous with existing
AST or consumption-ledger concepts, and they require neither import aliases nor conversion tables.

### Disposition of Span-Keyed Problems (the Parts SSA Cannot Resolve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism one: consumption-coverage assertion (mandatory and lowest-cost; superseded on 2026-10-13
by a stronger form).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). The
original addition proposed in this document was to implement it as a **runtime count** rather than
only a test. The implemented form is stronger—a **consumption ledger (`ConsumptionLedger<K>`) + a
single settlement point (E-level `W1090`)**—with no new diagnostic code and without relying on an
internal ir_gen counter to validate itself:

```rust
// Around ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each :1942 hit, self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works:** a `drop()` instruction for a non-`ref` local is observable at runtime through
destructor side effects, but **“the value that should have been dropped was not dropped” may not
crash immediately under YaoXiang’s current value semantics and Arc/Rc model**—which is precisely why
the failure was silent. Counting turns “silent” into “detectable.”

**Mechanism two: change span keys to `DefId` or explicit plan IDs (**landed on 2026-10-13**: RFC-039
D20 / WBS 4.5 implemented this mechanism as a `PlanId(u32)` structural-identity key + mandatory
consumption settlement; see [02](02-stage-contract.md) §2 revision note for the implemented form).**
The fundamental fix is **not to use Span as a key**. `Span` is a source location and may drift
because of macro expansion, `include`, and multi-file merging. `FunctionIR` already has a
`def: Option<DefId>` field (`ir.rs:677`), demonstrating that DefId is available at the IR layer.
**Design judgment:** introduce `PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, assigned
by ownership / overload resolution at production time and matched by `PlanId` during IR consumption.
**This belongs to the obligation ledger (`Obligations`) in 02 and should be implemented together
with [02](02-stage-contract.md), not in this document.**

**Mechanism three: the definition-order dependency in `method_def_ordinals` (an independent small
item, **landed on 2026-11-25**).** The field was originally a `HashMap<String, usize>`, with the key
being the module-unqualified `"{type_name}.{method_name}"`, and assigned `#N` mangled names
according to definition traversal order—if typecheck registration order differed from lowering
definition order, methods with the same name could resolve to different targets. **Implemented
form:** typecheck produces `method_def_ir_names: HashMap<PlanId, String>` (a bare name or `#N`
mangled name) **once**, at the same point where it registers overload candidates, and passes it to
IR generation through `TypeCheckResult`. The key is the **method declaration statement’s `PlanId`**
(an AST structural identity, the same model as D20), because `DefId` is not allocated until
`assign_defs` at the end of lowering and therefore does not exist on the typecheck side. Lowering
reads the name solely by declaration identity, and **a miss fails loudly with E3005** (it does not
fall back to the bare name, because that would disguise “the definition name and call name came from
different sources” as a runnable program). The mutable counter and its write sites have been
removed, and definition names no longer have a second derivation entry point.

### Tracking the Three Hard-Coded Discarded Fields

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hard-coded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                                               |
| ------------------ | -------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count becomes zero after a `.42` round trip                                                                               |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **The `.42` artifact loses the entire exception table**                                                                                   |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **The `.42` artifact loses global-variable information** (note that `global_names` at `2347` **is** populated, creating an inconsistency) |

**Design judgment (revised on 2026-10-05, decision D52): all three are brought into this document’s
scope as P7 batch e (7e), rather than being split into separate issues.** The original judgment—and
the reasons it was overturned—are recorded as follows:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields) and are
   unrelated to SSA.~~ This is true, but does not constitute grounds for exclusion—P7 already
   modifies `bytecode.rs`, and fixing all three requires only one `VERSION` bump (4→5). Splitting
   them into two version bumps would be more expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → a version bump → expanding scope from
   “IR shape” to “artifact format.”~~ The versioning mechanism already exists (`MAGIC` + `VERSION`
   header, rejected by the reader when mismatched), and `.42` is a build artifact; cross-version
   compatibility is not a goal. What expanded was the workload, not the risk.
3. ~~`2315` / `2341` are actually low-risk, and running `.42` directly is a secondary path.~~
   **Overturned:** losing the exception table causes throw/try to behave incorrectly when `.42` is
   run directly—throw/try is core language semantics; a secondary path does not mean errors may be
   silently introduced. This refactoring’s principle is **not to leave core functionality as an
   unimplemented legacy** (`Vec::new() // Not implemented yet` is exactly such a legacy).

**Retained simultaneously:** the three items must still be registered in the `Obligations` ledger as
“produced but not serialized” until 7e is complete, so that they do not become a fourth category of
“something nobody knows about.” `2312` is to land with the version bump (D17), and the allocated but
unused opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Knock-On Effects of IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                                                     |
| ------------------------------------------------------------------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add `ValueId(u32)` type                                                   | File header (`ValueId` definition)            | New (**landed in 7b.1a**)                                                                                  |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**landed in 7b.1a**)                                                       |
| Add `Phi` variant to `Instruction`                                        | End of the `Instruction` enum (`Phi` variant) | **Variant +1** (**landed in 7b.1a**; the enum now has 75 variants)                                         |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | Field +1 (**landed in 7b.1b**)                                                                             |
| Add `ValueInfo` struct                                                    | `ValueInfo`                                   | New (**landed in 7b.1b**)                                                                                  |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                    | Add `values()` / `values_mut()` accessors (**landed in 7b.1b**, with the same shape as existing accessors) |

The `pub use ...ast::Type` at the file header is **not changed by this document**—it belongs to
[03](03-type-unification.md). However, `ValueInfo::ty` must use the **single converged type**
from 03.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                        | Location                              | Nature                                                                                                           |
| ----------------------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or use the “expand to Move at runtime” strategy | `127-554` (currently **66** variants) | Variant +1 or +0 (see below)                                                                                     |
| Add a `Phi` arm to `opcode()`                                                 | `558`                                 | Mandatory (`BytecodeInstr` has an exhaustive match)                                                              |
| Add a `Phi` arm to `size()`                                                   | `649`                                 | Mandatory                                                                                                        |
| Add a `Phi` arm to the `From<BytecodeFile>` decoding match                    | `943-2350`                            | Mandatory                                                                                                        |
| Correct `upvalue_count: 0`                                                    | `2312`                                | Fixed incidentally by this document (in-memory format; on-disk persistence requires a `VERSION` bump, see below) |

**The bytecode shape of `Phi` requires a design decision.** There are two options (**design
judgment: A is recommended**, for the reasons under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded by `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  selecting the value according to the predecessor block index. **No new opcode is added**, and the
  `.42` format does not change. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode + predecessor-index encoding, and have the interpreter maintain register
  versions when jumping. **This requires changing the `executor/` state model + the `.42` format +
  the version number.** The only benefit is smaller output.

The decisive reason for choosing A is that changing the `.42` format would trigger version-number
issues and a full compatibility discussion, while **A’s size cost is irrelevant in
interpreted-execution scenarios** (the executor runs Move instructions, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions distributed over `101-1558`)

| Change                                                     | Location                                                                                                                         | Nature                                                                          |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                               | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add an arm                                                                      |
| `Instruction::Phi` translation arm                         | Inside the dispatch match, before `Free` (533)                                                                                   | New branch that expands into a Move sequence (option A)                         |
| Keep the existing 9 NOP degradations in the same positions | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9:** `655-657` covers 2 variants, and `658-660` covers 4 variants |

**SSA-ification does not reduce the number of NOPs.** These 9 degradations reflect expressiveness
gaps where “the IR has it, but bytecode does not,” and SSA-ification does not touch them.

#### `src/backends/interpreter/executor/`

Under option A, `executor/` requires **no changes**—`Phi` disappears during the `translator.rs`
phase, before the interpreter can see it. **This is the second decisive reason for choosing A.**

`BytecodeInstr::Switch` has an active implementation at `executor/ops/control.rs:90`, but **no**
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs` (measured: all three names have zero
hits in `ir.rs`)—**this is a reverse gap**: bytecode contains an instruction that the IR layer
cannot produce. It does not affect this document, but it is recorded here because it shows that “IR
→ bytecode” is not surjective.

The dispatch match at `executor/debug.rs:194` likewise requires no changes (under option A there is
no new opcode).

### Compiler Change Checklist

**File by file and function by function.** All line numbers are the measured starting points for the
changes:

| No. | File                                                                      | Location                                                                           | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                                                     | `Operand` enum definition                                                          | Add/remove `Operand` variants (**landed in 7b.1a**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 2   | `src/middle/ir/ir.rs`                                                     | End of the `Instruction` enum (`Phi` variant)                                      | Add `Instruction::Phi` (**landed in 7b.1a**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 3   | `src/middle/ir/ir.rs`                                                     | `FunctionBody::Code`                                                               | Add `values` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 4   | `src/middle/ir/ir.rs`                                                     | `ValueInfo`                                                                        | Add `ValueInfo` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 5   | `src/middle/ir/ir.rs`                                                     | `values()` / `values_mut()` accessors                                              | Add accessors (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 6   | `src/middle/ir/ir.rs`                                                     | `pub use ast::Type`                                                                | **Do not change `pub use ast::Type`** (scope of [03](03-type-unification.md); recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 7   | `src/middle/core/ir_gen.rs`                                               | `884-891` `next_temp_reg`                                                          | Return `Operand::Value`, increasing monotonically without rewinding (**landed in 7b.2**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; add an independent `next_value` counter starting at 0 per function body; the three statement-level reclamation sites only rewind `next_temp` and do not touch it)                                                                                                                                                                                                   |
| 8   | `src/middle/core/ir_gen.rs`                                               | `1954-1959`                                                                        | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 9   | `src/middle/core/ir_gen.rs`                                               | `3632-3638`                                                                        | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 10  | `src/middle/core/ir_gen.rs`                                               | `4737-4742`                                                                        | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 11  | `src/middle/core/ir_gen.rs`                                               | `1745-1759` / `1819-1822`                                                          | Replace with an RAII guard (`generate_method_ir`, entry `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 12  | `src/middle/core/ir_gen.rs`                                               | `1858-1866` / `2026-2029`                                                          | Replace with an RAII guard (`generate_function_ir`, entry `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 13  | `src/middle/core/ir_gen.rs`                                               | `2116` / `2180`                                                                    | Replace with an RAII guard (`generate_curry_innermost_func`, entry `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 14  | `src/middle/core/ir_gen.rs`                                               | `2224-2229` / `2284-2288`                                                          | Replace with an RAII guard (`generate_curry_function_ir`, entry `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 15  | `src/middle/core/ir_gen.rs`                                               | `2768-2777` / `2820-2851`                                                          | Replace with an RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 16  | `src/middle/core/ir_gen.rs`                                               | `4686-4695` / `4767-4774`                                                          | Replace with an RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 17  | `src/middle/core/ir_gen.rs`                                               | `1987-2000`                                                                        | Extract the E3014 check as `check_register_budget()`, shared after the conventions at `1798`/`2001` are unified                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 18  | `src/middle/core/ir_gen.rs`                                               | `1798`                                                                             | Change `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`, eliminating the inconsistent convention                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 19  | `src/middle/core/ir_gen.rs`                                               | `248-249`                                                                          | Delete the comment contract (now enforced by the type system), replacing it with a reference to `verify`                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 20  | `src/middle/lower/loops.rs`                                               | `generate_spawn_for_ir`                                                            | Change to call `synth::synth_closure` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 21  | `src/middle/lower/expr/spawn.rs`                                          | `generate_spawn_expr_ir`                                                           | Change to call `synth::synth_closure`; delete writes/clears of shared pending fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 22  | `src/middle/lower/expr/closure.rs`                                        | `generate_lambda_expr_ir`                                                          | Read `SynthEnv` from the parameters instead of `mem::take`-ing shared fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 23  | `src/middle/lower/{mod,scope}.rs`                                         | Symbol table and `NestedBodyGuard`                                                 | Delete the `closure_captures` field; use `BindingLocation::Upvalue` lexical binding and depth-based restoration (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                            |
| 24  | `src/middle/lower/expr/{access,closure}.rs` + `query.rs`                  | Variable and namespace queries                                                     | Replace `closure_captures.contains_key` with a lexical-upvalue lookup; materialize captures uniformly through `capture_operand` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                            |
| 25  | `src/middle/core/ir_gen.rs` (audit-time path)                             | `341` + `1942`                                                                     | ~~Add a span-keyed consumption counter~~ → covered by 4.5’s `PlanId` consumption ledger (read and record in the same operation, settlement through W1090); 7c.2 verifies zero triggers, with no code changes                                                                                                                                                                                                                                                                                                                                    |
| 26  | `src/middle/lower/{mod,stmt}.rs`, semantic side `sema/{checker,types}.rs` | Definition name for `generate_method_ir` + authoritative `TypeCheckResult` mapping | Make `method_def_ordinals` read-only (**landed in 7c.3**: typecheck produces the mangled name according to declaration `PlanId`, and lowering only reads it)                                                                                                                                                                                                                                                                                                                                                                                    |
| 27  | `src/middle/core/ir_gen.rs`                                               | `7269-8014`                                                                        | **Do this last**: split into a top-level `CallArgs` + 6 `emit_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 28  | `src/middle/core/ir_gen.rs`                                               | `7838-7846`                                                                        | Change the “fill in 0” fallback to return a diagnostic                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 29  | `src/middle/lower/synth.rs`                                               | **New file**                                                                       | Synthesized-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 30  | `src/middle/ir/verify.rs`                                                 | **New file** (first layer of [07](07-equivalence-oracle.md))                       | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 31  | `src/middle/passes/codegen/translator.rs`                                 | Dispatch match                                                                     | Translate `Instruction::Phi` (**landed in 7b.3**: option A expands it at **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit a group of Moves per predecessor, existing `JMP`s skip between groups, and incoming-edge jumps are redirected to the group start using `(predecessor, convergence block’s first instruction)`; the `Phi` arm of `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return position cannot hold multiple Moves distributed across predecessors, so reaching it fails loudly) |
| 32  | `src/middle/passes/codegen/operand.rs`                                    | `36-45` / `72-77`                                                                  | Add the `Operand::Value` resolution arm; delete the `Temp` arm (`39-45`) (**landed in 7b.2**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); deletion of the `Temp` arm landed first in 7b.1a)                                                                                                                                                                                                                                              |
| 33  | `src/middle/core/bytecode.rs`                                             | `127-554`                                                                          | **Unchanged under option A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 34  | `src/middle/core/bytecode.rs`                                             | `558` / `649` / `943-2350`                                                         | **Unchanged under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 35  | `src/middle/core/bytecode.rs`                                             | `2312`                                                                             | Correct `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 36  | `src/backends/interpreter/executor/**`                                    | —                                                                                  | **Zero changes under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 37  | `src/middle/lower/regalloc.rs`                                            | **New file**                                                                       | Linear-scan allocator (**landed in 7b.4**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)` returns `value_slots` and `frame_slots = named_base + max_live`; overflow returns `Budget`, which `finalize_code_body` reports as E3014)                                                                                                                                                                                                                                                                                |
| 38  | `scripts/ci/check-synth-boundary.py`                                      | **Canceled**                                                                       | The boundary sentinel script was canceled by user decision D on 2026-11-24; do not create a new one                                                                                                                                                                                                                                                                                                                                                                                                                                             |

### Backward Compatibility: Versioning for the `.42` Format

**Verified fact:** the `.42` on-disk format **already has a version-number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (“YXBC”) and
`VERSION: u32 = 4`, writes them to the header (`:342-343`), and validates them when reading
(`:454-467`, reporting “unsupported bytecode version” on mismatch).

**Design judgment (a direct consequence of option A)**:

- `Phi` is expanded into a `Move` sequence during the `translator.rs` phase and **does not produce a
  new opcode**.
- The 83 constants in `opcode.rs` have **zero changes**, `opcode_name()` (`120`) is unchanged, and
  the decoding match (`bytecode.rs:943-2350`) is unchanged.
- The `.42` format’s field layout has **zero changes**, and `VERSION` remains 4. Old `.42` files can
  still be read by the new binary, and new `.42` files can still be read by the old binary.
  **SSA-ification itself does not require a version bump.**

**The only scenario requiring a version bump is correction of `2312 upvalue_count`** (decision D17:
fix it and handle it with the version bump): adding an `upvalue_count` field to `BytecodeFunction`
and having the encoder write it is a **new format field**, raising `VERSION` from 4 to 5. The
existing reader-side version check rejects old files according to existing behavior—`.42` is a build
artifact (`main.rs:671`), cross-version read compatibility is not a goal, and the version bump
requires no additional migration mechanism. The allocated but unused opcode values (D32/D34’s
`Switch` / `TailCall`) are reclaimed in the same version bump.

## Implementation Notes

### Batch Strategy

Divide the work into four batches. **Every item within a batch gets an independent commit,
independent criteria, and an independent revert.** “Change items” in the table refer to the numbers
in the “Compiler Change Checklist” above.

| Batch       | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Change items                 | Criteria                                                                                                                                                                 | Rollback point                                                                                                                                                       |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Eliminate multiple definitions: delete the 3 statement-level reclamation sites → replace 6 save/restore sites with an RAII guard → unify conventions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 8-10 (first) → 11-16 → 17-18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariants). RAII and convention unification are pure movement: **C1**, zero-diff snapshots | One commit per item; `git revert` rolls it back                                                                                                                      |
| **Batch b** | Switch to the SSA shape: structural changes to `ir.rs` + mechanical replacement of temporary values with `Operand::Value` → `Phi` arm in `translator.rs` (option A) → linear-scan allocator. Within the batch, proceed serially through the three WBS levels: 7b.1a (`Operand` variant convergence + introduction of `Instruction::Phi`, **landed**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **landed**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **landed**) → 7b.3 (`Phi` arm in `translator.rs` + population of `BasicBlock.successors`, **landed**) → 7b.4 (linear-scan allocator + deletion of the three statement-level reclamation sites, in the same batch as 7a.1, **landed**) | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` fully green + `.42` round-trip tests                                                                                       | The `ir.rs` changes **must occupy one commit by themselves** (the type changes affect compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary → verify the existing consumption ledger (no code changes) → make `method_def_ordinals` read-only (the sentinel script was canceled by user decision D on 2026-11-24)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 20-24, 29 → 25 → 26          | **C4**: behavioral equivalence; the new diagnostic reports zero consumption-count triggers (otherwise, a real mismatch exists)                                           | Independent commit                                                                                                                                                   |
| **Batch d** | Split `generate_call_expr_ir` + turn the “fill in 0” fallback into an assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 27-28                        | Pure-movement portion: **C1**, zero-diff snapshots. Assertion portion: **C4**, corpus differential + first-layer verifier                                                | Independent commit                                                                                                                                                   |

**There are three ordering dependencies that cannot be exchanged within the batches**:

1. Batch a’s “delete reclamation” follows the `ir.rs` changes in batch b (deletion of the three
   statement-level reclamation sites landed with 7b.4 and the linear-scan allocator; 7a.1 executed
   together)—first make `next_temp` increase monotonically, so that `ValueId` has the semantic basis
   for “unique definition” (this semantics becomes effective after 7b.2 changes `next_temp_reg` to
   produce `Value` and 7b.3 lands the `Phi` arm).
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise, decomposition would carry the
   old `arg_regs` ordering assumptions into the new files unchanged.
3. Batch d must come last—it depends on the preceding two batches having converged `arg_regs`
   semantics to “one independent `ValueId` per argument.”

**Position relative to RFC-039:** all four batches fall within **P7** of RFC-039, and the criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial** with respect
to one another, but items 8-10 / 11-16 / 17-18 within batch a have ordering constraints but no
coupling.

### Hard Prerequisites

**Prerequisite one: criterion baseline.** The `verify_loose` mode of the first-layer verifier in
[07](07-equivalence-oracle.md) **must first pass on the existing (non-SSA) IR** before batch a may
begin.

The reason is that the SSA-ification criterion is C4 (behavioral equivalence + invariants). If the
invariant verifier itself cannot pass on the existing IR before refactoring, then every acceptance
step in batches a–d lacks an executable criterion and can rely only on corpus differentials—yet
corpus differentials are precisely **insensitive** to Defect Classes 1/2/3 (as demonstrated by
“alternative A” in 07). **This is a real ordering constraint.**

**Prerequisite two: convergence of type representations.** [03](03-type-unification.md) must
complete the convergence of `pub use ast::Type` at `ir.rs:3` and remove the
`From<MonoType> for IrType` bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` would
introduce a third set of type-compatibility checks.**

### Rollback Strategy

- Every change gets an independent commit, with `git revert` granularity equal to change-item
  granularity.
- The `ir.rs` structural changes in batch b are **the only part that cannot be rolled back
  incrementally** (type changes affect compilation throughout the repository). They must therefore
  **occupy one commit by themselves**, and batch a must already have passed and established a
  revertible baseline before them.
- **Do not use a feature flag.** The reason is that adding/removing `Operand` variants cannot be
  isolated by a runtime switch; a feature flag would leave CI permanently untested on the other
  path—a reenactment of the “test wiring depends on people remembering, so corruption happens
  silently” issue recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front; otherwise, “the line count did not decrease” will become an issue
midway through implementation.**

| Item                                       | Line-count change                                                                                                                                                                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                             | `ir_gen.rs` **8,448**                                                                                                                                                                             |
| Delete 3 statement-level reclamation sites | **−15**                                                                                                                                                                                           |
| Replace 6 save/restore sites with RAII     | Net ±0 (the RAII guard definition is about +20 lines, while usage sites fall from 22 lines to about 12)                                                                                           |
| Refactor `next_temp_reg`                   | +5                                                                                                                                                                                                |
| Split `generate_call_expr_ir`              | **+40–80** (the signatures and parameter-passing overhead of the 6 `emit_*` functions; this is the typical cost of a pure-movement decomposition, and **decomposition increases the line count**) |
| Span-keyed consumption counter             | +15                                                                                                                                                                                               |
| Make `method_def_ordinals` read-only       | +20                                                                                                                                                                                               |
| New `synth.rs` file                        | +80–120                                                                                                                                                                                           |
| New `verify.rs` file                       | +400–600 (the first layer of 07, although within this document’s scope, it is counted independently)                                                                                              |
| New `regalloc.rs` file                     | **+500–1,000** (measured at 271 lines in 7b.4)                                                                                                                                                    |
| **Total**                                  | **`ir_gen.rs` grows from approximately 8,448 to 8,600–8,800; including the new files, the net increase is 900–1,600 lines**                                                                       |

**The early expectation of “8,448 → 5,000–6,000” was wrong**, and measured analysis produced the
opposite result. The reason is:

1. **The core benefit of this document is not fewer lines, but eliminating an entire class of
   defects that fail without reporting errors.** The 6 manual save/restore sites (22 lines) become
   about 12 lines plus a guard type under SSA + RAII—**the line count barely changes, but “forgot to
   restore” changes from a possibility into a compile error**.
2. **Pure-movement decomposition necessarily increases the line count.** Splitting the 746-line
   `generate_call_expr_ir` into 7 functions repeats the signature (the original function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` binding, and error propagation in
   each one. **“Split the file to reduce line count” is a misconception**—it reduces cognitive load,
   not physical lines.
3. **The new verifier and allocator are net line increases.** These two components (approximately
   900–1,600 lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of line count, this document will most likely produce a **net increase**. The benefit
> lies in **verifiability**—turning defects that “fail without reporting errors” into invariants
> that “fail the run if violated.” If line count were used as the acceptance criterion, this
> document would fail; the correct acceptance criteria are those in the “Batch Strategy” section.

**This stage intentionally increases the amount of code in `ir_gen.rs`.** The 2026-10-03 decision
canceled all line-count / size gates (see prohibition two in [08](08-maintenance-mechanism.md)), so
**no line-count exemption list is necessary**—the legitimacy of the increase follows from the
separation-of-responsibilities argument: SSA-ification extracts “register allocation + definition
discipline” from lowering into independent modules, and the additional code buys the elimination of
the two manual-discipline classes in which “misalignment does not report an error and simply
produces an incorrect value”: 6 manual save/restore sites and `arg_regs` semantic reordering.

## Key Decisions and Rationale

### Three Core Decisions

**Decision one: temporary-value strategy—delete the three statement-level rewinds so that unique
definition becomes a mathematical consequence of construction.** The current rewinds (`1954-1959` /
`3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp` increase
monotonically, so every temporary slot naturally has one definition point. **The criterion is the
same, but the guards differ** (`3632` has a `result_reg.is_none()` guard and the other two do not),
and these hand-copied differences disappear with it. The cost is increased slot occupancy—the
existing 255 upper-limit check for `temp_high_water` (`1992-2000`) provides a backstop.

**Decision two: do not perform mem2reg; perform explicit renaming only.** The reasons are given
under “Value Identification Method”: YaoXiang’s `&` / reference / ownership semantics make “which
slots can be promoted” a type-system problem requiring the converged type representation from
[03](03-type-unification.md). **Performing mem2reg before 03 is guessing.** Explicit renaming
requires changing the return-value semantics of one `next_temp_reg` call site and deleting the three
rewinds, without touching the control flow of any lowering algorithm.

**Decision three: use option A for `Phi` (an IR-only pseudo-instruction expanded into a Move
sequence).** `executor/` and the `.42` format have **zero changes**, confining the regression
surface to L3. The decisive reason is that changing the `.42` format would trigger version-number
issues and a full compatibility discussion, while A’s size cost is irrelevant under interpreted
execution (the executor runs Move instructions, not Phi). The cost is that `Phi` is invisible in
`dump_bytecode` (when entering the C1/C2 stage, note that the third layer in 07 compares it).

### Rejected Alternatives

**A. Decompose everything at once (SSA + decomposition + `synth` boundary + allocator
simultaneously). Rejected.** This violates 07’s core principle: criteria must be graded by category.
Mixing four batches into one PR mixes pure-movement changes with C4 changes, so snapshot drift
cannot be attributed—if a snapshot changes, there is no way to know whether the decomposition moved
something incorrectly or the SSA change did. **The rollback granularity degrades from “change item”
to “everything.”**

**B. Split files first, then optimize ordering. Rejected.** Splitting eliminates no root cause: the
6 manual save/restore sites remain scattered across 10 files, and `arg_regs` semantic reordering
still requires manual reasoning. Worse, **splitting first loses the opportunity to “build the
criterion before changing the code”**—decomposition creates a large line-level diff that mixes into
the subsequent SSA diff and invalidates review. This has the same origin as the 8 orphan test trees
recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests that never ran): **with the wrong
ordering, every subsequent step accelerates along an incorrect foundation.**

**C. Introduce an intermediate SSA layer without changing the existing lowering. Rejected, but
recorded as “a third path once considered.”** The shape is to leave `ir_gen.rs` unchanged and add a
pass that converts its non-SSA IR into SSA IR. **The advantage** is that lowering is untouched and
rollback is easy. **The reasons for rejection are:** (1) it does not eliminate the root cause of
Defect Class 1—the three statement-level reclamation sites remain in `ir_gen.rs`, and the 6
save/restore sites remain separated by 59–161 lines. A conversion pass can merge multiple
definitions into `Phi`, but **it cannot repair “an inner function’s name leaks into the outer
function”**—that is a `cur_locals` save/restore error occurring before the IR shape, and a
conversion pass cannot see it. (2) It makes SSA a second representation—the repository already has
the precedent of 3 parallel type representations, and the project’s core lesson is that
**“boundaries exist only in people’s awareness.”** (3) `verify_loose` becomes a permanent burden.

**D. Keep the current implementation and only add a verifier. Rejected as the sole solution, but it
is a component of this document.** RFC-039 alternative B reaches the same conclusion: a gate can
prevent regressions but cannot repair the current state. **However, D’s capabilities must be stated
clearly**: D can **catch instances** of Defect Classes 1/2/3 (provided the verifier covers
dominance, type consistency, and inner isolation), but it cannot catch Defect Class 4
(`closure_captures` leakage), which requires a cross-function liveness assertion beyond the boundary
of `verify(&ModuleIR)`. **Therefore, D is not an alternative to this document; it is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not the risk
  point**—the 2026-10-03 decision canceled line-count / size gates, and the scale issue is addressed
  through separation of responsibilities (see prohibition two in [08](08-maintenance-mechanism.md)).
  The increase is justified because it replaces the two manual-discipline classes in which
  “misalignment does not report an error and simply produces an incorrect value”: 6 manual
  save/restore sites and `arg_regs` semantic reordering.
- **The “unique definition” check in `verify.rs` is meaningless in the non-SSA shape**
  (`Operand::Local` permits multiple definitions)—07 already records this limitation, and
  `verify_loose` is its degraded mode before this batch.
- **Deleting statement-level reclamation changes the temporary-slot allocation pattern** (executed
  in 7b.4, together with 7a.1): after reclamation is deleted, slots are reused by the linear-scan
  allocator according to live intervals, so **the allocation result changes**—this is exactly why C4
  must genuinely run against the full corpus; the measured 403/403 differential is empty.
- **`u8` / 255 slot limit:** the two corpus cases that exceeded the limit after reclamation was
  deleted (`string_ops.yx` with 354 monotonic slots and `list_comp.yx` with 323 monotonic slots) are
  compressed by the allocator to **15 / 24 frame slots**; the budget gate now reports E3014 only
  when the function body truly does not fit after allocation (with the function-body declaration
  span), rather than incorrectly rejecting based on the cumulative number of temporaries.
- **The benefit must also be recorded:** it turns “errors that cannot be tested” into “invariants
  that can be tested”; it does not modify the backend (`executor/` and `.42` have zero changes); it
  does not change the semantics of `Instruction` (all 76 → 77 variants are retained, and all 50
  `translate_*` functions are retained); and it is incremental (every item can be reverted
  independently).

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> decisions (D1–D50). **This document retains no pending items.**

## See Also

### Documentation

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching architecture; four-layer model, criterion tiers, and P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definitions for
  criterion category C4, `verify_loose` / `verify_ssa`, the three criterion layers, and the
  limitation that snapshots are immune to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the `PlanId`
  change for `ReleasePlan`
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; a cautionary example of
  “building the directory first and then failing to wire it up”

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
  `PlanId` scheme)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect Class 1 (multiple definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for 7 nested-function-body entry
  points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — the sole slot-table extraction + E3014
  budget point (`MAX_REGISTERS = 255`), combining block splitting / slot allocation / table
  extraction
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — the linear-scan allocator (the sole production point for
  `value_slots` and `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamation
  removed in 7b.4; the allocator now reuses slots
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant
  referenced in 07

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  a clone
- `src/middle/core/ir_gen.rs:7838-7846` — the silent “fill in 0” fallback

**Defect Class 3 (span keying)** — the following are **audit-time evidence pointers** (after the 4.5
key migration: the `release_plan` / `overload_resolutions` keys have been converted to `PlanId`, and
the current pointers appear in the §2 revision note of [02](02-stage-contract.md);
`method_def_ordinals` was eliminated by 7c.3 through mechanism three)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of the three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write site of
  `method_def_ordinals` (key is the module-unqualified `"Type.method"`; **the field was deleted in
  7c.3**, and these are audit-time evidence)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the transfer structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer transfer
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (implicit order dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — the equivalent handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — two read sites of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — 3 other `ast::Type` constructions

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
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` limit of
  255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current test state**

- `src/middle/core/ir_gen.rs` — 8,448 lines, 0 `#[cfg(test)]` blocks, 0 `mod tests` modules
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no IR structural
  invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example of a per-opcode round-trip criterion; the last test in the file; the file contains 1,209
  lines / 23 `#[test]` functions)

**Engineering configuration**

- `Cargo.toml:37-39` — dev-dependencies include only `criterion` / `proptest`; `[dependencies]`
  contains no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans the `/rfc/draft`
  directory; this document is not under that directory and is referenced by RFC-039 and the index in
  this directory
