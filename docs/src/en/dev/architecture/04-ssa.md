# SSA-ifying the Intermediate Representation

> **Companion design document**. This article is a companion document to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main RFC-039 text for the four-layer model, acceptance-criterion tiers, and execution phase
> order; see the [index for this directory](index.md) for the positioning of each companion
> document.

## Positioning and Scope

This document addresses construction discipline for IR construction at the L3 layer. It proposes a
“one definition per value” construction method for `src/middle/core/ir_gen.rs` (8,448 lines). **The
goal is not to introduce a new instruction set, but to eliminate three classes of discipline defects
that fail without producing any errors**:

1. Six instances of manual register save/restore scattered dozens of lines apart;
2. Semantic reordering of `arg_regs` shared by six branches inside `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: changes to the IR form (`Operand` / `Instruction`),
temporary-value allocation discipline, making cross-layer contracts explicit, splitting
call-expression dispatch, and integrating the static IR validator.

This document is not responsible for: convergence of type representations
([03](03-type-unification.md)), stage contracts and the obligation ledger
([02](02-stage-contract.md)), definition and gating of equivalence criteria
([07](07-equivalence-oracle.md)), the frontend paradigm ([05](05-frontend-paradigm.md)), or cleanup
of dead code and speculative design ([06](06-cleanup-inventory.md)). This document repeatedly cites
conclusions from those documents and does not repeat their arguments.

**Criterion category** follows **C4 (IR Form Changes)** from
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, rather than byte-for-byte equality of IR snapshots. SSA-ification necessarily changes
the IR form; forcing snapshot equality would encourage the team to weaken the criteria, which is
exactly the trap recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, with acceptance requiring behavioral equivalence and a green `verify_ssa`). The
implementation batch numbers within this document (batches a–d) are distinct from the ordering
within P7: the former are this document’s batching strategy, while the latter is RFC-039’s global
phase structure.

## Feasibility Preconditions

**Verified facts.** The IR already possesses **all structural prerequisites** required for SSA
construction:

| SSA prerequisite                 | Current repository state                                               | Location                                                                 |
| -------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Explicit CFG                     | `BasicBlock { label, instructions, successors }`                       | `BasicBlock` in `src/middle/ir/ir.rs`                                    |
| Explicit entry block             | `FunctionBody::Code { blocks, entry, locals }`                         | `FunctionBody` in `src/middle/ir/ir.rs`                                  |
| Scope nesting information        | `LocalSlot::scope_depth` (`"0 = 函数参数层，供嵌套作用域重名消歧"`)    | `LocalSlot` in `src/middle/ir/ir.rs`                                     |
| Three-address form               | 76 variants: 54 with `dst: Operand` and 3 with `dst: Option<Operand>`  | `Instruction` in `src/middle/ir/ir.rs`                                   |
| Per-instruction span             | Every variant has a `span: Span` field                                 | Same as above                                                            |
| Value types can be annotated     | `LocalSlot::ty: MonoType`                                              | `LocalSlot` in `src/middle/ir/ir.rs`                                     |
| Upper-bound check for slot count | A single table-retrieval and E3014 budget gate (`MAX_REGISTERS = 255`) | `finalize_code_body` in `src/middle/lower/scope.rs` (starting with 7b.4) |

**If these prerequisites were absent, the cost estimate in this document would be entirely
different.**

**The two decisive points are as follows:**

**First, SSA-ification does not add a capability; it removes a dependency on one.** In the current
design, temporary slots are **deliberately designed to be defined multiple
times**—`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`, explicitly roll `next_temp` back in
three places. The first step of SSA-ification is to **delete those three rollbacks**, making
`next_temp` increase monotonically. Once they are deleted, **every temporary slot naturally has
exactly one definition point**—single definition changes from “a rule that must be checked” into “a
mathematical consequence of how values are constructed.” Batch a only converged the save/restore
surface (see “Defect Class 1”); the three rollbacks were removed together with batch b’s 7b.4 (the
allocator took over slot reuse) on 2026-11-22.

**Second, the upper bound on slot count is a hard check; deleting the rollbacks briefly caused slot
counts to become uncontrolled (resolved by 7b.4).** The budget gate in `src/middle/lower/scope.rs`
reports E3014 when the count exceeds 255. After deleting the three reclamation points, running the
full corpus produced two cases that exceeded the limit and triggered E3014:
`src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; peak slot counts in the
remaining corpus rose from 51 to 211. **After 7b.4 landed**, the linear-scan allocator reuses slots
according to live intervals; the two cases now have measured frame-slot counts of **15 / 24** (with
values of 344 / 304), and the remaining corpus is behaviorally equivalent in 403/403 cases. From
this point onward, E3014 means only “frame slots after allocation exceed 255.”

**Taken together, these two points mean:** lowering already maintains all the bookkeeping
information required for SSA; it merely **reclaims** that information. 7b.4 makes the allocator
responsible for slot reuse and removes the reclamation. The main SSA-ification workload lies in
splitting `generate_call_expr_ir` and making cross-layer contracts explicit, not in building new IR
expressive power.

### Prerequisite: Unification of Type Representations

**Verified facts.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The third representation is `ir::Type` on the serialization side,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR simultaneously contains two type representations, and one of them directly
`pub use`s the AST’s type.**

SSA-ification requires a new `Phi` variant, a type record for every SSA value (to support the
verifier’s “type consistency” invariant in 07), and a determination of whether the types of two
predecessors are compatible at a CFG convergence point. **If type representations have not yet
converged, SSA will require a separate type-compatibility implementation for every
representation**—and the third representation will grow accordingly.

This is not a question of which approach is “better to do first”; it is a question of **which
approach would produce three sets of type-compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first complete the convergence of
`pub use ast::Type` at `ir.rs:3` and remove the bridge at `bytecode.rs:2353`. Item 6 in the change
list explicitly records that `ir.rs:3` is **not changed by this document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not merely “an ugly file that happens to work.” It is a file that **places
correctness in the hands of line-by-line human review**:

- The file contains **0 `#[cfg(test)]` blocks and 0 `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares the `bytecode` / `def_assign` / `local_slots` modules,
  with 29 tests in total. Of these, `def_assign.rs` and `local_slots.rs` **positively** cover
  ir_gen’s DefId allocation and local-slot naming (at sentinel level), while all four references to
  ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **negative assertions**—“ir_gen does
  not produce,” “ir_gen does not construct,” and “the ir_gen frontend does not construct the
  corresponding IR.”

In other words, ir_gen has 29 sentinel-level tests pinning local behavior, but **no IR structural
invariant validation** (dominance, single definition, jump targets, type consistency)—none of the
three modules declared by `middle/core/tests/mod.rs` positively asserts the overall IR form.

This file also happens to contain several classes of defects that **compile successfully, pass all
checks, and let the program run normally, but produce incorrect values**. Their common property is:
**failure produces no error**.

This is where the highest risk lies. It is not that “the function is too long,” nor that “the naming
is confusing,” but that **there is no machine-executable enforcement mechanism for this file’s core
invariants**.

The contract “save state before generating a nested function body and restore it afterward, or the
inner function’s names will leak into the outer scope” is enforced by the type system:
`NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator,
and frame removal and restoration each have only one implementation (see “Defect Class 1”).

**Why a verifier alone is not enough**: a verifier can **catch** these defects, but it cannot catch
**the reasons they arise**. The reason is not “someone missed them while reviewing,” but rather:

> **The current IR form itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator deliberately depends on that fact.**

The concrete evidence is three statement-level register reclamations (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`). At the end of every statement, they **roll `next_temp` back**
to the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and its number of
writes, write order, and live ranges are all determined by a single manual decision: **when to roll
back**.

Under this form, “single definition” is not a rule that can be introduced incrementally; it
**directly conflicts with the register-allocation strategy**. SSA-ification is therefore not a
matter of adding a check to the existing code; it is a matter of **replacing the allocation
strategy’s dependency on multiple definitions**. The good news is that replacing this strategy is
inexpensive—see “Feasibility Preconditions” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (the Three Reclamation Points Were Removed in 7b.4)

**Verified facts.** There are seven entry points that require save/restore when entering a nested
function body. Batch a (7a.2) converged them into one RAII guard: `NestedBodyGuard<'a>` in
`src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, with unconditional restoration in
`Drop`, and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Restoration scope                                       |
| ------------------------------------ | ---------------------------------- | ----------------------------- | ------------------------------------------------------- |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (the three counters are not restored) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                           |
| Nested function-definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                      |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                           |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                           |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                           |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                           |

- Frame removal occurs only in the two constructors (`enter` / `enter_decl`), and restoration occurs
  only in `Drop`; the guard exclusively holds `&mut` access to the generator for its lifetime, so no
  second `&mut` is available in scope, and `?`/early returns always restore state through `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`; starting with 7b.4,
  `next_temp_reg` only maintains the named high-water mark (temporary-value slots belong to the
  linear-scan allocator; `temp_floor` was removed together with the three reclamation points).
- `enter_decl` does not restore the three counters: after a function body or method body returns to
  the module-initialization flow, subsequent definitions continue from the child body’s terminal
  value number. This is the constraint for zero drift in C1 snapshots and the meaning of “the
  declaration surface restores only the table.”

**Unified accounting and a single budget point (starting with 7b.4)**: `finalize_code_body` in
`src/middle/lower/scope.rs` is the sole table-retrieval and budget point. It retrieves the table
using `frame_slots = named_base + max_live`, reports E3014 with the function-body span when out of
range, and defines `MAX_REGISTERS = 255` in exactly one place in the repository. All six
function-body forms (function / method / lambda / anonymous binding / middle curry layer / innermost
curry layer) are finalized through it, so there is no second definition of “how many slots a
function body should have.”

**The three statement-level reclamations were removed (7a.1 landed with 7b.4 on 2026-11-22)**: the
reclamation had been hand-copied into `generate_function_ir` / `generate_block_ir` (with a
`result_reg.is_none()` guard) / `generate_lambda_body_ir`. After the allocator took over slot reuse,
the reclamation degenerated into an identity operation, so even the guard, the `temp_floor` /
`saved_temp_floor` bookkeeping, and the loop-entry/loop-exit bookkeeping in `loops.rs` were removed.
Module init / top-level statements still follow the old monotonic slot path with `value_mode=false`
and do not run the allocator; their slot numbers and table lengths shift with the number of named
slots in the function body (measured across three snapshots; behavioral equivalence is covered by
403/403 cases).

**Consequences (current form)**: the guard exclusively holds `&mut`, restoration has only one
implementation in `Drop`, and there is no writable location where a restore could be accidentally
omitted. The temporary-slot dependency on “one slot, multiple definitions” has disappeared together
with the reclamation points: value slots are reused by the linear-scan allocator according to live
intervals, giving the single-definition criterion in `verify_ssa` a construction-side foundation.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified facts.** `generate_call_expr_ir` is located at `ir_gen.rs:7269-8014`, spans **746
lines**, and is a single function. Its signature (`7269-7279`) accepts `func: &Expr` /
`args: &[Expr]` / `named_args: &[(String, Expr)]` / `span: &Span` (along with `_expr` / `result_reg`
/ `instructions` / `constants`). Its six internal branches share the same `Vec<Operand>` and perform
**semantic rewriting**:

| Line        | Semantics                                                                                                                                                     |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flattened by `slots` and then re-`collect`ed—named arguments are reordered                                                                    |
| `7724`      | Struct construction: each slot in `final_args` is `unwrap`ed and then re-`collect`ed—fields are reordered                                                     |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **fills absent slots with a new register whose value is 0** (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—cloned before instructions are emitted                                                                     |

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

The comment for `7835-7848` states: “Uncovered slots: remain absent so that the type-checking stage
reports E1010. For runtime safety, fill with 0 (the normal path never reaches this).”

**This design has two kinds of risk**:

1. **Misalignment does not produce an error; it produces the wrong value.** Each of the six branches
   independently determines the final order of `arg_regs`, and any incorrect ordering assumption
   does not cause a panic.
2. **Normalized snapshots are immune to this defect.** Document 07 already records this limitation:
   after the snapshot tool renames temporary values by “first-appearance order,” “the third argument
   should have used the fifth register” is identical in the snapshot. **This class of defect does
   not even have the snapshot layer as a criterion in phase C4.**

**Additional verification: the struct-construction path’s reordering has no duplicate-slot guard.**
`final_args` at `7724` is populated by “positional arguments enter their slots first, then named
arguments overwrite by field name.” **Before overwriting, there is no duplicate-slot check such as
`slots[idx].is_some()`** (unlike the namespace path, which has one at `7381`). When a named argument
silently replaces a positional argument, there is neither a panic nor a diagnostic; furthermore,
**argument evaluation order** (source order) diverges from the final `arg_regs` order
(field-declaration order). The assertion-based `fill_missing` in batch d must also cover the
“duplicate slot” check; otherwise the struct-construction path’s silent replacement will survive
unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified facts (at the time of the audit).** The repository contains **two** Span-keyed
cross-layer contracts:

| Contract               | Declaration                                                                                                                              | Production                                                                                                                   | Propagation                                                                                   | Consumption                                                                                                                |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                                | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatches `walk_stmt` by statement identity)          | `typecheck/types.rs:38` ← passed through by `checker.rs` ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, the sole entry point; settlement-complete marker `:1095`)                        |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (the table and consumption ledger are one object, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` only passes it through and merges) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held in an `ir_gen.rs` field            | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; settlement-complete marker `:1097`) |

The form at the time of the audit (`ir_gen.rs:1942`, **now absent**) was:

```rust
// NLL Release: insert Drop instructions at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some`—**if the lookup fails, it simply fails: there is no else branch, no assertion, and no
counter**. If the two sides calculate spans differently in any respect, **Drop instructions
disappear silently, with zero errors**. The Drop sequence of refinement types is a central carrier
of this project’s ownership semantics (`ownership.rs:26-32` describes itself as an “NLL precise
release plan”), and its lowering used to be silent.

The same pattern appears at `7420` / `7493`
(`if let Some(mangled) = self.overload_resolutions.get(span)`), causing overload resolution to fall
back to the default binding.

**Disposition (landed in 4.5)**: both keys were changed to the structural identity `PlanId`, and
consumption uses **record-on-hit** accounting (the read and accounting occur in the same call). A
non-empty difference is reported at the sole settlement point as an **E-level `W1090`**—“not found”
is no longer silent. See §2’s revision note (4.5) in [02](02-stage-contract.md) for the implemented
form and criteria. These line numbers (`1942` / `7420` / `7493`) are retained as evidence from the
time of the audit and are not pointers to the current implementation.

> **`method_def_ordinals` is not Span-keyed.** It is a `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with its sole write point at `:1692-1702` (assigning `#N` suffixes in
> definition order by `base_name`). Its risk is **different**: dependence on definition order. The
> `#N` suffix is determined by AST definition order, so if generation order differs from
> type-checking registration order, the same function name may resolve to a different target. This
> is not a Span mismatch, but it is also “wrong without an error.” Its disposition is described
> under mechanism three in “Additional Mechanisms for Span-Keyed Problems.”

### Defect Class 4: Implicit spawn/lambda Handshake (Eliminated in 7c.1)

**Eliminated.** spawn and spawn-for previously required callers to write `pending_env_vars` /
`pending_env_names` first, after which a synthesized Lambda would consume them; closure bodies
separately relied on `closure_captures` to determine whether a variable read an upvalue. Three sets
of fields placed correctness in the hands of call order and manual cleanup: nested synthesized
closures could mistake an outer local slot with the same name for a captured value, and early return
from closure-body generation could leak capture state into subsequent unrelated lowering.

7c.1 converged synthesized ASTs into `middle/lower/synth.rs`, replacing the three shared fields with
an explicit `SynthEnv`, and registering named captures as lexical upvalues. Nested frames are now
restored by symbol-table depth, so error paths no longer leave capture state behind. See the
“`synth.rs` Boundary” section for the implemented structure.

### What SSA Will Not Solve (Honestly Listed)

| Problem                                           | Location                                                                                                                                          | Why SSA does not solve it                                                                                                                                                                                                                                |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                      | `ir_gen.rs:5390-5687`, **298 lines**, recursively calls itself 4 times (`5473` / `5504` / `5619` / `5640`), with its external call site at `5270` | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **inherent to the algorithm**. SSA only turns the `JmpIfNot` instructions produced by guards into `Phi`; it does not reduce the number of guards |
| `eval_const_expr` complexity                      | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)                           | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                                                          |
| Span-keyed failure (**eliminated**)               | Audit-time evidence: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`); after 4.5, `PlanId` keys + mandatory settlement           | **SSA does not touch this layer at all** (the conclusion is unchanged). It has been independently resolved by the obligation-ledger mechanism in [02](02-stage-contract.md); see mechanism two under “Handling Span-Keyed Problems”                      |
| Number of `Instruction` variants                  | 75 (`Instruction` enum in `ir.rs`; includes `Phi` introduced by 7b.1a)                                                                            | SSA adds `Phi`, a **net change of +1** (already landed). Variants either correspond to bytecode opcodes or are lowered to NOP, so they are not reduced by SSA; under option A, `Phi` is an IR-only pseudo-instruction and adds no opcode                 |
| `method_def_ordinals` definition-order dependence | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                                             | Similar to but distinct from Span-keying; see mechanism three under “Additional Mechanisms for Span-Keyed Problems”                                                                                                                                      |

## Target Design

### SSA Form Definition

**Design judgment.** The target form is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has five variants. **Repository-wide measurements** (hit counts summed
by file from `git grep -c`, including tests under `src/`; construction-point counts refer to
value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                          | Disposition                                                                                       |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                                  | **Retained**, unchanged                                                                           |
| `Value(ValueId)`    | 7 files / 26 hits; introduced by 7b.1a with **0 production construction points**, and from 7b.2 onward `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`)              | **Retained** (introduced by 7b.1a)                                                                |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                                  | **Retained, but split semantically** (see below)                                                  |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                    | **Retained** as the implicit definition at function entry (parameters are definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                                   | **Retained**, unchanged                                                                           |
| `Temp(usize)`       | **0 construction points** (only handled in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, both removed with the variant)                                                       | **Removed** (dead variant, 7b.1a)                                                                 |
| `Label(usize)`      | **0 construction points**                                                                                                                                                                            | **Removed** (dead variant, 7b.1a)                                                                 |
| `Register(u8)`      | **0 construction points** (the comment at the original variant declaration said “Added for codegen,” but codegen uses `OperandResolver` in `codegen/operand.rs` and does not construct this variant) | **Removed** (dead variant, 7b.1a)                                                                 |

`Local(usize)` is split into two variants because it currently **serves two mutually exclusive
semantics** simultaneously—“the storage location of a named variable” (which may be written multiple
times) and “a compiler temporary register” (which should be written only once):

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
densely numbered within a function, **incrementing monotonically and never rolling back**. The split
and `ValueId` were introduced by 7b.1a (the `Operand` enum in `src/middle/ir/ir.rs`); the production
side (`next_temp_reg` producing `Operand::Value`) was introduced by 7b.2 (see “Production Form”
under “Value Identification Scheme”).

#### `Phi` as a New `Instruction` Variant

```rust
/// Convergence point: each predecessor basic block supplies one definition of this value
/// `incoming` is sorted by predecessor block label in ascending order (guaranteed during construction, for normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

There are three construction constraints (see the first layer in [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of the block`.
2. The label set of `incoming` == the label set of those blocks in `blocks` whose `successors`
   contain this block’s label.
3. All `incoming` values have the same `MonoType` (**dependent on the convergence of type
   representations in 03**).

The implementation status of `verify_ssa` (the first three constraints were completed by 7b.1b):
constraints 1 and 2 require the value table and block-level CFG. They were implemented in **7b.1b**
as separate criteria for “block-level predecessor count” and “predecessor label set”—the predecessor
set is inferred from `BasicBlock.successors` (`block_predecessors` in `src/middle/ir/verify.rs`). A
count mismatch reports only the count, while a set mismatch reports the set, so the same defect is
not reported twice. Constraint 3, per-input type consistency, landed at the same time. Its values
come from the value table at `values[v].ty` and **reuse the existing `types_compatible`** (no third
type-compatibility implementation was introduced; convergence of type representations in 03 remains
“Precondition (hard) · Precondition two”; even then, `ValueInfo::ty` will remain `MonoType`). The
violation category for all three criteria is `InvariantKind::PhiConsistency`.

**Three forms in which the type is unavailable (explicitly registered, not silently permitted)**: an
empty value table means that the basis for judgment does not exist (the constant form for module
init / top-level statements, via the `value_mode=false` path); a non-empty value table with an
out-of-range `ValueId` reports a violation; a non-`Value` operand (`Const` / `Local` / `Arg` /
`Global`) has no value-table type source and is not judged.

**Measured prerequisite (fulfilled by 7b.3)**: before 7b.3, every production construction of
`BasicBlock.successors` in the repository used `Vec::new()` (block-level predecessors were always
empty, so the predecessor-count and predecessor-set criteria were then covered only by synthetic
fixtures). **Starting with 7b.3, block-level `successors` are genuinely populated by
`partition_blocks` in `src/middle/ir/ir.rs` at every production construction site** (all six sites
in `lower` and the synthesized function-body site in codegen pass through this sole entry point;
lowering output includes loop / branch blocks with multiple blocks, multiple outgoing edges, and
back edges). The predecessor-count and predecessor-set criteria therefore have data from real
IRs—direct tests on true lowering output pin the registration-to-consumption loop. The number of
production construction sites for `Phi` is still 0 (the production side belongs to a later batch);
when it lands, it will be directly constrained by this criterion.

7b.1a only introduced the variant: there were 0 lowering production construction sites (and there
remain 0; production belongs to a later batch). The consumer side in `translator.rs` landed in
**7b.3**: under option A, `Phi` is expanded at block entry into a sequence of `Move` instructions
(the group is placed at the start of the convergence block, existing `JMP` instructions skip between
groups, and incoming-edge jumps are redirected to the group start according to the predecessor). The
`Phi` arm in `translate_instruction` is an unreachable E8001 defensive arm (the `executor` never
sees `Phi`).

**Disposition of the guard issue at `3632-3638`**: the reclamation in the current
`generate_block_ir` has an `if result_reg.is_none()` guard (the comment at `3626-3631` explains that
the block is the operand position of an expression and the outer scope may hold sibling argument
temporaries that live across blocks). After SSA-ification, this guard is **no longer
needed**—because temporary values are no longer rolled back, a sibling argument that lives across
blocks holds its own `ValueId` and cannot be affected by a rollback. **This is one area where
SSA-ification genuinely simplifies the code rather than merely renaming it.**

#### Value Identification Scheme

**Design judgment**: do not perform mem2reg, do not introduce virtual-register numbering, and
**perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots may be promoted to registers” a
  **type-system problem**, not a CFG problem. It has been verified that `LocalSlot::ty: MonoType` in
  `ir.rs` carries complete type information, but determining “whether this `MonoType` may be
  promoted to a register” requires the converged type representation from 03 and coordination with
  refinement-type rules. **Doing mem2reg before 03 is guessing.**
- Explicit renaming only needs to change the return-value semantics of one function
  (`next_temp_reg()` in `src/middle/lower/scope.rs`) and delete three rollbacks; it **does not touch
  the control flow of any lowering algorithm**.

Value-table form (**landed in 7b.1b**): `FunctionBody::Code` gains `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot), None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block containing the definition
}
```

The accessors have the same shape as the existing accessors (`impl FunctionIR` in
`src/middle/ir/ir.rs`): `values()` is read-only and returns an empty table for TypeDecl (mirroring
`locals()`); `values_mut()` is mutable and panics for TypeDecl (mirroring `blocks_mut()`).

**Production form (landed in 7b.2)**: `next_temp_reg()` returns `Operand::Value(ValueId)` and
registers one `ValueInfo` in the current function body’s value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). The implemented details are:

- **Counter isolation**: `ValueId` is issued by an independent counter, `next_value: u32`. Each
  function body starts at 0, increases monotonically, and never rolls back; `NestedBodyGuard` saves
  and restores it together with the value table (one set per function body). The three
  statement-level reclamations only roll back `next_temp` (slots) and do not touch `next_value`;
  each curry layer resets separately (one `FunctionIR` per layer). Module init / top-level
  statements have no value table—when `value_mode=false`, `next_temp_reg` falls back to the old
  `Local` slot allocation, leaving the init sequence unchanged.
- **Type source** (only existing lowering information; no new type decision procedure): literals use
  `ConstValue::mono_type()` (converged with the verifier’s constant-type mapping into a single
  source); variables/calls look up the type-checking result and the bound function’s return type at
  the `generate_expr_ir` entry point; comparison/logic/string families/forward propagation are
  filled in at function-body finalization according to instruction semantics. When unavailable, the
  type is **explicitly unknown `MonoType::TypeRef("?")`** (not silently replaced with `Int(64)`;
  `LocalSlot.ty` is never read). Across 232 snapshots, 9,425 values were measured: 7,080 had real
  types and 2,345 were explicitly unknown (24.9%), with unknown types concentrated in
  call/field/index/container-construction forms.
- **defining_block**: takes the current block label during generation. **Block splitting landed in
  7b.3** (a true lowering while body consists of multiple blocks), but the field’s value is still
  the generation-time `cur_block_label` (currently always 0), so it does not yet agree with the
  actual defining block. When 7b.4 regenerated 232 snapshots, this field was not changed (the
  `@block` values in the `values` section did not drift); correction must occur in the same batch as
  regeneration of the `values` section and is **explicitly handed off to the `Phi` production
  batch**.
- **debug_name**: always `None` (named variables remain `Local` and do not move into the value
  table).
- **Indexing**: `value_slots: Vec<usize>` (index = ValueId) has the same indexing as `values` and
  records the value → frame-slot mapping. Starting with 7b.4, it is recomputed by the linear-scan
  allocator in `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`); codegen’s `OperandResolver` only consumes it and does not
  allocate.

`values` and `locals` coexist: `locals` carries addressable named variables (which may be defined
multiple times), while `values` carries single-definition SSA values. **The existence of `locals`
does not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Division of responsibility with `LocalSlot::origin`**: the two are **different dimensions** and
must not be conflated. `LocalSlot::origin` (the **slot identity** field introduced by the
origin-side work in 2.1.4, recording the generating frame `FrameId` of the slot) answers “which
frame does this slot belong to,” while `ValueInfo.defining_block` answers “in which basic block is
this SSA value defined?” Value identity belongs to `ValueInfo`; slot identity belongs to
`LocalSlot::origin`. 7b.1b only adds the value table and **does not change `LocalSlot::origin`**
(verified as zero changes).

#### Change Estimate

| Item                                                        | Estimate                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ----------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Modify `next_temp_reg`                                      | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Delete three statement-level reclamations                   | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Converge six save/restore sites into RAII                   | Net ±0 lines                          | See “Per-Item Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `ir.rs` structural changes                                  | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **7b.1a landed the first two and 7b.1b the latter two** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Add a `Phi` arm to `translator.rs`                          | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `executor/`                                                 | **Zero changes**                      | Option A: `Phi` is an IR-only pseudo-instruction, expanded into a sequence of Move instructions by `translator.rs`, and does not enter bytecode (Decision three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Call sites that switch temporary values to `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (`git grep -c` summed by file, `src/` in full including tests): `Operand::Local(` occurs 415 times across 28 files, of which `src/middle/lower/` contains 332 occurrences across 14 files; the temporary-value production point is the single `next_temp_reg`. **7b.2 measurement (baseline revision remeasurement and landed net amount)**: 140 `next_temp_reg()` call sites across 14 files, a `result_reg: usize` parameter chain spanning 35 functions + 3 `Option<usize>` result slots, and 342 `Operand::Local(` occurrences within `src/middle/lower/`. The code-only diff for `src/` totals +23,319/−12,495, of which regenerating 232 IR snapshots accounts for +21,866/−11,950; source and tests net +1,453/−545 (mechanical replacements, normalize rendering, mono projections, six direct tests, plus five error-code artifacts committed alongside the change) |

### Per-Item Disposition Table

| Defect class                                                | Eliminated by SSA?         | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                     | Criterion                                                                                                          |
| ----------------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **1. Multiple definitions and manual save/restore**         | **Yes (resolved by 7b.4)** | The seven nested-function-body entry points are consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the counters), turning “forgot to restore” into a compile error; the three statement-level reclamations were removed with 7b.4 (the allocator reuses slots according to live intervals), and temporary values no longer occupy `next_temp` | First-layer “inner isolation” invariant in 07 + `verify_loose`/`verify_ssa`                                        |
| **1b. Inconsistent accounting**                             | **Yes (landed)**           | Unified in `finalize_code_body` in `src/middle/lower/scope.rs`: retrieve the table using `frame_slots`, with the E3014 budget check at the same point and with the function-body span; all six function-body forms are covered; `MAX_REGISTERS` is defined in exactly one place in the repository                                                                                                                               | Corpus differential + zero snapshot drift                                                                          |
| **1c. Criteria for the three statement-level reclamations** | **Yes (landed by 7b.4)**   | The three reclamations + the `result_reg.is_none()` guard in `generate_block_ir` + the `temp_floor` bookkeeping were removed; slot reuse is now handled by the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                                                  | C4: behavioral equivalence (403/403) + invariants                                                                  |
| **2. `arg_regs` semantic reordering**                       | **Partial**                | SSA turns “using the wrong register” into “using the wrong value ID,” which dominance checking can catch, but **the reordering logic itself remains**. Actual elimination depends on function decomposition: each of the six branches performs its own `push`, while ownership of `arg_regs` remains at the top of the function                                                                                                 | First-layer dominance invariant (snapshots are immune)                                                             |
| **3. Span-keyed failure**                                   | **No**                     | SSA does not touch it at all. See “Additional Mechanisms for Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                               | `test_release_plan_spans_consumed` in 07                                                                           |
| **4. Implicit order dependency**                            | **No**                     | See “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                                                       | Guaranteed by code structure and review; the original gate script was canceled by the user’s 2026-11-24 Decision D |
| `compile_pattern`, 298 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                                   | —                                                                                                                  |
| `eval_const_expr`, 191 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                                   | —                                                                                                                  |
| 75 `Instruction` variants                                   | No (+1, landed)            | `Phi` was introduced by 7b.1a; under option A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                                            | —                                                                                                                  |

### Register Allocator: Custom Implementation or Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **0 construction points throughout the repository**—the variant is no
   longer present in the `Operand` enum in `ir.rs` (removed by 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper bound of 255 (exceeding it
   produces E3014); starting with 7b.4, `Operand::Value` is resolved by the allocator’s
   `value_slots` output to a frame slot (`>255` reports E3014, while no mapping reports E3017 with
   the instruction span).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50
   `translate_*` functions, with function bodies distributed across `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`, where opcodes come from
   `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` contains the interpreter’s opcode dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Assessment of three options**:

| Option                                             | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both require rustc’s `Function` trait and MIR’s `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait set (CFG, liveness, spill slots, scratch space), meaning **500+ lines of adapter code for zero benefit**, because there is no real register file to color. rustc itself only performs linear scanning at this point (when `enable-llvm` is disabled)                                                                                             |
| `graph-alloc` family                               | **Not adopted.** The same reasoning applies, and the API is less stable                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| **Custom linear scan (simplified Chaitin-Briggs)** | **Adopted (landed in 7b.4).** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot according to its live interval (`OperandResolver` already has a `u8` contract and the E3014 limit). Implement `src/middle/lower/regalloc.rs`: block-level `successors` backward dataflow, closed intervals, Phi predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`, with overflow returning `Budget` for `finalize_code_body` to report E3014 (with the declaration span) |

**Measured implementation (7b.4)**: the two corpora that originally exceeded the limit (354 / 323
monotonic slots) now have **15 / 24** frame slots after allocation (344 / 304 values), and the
remaining corpus is behaviorally equivalent in 403/403 cases. From this point onward, E3014 means
only “frame slots after allocation exceed 255.” The fallback counter for back edges is not currently
exposed (`wrap_count` is computed and discarded); the `Phi` production batch must expose and retest
it first (a prerequisite alongside `defining_block`). **Overflow handling**: `u8`/255 is still
treated as an error by the budget gate; this document does not solve “a large function has
insufficient registers” (a downstream consequence of `compile_pattern` recursion depth).

The dev-dependencies at `Cargo.toml:37-39` are only `criterion` / `proptest`; `[dependencies]`
contains no regalloc / petgraph / graphlib. Introducing any such crate would add a new dependency
surface.

### Splitting Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA form and criteria must exist first**. The reason is that
decomposition itself is C1 (pure movement, judged by zero-diff snapshots), but the decomposition
will not carry old ordering assumptions into a new file unless `arg_regs` semantics have first been
eliminated by SSA down to “one independent `ValueId` per argument.”

**Core design constraint: ownership of `arg_regs` remains at the top of the function; the six
branches only `push` and never reorder.**

```
generate_call_expr_ir (746 lines)
├── Argument evaluation: evaluate in source order and push into arg_regs — shared by all six branches and the only place append is allowed
├── CallArgs (value object)
│   └── arg_regs: Vec<Operand>  — created only at the top; the six branches only push and never re-collect/reorder
├── Branch 1 → emit_namespace_call(args: CallArgs)         [extract the 7395 branch]
├── Branch 2 → emit_bound_call(args: CallArgs)              [extract the 7406+ branch]
├── Branch 3 → emit_struct_ctor(args: CallArgs)             [extract the 7724 branch]
├── Branch 4 → emit_plain_call(args: CallArgs)              [extract the 7835-7848 branch]
├── Branch 5 → emit_curried_call(args: CallArgs)
└── Branch 6 → emit_method_call(args: CallArgs)             [extract the 7420 / 7493 branches]
```

The “fill with 0” logic in `7835-7848` (`7838-7846`) moves down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the
self-description that “the normal path never reaches this” is replaced with an assertion**: if any
slot is absent and type checking did not report E1010, return a diagnostic directly. **This turns a
silent fallback into a hard error and is the document’s most direct improvement to defect class 2.**

**Criteria**: use zero-diff snapshots for the pure-movement portion (with the SSA form unchanged);
the assertion-based `fill_missing` is C4 and uses a corpus differential + the first-layer validator.

### `synth.rs` Boundary: Turning an Implicit Callback into an Explicit Synthesized-AST Boundary

**Implemented form**: `src/middle/lower/synth.rs` is the only L3/L4 production module permitted to
construct `ast::Expr` / `ast::Stmt` / `ast::Type`. Pattern matching, read-only queries, and test
fixtures are not synthesis. The current production synthesis sites are spawn’s Lambda/Stmt and
spawn-for’s Lambda, all of which pass through this module.

AST nodes do not carry a lowering environment, so `SynthEnv` is not an AST field and is not
pretended to be consumed by AST helpers. It contains runtime environment operands and named-capture
names, and the caller explicitly supplies it to `generate_expr_ir_with_env` alongside the
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

The environment flows as follows:

- spawn and spawn-for construct `SynthEnv` and explicitly pass it into lowering alongside the
  synthesized Lambda; they no longer pre-write `pending_env_vars` / `pending_env_names`. The total
  environment slot count is derived from `env_vars.len()`, and named upvalues are offset from the
  length of the anonymous prefix; spawn-for’s anonymous iteration value, outer captures, and
  parameters therefore each occupy their proper positions.
- Ordinary lambdas complete the environment from the existing free-variable collector. List
  comprehensions, nested Lambdas, and nested spawns are traversed by the same collector, with
  generator variables and nested parameters excluded according to lexical scope.
- Closure bodies register `SynthEnv` as `BindingLocation::Upvalue` in the existing symbol table.
  Variable reads and namespace-shadowing queries share this lexical binding and no longer read
  `closure_captures`.
- Nested closures first read the outer upvalue through `capture_operand`, then pass the value to the
  inner environment; an outer local slot with the same name is never mistaken for a captured value.
- `NestedBodyGuard` records the symbol-table depth on entry and truncates to that depth on exit
  (including early return via `?`), so capture bindings do not leak into subsequent unrelated
  lowering.
- Sema’s expression-body function check parameterizes existing inputs with
  `body_stmts + expression_body + span`, directly calls `check_expr`, and reuses the tail validation
  instead of synthesizing a Block/Stmt/Expr; there is no sema→middle/lower reverse dependency or
  additional architectural exception.
- SpawnFromList shares the `env + [anonymous item]` argument layout between Embedded and Standard
  and injects upvalues from the same environment. The anonymous iteration value is fixed at env[0],
  named captures are read after the anonymous prefix, and parameters start after the entire
  environment.
- Dual-runtime behavior tests explicitly call main() and separately assert, inside a spawn-for task,
  both the no-capture item value and the item + outer capture; the no-capture form also covers the
  regression restoration introduced by 74febd1f. IR assertions and actual execution are recorded
  separately, and zero drift across the existing 403 corpus cases is not presented as behavioral
  coverage of this form.

**Criterion**: D22’s synthesized-AST boundary is guaranteed by code structure—L3/L4 production code
constructs `ast::Expr` / `ast::Stmt` / `ast::Type` only in `src/middle/lower/synth.rs`; pattern
matching, read-only queries, and test fixtures are not synthesis, and synthesis sites outside
`synth.rs` are rejected during review. The original `scripts/ci/check-synth-boundary.py` and the CI
`synth-boundary` job were removed under the user’s 2026-11-24 Decision D and are no longer
maintained as an automatic scanning gate: heuristic token scanning cannot cover arbitrary legal use
/ alias / re-export forms (successive gap closures were still bypassed by `pub type` re-exports),
and the maintenance cost exceeds the benefit.

D1/D2: `SynthEnv` and `BindingLocation` are both new internal types introduced in this batch, not
reused existing types. The former centralizes production-side environment operands, named items, and
slot derivation as one explicit parameter; the latter elevates the original `SymbolEntry.local_idx`
into a discriminable Local/Upvalue location. The alternatives are to continue passing three shared
fields (which cannot prevent nested overwrites or erroneous residue), or to pass only `Vec<Operand>`
(which cannot express the correspondence between the anonymous prefix and named captures); both are
rejected for correctness reasons. The new concepts are not synonymous with existing ASTs or
consumption ledgers and require neither import aliases nor conversion tables.

### Handling Span-Keyed Problems (the Parts SSA Does Not Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism one: consumption-coverage assertion (mandatory, lowest cost; superseded by a stronger
form on 2026-10-13).** Document 07 already defines `test_release_plan_spans_consumed` (now settled
by `PlanId`). This document originally proposed turning it into a **runtime counter** rather than
merely a test. The implemented form is stronger: a **consumption ledger (`ConsumptionLedger<K>`) + a
single settlement point (E-level `W1090`)**, introducing no new diagnostic code and not relying on
an internal ir_gen counter to prove itself:

```rust
// Around ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each :1942 hit, self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* New diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a value that should have been dropped was not dropped” may not immediately
crash under YaoXiang’s current value semantics + Arc/Rc model**—which is exactly why it was silent.
Counting turns “silence” into “detectability.”

**Mechanism two: change Span keys to `DefId` or explicit plan IDs (**landed on 2026-10-13**: RFC-039
D20 / WBS 4.5 implemented this mechanism as `PlanId(u32)` structural-identity keys + mandatory
consumption settlement; see §2’s revision note in [02](02-stage-contract.md)).** The fundamental fix
is **not to use Span as a key**. A Span is a source location and may drift because of macro
expansion, `include`, or multi-file merging. `FunctionIR` already has a `def: Option<DefId>` field
(`ir.rs:677`), demonstrating that DefId is available at the IR layer. **Design judgment**: introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, assign it when ownership / overload
resolution produces the plan, and match by `PlanId` when the IR consumes it. **This belongs to 02’s
obligation ledger (`Obligations`) and should be implemented with [02](02-stage-contract.md), not in
this document.**

**Mechanism three: `method_def_ordinals` definition-order dependence (independent small item).**
This field is a `HashMap<String, usize>` (`ir_gen.rs:224`) whose key is
`"{type_name}.{method_name}"` (**without module qualification**, written at `:1691-1700`). If two
modules define the same `Type.method`, they share one counter and the mixed `#N` names drift
accordingly. **Its failure mode is not a Span mismatch but an ordering/name mismatch**—if
type-checking registration order differs from ir_gen definition order, the same method name may
resolve to a different target. Disposition: have the type-checking side **produce
`HashMap<DefId, String>` once** (bare names or mixed `#N` names), and let ir_gen only look up and
never modify it. **This eliminates one mutable cross-function state in `ir_gen`.**

### Tracking Three Hardcoded Discarded Fields

**Verified facts.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hardcoded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                                           |
| ------------------ | -------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count becomes zero after a `.42` round trip                                                                           |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **`.42` artifacts lose the entire exception table**                                                                                   |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` artifacts lose global-variable information** (note that `global_names` at `2347` **is** populated, creating an inconsistency) |

**Design judgment (revised on 2026-10-05, Decision D52): all three are brought into this document’s
scope as P7 batch e (7e), rather than creating separate issues.** The original judgment and the
reasons it was overturned are recorded below:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields) and are
   unrelated to SSA.~~ This is true, but it is not grounds for exclusion—P7 already modifies
   `bytecode.rs`, and fixing all three together requires only one `VERSION` bump (4→5); splitting
   them into two version bumps would be more expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → bumping the version → expanding the
   scope from “IR form” to “artifact format.”~~ The versioning mechanism already exists (`MAGIC` +
   `VERSION` header, rejected by the reader on mismatch), and `.42` is a build artifact for which
   cross-version compatibility is not a goal—the expansion is in effort, not risk.
3. ~~The actual risk level of `2315` / `2341` is low, and directly running `.42` is a secondary
   path.~~ **Overturned**: losing the exception table makes throw/try behave incorrectly when `.42`
   is run directly—throw/try is core language semantics; a secondary path does not mean it may
   silently be wrong. The principle of this refactoring is **not to leave core functionality as an
   unimplemented legacy item** (`Vec::new() // Not implemented yet` is precisely such an item).

**At the same time, retain**: all three must still be registered in the `Obligations` ledger as
“produced but not serialized” until 7e is complete, to prevent them from becoming a fourth kind of
“thing nobody knows about.” `2312` lands with the version bump (D17), and the allocated but unused
opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Knock-on Effects of IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                                                         |
| ------------------------------------------------------------------------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Add the `ValueId(u32)` type                                               | File header (`ValueId` definition)            | New (**landed in 7b.1a**)                                                                                      |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**landed in 7b.1a**)                                                           |
| Add the `Phi` variant to `Instruction`                                    | End of the `Instruction` enum (`Phi` variant) | **+1 variant** (**landed in 7b.1a**; the enum now has 75 variants)                                             |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | +1 field (**landed in 7b.1b**)                                                                                 |
| Add the `ValueInfo` structure                                             | `ValueInfo`                                   | New (**landed in 7b.1b**)                                                                                      |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors with the same names                 | Add `values()` / `values_mut()` accessors (**landed in 7b.1b**, with the same shape as the existing accessors) |

The `pub use ...ast::Type` at the top of the file is **not changed by this document**—it belongs to
the scope of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **converged**,
single type representation.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                          | Location                              | Nature                                                                                                     |
| ------------------------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or adopt the “expand to Move at runtime” strategy | `127-554` (currently **66** variants) | +1 or +0 variants (see below)                                                                              |
| Add a `Phi` arm to `opcode()`                                                   | `558`                                 | Mandatory (`BytecodeInstr` uses an exhaustive match)                                                       |
| Add a `Phi` arm to `size()`                                                     | `649`                                 | Mandatory                                                                                                  |
| Add a `Phi` arm to the `From<BytecodeFile>` decoding match                      | `943-2350`                            | Mandatory                                                                                                  |
| Correct `upvalue_count: 0`                                                      | `2312`                                | Fixed incidentally by this document (in-memory format; persisting it requires a `VERSION` bump; see below) |

**The bytecode form of `Phi` requires a design decision**. There are two options (**design judgment:
recommend A**, for the reasons given under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded by `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  with the incoming predecessor block index selecting the value. **No opcode is added**, and the
  `.42` format does not change. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode + predecessor-index encoding, and have the interpreter maintain register
  versions on jumps. **This requires changing the `executor/` state model + the `.42` format + the
  version number**. The only benefit is size.

The decisive reason for choosing A is that **a `.42` format change triggers version-number issues
and a full compatibility discussion, while A’s size cost is irrelevant in interpreted execution**
(the interpreter executes Move instructions, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions, distributed across `101-1558`)

| Change                                                           | Location                                                                                                                         | Nature                                                                                       |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                                     | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add an arm                                                                                   |
| Translation arm for `Instruction::Phi`                           | Inside the dispatch match, before `Free` (533)                                                                                   | New branch that expands into a sequence of Move instructions (option A)                      |
| Keep the positions of the existing 9 NOP lowerings **unchanged** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9 instructions**: `655-657` covers 2 variants, and `658-660` covers 4 variants |

**SSA-ification does not reduce the number of NOPs.** These 9 lowerings reflect
expressive-capability gaps where “the IR has it, but bytecode does not,” and SSA-ification does not
touch them.

#### `src/backends/interpreter/executor/`

Under option A, `executor/` requires **no changes whatsoever**—`Phi` disappears during the
`translator.rs` phase and is never visible to the interpreter. **This is the second decisive reason
for choosing A.**

`BytecodeInstr::Switch` has an active implementation at `executor/ops/control.rs:90`, but `ir.rs`
contains **no** `Switch` / `BrTable` / `JumpTable` variant (all three names have zero hits in
`ir.rs`)—**this is a reverse gap**: the bytecode layer contains instructions that the IR layer
cannot produce. It does not affect this document, but it is recorded here because it shows that “IR
→ bytecode” is not surjective.

The dispatch match in `executor/debug.rs:194` likewise requires no changes (there is no new opcode
under option A).

### Compiler Change List

**File by file and function by function. All line numbers are the verified starting points for each
change:**

| No. | File                                                     | Location                                                     | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| --- | -------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                                    | `Operand` enum definition                                    | Add/remove `Operand` variants (**landed in 7b.1a**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 2   | `src/middle/ir/ir.rs`                                    | End of the `Instruction` enum (`Phi` variant)                | Add `Instruction::Phi` (**landed in 7b.1a**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 3   | `src/middle/ir/ir.rs`                                    | `FunctionBody::Code`                                         | Add `values` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 4   | `src/middle/ir/ir.rs`                                    | `ValueInfo`                                                  | Add `ValueInfo` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 5   | `src/middle/ir/ir.rs`                                    | `values()` / `values_mut()` accessors                        | Add accessors (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 6   | `src/middle/ir/ir.rs`                                    | `pub use ast::Type`                                          | **Do not change `pub use ast::Type`** (scope of [03](03-type-unification.md); recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 7   | `src/middle/core/ir_gen.rs`                              | `884-891` `next_temp_reg`                                    | Return `Operand::Value`, increasing monotonically without rollback (**landed in 7b.2**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; add an independent `next_value` counter that starts at 0 for each function body, while the three statement-level reclamations only roll back `next_temp` and do not touch it)                                                                                                                                                                                      |
| 8   | `src/middle/core/ir_gen.rs`                              | `1954-1959`                                                  | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 9   | `src/middle/core/ir_gen.rs`                              | `3632-3638`                                                  | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 10  | `src/middle/core/ir_gen.rs`                              | `4737-4742`                                                  | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 11  | `src/middle/core/ir_gen.rs`                              | `1745-1759` / `1819-1822`                                    | Replace with an RAII guard (`generate_method_ir`, entry at `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 12  | `src/middle/core/ir_gen.rs`                              | `1858-1866` / `2026-2029`                                    | Replace with an RAII guard (`generate_function_ir`, entry at `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 13  | `src/middle/core/ir_gen.rs`                              | `2116` / `2180`                                              | Replace with an RAII guard (`generate_curry_innermost_func`, entry at `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 14  | `src/middle/core/ir_gen.rs`                              | `2224-2229` / `2284-2288`                                    | Replace with an RAII guard (`generate_curry_function_ir`, entry at `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 15  | `src/middle/core/ir_gen.rs`                              | `2768-2777` / `2820-2851`                                    | Replace with an RAII guard (`generate_anon_binding_ir`, entry at `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 16  | `src/middle/core/ir_gen.rs`                              | `4686-4695` / `4767-4774`                                    | Replace with an RAII guard (`generate_lambda_body_ir`, entry at `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 17  | `src/middle/core/ir_gen.rs`                              | `1987-2000`                                                  | Extract the E3014 check into `check_register_budget()`, shared by `1798`/`2001` after their accounting is unified                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 18  | `src/middle/core/ir_gen.rs`                              | `1798`                                                       | Change `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)` to eliminate inconsistent accounting                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 19  | `src/middle/core/ir_gen.rs`                              | `248-249`                                                    | Delete the comment contract (now enforced by the type system) and replace it with a reference to `verify`                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 20  | `src/middle/lower/loops.rs`                              | `generate_spawn_for_ir`                                      | Change the call to `synth::synth_closure` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 21  | `src/middle/lower/expr/spawn.rs`                         | `generate_spawn_expr_ir`                                     | Change the call to `synth::synth_closure` and remove writes/clears of shared pending fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 22  | `src/middle/lower/expr/closure.rs`                       | `generate_lambda_expr_ir`                                    | Read `SynthEnv` from the parameters instead of `mem::take`ing shared fields (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 23  | `src/middle/lower/{mod,scope}.rs`                        | Symbol table and `NestedBodyGuard`                           | Remove the `closure_captures` field; replace it with `BindingLocation::Upvalue` lexical binding and depth-based restoration (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                              |
| 24  | `src/middle/lower/expr/{access,closure}.rs` + `query.rs` | Variable and namespace queries                               | Replace `closure_captures.contains_key` with a lexical-upvalue lookup; unify capture materialization through `capture_operand` (**landed in 7c.1**)                                                                                                                                                                                                                                                                                                                                                                                           |
| 25  | `src/middle/core/ir_gen.rs`                              | `341` + `1942`                                               | Add Span-keyed consumption counting                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 26  | `src/middle/core/ir_gen.rs`                              | `224` / `369` / `1692-1702`                                  | Make `method_def_ordinals` read-only                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 27  | `src/middle/core/ir_gen.rs`                              | `7269-8014`                                                  | **Do this last**: decompose into a top-level `CallArgs` + 6 `emit_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 28  | `src/middle/core/ir_gen.rs`                              | `7838-7846`                                                  | Replace the “fill with 0” fallback with a returned diagnostic                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 29  | `src/middle/lower/synth.rs`                              | **New file**                                                 | Synthesized-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 30  | `src/middle/ir/verify.rs`                                | **New file** (first layer of [07](07-equivalence-oracle.md)) | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 31  | `src/middle/passes/codegen/translator.rs`                | Dispatch match                                               | Translate `Instruction::Phi` (**landed in 7b.3**: option A expands it at **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit `Move` groups per predecessor, existing `JMP` instructions skip between groups, and incoming-edge jumps are redirected to `(predecessor, convergence-block first instruction)` at the group start; the `Phi` arm in `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return slot cannot hold the multiple per-predecessor Move instructions, so arrival fails loudly**) |
| 32  | `src/middle/passes/codegen/operand.rs`                   | `36-45` / `72-77`                                            | Add the `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`) (**landed in 7b.2**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); removal of the `Temp` arm landed first in 7b.1a)                                                                                                                                                                                                                                             |
| 33  | `src/middle/core/bytecode.rs`                            | `127-554`                                                    | **Unchanged under option A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 34  | `src/middle/core/bytecode.rs`                            | `558` / `649` / `943-2350`                                   | **Unchanged under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 35  | `src/middle/core/bytecode.rs`                            | `2312`                                                       | Correct `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 36  | `src/backends/interpreter/executor/**`                   | —                                                            | **Zero changes under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 37  | `src/middle/lower/regalloc.rs`                           | **New file**                                                 | Linear-scan allocator (**landed in 7b.4**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)` returns `value_slots` and `frame_slots = named_base + max_live`; overflow returns `Budget` for `finalize_code_body` to report E3014)                                                                                                                                                                                                                                                                                  |
| 38  | `scripts/ci/check-synth-boundary.py`                     | **Canceled**                                                 | The boundary-sentinel script was canceled under the user’s 2026-11-24 Decision D; no replacement is created                                                                                                                                                                                                                                                                                                                                                                                                                                   |

### Backward Compatibility: Version Number for the `.42` Format

**Verified facts**: the `.42` on-disk format **already has a versioning
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (“YXBC”) and
`VERSION: u32 = 4`, writes them into the header (`:342-343`), and validates them when reading
(`:454-467`, reporting “unsupported bytecode version” on a mismatch).

**Design judgment (a direct consequence of option A)**:

- `Phi` is expanded into a sequence of `Move` instructions during the `translator.rs` phase and
  **produces no new opcode**.
- The 83 constants in `opcode.rs` **remain unchanged**, as does `opcode_name()` (`120`) and the
  decoding match (`bytecode.rs:943-2350`).
- The field layout of the `.42` format **does not change**, and `VERSION` remains 4. Old `.42` files
  can still be read by the new binary, and new `.42` files can still be read by the old binary.
  **SSA-ification itself does not require a version bump.**

**The only scenario requiring a version bump is the correction to `2312 upvalue_count`** (Decision
D17: fix it and handle it together with the version bump): add an `upvalue_count` field to
`BytecodeFunction` and have the encoder write it. This is a **new field in the format**, raising
`VERSION` from 4 to 5. The reader’s existing version check rejects old files according to its
existing behavior—`.42` is a build artifact (`main.rs:671`), and cross-version read compatibility is
not a goal, so the version bump requires no additional migration mechanism. The allocated but unused
opcode values (D32/D34: `Switch` / `TailCall`) are reclaimed in the same batch.

## Implementation

### Batch Strategy

Divide the work into four batches. **Every item within a batch gets an independent commit,
independent criterion, and independent revert**. The “Change Item” column below refers to the
numbering in the “Compiler Change List.”

| Batch       | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Change items                 | Criterion                                                                                                                                                              | Rollback point                                                                                                                                                     |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Batch a** | Break multiple definitions: delete the 3 statement-level reclamations → replace 6 save/restore sites with an RAII guard → unify accounting                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 8-10 (first) → 11-16 → 17-18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and accounting unification are pure movement: **C1** zero-diff snapshots | One commit per item; `git revert` rolls it back                                                                                                                    |
| **Batch b** | Switch to the SSA form: `ir.rs` structural changes + mechanical replacement of temporary values with `Operand::Value` → `Phi` arm in `translator.rs` (option A) → linear-scan allocator. Within the batch, WBS level-three tasks run serially: 7b.1a (`Operand` variant convergence + `Instruction::Phi` introduction, **landed**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **landed**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **landed**) → 7b.3 (`Phi` arm in `translator.rs` + `BasicBlock.successors` population, **landed**) → 7b.4 (linear-scan allocator + deletion of the three statement-level reclamations, together with 7a.1, **landed**) | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + all-green `verify_ssa` + `.42` round-trip test                                                                                        | The `ir.rs` changes must **occupy a commit by themselves** (the type changes affect compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary → Span consumption counting → make `method_def_ordinals` read-only (the sentinel script was canceled under the user’s 2026-11-24 Decision D)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | 20-24, 29 → 25 → 26          | **C4**: behavioral equivalence; consumption counting produces zero hits on the new diagnostic (otherwise, a real mismatch exists)                                      | Independent commit                                                                                                                                                 |
| **Batch d** | Decompose `generate_call_expr_ir` + turn the “fill with 0” fallback into an assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | 27-28                        | Pure-movement portion: **C1** zero-diff snapshots. Assertion portion: **C4** corpus differential + first-layer validator                                               | Independent commit                                                                                                                                                 |

**Three dependencies whose order cannot be exchanged:**

1. Batch a’s “delete reclamation” comes after batch b’s `ir.rs` changes (the three statement-level
   reclamations were removed with 7b.4 and the linear-scan allocator in the same batch, with 7a.1
   executed alongside them)—first make `next_temp` increase monotonically, so that `ValueId` has the
   semantic foundation of “single definition” (this semantic only holds after `next_temp_reg` starts
   producing `Value` in 7b.2 and the `Phi` arm lands in 7b.3).
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise the decomposition would carry
   the old `arg_regs` ordering assumptions into the new file unchanged.
3. Batch d must come last—it depends on the previous two batches having converged `arg_regs`
   semantics down to “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within **P7** of RFC-039, and criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial** relative to
one another, but the 8-10 / 11-16 / 17-18 items within batch a have only ordering constraints and no
coupling.

### Preconditions (Hard)

**Precondition one: criterion baseline.** The `verify_loose` mode of the first-layer validator in
[07](07-equivalence-oracle.md) **must first run green on the existing (non-SSA) IR** before batch a
may begin.

The reason is that SSA-ification is judged by C4 (behavioral equivalence + invariants). If the
invariant validator itself cannot run green before the refactoring, every acceptance of batches a–d
lacks an executable criterion and can rely only on corpus differentials—yet corpus differentials are
**precisely insensitive** to defect classes 1/2/3 (as demonstrated by “Alternative A” in 07). **This
is a genuine ordering constraint.**

**Precondition two: convergence of type representations.** [03](03-type-unification.md) must
complete the convergence of `pub use ast::Type` at `ir.rs:3` and remove the
`From<MonoType> for IrType` bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` would
introduce a third set of type-compatibility logic.**

### Rollback Strategy

- Each change gets an independent commit, with `git revert` granularity equal to change-item
  granularity.
- The `ir.rs` structural changes in batch b are **the only part that cannot be rolled back
  incrementally** (type changes affect compilation throughout the repository). They must therefore
  **occupy a commit by themselves**, and batch a must already have run green and established a
  revertible baseline before them.
- **Do not use a feature flag.** The reason is that adding/removing `Operand` variants cannot be
  isolated by a runtime switch; a feature flag would cause CI to stop testing the other path in the
  long term—a recurrence of the “test wiring depends on human memory, and silent decay follows”
  problem recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated clearly up front, or implementation will be questioned midway for “not
reducing the line count.”**

| Item                                      | Line-count change                                                                                                                                                                            |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                            | `ir_gen.rs` **8,448**                                                                                                                                                                        |
| Delete the 3 statement-level reclamations | **−15**                                                                                                                                                                                      |
| Replace 6 save/restore sites with RAII    | Net ±0 (the RAII guard definition is about +20 lines, while the use sites fall from 22 lines to about 12)                                                                                    |
| Modify `next_temp_reg`                    | +5                                                                                                                                                                                           |
| Decompose `generate_call_expr_ir`         | **+40–80** (the overhead of signatures and parameter passing for 6 `emit_*` functions; this is the typical cost of a pure-movement decomposition—**decomposition increases the line count**) |
| Span-keyed consumption counting           | +15                                                                                                                                                                                          |
| Make `method_def_ordinals` read-only      | +20                                                                                                                                                                                          |
| New `synth.rs` file                       | +80–120                                                                                                                                                                                      |
| New `verify.rs` file                      | +400–600 (the first layer of 07, although it is within this document’s scope it is counted separately)                                                                                       |
| New `regalloc.rs` file                    | **+500–1,000** (measured at 271 lines, 7b.4)                                                                                                                                                 |
| **Total**                                 | **`ir_gen.rs` grows from about 8,448 to 8,600–8,800; including new files, the net increase is 900–1,600 lines**                                                                              |

**The early expectation that “8,448 will become 5,000–6,000” was wrong**; the measured projection
points in the opposite direction. The reason must be explained:

1. **The core benefit of this document is not reducing the line count; it is eliminating an entire
   class of defects that “fail silently.”** The six manual save/restore sites (22 lines) become
   about 12 lines plus a guard type after SSA + RAII—**the line count is almost unchanged, but
   “forgot to restore” changes from a possibility into a compile error**.
2. **A pure-movement decomposition necessarily increases the line count.** Decomposing the 746-line
   `generate_call_expr_ir` into 7 functions repeats the signature (the function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), the `&mut self` binding, and error propagation
   for each one. **“Splitting the file reduces the line count” is a misconception—it reduces
   cognitive load, not physical lines.**
3. **The new verifier and allocator are net line-count increases.** These two components (about
   900–1,600 lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of lines, this document will most likely produce a **net increase**. The benefit lies in
> **verifiability**—turning defects that “fail silently” into invariants that “fail the build.” If
> line count is used as the acceptance criterion, this document will fail; the correct acceptance
> criteria are those in the “Batch Strategy” section.

**This stage intentionally increases the amount of code in `ir_gen.rs`.** The 2026-10-03 decision
canceled all line-count / size gates (see Prohibition Two in [08](08-maintenance-mechanism.md)), so
**no line-count exemption list is needed**—the legitimacy of the increase is established by the
separation-of-responsibilities argument: SSA-ification extracts “register allocation + definition
discipline” from lowering into independent modules, trading increased code size for the elimination
of six manual save/restore sites and the `arg_regs` semantic-reordering discipline that produces
“misalignment without errors, only wrong values.”

## Key Decisions and Rationale

### Three Core Decisions

**Decision one: temporary-value strategy—delete the three statement-level rollbacks so that single
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, and every temporary slot naturally has exactly one definition point.
**Manual-copy discrepancies such as “the criterion is the same, but the guard differs”** (`3632` has
a `result_reg.is_none()` guard, while the other two do not) disappear as well. The cost is increased
slot occupancy—the existing 255 upper-bound check on `temp_high_water` (`1992-2000`) provides a
backstop.

**Decision two: do not perform mem2reg; perform explicit renaming only.** The reasons are given
under “Value Identification Scheme”: YaoXiang’s `&` / reference / ownership semantics make “which
slots may be promoted to registers” a type-system problem that requires the type representation
converged by [03](03-type-unification.md). **Doing mem2reg before 03 is guessing.** Explicit
renaming only needs to change the return-value semantics of one `next_temp_reg` and delete three
rollbacks; it does not touch the control flow of any lowering algorithm.

**Decision three: use option A for `Phi` (an IR-only pseudo-instruction expanded into a sequence of
Move instructions).** `executor/` and the `.42` format require **zero changes**, and the regression
surface is confined to L3. The decisive reason is that a `.42` format change triggers version-number
issues and a full compatibility discussion, while option A’s size cost is irrelevant under
interpretation (the interpreter executes Move instructions, not Phi). The cost is that `Phi` is not
visible in `dump_bytecode` (when entering the C1/C2 stages, note that the third layer in 07 compares
it).

### Rejected Alternatives

**A. Decompose everything at once (simultaneously performing SSA + decomposition + the `synth`
boundary + the allocator). Rejected.** This violates 07’s core principle: criteria must be tiered by
category. Mixing all four batches into one PR mixes pure-movement changes with C4 changes, making
snapshot drift impossible to attribute—if a snapshot changes, it is impossible to tell whether the
decomposition moved something incorrectly or SSA changed it incorrectly. **Rollback granularity
degrades from “change item” to “everything.”**

**B. Split the file first, then optimize the ordering. Rejected.** Decomposition does not eliminate
any root cause: the six manual save/restore sites remain scattered across 10 files, and the
`arg_regs` semantic reordering still requires manual reasoning. Worse, **splitting first loses the
opportunity to “establish criteria before changing anything”**—decomposition produces a large
line-level diff that gets mixed into subsequent SSA diffs, making review ineffective. This has the
same root cause as the 8 orphan test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78
tests that never run): **with the wrong order, every subsequent step accelerates along an incorrect
foundation.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as “a third path once considered.”** The form retains `ir_gen.rs` unchanged and adds a pass that
converts its non-SSA IR into SSA IR. Its **advantages** are that lowering is untouched and rollback
is easy. **Reasons for rejection**: (1) It does not eliminate the root cause of defect class 1—the
three statement-level reclamations remain in `ir_gen.rs`, and the six save/restore sites remain
separated by 59–161 lines. A conversion pass can merge multiple definitions into `Phi`, but **it
cannot fix “the inner function’s names leak into the outer scope”**—that is a `cur_locals`
save/restore error occurring before the IR form, and the conversion pass cannot see it. (2) It turns
SSA into a second representation—the repository already has the cautionary tale of 3 parallel type
representations, and this project’s core lesson is that **“boundaries exist only in people’s
awareness.”** (3) `verify_loose` becomes a permanent burden.

**D. Keep the current implementation and only add a verifier. Rejected as the sole solution, but it
is part of this document.** Alternative B in RFC-039 reaches the same conclusion: a gate can prevent
regression but cannot fix the existing implementation. **It must nevertheless be clear what D can
accomplish**: D can **catch instances** of defect classes 1/2/3 (provided that the verifier covers
dominance, type consistency, and inner isolation), but it cannot catch defect class 4
(`closure_captures` leakage)—that requires a cross-function liveness assertion beyond the function
boundary of `verify(&ModuleIR)`. **D is therefore not an alternative to this document; it is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point**—the 2026-10-03 decision canceled line-count / size gates, and the scale issue is addressed
  through separation of responsibilities (see Prohibition Two in [08](08-maintenance-mechanism.md)).
  The increase is justified by the fact that it replaces six manual save/restore sites and the
  `arg_regs` semantic-reordering discipline that produces “misalignment without errors, only wrong
  values.”
- **The “single definition” check in `verify.rs` is meaningless in the non-SSA form**
  (`Operand::Local` permits multiple definitions)—07 already records this limitation, and
  `verify_loose` is its degraded mode before this batch.
- **Deleting statement-level reclamation changes the temporary-slot allocation pattern** (executed
  in 7b.4, together with 7a.1): after deletion, slots are reused by the linear-scan allocator
  according to live intervals, so **the allocation result changes**—this is precisely why C4 must
  genuinely run the full corpus; the measured 403/403 differential is empty.
- **`u8` / 255 slot limit**: the two corpora that exceeded the limit after reclamation
  (`string_ops.yx`, 354 monotonic slots; `list_comp.yx`, 323 monotonic slots) are compressed by the
  allocator to **15 / 24** frame slots; the budget gate now reports E3014 only when the slots
  genuinely do not fit after allocation (with the function-body declaration span), rather than
  falsely rejecting based on the cumulative temporary count.
- **The advantage must also be recorded**: it turns “errors that cannot be tested” into “testable
  invariants”; it does not modify the backend (`executor/` and `.42` remain unchanged); it does not
  change the semantics of `Instruction` (all 76 → 77 variants are retained, and all 50 `translate_*`
  functions are retained); and it is incremental (each change can be reverted independently).

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for each
> decision. **This document leaves no unresolved items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) — the
  overarching design; four-layer model, criterion tiers, P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definition of the C4
  criterion class, `verify_loose` / `verify_ssa`, three-layer criteria, and the snapshot tool’s
  immunity to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — the `Obligations` ledger; ownership of the `PlanId`
  refactoring for `ReleasePlan`
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; the cautionary tale of
  “creating a directory first, then failing to wire it up”

### Code Locations

**IR definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — 7 `Operand` variants
- `src/middle/core/ir.rs:47-533` — 76 `Instruction` variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for the `PlanId`
  option)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect class 1 (multiple definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for the 7 nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — the sole table-retrieval + E3014 budget
  point (`MAX_REGISTERS = 255`), combining block splitting / slot allocation / table retrieval
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — the linear-scan allocator (the sole producer of `value_slots` and
  `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamation
  removed (7b.4); slots are now reused by the allocator
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant
  referenced in 07

**Defect class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  the clone
- `src/middle/core/ir_gen.rs:7838-7846` — the silent “fill with 0” fallback

**Defect class 3 (Span keying)** — the following are **evidence pointers from the time of the
audit** (after the 4.5 key migration: both `release_plan` and `overload_resolutions` keys have
become `PlanId`s; see §2’s revision note in [02](02-stage-contract.md); `method_def_ordinals`
remains the mechanism-three to-do item)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of the three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization of `method_def_ordinals` and its
  sole write point (the key is the unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the propagation structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer propagation
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect class 4 (implicit order dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — the equivalent handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — the two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — three other `ast::Type` construction sites

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — 66 `BytecodeInstr` variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hardcoded discards (compare `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (the bridge awaiting removal
  in [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  lowerings
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  bound of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants in total
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Test status**

- `src/middle/core/ir_gen.rs` — 8,448 lines, 0 `#[cfg(test)]` blocks, 0 `mod tests` modules
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no IR structural
  invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — four references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example per-opcode round-trip criterion; the final test in the file; the file contains 1,209 lines
  / 23 `#[test]` functions)

**Engineering configuration**

- `Cargo.toml:37-39` — dev-dependencies are only `criterion` / `proptest`; `[dependencies]` contains
  no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans the `/rfc/draft`
  directory; this document is not in that directory and is referenced by RFC-039 and this
  directory’s index
