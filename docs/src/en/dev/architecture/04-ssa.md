# SSA Transformation of the Intermediate Representation

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution-phase order are covered in the main
> RFC-039 text; the positioning of each companion document is covered in the
> [index for this directory](index.md).

## Positioning and Scope

This article addresses L3 IR construction discipline and proposes a way to construct
`src/middle/core/ir_gen.rs` (8,448 lines) around the principle of “one definition per value.” **The
goal is not to introduce a new instruction set, but to eliminate three classes of discipline defects
that fail without producing any error**:

1. Six manual register save/restore sites separated by dozens of lines;
2. Semantic reordering of shared `arg_regs` across six branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This article is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, explicit cross-layer contracts, decomposition of
call-expression dispatch, and the wiring of the static IR verifier.

This article is not responsible for: type-representation convergence ([03](03-type-unification.md)),
stage contracts and the obligation ledger ([02](02-stage-contract.md)), the definition and gating of
equivalence criteria ([07](07-equivalence-oracle.md)), frontend paradigms
([05](05-frontend-paradigm.md)), or cleanup of dead code and hollow designs
([06](06-cleanup-inventory.md)). This article repeatedly cites the conclusions of those documents
without repeating their arguments.

**The criterion category** follows the **C4 (IR shape change)** category in
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, not byte-for-byte equality of IR snapshots. SSA transformation necessarily changes the
IR shape; forcing snapshot equality would encourage the team to weaken the criteria, which is
precisely the trap recorded in 07.

**Its position in the RFC-039 stage sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`), with acceptance based on behavioral equivalence and a green `verify_ssa`. The
implementation-batch numbers within this article (batches a–d) are distinct from the ordering within
P7: the former are this article’s incremental strategy, while the latter are RFC-039’s global
stages.

## Feasibility Preconditions

**Verified fact.** The IR already has **all the structural prerequisites required for SSA
construction**:

| SSA prerequisite             | Current repository state                                                                     | Location                                                               |
| ---------------------------- | -------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                             | `BasicBlock` in `src/middle/ir/ir.rs`                                  |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                               | `FunctionBody` in `src/middle/ir/ir.rs`                                |
| Scope-nesting information    | `LocalSlot::scope_depth` (`"0 = 函数参数层，供嵌套作用域重名消歧"`)                          | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Three-address form           | 76 variants, 54 with `dst: Operand`, and 3 with `dst: Option<Operand>`                       | `Instruction` in `src/middle/ir/ir.rs`                                 |
| Per-instruction span         | Every variant has a `span: Span` field                                                       | Same as above                                                          |
| Value types can be annotated | `LocalSlot::ty: MonoType`                                                                    | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Slot-count upper-bound check | Slot-table extraction and the E3014 budget gate share a single point (`MAX_REGISTERS = 255`) | `finalize_code_body` in `src/middle/lower/scope.rs` (from 7b.4 onward) |

**If these prerequisites were absent, the cost estimates in this article would be entirely
different.**

**The two decisive points are as follows:**

**First, SSA transformation does not add a capability; it removes a capability dependency.**
Temporary slots are currently **deliberately designed to be defined multiple times**:
`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`, explicitly roll `next_temp` back in
three places. The first step of SSA transformation is to **delete these three rollbacks**, making
`next_temp` increase monotonically. After deletion, **every temporary slot naturally has exactly one
definition point**—unique definition changes from “a rule that must be checked” into “a mathematical
consequence of the construction method.” Batch a only consolidated the save/restore surface (see
“Defect Class 1”); the three rollbacks were removed with 7b.4 in batch b (the allocator took over
slot reuse) on 2026-11-22.

**Second, the slot-count upper bound is a hard check, and deleting the rollbacks briefly caused the
slot count to become unbounded (resolved by 7b.4).** The budget gate in `src/middle/lower/scope.rs`
reports E3014 when the count exceeds 255. After removing the three reclamation sites and running the
full corpus, two corpus cases crossed the limit and triggered E3014: `src/std/tests/string_ops.yx`
with 354 slots and `tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; peak slot
counts in the remaining corpus increased from 51 to 211. **After 7b.4 landed:** the linear-scan
allocator reuses slots by live interval, and the two corpus cases now use **15 / 24** frame slots
(with 344 / 304 values) in practice; the other 403/403 corpus cases remain behaviorally equivalent.
From that point onward, E3014 means only “frame slots after allocation exceed 255.”

**The combined meaning of these two points is:** lowering already maintains all the bookkeeping
information required for SSA, but **throws that information away**; 7b.4 makes the allocator
responsible for slot reuse and removes reclamation. The bulk of the SSA transformation effort
therefore lies in decomposing `generate_call_expr_ir` and making cross-layer contracts explicit, not
in building IR expressive power.

### Prerequisite: Unifying Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`; `LocalSlot::ty` (`ir.rs:656`)
uses `MonoType`. The third representation on the serialization side is `ir::Type`, bridged by
`impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR contains two type representations simultaneously, and one of them directly
`pub use`s the AST type.**

SSA transformation requires a new `Phi` variant, requires each SSA value to record its type (for the
“type consistency” invariant in the 07 verifier), and requires determining whether the types of two
predecessors are compatible at a CFG join. **If type representations have not yet been unified, SSA
will require a separate type-compatibility check for every representation**—and the third
representation will grow accordingly.

This is not a question of “which should be done first”; it is a question of **which order would
produce three separate type-compatibility implementations**.
[03 Type Representation Unification](03-type-unification.md) must first consolidate the
`pub use ast::Type` at `ir.rs:3` and remove the bridge at `bytecode.rs:2353`. Item 6 of the change
list explicitly records that `ir.rs:3` **is not changed by this article**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not a file that is merely “ugly but functional.” It is a file whose **correctness
depends on line-by-line human review**:

- The file contains **0 `#[cfg(test)]` and 0 `mod tests`** (measured).
- `src/middle/core/tests/mod.rs` declares the `bytecode` / `def_assign` / `local_slots` modules,
  with 29 tests in total. `def_assign.rs` and `local_slots.rs` provide **positive** coverage of
  ir_gen’s DefId allocation and local-slot naming (at sentinel level), while all four ir_gen
  mentions in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **negative assertions**—“ir_gen
  does not produce,” “ir_gen does not construct,” or “the ir_gen frontend does not construct the
  corresponding IR.”

In other words, ir_gen has 29 sentinel-level tests pinning local behavior, but **has no IR
structural-invariant validation at all**—dominance, unique definition, jump targets, and type
consistency. None of the three modules declared by `middle/core/tests/mod.rs` makes positive
assertions about the overall IR shape.

This file also happens to contain several classes of defect that **compile successfully, pass every
check, and let the program continue running, merely producing wrong values**. Their common property
is: **failure produces no error**.

This is where the highest risk lies. The problem is not “the function is too long,” nor is it “the
naming is confusing.” The problem is that **there is no machine-executable enforcement of this
file’s core invariants**.

The contract “save before generating a nested function body and restore afterward, or the inner
function’s names will leak into the outer scope” is enforced by the type system: `NestedBodyGuard`
in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator, and frame
extraction and restoration each have only one implementation (see “Defect Class 1”).

**Why simply “adding a verifier” is insufficient**: a verifier can **catch** these defects, but it
cannot catch their **causes**. The cause is not “someone missed it during review”; it is:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator deliberately depends on this behavior.**

The concrete evidence is three statement-level register reclamations (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`). At the end of each statement, they **roll `next_temp` back** to
the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and the number and
order of writes and the live intervals are determined entirely by one manual decision: **when to
roll back**.

Under this shape, “unique definition” is not a rule that can be imposed incrementally; it **directly
conflicts with the register-allocation strategy**. Therefore, SSA transformation is not a matter of
adding a check to the existing code; it means **replacing the allocation strategy that depends on
multiple definitions**. The good news is that replacing this strategy is inexpensive—see
“Feasibility Preconditions” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (the Three Reclamation Sites Removed by 7b.4)

**Verified fact.** There are seven entry points that require save/restore before entering a nested
function body. Batch a (7a.2) consolidated them into one RAII guard: `NestedBodyGuard<'a>` in
`src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, with unconditional restoration in
`Drop` and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Rollback scope                                         |
| ------------------------------------ | ---------------------------------- | ----------------------------- | ------------------------------------------------------ |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (three counters are not rolled back) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                          |
| Nested-function definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                     |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                          |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                          |

- Frame extraction occurs only in the two constructors (`enter` / `enter_decl`), while restoration
  occurs only in `Drop`. The guard exclusively holds `&mut` access to the generator for its
  lifetime, so no second `&mut` is available in the scope; `?` and early returns all restore state
  through `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`. From 7b.4 onward,
  `next_temp_reg` only maintains the named-variable high-water mark (temporary-value slots belong to
  the linear-scan allocator; `temp_floor` was removed with the three reclamations).
- `enter_decl` does not roll back the three counters: after a function body or method body returns
  to the module-initialization flow, subsequent definitions continue from the nested body’s final
  identifier. This is the constraint for zero drift in C1 snapshots and the convention that “the
  declaration surface rolls back only the table.”

**Convention unification and a single budget point (from 7b.4 onward)**: `finalize_code_body` in
`src/middle/lower/scope.rs` is the only slot-table extraction and budget point. It extracts the
table using `frame_slots = named_base + max_live`, reports E3014 with the function-body span when
out of bounds, and is the only repository-wide definition of `MAX_REGISTERS = 255`. All six
function-body shapes—function / method / lambda / anonymous binding / intermediate curry layer /
innermost curry layer—use it for finalization. There is no second convention for “how many slots a
function body should have.”

**The three statement-level reclamations were removed (landed with 7b.4 under 7a.1, 2026-11-22)**:
reclamation was previously hand-copied in `generate_function_ir` / `generate_block_ir` (with a
`result_reg.is_none()` guard) / `generate_lambda_body_ir`. After the allocator took over slot reuse,
reclamation degenerated into an identity operation, so even the guard, the `temp_floor` /
`saved_temp_floor` bookkeeping, and the loop-entry/loop-exit bookkeeping in `loops.rs` were removed.
Module init / top-level statements still follow the old monotonic-slot path with `value_mode=false`
(without running the allocator); their slot numbers and table lengths shift with the number of named
slots in function bodies (three snapshots measured; behavioral equivalence is covered by 403/403).

**Consequences (current form)**: the guard exclusively holds `&mut`, and restoration has only one
implementation in `Drop`, leaving no writable location where a restore could be omitted. The
temporary-slot dependency on “one slot, multiple definitions” disappeared when the reclamations were
removed—value slots are now reused by the linear-scan allocator according to live intervals, giving
the value-unique-definition criterion in `verify_ssa` a construction-side foundation.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified fact (as of the audit; the current location has been synchronized with the batch-c split
under 7c).** `generate_call_expr_ir` is now in `src/middle/lower/expr/call.rs` (split out of
`src/middle/core/ir_gen.rs`, which no longer exists), as a single function. Its signature accepts
`func: &Expr` / `args: &[Expr]` / `named_args: &[(String, Expr)]` / `span: &Span` /
`call_plan_id: PlanId` / `result_reg: Operand` / `instructions` / `constants` (`call_plan_id` is the
decision-consumption key from RFC-039 D20). Multiple branches internally share the same
`Vec<Operand>` and perform **semantic rewriting** (the line numbers below are from the audited
version; after the split, locate by symbol):

| Line        | Semantics                                                                                                                                                   |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten `slots`, then recollect—named arguments are reordered                                                                               |
| `7724`      | Struct construction: `unwrap` each `final_args` slot, then recollect—fields are reordered                                                                   |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **fills a missing slot with a new register containing 0** (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—clone before emitting instructions                                                                       |

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

The comment on `7835-7848` states: “Uncovered slots: remain absent, and the type-checking stage
reports E1010. For runtime safety, fill 0 (the normal path never reaches this).”

**Current state of named arguments (after narrowing in 4.9.9)**: before reordering, named arguments
are bound according to the **declaration-time formal-parameter name table** using the shared
criterion `arg_binding::bind_args_to_params`. The IR-side name table is obtained through
`lookup_param_info`, whose sources are this unit’s `fn_param_names`, cross-unit method candidate
groups (`method_param_info`, queried by the qualified name in `method_dispatch_name`), and the
module export surface. Cross-unit method arms without overload resolution do not carry named
arguments (the old behavior silently discarded them; this known gap is recorded in 09’s 4.9.9 →
4.9.10). The “fill 0” behavior in the table above remains a runtime safety net for positional
arguments and fallback arms and is unreachable on the normal path.

**This design has two risks**:

1. **Misalignment does not report an error; it produces a wrong value.** Each of the six branches
   independently determines the final order of `arg_regs`, and any incorrect ordering assumption
   will not panic.
2. **Normalized snapshots are immune to this.** 07 already records this limitation: after the
   snapshot tool renames temporary values by “first-occurrence order,” “the third argument should
   have used the fifth register” is identical in the snapshot. **This class of defect is not even
   covered by the snapshot criterion in the C4 stage.**

**Additional verification: the struct-construction path’s reordering has no duplicate-slot guard.**
`final_args` at `7724` is populated by “positional arguments first, then named arguments overwrite
by field name.” **There is no duplicate-slot check such as `slots[idx].is_some()` before the
overwrite** (unlike the namespace path at `7381`). When a named argument silently replaces a
positional argument, there is no panic or diagnostic. Furthermore, the argument **evaluation order**
(source order) diverges from the final `arg_regs` order (field-declaration order). The
`fill_missing` assertion in batch d must also cover the “duplicate slot” check; otherwise, silent
replacement in the struct-construction path will survive unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified fact (as of the audit).** The repository has **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                    | Production                                                                                                          | Transmission                                                                          | Consumption                                                                                                                    |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                      | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (statement identity dispatch via `walk_stmt`)  | `typecheck/types.rs:38` ← passed by `checker.rs` ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, the only entry point; settlement-complete marker `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger combined, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` only forwards and merges) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held by the `ir_gen.rs` field   | `ir_gen.rs:7697` (with a binding), `7772` (without a binding) (`OverloadResolutions::get`; settlement-complete marker `:1097`) |

The form at the time of the audit (`ir_gen.rs:1942`, **now nonexistent**) was:

```rust
// NLL Release: insert a Drop instruction at the statement boundary
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

The `if let Some` means that **a miss is simply a miss: there is no else branch, no assertion, and
no counter**. If the two sides compute spans in any different way → **the Drop instruction silently
disappears, with zero errors**. The Drop sequence for refinement types is the core carrier of this
project’s ownership semantics (`ownership.rs:26-32` describes the “NLL precise release plan”), and
its realization was once silent.

The same form appears at `7420` / `7493`
(`if let Some(mangled) = self.overload_resolutions.get(span)`), with the consequence that overload
resolution falls back to the default binding.

**Disposition (landed in 4.5)**: both keys were changed to the structural identity `PlanId`, and
consumption is **recorded on every hit** (the read and accounting occur in the same call). A
nonempty difference is reported as an **E-level `W1090`** at the sole settlement point—a “not found”
condition is no longer silent. The landed form and criteria are described in the §2 revision note in
[02](02-stage-contract.md) (4.5). The line numbers here (`1942` / `7420` / `7493`) are retained as
evidence from the audit and are not current pointers.

> **`method_def_ordinals` is not span-keyed (**eliminated in 7c.3**).** It was originally a
> `HashMap<String, usize>` that assigned `#N` suffixes according to the definition traversal order
> of `base_name`. Its risk was **different**: definition-order dependency—if generation order
> differs from typechecking registration order, the same function name resolves to a different
> target. The landed form is described in “Mechanism Three: Additional Mechanisms for Span-Keyed
> Problems”: the definition name is produced once by typechecking from declaration identity, and
> lowering only consumes it.

### Defect Class 4: Implicit spawn/lambda Handshake (Eliminated in 7c.1)

**Eliminated.** spawn and spawn-for previously required the caller to write `pending_env_vars` /
`pending_env_names` before a synthesized Lambda could consume them; closure bodies separately used
`closure_captures` to determine whether a variable read an upvalue. Three fields placed correctness
at the mercy of call order and manual cleanup: a nested synthesized closure could mistake an outer
local slot with the same name for a captured value, and an early return from closure-body generation
could leak capture state into subsequent unrelated lowering.

7c.1 consolidated synthesized AST into `middle/lower/synth.rs`, replaced the three shared fields
with an explicit `SynthEnv`, and registered named captures as lexical upvalues. Nested frames
restore by symbol-table depth, and error paths no longer leave capture state behind. See the
“`synth.rs` Boundary” section for the landed structure.

### What SSA Does Not Resolve (An Honest List)

| Problem                                                            | Location                                                                                                                              | Why SSA Does Not Resolve It                                                                                                                                                                                                               |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                                       | `ir_gen.rs:5390-5687`, **298 lines**, recursively calls itself 4 times (`5473` / `5504` / `5619` / `5640`), external call site `5270` | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **intrinsic to the algorithm**. SSA only turns guards that produce `JmpIfNot` into `Phi`; it does not reduce the number of guards |
| `eval_const_expr` complexity                                       | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)               | Constant-folding expression coverage is unrelated to SSA                                                                                                                                                                                  |
| Span-keyed failure (**eliminated**)                                | At audit time: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)—after 4.5, `PlanId` keys + mandatory settlement      | **SSA does not touch this layer at all** (the conclusion remains unchanged). It was independently resolved by the obligation-ledger mechanism in [02](02-stage-contract.md); see Mechanism Two in “Handling Span-Keyed Problems”          |
| Number of `Instruction` variants                                   | 75 (the `Instruction` enum in `ir.rs`; includes `Phi` introduced in 7b.1a)                                                            | SSA adds `Phi`, for a **net change of +1** (already landed). Variants map to bytecode opcodes or degrade to NOP and will not decrease because of SSA; under Option A, `Phi` is an IR-only pseudo-instruction and adds no opcode           |
| `method_def_ordinals` definition-order dependency (**eliminated**) | At audit time: `ir_gen.rs:224` / `369` / `1692-1702`; landed in 7c.3; see Mechanism Three in “Handling Span-Keyed Problems”           | Similar to span keying but with a different cause; the definition name is now produced by typechecking from declaration identity, and lowering only reads it                                                                              |

## Target Design

### Definition of the SSA Form

**Design judgment.** The target form is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has five variants in total. A **repository-wide measurement** (hit
counts summed by file using `git grep -c`, with all of `src/`, including tests; construction-point
counts are value-position constructions) gives:

| Variant             | Repository-wide measurement                                                                                                                                                                  | Disposition                                                                                        |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                          | **Retained**, unchanged                                                                            |
| `Value(ValueId)`    | 7 files / 26 hits; introduced in 7b.1a with **0 production construction points**, and from 7b.2 onward `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`)      | **Retained** (introduced in 7b.1a)                                                                 |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                          | **Retained but with split semantics** (see below)                                                  |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                            | **Retained**, as the implicit definition at function entry (parameters are definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                           | **Retained**, unchanged                                                                            |
| `Temp(usize)`       | **0 construction points** (the handling arm only exists in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, and is removed with the variant)                             | **Removed** (dead variant, 7b.1a)                                                                  |
| `Label(usize)`      | **0 construction points**                                                                                                                                                                    | **Removed** (dead variant, 7b.1a)                                                                  |
| `Register(u8)`      | **0 construction points** (the original variant declaration was commented “Added for codegen,” but codegen uses `OperandResolver` in `codegen/operand.rs` and never constructs this variant) | **Removed** (dead variant, 7b.1a)                                                                  |

`Local(usize)` is split into two variants because it currently carries **two mutually exclusive
semantics** simultaneously—“the storage location of a named variable” (which may be written multiple
times) and “a compiler temporary register” (which should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within the function, with a single definition
    Value(ValueId),
    /// Storage slot for a named local variable: addressable and allowed to be defined multiple times (outside the SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **increasing monotonically and never rolled back**. The split
and `ValueId` were landed in 7b.1a (the `Operand` enum in `src/middle/ir/ir.rs`); the production
side (`next_temp_reg` now producing `Operand::Value`) was landed in 7b.2 (see “Production Form”
under “Value Identification”).

#### `Phi` as a New `Instruction` Variant

```rust
/// Join point: each predecessor basic block provides one definition of this value
/// incoming is sorted in ascending order of predecessor block labels (guaranteed at construction time to simplify normalization)
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
3. All `incoming` values have the same `MonoType` (**dependent on 03 having unified the type
   representations**).

The landed state of `verify_ssa` (the first three constraints were completed in 7b.1b): Constraints
1 and 2 require the value table and block-level CFG, and landed in **7b.1b** as two criteria:
“block-level predecessor count” and “predecessor label set.” The predecessor set is derived from
`BasicBlock.successors` (`block_predecessors` in `src/middle/ir/verify.rs`); a count mismatch
reports only the count, while a set mismatch reports the set, so the same defect is not reported
twice. Constraint 3, per-input type consistency, landed at the same time, obtaining values from the
value table at `values[v].ty` and **reusing the existing `types_compatible`** (no third
type-compatibility check was introduced; unification of type representations in 03 remains
“Precondition Two (hard),” at which point `ValueInfo::ty` will still be `MonoType`). All three
criteria use `InvariantKind::PhiConsistency` as their violation category.

**Three forms in which the type is unavailable (explicitly recorded, never silently accepted)**: an
empty value table means that no basis for judgment exists (the constant form used by module init /
top-level statements, the `value_mode=false` path); a nonempty value table with an out-of-range
`ValueId` is a violation; and a non-`Value` operand (`Const` / `Local` / `Arg` / `Global`) has no
value-table type source and is therefore not judged.

**Measured precondition (fulfilled in 7b.3)**: before 7b.3, all repository-wide production
constructions of `BasicBlock.successors` used `Vec::new()` (block-level predecessors were always
empty, and the predecessor-count and predecessor-set criteria were then carried by synthetic
fixtures). **From 7b.3 onward, block-level `successors` are genuinely populated by
`partition_blocks` in `src/middle/ir/ir.rs` at every production construction point** (all 6 lower
sites + 1 synthesized function body in codegen use this single entry point; lower-produced loop /
branch blocks have multiple blocks, multiple outgoing edges, and back edges). The predecessor-count
and predecessor-set criteria therefore have data from real IR, and a direct-drive test on real
lowering output pins down the registration-consumption closed loop. The production construction
points for `Phi` remain at 0 (production belongs to a later batch), and when implemented they will
be directly subject to these criteria.

7b.1a introduced only the variant: lowering has 0 production construction points (still 0 today;
production belongs to a later batch). The consumption side in `translator.rs` landed in **7b.3**:
under Option A, `Phi` is expanded at block entry into a sequence of `Move` instructions (the group
is placed at the join block’s entry, existing `JMP`s between groups skip over it, and incoming-edge
jumps are redirected according to the predecessor to the group’s start). The `Phi` arm in
`translate_instruction` is an unreachable E8001 defensive arm (`executor` never sees `Phi`).

**Disposition of the guard problem at `3632-3638`**: the reclamation in the current
`generate_block_ir` has an `if result_reg.is_none()` guard (the comment at `3626-3631` states:
blocks that are expression operand slots may have sibling argument temporaries that remain live
across blocks). After SSA transformation, this guard is **no longer needed**—because temporary
values are no longer rolled back, sibling arguments that remain live across blocks hold their own
`ValueId` and are unaffected by rollback. **This is a place where SSA transformation genuinely
simplifies the code rather than merely renaming things.**

#### Value Identification

**Design judgment**: do not perform mem2reg, do not use virtual-register numbers, and **perform only
explicit renaming**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots are eligible for memory
  promotion” a **type system problem**, not a CFG problem. It has been verified that
  `LocalSlot::ty: MonoType` in `ir.rs` carries complete type information, but determining “whether
  this `MonoType` is promotable to memory” requires the unified type representation from 03 and
  coordination with refinement-type rules. **Performing mem2reg before 03 is guessing.**
- Explicit renaming only requires changing the return-value semantics of `next_temp_reg()`
  (`next_temp_reg` in `src/middle/lower/scope.rs`) and deleting the three rollbacks. **It does not
  touch the control flow of any lowering algorithm.**

Value-table form (**landed in 7b.1b**): `FunctionBody::Code` gains `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // Label of the basic block in which it is defined
}
```

The accessors have the same shape as existing accessors (`impl FunctionIR` in
`src/middle/ir/ir.rs`): `values()` is read-only and returns an empty table for TypeDecl (mirroring
`locals()`); `values_mut()` is mutable and panics on TypeDecl (mirroring `blocks_mut()`).

**Production form (landed in 7b.2)**: `next_temp_reg()` returns `Operand::Value(ValueId)` and
records one `ValueInfo` in the current function body’s value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). The landed form is as follows:

- **Counter isolation**: `ValueId` is issued by an independent counter, `next_value: u32`. It starts
  at 0 for each function body, increases monotonically, and is never rolled back; `NestedBodyGuard`
  saves and restores it together with the value table (one set per function body). The three
  statement-level reclamations roll back only `next_temp` (slots) and do not touch `next_value`.
  Curry layers reset it layer by layer (one `FunctionIR` per layer). Module init / top-level
  statements have no value table—when `value_mode=false`, `next_temp_reg` degrades to the old
  `Local` slot allocation, leaving the init sequence unchanged.
- **Source of `ty`** (only existing lowering information is used; no new type classifier is
  created): literals use `ConstValue::mono_type()` (converged with the verifier’s constant-type
  mapping into a single source); variables/calls use the type-checking result and binding function
  return type at the `generate_expr_ir` entry point;
  comparison/logical/string-family/forward-propagation forms are filled in according to instruction
  semantics when the function body is finalized. When unavailable, an **explicit unknown
  `MonoType::TypeRef("?")`** is used (not a silent `Int(64)` placeholder; `LocalSlot.ty` is never
  read). Across 232 snapshots, 9,425 values were measured: 7,080 had real types and 2,345 had
  explicit unknown types (24.9%), concentrated in call/field/index/container-construction forms.
- **`defining_block`**: takes the current block label at generation time; **block partitioning
  landed in 7b.3** (a real lowering `while` body consists of multiple blocks), but the field’s value
  is still the generation-time `cur_block_label` (currently always 0), so it does not yet agree with
  the actual defining block. The 232 snapshots regenerated in 7b.4 did not change this field (the
  `@block` section under `values` did not drift). Its correction must occur in the same batch as
  regeneration of the `values` section and is **explicitly deferred to the `Phi` production batch**.
- **`debug_name`**: always `None` (named variables remain `Local` and do not migrate to the value
  table).
- **Indexing**: `value_slots: Vec<usize>` (index = `ValueId`) is parallel to `values` and records
  the value → frame-slot mapping; from 7b.4 onward, it is recomputed by the linear-scan allocator in
  `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`). Codegen’s `OperandResolver` only consumes it and does not
  allocate.

`values` and `locals` coexist: `locals` carries addressable named variables (which may be defined
multiple times), while `values` carries single-definition SSA values. **The existence of `locals`
does not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Normalization with `LocalSlot::origin`**: the two are **different dimensions** and must not be
conflated. `LocalSlot::origin` (the **slot-identity** field landed in 2.1.4 on the origin side,
recording the generating `FrameId` of the slot) answers “which frame does this slot belong to,”
whereas `ValueInfo.defining_block` answers “in which basic block is this SSA value defined.” Value
identity belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`; 7b.1b only adds
the value table and **does not change `LocalSlot::origin`** (measured: zero changes).

#### Change-Volume Estimate

| Item                                                         | Estimate                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------ | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` modification                                 | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Deletion of three statement-level reclamations               | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Consolidation of six save/restore sites into RAII            | Net ±0 lines                          | See “Item-by-Item Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `ir.rs` structural changes                                   | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **the first two landed in 7b.1a and the last two in 7b.1b** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Add `Phi` arm to `translator.rs`                             | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `executor/`                                                  | **Zero changes**                      | Option A: `Phi` is an IR-only pseudo-instruction expanded into a sequence of `Move`s in `translator.rs` and does not enter bytecode (Decision Three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Call sites that produce temporary values as `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (`git grep -c` summed by file, all of `src/`, including tests): `Operand::Local(` has 415 occurrences in 28 files, including 332 in 14 files under `src/middle/lower/`; the single production point for temporary values is `next_temp_reg`. **Measured in 7b.2 (baseline-revision retest and landed net amount)**: 140 call sites of `next_temp_reg()` in 14 files, a `result_reg: usize` parameter chain spanning 35 functions + 3 `Option<usize>` result slots, and 342 occurrences of `Operand::Local(` under `src/middle/lower/`; net source diff for `src/` totals +23,319/−12,495, of which regeneration of 232 IR snapshots accounts for +21,866/−11,950, leaving +1,453/−545 for source and tests (mechanical replacements and normalize rendering, mono projection, and six direct-drive tests; this also includes five diagnostic-code artifacts committed alongside the source) |

### Item-by-Item Disposition Table

| Defect class                                        | Does SSA eliminate it?     | Disposition                                                                                                                                                                                                                                                                                                                                                                                                          | Criterion                                                                                                                 |
| --------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore** | **Yes (resolved in 7b.4)** | The seven nested-function-body entry points are consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the counters), turning “forgot to restore” into a compile error. The three statement-level reclamations were removed with 7b.4 (the allocator reuses slots by live interval), and temporary values no longer occupy `next_temp` | First-layer “inner-scope isolation” invariant in 07 + `verify_loose`/`verify_ssa`                                         |
| **1b. Inconsistent conventions**                    | **Yes (landed)**           | Unified under `finalize_code_body` in `src/middle/lower/scope.rs`: table extraction uses `frame_slots`; the E3014 budget check and table extraction occur at the same point and include the function-body span; all six function-body shapes are covered; `MAX_REGISTERS` has one repository-wide definition                                                                                                         | Corpus differential testing + zero snapshot drift                                                                         |
| **1c. Three statement-level reclamations**          | **Yes (landed in 7b.4)**   | The three reclamations, the `result_reg.is_none()` guard in `generate_block_ir`, and the `temp_floor` bookkeeping were removed; slot reuse is assigned to the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                                        | C4: behavioral equivalence (403/403) + invariants                                                                         |
| **2. `arg_regs` semantic reordering**               | **Partially**              | SSA turns “using the wrong register” into “using the wrong value ID,” which dominance checking can catch, but **the reordering logic itself remains**. Actual elimination depends on decomposing the function—each of the six branches performs its own `push`, while ownership of `arg_regs` remains at the top of the function                                                                                     | First-layer dominance invariant (snapshots are immune)                                                                    |
| **3. Span-keyed failure**                           | **No**                     | SSA does not touch it. See “Handling Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                                            | `test_release_plan_spans_consumed` in 07                                                                                  |
| **4. Implicit order dependency**                    | **No**                     | See the “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                                        | Guaranteed by code structure and review; the original gate script was cancelled by the user’s decision D dated 2026-11-24 |
| `compile_pattern`, 298 lines                        | No                         | See “What SSA Does Not Resolve”                                                                                                                                                                                                                                                                                                                                                                                      | —                                                                                                                         |
| `eval_const_expr`, 191 lines                        | No                         | See “What SSA Does Not Resolve”                                                                                                                                                                                                                                                                                                                                                                                      | —                                                                                                                         |
| 75 `Instruction` variants                           | No (+1, landed)            | `Phi` was introduced in 7b.1a; under Option A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                                 | —                                                                                                                         |

### Register Allocator: In-House or Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **0 construction points repository-wide**—the variant is no longer in
   the `Operand` enum in `ir.rs` (removed in 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper bound of 255 (exceeding it
   reports E3014); from 7b.4, `Operand::Value` is resolved by the allocator’s `value_slots` output
   to a frame slot (`>255` reports E3014; no mapping reports E3017 with the instruction span).
3. `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50 `translate_*`
   functions distributed across `101-1558`) outputs `BytecodeInstruction::new(opcode, operands)`.
   Opcodes come from `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` contains the interpreter’s opcode-dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Evaluation of three options**:

| Option                                               | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `regalloc` / `regalloc2` (from the rustc lineage)    | **Not adopted.** Both take rustc’s `Function` trait and MIR’s `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait set (CFG, liveness, spill slots, scratch space): **500+ lines of adapter code for zero benefit**, because there is no real register file to color. Rustc itself also uses only linear scanning at this point (when `enable-llvm` is disabled)                                                                                                                 |
| `graph-alloc` family                                 | **Not adopted.** Same reasons, with a less stable API                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| **In-house linear scan (simplified Chaitin-Briggs)** | **Adopted (landed in 7b.4).** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot according to its live interval (`OperandResolver` already has a `u8` contract and the E3014 limit). Implement `src/middle/lower/regalloc.rs`: block-level `successors` backward dataflow, closed intervals, Phi predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`; an out-of-bounds result returns `Budget`, which `finalize_code_body` reports as E3014 with the declaration span |

**Measured results (7b.4)**: the two corpus cases that previously exceeded the limit (354 / 323
monotonic slots) now use **15 / 24** frame slots (with 344 / 304 values) after allocation; the other
403/403 corpus cases remain behaviorally equivalent. From that point onward, E3014 means only “frame
slots after allocation exceed 255.” The counter used for the back-edge fallback is currently not
exposed (`wrap_count` is computed and discarded), and the `Phi` production batch must expose and
retest it first (a prerequisite alongside `defining_block`). **Overflow handling**: `u8`/255 is
still treated as an error by the budget gate—this article does not solve “a large function has
insufficient registers,” which is a downstream consequence of `compile_pattern` recursion depth.

The dev-dependencies at `Cargo.toml:37-39` are only `criterion` / `proptest`; `[dependencies]`
contains no regalloc / petgraph / graphlib. Introducing any crate would add a new dependency
surface.

### Decomposition Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA form and criteria must exist first**. The reason is that
the decomposition itself is C1 (pure code motion, judged by zero-diff snapshots), but only after the
semantics of `arg_regs` have been eliminated by SSA—until each argument has an independent
`ValueId`—will the decomposition avoid carrying the old ordering assumptions into the new file
unchanged.

**Core design constraint: ownership of `arg_regs` remains at the top of the function, and the six
branches only `push`; they never reorder.**

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

The “fill 0” logic in `7835-7848` (`7838-7846`) is pushed down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the comment
claiming that “the normal path never reaches this” is replaced with an assertion**: if any slot is
missing and typechecking did not report E1010, return a diagnostic directly. **This turns a silent
fallback into a hard error and is this article’s most direct improvement to Defect Class 2.**

**Criteria**: the pure code-motion portion (without an SSA-form change) uses zero-diff snapshots;
making `fill_missing` assertive is C4 and uses corpus differential testing plus the first-layer
verifier.

### The `synth.rs` Boundary: Turning an Implicit Callback into an Explicit Synthesized-AST Boundary

**Landed form**: `src/middle/lower/synth.rs` is the only module in L3/L4 production code permitted
to construct `ast::Expr` / `ast::Stmt` / `ast::Type`. Pattern matching, read-only queries, and test
fixtures are not synthesis. The current production synthesis sites are spawn’s Lambda/Stmt and
spawn-for’s Lambda, all of which use this module.

AST nodes do not carry a lowering environment, so `SynthEnv` is not an AST field and is not
pretended to be consumed by AST helpers. It contains runtime environment operands and capture names,
and callers explicitly pass it with the synthesized node to `generate_expr_ir_with_env`:

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

- spawn and spawn-for construct `SynthEnv` and explicitly pass it into lowering with the synthesized
  Lambda; they no longer prewrite `pending_env_vars` / `pending_env_names`. The total
  environment-slot count is derived from `env_vars.len()`, and named upvalues begin after the
  anonymous-prefix length; spawn-for’s anonymous iteration value, outer captures, and parameters
  therefore each occupy their proper positions.
- Ordinary lambdas complete the environment from the existing free-variable collector. List
  comprehensions, nested Lambdas, and nested spawns are traversed within the same collector, with
  generator variables and nested parameters excluded according to lexical scope.
- Closure bodies register `SynthEnv` as a `BindingLocation::Upvalue` in the existing symbol table.
  Variable reads and namespace-shadowing queries share this lexical binding and no longer read
  `closure_captures`.
- Nested closures use `capture_operand` to read the outer upvalue first, then pass the value into
  the inner environment; an outer local slot with the same name is never mistaken for a captured
  value.
- `NestedBodyGuard` records the symbol-table depth on entry and truncates to that depth on exit
  (including early return through `?`), so capture bindings do not leak into subsequent unrelated
  lowering.
- Sema’s expression-body function check parameterizes the existing inputs with
  `body_stmts + expression_body + span`, calls `check_expr` directly, and reuses the tail
  validation, without synthesizing a Block/Stmt/Expr; there is no sema→middle/lower reverse
  dependency or additional architectural exception.
- SpawnFromList uses the `env + [anonymous item]` argument layout in both Embedded and Standard and
  injects upvalues from the same environment. The anonymous iteration value is fixed at env[0],
  named captures are read after the anonymous prefix, and parameters begin after the entire
  environment.
- Dual-runtime behavior tests explicitly invoke `main()` and, within a spawn-for task, separately
  assert the no-capture item value and the item-plus-outer-capture value; the no-capture form also
  covers the regression restoration introduced by 74febd1f. IR assertions and actual execution are
  recorded separately, and zero drift across the 403 existing corpus cases does not masquerade as
  behavioral coverage of this form.

**Criterion**: the AST synthesis boundary from D22 is guaranteed by code structure—L3/L4 production
code constructs `ast::Expr` / `ast::Stmt` / `ast::Type` only in `src/middle/lower/synth.rs`; pattern
matching, read-only queries, and test fixtures are not synthesis; synthesis sites outside `synth.rs`
are rejected during review. The original `scripts/ci/check-synth-boundary.py` and the CI
`synth-boundary` job were removed under the user’s decision D dated 2026-11-24 and are no longer
maintained as an automatic scanning gate: heuristic token scanning cannot cover arbitrary legal use
/ alias / re-export forms (even after successive gap-filling attempts, `pub type` re-exports
remained a bypass), and the maintenance cost exceeds the benefit.

D1/D2: `SynthEnv` and `BindingLocation` are both new internal types introduced in this batch, not
reused existing types. The former centralizes the production-side environment operands, named items,
and slot derivation into one explicit parameter; the latter promotes the original
`SymbolEntry.local_idx` into a discriminated Local/Upvalue location. The alternatives are to
continue passing three shared fields (which cannot prevent nested overwrites or erroneous residue)
or to pass only `Vec<Operand>` (which cannot express the correspondence between the anonymous prefix
and named captures). Both are rejected for correctness reasons. The new concepts are not synonymous
with existing AST or consumption-ledger concepts and require neither import aliases nor conversion
tables.

### Handling Span-Keyed Problems (the Part SSA Cannot Resolve)

**Design judgment**: three independent mechanisms, ordered by cost:

**Mechanism One: Consumption-Coverage Assertion (Mandatory and Lowest-Cost; Superseded by a Stronger
Form on 2026-10-13).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). This
article’s original supplement proposed implementing it as a **runtime counter** rather than only a
test. The landed form is stronger than that proposal—a **consumption ledger
(`ConsumptionLedger<K>`) + a single settlement point (E-level `W1090`)**—without introducing a new
diagnostic code or relying on an internal ir_gen counter to prove itself:

```rust
// Around ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On every hit at :1942, self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* New diagnostic code: NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a value that should have been dropped was not dropped” may not immediately
crash under YaoXiang’s current value semantics + Arc/Rc model**—which is precisely why it was
silent. The counter turns “silent” into “detectable.”

**Mechanism Two: Change Span Keys to `DefId` or Explicit Plan IDs (**landed on 2026-10-13**):
RFC-039 D20 / WBS 4.5 implemented this mechanism using a `PlanId(u32)` structural-identity key +
mandatory consumption settlement. The landed form is described in the §2 revision note in
[02](02-stage-contract.md).** The fundamental fix is to **stop using Span as a key**. `Span` is a
source location and may drift because of macro expansion, `include`, and multi-file merging.
`FunctionIR` already has a `def: Option<DefId>` field (`ir.rs:677`), demonstrating that DefId is
available at the IR layer. **Design judgment**: introduce `PlanId(u32)` for `ReleasePlan` and
`overload_resolutions`, allocated by ownership / overload resolution when producing the plan and
matched by `PlanId` during IR consumption. **This belongs to 02’s obligation ledger (`Obligations`)
and should be implemented with [02](02-stage-contract.md), not in this article.**

**Mechanism Three: Definition-Order Dependency in `method_def_ordinals` (Independent Small Item,
**landed on 2026-11-25**).** The field was originally a `HashMap<String, usize>`, with an
unqualified `"{type_name}.{method_name}"` key, assigning `#N` mangled names according to definition
traversal order. If typechecking registration order differed from lowering definition order, methods
with the same name could resolve to different targets. **Landed form**: typechecking **produces
once**, at the same point where it registers overload candidates, a
`method_def_ir_names: HashMap<PlanId, String>` (bare names or `#N` mangled names), which is passed
to IR generation through `TypeCheckResult`. The key is the **method declaration statement’s
`PlanId`** (an AST structural identity, the same model as D20)—`DefId` is not assigned until
`assign_defs` at the end of lowering and therefore does not exist on the typechecking side. Lowering
only reads the name by declaration identity, and **a miss fails loudly with E3005** (it does not
fall back to the bare name—falling back would disguise “the definition name and call name do not
come from the same source” as a runnable program). The mutable counter and its write site were
removed, and the definition name no longer has a second derivation entry point.

### Tracking Hardcoded Discards

The original `impl From<BytecodeFile> for BytecodeModule` (now `src/middle/bytecode/mod.rs`) had
three hardcoded discards. After the 7e review, there were **two genuine losses + one false
positive**:

| Location (old line number) | Field                | Original value                                 | Review conclusion                                                                                                                                                                                                                                                                                                                                                                                |
| -------------------------- | -------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `bytecode.rs:2312`         | `upvalue_count`      | `0` // Not stored in BytecodeFile              | **Genuine loss**: closure upvalue-slot count becomes zero after a `.42` round trip; persisted with v5 (7e.1)                                                                                                                                                                                                                                                                                     |
| `bytecode.rs:2315`         | `exception_handlers` | `Vec::new()` // Not implemented yet            | **False positive**: the field has no producer or consumer, and the language layer has no throw/try/catch (RFC-030 records `Throw` as a bytecode-layer reservation, and RFC-036 rejects the in-process catch boundary) ⇒ there is no “lost data”; it is a dead field. It is not persisted, and removal belongs to P10 S6’s opcode dead-path cleanup (`THROW` remains a separate pending decision) |
| `bytecode.rs:2341`         | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **Genuine loss**: the global table (names + types) becomes zero after a round trip; persisted with v5 (7e.3), with the carrier and function signature both using `MonoType` (7e.4)                                                                                                                                                                                                               |

**Criterion (Decision D52)**: an empty table with no producer is not lost data; it is a dead
path—the same criterion as D32. Only data with a producer whose container is not persisted
constitutes data loss. Therefore, `2312` / `2341` are included in P7 batch e (7e), while `2315` is
not persisted and is assigned to P10 S6’s opcode dead-path cleanup (`THROW` is listed separately for
decision). One version bump covers both genuine losses (`VERSION` 4→5, D17). `.42` is a build
artifact, and cross-version compatibility is not a goal.

**The third format distortion in the same 7e batch**: the `.42` type-number table was irreversible
and had collisions—`Int(64)` and `Float(32)` both mapped to `9`, and `Float(64)` was read back as
`TypeRef("_")`. The number table was changed to be reversible and collision-free (7e.4), and the
write side now rejects unknown bit widths and compile-time FFI types loudly.

## Detailed Design

### Knock-On Effects of IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                                                     |
| ------------------------------------------------------------------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add the `ValueId(u32)` type                                               | File header (`ValueId` definition)            | New (**landed in 7b.1a**)                                                                                  |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**landed in 7b.1a**)                                                       |
| Add the `Phi` variant to `Instruction`                                    | End of the `Instruction` enum (`Phi` variant) | **+1 variant** (**landed in 7b.1a**; the enum now has 75 variants)                                         |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | +1 field (**landed in 7b.1b**)                                                                             |
| Add the `ValueInfo` structure                                             | `ValueInfo`                                   | New (**landed in 7b.1b**)                                                                                  |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                    | Add `values()` / `values_mut()` accessors (**landed in 7b.1b**, with the same shape as existing accessors) |

The `pub use ...ast::Type` at the top of the file is **not changed by this article**—it belongs to
the scope of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **unified** single
type.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                        | Location                              | Nature                                                                                                             |
| ----------------------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Add `Phi` to `BytecodeInstr`, or use the “expand to Move at runtime” strategy | `127-554` (currently **66** variants) | +1 or +0 variants (see below)                                                                                      |
| Add a `Phi` arm to `opcode()`                                                 | `558`                                 | Mandatory (`BytecodeInstr` has an exhaustive match)                                                                |
| Add a `Phi` arm to `size()`                                                   | `649`                                 | Mandatory                                                                                                          |
| Add a `Phi` arm to the `From<BytecodeFile>` decode match                      | `943-2350`                            | Mandatory                                                                                                          |
| Correct `upvalue_count: 0`                                                    | `2312`                                | Fixed as a side change in this article (in-memory format; persistence requires a version bump, as described below) |

**The bytecode form of `Phi` requires a design decision**. There are two options (**design judgment:
Option A is recommended**, for the reasons in “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  selected by predecessor-block index. **No new opcode is added**, and the `.42` format remains
  unchanged. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode + predecessor-index encoding, with the interpreter maintaining register
  versions on jumps. **This requires changing the `executor/` state model + the `.42` format + the
  version number.** The only benefit is size.

The decisive reason for choosing A is that a `.42` format change triggers version-number issues and
a full compatibility discussion, **while A’s size cost is irrelevant in the interpreted-execution
scenario** (the executed instructions are Moves, not Phis).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions distributed across `101-1558`)

| Change                                             | Location                                                                                                                         | Nature                                                                                  |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                       | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add an arm                                                                              |
| Translation arm for `Instruction::Phi`             | Within the dispatch match, before `Free` (`533`)                                                                                 | New branch that expands into a sequence of Moves (Option A)                             |
| Existing nine NOP degradations **remain in place** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as nine**: `655-657` covers two variants, and `658-660` covers four variants |

**SSA transformation does not reduce the number of NOPs.** These nine degradations reflect
expressiveness gaps where “the IR has it, but bytecode does not”; SSA transformation does not touch
them.

#### `src/backends/interpreter/executor/`

With Option A, `executor/` requires **no changes whatsoever**—`Phi` disappears at the
`translator.rs` stage and is never visible to the interpreter. **This is the second decisive reason
for choosing A.**

`BytecodeInstr::Switch` has an active implementation at `executor/ops/control.rs:90`, but **none**
of the `Switch` / `BrTable` / `JumpTable` variants exists in `ir.rs` (all three names have zero hits
in `ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions that the IR layer cannot
produce. It does not affect this article, but is recorded here because it shows that “IR → bytecode”
is not surjective.

The dispatch match at `executor/debug.rs:194` likewise requires no changes (under Option A, there is
no new opcode).

### Compiler Change List

**File by file and function by function.** All line numbers are the **starting points** of changes
(verified by measurement):

| No. | File                                                                      | Location                                                                       | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                                                     | `Operand` enum definition                                                      | Add/remove `Operand` variants (**7b.1a landed**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 2   | `src/middle/ir/ir.rs`                                                     | End of the `Instruction` enum (`Phi` variant)                                  | Add `Instruction::Phi` (**7b.1a landed**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 3   | `src/middle/ir/ir.rs`                                                     | `FunctionBody::Code`                                                           | Add `values` (**7b.1b landed**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 4   | `src/middle/ir/ir.rs`                                                     | `ValueInfo`                                                                    | Add `ValueInfo` (**7b.1b landed**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 5   | `src/middle/ir/ir.rs`                                                     | `values()` / `values_mut()` accessors                                          | Add accessors (**7b.1b landed**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 6   | `src/middle/ir/ir.rs`                                                     | `pub use ast::Type`                                                            | **Leave `pub use ast::Type` unchanged** (scope of [03](03-type-unification.md); recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 7   | `src/middle/core/ir_gen.rs`                                               | `884-891` `next_temp_reg`                                                      | Return `Operand::Value`, increasing monotonically without rollback (**7b.2 landed**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; add an independent `next_value` counter starting at 0 for each function body; the three statement-level reclamations roll back only `next_temp` and do not touch it)                                                                                                                                                                                                       |
| 8   | `src/middle/core/ir_gen.rs`                                               | `1954-1959`                                                                    | **Delete** the statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 9   | `src/middle/core/ir_gen.rs`                                               | `3632-3638`                                                                    | **Delete** the statement-level reclamation (including the `result_reg.is_none()` guard)                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 10  | `src/middle/core/ir_gen.rs`                                               | `4737-4742`                                                                    | **Delete** the statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 11  | `src/middle/core/ir_gen.rs`                                               | `1745-1759` / `1819-1822`                                                      | Replace with an RAII guard (`generate_method_ir`, entry `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 12  | `src/middle/core/ir_gen.rs`                                               | `1858-1866` / `2026-2029`                                                      | Replace with an RAII guard (`generate_function_ir`, entry `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 13  | `src/middle/core/ir_gen.rs`                                               | `2116` / `2180`                                                                | Replace with an RAII guard (`generate_curry_innermost_func`, entry `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 14  | `src/middle/core/ir_gen.rs`                                               | `2224-2229` / `2284-2288`                                                      | Replace with an RAII guard (`generate_curry_function_ir`, entry `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 15  | `src/middle/core/ir_gen.rs`                                               | `2768-2777` / `2820-2851`                                                      | Replace with an RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 16  | `src/middle/core/ir_gen.rs`                                               | `4686-4695` / `4767-4774`                                                      | Replace with an RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 17  | `src/middle/core/ir_gen.rs`                                               | `1987-2000`                                                                    | Extract the E3014 check into `check_register_budget()`, shared after unifying the conventions at `1798`/`2001`                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 18  | `src/middle/core/ir_gen.rs`                                               | `1798`                                                                         | Change `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`, eliminating the inconsistent convention                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 19  | `src/middle/core/ir_gen.rs`                                               | `248-249`                                                                      | Delete the comment contract (now enforced by the type system) and replace it with an explanation pointing to `verify`                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 20  | `src/middle/lower/loops.rs`                                               | `generate_spawn_for_ir`                                                        | Change to call `synth::synth_closure` (**7c.1 landed**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 21  | `src/middle/lower/expr/spawn.rs`                                          | `generate_spawn_expr_ir`                                                       | Change to call `synth::synth_closure`; delete the shared pending-field writes and clears (**7c.1 landed**)                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 22  | `src/middle/lower/expr/closure.rs`                                        | `generate_lambda_expr_ir`                                                      | Read `SynthEnv` from the arguments instead of `mem::take`ing shared fields (**7c.1 landed**)                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 23  | `src/middle/lower/{mod,scope}.rs`                                         | Symbol table and `NestedBodyGuard`                                             | Delete the `closure_captures` field; use `BindingLocation::Upvalue` lexical binding and depth-based restoration (**7c.1 landed**)                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 24  | `src/middle/lower/expr/{access,closure}.rs` + `query.rs`                  | Variable and namespace queries                                                 | Replace `closure_captures.contains_key` with a lexical-upvalue query; materialize captures uniformly through `capture_operand` (**7c.1 landed**)                                                                                                                                                                                                                                                                                                                                                                                                   |
| 25  | `src/middle/core/ir_gen.rs` (path at audit time)                          | `341` + `1942`                                                                 | ~~Add a span-keyed consumption counter~~ → covered by 4.5’s `PlanId` consumption ledger (read and accounting in the same operation, settlement via W1090); 7c.2 verified zero triggers, with no code changes                                                                                                                                                                                                                                                                                                                                       |
| 26  | `src/middle/lower/{mod,stmt}.rs`, semantic-side `sema/{checker,types}.rs` | `generate_method_ir` definition name + authoritative `TypeCheckResult` mapping | Make `method_def_ordinals` read-only (**7c.3 landed**: typechecking produces the mangled name from declaration `PlanId`, and lowering only reads it)                                                                                                                                                                                                                                                                                                                                                                                               |
| 27  | `src/middle/core/ir_gen.rs`                                               | `7269-8014`                                                                    | **Do this last**: decompose into a top-level `CallArgs` + six `emit_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 28  | `src/middle/core/ir_gen.rs`                                               | `7838-7846`                                                                    | Change the “fill 0” fallback to return a diagnostic                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 29  | `src/middle/lower/synth.rs`                                               | **New**                                                                        | Synthesized-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 30  | `src/middle/ir/verify.rs`                                                 | **New** (first layer of [07](07-equivalence-oracle.md))                        | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 31  | `src/middle/passes/codegen/translator.rs`                                 | Dispatch match                                                                 | Translation of `Instruction::Phi` (**7b.3 landed**: Option A expands at the **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit a group of Moves per predecessor, existing `JMP`s between groups skip over them, and incoming-edge jumps are redirected by `(predecessor, join block’s first instruction)` to the group start; the `Phi` arm in `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return slot cannot hold the multiple Moves distributed across predecessors, so reaching it fails loudly) |
| 32  | `src/middle/passes/codegen/operand.rs`                                    | `36-45` / `72-77`                                                              | Add the `Operand::Value` resolution arm; delete the `Temp` arm (`39-45`) (**7b.2 landed**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); deletion of the `Temp` arm landed earlier in 7b.1a)                                                                                                                                                                                                                                                  |
| 33  | `src/middle/core/bytecode.rs`                                             | `127-554`                                                                      | **Unchanged under Option A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 34  | `src/middle/core/bytecode.rs`                                             | `558` / `649` / `943-2350`                                                     | **Unchanged under Option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 35  | `src/middle/core/bytecode.rs`                                             | `2312`                                                                         | Correct `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 36  | `src/backends/interpreter/executor/**`                                    | —                                                                              | **Zero changes under Option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 37  | `src/middle/lower/regalloc.rs`                                            | **New**                                                                        | Linear-scan allocator (**7b.4 landed**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)` returns `value_slots` and `frame_slots = named_base + max_live`; an out-of-bounds result returns `Budget`, which `finalize_code_body` reports as E3014)                                                                                                                                                                                                                                                                       |
| 38  | `scripts/ci/check-synth-boundary.py`                                      | **Cancelled**                                                                  | The boundary sentinel script was cancelled by the user’s decision D dated 2026-11-24 and is not recreated                                                                                                                                                                                                                                                                                                                                                                                                                                          |

### Backward Compatibility: the `.42` Format Version Number

**Verified fact**: the `.42` persisted format **already has a version-number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (`"YXBC"`)
and `VERSION: u32 = 4`, writes them in the header (`:342-343`), and validates them while reading
(`:454-467`, reporting `"unsupported bytecode version"` on a mismatch).

**Design judgment (a direct consequence of Option A)**:

- `Phi` is expanded into a sequence of `Move`s at the `translator.rs` stage and **does not produce a
  new opcode**.
- `opcode.rs` has **zero changes** across its 83 constants; `opcode_name()` (`120`) is unchanged,
  and the decode match (`bytecode.rs:943-2350`) is unchanged.
- The `.42` field layout has **zero changes**, and `VERSION` remains 4. Old `.42` files can still be
  read by the new binary, and new `.42` files can still be read by the old binary. **SSA
  transformation itself does not require a version bump.**

**The only scenario requiring a version bump is the correction of `2312 upvalue_count`** (Decision
D17: fix it and handle it with the version bump): adding an `upvalue_count` field to
`BytecodeFunction` and having the encoder write it is a **new format field**, raising `VERSION` from
4 to 5. The existing version check on the read side rejects old files according to existing
behavior—`.42` is a build artifact (`main.rs:671`), cross-version read compatibility is not a goal,
and the version bump requires no additional migration mechanism. Already allocated but unused opcode
values (`Switch` / `TailCall` under D32/D34) are reclaimed with the version bump.

## Implementation Notes

### Batch Strategy

Divide the work into four batches. **Every item within a batch is an independent commit with an
independent criterion and independent revert.** “Change item” in the table below refers to a number
in the preceding “Compiler Change List.”

| Batch       | Contents                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Change items                 | Criterion                                                                                                                                                                  | Rollback point                                                                                                                                            |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Break multiple definitions: delete the three statement-level reclamations → replace six save/restore sites with RAII guards → unify conventions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 8-10 (first) → 11-16 → 17-18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and convention unification are pure code motion: **C1**, zero-diff snapshots | One commit per item; `git revert` rolls it back                                                                                                           |
| **Batch b** | Switch to the SSA form: `ir.rs` structural changes + mechanical replacement of temporary-value production with `Operand::Value` → `Phi` arm in `translator.rs` (Option A) → linear-scan allocator. Within the batch, WBS level-three tasks run serially: 7b.1a (`Operand` variant consolidation + introduction of `Instruction::Phi`, **landed**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **landed**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **landed**) → 7b.3 (`Phi` arm in `translator.rs` + population of `BasicBlock.successors`, **landed**) → 7b.4 (linear-scan allocator + deletion of the three statement-level reclamations, in the same batch as 7a.1, **landed**) | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` fully green + `.42` round-trip tests                                                                                         | The `ir.rs` changes **occupy a commit by themselves** (type changes affect compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary → verify the existing consumption ledger (no code changes) → make `method_def_ordinals` read-only (the sentinel script was cancelled by the user’s decision D dated 2026-11-24)                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | 20-24, 29 → 25 → 26          | **C4**: behavioral equivalence; the consumption counter triggers zero times on the new diagnostic (otherwise, a real mismatch exists)                                      | Independent commit                                                                                                                                        |
| **Batch d** | Decompose `generate_call_expr_ir` + make the “fill 0” fallback assertive                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | 27-28                        | Pure code-motion portion: **C1**, zero-diff snapshots. Assertion portion: **C4**, corpus differential testing + first-layer verifier                                       | Independent commit                                                                                                                                        |

**There are three dependencies whose order cannot be exchanged:**

1. Batch a’s “delete reclamation” comes after batch b’s `ir.rs` change (deletion of the three
   statement-level reclamations landed with 7b.4 and the linear-scan allocator, with 7a.1 executed
   together)—first make `next_temp` increase monotonically, so that `ValueId` has the semantic basis
   for “unique definition” (this semantic is established only after 7b.2’s `next_temp_reg` switch to
   `Value` and 7b.3’s `Phi` arm land).
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise, the decomposition would carry
   the old `arg_regs` ordering assumptions into the new file unchanged.
3. Batch d must come last—it depends on the first two batches having converged `arg_regs` semantics
   to “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within **P7** of RFC-039, and the criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial** relative to
one another, but items 8-10 / 11-16 / 17-18 within batch a have only an ordering constraint and no
coupling.

### Hard Preconditions

**Precondition One: Criterion Baseline.** The `verify_loose` mode of the first-layer verifier in
[07](07-equivalence-oracle.md) **must first pass on the existing non-SSA IR** before batch a may
begin.

The reason is that the SSA criterion is C4 (behavioral equivalence + invariants). If the invariant
verifier itself cannot pass on the pre-change IR, every acceptance step in batches a–d lacks an
executable criterion and can rely only on corpus differential testing—which is **precisely
insensitive** to Defect Classes 1/2/3 (as argued in 07’s “Alternative A”). **This is a real ordering
constraint.**

**Precondition Two: Type-Representation Unification.** [03](03-type-unification.md) must complete
the consolidation of `pub use ast::Type` at `ir.rs:3` and removal of the `From<MonoType> for IrType`
bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` would introduce a third
type-compatibility check.**

### Rollback Strategy

- Each change is one independent commit; `git revert` granularity equals change-item granularity.
- Batch b’s `ir.rs` structural change is **the only part that cannot be rolled back incrementally**
  (type changes affect compilation throughout the repository). It must therefore **occupy a commit
  by itself**, and batch a must already be green with a revertible baseline before it.
- **Do not use a feature flag.** The reason is that adding/removing `Operand` variants cannot be
  isolated by a runtime switch; a feature flag would cause CI to stop testing the other path over
  the long term—repeating the “test wiring depends on memory, and corruption occurs silently”
  problem recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front; otherwise, the work will be questioned midway through implementation
because “the line count did not decrease.”**

| Item                                      | Line-count change                                                                                                                                                                      |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                            | `ir_gen.rs` **8,448**                                                                                                                                                                  |
| Delete three statement-level reclamations | **−15**                                                                                                                                                                                |
| Replace six save/restore sites with RAII  | Net ±0 (the RAII guard definition is about +20 lines; use sites decrease from 22 lines to about 12)                                                                                    |
| Modify `next_temp_reg`                    | +5                                                                                                                                                                                     |
| Decompose `generate_call_expr_ir`         | **+40–80** (signature and argument-passing overhead for six `emit_*` functions; this is the typical cost of pure code-motion decomposition—**decomposition increases the line count**) |
| Span-keyed consumption counter            | +15                                                                                                                                                                                    |
| Make `method_def_ordinals` read-only      | +20                                                                                                                                                                                    |
| New `synth.rs` file                       | +80–120                                                                                                                                                                                |
| New `verify.rs` file                      | +400–600 (the first layer of 07 is within this article’s scope but counted separately)                                                                                                 |
| New `regalloc.rs` file                    | **+500–1,000** (measured at 271 lines, 7b.4)                                                                                                                                           |
| **Total**                                 | **`ir_gen.rs` grows from about 8,448 to 8,600–8,800; including the new files, the net increase is 900–1,600 lines**                                                                    |

**The early expectation of “8,448 → 5,000–6,000” was wrong**; the measured analysis produced the
opposite result. The reason must be explained:

1. **The core benefit of this article is not fewer lines; it is eliminating an entire class of
   defects that “produce wrong values without reporting errors.”** The six manual save/restore sites
   (22 lines) become about 12 lines plus a guard type after SSA + RAII—**the line count is almost
   unchanged, but “forgot to restore” changes from a possibility into a compile error**.
2. **Pure code-motion decomposition necessarily increases the line count.** Decomposing the 746-line
   `generate_call_expr_ir` into seven functions repeats the signature (the function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), the `&mut self` binding, and error propagation
   in each one. **“Splitting the file to reduce the line count” is a misconception—it reduces
   cognitive load, not physical line count.**
3. **The new verifier and allocator are net line-count increases.** These two components (about
   900–1,600 lines) are precisely **the carriers of the benefit**.

**Honest statement of the net benefit**:

> In terms of line count, this article will most likely produce a **net increase**. The benefit lies
> in **verifiability**—turning defects that “produce wrong values without reporting errors” into
> invariants that fail verification. If line count is used as the acceptance criterion, this article
> will fail; the correct acceptance criteria are those in the “Batch Strategy” section.

**This stage deliberately increases the amount of code in `ir_gen.rs`.** The 2026-10-03 decision
cancelled all line-count / size gates (see Prohibition Two in [08](08-maintenance-mechanism.md)), so
**no line-count exemption list is needed**—the justification for the increase rests on separation of
responsibilities: SSA transformation extracts “register allocation + definition discipline” from
lowering into standalone modules, trading increased code volume for the elimination of the manual
discipline behind six manual save/restore sites and the semantic reordering of `arg_regs`, both of
which can “misalign without reporting an error and merely produce wrong values.”

## Key Decisions and Rationale

### Three Core Decisions

**Decision One: Temporary-Value Strategy—Delete the Three Statement-Level Rollbacks So That Unique
Definition Becomes a Mathematical Consequence of Construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, so every temporary slot naturally has exactly one definition point. **The
criteria agree, but the guards differ** (`3632` has a `result_reg.is_none()` guard, while the other
two do not), and such hand-copied differences disappear as a result. The cost is increased slot
occupancy—`temp_high_water` already has a 255-limit check (`1992-2000`) as a backstop.

**Decision Two: Do Not Perform mem2reg; Perform Only Explicit Renaming.** The rationale is given
under “Value Identification”: YaoXiang’s `&` / reference / ownership semantics make “which slots are
eligible for memory promotion” a type-system problem that requires the unified type representation
from [03](03-type-unification.md). **Performing mem2reg before 03 is guessing.** Explicit renaming
only requires changing the return-value semantics of one `next_temp_reg` call site and deleting
three rollbacks, without touching the control flow of any lowering algorithm.

**Decision Three: Use Option A for `Phi` (an IR-only pseudo-instruction expanded into a sequence of
Moves).** `executor/` and the `.42` format have **zero changes**, confining the regression surface
to L3. The decisive reason is that a `.42` format change triggers version-number issues and a full
compatibility discussion, while A’s size cost is irrelevant under interpreted execution (the
executed instructions are Moves, not Phis). The cost is that `Phi` is not visible in `dump_bytecode`
(when entering the C1/C2 stages, note that the third layer in 07 compares it).

### Rejected Alternatives

**A. Decompose everything at once (perform SSA + decomposition + the `synth` boundary + the
allocator simultaneously). Rejected.** This violates 07’s core principle: criteria must be tiered by
category. Mixing four batches into one PR mixes pure code-motion changes with C4 changes, making
snapshot drift impossible to attribute—when a snapshot changes, it is impossible to tell whether the
decomposition moved something incorrectly or the SSA change did. **Rollback granularity degrades
from “change item” to “everything.”**

**B. Split the file first, then optimize ordering. Rejected.** Decomposition eliminates no root
cause: the six manual save/restore sites remain scattered across ten files, and the `arg_regs`
semantic reordering still requires manual reasoning. Worse, **splitting first loses the opportunity
to “build criteria before changing anything”**—decomposition produces a large line-level diff that
mixes into subsequent SSA changes and invalidates review. This has the same origin as the eight
orphaned test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests that never ran):
**with the wrong order, every subsequent step accelerates along an incorrect foundation.**

**C. Introduce an Intermediate SSA Layer Without Changing Existing Lowering. Rejected, but recorded
as “a third path once considered.”** The form retains `ir_gen.rs` unchanged and adds a pass that
converts its non-SSA IR into SSA IR. Its **advantages** are that lowering is untouched and rollback
is easy. **Reasons for rejection**: (1) It does not eliminate the root cause of Defect Class 1—the
three statement-level reclamations remain in `ir_gen.rs`, and the six save/restore sites remain
separated by 59–161 lines. A conversion pass can merge multiple definitions into `Phi`, but **it
cannot fix “an inner function’s names leak into the outer scope”**—that is a `cur_locals`
save/restore error that occurs before the IR form and is invisible to the conversion pass. (2) It
turns SSA into a second representation—the repository already has the cautionary example of three
parallel type representations, and this project’s core lesson is that **“boundaries exist only in
people’s awareness.”** (3) `verify_loose` becomes a permanent burden.

**D. Keep the Status Quo and Add Only a Verifier. Rejected as the sole solution, but it is a
component of this article.** RFC-039’s Alternative B reaches the same judgment: a gate can prevent
regression but cannot repair the existing state. **What D can accomplish must nevertheless be stated
clearly**: D can **catch instances** of Defect Classes 1/2/3 (provided that the verifier covers
dominance, type consistency, and inner-scope isolation), but it cannot catch Defect Class 4
(`closure_captures` leakage)—that requires a cross-function liveness assertion beyond the
`verify(&ModuleIR)` function boundary. **Therefore, D is not an alternative to this article; it is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point**—the 2026-10-03 decision cancelled line-count / size gates, and the scale issue is
  addressed through separation of responsibilities (see Prohibition Two in
  [08](08-maintenance-mechanism.md)). The increase is justified by replacing the manual discipline
  behind six manual save/restore sites and the semantic reordering of `arg_regs`, both of which can
  “misalign without reporting an error and merely produce wrong values.”
- **The “unique definition” check in `verify.rs` is meaningless in the non-SSA form**
  (`Operand::Local` permits multiple definitions)—07 records this limitation, and `verify_loose` is
  its degraded mode for this batch.
- **Deleting statement-level reclamation changes the temporary-slot allocation pattern** (executed
  in 7b.4, together with 7a.1): after deletion, slots are reused by the linear-scan allocator
  according to live intervals, and **the allocation result changes**—this is exactly why C4 must
  genuinely run the full corpus; the measured 403/403 differential result was empty.
- **`u8` / 255 slot limit**: the two corpus cases that exceeded the limit after reclamation
  (`string_ops.yx` with 354 monotonic slots and `list_comp.yx` with 323 monotonic slots) are
  compressed by the allocator to **15 / 24** frame slots. The budget gate now reports E3014 only
  when the post-allocation frame genuinely does not fit, including the function-body declaration
  span, rather than incorrectly rejecting based on “cumulative temporary count.”
- **The benefit must also be recorded**: turning “errors that cannot be tested” into “invariants
  that can be tested”; no backend changes (`executor/` and `.42` have zero changes); no changes to
  `Instruction` semantics (all 76 → 77 variants are retained, as are all 50 `translate_*`
  functions); incremental delivery (every item can be reverted independently).

> **All open questions originally listed in this section have been adjudicated.** The item-by-item
> decisions are recorded in
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> article leaves no pending items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) — the
  overarching general plan; four-layer model, criterion tiers, and P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definition of the C4
  criterion category, `verify_loose` / `verify_ssa`, the three layers of criteria, and the
  limitation that snapshots are immune to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; consolidation of three
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the
  `ReleasePlan` `PlanId` transformation
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — eight orphaned test trees; the counterexample
  to “create the directory first, then fail to wire it up”

### Code Locations

**IR Definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — 7 `Operand` variants
- `src/middle/core/ir.rs:47-533` — 76 `Instruction` variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (the feasibility basis for the
  `PlanId` solution)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect Class 1 (Multiple Definitions and Save/Restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for the seven nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — the sole table-extraction + E3014 budget
  point (`MAX_REGISTERS = 255`), combining block partitioning / slot allocation / table extraction
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named-variable high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — linear-scan allocator (the sole production point for
  `value_slots` and `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamation
  removed (7b.4); slot reuse is now handled by the allocator
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant in 07

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  a clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill 0” fallback

**Defect Class 3 (Span-Keyed Contracts)** — the following are **audit-time evidence pointers**
(after the 4.5 key migration: the `release_plan` / `overload_resolutions` keys have been converted
to `PlanId`; current pointers are in the §2 revision note in [02](02-stage-contract.md);
`method_def_ordinals` was eliminated in 7c.3 through Mechanism Three)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consuming `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consuming `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write site of
  `method_def_ordinals` (the key is the unqualified `"Type.method"`; **7c.3 removed this field**,
  and this is evidence from the audit)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the transmission structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — definition and
  production of `ReleasePlan`
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer transmission
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (Implicit Order Dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — the equivalent handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — two read sites of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — three other constructions of `ast::Type`

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — 66 `BytecodeInstr` variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hardcoded discards (compare `2344` /
  `2347`); old audit records, with review conclusions in this article’s “Tracking Hardcoded
  Discards” section (two genuine losses + one false positive)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (the bridge awaiting removal
  in [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — nine NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  bound of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current Tests**

- `src/middle/core/ir_gen.rs` — 8,448 lines, 0 `#[cfg(test)]`, 0 `mod tests`
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no IR
  structural-invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — four mentions of ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example per-opcode round-trip criterion; the final test in the file; the file contains 1,209 lines
  / 23 `#[test]` functions)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies contain only `criterion` / `proptest`; `[dependencies]`
  contains no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans the `/rfc/draft`
  directory; this article is not under that directory and is referenced by RFC-039 and this
  directory’s index
