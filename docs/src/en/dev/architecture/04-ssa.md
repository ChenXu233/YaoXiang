# SSA Conversion of the Intermediate Representation

> **Supporting design document**. This article is a supporting document for
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion tiers, and execution phase sequence are covered in the main
> RFC-039 text; the role of each supporting document is described in the
> [index for this directory](index.md).

## Position and Scope

This article addresses IR construction discipline at the L3 layer and proposes a construction method
based on the principle of “one definition per value” for `src/middle/core/ir_gen.rs` (8,448 lines).
**The goal is not to introduce a new instruction set, but to eliminate three categories of
discipline gaps that produce no errors when they fail**:

1. Six hand-written register save/restore sites separated by dozens of lines;
2. Semantic reordering of `arg_regs` shared by six branches inside `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of span-keyed cross-layer contracts.

This article is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, explicit cross-layer contracts, splitting the call-expression
dispatcher, and the wiring of an IR static verifier.

This article is not responsible for: convergence of type representations
([03](03-type-unification.md)), stage contracts and the obligation ledger
([02](02-stage-contract.md)), definition of equivalence criteria and gates
([07](07-equivalence-oracle.md)), frontend paradigms ([05](05-frontend-paradigm.md)), or cleanup of
dead-code and placeholder designs ([06](06-cleanup-inventory.md)). This article references the
conclusions of those documents in several places without repeating their arguments.

**The criterion category** is **C4 (IR shape changes)** as defined in
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, without requiring byte-for-byte equality of IR snapshots. SSA conversion necessarily
changes the IR shape; forcing snapshot equality would encourage the team to weaken the criterion—the
trap recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, with acceptance requiring behavioral equivalence and a green `verify_ssa`). The
implementation batch numbers within this article (batches a–d) are distinct from the internal
ordering of P7: the former are the batching strategy for this article, while the latter are the
global phases of RFC-039.

## Feasibility Preconditions

**Verified facts.** The IR already has **all structural prerequisites** required for SSA
construction:

| SSA prerequisite                 | Current state                                                                                                    | Location                                          |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| Explicit CFG                     | `BasicBlock { label, instructions, successors }`                                                                 | `BasicBlock` in `src/middle/ir/ir.rs`             |
| Explicit entry block             | `FunctionBody::Code { blocks, entry, locals }`                                                                   | `FunctionBody` in `src/middle/ir/ir.rs`           |
| Nested scope information         | `LocalSlot::scope_depth` (“0 = function parameter level, used to disambiguate duplicate names in nested scopes”) | `LocalSlot` in `src/middle/ir/ir.rs`              |
| Three-address form               | 76 variants, 54 with `dst: Operand`, and 3 with `dst: Option<Operand>`                                           | `Instruction` in `src/middle/ir/ir.rs`            |
| Per-instruction span             | Every variant has a `span: Span` field                                                                           | Same as above                                     |
| Annotatable value types          | `LocalSlot::ty: MonoType`                                                                                        | `LocalSlot` in `src/middle/ir/ir.rs`              |
| Upper-bound check for slot count | Single point for table extraction and the E3014 budget gate (`MAX_REGISTERS = 255`)                              | `take_body_locals` in `src/middle/lower/scope.rs` |

**If these prerequisites were absent, the cost estimate in this article would be entirely
different.**

**The two decisive points:**

**First, SSA conversion is not a new capability; it removes an existing capability dependency.**
Temporary slots are currently **deliberately designed to be defined multiple
times**—`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`, explicitly roll back `next_temp` in
three places. The first step in SSA conversion is to **delete these three rollbacks**, allowing
`next_temp` to increase monotonically. Once they are removed, **every temporary slot naturally has
exactly one definition point**—unique definition changes from “a rule that needs checking” to “a
mathematical consequence of the construction method.” Batch a only converged the save/restore
surface (see “Defect Class 1”); deleting the rollbacks itself is done with the allocator in batch b.

**Second, the slot-count upper bound is a hard check, and removing the rollbacks causes the number
of slots to become uncontrolled (measured).** `take_body_locals` in `src/middle/lower/scope.rs`
extracts the table according to `temp_high_water` and reports E3014 when the count exceeds 255.
After measuring with all three reclamation points removed across the full corpus, two corpus entries
crossed the limit and triggered E3014: `src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; the peak slot count for the
remaining corpus entries rose from 51 to 211. `temp_high_water` records the historical maximum
occupancy and therefore does not itself change with rollback; however, once rollback is removed, the
allocator no longer reuses slots, so the peak rises.

**The combined meaning of these two points is:** lowering already maintains all the bookkeeping
information required for SSA, but **recycles it**. Deleting the rollbacks must wait until the
allocator (7b.4) assumes responsibility for slot reuse. The main workload of SSA conversion lies in
splitting `generate_call_expr_ir` and making cross-layer contracts explicit, not in building IR
expressiveness.

### Prerequisite: Unification of Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The third representation on the serialization side is `ir::Type`,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR currently contains two type representations, one of which directly
`pub use`s the AST type.**

SSA conversion requires a new `Phi` variant, a type for each SSA value (for the “type consistency”
invariant in the 07 verifier), and a determination of whether the types of two predecessors are
compatible at a CFG join point. **If type representations have not yet converged, SSA will require a
separate type-compatibility check for each representation**, allowing the third representation to
grow.

This is not a question of which task should come first; it is a question of whether the first
implementation will produce **three sets of type-compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first converge the
`pub use ast::Type` at `ir.rs:3` and remove the bridge at `bytecode.rs:2353`. Item 6 in this
article’s change list explicitly records that `ir.rs:3` **is not modified by this article**.

## Current State: Four Categories of Defects

`ir_gen.rs` is not “an ugly file that happens to work.” It is a file that **places correctness
entirely in the hands of line-by-line human review**:

- The file contains **zero `#[cfg(test)]` blocks and zero `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares three modules—`bytecode` / `def_assign` /
  `local_slots`—with 29 tests in total. Among them, `def_assign.rs` and `local_slots.rs` provide
  **positive** coverage of ir_gen’s DefId allocation and local-slot naming (at sentinel level),
  while the four references to ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **all
  negative assertions**—“ir_gen does not produce,” “ir_gen also does not construct,” and “the ir_gen
  frontend does not construct the corresponding IR.”

In other words: ir_gen has 29 sentinel-level tests pinning local behavior, but **no IR structural
invariant validation** (dominance, unique definition, jump targets, or type consistency). None of
the three modules declared by `middle/core/tests/mod.rs` makes positive assertions about the overall
IR shape.

This file also happens to contain several categories of defects that **compile successfully, pass
all checks, and run normally, but produce incorrect values**. Their common property is: **failure
produces no error**.

This is where the highest risk lies. The problem is not “the function is too long,” nor is it “the
naming is confusing.” The problem is that **there is no machine-executable enforcement mechanism for
the core invariants of this file**.

The contract “save before generating a nested function body, then restore it afterward, or names
from the inner function will leak into the outer function” is enforced by the type system:
`NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator,
and frame removal and restoration each have only one implementation (see “Defect Class 1”).

**Why simply adding a verifier is not enough**: a verifier can **catch** these defects, but it
cannot catch their **causes**. The cause is not “a reviewer missed something,” but rather:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator actively depends on this behavior.**

The concrete evidence is three statement-level register reclamation points (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`). At the end of every statement, they **roll back** `next_temp`
to the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and the number and
order of writes, as well as live ranges, are determined entirely by the single manual decision of
**when to roll back**.

Under this shape, “unique definition” is not a rule that can be applied incrementally; it **directly
conflicts with the register-allocation strategy**. Therefore, SSA conversion is not a matter of
adding a check to the existing code; it is a matter of **replacing the allocation strategy’s
dependency on multiple definitions**. The good news is that replacing this strategy is
inexpensive—see “Feasibility Preconditions” above.

### Defect Class 1: Multiple Definitions and Manual save/restore (the restore surface has been converged in batch a)

**Verified fact.** There are seven entry points at which entering a nested function body requires
save/restore. Batch a (7a.2) converged them into one RAII guard: `NestedBodyGuard<'a>` in
`src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, unconditional restoration in
`Drop`, and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Restoration scope                                   |
| ------------------------------------ | ---------------------------------- | ----------------------------- | --------------------------------------------------- |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (three counters are not restored) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                       |
| Nested function definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                  |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |

- Frame removal occurs only in the two constructors (`enter` / `enter_decl`), while restoration
  occurs only in `Drop`; the guard exclusively holds `&mut` access to the generator for its
  lifetime, so no second `&mut` is available in scope, and `?`/early returns all restore through
  `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span`
  / `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`; `next_temp_reg` continues
  to maintain the true high-water mark in `temp_high_water` (statement-level reclamation rolls back
  `next_temp`, while the total number of slots is based on the historical maximum occupancy).
- `enter_decl` does not restore the three counters: after a function body/method body returns to the
  module initialization flow, subsequent definitions continue from the child body’s final value
  number. This is the C1 zero-drift snapshot constraint and the meaning of “the declaration side
  restores only the table.”

**Unified semantics and a single budget point (7a.3 implemented)**: `take_body_locals` in
`src/middle/lower/scope.rs` is the only table-extraction and budget point—it truncates according to
`temp_high_water`, reports E3014 with the function-body span when out of range, and defines
`MAX_REGISTERS = 255` in exactly one place in the repository. All six function-body forms—function /
method / lambda / anonymous binding / curry intermediate layer / curry innermost layer—use it to
extract their tables; there is no second definition of “how many slots a function body should have.”

**The three duplicated statement-level register reclamations still remain (7a.1 split out into batch
a and moved to batch b, together with 7b.4)**:

| Location                           | Function                  | Guard                     |
| ---------------------------------- | ------------------------- | ------------------------- |
| `src/middle/lower/stmt.rs`         | `generate_function_ir`    | None                      |
| `src/middle/lower/stmt.rs`         | `generate_block_ir`       | `if result_reg.is_none()` |
| `src/middle/lower/expr/closure.rs` | `generate_lambda_body_ir` | None                      |

The guard in `generate_block_ir` has a clear rationale: it handles blocks that produce an expression
operand, and the outer scope may hold sibling argument temporaries that live across the block. The
other two sites have no equivalent guard. **The same criterion is copied by hand in three places,
with one guarded implementation and two unguarded implementations**—exactly the kind of
inconsistency SSA is intended to eliminate.

**Why it was split out**: deleting rollback causes the slot count to become uncontrolled (see
“Feasibility Preconditions,” item two, and “Known Limitations and Risks”), so a linear-scan
allocator (7b.4) must first assume responsibility for slot reuse.

**Consequences (current shape)**: the guard exclusively holds `&mut`, restoration has only one
implementation in `Drop`, and there is no writable location at which a restore can be missed. What
remains is “one slot with multiple definitions” itself—the allocator depends on it, and the I-layer
invariant therefore cannot assert unique definition until the reclamations are deleted in batch b.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified fact.** `generate_call_expr_ir` is located at `ir_gen.rs:7269-8014` and contains **746
lines** in a single function. Its signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (along with `_expr` / `result_reg` / `instructions`
/ `constants`). Internally, six branches share the same `Vec<Operand>` and perform **semantic
rewriting**:

| Line        | Semantics                                                                                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `7395`      | Namespace call: flatten according to `slots`, then recollect—named arguments are reordered                                                                   |
| `7724`      | Struct construction: unwrap `final_args` slot by slot, then recollect—fields are reordered                                                                   |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **adds a new register with value 0** for each missing slot (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—cloned before emitting instructions                                                                       |

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
reports E1010. As a runtime safety measure, fill in 0 (the normal path should never reach here).”

**This design has two kinds of risk**:

1. **Misalignment produces no error, only an incorrect value.** Each of the six branches determines
   the final order of `arg_regs`, and any incorrect ordering assumption will not panic.
2. **Normalized snapshots are immune to this problem.** The limitation is already recorded in 07:
   after the snapshot tool renames temporary values by “first-occurrence order,” “the third argument
   should have used the fifth register” is identical in the snapshot. **This class of defect does
   not even have a snapshot-level criterion at the C4 stage.**

**Additional verification: the struct-construction path has no duplicate-slot guard when
reordering.** The `final_args` at `7724` is populated by “positional arguments first, then named
arguments overwrite by field name,” with **no duplicate-slot check such as `slots[idx].is_some()`
before overwriting** (unlike the namespace path at `7381`). When a named argument silently replaces
a positional argument, there is no panic or diagnostic; moreover, the argument **evaluation order**
(source order) diverges from the final `arg_regs` order (field declaration order). The
`fill_missing` assertion work in batch d must also cover the duplicate-slot check, or the silent
replacement in the struct-construction path will survive unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified facts (at the time of the audit).** The repository contains **two** span-keyed
cross-layer contracts:

| Contract               | Declaration                                                                                                                    | Producer                                                                                                             | Propagation                                                                           | Consumer                                                                                                                    |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                      | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatches `walk_stmt` by statement identity)  | `typecheck/types.rs:38` ← passed by `checker.rs` ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, sole entry point; settlement-completed marker `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger combined, `types.rs:62`) | `inference/expressions.rs:4550` (writes by call-site `PlanId`; `inference/statements.rs` only propagates and merges) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held as an `ir_gen.rs` field    | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; settlement-completed marker `:1097`) |

The shape at the time of the audit (`ir_gen.rs:1942`, **now absent**):

```rust
// NLL Release: insert a Drop instruction at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

The `if let Some` means that **a lookup miss is simply a miss**: there is no `else` branch, no
assertion, and no counter. Any mismatch in how the two sides compute spans causes the **Drop
instruction to disappear silently, with zero errors**. The Drop sequence for refinement types is a
central carrier of this project’s ownership semantics (`ownership.rs:26-32` describes it as the “NLL
precise release plan”), yet its lowering was silent.

The same pattern at `7420` / `7493` (`if let Some(mangled) = self.overload_resolutions.get(span)`)
causes overload resolution to fall back to the default binding.

**Disposition (implemented in 4.5)**: both keys were changed to the structural identity `PlanId`,
and consumption is **accounted for at the point of a hit** (the read and accounting occur in the
same call). A non-empty difference is reported as an **E-level `W1090`** at the sole settlement
point—“lookup miss” is no longer silent. The implemented shape and criteria are described in the
revision note in [02](02-stage-contract.md) §2 (4.5). These line numbers (`1942` / `7420` / `7493`)
are retained as evidence from the audit and are not pointers to the current implementation.

> **`method_def_ordinals` is not span-keyed.** It is a `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with its sole write point at `:1692-1702` (allocating `#N` suffixes in
> definition order based on `base_name`). Its risk is **different**: definition-order dependence.
> The `#N` suffix is determined by AST definition order, so if generation order differs from
> typecheck registration order, the same function name may resolve to a different target. This is
> not a span mismatch, but it likewise “fails without reporting an error.” Its disposition is
> described under mechanism three in “Additional Mechanisms for Span-Keyed Problems.”

### Defect Class 4: Implicit spawn/lambda Handshake

**Verified fact.** `ir_gen.rs` has **two** locations that construct `ast::Expr::Lambda` themselves
and then call back into `generate_expr_ir`, relying on a handshake through mutable fields:

**Handshake A — `generate_spawn_for_ir` (entry point `4143`):**

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir` (entry point `6543`) → `6571-6572` consumes
`std::mem::take(&mut self.pending_env_vars)`.

**Handshake B — `generate_spawn_expr_ir` (entry point `6641`):**

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**Handshake C — `closure_captures`**: written at `6589-6593`, with `generate_lambda_body_ir` (entry
point `4678`) called **in the same function** at `6598`, and the table cleared at `6600`. There are
two read points: `:6284` (`LoadUpvalue` in a closure body) and **`:458`**
(`closure_captures.contains_key(head)` on the `flatten_namespace` path).

The `:458` read point is the most dangerous: it lies on the **namespace-resolution path**, meaning
that the live state of a `HashMap<String, usize>` can change variable-name resolution. A complete
closure-body generation (`6598`) occurs between the write at `6589` and the clear at `6600`; if any
path inside `generate_lambda_body_ir` returns early without clearing the table—there are multiple
return points when the `?` operator propagates an error—**`closure_captures` remains in the
unrelated code at `:458`**.

Other implicit state of the same kind includes `pending_env_names` (written by handshake B at
`6699`, cleared at `6714`).

### What SSA Will Not Solve (An Honest List)

| Problem                                              | Location                                                                                                                               | Why SSA Does Not Solve It                                                                                                                                                                                                                        |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `compile_pattern` complexity                         | `ir_gen.rs:5390-5687`, **298 lines**, recursively self-calls four times (`5473` / `5504` / `5619` / `5640`), external call site `5270` | Dispatch complexity for pattern matching (union payload / tuple / struct / literal / nested) is **intrinsic to the algorithm**. SSA only turns the `JmpIfNot` generated by guards into `Phi`; it does not reduce the number of guards            |
| `eval_const_expr` complexity                         | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming recursive self-calls)          | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                                                  |
| Span-keyed failure (**eliminated**)                  | At audit time: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)—after 4.5, `PlanId` keys + mandatory settlement       | **SSA does not touch this layer at all** (the conclusion is unchanged). It was independently resolved by the obligation-ledger mechanism in [02](02-stage-contract.md); see mechanism two under “Disposition of Span-Keyed Problems”             |
| Number of `Instruction` variants                     | 75 (the `Instruction` enum in `ir.rs`; includes the `Phi` introduced by 7b.1a)                                                         | SSA adds `Phi`, for a **net change of +1** (already implemented). Variants correspond to bytecode opcodes or are degraded to NOP; they do not decrease because of SSA. Under option A, `Phi` is an IR-only pseudo-instruction and adds no opcode |
| Definition-order dependence of `method_def_ordinals` | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                                  | Similar to span-keying but with a different cause; see mechanism three under “Additional Mechanisms for Span-Keyed Problems”                                                                                                                     |

## Target Design

### Definition of the SSA Shape

**Design judgment.** The target shape is as follows.

#### Changes to the `Operand` Variants

The `Operand` enum in `ir.rs` has 5 variants. **Repository-wide measurements** (hit counts obtained
by summing `git grep -c` per file over all of `src/`, including tests; construction-point counts
refer to value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                              | Disposition                                                                                      |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                                      | **Retained**, unchanged                                                                          |
| `Value(ValueId)`    | 7 files / 26 hits; **0 production construction points** (production is handled by 7b.2’s `next_temp_reg`)                                                                                                | **Retained** (introduced by 7b.1a)                                                               |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                                      | **Retained, but semantics split** (see below)                                                    |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                        | **Retained** as an implicit definition at function entry (parameters are definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                                       | **Retained**, unchanged                                                                          |
| `Temp(usize)`       | **0 construction points** (the handling arm exists only in `codegen/operand.rs` in `OperandResolver::to_reg` and `is_register`, and is removed with the variant)                                         | **Removed** (dead variant, 7b.1a)                                                                |
| `Label(usize)`      | **0 construction points**                                                                                                                                                                                | **Removed** (dead variant, 7b.1a)                                                                |
| `Register(u8)`      | **0 construction points** (the comment at the original variant declaration said “Added for codegen,” but codegen uses the `OperandResolver` in `codegen/operand.rs` and does not construct this variant) | **Removed** (dead variant, 7b.1a)                                                                |

`Local(usize)` is split into two variants because it currently **serves two mutually exclusive
semantics**—“storage location for a named variable” (which may be written multiple times) and
“compiler temporary register” (which should be written exactly once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within a function, defined exactly once
    Value(ValueId),
    /// Storage slot for a named local variable: addressable and allowed multiple definitions (non-SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing and never rolled back**. The split
and `ValueId` were implemented in 7b.1a (the `Operand` enum in `src/middle/ir/ir.rs`); this batch
has 0 production construction points for `Value`—the production side (changing `next_temp_reg` to
produce `Operand::Value`) belongs to 7b.2.

#### `Phi` as a New `Instruction` Variant

```rust
/// Join point: each predecessor basic block provides one definition of this value
/// incoming is sorted by predecessor block label in ascending order (guaranteed at construction time, to simplify normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (see the first layer in [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set of `incoming` equals the label set of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. The `MonoType` of all `incoming` values is identical (**dependent on the type representations
   having converged in 03**).

Current status of `verify_ssa`: constraints 1 and 2 require the value table and block-level CFG, so
they belong to **7b.1b**. This batch (7b.1a) implements the two construction-side constraints—`dst`
must be `Operand::Value`, and the labels in `incoming` must be strictly ascending
(`check_phi_structure` in `src/middle/ir/verify.rs`, violation category
`InvariantKind::PhiConsistency`). Constraint 3 depends on convergence of the type representations in
03 and proceeds together with 7b.1b.

7b.1a only introduces the variant: there are 0 lowering production construction points, and the
consuming arm in `translator.rs` is a placeholder with an invariant comment that **fails loudly
rather than silently skipping or producing an incorrect value**. Actual translation belongs to 7b.3
(option A: expansion into a sequence of Moves).

**Disposition of the guard problem at `3632-3638`**: the current reclamation in `generate_block_ir`
has an `if result_reg.is_none()` guard (the comment at `3626-3631` explains that the block produces
an expression operand, and the outer scope may hold sibling argument temporaries that live across
the block). After SSA conversion, this guard is **no longer needed**, because temporary values are
no longer rolled back, and sibling arguments that live across blocks hold their own `ValueId`, which
is unaffected by rollback. **This is one area where SSA conversion genuinely simplifies the code,
rather than merely renaming it.**

#### Value Identification Method

**Design judgment**: do not perform mem2reg, do not introduce virtual-register numbering, and
**perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots can be promoted to registers” a
  **type-system problem**, not a CFG problem. It has been verified that `LocalSlot::ty: MonoType` in
  `ir.rs` carries complete type information, but determining “whether this `MonoType` can be
  promoted” requires the converged type representation from 03 and coordination with refinement-type
  rules. **Performing mem2reg before 03 is guessing.**
- Explicit renaming only requires changing the return-value semantics of one function
  (`next_temp_reg()` in `src/middle/lower/scope.rs`) and deleting three rollbacks; it **does not
  touch the control flow of any lowering algorithm**.

Value-table shape (**to be implemented in 7b.1b**): `FunctionBody::Code` gains
`values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None indicates a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block containing the definition
}
```

`values` and `locals` coexist: `locals` carries addressable named variables (which may have multiple
definitions), while `values` carries single-definition SSA values. **The presence of `locals` does
not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

#### Change-Size Estimate

| Item                                                                  | Estimate                              | Basis                                                                                                                                                                                                                                                               |
| --------------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change to `next_temp_reg`                                             | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                      |
| Delete three statement-level reclamations                             | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                             |
| Converge six save/restore sites into RAII                             | Net ±0 lines                          | See “Item-by-Item Disposition Table”                                                                                                                                                                                                                                |
| `ir.rs` structural changes                                            | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **the first two were implemented in 7b.1a** (measured: `src/middle/ir/ir.rs` +37/−3 lines, `normalize.rs` +22/−5, `verify.rs` +63/−14); `ValueInfo` and the value table await 7b.1b                                  |
| Add a `Phi` arm to `translator.rs`                                    | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                      |
| `executor/`                                                           | **No changes**                        | Option A: `Phi` is an IR-only pseudo-instruction, expanded into a sequence of Moves in `translator.rs`, and does not enter bytecode (Decision Three)                                                                                                                |
| Call sites that change temporary-value production to `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (`git grep -c` summed per file over all of `src/`, including tests): `Operand::Local(` occurs 415 times across 28 files, including 332 times across 14 files in `src/middle/lower/`; temporary values are produced at the single point `next_temp_reg` |

### Item-by-Item Disposition Table

| Defect class                                                | Eliminated by SSA?                                                      | Disposition                                                                                                                                                                                                                                                                                                                                                                                         | Criterion                                                                   |
| ----------------------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore**         | **Partially (restore surface converged; unique definition awaits SSA)** | The seven nested-function-body entry points have been consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the three counters), making “forgot to restore” a compile error; the three statement-level reclamations remain until batch b (7a.1 and 7b.4 in the same batch), and `next_temp` can still be rolled back | First-layer “inner isolation” invariant in 07 + `verify_loose`/`verify_ssa` |
| **1b. Inconsistent semantics**                              | **Yes (implemented)**                                                   | Unified under `take_body_locals` in `src/middle/lower/scope.rs`: truncation according to `temp_high_water`, with the E3014 budget check at the same extraction point and with the function-body span; all six function-body forms are covered; `MAX_REGISTERS` is defined in exactly one place                                                                                                      | Corpus differential + zero-diff snapshots (demonstrated)                    |
| **1c. Criteria for the three statement-level reclamations** | **Yes (pending batch b)**                                               | The three reclamations still exist; deletion is performed in batch b together with 7b.4 (after the allocator assumes responsibility for slot reuse); the `result_reg.is_none()` guard in `generate_block_ir` is removed as well                                                                                                                                                                     | C4: behavioral equivalence + invariants                                     |
| **2. Semantic reordering of `arg_regs`**                    | **Partially**                                                           | SSA turns “wrong register” into “wrong value ID,” which can be caught by dominance checks, but **the reordering logic itself remains**. Actual elimination depends on splitting the function—each of the six branches performs `push`, while ownership of `arg_regs` remains at the top of the function                                                                                             | First-layer dominance invariant (snapshots are immune)                      |
| **3. Span-keyed failure**                                   | **No**                                                                  | SSA does not touch it. See “Additional Mechanisms for Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                          | `test_release_plan_spans_consumed` from 07                                  |
| **4. Implicit ordering dependency**                         | **No**                                                                  | See “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                           | A new explicit criterion is required                                        |
| `compile_pattern`, 298 lines                                | No                                                                      | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                       | —                                                                           |
| `eval_const_expr`, 191 lines                                | No                                                                      | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                       | —                                                                           |
| 75 `Instruction` variants                                   | No (+1, implemented)                                                    | `Phi` was introduced by 7b.1a; under option A, it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                               | —                                                                           |

### Register Allocator: Custom Implementation or Existing Crate

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **0 construction points across the repository**—the variant is no
   longer present in the `Operand` enum in `ir.rs` (removed by 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper limit of 255 (exceeding it
   produces E3014); resolution of `Operand::Value` slots awaits 7b.2 and currently returns the
   explicit diagnostic E3017.
3. `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50 `translate_*`
   functions distributed across `101-1558`) outputs `BytecodeInstruction::new(opcode, operands)`,
   with opcodes from `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter’s opcode-dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Assessment of three options**:

| Option                                             | Assessment                                                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both take rustc’s `Function` trait plus MIR `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait set (CFG, liveness, spill slots, scratch), requiring **500+ lines of adapter code** for no benefit, because there is no real register file to color. Rustc itself only performs linear scanning at this point (`enable-llvm` disabled) |
| `graph-alloc` family                               | **Not adopted.** Same reason, with a less stable API                                                                                                                                                                                                                                                                                                                                                               |
| **Custom linear scan (simplified Chaitin-Briggs)** | **Adopted.** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot in `values` order (`OperandResolver` already has the `u8` contract and E3014 limit). This is essentially **maximum-clique coloring of non-overlapping live intervals**, which, without a register-file constraint, degenerates into “allocate the first available slot in definition order”                |

**Design judgment**: implement a custom allocator targeting 500–1,000 lines (including the
`values → Reg` mapping, live-interval computation, and its interface with `OperandResolver`). **At
the same time, it must be acknowledged that if the `u8`/255 slot limit is insufficient under real
workloads, the custom allocator must handle overflow; the existing code already treats this as an
error at `1992-2000`.** This means that this article **does not solve** the “large function does not
have enough registers” problem—**that is a downstream consequence of `compile_pattern` recursion
depth and belongs to another topic.**

The dev-dependencies at `Cargo.toml:37-39` consist only of `criterion` / `proptest`;
`[dependencies]` contains no regalloc / petgraph / graphlib. Introducing any such crate would add a
new dependency surface.

### Splitting Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA shape and criteria must exist first**. The reason is that
the split itself is C1 (pure movement, with a zero-diff snapshot criterion), but only after the
`arg_regs` semantics have been eliminated by SSA to “one independent `ValueId` per argument” will
the split avoid carrying the old ordering assumptions into a new file.

**Core design constraint: ownership of `arg_regs` remains at the top of the function; the six
branches only `push` and never reorder.**

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

The “fill in 0” logic at `7835-7848` (`7838-7846`) is moved down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the “the normal
path should never reach here” self-description is replaced with an assertion**: if any slot is
missing and typecheck did not report E1010, return a diagnostic directly. **This turns a silent
fallback into a hard error and is the most direct improvement this article makes to Defect
Class 2.**

**Criterion**: the pure-movement portion (before the SSA shape changes) uses zero-diff snapshots;
assertion-based `fill_missing` is C4 and uses corpus differentials plus the first-layer verifier.

### `synth.rs` Boundary: Turning Implicit Callbacks into an Explicit Synthetic-AST Boundary

**Verified fact**: the locations in `ir_gen.rs` that construct `ast::Expr` are **exactly
two**—`:4246` and `:6701` (with `:6704` constructing `ast::Stmt`, and `:510` / `:1266` / `:1399`
constructing `ast::Type`).

**Proposal**: create `src/middle/lower/synth.rs` as the **only module in the repository** permitted
to construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not “tidiness,”
but rather:

> Constructing an AST means **re-entering the frontend-lowering entry point**. However,
> `generate_expr_ir` expects the `pending_env_vars` / `closure_captures` state to be **a
> precondition prepared by the caller**, and this precondition is currently expressed by neither a
> type nor a signature—it holds only because of the runtime fact of “who calls me.”

Change this to an explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST that captures env; the capture set is provided explicitly as a parameter,
/// without relying on the caller to populate self.pending_env_* in advance
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Specific changes:

| Location                       | Current state                                                                                              | New state                                                                                                                                                                                                                   |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → construct Lambda → call back                                                    | Call `synth::synth_closure(...)` to obtain the AST and pass `SynthEnv` explicitly; add an internal entry point carrying `SynthEnv` to `generate_lambda_expr_ir` (`6543`), which **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → construct → call back → clear                                                                      | Same as above; the `clear()` calls at `6713-6714` disappear because there is no shared state to clear                                                                                                                       |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                                    | Read from the parameter instead                                                                                                                                                                                             |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                                           | Same as above; `closure_captures` becomes a **local variable during closure-body generation**, and the `clear()` at `6600` disappears                                                                                       |
| `ir_gen.rs:458`                | Reads the shared field through `self.closure_captures.contains_key(head)` on the namespace-resolution path | **This is the most dangerous location**: change it to an explicit parameter, or any `?`-triggered early return from `generate_lambda_body_ir` can leak the capture table into unrelated code                                |

**Criterion**: add the sentinel test `test_no_ast_construction_outside_synth`, which scans
`src/middle/**` and rejects `ast::Expr::` / `ast::Stmt {` construction syntax outside `synth.rs`.
Its form is the same as RFC-039’s `check-module-boundary.py`.

### Disposition of Span-Keyed Problems (the Part SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism One: Consumption-Coverage Assertion (mandatory, lowest cost; superseded by a stronger
form on 2026-10-13).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). The
original addition proposed in this article was to make it a **runtime count** rather than merely a
test. The implemented form is stronger—**a consumption ledger (`ConsumptionLedger<K>`) + a sole
settlement point (E-level `W1090`)**—with no new diagnostic code and without relying on an internal
counter in ir_gen to prove itself:

```rust
// near ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each hit at :1942, self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan was not consumed by the IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a required drop was omitted” may not immediately crash under YaoXiang’s
current value semantics and Arc/Rc model**—which is precisely why it was silent. Counting turns
“silent” into “detectable.”

**Mechanism Two: Change Span Keys to `DefId` or Explicit Plan IDs (**implemented on 2026-10-13**:
RFC-039 D20 / WBS 4.5 implements this mechanism as `PlanId(u32)` structural-identity keys +
mandatory consumption settlement; the implemented shape is described in the revision note in
[02](02-stage-contract.md) §2).** The fundamental fix is to **stop using Span as a key**. `Span` is
a source location and may drift because of macro expansion, `include`, and multi-file merging.
`FunctionIR` already has a `def: Option<DefId>` field (`ir.rs:677`), demonstrating that DefId is
available at the IR layer. **Design judgment**: introduce `PlanId(u32)` for `ReleasePlan` and
`overload_resolutions`, allocated by ownership/overload resolution when producing the entries and
matched by `PlanId` when the IR consumes them. **This belongs to the obligation ledger
(`Obligations`) in 02, should be implemented together with [02](02-stage-contract.md), and is not
part of this article.**

**Mechanism Three: Definition-Order Dependence of `method_def_ordinals` (an independent small
item).** The field is a `HashMap<String, usize>` (`ir_gen.rs:224`), whose key is
`"{type_name}.{method_name}"` (**without module qualification**; written at `:1691-1700`)—two
modules that define the same `Type.method` share one counter, causing the mixed `#N` names to drift.
**Its failure mode is not a span mismatch but an ordering/name-collision mismatch**: if typecheck
registration order differs from ir_gen definition order, the same method name may resolve to a
different target. Disposition: have the typecheck side **produce `HashMap<DefId, String>` once**
(bare names or mixed `#N` names), and make ir_gen read-only. **This eliminates one mutable
cross-function state in ir_gen.**

### Tracking the Three Hard-Coded Discarded Fields

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hard-coded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                                             |
| ------------------ | -------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | The closure upvalue count becomes zero after a `.42` round trip                                                                         |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **A `.42` artifact loses the entire exception table**                                                                                   |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **A `.42` artifact loses global-variable information** (note that `global_names` at `2347` **is** populated, creating an inconsistency) |

**Design judgment (revised on 2026-10-05, Decision D52): all three are brought into the scope of
this article as P7 batch e (7e), and no separate issue will be created.** The original
judgment—creating separate issues for the latter two—and its rationale are recorded below, together
with the reasons for overturning it:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields) and are
   unrelated to SSA.~~ This is true, but it is not grounds for exclusion—P7 already modifies
   `bytecode.rs`, and fixing all three requires only one `VERSION` bump (4→5), making two separate
   bumps more expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → bumping the version → expanding the
   scope from “IR shape changes” to “artifact format.”~~ The versioning mechanism already exists
   (the `MAGIC` + `VERSION` header, with the reader rejecting mismatches), and `.42` is a build
   artifact for which cross-version compatibility is not a goal—the increase is in workload, not
   risk.
3. ~~The actual risk level of `2315` / `2341` is low, and running `.42` directly is a secondary
   path.~~ **Overturned**: losing the exception table makes throw/try behave incorrectly when `.42`
   is run directly—throw/try is a core language semantic. A secondary path is not the same as being
   allowed to fail silently. This refactor follows the principle of **leaving no unimplemented
   legacy in core functionality** (`Vec::new() // Not implemented yet` is exactly such a legacy).

**Also retained**: all three must still be registered in the `Obligations` ledger as “produced but
not serialized” until 7e is complete, to prevent them from becoming a fourth kind of “thing nobody
knows about.” `2312` is implemented together with the version bump (D17), and the already allocated
but unused opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Knock-on Effects of the IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                  |
| ------------------------------------------------------------------------- | --------------------------------------------- | ----------------------------------------------------------------------- |
| Add the `ValueId(u32)` type                                               | File header (`ValueId` definition)            | New (**implemented in 7b.1a**)                                          |
| Remove `Temp` / `Label` / `Register` from `Operand`, add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**implemented in 7b.1a**)               |
| Add a `Phi` variant to `Instruction`                                      | End of the `Instruction` enum (`Phi` variant) | **+1 variant** (**implemented in 7b.1a**; the enum now has 75 variants) |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | +1 field (pending 7b.1b)                                                |
| Add the `ValueInfo` structure                                             | `ValueInfo`                                   | New (pending 7b.1b)                                                     |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                    | Add `values()` / `values_mut()` accessors (pending 7b.1b)               |

The `pub use ...ast::Type` at the top of the file **is not modified by this article**—it belongs to
the scope of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **converged single
type representation**.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                        | Location                              | Nature                                                                                                    |
| ----------------------------------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or use the “expand to Move at runtime” strategy | `127-554` (currently **66** variants) | +1 or +0 variant (see below)                                                                              |
| Add a `Phi` arm to `opcode()`                                                 | `558`                                 | Mandatory (the `BytecodeInstr` match is exhaustive)                                                       |
| Add a `Phi` arm to `size()`                                                   | `649`                                 | Mandatory                                                                                                 |
| Add a `Phi` arm to the `From<BytecodeFile>` decoding match                    | `943-2350`                            | Mandatory                                                                                                 |
| Correct `upvalue_count: 0`                                                    | `2312`                                | Incidentally fixed by this article (in-memory format; persisting it requires a `VERSION` bump, see below) |

**The bytecode shape of `Phi` requires a design decision.** There are two options (**design
judgment: option A is recommended**, with reasons under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  selecting by predecessor block index. **No opcode is added**, and the `.42` format does not
  change. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode plus predecessor-index encoding, and have the interpreter maintain register
  versions on jumps. **This requires changing the `executor/` state model + the `.42` format + the
  version number.** The only benefit is size.

The decisive reason for choosing A is that changing the `.42` format triggers version-number issues
and a full compatibility discussion, while **A’s size cost is irrelevant in interpreted execution**
(the interpreter executes Moves, not Phis).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions distributed across `101-1558`)

| Change                                             | Location                                                                                                                         | Nature                                                                                              |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                       | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add an arm                                                                                          |
| `Instruction::Phi` translation arm                 | In the dispatch match, before `Free` (533)                                                                                       | New branch, expanded into a sequence of Moves (option A)                                            |
| Existing nine NOP degradations **remain in place** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified to be nine** in total: `655-657` covers two variants, and `658-660` covers four variants |

**SSA conversion does not reduce the number of NOPs.** These nine degradations reflect gaps in
expressiveness between “the IR has it” and “bytecode does not,” and SSA conversion does not touch
them.

#### `src/backends/interpreter/executor/`

With option A, `executor/` requires **no changes whatsoever**, because `Phi` disappears at the
`translator.rs` stage and is never visible to the interpreter. **This is the second decisive reason
for choosing A.**

`BytecodeInstr::Switch` at `executor/ops/control.rs:90` has an active implementation, but **no
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs`** (all three names have zero matches in
`ir.rs`)—**this is a reverse gap**: the bytecode layer has an instruction that the IR layer cannot
produce. It does not affect this article, but it is recorded here because it shows that “IR →
bytecode” is not surjective.

The dispatch match in `executor/debug.rs:194` likewise requires no changes (there is no new opcode
under option A).

### Compiler Change List

**File by file and function by function.** All line numbers are the measured starting points for the
changes:

| No. | File                                      | Location                                                | Change                                                                                                                         |
| --- | ----------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `src/middle/ir/ir.rs`                     | `Operand` enum definition                               | Add/remove `Operand` variants (**7b.1a implemented**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)              |
| 2   | `src/middle/ir/ir.rs`                     | End of the `Instruction` enum (`Phi` variant)           | Add `Instruction::Phi` (**7b.1a implemented**)                                                                                 |
| 3   | `src/middle/ir/ir.rs`                     | `FunctionBody::Code`                                    | Add `values`                                                                                                                   |
| 4   | `src/middle/ir/ir.rs`                     | `ValueInfo`                                             | Add `ValueInfo`                                                                                                                |
| 5   | `src/middle/ir/ir.rs`                     | `values()` / `values_mut()` accessors                   | Add accessors                                                                                                                  |
| 6   | `src/middle/ir/ir.rs`                     | `pub use ast::Type`                                     | **Leave `pub use ast::Type` unchanged** (scope of [03](03-type-unification.md); recorded only)                                 |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                               | Return `Operand::Value`, increasing monotonically without rollback                                                             |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                             | **Delete** statement-level reclamation                                                                                         |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                             | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                                            |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                             | **Delete** statement-level reclamation                                                                                         |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                               | Replace with an RAII guard (`generate_method_ir`, entry point `1677`)                                                          |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                               | Replace with an RAII guard (`generate_function_ir`, entry point `1830`)                                                        |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                         | Replace with an RAII guard (`generate_curry_innermost_func`, entry point `2105`)                                               |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                               | Replace with an RAII guard (`generate_curry_function_ir`, entry point `2209`)                                                  |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                               | Replace with an RAII guard (`generate_anon_binding_ir`, entry point `2759`)                                                    |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                               | Replace with an RAII guard (`generate_lambda_body_ir`, entry point `4678`)                                                     |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                             | Extract the E3014 check as `check_register_budget()`, shared by `1798`/`2001` after their semantics are unified                |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                                  | Change `take_cur_locals(param_types.len())` to `take_cur_locals(self.temp_high_water)`, eliminating the semantic inconsistency |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                               | Delete the comment-based contract (now enforced by the type system) and replace it with a pointer to `verify`                  |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                             | Change to call `synth::synth_closure`                                                                                          |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                             | Change to call `synth::synth_closure`; delete `6713-6714`                                                                      |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                             | Read `SynthEnv` from the parameter; no longer use `mem::take` on shared state                                                  |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                                    | Make `closure_captures` local; delete `clear()`                                                                                |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                                   | Change `closure_captures.contains_key` to use an explicit parameter (**the highest-priority implicit dependency**)             |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                          | Add span-keyed consumption counting                                                                                            |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                             | Make `method_def_ordinals` read-only                                                                                           |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                             | **Last**: split into a top-level `CallArgs` + six `emit_*` functions                                                           |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                             | Replace the “fill in 0” fallback with a diagnostic                                                                             |
| 29  | `src/middle/lower/synth.rs`               | **New**                                                 | Synthetic-AST boundary                                                                                                         |
| 30  | `src/middle/ir/verify.rs`                 | **New** (first layer of [07](07-equivalence-oracle.md)) | `verify_loose` / `verify_ssa`                                                                                                  |
| 31  | `src/middle/passes/codegen/translator.rs` | Dispatch match                                          | Add an `Instruction::Phi` arm (option A: expand into a sequence of Moves)                                                      |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                       | Add the `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`)                                                       |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                               | **Unchanged under option A** (`Phi` does not enter bytecode)                                                                   |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                              | **Unchanged under option A**                                                                                                   |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                                  | Correct `upvalue_count: 0`                                                                                                     |
| 36  | `src/backends/interpreter/executor/**`    | —                                                       | **No changes under option A**                                                                                                  |
| 37  | `src/middle/passes/regalloc.rs`           | **New**                                                 | Linear-scan allocator                                                                                                          |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New**                                                 | `synth.rs` boundary sentinel check                                                                                             |

### Backward Compatibility: Versioning the `.42` Format

**Verified fact**: the `.42` on-disk format **already has a versioning
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (“YXBC”) and
`VERSION: u32 = 4`, writes them to the header (`:342-343`), and validates them when reading
(`:454-467`, reporting “unsupported bytecode version” on a mismatch).

**Design judgment (a direct consequence of option A)**:

- `Phi` is expanded into a sequence of `Move` instructions at the `translator.rs` stage, **producing
  no new opcode**.
- The 83 constants in `opcode.rs` **do not change**, `opcode_name()` (`120`) remains unchanged, and
  the decoding match (`bytecode.rs:943-2350`) remains unchanged.
- The field layout of the `.42` format **does not change**, and `VERSION` remains 4. Old `.42` files
  can still be read by the new binary, and new `.42` files can still be read by the old binary.
  **SSA conversion itself does not require a version bump.**

**The only scenario requiring a version bump is the correction of `2312 upvalue_count`** (Decision
D17: fix it, and handle it together with the version bump): add an `upvalue_count` field to
`BytecodeFunction` and have the encoder write it. This is a **new field in the format**, so bump
`VERSION` from 4 to 5. The reader’s existing version check rejects old files according to its
current behavior—`.42` is a build artifact (`main.rs:671`), cross-version read compatibility is not
a goal, and no additional migration mechanism is required for the version bump. The already
allocated but unused opcode values (D32/D34’s `Switch` / `TailCall`) are reclaimed together with the
version bump.

## Implementation Notes

### Batch Strategy

Implement in four batches. **Each item within a batch gets an independent commit, an independent
criterion, and an independent revert.** In the table below, “change item” refers to the number in
the “Compiler Change List” above.

| Batch       | Contents                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Change items                 | Criterion                                                                                                                                                             | Rollback point                                                                                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Eliminate multiple definitions: delete three statement-level reclamations → replace six save/restore sites with an RAII guard → unify semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | 8-10 (first) → 11-16 → 17-18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and semantic unification are pure movements: **C1** zero-diff snapshots | One commit per item; `git revert` is sufficient                                                                                                                |
| **Batch b** | Switch to the SSA shape: `ir.rs` structural changes + change temporary-value production to `Operand::Value` (mechanical replacement) → add a `Phi` arm to `translator.rs` → linear-scan allocator. Within the batch, follow the WBS’s three-level task order: 7b.1a (`Operand` variant convergence + introduction of `Instruction::Phi`, **implemented**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields) → 7b.2 (change `next_temp_reg` to produce `Operand::Value`) → 7b.3 (`Phi` arm in `translator.rs`) → 7b.4 (linear-scan allocator + delete the three statement-level reclamations, in the same batch as 7a.1) | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + all-green `verify_ssa` + `.42` round-trip tests                                                                                      | The `ir.rs` changes **must occupy a dedicated commit** (the type changes affect compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary + sentinel script → span consumption counting → make `method_def_ordinals` read-only                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + the new script integrated into CI; the consumption count must produce zero new diagnostics (otherwise, a real mismatch exists)       | Independent commit                                                                                                                                             |
| **Batch d** | Split `generate_call_expr_ir` + convert the “fill in 0” fallback into an assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | 27-28                        | Pure-movement portion: **C1** zero-diff snapshots. Assertion portion: **C4** corpus differential + first-layer verifier                                               | Independent commit                                                                                                                                             |

**Three dependencies whose order cannot be exchanged:**

1. Batch a’s “delete reclamation” comes after the `ir.rs` changes in batch b—the three
   statement-level reclamations are deleted together with 7b.4 and the linear-scan allocator, with
   7a.1 executed at the same time. First make `next_temp` increase monotonically; only then does
   `ValueId` have the semantic basis for “unique definition” (that semantics becomes effective after
   7b.2 changes `next_temp_reg` to produce `Value` and 7b.3 implements the `Phi` arm).
2. Batch b’s `Phi` arm must precede the split in batch d—otherwise the split would carry the old
   `arg_regs` ordering assumptions into the new file unchanged.
3. Batch d must come last—it depends on the previous two batches having converged `arg_regs`
   semantics to “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within **P7** of RFC-039, and the criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial**, but there
are only ordering constraints among items 8-10 / 11-16 / 17-18 within batch a; they are not coupled.

### Hard Prerequisites

**Prerequisite One: Criterion Baseline.** The `verify_loose` mode of the first-layer verifier in
[07](07-equivalence-oracle.md) **must first pass on the existing, non-SSA IR** before batch a may
begin.

Reason: the criterion for SSA conversion is C4 (behavioral equivalence + invariants). If the
invariant verifier itself cannot pass on the existing IR before the refactor, every acceptance gate
for batches a–d lacks an executable criterion and can rely only on corpus differentials—yet corpus
differentials are **precisely insensitive** to Defect Classes 1/2/3 (as demonstrated under
“Alternative A” in 07). **This is a genuine ordering constraint.**

**Prerequisite Two: Convergence of Type Representations.** [03](03-type-unification.md) must
complete the convergence of `pub use ast::Type` at `ir.rs:3` and the removal of the
`From<MonoType> for IrType` bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` introduces a
third set of type-compatibility checks.**

### Rollback Strategy

- Each change gets an independent commit, with `git revert` granularity equal to change-item
  granularity.
- The `ir.rs` structural changes in batch b are the **only changes that cannot be rolled back
  incrementally** (type changes affect compilation throughout the repository). They must therefore
  **occupy a dedicated commit**, and batch a must already be green with a revertible baseline before
  that commit.
- **Do not use a feature flag.** Reason: adding/removing `Operand` variants cannot be isolated with
  a runtime switch; a feature flag would cause CI to stop testing the other path over the long
  term—a replay of the “test wiring depends on human memory, and corruption occurs silently” problem
  recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front; otherwise, the implementation will be challenged midway for “not
reducing the line count.”**

| Item                                      | Line-count change                                                                                                                                                                   |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                            | `ir_gen.rs` **8,448**                                                                                                                                                               |
| Delete three statement-level reclamations | **−15**                                                                                                                                                                             |
| Replace six save/restore sites with RAII  | Net ±0 (approximately +20 lines for the RAII guard definition, with 22 lines of use sites reduced to approximately 12)                                                              |
| Change `next_temp_reg`                    | +5                                                                                                                                                                                  |
| Split `generate_call_expr_ir`             | **+40–80** (overhead from the signatures and parameter passing of six `emit_*` functions; this is the typical cost of a pure-movement split—**splitting increases the line count**) |
| Span-keyed consumption counting           | +15                                                                                                                                                                                 |
| Make `method_def_ordinals` read-only      | +20                                                                                                                                                                                 |
| New `synth.rs` file                       | +80–120                                                                                                                                                                             |
| New `verify.rs` file                      | +400–600 (the first layer of 07, although it is within this article’s scope, is counted independently)                                                                              |
| New `regalloc.rs` file                    | **+500–1,000**                                                                                                                                                                      |
| **Total**                                 | **`ir_gen.rs` grows from approximately 8,448 to 8,600–8,800; including the new files, the net increase is 900–1,600 lines**                                                         |

The early expectation of “8,448 → 5,000–6,000” was **incorrect**; the measured projection points in
the opposite direction. The reason must be explained:

1. **The core benefit of this article is not a reduction in line count; it is the elimination of an
   entire class of “incorrect without reporting an error” defects.** The six manual save/restore
   sites (22 lines) become approximately 12 lines plus a guard type after SSA + RAII—**the line
   count is almost unchanged, but “forgot to restore” changes from a possibility into a compile
   error**.
2. **A pure-movement split necessarily increases the line count.** Splitting the 746-line
   `generate_call_expr_ir` into seven functions duplicates the signature (the function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` binding, and error propagation.
   **“Splitting into files reduces the line count” is a misconception—it reduces cognitive load, not
   physical lines.**
3. **The new verifier and allocator increase the line count.** These two additions—approximately
   900–1,600 lines—are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of lines, this article will most likely produce a **net increase**. The benefit lies in
> **verifiability**—turning “incorrect without reporting an error” defects into invariants that fail
> the build. If line count is used as the acceptance criterion, this article will fail; the correct
> acceptance criteria are those in “Batch Strategy.”

**This stage will intentionally increase the amount of code in `ir_gen.rs`.** The 2026-10-03
decision canceled all line-count/size gates (see Prohibition Two in
[08](08-maintenance-mechanism.md)), so **no line-count exemption list is needed**. The legitimacy of
the increase is established through the separation-of-responsibilities argument: SSA conversion
separates “register allocation + definition discipline” from lowering into an independent module,
and the increase in code buys the elimination of two classes of manual discipline—“misalignment
without reporting an error or only producing an incorrect value”—covering six manual save/restore
sites and the semantic reordering of `arg_regs`.

## Key Decisions and Rationale

### Three Core Decisions

**Decision One: Temporary-Value Strategy—delete the three statement-level rollbacks so that unique
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, so every temporary slot naturally has exactly one definition point.
Differences such as **“the criterion is consistent, but the guards differ”** (`3632` has an
`result_reg.is_none()` guard while the other two do not) disappear as well. The cost is higher slot
occupancy—the existing `temp_high_water` upper-limit check (`1992-2000`) provides a fallback.

**Decision Two: Do not perform mem2reg; perform explicit renaming only.** The rationale is given
under “Value Identification Method”: YaoXiang’s `&` / reference / ownership semantics make “which
slots can be promoted” a type-system problem that requires the type representation converged in
[03](03-type-unification.md). **Performing mem2reg before 03 is guessing.** Explicit renaming only
changes the return-value semantics of one function (`next_temp_reg`) and deletes three rollbacks; it
does not touch the control flow of any lowering algorithm.

**Decision Three: Use Option A for `Phi` (an IR-only pseudo-instruction expanded into a sequence of
Moves).** `executor/` and the `.42` format undergo **no changes**, limiting the regression surface
to L3. The decisive reason is that changing the `.42` format triggers version-number issues and a
full compatibility discussion, while A’s size cost is irrelevant under interpreted execution (the
interpreter executes Moves, not Phis). The cost is that `Phi` is not visible in `dump_bytecode`
(this must be taken into account when entering the C1/C2 stage, where the third layer in 07 must be
compared).

### Rejected Alternatives

**A. Perform a one-time full split (SSA + split + `synth` boundary + allocator simultaneously).
Rejected.** This violates the core principle in 07: criteria must be tiered by category. Mixing all
four batches into one PR mixes pure-movement changes with C4 changes, making snapshot drift
impossible to attribute—if the snapshot changes, it is impossible to tell whether the split moved
something incorrectly or SSA changed it incorrectly. **The rollback granularity also degrades from
“change item” to “everything.”**

**B. Split files first, then optimize ordering. Rejected.** Splitting eliminates no root cause: the
six manual save/restore sites remain scattered across 10 files, and the semantic reordering of
`arg_regs` still requires manual reasoning. Worse, **splitting first loses the opportunity to
“establish the criterion before changing anything”**—the split produces a large line-level diff that
mixes into the diff of subsequent SSA changes, invalidating review. This has the same origin as the
eight orphan test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests that never ran):
**if the order is wrong, every subsequent step accelerates from an incorrect foundation.**

**C. Introduce an intermediate SSA layer without changing the existing lowering. Rejected, but
recorded as “a third path that was once considered.”** The shape is to leave `ir_gen.rs` unchanged
and add a pass that converts its non-SSA IR into SSA IR. Its **advantages** are that lowering
remains untouched and rollback is easy. **Reasons for rejection**: (1) It does not eliminate the
root cause of Defect Class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the
six save/restore sites remain separated by 59–161 lines. A conversion pass can merge multiple
definitions into `Phi`, but **it cannot fix “an inner function’s name leaks into the outer
function”**—that is a `cur_locals` save/restore error that occurs before the IR shape and is
therefore invisible to the conversion pass. (2) It turns SSA into a second representation—the
repository already has the precedent of three parallel type representations, and the core lesson of
this project is that **“boundaries exist only in human awareness.”** (3) `verify_loose` becomes a
permanent burden.

**D. Keep the current state and only add a verifier. Rejected as the sole solution, but it is a
component of this article.** Alternative B in RFC-039 reaches the same conclusion: a gate can
prevent regressions but cannot repair the current state. **What D can do must nevertheless be stated
clearly**: D can **catch instances** of Defect Classes 1/2/3 (provided that the verifier covers
dominance, type consistency, and inner isolation), but it cannot catch Defect Class 4
(`closure_captures` leakage), which requires a cross-function liveness assertion beyond the boundary
of `verify(&ModuleIR)`. **Therefore, D is not an alternative to this article; it is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not the risk
  point**—the 2026-10-03 decision canceled line-count/size gates, and the scale issue is addressed
  through separation of responsibilities (see Prohibition Two in [08](08-maintenance-mechanism.md)).
  The increase is justified by replacing two classes of manual discipline—“misalignment without
  reporting an error or only producing an incorrect value”—covering six manual save/restore sites
  and the semantic reordering of `arg_regs`.
- **The “unique definition” check in `verify.rs` is meaningless in the non-SSA shape**
  (`Operand::Local` permits multiple definitions). This limitation is recorded in 07, and
  `verify_loose` is its degraded mode before this batch.
- **Deleting statement-level reclamation changes the temporary-slot allocation pattern** (executed
  in batch b, with 7a.1 and 7b.4 in the same batch). Even if `temp_high_water` prevents overflow,
  **the allocation result changes**—which is why the C4-stage “behavioral equivalence” criterion
  must genuinely run over the full corpus rather than relying on sampling.
- **The `u8` / 255 slot limit is insufficient after deleting rollback (verified)**: after measuring
  with all three reclamations removed across the full corpus, two corpus entries crossed 255 and
  triggered E3014—`src/std/tests/string_ops.yx` with 354 slots and
  `tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; the peak slot count for the
  remaining corpus entries rose from 51 to 211. `take_body_locals` in `src/middle/lower/scope.rs`
  catches out-of-range cases, but catching them means compilation fails. Deleting rollback therefore
  comes after the allocator (7b.4): first let the allocator assume responsibility for slot reuse,
  then delete rollback.
- **The benefit must also be recorded**: turning “errors that cannot be tested” into “invariants
  that can be tested”; no backend changes (`executor/` and `.42` remain unchanged); no changes to
  the semantics of `Instruction` (all 76 → 77 variants are retained, and all 50 `translate_*`
  functions are retained); incremental implementation (each change can be reverted independently).

> **All open questions originally listed in this section have been adjudicated.** The decision for
> each item is recorded in
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> article leaves no unresolved items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching framework; four-layer model, criterion tiers, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definition of the C4
  category, `verify_loose` / `verify_ssa`, three-layer criteria, and the limitation that snapshots
  are immune to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of three
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the `PlanId`
  conversion for `ReleasePlan`
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — eight orphan test trees; the counterexample
  of “creating the directory first and then failing to wire it up”

### Code Locations

**IR Definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — `Operand`, 7 variants
- `src/middle/core/ir.rs:47-533` — `Instruction`, 76 variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (with `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (with `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (with `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (the feasibility basis for the
  `PlanId` design)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect Class 1 (Multiple Definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for the seven nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `take_body_locals` in `src/middle/lower/scope.rs` — sole table-extraction and E3014 budget point
  (`MAX_REGISTERS = 255`)
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `take_body_locals`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the true high-water mark in
  `temp_high_water`
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — three statement-level register
  reclamations
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant
  referenced in 07

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  one clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill in 0” fallback

**Defect Class 3 (Span Keying)** — the following are **evidence from the time of the audit** (after
the 4.5 key migration, both `release_plan` and `overload_resolutions` keys have been converted to
`PlanId`; current pointers are in the revision note in [02](02-stage-contract.md) §2;
`method_def_ordinals` remains the mechanism-three TODO)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write point of
  `method_def_ordinals` (the key is the unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the propagation structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — definition and
  production of `ReleasePlan`
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer propagation
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (Implicit Ordering Dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — the equivalent handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — the two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — three other `ast::Type` construction sites

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr`, 66 variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded discards (compare `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge awaiting removal in
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — nine NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  limit of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current Test Coverage**

- `src/middle/core/ir_gen.rs` — 8,448 lines, 0 `#[cfg(test)]` blocks, 0 `mod tests` modules
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, 29 tests in
  total; `def_assign` / `local_slots` positively pin DefId and slot naming; no IR structural
  invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — four references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example of a per-opcode round-trip criterion; the last test in the file; the file contains 1,209
  lines / 23 `#[test]` functions)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies consist only of `criterion` / `proptest`; `[dependencies]`
  contains no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans the `/rfc/draft`
  directory; this article is not under that directory and is referenced by RFC-039 and the index for
  this directory
