# SSA Transformation of the Intermediate Representation

> **Companion design document.** This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main RFC-039 document for the four-layer model, acceptance-criterion tiers, and
> execution-phase order; see the [index for this directory](index.md) for the positioning of each
> companion document.

## Purpose and Scope

This document addresses the IR construction discipline of the L3 layer. It proposes a construction
method based on “one definition per value” for `src/middle/core/ir_gen.rs` (8,448 lines). **The goal
is not to introduce a new instruction set, but to eliminate three classes of discipline defects that
fail without producing any error**:

1. Six hand-written register save/restore pairs separated from their counterparts by dozens of
   lines;
2. Semantic reordering of `arg_regs` shared by six branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, explicit cross-layer contracts, decomposition of
call-expression dispatch, and integration of the IR static verifier.

This document is not responsible for: convergence of type representations
([03](03-type-unification.md)), stage contracts and the obligations ledger
([02](02-stage-contract.md)), definition of equivalence criteria and gates
([07](07-equivalence-oracle.md)), frontend paradigms ([05](05-frontend-paradigm.md)), or cleanup of
dead code and placeholder designs ([06](06-cleanup-inventory.md)). This document repeatedly cites
the conclusions of those documents without repeating their arguments.

**The criterion category** is **C4 (IR shape changes)** as defined by
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, rather than exact IR snapshot equality. SSA transformation necessarily changes the IR
shape, and forcing snapshot equality would encourage the team to weaken the criteria—the exact trap
recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, with acceptance based on behavioral equivalence and a green `verify_ssa`). The
implementation batch numbers within this document (batches a–d) are distinct from the internal
ordering of P7: the former are this document’s incremental delivery strategy, while the latter are
RFC-039’s global phases.

## Feasibility Preconditions

**Verified fact.** The IR already has **all structural prerequisites** required for SSA
construction:

| SSA prerequisite             | Current repository state                                                                                           | Location                                          |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                                   | `BasicBlock` in `src/middle/ir/ir.rs`             |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                     | `FunctionBody` in `src/middle/ir/ir.rs`           |
| Scope nesting information    | `LocalSlot::scope_depth` (`"0 = function parameter layer, used to disambiguate duplicate names in nested scopes"`) | `LocalSlot` in `src/middle/ir/ir.rs`              |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                                 | `Instruction` in `src/middle/ir/ir.rs`            |
| Per-instruction span         | Every variant has a `span: Span` field                                                                             | Same as above                                     |
| Annotatable value type       | `LocalSlot::ty: MonoType`                                                                                          | `LocalSlot` in `src/middle/ir/ir.rs`              |
| Slot-count upper-bound check | Table extraction and E3014 budget gate at a single point (`MAX_REGISTERS = 255`)                                   | `take_body_locals` in `src/middle/lower/scope.rs` |

**If these prerequisites were absent, the cost estimate in this document would be entirely
different.**

**The two decisive points are as follows:**

**First, SSA transformation does not add a capability; it removes a capability dependency.**
Temporary slots are currently **intentionally designed for multiple
definitions**—`generate_function_ir` and `generate_block_ir` in `src/middle/lower/stmt.rs`, and
`generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs`, explicitly roll back `next_temp` in
three places. The first step of SSA transformation is to **remove these three rollbacks**, allowing
`next_temp` to increase monotonically. Once they are removed, **each temporary slot naturally has
exactly one definition point**—single definition changes from “a rule that must be checked” into “a
mathematical consequence of the construction method.” Batch a only converged the save/restore
surface (see “Defect Class 1”); removing the rollbacks itself is performed with the allocator in
batch b.

**Second, the slot-count upper bound is a hard check, and removing the rollbacks causes the slot
count to become uncontrolled (measured).** `take_body_locals` in `src/middle/lower/scope.rs`
extracts the table according to `temp_high_water` and reports E3014 when the count exceeds 255.
After removing the three reclamations, running the full corpus showed that two corpus cases crossed
the limit and triggered E3014: `src/std/tests/string_ops.yx` with 354 slots and
`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; the peak slot count for all
other corpus cases rose from 51 to 211. `temp_high_water` records the historical maximum occupancy
and therefore does not change when rollbacks occur; however, once rollbacks disappear, the allocator
no longer reuses slots, so the peak rises accordingly.

**Taken together, these two points mean**: lowering already maintains all bookkeeping information
required for SSA, but **reclaims it**. Removing the rollbacks must wait until the allocator (7b.4)
takes over slot reuse. The bulk of the SSA work lies in decomposing `generate_call_expr_ir` and
making cross-layer contracts explicit, not in building new IR expressiveness.

### Prerequisite: Unification of Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also contains `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The serialization side has a third representation, `ir::Type`,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **the IR contains two type representations simultaneously, and one of them directly
`pub use`s an AST type.**

SSA transformation must add a `Phi` variant, record a type for every SSA value (for the “type
consistency” invariant in 07), and determine whether the types of the two predecessors at a CFG join
point are compatible. **If type representations have not yet been unified, SSA will require a
separate type-compatibility check for every representation**—and the third representation will grow
accordingly.

This is not a question of which approach should come first; it is a question of **which approach
would produce three separate type-compatibility implementations**.
[03 Type Representation Unification](03-type-unification.md) must first complete the convergence of
`pub use ast::Type` at `ir.rs:3` and remove the bridge at `bytecode.rs:2353`. Item 6 in the compiler
change list explicitly records that `ir.rs:3` **is not changed by this document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not merely an “ugly but functional” file. It is a file that **bets correctness on
meticulous human line-by-line review**:

- The file contains **zero `#[cfg(test)]` blocks and zero `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares the `bytecode`, `def_assign`, and `local_slots` modules,
  with 29 tests in total. `def_assign.rs` and `local_slots.rs` provide **positive** coverage of
  ir_gen’s DefId allocation and local-slot naming (sentinel-level), while all four mentions of
  ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **negative assertions**—“ir_gen does
  not produce,” “ir_gen does not construct either,” and “the ir_gen frontend does not construct the
  corresponding IR.”

In other words: ir_gen has 29 sentinel-level tests pinning local behavior, but **has no IR
structural-invariant validation**—dominance, single definition, jump targets, or type consistency.
None of the three modules declared by `middle/core/tests/mod.rs` makes positive assertions about the
overall IR shape.

This file also happens to contain several classes of defects that **compile successfully, pass every
check, and run correctly while producing incorrect values**. Their shared property is: **failure
produces no error**.

This is where the highest risk lies. The problem is not “the function is too long” or “the naming is
confusing,” but that **the core invariants of this file have no machine-executable enforcement
mechanism**.

The contract “save state before generating a nested function body, then restore it afterward;
otherwise names from the inner function leak into the outer function” is enforced by the type
system: `NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the
generator, with exactly one implementation each for frame extraction and restoration (see “Defect
Class 1”).

**Why simply “adding a verifier” is not enough**: a verifier can **catch** these defects, but it
cannot catch their **causes**. The cause is not “a reviewer missed something,” but rather:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator deliberately depends on that behavior.**

The concrete evidence is three statement-level register reclamations (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`). At the end of every statement, they **roll back** `next_temp`
to the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within one function body**, and the number and
order of writes, along with live ranges, are entirely determined by a single manual decision: when
to perform the rollback.

Under this shape, “single definition” is not a rule that can be introduced incrementally; it
**directly conflicts with the register-allocation strategy**. SSA transformation is therefore not a
matter of adding a check to existing code; it is a matter of **replacing the allocation strategy
that depends on multiple definitions**. The good news is that this strategy is inexpensive to
replace—see “Feasibility Preconditions” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (Batch a Converged the Restore Surface)

**Verified fact.** There are seven entry points that require save/restore when entering a nested
function body. Batch a (7a.2) converged them into an RAII guard: `NestedBodyGuard<'a>` in
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

- Frame extraction occurs only in the two constructors (`enter` / `enter_decl`), and restoration
  occurs only in `Drop`. The guard exclusively holds `&mut` access to the generator for its
  lifetime, so no second `&mut` reference is available in scope; all `?`/early-return paths restore
  state through `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span`
  / `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`. `next_temp_reg` continues
  to maintain `temp_high_water` as the true high-water mark (statement-level reclamation rolls back
  `next_temp`, while the total slot count is based on the historical maximum occupancy).
- `enter_decl` does not restore the three counters: after a function body or method body returns to
  the module-initialization flow, subsequent definitions continue from the child body’s final value
  number. This is the constraint for zero C1 snapshot drift and the basis for the rule that “the
  declaration surface restores only the table.”

**Unified semantics and a single budget point (7a.3 implemented)**: `take_body_locals` in
`src/middle/lower/scope.rs` is the sole table-extraction and budget point. It slices according to
`temp_high_water`, reports E3014 with the function-body span when the bound is exceeded, and defines
`MAX_REGISTERS = 255` exactly once across the repository. All six function-body forms—function,
method, lambda, anonymous binding, curry intermediate layer, and curry innermost layer—obtain their
tables through it. There is no second definition of “how many slots a function body should have.”

**The three duplicated statement-level register reclamations still remain (split out of 7a.1 into
batch a, then moved to batch b to match 7b.4)**:

| Location                           | Enclosing function        | Guard                     |
| ---------------------------------- | ------------------------- | ------------------------- |
| `src/middle/lower/stmt.rs`         | `generate_function_ir`    | None                      |
| `src/middle/lower/stmt.rs`         | `generate_block_ir`       | `if result_reg.is_none()` |
| `src/middle/lower/expr/closure.rs` | `generate_lambda_body_ir` | None                      |

The guard in `generate_block_ir` has a clear rationale: the block is an expression operand, and the
outer scope may hold sibling argument temporaries whose lifetimes cross the block. The other two
locations have no equivalent guard. **The same criterion is copied by hand in three locations, with
one guarded and two unguarded**—exactly the kind of discrepancy SSA is intended to eliminate.

**Why it was split out**: removing the rollbacks would cause the slot count to become uncontrolled
(see “Feasibility Preconditions,” item two, and “Known Limitations and Risks”). A linear-scan
allocator (7b.4) must first take over slot reuse.

**Consequences (current shape)**: the guard exclusively holds `&mut`, restoration has exactly one
implementation in `Drop`, and there is nowhere to write a missed restoration. What remains is
“multiple definitions of one slot” itself—the allocator depends on it, so the I-layer invariant
cannot assert single definition until the reclamations are removed in batch b.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified fact.** `generate_call_expr_ir` is located at `ir_gen.rs:7269-8014`, is **746 lines**
long, and is a single function. Its signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]`
/ `named_args: &[(String, Expr)]` / `span: &Span` (along with `_expr` / `result_reg` /
`instructions` / `constants`). Internally, six branches share the same `Vec<Operand>` and perform
**semantic rewriting**:

| Line        | Semantics                                                                                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `7395`      | Namespace call: flattened by `slots`, then recollected—named arguments are reordered                                                                         |
| `7724`      | Struct construction: each slot in `final_args` is unwrapped and then recollected—fields are reordered                                                        |
| `7835-7848` | Function call: named arguments are reordered; `7838-7846` **adds a new register with value 0** for each missing slot (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—cloned before instructions are emitted                                                                    |

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

The comment at `7835-7848` states: “Uncovered slots: remain absent, and the type-checking stage
reports E1010. For runtime safety, fill in 0 (the normal path should not reach here).”

**This design has two risks**:

1. **Misalignment produces no error, only an incorrect value.** Each of the six branches
   independently determines the final order of `arg_regs`, and any incorrect ordering assumption
   will not cause a panic.
2. **Normalized snapshots are immune to this.** 07 already records this limitation: after the
   snapshot tool renames temporary values by “first-occurrence order,” “the third argument should
   have used the fifth register” is exactly the same in the snapshot. **For this class of defects,
   even the snapshot criterion does not exist in the C4 phase.**

**Additional verification: the struct-construction path has no duplicate-slot guard when
reordering.** `final_args` at `7724` is populated by “positional arguments enter their slots first,
then named arguments overwrite them by field name,” and there is **no duplicate-slot check such as
`slots[idx].is_some()` before overwriting** (unlike the namespace path at `7381`). When a named
argument silently replaces a positional argument, there is no panic and no diagnostic. Moreover, the
**argument evaluation order** (source order) diverges from the final `arg_regs` order
(field-declaration order). The `fill_missing` assertion in batch d must also cover the
duplicate-slot check; otherwise, the silent replacement in the struct-construction path will survive
unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified fact at audit time.** The repository has **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                     | Production                                                                                                               | Propagation                                                                                | Consumption                                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                       | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatches statement identity through `walk_stmt`) | `typecheck/types.rs:38` ← propagated by `checker.rs` ← `ir_gen.rs:360` (cloned read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, sole entry point; settlement-complete marker `:1095`)             |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger colocated, `types.rs:62`) | `inference/expressions.rs:4550` (writes by call-site `PlanId`; `inference/statements.rs` only propagates and merges)     | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held by the `ir_gen.rs` field        | `ir_gen.rs:7697` (bound), `7772` (unbound) (`OverloadResolutions::get`; settlement-complete marker `:1097`) |

The shape at audit time (`ir_gen.rs:1942`, **now absent**):

```rust
// NLL Release: insert Drop instructions at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some`—**if the lookup fails, it simply fails: there is no else branch, no assertion, and no
counter**. If the two sides compute spans in any different way, **the Drop instruction silently
disappears with zero errors**. The Drop sequence for refined types is a central carrier of this
project’s ownership semantics (`ownership.rs:26-32` describes itself as an “NLL precise release
plan”), yet its implementation was silent.

The same pattern appears at `7420` / `7493`
(`if let Some(mangled) = self.overload_resolutions.get(span)`), causing overload resolution to fall
back to the default binding.

**Disposition (implemented in 4.5)**: both keys were changed to the structural identity `PlanId`,
and consumption is **accounted for on a hit** (the read and accounting occur in the same call). A
non-empty difference is reported as an **E-level `W1090`** at the sole settlement point—“not found”
is no longer silent. See the revision note in §2 of [02](02-stage-contract.md) (4.5) for the
implemented shape and criteria. These line numbers (`1942` / `7420` / `7493`) are retained as
audit-time evidence and are not current-state pointers.

> **`method_def_ordinals` is not span-keyed.** It is a `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, and has a single write site at `:1692-1702` (assigning a `#N` suffix in
> definition order by `base_name`). Its risk is **different**: definition-order dependence. The `#N`
> suffix is determined by AST definition order, so if generation order differs from typecheck
> registration order, the same function name may resolve to a different target. This is not a span
> mismatch, but it is equally “incorrect without an error.” Its disposition is described under
> “Disposition of Span-Keyed Problems,” mechanism three.

### Defect Class 4: Implicit spawn/Lambda Handshake

**Verified fact.** There are **two** places in `ir_gen.rs` that construct `ast::Expr::Lambda`
themselves and call back into `generate_expr_ir`, using a handshake through mutable fields:

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

**Handshake C — `closure_captures`**: written at `6589-6593`, **within the same function** that
calls `generate_lambda_body_ir` at `6598` (entry point `4678`), and cleared at `6600`. It has two
read points: `:6284` (`LoadUpvalue` in the closure body) and **`:458`**
(`closure_captures.contains_key(head)` on the `flatten_namespace` path).

The `:458` read point is the most dangerous: it is on the **namespace-resolution path**, meaning
that the live state of a `HashMap<String, usize>` can change variable-name resolution. Between the
write at `6589` and the clear at `6600` lies a complete closure-body generation (`6598`). If any
path inside `generate_lambda_body_ir` returns early without clearing the table—`?` error propagation
provides several such return points—**`closure_captures` will remain and leak into unrelated code at
`:458`**.

Other implicit state of the same kind includes `pending_env_names` (written by handshake B at
`6699`, cleared at `6714`).

### What SSA Will Not Solve (Stated Explicitly)

| Problem                                              | Location                                                                                                                           | Why SSA does not solve it                                                                                                                                                                                                                   |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Complexity of `compile_pattern`                      | `ir_gen.rs:5390-5687`, **298 lines**, four levels of self-recursion (`5473` / `5504` / `5619` / `5640`), external call site `5270` | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **inherent to the algorithm**. SSA only turns the `JmpIfNot` produced by guards into `Phi`; it does not reduce the number of guards |
| Complexity of `eval_const_expr`                      | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)            | Constant-folding expression coverage is unrelated to SSA                                                                                                                                                                                    |
| Span-keyed failure (**eliminated**)                  | Audit time: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`); after 4.5, `PlanId` keys + mandatory settlement     | **SSA does not touch this layer at all** (the conclusion is unchanged). It was independently resolved by the obligations-ledger mechanism in [02](02-stage-contract.md); see “Disposition of Span-Keyed Problems,” mechanism two            |
| Number of `Instruction` variants                     | 75 (`Instruction` enum in `ir.rs`; includes the `Phi` introduced by 7b.1a)                                                         | SSA adds `Phi`, for a **net change of +1** (already implemented). Variants correspond to bytecode opcodes or degrade to NOP and do not decrease because of SSA; under option A, `Phi` is an IR-only pseudo-instruction and adds no opcode   |
| Definition-order dependence of `method_def_ordinals` | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                              | Similar to but distinct from span-keyed failure; see “Disposition of Span-Keyed Problems,” mechanism three                                                                                                                                  |

## Target Design

### Definition of the SSA Shape

**Design judgment.** The target shape is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has five variants. **Measured across the entire repository** (hit
counts summed by file using `git grep -c`, including all of `src/` and its tests; construction-point
counts refer to value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                           | Disposition                                                                                           |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                                   | **Retained**, unchanged                                                                               |
| `Value(ValueId)`    | 7 files / 26 hits; **zero production construction points** (the producer belongs to 7b.2’s `next_temp_reg`)                                                                                           | **Retained** (introduced in 7b.1a)                                                                    |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                                   | **Retained, but semantics split** (see below)                                                         |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                     | **Retained**, as the implicit definition at function entry (arguments are the definitions in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                                    | **Retained**, unchanged                                                                               |
| `Temp(usize)`       | **Zero construction points** (the only handling arms are in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, and are removed with the variant)                                    | **Removed** (dead variant, 7b.1a)                                                                     |
| `Label(usize)`      | **Zero construction points**                                                                                                                                                                          | **Removed** (dead variant, 7b.1a)                                                                     |
| `Register(u8)`      | **Zero construction points** (the comment at the original variant declaration said "Added for codegen", but codegen uses `OperandResolver` in `codegen/operand.rs` and never constructs this variant) | **Removed** (dead variant, 7b.1a)                                                                     |

`Local(usize)` is split into two variants because it currently **serves two mutually exclusive
semantics**—“storage location for a named variable” (which may be written multiple times) and
“compiler temporary register” (which should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within the function, single definition
    Value(ValueId),
    /// Storage slot for a named local variable: addressable, multiple definitions allowed (non-SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new `u32` type (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **increasing monotonically and never rolled back**. The split
and `ValueId` were implemented in 7b.1a (`Operand` enum in `src/middle/ir/ir.rs`); this batch has
zero production construction points for `Value`—the producer (`next_temp_reg` changed to produce
`Operand::Value`) belongs to 7b.2.

#### `Phi` as a New `Instruction` Variant

```rust
/// Merge point: each predecessor basic block provides one definition of this value
/// incoming is sorted in ascending predecessor block-label order (guaranteed during construction for canonicalization)
Phi {
    dst: Operand,                                  // Must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

There are three construction constraints (see the first layer in [07](07-equivalence-oracle.md)):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set of `incoming` equals the label set of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. All `incoming` values have the same `MonoType` (**dependent on 03’s type-representation
   convergence**).

The implementation status of `verify_ssa` (7b.1b has completed the first three criteria) is as
follows: constraints 1 and 2 require the value table and block-level CFG. They were implemented in
**7b.1b** as separate “block-level predecessor count” and “predecessor label set” criteria. The
predecessor set is inferred from `BasicBlock.successors` (`block_predecessors` in
`src/middle/ir/verify.rs`). A count mismatch reports only the count, while a set mismatch reports
the set, so the same defect is not reported twice. The per-input type-consistency criterion for
constraint 3 was implemented at the same time, taking its values from the value table at
`values[v].ty` and **reusing the existing `types_compatible`** (no third type-compatibility
implementation was introduced; convergence of the type representation in 03 remains “Precondition
(hard) · Precondition two,” and `ValueInfo::ty` will still be `MonoType` there). All three criteria
use the violation category `InvariantKind::PhiConsistency`.

**Three forms for which the type is unavailable (explicitly recorded, never silently accepted)**: an
empty value table means that no basis for judgment exists (the production shape in 7b.1b; it
tightens automatically after 7b.2 populates the table); a non-empty value table with an out-of-range
`ValueId` is reported as a violation; and a non-`Value` operand (`Const` / `Local` / `Arg` /
`Global`) has no value-table type source and is therefore not judged.

**Measured precondition (important)**: all 15 production constructions of `BasicBlock.successors`
across the repository use `Vec::new()`, and there is no production population site. Consequently,
block-level predecessors are always empty, and the predecessor-count and predecessor-set criteria
have no data on real IR; synthetic fixtures carry that load instead (this is not a missing
criterion). **Before 7b.3 produces `Phi`, block-level `successors` must first be populated**, or
every `Phi` with incoming values will be rejected by `verify_ssa`; this is recorded as a
precondition of 7b.3.

7b.1a only introduces the variant: there are zero lowering production construction points, and the
consuming arm in `translator.rs` is a placeholder with an invariant comment (fails loudly rather
than silently skipping or silently producing an incorrect value). Actual translation belongs to 7b.3
(option A: expansion into a sequence of Moves).

**Disposition of the guard problem at `3632-3638`**: the current reclamation in `generate_block_ir`
has an `if result_reg.is_none()` guard (`3626-3631` comment: the block is an expression operand, and
the outer scope may hold sibling argument temporaries whose lifetimes cross the block). After SSA
transformation, this guard is **no longer needed**—temporary values are no longer rolled back, and
sibling arguments whose lifetimes cross the block hold their own `ValueId`, which cannot be affected
by a rollback. **This is a place where SSA transformation can genuinely simplify the code, rather
than merely rename it.**

#### Value Identification Method

**Design judgment**: do not perform mem2reg, do not introduce virtual-register numbering, and
**perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots can be promoted to memory” a
  **type-system problem**, not a CFG problem. It has been verified that `LocalSlot::ty: MonoType` in
  `ir.rs` carries complete type information, but determining whether a given `MonoType` can be
  promoted to memory requires the converged type representation from 03 and coordination with
  refinement-type rules. **Performing mem2reg before 03 amounts to guessing.**
- Explicit renaming only requires changing the return-value semantics of one function
  (`next_temp_reg()` in `src/middle/lower/scope.rs`) and removing three rollbacks. It **does not
  touch the control flow of any lowering algorithm**.

The value-table shape (**implemented in 7b.1b**): `FunctionBody::Code` gains
`values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // Label of the basic block in which the definition occurs
}
```

The accessors have the same shape as existing accessors (`impl FunctionIR` in
`src/middle/ir/ir.rs`): `values()` is read-only and returns an empty table for TypeDecl (mirroring
`locals()`); `values_mut()` is mutable and panics for TypeDecl (mirroring `blocks_mut()`). **The
value table remains empty in this batch**—all 17 production constructions of `FunctionBody::Code`
use `values: Vec::new()` (non-empty value tables occur only in test fixtures), and production of
`Operand::Value` remains in 7b.2.

`values` and `locals` coexist: `locals` carries addressable named variables (which may have multiple
definitions), while `values` carries single-definition SSA values. **The presence of `locals` does
not violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Separation of concerns with `LocalSlot::origin`**: these are **two dimensions** and must not be
conflated. `LocalSlot::origin` (the **slot identity** field introduced on the origin side in 2.1.4,
recording the generating `FrameId` of the slot) answers “which frame owns this slot,” while
`ValueInfo.defining_block` answers “in which basic block is this SSA value defined?” Value identity
belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`. 7b.1b only adds the value
table and **does not modify `LocalSlot::origin`** (verified: zero changes).

#### Change-Size Estimate

| Item                                                         | Estimate                              | Basis                                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------ | ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` transformation                               | ~15 lines                             | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                            |
| Removal of three statement-level reclamations                | −15 lines                             | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                   |
| Convergence of six save/restore sites to RAII                | Net ±0 lines                          | See “Item-by-Item Disposition Table”                                                                                                                                                                                                                                                      |
| `ir.rs` structural changes                                   | +120–180 lines                        | `ValueId` / `Phi` / `ValueInfo` / value table; **7b.1a implements the first two and 7b.1b the latter two** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3 and 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14 and 7b.1b +162/−13 ⇒ cumulative +225/−27) |
| `Phi` arm in `translator.rs`                                 | +25–40 lines (pending 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                            |
| `executor/`                                                  | **Zero changes**                      | Option A: `Phi` is an IR-only pseudo-instruction expanded into a sequence of Moves in `translator.rs`, and does not enter the bytecode (decision three)                                                                                                                                   |
| Call sites that produce temporary values as `Operand::Value` | Mechanical replacement (pending 7b.2) | **Measured** (`git grep -c` summed by file, including all of `src/` and its tests): 415 occurrences / 28 files of `Operand::Local(`, including 332 occurrences / 14 files under `src/middle/lower/`; the production point for temporary values is the single `next_temp_reg`              |

### Item-by-Item Disposition Table

| Defect class                                                | Eliminated by SSA?                                                       | Disposition                                                                                                                                                                                                                                                                                                                                                                                             | Criterion                                                                   |
| ----------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore**         | **Partially (restore surface converged; single definition pending SSA)** | The seven nested-function-body entry points have been consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the three counters), making “forgetting to restore” a compile error; the three statement-level reclamations remain until batch b (7a.1 and 7b.4 in the same batch), and `next_temp` can still be rolled back | First-layer “inner isolation” invariant in 07 + `verify_loose`/`verify_ssa` |
| **1b. Inconsistent semantics**                              | **Yes (implemented)**                                                    | Unified under `take_body_locals` in `src/middle/lower/scope.rs`: slicing according to `temp_high_water`, with the E3014 budget check and table extraction at the same point and carrying the function-body span, covering all six function-body forms; `MAX_REGISTERS` is defined exactly once across the repository                                                                                    | Corpus differential + zero-diff snapshot (empirically verified)             |
| **1c. Criteria for the three statement-level reclamations** | **Yes (pending batch b)**                                                | The three reclamations remain; their removal is performed in batch b together with 7b.4 (first provide an allocator that handles slot reuse); the `result_reg.is_none()` guard in `generate_block_ir` is removed at the same time                                                                                                                                                                       | C4: behavioral equivalence + invariants                                     |
| **2. Semantic reordering of `arg_regs`**                    | **Partially**                                                            | SSA turns “using the wrong register” into “using the wrong value ID,” which can be caught by dominance checks, but **the reordering logic itself remains**. Actual elimination depends on function decomposition—each of the six branches pushes independently, while ownership of `arg_regs` remains at the top of the function                                                                        | First-layer dominance invariant (snapshots are immune)                      |
| **3. Span-keyed failure**                                   | **No**                                                                   | SSA does not touch it. See “Disposition of Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                         | `test_release_plan_spans_consumed` in 07                                    |
| **4. Implicit ordering dependency**                         | **No**                                                                   | See the “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                           | New explicit criterion required                                             |
| 298-line `compile_pattern`                                  | No                                                                       | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                           | —                                                                           |
| 191-line `eval_const_expr`                                  | No                                                                       | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                           | —                                                                           |
| 75 `Instruction` variants                                   | No (+1, implemented)                                                     | `Phi` was introduced in 7b.1a; under option A it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                    | —                                                                           |

### Register Allocator: Custom Implementation or Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **zero construction points across the repository**—the variant is no
   longer present in the `Operand` enum in `ir.rs` (removed in 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper limit of 255 (anything beyond
   that produces E3014). Resolution of `Operand::Value` slots is pending 7b.2 and currently returns
   the explicit diagnostic E3017.
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50
   `translate_*` functions, with function bodies distributed across `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`, where opcodes come from
   `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` contains the interpreter’s opcode dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Evaluation of three options**:

| Option                                             | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both take rustc’s `Function` trait and MIR’s `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait (CFG, liveness, spill slots, scratch), requiring **500+ lines of adapter code for zero benefit**—because there is no real register file to color. Moreover, rustc itself only performs linear scan at this point (when `enable-llvm` is disabled) |
| `graph-alloc` family                               | **Not adopted.** Same reason, and the API is less stable                                                                                                                                                                                                                                                                                                                                                                       |
| **Custom linear scan (simplified Chaitin-Briggs)** | **Adopted.** What YaoXiang actually needs is to map each `ValueId`, in `values` order, to a non-conflicting `u8` stack slot (`OperandResolver` already has the `u8` contract and E3014 upper limit). This is essentially **maximum-clique coloring of non-overlapping live intervals**, which, without register-file constraints, degenerates to “allocate the first available slot in definition order”                       |

**Design judgment**: implement it in-house, targeting 500–1,000 lines (including the `values → Reg`
mapping, live-interval computation, and the `OperandResolver` interface). **At the same time, it
must be acknowledged that if the `u8`/255 slot limit is insufficient for real workloads, the custom
allocator must handle overflow, whereas the existing code already treats this as an error at
`1992-2000`.** This means that this document **does not solve** the problem of “a large function
does not have enough registers”—**that is a downstream consequence of `compile_pattern` recursion
depth and belongs to a separate topic.**

The dev-dependencies at `Cargo.toml:37-39` contain only `criterion` and `proptest`; `[dependencies]`
contains neither regalloc nor petgraph nor graphlib. Adopting any such crate would add a new
dependency surface.

### Decomposition Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA shape and criteria must exist first**. The reason is that
decomposition itself is C1 (a pure move, with zero-diff snapshots as the criterion), but only after
the semantics of `arg_regs` have been eliminated by SSA—down to “each argument has an independent
`ValueId`”—will decomposition avoid carrying the old ordering assumptions into the new file.

**Core design constraint: ownership of `arg_regs` remains at the top of the function, and the six
branches only `push`; they never reorder it.**

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

The “fill with 0” logic at `7835-7848` (`7838-7846`) is moved down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the “the normal
path should not reach here” self-description is replaced with an assertion**: if any slot is missing
and typecheck has not reported E1010, it directly returns a diagnostic. **This turns a silent
fallback into a hard error and is this document’s most direct improvement to defect class 2.**

**Criterion**: the pure-move portion (before the SSA shape changes) uses zero-diff snapshots;
assertion of `fill_missing` is C4, using corpus differentials and the first-layer verifier.

### The `synth.rs` Boundary: Turning Implicit Callbacks into an Explicit Synthetic-AST Boundary

**Verified fact**: the locations in `ir_gen.rs` that construct `ast::Expr` are **exactly
two**—`:4246` and `:6701` (there are also constructions of `ast::Stmt` at `:6704` and of `ast::Type`
at `:510` / `:1266` / `:1399`).

**Proposal**: create `src/middle/lower/synth.rs` as the **only module in the entire repository**
permitted to construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not
“tidiness,” but rather:

> Constructing an AST means **reentering the frontend-lowering entry point**. The `pending_env_vars`
> / `closure_captures` state expected by `generate_expr_ir` is a **precondition for which the caller
> is responsible**, but this precondition is currently expressed by neither the type system nor the
> signature—it holds only because of the runtime fact of “who called me.”

Change it to an explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST that captures env; the capture set is explicitly provided as parameters,
/// without relying on the caller first writing to self.pending_env_*
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Specific changes:

| Location                       | Current state                                                                                  | Replacement                                                                                                                                                                                                             |
| ------------------------------ | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → construct Lambda → call back                                        | Call `synth::synth_closure(...)` to obtain the AST and pass `SynthEnv` explicitly; `generate_lambda_expr_ir` (`6543`) gains an internal entry point carrying `SynthEnv` and **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → construct → call back → clear                                                          | Same as above; the `clear()` calls at `6713-6714` disappear because there is no shared state to clear                                                                                                                   |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                        | Read from the parameters instead                                                                                                                                                                                        |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                               | Same as above; `closure_captures` becomes a **local variable during closure-body generation**, and the `clear()` at `6600` disappears                                                                                   |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads shared state on the namespace-resolution path | **This is the most dangerous location**: make it an explicit parameter; otherwise any `?`-triggered early return from `generate_lambda_body_ir` leaks the capture table into unrelated code                             |

**Criterion**: add the sentinel test `test_no_ast_construction_outside_synth`, which scans
`src/middle/**` and requires that no `ast::Expr::` / `ast::Stmt {` construction syntax appear
outside `synth.rs`. Its shape follows the `check-module-boundary.py` mechanism in RFC-039.

### Disposition of Span-Keyed Problems (the Portion SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism one: consumption-coverage assertion (mandatory, lowest cost; superseded on 2026-10-13 by
a stronger form).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). This
document originally proposed implementing it as a **runtime counter** rather than only a test. The
implemented form is stronger than that proposal—a **consumption ledger (`ConsumptionLedger<K>`) and
a single settlement point (E-level `W1090`)**—without introducing a new diagnostic code or relying
on an internal counter in ir_gen to attest to its own correctness:

```rust
// Near ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each hit at :1942, increment self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* New diagnostic code: E3xxx NLL release plan was not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a value that should have been dropped was not dropped” does not necessarily
crash immediately under YaoXiang’s current value semantics and Arc/Rc model**—which is precisely why
it was silent. Counting turns “silence” into something “detectable.”

**Mechanism two: change span keys to `DefId` or an explicit plan ID (**implemented on 2026-10-13**:
RFC-039 D20 / WBS 4.5 implements this mechanism as a `PlanId(u32)` structural-identity key plus
mandatory consumption settlement; see the revision note in §2 of [02](02-stage-contract.md)).** The
fundamental fix is **not to use Span as a key**. `Span` is a source location and may drift due to
macro expansion, `include`, or multi-file merging. `FunctionIR` already has a `def: Option<DefId>`
field (`ir.rs:677`), demonstrating that DefId is available at the IR layer. **Design judgment**:
introduce `PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, assigned by ownership/overload
resolution when the plans are produced and matched by `PlanId` when the IR consumes them. **This
belongs to the obligations ledger (`Obligations`) in 02 and should be implemented together with
[02](02-stage-contract.md), not in this document.**

**Mechanism three: definition-order dependence of `method_def_ordinals` (an independent small
item).** This field is a `HashMap<String, usize>` (`ir_gen.rs:224`) whose key is
`"{type_name}.{method_name}"` (**without module qualification**, written at `:1691-1700`). Two
modules that define the same `Type.method` share the same counter, causing the `#N` mangled names to
drift. **Its failure mode is neither a span mismatch nor an ordering/name mismatch**—if typecheck
registration order differs from ir_gen definition order, same-named methods may resolve to different
targets. Disposition: have the typecheck side **produce `HashMap<DefId, String>` once** (bare names
or `#N` mangled names), and make ir_gen only look up, never modify, that map. **This eliminates one
mutable cross-function state from ir_gen.**

### Tracking the Three Hard-Coded Discarded Fields

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hard-coded discarded fields:

| Location           | Field                | Value                                          | Consequence                                                                                                                               |
| ------------------ | -------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | The closure upvalue count becomes zero after a `.42` round trip                                                                           |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **A `.42` artifact loses the entire exception table**                                                                                     |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **A `.42` artifact loses global-variable information** (note that `global_names` at `2347` **is** populated, so the two are inconsistent) |

**Design judgment (revised on 2026-10-05, decision D52): all three are brought into this document’s
scope as P7 batch e (7e), and no separate issue is created.** The original judgment—and the reasons
it was overturned—are recorded below:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields) and are
   unrelated to SSA.~~ This is true, but not grounds for exclusion—P7 already modifies
   `bytecode.rs`, and fixing all three together requires only one `VERSION` bump (4→5); splitting
   the work into two version bumps would be more expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → bumping the version → expanding scope
   from “IR shape” to “artifact format.”~~ The version-bump mechanism already exists (the `MAGIC` +
   `VERSION` header, with the reader rejecting unsupported versions), and `.42` is a build artifact;
   cross-version compatibility is not a goal. What expands is the work, not the risk.
3. ~~The actual risk levels of `2315` / `2341` are low, and running `.42` directly is a secondary
   path.~~ **Overturned**: losing the exception table makes throw/try behave incorrectly when `.42`
   is run directly—throw/try is a core language semantic. A secondary path does not imply that
   errors may be silently accepted. The principle of this refactor is **to leave no core
   functionality pending under a “not yet implemented” legacy** (`Vec::new() // Not implemented yet`
   is precisely such a legacy).

**At the same time, retain**: all three must still be recorded in the `Obligations` ledger as
“produced but not serialized” until 7e is complete, to prevent them from becoming a fourth kind of
“thing nobody knows about.” `2312` is implemented together with the version bump (D17), and the
already allocated but unused opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Knock-on Effects of the IR Structural Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                                                              |
| ------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Add the `ValueId(u32)` type                                               | File header (`ValueId` definition)            | New (**implemented in 7b.1a**)                                                                                      |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**implemented in 7b.1a**)                                                           |
| Add the `Instruction::Phi` variant                                        | End of the `Instruction` enum (`Phi` variant) | **Variant +1** (**implemented in 7b.1a**; the enum now has 75 variants)                                             |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | Field +1 (**implemented in 7b.1b**)                                                                                 |
| Add the `ValueInfo` structure                                             | `ValueInfo`                                   | New (**implemented in 7b.1b**)                                                                                      |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                    | Add `values()` / `values_mut()` accessors (**implemented in 7b.1b**, with the same shape as the existing accessors) |

The `pub use ...ast::Type` at the top of the file **is not changed by this document**—it belongs to
the scope of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **single converged
type**.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                          | Location                              | Nature                                                                                                     |
| ------------------------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr`, or adopt the “expand to Move at runtime” strategy | `127-554` (currently **66** variants) | Variant +1 or +0 (see below)                                                                               |
| Add a `Phi` arm to `opcode()`                                                   | `558`                                 | Mandatory (`BytecodeInstr` has an exhaustive match)                                                        |
| Add a `Phi` arm to `size()`                                                     | `649`                                 | Mandatory                                                                                                  |
| Add a `Phi` arm to the `From<BytecodeFile>` decoding match                      | `943-2350`                            | Mandatory                                                                                                  |
| Correct `upvalue_count: 0`                                                      | `2312`                                | Fixed incidentally by this document (in-memory format; persisting it requires a `VERSION` bump, see below) |

**The bytecode shape of `Phi` requires a design decision**. There are two options (**design
judgment: recommend A**, for the reasons given under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the start of the block,
  selecting the incoming value according to the predecessor block index. **No opcode is added**, and
  the `.42` format remains unchanged. The cost is increased bytecode size (one copy per
  predecessor).
- B: Add a `PHI` opcode and predecessor-index encoding, and have the interpreter maintain register
  versions when jumping. **This requires changing the `executor/` state model, the `.42` format, and
  the version number.** The only benefit is reduced size.

The decisive reason for choosing A is that changing the `.42` format triggers version-number issues
and a full compatibility discussion, **whereas A’s size cost is irrelevant in an
interpreted-execution scenario** (the executed instructions are Moves, not Phis).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions, distributed across `101-1558`)

| Change                                                          | Location                                                                                                                                 | Nature                                                                                               |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                                    | `OperandResolver` (`codegen/operand.rs`)                                                                                                 | Add an arm                                                                                           |
| Translation arm for `Instruction::Phi`                          | Inside the dispatch match, before `Free`(533)                                                                                            | New branch, expanded into a sequence of Moves (option A)                                             |
| Existing nine NOP degradations **remain in the same positions** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart + UnsafeBlockEnd / `658-660` PtrFromRef + PtrDeref + PtrStore + PtrLoad | **Verified as nine instructions**: `655-657` covers two variants, and `658-660` covers four variants |

**SSA transformation does not reduce the number of NOPs.** These nine degradations represent gaps in
expressiveness—“the IR has it, but the bytecode does not”—and SSA transformation does not touch
them.

#### `src/backends/interpreter/executor/`

Under option A, `executor/` requires **no changes whatsoever**—`Phi` disappears during the
`translator.rs` phase and is never visible to the interpreter. **This is the second decisive reason
for choosing A.**

`BytecodeInstr::Switch` has an active implementation at `executor/ops/control.rs:90`, but **no
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs`** (verified: zero hits for all three
names in `ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions that the IR layer
cannot produce. It does not affect this document, but it is recorded here because it shows that “IR
→ bytecode” is not surjective.

The dispatch match at `executor/debug.rs:194` likewise requires no changes (under option A, there is
no new opcode).

### Compiler Change List

**File by file and function by function.** All line numbers are verified change **starting points**:

| No. | File                                      | Location                                                | Change                                                                                                                         |
| --- | ----------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `src/middle/ir/ir.rs`                     | `Operand` enum definition                               | Add/remove `Operand` variants (**7b.1a implemented**: remove `Temp` / `Label` / `Register`; add `Value(ValueId)`)              |
| 2   | `src/middle/ir/ir.rs`                     | End of the `Instruction` enum (`Phi` variant)           | Add `Instruction::Phi` (**7b.1a implemented**)                                                                                 |
| 3   | `src/middle/ir/ir.rs`                     | `FunctionBody::Code`                                    | Add `values` (**7b.1b implemented**)                                                                                           |
| 4   | `src/middle/ir/ir.rs`                     | `ValueInfo`                                             | Add `ValueInfo` (**7b.1b implemented**)                                                                                        |
| 5   | `src/middle/ir/ir.rs`                     | `values()` / `values_mut()` accessors                   | Add accessors (**7b.1b implemented**)                                                                                          |
| 6   | `src/middle/ir/ir.rs`                     | `pub use ast::Type`                                     | **Do not change `pub use ast::Type`** (scope of [03](03-type-unification.md); recorded only)                                   |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                               | Return `Operand::Value`; increase monotonically without rollback                                                               |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                             | **Remove** statement-level reclamation                                                                                         |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                             | **Remove** statement-level reclamation (including the `result_reg.is_none()` guard)                                            |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                             | **Remove** statement-level reclamation                                                                                         |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                               | Replace with the RAII guard (`generate_method_ir`, entry point `1677`)                                                         |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                               | Replace with the RAII guard (`generate_function_ir`, entry point `1830`)                                                       |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                         | Replace with the RAII guard (`generate_curry_innermost_func`, entry point `2105`)                                              |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                               | Replace with the RAII guard (`generate_curry_function_ir`, entry point `2209`)                                                 |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                               | Replace with the RAII guard (`generate_anon_binding_ir`, entry point `2759`)                                                   |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                               | Replace with the RAII guard (`generate_lambda_body_ir`, entry point `4678`)                                                    |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                             | Extract the E3014 check into `check_register_budget()` for shared use by `1798`/`2001` after their semantics are unified       |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                                  | Change `take_cur_locals(param_types.len())` to `take_cur_locals(self.temp_high_water)`, eliminating the inconsistent semantics |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                               | Remove the contractual comment (now enforced by the type system) and replace it with a reference to `verify`                   |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                             | Change to call `synth::synth_closure`                                                                                          |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                             | Change to call `synth::synth_closure`; remove `6713-6714`                                                                      |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                             | Read `SynthEnv` from the parameters instead of using `mem::take` on shared state                                               |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                                    | Make `closure_captures` local; remove `clear()`                                                                                |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                                   | Change `closure_captures.contains_key` to an explicit parameter (**the highest-priority implicit dependency**)                 |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                          | Add consumption counting for span-keyed contracts                                                                              |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                             | Make `method_def_ordinals` read-only                                                                                           |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                             | **Last**: decompose into a top-level `CallArgs` + six `emit_*` functions                                                       |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                             | Replace the “fill with 0” fallback with a returned diagnostic                                                                  |
| 29  | `src/middle/lower/synth.rs`               | **New**                                                 | Synthetic-AST boundary                                                                                                         |
| 30  | `src/middle/ir/verify.rs`                 | **New** (first layer of [07](07-equivalence-oracle.md)) | `verify_loose` / `verify_ssa`                                                                                                  |
| 31  | `src/middle/passes/codegen/translator.rs` | Dispatch match                                          | Add an `Instruction::Phi` arm (option A: expand into a sequence of Moves)                                                      |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                       | Add an `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`)                                                        |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                               | **Unchanged under option A** (`Phi` does not enter the bytecode)                                                               |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                              | **Unchanged under option A**                                                                                                   |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                                  | Correct `upvalue_count: 0`                                                                                                     |
| 36  | `src/backends/interpreter/executor/**`    | —                                                       | **Zero changes under option A**                                                                                                |
| 37  | `src/middle/passes/regalloc.rs`           | **New**                                                 | Linear-scan allocator                                                                                                          |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New**                                                 | Sentinel check for the `synth.rs` boundary                                                                                     |

### Backward Compatibility: Version Number for the `.42` Format

**Verified fact**: the `.42` on-disk format **already has a version-number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (“YXBC”) and
`VERSION: u32 = 4`, writes them to the header (`:342-343`), and validates them while reading
(`:454-467`, reporting “unsupported bytecode version” on a mismatch).

**Design judgment (a direct consequence of option A)**:

- `Phi` is expanded into a sequence of `Move` instructions in `translator.rs` and **produces no new
  opcode**.
- The 83 constants in `opcode.rs` have **zero changes**, `opcode_name()` (`120`) is unchanged, and
  the decoding match (`bytecode.rs:943-2350`) is unchanged.
- The field layout of the `.42` format has **zero changes**, and `VERSION` remains 4. Old `.42`
  files can still be read by the new binary, and new `.42` files can still be read by the old
  binary. **SSA transformation itself does not require a version bump.**

**The only scenario requiring a version bump is the correction of `2312 upvalue_count`** (decision
D17: fix it, and handle it together with the version bump): add an `upvalue_count` field to
`BytecodeFunction` and make the encoder write it. This is a **new field in the format**, so bump
`VERSION` from 4 to 5. The reader’s existing version check rejects old files according to its
existing behavior—`.42` is a build artifact (`main.rs:671`), and cross-version read compatibility is
not a goal, so the version bump requires no additional migration mechanism. The already allocated
but unused opcode values (D32/D34’s `Switch` / `TailCall`) are reclaimed in the same version bump.

## Implementation Essentials

### Batch Strategy

Divide the work into four batches. **Every item within a batch has an independent commit,
independent criterion, and independent revert**. In the table below, “change items” refer to the
numbers in the preceding “Compiler Change List.”

| Batch       | Contents                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Change items                 | Criterion                                                                                                                                                           | Rollback point                                                                                                                                               |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Batch a** | Cut off multiple definitions: remove three statement-level reclamations → replace six save/restore sites with an RAII guard → unify semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 8–10 (first) → 11–16 → 17–18 | Removing reclamations: **C4** (behavioral equivalence + first-layer dominance invariants). RAII and semantic unification are pure moves: **C1** zero-diff snapshots | One commit per item; `git revert` is sufficient                                                                                                              |
| **Batch b** | Switch to the SSA shape: `ir.rs` structural changes + change temporary-value production to `Operand::Value` (mechanical replacement) → add the `Phi` arm to `translator.rs` → linear-scan allocator. Within the batch, proceed serially through the three-level WBS tasks: 7b.1a (`Operand` variant convergence + `Instruction::Phi` introduction, **implemented**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **implemented**) → 7b.2 (`next_temp_reg` produces `Operand::Value`) → 7b.3 (`Phi` arm in `translator.rs`) → 7b.4 (linear-scan allocator + remove the three statement-level reclamations, in the same batch as 7a.1) | 1–5, 7 → 31–32 → 37          | **C4**: behavioral equivalence + all `verify_ssa` checks green + `.42` round-trip tests                                                                             | The `ir.rs` changes **must occupy one exclusive commit** (the type changes affect compilation across the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary + sentinel script → span consumption counting → make `method_def_ordinals` read-only                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 20–24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + new script integrated into CI; the consumption counter must trigger zero new diagnostics (otherwise, a real mismatch exists)       | Independent commit                                                                                                                                           |
| **Batch d** | Decompose `generate_call_expr_ir` + assert the “fill with 0” fallback                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 27–28                        | Pure-move portion: **C1** zero-diff snapshots. Assertion portion: **C4** corpus differential + first-layer verifier                                                 | Independent commit                                                                                                                                           |

**The three dependencies whose order within the batches cannot be changed are:**

1. Batch a’s “remove reclamations” follows the `ir.rs` changes in batch b (the three statement-level
   reclamations are removed in 7b.4 together with the linear-scan allocator, with 7a.1 handled at
   the same time)—first allow `next_temp` to increase monotonically, so that `ValueId` has the
   semantic basis for “single definition” (this semantics becomes effective only after
   `next_temp_reg` produces `Value` in 7b.2 and the `Phi` arm is implemented in 7b.3).
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise the decomposition would carry
   the old `arg_regs` ordering assumptions unchanged into the new file.
3. Batch d must be last—it depends on the preceding batches having converged the semantics of
   `arg_regs` to “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within **P7** in RFC-039. The criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial**, but there
is no coupling between items 8–10, 11–16, and 17–18 within batch a; only ordering constraints apply.

### Preconditions (Hard)

**Precondition one: criterion baseline.** The first-layer verifier’s `verify_loose` mode from
[07](07-equivalence-oracle.md) **must run green on the existing non-SSA IR** before batch a may
begin.

The reason is that the criterion for SSA transformation is C4 (behavioral equivalence + invariants).
If the invariant verifier itself cannot run green on the pre-refactor IR, then every acceptance step
in batches a–d lacks an executable criterion and can rely only on corpus differentials—which are
**precisely insensitive** to defect classes 1/2/3 (as argued under “Alternative A” in 07). **This is
a real ordering constraint.**

**Precondition two: type-representation convergence.** [03](03-type-unification.md) must complete
the convergence of `pub use ast::Type` at `ir.rs:3` and remove the `From<MonoType> for IrType`
bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` introduces a third type-compatibility
implementation.**

### Rollback Strategy

- Each change gets an independent commit, with `git revert` granularity equal to change-item
  granularity.
- The structural changes to `ir.rs` in batch b are **the only part that cannot be rolled back
  incrementally** (the type changes affect compilation across the repository). They must therefore
  **occupy an exclusive commit**, and batch a must already be green with a revertible baseline
  before that commit.
- **Do not use a feature flag.** The reason is that adding/removing `Operand` variants cannot be
  isolated by a runtime switch; a feature flag would cause CI to stop testing the other path
  long-term—a recurrence of exactly what RFC-039 records as “test wiring depends on memory, and
  silent decay occurs.”

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front, or the work will be questioned midway through implementation on the
grounds that “the line count did not decrease.”**

| Item                                      | Line-count change                                                                                                                                                                          |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Starting point                            | `ir_gen.rs` **8,448**                                                                                                                                                                      |
| Remove three statement-level reclamations | **−15**                                                                                                                                                                                    |
| Replace six save/restore sites with RAII  | Net ±0 (the RAII guard definition adds about 20 lines, while the call sites fall from 22 lines to about 12)                                                                                |
| Transform `next_temp_reg`                 | +5                                                                                                                                                                                         |
| Decompose `generate_call_expr_ir`         | **+40–80** (the signature and parameter-passing overhead of six `emit_*` functions; this is the typical cost of a pure-move decomposition, and **decomposition increases the line count**) |
| Span-keyed consumption counting           | +15                                                                                                                                                                                        |
| Make `method_def_ordinals` read-only      | +20                                                                                                                                                                                        |
| New `synth.rs` file                       | +80–120                                                                                                                                                                                    |
| New `verify.rs` file                      | +400–600 (the first layer in 07 is within this document’s scope but is counted separately)                                                                                                 |
| New `regalloc.rs` file                    | **+500–1,000**                                                                                                                                                                             |
| **Total**                                 | **`ir_gen.rs` grows from approximately 8,448 to 8,600–8,800; including the new files, the net increase is 900–1,600 lines**                                                                |

The early expectation of “8,448 → 5,000–6,000” was **incorrect**; measured analysis produces the
opposite result. The reasons are:

1. **The core benefit of this document is not reducing the line count; it is eliminating an entire
   class of defects that “fail without reporting errors.”** The six manual save/restore sites (22
   lines) become about 12 lines plus a guard type after SSA + RAII—**the line count is almost
   unchanged, but “forgetting to restore” changes from a possibility into a compile error**.
2. **Pure-move decomposition necessarily increases the line count.** Decomposing the 746-line
   `generate_call_expr_ir` into seven functions requires repeating each signature (the function
   already has `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` bindings, and error
   propagation. **“Splitting the file to reduce the line count” is a misconception—it reduces
   cognitive load, not physical lines.**
3. **The new verifier and allocator are net line-count increases.** These two components
   (approximately 900–1,600 lines) are precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of line count, this document will most likely produce a **net increase**. The benefit
> lies in **verifiability**—turning defects that “fail without reporting errors” into invariants
> that “fail the build when violated.” If line count is used as an acceptance criterion, this
> document will fail; the correct acceptance criteria are those in the “Batch Strategy.”

**This phase intentionally increases the amount of code in `ir_gen.rs`.** The 2026-10-03 decision
canceled all line-count / artifact-size gates (see prohibition two in
[08](08-maintenance-mechanism.md)); therefore, **no line-count exemption list is needed**. The
legitimacy of the increase follows from the separation-of-responsibilities argument: SSA
transformation extracts register allocation and definition discipline from lowering into independent
modules, trading increased code volume for the elimination of six manual save/restore sites and the
`arg_regs` semantic reordering—two classes of manual discipline in which “misalignment does not
report an error and only produces an incorrect value.”

## Key Decisions and Rationale

### Three Core Decisions

**Decision one: temporary-value strategy—remove the three statement-level rollbacks so that single
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Removing them allows `next_temp` to
increase monotonically, so each temporary slot naturally has exactly one definition point. **The
criterion is consistent but the guards differ** (`3632` has an `result_reg.is_none()` guard while
the other two do not), and these hand-copied discrepancies disappear accordingly. The cost is
increased slot occupancy—`temp_high_water` already provides the 255 upper-bound check (`1992-2000`)
as a backstop.

**Decision two: do not perform mem2reg; perform explicit renaming only.** The rationale was given
under “Value Identification Method”: YaoXiang’s `&` / reference / ownership semantics make “which
slots can be promoted to memory” a type-system problem that requires the converged type
representation from [03](03-type-unification.md). **Performing mem2reg before 03 amounts to
guessing.** Explicit renaming only requires changing the return-value semantics of one
`next_temp_reg` location and removing three rollbacks, without touching the control flow of any
lowering algorithm.

**Decision three: use option A for `Phi` (an IR-only pseudo-instruction expanded into a sequence of
Moves).** `executor/` and the `.42` format have **zero changes**, and the regression surface is
confined to L3. The decisive reason is that changing the `.42` format triggers version-number issues
and a full compatibility discussion, whereas A’s size cost is irrelevant under interpretation (the
executed instructions are Moves, not Phis). The cost is that `Phi` is not visible in `dump_bytecode`
(when entering the C1/C2 phases, note that layer three of 07 must compare it).

### Rejected Alternatives

**A. Decompose everything at once (simultaneously perform SSA + decomposition + the `synth`
boundary + the allocator). Rejected.** This violates the core principle of 07: criteria must be
tiered by category. Mixing four batches into one PR combines pure-move changes with C4 changes,
making snapshot drift impossible to attribute—when the snapshot changes, there is no way to tell
whether the decomposition moved something incorrectly or whether SSA changed it. **Rollback
granularity degrades from “change item” to “everything.”**

**B. Split files first, then optimize ordering. Rejected.** Splitting does not eliminate any root
cause: six manual save/restore sites remain scattered across ten files, and the semantics of
`arg_regs` still require manual reasoning. Worse, **splitting first loses the opportunity to
“establish criteria before making changes”**—decomposition produces a large line-level diff that
gets mixed into the subsequent SSA diff, invalidating review. This has the same origin as the eight
orphan test trees recorded in 06-cleanup-inventory.md (1,005 lines / 78 tests that never ran):
**with the wrong order, every subsequent step accelerates on top of an error.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as “a third path that was once considered.”** The shape is to leave `ir_gen.rs` unchanged and add a
pass that converts its non-SSA IR into SSA IR. Its **advantage** is that lowering remains untouched
and rollback is easy. **Reasons for rejection**: (1) It does not eliminate the root cause of defect
class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the six save/restore sites
remain separated by 59–161 lines. A conversion pass can merge multiple definitions into `Phi`, but
**it cannot repair “names from the inner function leak into the outer function”**—that is a
`cur_locals` save/restore error that occurs before the IR shape, and the conversion pass cannot see
it. (2) It turns SSA into a second representation—the repository already provides a cautionary
example in its three parallel type representations, while this project’s core lesson is that
**“boundaries exist only in people’s awareness.”** (3) `verify_loose` becomes a permanent burden.

**D. Keep the current implementation and only add a verifier. Rejected as the sole solution, but it
is a component of this document.** RFC-039’s alternative B reaches the same judgment: gates can
prevent regression but cannot repair the current implementation. **However, it must be clear what D
can accomplish**: D can **catch instances** of defect classes 1/2/3 (provided the verifier covers
dominance, type consistency, and inner isolation), but it cannot catch defect class 4
(`closure_captures` leakage)—that requires a cross-function liveness assertion beyond the boundary
of `verify(&ModuleIR)`. **Therefore, D and this document are not alternatives; D is a
prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point**—the 2026-10-03 decision canceled line-count / artifact-size gates, and the scale is
  addressed through separation of responsibilities (see prohibition two in
  [08](08-maintenance-mechanism.md)). The increase is justified by replacing six manual save/restore
  sites and the `arg_regs` semantic reordering—two classes of manual discipline in which
  “misalignment does not report an error and only produces an incorrect value.”
- **The “single definition” check in `verify.rs` is meaningless in the non-SSA shape**
  (`Operand::Local` permits multiple definitions)—07 already records this limitation, and
  `verify_loose` is its degraded mode before this batch.
- **Removing statement-level reclamations changes the temporary-slot allocation pattern** (performed
  in batch b, with 7a.1 and 7b.4 in the same batch). Even if `temp_high_water` guarantees no
  overflow, **the allocation result changes**—which is precisely why the C4 phase’s “behavioral
  equivalence” criterion must run against the full corpus rather than relying on sampling.
- **The `u8` / 255 slot limit is insufficient after removing rollbacks (verified)**: after removing
  the three reclamations and running the full corpus, two corpus cases crossed 255 and triggered
  E3014—`src/std/tests/string_ops.yx` with 354 slots and
  `tests/yaoxiang/01-syntax/control-flow/list_comp.yx` with 323 slots; the peak slot count for all
  other corpus cases rose from 51 to 211. `take_body_locals` in `src/middle/lower/scope.rs` catches
  the overflow, but catching it means compilation fails. Therefore, removing the rollbacks is
  scheduled after the allocator (7b.4): first let the allocator handle slot reuse, and only then
  remove the rollbacks.
- **The benefits must also be recorded**: turning “errors that cannot be tested” into “invariants
  that can be tested”; no backend changes (`executor/` and `.42` have zero changes); no changes to
  the semantics of `Instruction` (all 76 → 77 variants are retained, and all 50 `translate_*`
  functions are retained); incremental delivery (each item can be reverted independently).

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> decisions (D1–D50). **This document leaves no pending items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching framework; four-layer model, criterion tiers, P1–P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definition of C4
  criteria, `verify_loose` / `verify_ssa`, three-layer criteria, and the limitation that snapshots
  are immune to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of three
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the
  `ReleasePlan` `PlanId` transformation
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — eight orphan test trees; the counterexample
  to “create the directory first and fail to wire up the tests later”

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

**Defect class 1 (multiple definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for the seven nested-function-body
  entry points (`Deref`/`DerefMut` + `Drop`)
- `take_body_locals` in `src/middle/lower/scope.rs` — sole table-extraction + E3014 budget point
  (`MAX_REGISTERS = 255`)
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `take_body_locals`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the true `temp_high_water` mark
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs`, and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — three statement-level register
  reclamations
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant
  referenced in 07

**Defect class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  the clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill with 0” fallback

**Defect class 3 (span-keyed contracts)** — the following are **audit-time** evidence pointers
(after the 4.5 key migration, both `release_plan` and `overload_resolutions` keys are
`PlanId`-based; see the revision note in §2 of [02](02-stage-contract.md) for current pointers;
`method_def_ordinals` remains a mechanism-three to-do item)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consuming `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write site of
  `method_def_ordinals` (key is the unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the propagation structures
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer propagation
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect class 4 (implicit ordering dependencies)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — the equivalent handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — the two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — three other `ast::Type` constructions

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — 66 `BytecodeInstr` variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded discarded fields (compare
  with `2344` / `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (the bridge awaiting removal
  in [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — nine NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, with a `u8` upper
  limit of 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (with no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current test coverage**

- `src/middle/core/ir_gen.rs` — 8,448 lines, zero `#[cfg(test)]` blocks, zero `mod tests` modules
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no IR
  structural-invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — four mentions of ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop` (an
  example per-opcode round-trip criterion; the final test in the file; the file contains 1,209 lines
  / 23 `#[test]` functions)

**Engineering configuration**

- `Cargo.toml:37-39` — dev-dependencies contain only `criterion` / `proptest`; `[dependencies]`
  contains neither regalloc nor petgraph nor graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans `/rfc/draft`; this
  document is not under that directory and is referenced by RFC-039 and the directory index
