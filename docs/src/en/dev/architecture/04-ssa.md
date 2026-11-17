# SSA Transformation of the Intermediate Representation

> **Companion design document.** This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria categorization, and execution-phase order are covered in the
> main RFC-039 text; the positioning of each companion document is covered in the
> [index of this directory](index.md).

## Scope and Positioning

This document addresses the construction discipline for IR at the L3 layer. It proposes a
construction method based on “a single definition for every value” for `src/middle/core/ir_gen.rs`
(8,448 lines). **The goal is not to introduce a new instruction set, but to eliminate three classes
of discipline defects that produce no errors when they fail**:

1. Six manual register save/restore operations separated from their corresponding code by dozens of
   lines;
2. Semantic reordering of `arg_regs` shared by six branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, explicit cross-layer contracts, decomposition of
call-expression dispatch, and how the IR static verifier is wired in.

This document is not responsible for: convergence of type representations
([03](03-type-unification.md)), stage contracts and the obligation ledger
([02](02-stage-contract.md)), definitions and gates for equivalence criteria
([07](07-equivalence-oracle.md)), frontend paradigms ([05](05-frontend-paradigm.md)), or dead-code
and placeholder-design cleanup ([06](06-cleanup-inventory.md)). This document repeatedly references
the conclusions of those documents without repeating their arguments.

**The criteria category** follows the **C4 (IR shape changes)** category defined in
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, without requiring exact equality of IR snapshots. SSA transformation necessarily
changes the IR shape; insisting on snapshot equality would encourage the team to weaken the
criteria, which is precisely the trap recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, accepted on behavioral equivalence + a green `verify_ssa`). The implementation
batch numbers within this document (batches a–d) are distinct from the ordering within P7: the
former are this document’s incremental strategy, while the latter are RFC-039’s global phases.

## Feasibility Prerequisites

**Verified facts.** The IR already possesses **all structural prerequisites** required to construct
SSA:

| SSA prerequisite                 | Current repository state                                                                                         | Location                                          |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| Explicit CFG                     | `BasicBlock { label, instructions, successors }`                                                                 | `BasicBlock` in `src/middle/ir/ir.rs`             |
| Explicit entry block             | `FunctionBody::Code { blocks, entry, locals }`                                                                   | `FunctionBody` in `src/middle/ir/ir.rs`           |
| Scoped nesting information       | `LocalSlot::scope_depth` (“0 = function-parameter layer, used to disambiguate duplicate names in nested scopes”) | `LocalSlot` in `src/middle/ir/ir.rs`              |
| Three-address form               | 76 variants, 54 with `dst: Operand`, and 3 with `dst: Option<Operand>`                                           | `Instruction` in `src/middle/ir/ir.rs`            |
| Span on every instruction        | Every variant has a `span: Span` field                                                                           | Same as above                                     |
| Annotatable value types          | `LocalSlot::ty: MonoType`                                                                                        | `LocalSlot` in `src/middle/ir/ir.rs`              |
| Upper-bound check for slot count | Table extraction and E3014 budget gate share a single point (`MAX_REGISTERS = 255`)                              | `take_body_locals` in `src/middle/lower/scope.rs` |

**If these prerequisites were absent, the cost estimates in this document would be completely
different.**

**The two decisive points are as follows:**

**First, SSA transformation is not a new capability; it removes a capability dependency.** In the
current implementation, temporary slots are **deliberately designed to be defined multiple
times**—`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`, explicitly roll back `next_temp` in
three places. The first step of SSA transformation is to **delete these three rollbacks**, making
`next_temp` increase monotonically. After deletion, **every temporary slot naturally has exactly one
definition point**—unique definition changes from “a rule that must be checked” into “a mathematical
consequence of the construction method.” Batch a only converged the save/restore surface (see
“Defect Class 1”); deleting the rollbacks is performed together with the allocator in batch b.

**Second, the upper bound on slot count is a hard check, and deleting the rollbacks makes the slot
count unbounded (measured).** `take_body_locals` in `src/middle/lower/scope.rs` extracts the table
according to `temp_high_water` and reports E3014 when it exceeds 255. After deleting the three
reclamation points and running the full corpus, two corpus entries exceed the limit and trigger
E3014—`src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; the peak slot count for all
other corpus entries rises from 51 to 211. `temp_high_water` records the historical maximum
occupancy and is therefore unaffected by the rollbacks themselves; however, once the rollbacks
disappear, the allocator no longer reuses slots, causing the peak to rise.

**The combined meaning of these two points is:** lowering already maintains all the bookkeeping
information required for SSA; it merely “recycles” that information. Deleting the rollbacks must
wait until the allocator (7b.4) can perform slot reuse. The main workload of SSA transformation lies
in decomposing `generate_call_expr_ir` and making cross-layer contracts explicit, not in building
new IR capabilities.

### Prerequisite: Unification of Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The third representation is `ir::Type` on the serialization side,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR contains two type representations simultaneously, and one of them directly
`pub use`s the AST type.**

SSA transformation must add a `Phi` variant, record a type for every SSA value (for the “type
consistency” invariant in 07’s verifier), and determine whether the types of two predecessors at a
CFG join are compatible. **If type representations have not yet converged, SSA will require a
separate type-compatibility implementation for each representation**—and the third representation
will grow accordingly.

This is not a question of “which should be done first,” but of **which order would produce three
sets of type-compatibility logic**. [03 Type Representation Unification](03-type-unification.md)
must first complete the convergence of `pub use ast::Type` at `ir.rs:3` and the removal of the
bridge at `bytecode.rs:2353`. Item 6 in this document’s change list explicitly records that
`ir.rs:3` **is not modified by this document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not “ugly but functional.” It is a file that **places correctness on the burden of
human line-by-line review**:

- The file contains **zero `#[cfg(test)]` blocks and zero `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares three modules—`bytecode` / `def_assign` /
  `local_slots`—with 29 tests in total. Of these, `def_assign.rs` and `local_slots.rs`
  **positively** cover ir_gen’s DefId allocation and local-slot naming at the sentinel level, while
  all four references to ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **negative
  assertions**—“ir_gen does not produce…”, “ir_gen also does not construct…”, and “the ir_gen
  frontend does not construct the corresponding IR”.

In other words: ir_gen has 29 sentinel-level tests pinning local behavior, but **no IR
structural-invariant validation**—no dominance, unique-definition, jump-target, or type-consistency
checks. None of the three modules declared by `middle/core/tests/mod.rs` makes positive assertions
about the overall IR shape.

This file also happens to contain several classes of defects that **compile successfully, pass every
check, and let the program continue running with incorrect values**. Their common property is:
**failure produces no error**.

This is where the highest risk lies. The problem is not “the function is too long” or “the naming is
confusing”; it is that **the file’s core invariants have no machine-executable enforcement
mechanism**.

The contract “save before generating a nested function body, then restore it afterward; otherwise
names from the inner function will leak into the outer function” is enforced by the type system:
`NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator,
and frame capture and restoration each have exactly one implementation (see “Defect Class 1”).

**Why adding a verifier alone is not enough**: a verifier can **catch** these defects, but it cannot
catch their **causes**. The cause is not “someone overlooked them during review,” but rather:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator actively depends on this property.**

Concrete evidence is the three statement-level register reclamations—`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`. At the end of every statement, they **roll back** `next_temp` to
the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and both the
number and order of writes and the live ranges are determined entirely by the single manual decision
of **when to roll back**.

Under this shape, “unique definition” is not a rule that can be applied incrementally; it **directly
conflicts with the register-allocation strategy**. SSA transformation is therefore not a matter of
adding a check to the existing code; it is a matter of **replacing the allocation strategy that
depends on multiple definitions**. The good news is that the replacement cost is low—see
“Feasibility Prerequisites” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (Batch a Converged the Restore Surface)

**Verified fact.** There are seven entry points that require save/restore when entering a nested
function body. Batch a (7a.2) converged them into an RAII guard: `NestedBodyGuard<'a>` in
`src/middle/lower/scope.rs` (`Deref`/`DerefMut` to the generator, with unconditional restoration in
`Drop` and `#[must_use]`).

| Entry function                       | File                               | Guard form                    | Restore scope                                       |
| ------------------------------------ | ---------------------------------- | ----------------------------- | --------------------------------------------------- |
| `generate_method_ir`                 | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Slot table + span (three counters are not restored) |
| `generate_function_ir`               | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter_decl` | Same as above                                       |
| Nested-function definition call site | `src/middle/lower/stmt.rs`         | `NestedBodyGuard::enter`      | Slot table + span + three counters                  |
| `generate_lambda_body_ir`            | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_anon_binding_ir`           | `src/middle/lower/globals.rs`      | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_innermost_func`      | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |
| `generate_curry_function_ir`         | `src/middle/lower/curry.rs`        | `NestedBodyGuard::enter`      | Same as above                                       |

- Frame capture occurs only in the two constructors (`enter` / `enter_decl`), and restoration occurs
  only in `Drop`. The guard exclusively holds `&mut` access to the generator for its lifetime,
  leaving no second `&mut` available in the scope; all `?`/early-return paths restore through
  `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span`
  / `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`. `next_temp_reg` continues
  to maintain `temp_high_water` as the true high-water mark; statement-level reclamation rolls back
  `next_temp`, while the total number of slots is based on historical peak occupancy.
- `enter_decl` does not restore the three counters: after a function body/method body returns to the
  module-initialization flow, subsequent definitions continue from the child body’s final
  identifier. This is the C1 zero-drift snapshot constraint and the meaning of “the declaration
  surface restores only the table.”

**Unified accounting and a single budget point (7a.3 landed)**: `take_body_locals` in
`src/middle/lower/scope.rs` is the sole table-extraction and budget point. It truncates according to
`temp_high_water`, reports E3014 with the function-body span when the limit is exceeded, and defines
`MAX_REGISTERS = 255` in exactly one place in the repository. All six function-body shapes—function
/ method / lambda / anonymous binding / curry intermediate layer / curry innermost layer—extract
their tables through it. There is no second definition of “how many slots a function body should
have.”

**The three duplicated statement-level register reclamations still remain (7a.1 separated them into
batch a and moved them to batch b, together with 7b.4)**:

| Location                           | Enclosing function        | Guard                     |
| ---------------------------------- | ------------------------- | ------------------------- |
| `src/middle/lower/stmt.rs`         | `generate_function_ir`    | None                      |
| `src/middle/lower/stmt.rs`         | `generate_block_ir`       | `if result_reg.is_none()` |
| `src/middle/lower/expr/closure.rs` | `generate_lambda_body_ir` | None                      |

The guard in `generate_block_ir` has a clear rationale: the block occupies an expression operand
slot, and the outer scope may hold sibling argument temporaries that live across the block. The
other two locations have no equivalent guard. **The same criterion is copied by hand in three
places, with one location guarded and two unguarded**—this kind of discrepancy is precisely what SSA
is intended to eliminate.

**Why it was separated**: deleting the reclamations makes the slot count unbounded (see “Feasibility
Prerequisites,” item two, and “Known Limitations and Risks”), so a linear-scan allocator (7b.4) must
first take responsibility for slot reuse.

**Consequence in the current shape**: the guard exclusively holds `&mut`, and restoration has only
one implementation in `Drop`, leaving no writable location where a restore could be omitted. What
remains is “one slot, multiple definitions” itself—the allocator depends on it, so the I-layer
invariant cannot assert unique definition until batch b deletes the reclamation.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified fact.** `generate_call_expr_ir` is located at `ir_gen.rs:7269-8014` and is a **single
746-line function**. Its signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (along with `_expr` / `result_reg` / `instructions`
/ `constants`). Internally, six branches share the same `Vec<Operand>` and perform **semantic
rewriting**:

| Line        | Semantics                                                                                                                                                |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten according to `slots`, then recollect—named-argument reordering                                                                   |
| `7724`      | Struct construction: unwrap each slot according to `final_args`, then recollect—field reordering                                                         |
| `7835-7848` | Function call: named-argument reordering; `7838-7846` **adds a new register with value 0** for each missing slot (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—clone before emitting instructions                                                                    |

The original form of the four reorderings is as follows:

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

The comment at `7835-7848` states: “Uncovered slots: remain absent, and the type-checking stage
reports E1010. For runtime safety, fill in 0 (the normal path should never reach here).”

**This design carries two risks**:

1. **Misalignment produces neither an error nor a panic; it only produces an incorrect value.** Each
   of the six branches independently determines the final order of `arg_regs`, and any incorrect
   ordering assumption causes no panic.
2. **Normalized snapshots are immune to this defect.** 07 already records this limitation: after the
   snapshot tool renames temporary values by “first-occurrence order,” “the third argument should
   have used the fifth register” is exactly the same in the snapshot. **This class of defect does
   not even have snapshot-level criteria at the C4 stage.**

**Supplementary verification: the struct-construction path’s reordering has no duplicate-slot
guard.** At `7724`, `final_args` is populated by “placing positional arguments first, then
overwriting by field name for named arguments.” **Before overwriting, there is no duplicate-slot
check such as `slots[idx].is_some()`**—unlike the namespace path at `7381`. When a named argument
silently replaces a positional argument, there is neither a panic nor a diagnostic; moreover, the
argument **evaluation order** (source order) diverges from the final `arg_regs` order
(field-declaration order). Batch d’s assertion-based `fill_missing` must also cover the
duplicate-slot check; otherwise, the struct-construction path’s silent replacement will survive
unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified facts at the audit point.** The repository has **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                   | Production                                                                                                            | Propagation                                                                               | Consumption                                                                                                                 |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                     | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatches `walk_stmt` by statement identity)   | `typecheck/types.rs:38` ← propagated by `checker.rs` ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, sole entry point; settlement-completed marker `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger unified, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; `inference/statements.rs` only propagates and merges) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held by the `ir_gen.rs` field       | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; settlement-completed marker `:1097`) |

The shape at the audit point (`ir_gen.rs:1942`, **now nonexistent**) was:

```rust
// NLL Release: insert Drop instructions at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some`—**if the lookup fails, it simply fails: there is no else branch, no assertion, and no
counter**. If the two sides compute spans differently in any respect, the **Drop instruction
silently disappears, with zero errors**. The Drop sequence for refinement types is a central carrier
of this project’s ownership semantics (`ownership.rs:26-32` describes it as the “NLL precise release
plan”), and its implementation was once silent.

`7420` / `7493` use the same pattern (`if let Some(mangled) = self.overload_resolutions.get(span)`),
causing overload resolution to fall back to the default binding.

**Disposition (landed in 4.5):** both keys were changed to the structural identity `PlanId`, and
consumption is **accounted for on a hit** (reading and accounting occur in the same call). A
nonempty difference is reported as an **E-level `W1090`** at the sole settlement point—“not found”
is no longer silent. The implemented shape and criteria are documented in the §2 revision note in
[02](02-stage-contract.md) (4.5). These line numbers (`1942` / `7420` / `7493`) are retained as
audit-point evidence and are not pointers to the current implementation.

> **`method_def_ordinals` is not span-keyed.** It is a `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with its sole write point at `:1692-1702` (assigning `#N` suffixes in
> `base_name` definition order). Its risk is **different**: definition-order dependence. The `#N`
> suffix is determined by AST definition order, so if generation order differs from typechecker
> registration order, the same function name may resolve to a different target. This is not a span
> mismatch, but it likewise “fails without reporting an error.” Its disposition is mechanism three
> in “Additional Mechanisms for Span-Keyed Problems.”

### Defect Class 4: Implicit spawn/lambda Handshakes

**Verified fact.** `ir_gen.rs` has **two** locations that construct `ast::Expr::Lambda` themselves
and call back into `generate_expr_ir`, relying on a set of mutable fields as a handshake:

**Handshake A — `generate_spawn_for_ir` (entry point `4143`):**

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir` (entry point `6543`) → `6571-6572` consumes it
through `std::mem::take(&mut self.pending_env_vars)`.

**Handshake B — `generate_spawn_expr_ir` (entry point `6641`):**

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**Handshake C — `closure_captures`**: written at `6589-6593`, which is **in the same function** that
calls `generate_lambda_body_ir` at `6598` (entry point `4678`), and cleared at `6600`. It has two
read points: `:6284` (`LoadUpvalue` inside a closure body) and **`:458`**
(`closure_captures.contains_key(head)` on the `flatten_namespace` path).

The `:458` read point is the most dangerous: it is on the **namespace-resolution** path, meaning
that the live state of a `HashMap<String, usize>` can change variable-name resolution. A complete
closure-body generation (`6598`) occurs between the write at `6589` and the clear at `6600`. If any
path inside `generate_lambda_body_ir` returns early without clearing the table—there are multiple
return points that propagate errors through the `?` operator—**`closure_captures` will remain in the
unrelated code that follows at `:458`**.

Similar implicit state includes `pending_env_names` (written by handshake B at `6699`, cleared at
`6714`).

### What SSA Will Not Solve (Stated Honestly)

| Problem                                              | Location                                                                                                                                         | Why SSA does not solve it                                                                                                                                                                                                                 |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Complexity of `compile_pattern`                      | `ir_gen.rs:5390-5687`, **298 lines**, with four levels of self-recursion (`5473` / `5504` / `5619` / `5640`) and an external call site at `5270` | The dispatch complexity of pattern matching (union payload / tuple / struct / literal / nested) is **inherent to the algorithm**. SSA only turns the `JmpIfNot` produced by a guard into a `Phi`; it does not reduce the number of guards |
| Complexity of `eval_const_expr`                      | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)                          | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                                           |
| Span-keyed failure (**eliminated**)                  | Audit point: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)—after 4.5, `PlanId` keys + mandatory settlement                   | **SSA does not touch this layer at all** (the conclusion remains unchanged). It was solved independently by the obligation-ledger mechanism in [02](02-stage-contract.md); see mechanism two in “Disposition of the Span-Keyed Problem”   |
| Number of `Instruction` variants                     | 76 (`ir.rs:47-533`)                                                                                                                              | SSA adds `Phi`, a **net change of +1**. The 76 variants correspond to 76 bytecode opcodes or are lowered to NOP; SSA will not reduce this number                                                                                          |
| Definition-order dependence of `method_def_ordinals` | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                                            | Similar to span keying but caused by a different factor; see mechanism three in “Additional Mechanisms for Span-Keyed Problems”                                                                                                           |

## Target Design

### SSA Representation Definition

**Design judgment.** The target shape is as follows.

#### How the `Operand` Variants Change

The current `ir.rs:12-20` has seven variants. **Measured construction sites across the repository**:

| Variant             | Production construction sites                                                                                                                                  | Disposition                                                                                       |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 8 files                                                                                                                                                        | **Retained**, unchanged                                                                           |
| `Local(usize)`      | 7 files                                                                                                                                                        | **Retained, but its semantics are split** (see below)                                             |
| `Arg(usize)`        | 4 files                                                                                                                                                        | **Retained** as the implicit definition at function entry (parameters are definitions in block 0) |
| `Global(usize)`     | 5 files                                                                                                                                                        | **Retained**, unchanged                                                                           |
| `Temp(usize)`       | **0 construction sites** (handled only at `codegen/operand.rs:39-45` and `:75`)                                                                                | **Deleted** (dead variant)                                                                        |
| `Label(usize)`      | **0 construction sites**                                                                                                                                       | **Deleted** (dead variant)                                                                        |
| `Register(u8)`      | **0 construction sites** (`ir.rs:19` says “Added for codegen”, but codegen uses `OperandResolver` in `codegen/operand.rs` and does not construct this variant) | **Deleted** (dead variant)                                                                        |

`Local(usize)` is split into two variants because it currently **simultaneously serves two mutually
exclusive semantics**—“the storage location of a named variable” (which may be written multiple
times) and “a compiler temporary register” (which should be written exactly once):

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
densely numbered within a function, **increasing monotonically and never rolled back**.

#### `Phi` as a New `Instruction` Variant

```rust
/// Join point: each predecessor basic block provides one definition of this value
/// incoming is ordered by ascending predecessor block label (guaranteed at construction, for normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints, all enforced by `verify_ssa` (see the first layer in
[07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set in `incoming` equals the set of labels of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. All `incoming` values have the same `MonoType` (**depends on the type representation from 03
   having converged**).

**Disposition of the guard at `3632-3638`**: the current reclamation in `generate_block_ir` has an
`if result_reg.is_none()` guard (the comment at `3626-3631` explains: the block occupies an
expression operand slot, and the outer scope may hold sibling argument temporaries that live across
the block). After SSA transformation, this guard is **no longer needed**—temporary values are no
longer rolled back, and sibling arguments that live across the block hold their own `ValueId`, which
cannot be affected by a rollback. **This is a place where SSA transformation genuinely simplifies
the code, rather than merely renaming it.**

#### Value Identification Method

**Design judgment**: do not perform memory promotion (mem2reg), do not introduce virtual-register
numbering, and **perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots can be promoted to memory” a
  **type-system problem**, not a CFG problem. It has been verified that `ir.rs:656`’s
  `LocalSlot::ty: MonoType` carries complete type information, but determining “whether this
  `MonoType` can be promoted to memory” requires the converged type representation from 03 and
  coordination with refinement-type rules. **Performing mem2reg before 03 would be guessing.**
- Explicit renaming only requires changing the return-value semantics of `next_temp_reg()` in one
  place (`ir_gen.rs:884-891`) and deleting the three rollbacks. **It does not touch the control flow
  of any lowering algorithm.**

The value-table shape adds `values: Vec<ValueInfo>` to `FunctionBody::Code`, where:

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block containing the definition
}
```

`values` and `locals` coexist: `locals` carries addressable named variables (which may be defined
multiple times), while `values` carries single-definition SSA values. **The presence of `locals`
does not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

#### Change-Size Estimate

| Item                                                   | Estimate               | Basis                                                                                                                               |
| ------------------------------------------------------ | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Change `next_temp_reg`                                 | ~15 lines              | Single function at `ir_gen.rs:884-891`                                                                                              |
| Delete three statement-level reclamations              | −15 lines              | `1954-1959` / `3632-3638` / `4737-4742`                                                                                             |
| Consolidate six save/restore sites into RAII           | Net ±0 lines           | See “Per-Item Disposition Table”                                                                                                    |
| Change `ir.rs` structures                              | +120–180 lines         | `ValueId` / `Phi` / `ValueInfo` / value table                                                                                       |
| Add a `Phi` arm to `translator.rs`                     | +25–40 lines           | One of the existing 50 `translate_*` functions                                                                                      |
| Add a `PHI` instruction or runtime fill in `executor/` | +60–120 lines          | Depends on the bytecode representation selected for `Phi`                                                                           |
| Rename `Operand::Local` → `Value` at call sites        | Mechanical replacement | **Measured**: `Operand::Local(` occurs **323 times** in `ir_gen.rs`, with **355 occurrences across 6 files** in the full repository |

### Per-Item Disposition Table

| Defect class                                                | Eliminated by SSA?                                                       | Disposition                                                                                                                                                                                                                                                                                                                                                                                               | Criteria                                                                 |
| ----------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| **1. Multiple definitions and manual save/restore**         | **Partially (restore surface converged; unique definition pending SSA)** | The seven nested-function-body entry points have been consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the three counters), turning “forgot to restore” into a compile error. The three statement-level reclamations remain until batch b (7a.1 and 7b.4 in the same batch), and `next_temp` can still be rolled back | 07 first-layer “inner isolation” invariant + `verify_loose`/`verify_ssa` |
| **1b. Inconsistent accounting**                             | **Yes (landed)**                                                         | Unified under `take_body_locals` in `src/middle/lower/scope.rs`: truncation according to `temp_high_water`, with the E3014 budget check at the same point as table extraction and carrying the function-body span; all six function-body shapes are covered; `MAX_REGISTERS` is defined in exactly one place in the repository                                                                            | Corpus differential + zero-diff snapshot (empirically verified)          |
| **1c. Criteria for the three statement-level reclamations** | **Yes (pending batch b)**                                                | The three reclamations still exist and are deleted in batch b together with 7b.4 (first, the allocator takes responsibility for slot reuse); the `result_reg.is_none()` guard in `generate_block_ir` is also deleted                                                                                                                                                                                      | C4: behavioral equivalence + invariants                                  |
| **2. Semantic reordering of `arg_regs`**                    | **Partially**                                                            | SSA turns “using the wrong register” into “using the wrong value ID,” which dominance checking can catch, but **the reordering logic itself remains**. Actual elimination requires decomposing the function—each of the six branches performs `push`, while ownership of `arg_regs` remains at the top of the function                                                                                    | First-layer dominance invariant (snapshots are immune)                   |
| **3. Span-keyed failure**                                   | **No**                                                                   | SSA does not touch it at all. See “Additional Mechanisms for Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                         | 07’s `test_release_plan_spans_consumed`                                  |
| **4. Implicit ordering dependency**                         | **No**                                                                   | See the “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                             | A new explicit criterion is required                                     |
| 298-line `compile_pattern`                                  | No                                                                       | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                             | —                                                                        |
| 191-line `eval_const_expr`                                  | No                                                                       | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                             | —                                                                        |
| 76 `Instruction` variants                                   | No (+1)                                                                  | Adding `Phi` → 77                                                                                                                                                                                                                                                                                                                                                                                         | —                                                                        |

### Register Allocator: Custom Implementation or Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` (`ir.rs:19`) has **0 construction sites across the repository**.
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local/Temp/Arg` to `u8`, with an upper limit of 255 (the `Temp` arm at `39-45`
   reports E3014 on overflow).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50
   `translate_*` functions, with function bodies distributed across `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`. Opcodes come from `src/backends/common/opcode.rs`
   (83 constants).
4. `src/backends/interpreter/executor/ops/` contains the interpreter’s family of opcode dispatchers.

**Conclusion**: the supposed “registers” are **operand-stack slot indices, not physical registers**.
The target machine **has no finite register file, no register pressure, and no need for spilling or
splitting**.

**Evaluation of three approaches**:

| Approach                                           | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                          |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both take rustc’s `Function` trait and MIR’s `Place`/`Operand` abstractions as input. Writing an adapter for YaoXiang IR would require implementing the complete trait (CFG, liveness, spill slots, scratch space)—**500+ lines of adapter code for no benefit**, because there is no real register file to color. Moreover, rustc itself only performs linear scan at this point (when `enable-llvm` is disabled) |
| `graph-alloc` family                               | **Not adopted.** Same reason, with an even less stable API                                                                                                                                                                                                                                                                                                                                                                          |
| **Custom linear scan (simplified Chaitin-Briggs)** | **Adopted.** What YaoXiang actually needs is to map each `ValueId` to a non-conflicting `u8` stack slot in `values` order (`OperandResolver` already has a `u8` contract and the E3014 upper limit). This is fundamentally **maximum-clique coloring of non-overlapping live intervals**, which, without a register-file constraint, degenerates to “allocate the first available slot in definition order”                         |

**Design judgment**: implement a custom allocator targeting 500–1,000 lines (including the
`values → Reg` mapping, live-interval computation, and the `OperandResolver` interface). **At the
same time, it must be acknowledged: if the `u8`/255 slot limit is insufficient under real workloads,
the custom allocator must handle overflow, while the existing code already treats this as an error
at `1992-2000`.** This means that this document **does not solve** the problem of “a large function
not having enough registers”—**that is a downstream consequence of `compile_pattern` recursion depth
and belongs to a separate topic.**

The dev-dependencies in `Cargo.toml:37-39` are only `criterion` / `proptest`; there are no regalloc
/ petgraph / graphlib dependencies in `[dependencies]`. Introducing any crate would add a new
dependency surface.

### Decomposition Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA shape and criteria must exist first**. The reason is that
decomposition itself is C1 (a pure move, with zero-diff snapshots as its criterion), but
decomposition will not carry the old ordering assumptions into the new files unless the semantics of
`arg_regs` have first been reduced by SSA to “one independent `ValueId` per argument.”

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

The “fill in 0” logic at `7835-7848` (`7838-7846`) moves down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, **and the comment
claiming that “the normal path should never reach here” is replaced by an assertion**: if any slot
is absent and typechecking did not report E1010, return a diagnostic directly. **This turns a silent
fallback into a hard error and is this document’s most direct improvement to Defect Class 2.**

**Criteria**: for the pure-move portion (before the SSA shape changes), use zero-diff snapshots;
assertion-based `fill_missing` is C4, using corpus differentials and the first-layer verifier.

### The `synth.rs` Boundary: Turning Implicit Callbacks into an Explicit Synthetic-AST Boundary

**Verified fact**: the locations in `ir_gen.rs` that construct `ast::Expr` are **exactly
two**—`:4246` and `:6701` (with `:6704` constructing `ast::Stmt` and `:510` / `:1266` / `:1399`
constructing `ast::Type`).

**Proposal**: create `src/middle/lower/synth.rs` as the **only module in the repository** permitted
to construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not “tidiness,”
but rather:

> Constructing AST means **reentering the frontend lowering entry point**. The `pending_env_vars` /
> `closure_captures` state expected by `generate_expr_ir` is a **precondition owned by the caller**,
> yet this precondition is currently expressed by neither a type nor a signature—it holds only
> because of the runtime fact of “who calls me.”

Change this to an explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST that captures env; the capture set is explicitly provided as an argument,
/// without depending on the caller having first written self.pending_env_*
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Specific changes:

| Location                       | Current state                                                                                    | Change                                                                                                                                                                                                                      |
| ------------------------------ | ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → construct Lambda → call back                                          | Call `synth::synth_closure(...)` to obtain the AST and pass `SynthEnv` explicitly; add an internal entry point carrying `SynthEnv` to `generate_lambda_expr_ir` (`6543`), which **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → construct → call back → clear                                                            | Same as above; the `clear()` calls at `6713-6714` disappear because there is no shared state to clear                                                                                                                       |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                          | Read from the argument instead                                                                                                                                                                                              |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                                 | Same as above; `closure_captures` becomes a **local variable during closure-body generation**, and the `clear()` at `6600` disappears                                                                                       |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads a shared field on the namespace-resolution path | **This is the most dangerous location**: change it to an explicit argument; otherwise, any `?` early return in `generate_lambda_body_ir` lets the capture table leak into unrelated code                                    |

**Criteria**: add the sentinel test `test_no_ast_construction_outside_synth`, which scans
`src/middle/**` and rejects any `ast::Expr::` / `ast::Stmt {` construction syntax outside
`synth.rs`. Its shape follows RFC-039’s `check-module-boundary.py`.

### Disposition of the Span-Keyed Problem (the Portion SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism 1: Consumption-coverage assertion (mandatory, lowest cost; superseded by a stronger form
on 2026-10-13).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). This
document originally supplemented it by making it a **runtime counter** rather than merely a test.
The implemented shape is stronger than that proposal: a **consumption ledger
(`ConsumptionLedger<K>`) + a single settlement point (E-level `W1090`)**, with no new diagnostic
code and no reliance on an internal ir_gen counter to certify itself:

```rust
// Near ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// Each time :1942 hits, increment self.release_plan_hit by 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan was not consumed by the IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a required drop did not occur” may not immediately crash under YaoXiang’s
current value semantics + Arc/Rc model**—which is precisely why it was silent. Counting turns
“silent” into “detectable.”

**Mechanism 2: Change Span keys to `DefId` or an explicit plan ID (**landed on 2026-10-13**: RFC-039
D20 / WBS 4.5 implements this mechanism as a `PlanId(u32)` structural-identity key + mandatory
consumption settlement; see the §2 revision note in [02](02-stage-contract.md)).** The fundamental
fix is **not to use Span as a key**. `Span` is a source location and can drift due to macro
expansion, `include`, and multi-file merging. `FunctionIR` already has a `def: Option<DefId>` field
(`ir.rs:677`), demonstrating that DefId is available at the IR layer. **Design judgment**: introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, allocated by ownership / overload
resolution when producing the plans and matched by `PlanId` when the IR consumes them. **This
belongs to 02’s obligation ledger (`Obligations`) and should be implemented together with
[02](02-stage-contract.md), not in this document.**

**Mechanism 3: Definition-order dependence of `method_def_ordinals` (independent small item).** This
field is a `HashMap<String, usize>` (`ir_gen.rs:224`) whose key is `"{type_name}.{method_name}"`
(**without module qualification**, written at `:1691-1700`). Two modules that define the same
`Type.method` share one counter, causing the `#N` mangled name to drift. **Its failure mode is not a
span mismatch but an ordering/name mismatch**—if typechecker registration order differs from ir_gen
definition order, methods with the same name resolve to different targets. Disposition: have the
typechecker side **produce once** a `HashMap<DefId, String>` (bare names or `#N` mangled names), and
make ir_gen read-only. **This removes another mutable cross-function state field from ir_gen.**

### Tracking the Three Hard-Coded Drops

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hard-coded drops:

| Location           | Field                | Value                                          | Consequence                                                                                                                           |
| ------------------ | -------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count is reset to zero after a `.42` round trip                                                                       |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **`.42` artifacts lose the entire exception table**                                                                                   |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` artifacts lose global-variable information** (note that `global_names` at `2347` **is** populated, creating an inconsistency) |

**Design judgment (revised 2026-10-05, decision D52): all three are brought into this document’s
scope as P7 batch e (7e), and no separate “independent issue” will be created.** The original
judgment—creating independent issues for the latter two—and its rationale are recorded below,
together with the reasons it was overturned:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields), and are
   unrelated to SSA.~~ This is true, but does not justify exclusion—P7 already modifies
   `bytecode.rs`, and fixing all three together requires only one `VERSION` bump (4→5). Splitting it
   across two bumps is more expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → bumping the version → expanding scope
   from “IR shape” to “artifact format.”~~ The version-bump mechanism already exists (a `MAGIC` +
   `VERSION` header, with the reader rejecting mismatches), and `.42` is a build artifact;
   cross-version compatibility is not a goal. The scope expands the workload, not the risk.
3. ~~The actual risk levels of `2315` / `2341` are low, and running `.42` directly is a secondary
   path.~~ **Overturned**: losing the exception table makes throw/try behave incorrectly when `.42`
   is run directly—throw/try is core language semantics. A secondary path does not imply that errors
   may remain silent. This refactoring’s principle is **not to leave core functionality as an
   unimplemented legacy** (`Vec::new() // Not implemented yet` is precisely such a legacy).

**Retained at the same time**: all three must still be registered in the `Obligations` ledger as
“produced but not serialized” until 7e is complete, to prevent them from becoming a fourth kind of
“thing nobody knows about.” The fix at `2312` lands with the version bump (D17), and the assigned
but unused opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Knock-On Effects of the IR Structural Changes

#### `src/middle/core/ir.rs` (905 lines)

| Change                                                                    | Location                     | Nature                                    |
| ------------------------------------------------------------------------- | ---------------------------- | ----------------------------------------- |
| Add the `ValueId(u32)` type                                               | Near the file header at `10` | New                                       |
| Delete `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `12-20`                      | **Variant additions/removals**            |
| Add a `Phi` variant to `Instruction`                                      | End of `47-533`              | **+1 variant → 77**                       |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `636-642`                    | +1 field                                  |
| Add the `ValueInfo` structure                                             | Near `660`                   | New                                       |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | `689-719`                    | Add `values()` / `values_mut()` accessors |

The `pub use ...ast::Type` at `ir.rs:3` **is not modified by this document**—it belongs to the scope
of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **single converged type**
after 03.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                      | Location                              | Nature                                                                                                       |
| --------------------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Add `Phi` to `BytecodeInstr`, or use a “runtime expansion to Move” strategy | `127-554` (currently **66** variants) | +1 variant or +0 (see below)                                                                                 |
| Add a `Phi` arm to `opcode()`                                               | `558`                                 | Mandatory (`BytecodeInstr` uses an exhaustive match)                                                         |
| Add a `Phi` arm to `size()`                                                 | `649`                                 | Mandatory                                                                                                    |
| Add a `Phi` arm to the `From<BytecodeFile>` decode match                    | `943-2350`                            | Mandatory                                                                                                    |
| Fix `upvalue_count: 0`                                                      | `2312`                                | Fixed as a side effect in this document (in-memory format; persistence requires a `VERSION` bump, see below) |

**The bytecode shape of `Phi` requires a design decision.** There are two options (**design
judgment: recommend A**, for the reasons in “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded by `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the beginning of the block,
  selected according to the predecessor block index. **No new opcode is added**, and the `.42`
  format remains unchanged. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode plus predecessor-index encoding, and have the interpreter maintain register
  versions when jumping. **This requires changes to the `executor/` state model + the `.42` format +
  the version number.** The only benefit is size.

The decisive reason for choosing A is that changing the `.42` format triggers version-number issues
and a full compatibility discussion, **whereas A’s size cost is irrelevant in interpreted
execution** (the interpreter executes Move instructions, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*`, distributed across `101-1558`)

| Change                                                           | Location                                                                                                                         | Nature                                                                                  |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                                     | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add an arm                                                                              |
| Add an `Instruction::Phi` translation arm                        | Inside the dispatch match, before `Free` (`533`)                                                                                 | New branch that expands into a Move sequence (option A)                                 |
| Leave the positions of the existing nine NOP lowerings unchanged | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as nine**: `655-657` covers two variants, and `658-660` covers four variants |

**SSA transformation will not reduce the number of NOPs.** These nine lowerings reflect
**expression-capability gaps where the IR has something the bytecode does not**; SSA transformation
does not touch them.

#### `src/backends/interpreter/executor/`

With option A, `executor/` requires **no changes**—`Phi` disappears during the `translator.rs` stage
and is never visible to the interpreter. **This is the second decisive reason for choosing A.**

`BytecodeInstr::Switch` has an active implementation at `executor/ops/control.rs:90`, but **no**
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs` (measured: zero matches for these three
names in `ir.rs`)—**this is a reverse gap**: the bytecode layer contains instructions that the IR
layer cannot produce. It does not affect this document, but it is recorded here because it shows
that “IR → bytecode” is not surjective.

The dispatch match at `executor/debug.rs:194` likewise requires no changes (there is no new opcode
under option A).

### Compiler Change List

**File by file and function by function.** All line numbers are verified **starting points** for the
changes:

| No. | File                                      | Location                                                     | Change                                                                                                                           |
| --- | ----------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/core/ir.rs`                   | `12-20`                                                      | Add/remove `Operand` variants                                                                                                    |
| 2   | `src/middle/core/ir.rs`                   | End of `47-533`                                              | Add `Instruction::Phi`                                                                                                           |
| 3   | `src/middle/core/ir.rs`                   | `636-642`                                                    | Add `values` to `FunctionBody::Code`                                                                                             |
| 4   | `src/middle/core/ir.rs`                   | Near `660`                                                   | Add `ValueInfo`                                                                                                                  |
| 5   | `src/middle/core/ir.rs`                   | `689-719`                                                    | Add `values()` / `values_mut()` accessors                                                                                        |
| 6   | `src/middle/core/ir.rs`                   | `3`                                                          | **Do not change `pub use ast::Type`** (scope of [03](03-type-unification.md); recorded only)                                     |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                                    | Return `Operand::Value`; increase monotonically without rollback                                                                 |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                                  | **Delete** the statement-level reclamation                                                                                       |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                                  | **Delete** the statement-level reclamation (including the `result_reg.is_none()` guard)                                          |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                                  | **Delete** the statement-level reclamation                                                                                       |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                                    | Replace with an RAII guard (`generate_method_ir`, entry point `1677`)                                                            |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                                    | Replace with an RAII guard (`generate_function_ir`, entry point `1830`)                                                          |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                              | Replace with an RAII guard (`generate_curry_innermost_func`, entry point `2105`)                                                 |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                                    | Replace with an RAII guard (`generate_curry_function_ir`, entry point `2209`)                                                    |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                                    | Replace with an RAII guard (`generate_anon_binding_ir`, entry point `2759`)                                                      |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                                    | Replace with an RAII guard (`generate_lambda_body_ir`, entry point `4678`)                                                       |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                                  | Extract the E3014 check into `check_register_budget()`, shared after the accounting at `1798`/`2001` is unified                  |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                                       | Change `take_cur_locals(param_types.len())` to `take_cur_locals(self.temp_high_water)`, eliminating the accounting inconsistency |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                                    | Delete the comment contract (now enforced by the type system) and replace it with a pointer to `verify`                          |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                                  | Change to call `synth::synth_closure`                                                                                            |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                                  | Change to call `synth::synth_closure`; delete `6713-6714`                                                                        |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                                  | Read `SynthEnv` from the argument; no longer use `mem::take` on a shared field                                                   |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                                         | Make `closure_captures` local; delete `clear()`                                                                                  |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                                        | Change `closure_captures.contains_key` to use an explicit argument (**the highest-priority implicit dependency**)                |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                               | Add span-keyed consumption counting                                                                                              |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                                  | Make `method_def_ordinals` read-only                                                                                             |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                                  | **Last**: decompose into a top-level `CallArgs` + six `emit_*` functions                                                         |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                                  | Replace the “fill in 0” fallback with a returned diagnostic                                                                      |
| 29  | `src/middle/lower/synth.rs`               | **New file**                                                 | Synthetic-AST boundary                                                                                                           |
| 30  | `src/middle/ir/verify.rs`                 | **New file** (first layer of [07](07-equivalence-oracle.md)) | `verify_loose` / `verify_ssa`                                                                                                    |
| 31  | `src/middle/passes/codegen/translator.rs` | Dispatch match                                               | Add an `Instruction::Phi` arm (option A: expand into a Move sequence)                                                            |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                            | Add an `Operand::Value` resolution arm; delete the `Temp` arm (`39-45`)                                                          |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                                    | **Unchanged under option A** (`Phi` does not enter the bytecode)                                                                 |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                                   | **Unchanged under option A**                                                                                                     |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                                       | Fix `upvalue_count: 0`                                                                                                           |
| 36  | `src/backends/interpreter/executor/**`    | —                                                            | **Zero changes under option A**                                                                                                  |
| 37  | `src/middle/passes/regalloc.rs`           | **New file**                                                 | Linear-scan allocator                                                                                                            |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New file**                                                 | `synth.rs` boundary sentinel check                                                                                               |

### Backward Compatibility: Versioning the `.42` Format

**Verified fact**: the `.42` on-disk format **already has a version-number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (“YXBC”) and
`VERSION: u32 = 4`, writes them to the header (`:342-343`), and validates them while reading
(`:454-467`, reporting “unsupported bytecode version” on mismatch).

**Design judgment (a direct consequence of option A)**:

- `Phi` is expanded by `translator.rs` into a Move sequence and **produces no new opcode**.
- The 83 constants in `opcode.rs` have **zero changes**, `opcode_name()` (`120`) is unchanged, and
  the decode match (`bytecode.rs:943-2350`) is unchanged.
- The `.42` format’s field layout has **zero changes**, and `VERSION` remains 4. Old `.42` files can
  still be read by the new binary, and new `.42` files can still be read by the old binary. **SSA
  transformation itself does not require a version bump.**

**The only scenario requiring a version bump is the fix at `2312 upvalue_count`** (decision D17: fix
it and handle it together with the version bump): add an `upvalue_count` field to `BytecodeFunction`
and have the encoder write it. Because this is **a new format field**, bump `VERSION` from 4 to 5.
The reader’s existing version check rejects old files under its established behavior—`.42` is a
build artifact (`main.rs:671`), and cross-version read compatibility is not a goal, so the version
bump requires no additional migration mechanism. The assigned but unused opcode values (D32/D34’s
`Switch` / `TailCall`) are reclaimed with the version bump.

## Implementation Notes

### Batch Strategy

Divide the work into four batches. **Every item within a batch is an independent commit with
independent criteria and an independent revert**. The “change items” in the table below refer to the
numbers in the preceding “Compiler Change List.”

| Batch       | Content                                                                                                                                                                    | Change items                 | Criteria                                                                                                                                                             | Rollback point                                                                                                                                              |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Break multiple definitions: delete the three statement-level reclamations → replace six save/restore sites with an RAII guard → unify accounting                           | 8-10 (first) → 11-16 → 17-18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariants). RAII and accounting unification are pure moves: **C1** zero-diff snapshots | One commit per item; roll back with `git revert`                                                                                                            |
| **Batch b** | Switch to the SSA shape: change `ir.rs` structures + mechanically replace `Operand` (323 locations) → add the `Phi` arm to `translator.rs` → add the linear-scan allocator | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + all-green `verify_ssa` + `.42` round-trip tests                                                                                     | The `ir.rs` changes **must occupy an exclusive commit** (the type changes affect compilation across the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary + sentinel script → span consumption counting → make `method_def_ordinals` read-only                                 | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + the new script integrated into CI; consumption counting must trigger zero diagnostics (otherwise, a real mismatch exists)           | Independent commit                                                                                                                                          |
| **Batch d** | Decompose `generate_call_expr_ir` + make the “fill in 0” fallback assertion-based                                                                                          | 27-28                        | Pure-move portion: **C1** zero-diff snapshots. Assertion-based portion: **C4** corpus differential + first-layer verifier                                            | Independent commit                                                                                                                                          |

**Three dependencies whose order within the batches cannot be exchanged**:

1. Batch a’s “delete reclamation” must precede batch b’s `ir.rs` changes—first make `next_temp`
   increase monotonically, so that `ValueId` has the semantic basis for “unique definition.”
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise, the decomposition carries the
   old `arg_regs` ordering assumptions unchanged into the new file.
3. Batch d must come last—it depends on the preceding batches having already reduced the semantics
   of `arg_regs` to “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within **P7** in RFC-039, and the criteria
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial**, but items
8-10 / 11-16 / 17-18 within batch a have only ordering constraints and are not coupled to one
another.

### Hard Prerequisites

**Prerequisite 1: criteria baseline.** The first-layer verifier’s `verify_loose` mode in
[07](07-equivalence-oracle.md) **must first pass on the existing non-SSA IR** before batch a may
begin.

The reason is that the criteria for SSA transformation are C4 (behavioral equivalence + invariants).
If the invariant verifier itself cannot pass on the existing IR before the transformation, every
acceptance gate in batches a–d lacks executable criteria and can rely only on corpus
differentials—yet corpus differentials are **precisely insensitive** to Defect Classes 1/2/3 (as
demonstrated by “Alternative A” in 07). **This is a genuine ordering constraint.**

**Prerequisite 2: type-representation convergence.** [03](03-type-unification.md) must complete the
convergence of `pub use ast::Type` at `ir.rs:3` and the removal of the `From<MonoType> for IrType`
bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` would introduce a third set of
type-compatibility checks.**

### Rollback Strategy

- Each change is an independent commit, with `git revert` granularity equal to change-item
  granularity.
- Batch b’s `ir.rs` structural changes are **the only part that cannot be rolled back
  incrementally** (the type changes affect compilation across the repository). They must therefore
  **occupy an exclusive commit**, and batch a must already be green and have a revertable baseline
  beforehand.
- **Do not use a feature flag.** The reason is that adding/removing `Operand` variants cannot be
  isolated with a runtime switch; a feature flag would leave CI permanently untested on one path,
  repeating the failure recorded in RFC-039: “test wiring depends on someone remembering, and
  corruption occurs silently.”

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front; otherwise, the work will be questioned midway for “not reducing the
line count.”**

| Item                                      | Line-count change                                                                                                                                                                               |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                            | `ir_gen.rs` **8,448**                                                                                                                                                                           |
| Delete three statement-level reclamations | **−15**                                                                                                                                                                                         |
| Replace six save/restore sites with RAII  | Net ±0 (the RAII guard definition adds about +20 lines, while call sites fall from 22 lines to about 12)                                                                                        |
| Change `next_temp_reg`                    | +5                                                                                                                                                                                              |
| Decompose `generate_call_expr_ir`         | **+40–80** (overhead from the signatures and parameter passing of the six `emit_*` functions; this is the typical cost of a pure-move decomposition—**decomposition increases the line count**) |
| Span-keyed consumption counting           | +15                                                                                                                                                                                             |
| Make `method_def_ordinals` read-only      | +20                                                                                                                                                                                             |
| New `synth.rs` file                       | +80–120                                                                                                                                                                                         |
| New `verify.rs` file                      | +400–600 (07 first layer; within this document’s scope, but counted separately)                                                                                                                 |
| New `regalloc.rs` file                    | **+500–1,000**                                                                                                                                                                                  |
| **Total**                                 | **`ir_gen.rs`: approximately 8,448 → 8,600–8,800; including new files, a net increase of 900–1,600 lines**                                                                                      |

**The early expectation of “8,448 → 5,000–6,000” was incorrect**; the measured analysis yields the
opposite result. The reasons are as follows:

1. **The core benefit of this document is not fewer lines; it is eliminating an entire class of
   “wrong without reporting an error” defects.** The six manual save/restore operations (22 lines)
   become approximately 12 lines plus a guard type under SSA + RAII—**the line count is almost
   unchanged, but “forgot to restore” changes from a possibility into a compile error**.
2. **Pure-move decomposition necessarily increases the line count.** Splitting the 746-line
   `generate_call_expr_ir` into seven functions requires repeating the signature (the function
   already has `#[allow(clippy::too_many_arguments)]` at `7268`), `&mut self` binding, and error
   propagation. **“Splitting the file to reduce the line count” is a misconception—it reduces
   cognitive load, not physical lines.**
3. **The new verifier and allocator are net additions.** These two components (approximately
   900–1,600 lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of lines, this document is most likely a **net increase**. The benefit lies in
> **verifiability**—turning “wrong without reporting an error” defects into invariants that **fail
> the build**. If line count were used as the acceptance criterion, this document would fail. The
> correct acceptance criteria are those in the “Batch Strategy” section.

**This stage intentionally increases the amount of code in `ir_gen.rs`.** The 2026-10-03 decision
eliminated all line-count / size gates (see prohibition two in [08](08-maintenance-mechanism.md));
therefore, **no line-count exemption list is needed**. The increase is justified by separation of
responsibilities: SSA transformation separates “register allocation + definition discipline” from
lowering into an independent module, trading increased code size for the elimination of the two
classes of manual discipline that cause “misalignment without errors, with only incorrect values”:
six manual save/restore operations and the semantic reordering of `arg_regs`.

## Key Decisions and Rationale

### Three Core Decisions

**Decision 1: Temporary-value strategy—delete the three statement-level rollbacks so that unique
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, and every temporary slot naturally has exactly one definition point.
**Differences in criteria despite otherwise-consistent behavior**—the guard at `3632` uses
`result_reg.is_none()`, while the other two locations do not—then disappear along with the
hand-copied code. The cost is increased slot occupancy—`temp_high_water` already provides a 255
upper-bound check (`1992-2000`) as a backstop.

**Decision 2: Do not perform mem2reg; perform explicit renaming only.** The rationale is given in
“Value Identification Method”: YaoXiang’s `&` / reference / ownership semantics make “which slots
can be promoted to memory” a type-system problem that requires the type representation converged by
[03](03-type-unification.md). **Performing mem2reg before 03 would be guessing.** Explicit renaming
only requires changing the return-value semantics of one `next_temp_reg` location and deleting the
three rollbacks, without touching the control flow of any lowering algorithm.

**Decision 3: Use option A for `Phi` (an IR-only pseudo-instruction expanded into a Move
sequence).** `executor/` and the `.42` format require **zero changes**, limiting the regression
surface to L3. The decisive reason is that changing the `.42` format triggers version-number issues
and a full compatibility discussion, whereas option A’s size cost is irrelevant under interpreted
execution (the interpreter executes Move, not Phi). The cost is that `Phi` is invisible in
`dump_bytecode` (when entering the C1/C2 stages, note that 07’s third layer compares them).

### Rejected Alternatives

**A. Perform the entire decomposition at once (SSA + decomposition + `synth` boundary + allocator
simultaneously). Rejected.** This violates 07’s core principle: criteria must be categorized into
tiers. Mixing four batches into one PR mixes pure-move changes with C4 changes, making snapshot
drift impossible to attribute—if a snapshot changes, it is impossible to tell whether the
decomposition moved something incorrectly or the SSA change was incorrect. **Rollback granularity
degrades from “change item” to “everything.”**

**B. Split files first and optimize ordering afterward. Rejected.** Splitting eliminates no root
cause: the six manual save/restore operations remain distributed across 10 files, and the semantic
reordering of `arg_regs` still requires manual reasoning. Worse, **splitting first loses the
opportunity to “establish criteria before changing anything”**—the split produces a large line-level
diff that gets mixed into subsequent SSA diffs, making review ineffective. This has the same root
cause as the eight orphan test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests
that have never run): **if the order is wrong, every subsequent step accelerates on the wrong
foundation.**

**C. Introduce an intermediate SSA layer without changing the existing lowering. Rejected, but
recorded as “a third option once considered.”** The shape is to leave `ir_gen.rs` unchanged and add
a pass that converts its non-SSA IR into SSA IR. Its **advantage** is that lowering is untouched and
rollback is easy. **Reasons for rejection**: (1) It does not eliminate the root cause of Defect
Class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the six save/restore
operations remain separated by 59–161 lines. A conversion pass can merge multiple definitions into
`Phi`, but it **cannot fix “names from an inner function leaking into the outer function”**—that is
a `cur_locals` save/restore error that occurs before the IR shape and is therefore invisible to a
conversion pass. (2) It turns SSA into a second representation—the repository already has the
cautionary tale of three parallel type representations, and this project’s core lesson is that
**“boundaries exist only in people’s discipline.”** (3) `verify_loose` would become a permanent
burden.

**D. Keep the current implementation and only add a verifier. Rejected as the sole solution, but it
is a component of this document.** RFC-039’s alternative B reaches the same conclusion: a gate can
prevent regressions but cannot repair the existing implementation. **However, what D can accomplish
must be stated clearly**: D can **catch instances** of Defect Classes 1/2/3 (provided the verifier
covers dominance, type consistency, and inner isolation), but it cannot catch Defect Class 4
(`closure_captures` leakage), which requires a cross-function liveness assertion beyond the boundary
of `verify(&ModuleIR)`. **D is therefore not an alternative to this document; it is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not the
  risk**—the 2026-10-03 decision eliminated line-count / size gates, and the scale issue is
  addressed through separation of responsibilities (see prohibition two in
  [08](08-maintenance-mechanism.md)). The increase is justified because it replaces the two classes
  of manual discipline that cause “misalignment without errors, with only incorrect values”: six
  manual save/restore operations and the semantic reordering of `arg_regs`.
- **The “unique definition” check in `verify.rs` is meaningless in the non-SSA shape**
  (`Operand::Local` permits multiple definitions)—07 already records this limitation, and
  `verify_loose` is its degraded mode before this batch.
- **Deleting statement-level reclamation changes the temporary-slot allocation pattern** (executed
  in batch b, with 7a.1 and 7b.4 in the same batch). Even if `temp_high_water` guarantees no
  overflow, **the allocation result changes**—which is why the C4-stage “behavioral equivalence”
  criterion must genuinely run over the full corpus rather than relying on sampling.
- **The `u8` / 255 slot limit is insufficient after deleting the rollbacks (verified)**: after
  deleting the three reclamations and running the full corpus, two corpus entries exceed 255 and
  trigger E3014—`src/std/tests/string_ops.yx` with 354 slots and
  `tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; the peak slot count for all
  other corpus entries rises from 51 to 211. `take_body_locals` in `src/middle/lower/scope.rs`
  catches the overflow, but catching it means compilation fails. Therefore, deleting the
  reclamations comes after the allocator (7b.4): first let the allocator take responsibility for
  slot reuse, then delete the reclamations.
- **The advantage must also be recorded**: turning “errors that cannot be tested” into “invariants
  that can be tested”; no backend changes (`executor/` and `.42` remain unchanged); no changes to
  the semantics of `Instruction` (all 76 variants are retained, increasing to 77, and all 50
  `translate_*` functions are retained); incremental implementation is possible (each item can be
  reverted independently).

> **All open questions originally listed in this section have been adjudicated.** The decisions are
> recorded individually in
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching framework; four-layer model, criteria tiers, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definitions of the
  C4 criteria category, `verify_loose` / `verify_ssa`, three-tier criteria, and the limitation that
  snapshots are immune to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of three
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the
  `ReleasePlan` `PlanId` transformation
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — eight orphan test trees; the cautionary
  example of “creating the directory first and then failing to wire it up”

### Code Locations

**IR Definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` (scope of [03](03-type-unification.md))
- `src/middle/core/ir.rs:12-20` — seven `Operand` variants
- `src/middle/core/ir.rs:47-533` — 76 `Instruction` variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (the feasibility basis for the
  `PlanId` proposal)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect Class 1 (Multiple Definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for seven nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `take_body_locals` in `src/middle/lower/scope.rs` — sole table-extraction point + E3014 budget
  point (`MAX_REGISTERS = 255`)
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only from inside `take_body_locals`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the true high-water mark in
  `temp_high_water`
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — three statement-level register
  reclamations
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant
  referenced by 07

**Defect Class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  a clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill in 0” fallback

**Defect Class 3 (Span Keying)** — the following are **audit-point evidence pointers** (after the
4.5 key migration, both `release_plan` / `overload_resolutions` keys use `PlanId`; see the §2
revision note in [02](02-stage-contract.md) for current pointers; `method_def_ordinals` remains a
mechanism-three to-do item)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write point of
  `method_def_ordinals` (the key is the unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — the two fields in the propagation structures
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — the `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer propagation
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect Class 4 (Implicit Ordering Dependencies)**

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
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded drops (compare `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (the bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — nine NOP
  lowerings
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  limit of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (with no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current Test Coverage**

- `src/middle/core/ir_gen.rs` — 8,448 lines, zero `#[cfg(test)]` blocks, zero `mod tests` modules
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, 29 tests in
  total; `def_assign` / `local_slots` positively pin DefIds and slot names, with no IR
  structural-invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — four references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example of per-opcode round-trip criteria; the last test in the file; 1,209 lines / 23 `#[test]`
  functions in total)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies are only `criterion` / `proptest`; there are no regalloc /
  petgraph / graphlib dependencies in `[dependencies]`
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans the `/rfc/draft`
  directory; this document is not under that directory and is referenced by RFC-039 and this
  directory’s index
