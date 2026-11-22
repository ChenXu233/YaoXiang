# SSA Transformation of the Intermediate Representation

> **Companion design document.** This article is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the body of RFC-039 for the four-layer model, acceptance-criterion tiers, and execution-phase
> ordering; see the [index for this directory](index.md) for the positioning of each companion
> document.

## Positioning and Scope

This article addresses IR construction discipline at the L3 layer and proposes a construction method
based on “one definition per value” for `src/middle/core/ir_gen.rs` (8,448 lines). **The goal is not
to introduce a new instruction set, but to eliminate three classes of discipline defects that fail
without producing any error**:

1. Six hand-written register save/restore sites scattered dozens of lines apart;
2. Shared semantic reordering of `arg_regs` across six branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This article is responsible for: changes to the IR shape (`Operand` / `Instruction`),
temporary-value allocation discipline, making cross-layer contracts explicit, splitting
call-expression dispatch, and integrating an IR static verifier.

This article is not responsible for: converging type representations ([03](03-type-unification.md)),
stage contracts and the obligations ledger ([02](02-stage-contract.md)), defining equivalence
criteria and gates ([07](07-equivalence-oracle.md)), frontend paradigms
([05](05-frontend-paradigm.md)), or cleaning up dead code and empty promissory designs
([06](06-cleanup-inventory.md)). It repeatedly relies on the conclusions of those documents without
repeating their arguments.

**Criterion categories** follow **C4 (IR shape changes)** from
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, rather than exact equality of IR snapshots. SSA transformation necessarily changes the
IR shape; forcing snapshot equality would encourage the team to weaken the criteria, which is
precisely the trap recorded in 07.

**Its position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, accepted with behavioral equivalence and a green `verify_ssa`). The implementation
batch numbers within this article (batches a–d) are distinct from the ordering within P7: the former
are this article’s incremental strategy, while the latter is RFC-039’s global phase.

## Feasibility Premises

**Verified facts.** The IR already possesses **all structural prerequisites** required for SSA
construction:

| SSA prerequisite             | Current state of this repository                                                                             | Location                                                               |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                             | `BasicBlock` in `src/middle/ir/ir.rs`                                  |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                               | `FunctionBody` in `src/middle/ir/ir.rs`                                |
| Scope nesting information    | `LocalSlot::scope_depth` (`"0 = function parameter level, used to disambiguate shadowing in nested scopes"`) | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                           | `Instruction` in `src/middle/ir/ir.rs`                                 |
| Per-instruction span         | Every variant has a `span: Span` field                                                                       | Same as above                                                          |
| Value types can be annotated | `LocalSlot::ty: MonoType`                                                                                    | `LocalSlot` in `src/middle/ir/ir.rs`                                   |
| Slot-count upper-bound check | Table extraction and the E3014 budget gate share a single point (`MAX_REGISTERS = 255`)                      | `finalize_code_body` in `src/middle/lower/scope.rs` (from 7b.4 onward) |

**If these prerequisites were absent, the cost estimates in this article would be entirely
different.**

**The two decisive points:**

**First, SSA transformation is not a new capability; it removes a capability dependency.** Temporary
slots are currently **actively designed to receive multiple definitions**—`generate_function_ir` and
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs` explicitly roll back `next_temp` in three places. The first step
of SSA transformation is to **delete these three rollbacks**, making `next_temp` increase
monotonically. Once they are deleted, **every temporary slot naturally has exactly one definition
point**—unique definition changes from “a rule that must be checked” into “a mathematical
consequence of the construction method.” Batch a only converged the save/restore surface (see
“Defect Class 1”); the three rollbacks were removed together with 7b.4 in batch b (allocator
takeover of slot reuse) (2026-11-22).

**Second, the slot-count upper bound is a hard check, and deleting the rollbacks briefly caused slot
counts to become uncontrolled (resolved in 7b.4).** The budget gate in `src/middle/lower/scope.rs`
reports E3014 when the count exceeds 255. After deleting the three reclamation sites, running the
full corpus produced two cases that crossed the limit and triggered E3014—354 slots in
`src/std/tests/string_ops.yx` and 323 slots in `tests/yaoxiang/01-syntax/control-flow/list_comp.yx`;
the peak slot count in the remaining corpus rose from 51 to 211. **After 7b.4 landed**: the
linear-scan allocator reuses slots by live interval, and the two measured corpus cases use **15 / 24
frame slots** (344 / 304 values); the remaining corpus is behaviorally equivalent in 403/403 cases.
E3014 now means only “frame slots after allocation exceed 255.”

**The combined meaning of these two points**: lowering already maintains all bookkeeping information
required for SSA; it merely **reclaims** that information. 7b.4 makes the allocator responsible for
slot reuse and removes reclamation. The bulk of SSA-transformation work lies in splitting
`generate_call_expr_ir` and making cross-layer contracts explicit, not in building IR expressive
capability.

### Prerequisite: Unifying Type Representations

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The third representation is on the serialization side: `ir::Type`,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

In other words: **two type representations coexist in the IR, and one of them directly `pub use`s an
AST type.**

SSA transformation must add a `Phi` variant, record a type for every SSA value (for the verifier’s
“type consistency” invariant in 07), and determine whether the types of two predecessors at a CFG
merge are compatible. **If type representations have not converged, SSA will require a separate
type-compatibility check for every representation**—and the third representation will grow
accordingly.

This is not a question of “which should be done first,” but of **which approach would produce three
sets of type-compatibility logic**. [03 Type Representation Unification](03-type-unification.md)
must first complete the convergence of `pub use ast::Type` at `ir.rs:3` and remove the bridge at
`bytecode.rs:2353`. Item 6 of the change list explicitly records that `ir.rs:3` **is not changed by
this article**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not a file that is “ugly but functional.” It is a file that **bases correctness on
meticulous human line-by-line review**:

- The file contains **0 `#[cfg(test)]` blocks and 0 `mod tests` modules** (measured).
- `src/middle/core/tests/mod.rs` declares the `bytecode` / `def_assign` / `local_slots` modules,
  with 29 tests in total. `def_assign.rs` and `local_slots.rs` provide **positive** coverage of
  ir_gen’s DefId allocation and local-slot naming (sentinel-level), while the four references to
  ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **all negative assertions**—“ir_gen
  does not produce,” “ir_gen does not construct,” and “ir_gen’s frontend does not construct the
  corresponding IR.”

In other words, ir_gen has 29 sentinel-level tests pinning local behavior, but **no IR
structural-invariant validation** (dominance, unique definition, jump targets, type
consistency)—none of the three modules declared in `middle/core/tests/mod.rs` makes positive
assertions about the IR’s overall shape.

This file also happens to contain several classes of defects that **compile successfully, pass all
checks, and let the program run, but produce incorrect values**. Their common property is: **failure
produces no error**.

This is where the highest risk lies. It is not that “the function is too long,” nor that “the naming
is confusing,” but that **the file’s core invariants have no machine-executable enforcement
mechanism**.

The contract “save before generating a nested function body, then restore it afterward; otherwise
names from the inner function will leak into the outer function” is enforced by the type system:
`NestedBodyGuard` in `src/middle/lower/scope.rs` exclusively holds `&mut` access to the generator,
and frame extraction and restoration each have only one implementation (see “Defect Class 1”).

**Why adding a verifier is not enough**: a verifier can **catch** these defects, but it cannot catch
their **causes**. The cause is not “someone overlooked it during review,” but:

> **The current IR shape itself permits a slot to be defined multiple times, and ir_gen’s
> temporary-register allocator actively depends on this property.**

The concrete evidence is three statement-level register reclamations (`generate_function_ir` /
`generate_block_ir` in `src/middle/lower/stmt.rs`, and `generate_lambda_body_ir` in
`src/middle/lower/expr/closure.rs`)—at the end of every statement, they roll `next_temp` **back** to
the greater of “the last named slot ⋈ `temp_floor`”
(`self.next_temp = named_watermark.max(self.temp_floor);`). This means that the same
`Operand::Local(n)` slot is **written repeatedly within the same function body**, and both the
number and order of writes and the live ranges are determined entirely by one manual decision: when
to roll back.

Under this shape, “unique definition” is not a rule that can be applied incrementally; it **directly
conflicts with the register-allocation strategy**. SSA transformation therefore is not merely a
check added to existing code; it **replaces the allocation strategy that depends on multiple
definitions**. The good news is that replacing this strategy is inexpensive—see “Feasibility
Premises” above.

### Defect Class 1: Multiple Definitions and Manual Save/Restore (Three Reclamation Sites Removed in 7b.4)

**Verified facts.** There are seven entry points that require save/restore when entering a nested
function body. Batch a (7a.2) converged them into one RAII guard: `NestedBodyGuard<'a>` in
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

- Frame extraction occurs only in the two constructors (`enter` / `enter_decl`), while restoration
  occurs only in `Drop`. The guard exclusively holds `&mut` access to the generator for its
  lifetime, so no second `&mut` is available in scope; `?` and early returns all restore state
  through `Drop`.
- The fields involved are `next_temp` / `temp_high_water` / `cur_locals` / `cur_span` /
  `loop_stack`, carried by `SavedFrame` in `src/middle/lower/scope.rs`; from 7b.4 onward,
  `next_temp_reg` only maintains the named high-water mark (temporary-value slots belong to the
  linear-scan allocator; `temp_floor` was removed together with the three reclamations).
- `enter_decl` does not roll back the three counters: after a function body or method body returns
  to the module initialization flow, subsequent definitions continue from the child body’s final
  value number. This is the constraint for C1 snapshot zero drift and the meaning of “the
  declaration side rolls back only the table.”

**Unified accounting and the single budget point (from 7b.4 onward)**: `finalize_code_body` in
`src/middle/lower/scope.rs` is the sole table-extraction and budget point—it extracts the table
using `frame_slots = named_base + max_live`, reports E3014 with the function-body span when out of
bounds, and defines `MAX_REGISTERS = 255` in exactly one place in the repository. All six
function-body forms (function / method / lambda / anonymous binding / intermediate curry layer /
innermost curry layer) are finalized through it, so there is no second definition of “how many slots
a function body should have.”

**The three statement-level reclamations were removed (7a.1 landed with 7b.4, 2026-11-22)**: the
reclamations had been hand-copied in `generate_function_ir` / `generate_block_ir` (with a
`result_reg.is_none()` guard) / `generate_lambda_body_ir`; after the allocator took over slot reuse,
the reclamations degenerated into identity operations, so the guards, `temp_floor` /
`saved_temp_floor`, and the loop-entry/loop-exit bookkeeping in `loops.rs` were all removed. Module
initialization and top-level statements still follow the legacy monotonic-slot path with
`value_mode=false` (the allocator is not run), so their slot numbers and table lengths shift with
the number of named slots in the function body (three snapshots measured; behavioral equivalence is
covered by 403/403).

**Consequences (current shape)**: the guard exclusively holds `&mut`, restoration has only one
implementation in `Drop`, and there is no writable location where a restore can be accidentally
omitted; the temporary-slot dependency on “one slot receives multiple definitions” has disappeared
together with the reclamations—value slots are reused by the linear-scan allocator according to live
intervals, and `verify_ssa`’s unique-value-definition criterion now has a construction-side
foundation.

### Defect Class 2: Semantic Reordering of `arg_regs` in `generate_call_expr_ir`

**Verified facts.** `generate_call_expr_ir` is at `ir_gen.rs:7269-8014`, **746 lines** long in a
single function. Its signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (and also `_expr` / `result_reg` / `instructions` /
`constants`). Six branches internally share the same `Vec<Operand>` and perform **semantic
rewriting**:

| Line        | Semantics                                                                                                                                                |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flattened according to `slots`, then re-`collect`ed—named-argument reordering                                                            |
| `7724`      | Struct construction: `unwrap` each slot according to `final_args`, then re-`collect`ed—field reordering                                                  |
| `7835-7848` | Function call: named-argument reordering; `7838-7846` **adds a new register containing 0** for each missing slot (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();`—cloned before emitting instructions                                                                   |

The original form of the four reorderings:

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
reports E1010. As a runtime safety measure, fill in 0 (the normal path should never reach this).”

**This design has two risks**:

1. **Misalignment does not report an error; it only produces a wrong value.** Each of the six
   branches independently determines the final order of `arg_regs`, and no incorrect ordering
   assumption causes a panic.
2. **Normalized snapshots are immune to this problem.** 07 already records this limitation: after
   the snapshot tool renames temporary values by “first-occurrence order,” “the third argument
   should have used the fifth register” looks exactly the same in the snapshot. **At the C4 stage,
   this class of defect is not even covered by the snapshot criterion.**

**Additional verification: the struct-construction path has no duplicate-slot guard when
reordering.** `final_args` at `7724` is populated by “placing positional arguments first, then
overwriting by field name for named arguments,” and **there is no duplicate-slot check such as
`slots[idx].is_some()` before overwriting** (unlike the namespace path at `7381`)—when a named
argument silently replaces a positional argument, there is no panic or diagnostic. Furthermore, the
**argument evaluation order** (source order) diverges from the final `arg_regs` order
(field-declaration order) on this path. The `fill_missing` assertions in batch d must also cover the
duplicate-slot check; otherwise, the silent replacement in the struct-construction path will survive
unchanged.

### Defect Class 3: Silent Failure of Span-Keyed Cross-Layer Contracts (**Eliminated**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified facts (at audit time).** The repository has **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                                                                                                                 | Production                                                                                                                     | Transfer                                                                             | Consumption                                                                                                                     |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                                   | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatches `walk_stmt` by statement identity)            | `typecheck/types.rs:38` ← passed by `checker.rs` ← `ir_gen.rs:360` (clone read site) | `ir_gen.rs:3103` (`ReleasePlan::consume`, sole entry point; settlement-completion marker at `:1095`)                            |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table and consumption ledger in the same structure, `types.rs:62`) | `inference/expressions.rs:4550` (writes by call-site `PlanId`; `inference/statements.rs` only passes it through and merges it) | `typecheck/types.rs:62` ← assembled by `checker.rs` ← held by an `ir_gen.rs` field   | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; settlement-completion marker at `:1097`) |

The shape at audit time (`ir_gen.rs:1942`, **now nonexistent**):

```rust
// NLL Release: insert a Drop instruction at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

With `if let Some`—**if the lookup fails, it simply fails: there is no `else` branch, no assertion,
and no counter**. If the two sides compute spans differently in any respect, the **Drop instruction
silently disappears, with zero errors**. The Drop sequence of refinement types is a core carrier of
this project’s ownership semantics (`ownership.rs:26-32` describes the “NLL precise release plan”),
yet its implementation was silent.

`7420` / `7493` use the same pattern (`if let Some(mangled) = self.overload_resolutions.get(span)`),
causing overload resolution to fall back to the default binding.

**Remediation (landed in 4.5)**: both keys were changed to the structural identity `PlanId`, and
consumption **records a hit in the same operation as the lookup**. A non-empty difference is
reported at the sole settlement point as an **E-level `W1090`**—“not found” is no longer silent. See
the revised note in §2 of [02](02-stage-contract.md) (4.5) for the implemented form and criteria;
these line numbers (`1942` / `7420` / `7493`) are retained as evidence from the audit point and are
not pointers to the current implementation.

> **`method_def_ordinals` is not span-keyed.** It is a `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with its sole write site at `:1692-1702` (assigning `#N` suffixes in
> definition order by `base_name`). Its risk is **different**: dependence on definition order. The
> `#N` suffix is determined by AST definition order, so if generation order differs from
> typechecking registration order, the same function name may resolve to a different target. This is
> not a span mismatch, but it likewise “fails without reporting an error.” Its remediation is
> covered by mechanism three under “Handling of Span-Keyed Problems.”

### Defect Class 4: Implicit spawn/lambda Handshakes

**Verified facts.** `ir_gen.rs` has **two** sites that construct `ast::Expr::Lambda` themselves and
call back into `generate_expr_ir`, using a set of mutable fields as an implicit handshake:

**Handshake A — `generate_spawn_for_ir` (entry point `4143`):**

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir` (entry point `6543`) → `6571-6572`
`std::mem::take(&mut self.pending_env_vars)` consumes the value.

**Handshake B — `generate_spawn_expr_ir` (entry point `6641`):**

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**Handshake C — `closure_captures`**: written at `6589-6593`, which calls `generate_lambda_body_ir`
(entry point `4678`) **in the same function**, and cleared at `6600`. It has two read sites: `:6284`
(`LoadUpvalue` in the closure body) and **`:458`** (`closure_captures.contains_key(head)` on the
`flatten_namespace` path).

The `:458` read site is the most dangerous: it lies on the **namespace-resolution path**, meaning
that the live state of a `HashMap<String, usize>` can change variable-name resolution. A complete
closure-body generation (`6598`) occurs between the write at `6589` and the clear at `6600`; if any
path inside `generate_lambda_body_ir` returns early without clearing the table (it has multiple
return points when `?` propagates errors), **`closure_captures` will remain in the unrelated code
that follows at `:458`**.

Other implicit state includes `pending_env_names` (written by handshake B at `6699`, cleared at
`6714`).

### What SSA Will Not Solve (Listed Honestly)

| Problem                                              | Location                                                                                                                                | Why SSA does not solve it                                                                                                                                                                                                               |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                         | `ir_gen.rs:5390-5687`, **298 lines**, recursively calls itself 4 times (`5473` / `5504` / `5619` / `5640`), external call site `5270`   | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **intrinsic to the algorithm**. SSA only turns guards into `Phi` instead of `JmpIfNot`; it does not reduce the number of guards |
| `eval_const_expr` complexity                         | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming recursion)                      | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                                         |
| Span-keyed failure (**eliminated**)                  | Audit-time evidence: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`); after 4.5, `PlanId` keys + mandatory settlement | **SSA does not touch this layer at all** (the conclusion remains unchanged). It was independently resolved by the obligations-ledger mechanism in [02](02-stage-contract.md); see mechanism two under “Handling of Span-Keyed Problems” |
| Number of `Instruction` variants                     | 75 (`Instruction` enum in `ir.rs`; includes the `Phi` introduced by 7b.1a)                                                              | SSA adds `Phi`, for a **net change of +1** (already landed). Each variant maps to a bytecode opcode or is degraded to NOP; SSA does not reduce the count; under option A, `Phi` is an IR-only pseudo-instruction and adds no opcode     |
| `method_def_ordinals` dependence on definition order | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                                   | Similar to but distinct from span-keying; see mechanism three under “Handling of Span-Keyed Problems”                                                                                                                                   |

## Target Design

### Definition of the SSA Shape

**Design decision.** The target shape is as follows.

#### How the `Operand` Variants Change

The `Operand` enum in `ir.rs` has 5 variants. **Repository-wide measurements** (hit counts obtained
by summing `git grep -c` per file over all of `src/`, including tests; construction-point counts
refer to value-position constructions):

| Variant             | Repository-wide measurement                                                                                                                                                                          | Disposition                                                                                    |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 35 files / 148 hits                                                                                                                                                                                  | **Retained**, unchanged                                                                        |
| `Value(ValueId)`    | 7 files / 26 hits; 0 production construction points when introduced by 7b.1a; from 7b.2 onward, `next_temp_reg` is the sole production entry point (`src/middle/lower/scope.rs`)                     | **Retained** (introduced by 7b.1a)                                                             |
| `Local(usize)`      | 29 files / 417 hits                                                                                                                                                                                  | **Retained, but semantics split** (see below)                                                  |
| `Arg(usize)`        | 8 files / 11 hits                                                                                                                                                                                    | **Retained**, as the implicit definition at function entry (a parameter is defined in block 0) |
| `Global(usize)`     | 16 files / 26 hits                                                                                                                                                                                   | **Retained**, unchanged                                                                        |
| `Temp(usize)`       | **0 construction points** (the handling arm only exists in `OperandResolver::to_reg` and `is_register` in `codegen/operand.rs`, and is removed with the variant)                                     | **Removed** (dead variant, 7b.1a)                                                              |
| `Label(usize)`      | **0 construction points**                                                                                                                                                                            | **Removed** (dead variant, 7b.1a)                                                              |
| `Register(u8)`      | **0 construction points** (the comment at the original variant declaration said "Added for codegen", but codegen uses `OperandResolver` in `codegen/operand.rs` and does not construct this variant) | **Removed** (dead variant, 7b.1a)                                                              |

`Local(usize)` is split into two variants because it currently **serves two mutually exclusive
semantics**—“storage location for a named variable” (which may be written multiple times) and
“compiler temporary register” (which should be written only once):

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
densely numbered within a function and **monotonically increasing, never rolled back**. The split
and `ValueId` were landed by 7b.1a (`Operand` enum in `src/middle/ir/ir.rs`); the production side
(`next_temp_reg` changed to produce `Operand::Value`) was landed by 7b.2 (see the production form
under “Value Identification”).

#### `Phi` as a New `Instruction` Variant

```rust
/// Merge point: each predecessor basic block provides one definition of this value
/// incoming is sorted by predecessor block label (guaranteed at construction for easy normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (see the first layer of [07](07-equivalence-oracle.md)):

1. `incoming.len() ==` the number of predecessors of the block.
2. The label set of `incoming` equals the label set of all blocks in `blocks` whose `successors`
   contain this block’s label.
3. All `incoming` values have the same `MonoType` (**dependent on the type representations having
   converged in 03**).

The implementation status of `verify_ssa` (the first three constraints were completed in 7b.1b):
constraints 1 and 2 require the value table and block-level CFG, and were landed in **7b.1b** as
separate criteria for “block-level predecessor count” and “predecessor label set”—the predecessor
set is inferred from `BasicBlock.successors` (`block_predecessors` in `src/middle/ir/verify.rs`); a
count mismatch reports only the count, while a set mismatch reports the set, so the same defect is
not reported twice. Constraint 3, per-input type consistency, landed at the same time and obtains
its values from the value table `values[v].ty`, **reusing the existing `types_compatible`** (no
third type-compatibility check was introduced; convergence of type representations in 03 remains
“Prerequisite Two (hard),” and `ValueInfo::ty` will still be `MonoType` at that point). All three
criteria use the violation category `InvariantKind::PhiConsistency`.

**Three forms for which a type is unavailable (explicitly registered, not silently allowed)**: an
empty value table means that no basis for judgment exists (the constant shape of module
initialization / top-level statements, on the `value_mode=false` path); a non-empty value table with
an out-of-range `ValueId` is a violation; and a non-`Value` operand (`Const` / `Local` / `Arg` /
`Global`) has no value-table type source and is not judged.

**Measured prerequisite (fulfilled in 7b.3)**: before 7b.3, every repository-wide production
construction of `BasicBlock.successors` used `Vec::new()` (block-level predecessors were always
empty, so synthetic fixtures carried the predecessor-count and predecessor-set criteria). **From
7b.3 onward, block-level `successors` are genuinely populated by `partition_blocks` in
`src/middle/ir/ir.rs` at every production construction site** (all 6 in `lower` and 1 synthetic
function-body constructor in codegen use this sole entry point; lowering products contain
multi-block loops/branches, multiple outgoing edges, and back edges). The predecessor-count and
predecessor-set criteria therefore operate on real IR data—the direct-driver tests on real lowering
products pin this registration-to-consumption loop. The production construction points of `Phi`
remain at 0 (production belongs to a later batch), and will be directly subject to this criterion
when implemented.

7b.1a only introduced the variant: there were 0 lowering production construction sites (there are
still 0; production belongs to a later batch). The consumer side in `translator.rs` landed in
**7b.3**: under option A, it expands at block entry into a sequence of `Move` instructions (the
group is placed at the merge block’s entry, existing `JMP`s skip between groups, and incoming-edge
jumps are redirected to the group start by predecessor), while the `Phi` arm of
`translate_instruction` is an unreachable E8001 defensive arm (the `executor` never sees `Phi`).

**Remediating the guard issue at `3632-3638`**: the reclamation in the current `generate_block_ir`
has an `if result_reg.is_none()` guard (`3626-3631` comment: a block whose expression operand slot
may be held by the outer scope as a sibling argument temporary that lives across blocks). After SSA
transformation, this guard is **no longer necessary**—temporary values are no longer rolled back,
and a sibling argument that lives across blocks holds its own `ValueId`, which cannot be affected by
a rollback. **This is a place where SSA transformation can genuinely simplify the code, rather than
merely rename it.**

#### Value Identification

**Design decision**: do not perform memory promotion (mem2reg), do not use virtual-register
numbering, and **perform explicit renaming only**. The reasons are:

- YaoXiang’s `&` / reference / ownership semantics make “which slots can be promoted to registers” a
  **type-system problem**, not a CFG problem. `LocalSlot::ty: MonoType` in `ir.rs` has been verified
  to carry complete type information, but deciding “whether this `MonoType` can be promoted to a
  register” requires the converged type representation from 03 and coordination with refinement-type
  rules. **Performing mem2reg before 03 is guessing.**
- Explicit renaming only requires changing the return-value semantics of one function
  (`next_temp_reg()` in `src/middle/lower/scope.rs`) and deleting the three rollbacks; it **does not
  touch the control flow of any lowering algorithm**.

Value-table shape (**landed in 7b.1b**): `FunctionBody::Code` gains `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot); None means a pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block containing the definition
}
```

The accessors mirror the existing accessors (`impl FunctionIR` in `src/middle/ir/ir.rs`): `values()`
is read-only and returns an empty table for TypeDecl (mirroring `locals()`); `values_mut()` is
mutable and panics for TypeDecl (mirroring `blocks_mut()`).

**Production form (landed in 7b.2)**: `next_temp_reg()` returns `Operand::Value(ValueId)` and
registers one `ValueInfo` entry in the current function body’s value table (`next_temp_reg` in
`src/middle/lower/scope.rs`). The implemented details are:

- **Counter isolation**: `ValueId` is issued by an independent `next_value: u32` counter. Each
  function body starts at 0, increases monotonically, and is never rolled back; `NestedBodyGuard`
  saves and restores it together with the value table (one set per function body); the three
  statement-level reclamations roll back only `next_temp` (slots) and do not touch `next_value`;
  each curry layer resets the counter (one `FunctionIR` per layer). Module initialization and
  top-level statements have no value table—when `value_mode=false`, `next_temp_reg` degrades to the
  legacy `Local` slot allocation, leaving the initialization sequence unchanged.
- **Type source** (uses only information already available during lowering; no new type classifier
  is introduced): literals use `ConstValue::mono_type()` (the constant-type mapping is converged
  with the verifier’s mapping into a single source); variables/calls query the typechecking result
  and the bound function’s return type at the `generate_expr_ir` entry point;
  comparison/logical/string-family/forward-propagation cases are filled in at function-body
  finalization according to instruction semantics. When unavailable, they explicitly use
  `MonoType::TypeRef("?")` (no silent `Int(64)` placeholder; `LocalSlot.ty` is never read). Across
  232 snapshots, 9,425 values were measured: 7,080 had real types and 2,345 were explicitly unknown
  (24.9%), concentrated in call/field/index/container-construction forms.
- **`defining_block`**: takes the current block label during generation; **block splitting landed in
  7b.3** (a real lowering `while` body contains multiple blocks), but the field’s value is still the
  generation-time `cur_block_label` (currently always 0), so it does not yet match the real defining
  block; 7b.4 did not change this field when regenerating the 232 snapshots (the `@block` section
  under `values` did not drift). Correcting it must be done in the same batch as regenerating the
  `values` section and is **explicitly handed over to the `Phi` production batch**.
- **`debug_name`**: always `None` (named variables remain `Local` and do not move into the value
  table).
- **Indexing**: `value_slots: Vec<usize>` (index = `ValueId`) is parallel to `values` and records
  the value-to-frame-slot mapping; from 7b.4 onward, it is recomputed by the linear-scan allocator
  in `src/middle/lower/regalloc.rs` according to live intervals
  (`frame_slots = named_base + max_live`), and codegen’s `OperandResolver` only consumes it rather
  than allocating slots.

`values` and `locals` coexist: `locals` holds addressable named variables (which may have multiple
definitions), while `values` holds single-definition SSA values. **The presence of `locals` does not
violate SSA**—it is the “storage” portion outside SSA (LLVM’s alloca model).

**Division of responsibility with `LocalSlot::origin`**: these are **two separate dimensions** and
must not be conflated. `LocalSlot::origin` (the **slot-identity** field landed on the origin side in
2.1.4, recording the generation `FrameId` to which the slot belongs) answers “which frame owns this
slot?”, while `ValueInfo.defining_block` answers “in which basic block is this SSA value defined?”
Value identity belongs to `ValueInfo`, and slot identity belongs to `LocalSlot::origin`; 7b.1b only
adds the value table and **does not change `LocalSlot::origin`** (zero changes measured).

#### Estimated Change Volume

| Item                                                         | Estimate                                  | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------------------------------------------ | ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change `next_temp_reg`                                       | ~15 lines                                 | Single function in `src/middle/lower/scope.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Delete three statement-level reclamations                    | −15 lines                                 | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Consolidate 6 save/restore sites into RAII                   | Net ±0 lines                              | See “Item-by-Item Disposition Table”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Changes to `ir.rs`                                           | +120–180 lines                            | `ValueId` / `Phi` / `ValueInfo` / value table; **the first two landed in 7b.1a and the latter two in 7b.1b** (measured: `src/middle/ir/ir.rs` 7b.1a +37/−3, 7b.1b +36/−0 ⇒ cumulative +73/−3; `normalize.rs` +22/−5; `verify.rs` 7b.1a +63/−14, 7b.1b +162/−13 ⇒ cumulative +225/−27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Add a `Phi` arm to `translator.rs`                           | +25–40 lines (pending for 7b.3)           | One of the existing 50 `translate_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `executor/`                                                  | **Zero changes**                          | Option A: `Phi` is an IR-only pseudo-instruction, expanded into a Move sequence by `translator.rs`, and does not enter bytecode (Decision Three)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Call sites that produce temporary values as `Operand::Value` | Mechanical replacement (pending for 7b.2) | **Measured** (`git grep -c` summed per file over all of `src/`, including tests): `Operand::Local(` has 415 occurrences in 28 files, including 332 in 14 files under `src/middle/lower/`; the production point for temporary values is the single `next_temp_reg`. **Measured in 7b.2 (baseline revision retest and net landed volume)**: 140 call sites of `next_temp_reg()` in 14 files, 35 functions with a `result_reg: usize` parameter chain plus 3 `Option<usize>` result slots, and 342 `Operand::Local(` occurrences under `src/middle/lower/`; line-by-line diff across `src/` totals +23319/−12495, of which regenerating 232 IR snapshots accounts for +21866/−11950, while source and tests net +1453/−545 (mechanical replacements plus normalize rendering, mono projections, and six direct-driver tests; this also includes 5 generated error artifacts committed alongside) |

### Item-by-Item Disposition Table

| Defect class                                                | Eliminated by SSA?         | Disposition                                                                                                                                                                                                                                                                                                                                                                                                      | Criterion                                                                         |
| ----------------------------------------------------------- | -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| **1. Multiple definitions and manual save/restore**         | **Yes (resolved in 7b.4)** | The 7 nested-function-body entry points are consolidated into `NestedBodyGuard<'a>` in `src/middle/lower/scope.rs` (`Drop` restores `cur_locals` / `loop_stack` / `cur_span` and the counters), turning “forgot to restore” into a compile error; the three statement-level reclamations were removed with 7b.4 (the allocator reuses slots by live interval), and temporary values no longer occupy `next_temp` | First-layer “inner-scope isolation” invariant in 07 + `verify_loose`/`verify_ssa` |
| **1b. Inconsistent accounting**                             | **Yes (landed)**           | Unified in `finalize_code_body` in `src/middle/lower/scope.rs`: the table is extracted according to `frame_slots`, and the E3014 budget check and table extraction share the same point and carry the function-body span; all six function-body forms are covered; `MAX_REGISTERS` has one repository-wide definition                                                                                            | Corpus differential + snapshot zero drift                                         |
| **1c. Criteria for the three statement-level reclamations** | **Yes (landed in 7b.4)**   | The three reclamations, the `result_reg.is_none()` guard in `generate_block_ir`, and the `temp_floor` bookkeeping were deleted; slot reuse belongs to the linear-scan allocator in `src/middle/lower/regalloc.rs` (`frame_slots = named_base + max_live`)                                                                                                                                                        | C4: behavioral equivalence (403/403) + invariants                                 |
| **2. Semantic reordering of `arg_regs`**                    | **Partial**                | SSA turns “using the wrong register” into “using the wrong value ID,” which dominance checks can catch, but **the reordering logic itself remains**. Actual elimination requires function decomposition: each of the 6 branches performs `push`, while ownership of `arg_regs` remains at the top of the function                                                                                                | First-layer dominance invariant (immune to snapshots)                             |
| **3. Span-keyed failure**                                   | **No**                     | SSA does not touch it. See “Handling of Span-Keyed Problems”                                                                                                                                                                                                                                                                                                                                                     | `test_release_plan_spans_consumed` in 07                                          |
| **4. Implicit ordering dependency**                         | **No**                     | See “`synth.rs` Boundary”                                                                                                                                                                                                                                                                                                                                                                                        | New explicit criterion required                                                   |
| `compile_pattern`, 298 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                    | —                                                                                 |
| `eval_const_expr`, 191 lines                                | No                         | See “What SSA Will Not Solve”                                                                                                                                                                                                                                                                                                                                                                                    | —                                                                                 |
| 75 `Instruction` variants                                   | No (+1, landed)            | `Phi` was introduced by 7b.1a; under option A, it is an IR-only pseudo-instruction and adds no opcode                                                                                                                                                                                                                                                                                                            | —                                                                                 |

### Register Allocator: Custom Implementation or Existing Crate?

**Verified facts (decisive)**:

1. `Operand::Register(u8)` has **0 construction points repository-wide**—the variant is no longer in
   the `Operand` enum in `ir.rs` (removed by 7b.1a).
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local` and `Operand::Arg` to `u8`, with an upper bound of 255 (exceeding it
   reports E3014); the 7b.4 allocator resolves `Operand::Value` to frame slots through `value_slots`
   (`>255` reports E3014; no mapping reports E3017 with the instruction span).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1,577 lines, 50
   `translate_*` functions distributed over `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`, with opcodes from `src/backends/common/opcode.rs`
   (83 constants).
4. `src/backends/interpreter/executor/ops/` contains the interpreter’s opcode-dispatch family.

**Conclusion**: the so-called “registers” are **operand-stack slot indices, not physical
registers**. The target machine **has no finite register file, no register pressure, and no need for
spilling or splitting**.

**Evaluation of three options**:

| Option                                             | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (from the rustc lineage)  | **Not adopted.** Both take rustc’s `Function` trait and MIR’s `Place`/`Operand` abstractions as input. An adapter for YaoXiang IR would need to implement the complete trait set (CFG, liveness, spill slots, scratch), exchanging **500+ lines of adapter code** for zero benefit—there is no real register file to color. Moreover, rustc itself only uses linear scan at this point (when `enable-llvm` is disabled)                                                                                                                     |
| `graph-alloc` family                               | **Not adopted.** Same reason, plus a less stable API                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| **Custom linear scan (simplified Chaitin-Briggs)** | **Adopted (landed in 7b.4).** What YaoXiang actually needs is to map every `ValueId` to a non-conflicting `u8` stack slot according to its live interval (`OperandResolver` already has a `u8` contract and the E3014 limit). Implement `src/middle/lower/regalloc.rs`: block-level `successors` backward dataflow, closed intervals, Phi predecessor exits, and pinned fixed slots; `frame_slots = named_base + max_live`; an out-of-bounds result returns `Budget`, which `finalize_code_body` reports as E3014 with the declaration span |

**Landing measurements (7b.4)**: the two originally over-limit corpus cases (354 / 323 monotonic
slots) use **15 / 24 frame slots** after allocation (344 / 304 values), and the remaining corpus is
behaviorally equivalent in 403/403 cases. E3014 now means only “frame slots after allocation exceed
255.” The counter for the back-edge fallback is currently not exposed (`wrap_count` is calculated
and discarded); the `Phi` production batch must expose and retest it first (alongside
`defining_block` in the prerequisite list). **Overflow handling**: `u8`/255 remains an error handled
by the budget gate—this article does not solve “a function has too few registers” (a downstream
consequence of `compile_pattern` recursion depth).

The dev-dependencies at `Cargo.toml:37-39` contain only `criterion` / `proptest`; `[dependencies]`
contains no regalloc / petgraph / graphlib. Introducing any crate would add a new dependency
surface.

### Splitting Strategy for the 746-Line `generate_call_expr_ir`

**This must be done last**, and **the SSA shape and criteria must exist first**. The reason:
decomposition itself is C1 (a pure move, judged by zero-diff snapshots), but unless the semantics of
`arg_regs` have first been eliminated by SSA so that “each argument has an independent `ValueId`,”
decomposition would merely copy the old ordering assumptions into a new file.

**Core design constraint: ownership of `arg_regs` remains at the top of the function; the 6 branches
only `push` and never reorder.**

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

The “fill in 0” logic (`7838-7846`) at `7835-7848` is moved down into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, and **the “the normal
path should never reach this” self-description is replaced with an assertion**: if any slot is
missing and typechecking has not reported E1010, return a diagnostic directly. **This turns a silent
fallback into a hard error and is the most direct improvement in this article to defect class 2.**

**Criteria**: the pure-move portion (with no SSA-shape change) uses zero-diff snapshots; making
`fill_missing` assertive is C4 and uses a corpus differential plus the first-layer verifier.

### `synth.rs` Boundary: Turning Implicit Callbacks into an Explicit Synthetic-AST Boundary

**Verified facts**: there are **exactly 2** sites in `ir_gen.rs` that construct `ast::Expr`: `:4246`
and `:6701` (there are also `:6704` constructing `ast::Stmt` and `:510` / `:1266` / `:1399`
constructing `ast::Type`).

**Proposal**: create `src/middle/lower/synth.rs` as the **only module in the entire repository**
permitted to construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not
“tidiness,” but:

> Constructing an AST means **re-entering the frontend lowering entry point**. The
> `pending_env_vars` / `closure_captures` state expected by `generate_expr_ir` consists of
> **prerequisites for which the caller is responsible**, but no type or signature currently
> expresses those prerequisites—they hold only because of the runtime fact of “who called me.”

Change to an explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST that captures env; the capture set is explicitly provided by the arguments,
/// without relying on the caller pre-populating self.pending_env_*
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
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → construct Lambda → call back                                          | Call `synth::synth_closure(...)` to obtain the AST and explicitly pass `SynthEnv`; add an internal entry point carrying `SynthEnv` to `generate_lambda_expr_ir` (`6543`), which **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → construct → call back → clear                                                            | Same as above; the `clear()` calls at `6713-6714` disappear (there is no shared state to clear)                                                                                                                             |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                          | Read from the argument instead                                                                                                                                                                                              |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                                 | Same as above; `closure_captures` is demoted to a **local variable during closure-body generation**, and the `clear()` at `6600` disappears                                                                                 |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads a shared field on the namespace-resolution path | **This is the most dangerous site**: change it to an explicit argument; otherwise, any `?` early return from `generate_lambda_body_ir` will leak the capture table into unrelated code                                      |

**Criterion**: add the sentinel test `test_no_ast_construction_outside_synth`, which scans
`src/middle/**` and requires that no `ast::Expr::` / `ast::Stmt {` construction syntax appear
outside `synth.rs`. Its form follows `check-module-boundary.py` from RFC-039.

### Handling of Span-Keyed Problems (the Part SSA Cannot Solve)

**Design decision.** Three independent mechanisms, ordered by cost:

**Mechanism One: consumption-coverage assertion (mandatory and lowest-cost; replaced by a stronger
form on 2026-10-13).** 07 defines `test_release_plan_spans_consumed` (now settled by `PlanId`). This
article originally proposed supplementing it with a **runtime count** rather than merely a test; the
implemented form is stronger than that proposal—a **consumption ledger (`ConsumptionLedger<K>`) + a
single settlement point (E-level `W1090`)**—without introducing a new diagnostic code or relying on
an internal ir_gen counter for self-certification:

```rust
// ir_gen.rs:341 vicinity
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// On each hit at :1942, increment self.release_plan_hit += 1;
// At the end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are observable at runtime (destructor
side effects), but **“a value that should have been dropped was not dropped” may not immediately
crash under YaoXiang’s current value semantics and Arc/Rc model**—which is precisely why it was
silent. The count turns “silent” into “detectable.”

**Mechanism Two: change Span keys to `DefId` or explicit plan IDs (**landed on 2026-10-13**: RFC-039
D20 / WBS 4.5 implements this mechanism as `PlanId(u32)` structural-identity keys + mandatory
consumption settlement; see the revised note in §2 of [02](02-stage-contract.md)).** The fundamental
fix is **not to use Span as a key**. `Span` is a source position and may drift due to macro
expansion, `include`, and multi-file merging. `FunctionIR` already has a `def: Option<DefId>` field
(`ir.rs:677`), showing that DefId is available at the IR layer. **Design decision**: introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, assign it when ownership / overload
resolution produces the plan, and match by `PlanId` when the IR consumes it. **This belongs to the
obligations-ledger (`Obligations`) category in 02 and should be implemented together with
[02](02-stage-contract.md), not in this article.**

**Mechanism Three: `method_def_ordinals` dependence on definition order (independent small item).**
This field is a `HashMap<String, usize>` (`ir_gen.rs:224`), whose key is
`"{type_name}.{method_name}"` (**not module-qualified**; written at `:1691-1700`). If two modules
define the same `Type.method`, they share one counter and their mixed `#N` names drift. **Its
failure mode is not a span mismatch but an ordering/name mismatch**—if typechecking’s registration
order differs from ir_gen’s definition order, methods with the same name may resolve to different
targets. Remediation: have the typechecking side **produce `HashMap<DefId, String>` once** (bare
names or mixed `#N` names), and have ir_gen only look up names, never modify them. **This eliminates
one mutable cross-function state from `ir_gen`.**

### Tracking the Three Hard-Coded Discarded Fields

**Verified facts.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1,408
lines) contains three hard-coded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                                               |
| ------------------ | -------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count becomes zero after a `.42` round trip                                                                               |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **The `.42` artifact loses the entire exception table**                                                                                   |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **The `.42` artifact loses global-variable information** (note that `global_names` at `2347` **is** populated, creating an inconsistency) |

**Design decision (revised on 2026-10-05, decision D52): all three are brought into this article’s
scope as P7 batch e (7e), and no separate issue will be created.** The original rationale for
treating the latter two as separate issues, and why it was overturned, is recorded below:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields), unrelated to
   SSA.~~ This is true, but is not grounds for exclusion—P7 already modifies `bytecode.rs`, and
   fixing all three together requires only one `VERSION` bump (4→5), making two separate bumps more
   expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → version bump → scope expansion from
   “IR shape” to “artifact format.”~~ The version-bump mechanism already exists (`MAGIC` + `VERSION`
   header, with the reader rejecting mismatches), and `.42` is a build artifact; cross-version
   compatibility is not a goal—the expansion is in workload, not risk.
3. ~~The actual risk levels of `2315` / `2341` are low, and running `.42` directly is a secondary
   path.~~ **Overturned**: losing the exception table makes throw/try behavior incorrect when `.42`
   is run directly—throw/try is core language semantics; a secondary path does not mean silent
   errors are acceptable. The principle of this refactor is to **leave no unimplemented legacy in
   core functionality** (`Vec::new() // Not implemented yet` is precisely such a legacy).

**Retained requirement**: all three must still be registered in the `Obligations` ledger as
“produced but not serialized” until 7e is complete, to prevent them from becoming a fourth category
of “something nobody knows about.” `2312` will land together with the version bump (D17), and the
allocated but unused opcode values (D32/D34) will be reclaimed in the same batch.

## Detailed Design

### Cascading Effects of IR Structure Changes

#### `src/middle/ir/ir.rs`

| Change                                                                    | Location                                      | Nature                                                                                            |
| ------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Add `ValueId(u32)` type                                                   | File header (`ValueId` definition)            | New (**landed in 7b.1a**)                                                                         |
| Remove `Temp` / `Label` / `Register` from `Operand`; add `Value(ValueId)` | `Operand` enum definition                     | **Variant additions/removals** (**landed in 7b.1a**)                                              |
| Add `Phi` variant to `Instruction`                                        | End of the `Instruction` enum (`Phi` variant) | **+1 variant** (**landed in 7b.1a**; the enum now has 75 variants)                                |
| Add `values: Vec<ValueInfo>` to `FunctionBody::Code`                      | `FunctionBody::Code`                          | +1 field (**landed in 7b.1b**)                                                                    |
| Add `ValueInfo` struct                                                    | `ValueInfo`                                   | New (**landed in 7b.1b**)                                                                         |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`                   | Accessors of the same name                    | Add `values()` / `values_mut()` accessors (**landed in 7b.1b**, mirroring the existing accessors) |

The `pub use ...ast::Type` at the head of the file **is not changed by this article**—it belongs to
the scope of [03](03-type-unification.md). However, `ValueInfo::ty` must use the **single converged
type** after 03.

#### `src/middle/core/bytecode.rs` (2,422 lines)

| Change                                                                         | Location                              | Nature                                                                                                     |
| ------------------------------------------------------------------------------ | ------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Add `Phi` to `BytecodeInstr` or use the “expand into Move at runtime” strategy | `127-554` (currently **66** variants) | +1 or +0 variant (see below)                                                                               |
| Add a `Phi` arm to `opcode()`                                                  | `558`                                 | Mandatory (`BytecodeInstr` has an exhaustive match)                                                        |
| Add a `Phi` arm to `size()`                                                    | `649`                                 | Mandatory                                                                                                  |
| Add a `Phi` arm to the `From<BytecodeFile>` decoding match                     | `943-2350`                            | Mandatory                                                                                                  |
| Correct `upvalue_count: 0`                                                     | `2312`                                | Fixed incidentally by this article (in-memory format; on-disk format requires a `VERSION` bump, see below) |

**The bytecode form of `Phi` requires a design decision**. Two options (**design decision: A is
recommended**, for the reasons under “Key Decisions and Rationale”):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded by `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` instructions at the start of the block,
  selected by predecessor-block index. **No new opcode is added**, and the `.42` format is
  unchanged. The cost is increased bytecode size (one copy per predecessor).
- B: Add a `PHI` opcode plus predecessor-index encoding, and have the interpreter maintain register
  versions on jumps. **This requires changing the `executor/` state model, the `.42` format, and the
  version number.** The only benefit is size.

The decisive reason for choosing A: changing the `.42` format triggers version-number issues and a
full compatibility discussion, while **A’s size cost is irrelevant in interpreted execution** (the
executor runs Move instructions, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1,577 lines, 50 `translate_*` functions distributed over `101-1558`)

| Change                                          | Location                                                                                                                         | Nature                                                                                      |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Dispatch `Operand::Value(v)`                    | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                                     |
| `Instruction::Phi` translation arm              | In the dispatch match, before `Free` (533)                                                                                       | New branch that expands into a Move sequence (option A)                                     |
| Existing 9 NOP degradations **remain in place** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9 instructions**: `655-657` covers 2 variants and `658-660` covers 4 variants |

**SSA transformation does not reduce the NOP count.** These 9 degradations reflect expressiveness
gaps where “the IR has it, but bytecode does not,” and SSA transformation does not touch them.

#### `src/backends/interpreter/executor/`

With option A, `executor/` requires **no changes**—`Phi` disappears during the `translator.rs` stage
and is never visible to the interpreter. **This is the second decisive reason for choosing A.**

`executor/ops/control.rs:90` has an active `BytecodeInstr::Switch` implementation, but `ir.rs`
contains **no** `Switch` / `BrTable` / `JumpTable` variant (verified: all three names have zero hits
in `ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions the IR layer cannot
produce. It does not affect this article, but it is recorded here because it shows that “IR →
bytecode” is not surjective.

The dispatch match at `executor/debug.rs:194` likewise requires no changes (there is no new opcode
under option A).

### Compiler Change Checklist

**File by file and function by function.** All line numbers are change **starting points** (measured
and verified):

| No. | File                                      | Location                                                | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | ----------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/ir/ir.rs`                     | `Operand` enum definition                               | Add/remove `Operand` variants (**landed in 7b.1a**: remove `Temp` / `Label` / `Register`, add `Value(ValueId)`)                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 2   | `src/middle/ir/ir.rs`                     | End of the `Instruction` enum (`Phi` variant)           | Add `Instruction::Phi` (**landed in 7b.1a**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 3   | `src/middle/ir/ir.rs`                     | `FunctionBody::Code`                                    | Add `values` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 4   | `src/middle/ir/ir.rs`                     | `ValueInfo`                                             | Add `ValueInfo` (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 5   | `src/middle/ir/ir.rs`                     | `values()` / `values_mut()` accessors                   | Add accessors (**landed in 7b.1b**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 6   | `src/middle/ir/ir.rs`                     | `pub use ast::Type`                                     | **Leave `pub use ast::Type` unchanged** ([03](03-type-unification.md) scope; recorded only)                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                               | Return `Operand::Value`, increasing monotonically without rollback (**landed in 7b.2**: `next_temp_reg` in `src/middle/lower/scope.rs` produces `Operand::Value(ValueId)`; add an independent `next_value` counter starting at 0 for each function body, and ensure the three statement-level reclamations roll back only `next_temp`)                                                                                                                                                                                              |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                             | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                             | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                             | **Delete** statement-level reclamation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                               | Replace with an RAII guard (`generate_method_ir`, entry point `1677`)                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                               | Replace with an RAII guard (`generate_function_ir`, entry point `1830`)                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                         | Replace with an RAII guard (`generate_curry_innermost_func`, entry point `2105`)                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                               | Replace with an RAII guard (`generate_curry_function_ir`, entry point `2209`)                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                               | Replace with an RAII guard (`generate_anon_binding_ir`, entry point `2759`)                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                               | Replace with an RAII guard (`generate_lambda_body_ir`, entry point `4678`)                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                             | Extract the E3014 check into `check_register_budget()` for shared use after `1798`/`2001` are unified                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                                  | Change `take_cur_locals(param_types.len())` to `take_cur_locals(self.temp_high_water)` to eliminate inconsistent accounting                                                                                                                                                                                                                                                                                                                                                                                                         |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                               | Delete the contract comment (now enforced by the type system) and replace it with a pointer to `verify`                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                             | Change to call `synth::synth_closure`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                             | Change to call `synth::synth_closure`; delete `6713-6714`                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                             | Read `SynthEnv` from the argument instead of `mem::take`ing a shared field                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                                    | Demote `closure_captures` to a local variable and delete `clear()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                                   | Change `closure_captures.contains_key` to use an explicit argument (**highest-priority implicit dependency**)                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                          | Add span-keyed consumption counting                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                             | Make `method_def_ordinals` read-only                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                             | **Last**: split into a top-level `CallArgs` + 6 `emit_*` functions                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                             | Change the “fill in 0” fallback to return a diagnostic                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 29  | `src/middle/lower/synth.rs`               | **New**                                                 | Synthetic-AST boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 30  | `src/middle/ir/verify.rs`                 | **New** (first layer of [07](07-equivalence-oracle.md)) | `verify_loose` / `verify_ssa`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 31  | `src/middle/passes/codegen/translator.rs` | Dispatch match                                          | Translate `Instruction::Phi` (**landed in 7b.3**: option A expands at the **block entry**—`plan_phi_edges` / `emit_phi_prologue` emit `Move` groups by predecessor, existing `JMP`s skip between groups, and incoming-edge jumps are redirected to the group start using `(predecessor, merge-block first instruction)`; the `Phi` arm of `translate_instruction` is reduced to an unreachable E8001 defensive arm—a single return location cannot hold the multiple Moves distributed by predecessor, so reaching it fails loudly) |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                       | Add an `Operand::Value` resolution arm; remove the `Temp` arm (`39-45`) (**landed in 7b.2**: `to_reg(Value(v))` looks up `FunctionBody::Code.value_slots[v]` → `u8` (`>255` reports E3014; no mapping reports E3017 with the instruction span); removal of the `Temp` arm landed first in 7b.1a)                                                                                                                                                                                                                                    |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                               | **Unchanged under option A** (`Phi` does not enter bytecode)                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                              | **Unchanged under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                                  | Correct `upvalue_count: 0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 36  | `src/backends/interpreter/executor/**`    | —                                                       | **Zero changes under option A**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 37  | `src/middle/lower/regalloc.rs`            | **New**                                                 | Linear-scan allocator (**landed in 7b.4**: 271 lines; `allocate_value_slots(blocks, values, named_base, pinned, limit)` returns `value_slots` and `frame_slots = named_base + max_live`; an out-of-bounds result returns `Budget`, which `finalize_code_body` reports as E3014)                                                                                                                                                                                                                                                     |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New**                                                 | `synth.rs` boundary sentinel check                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

### Backward Compatibility: Versioning the `.42` Format

**Verified facts**: the `.42` on-disk format **already has a versioning
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` (`"YXBC"`)
and `VERSION: u32 = 4`, writes them in the header (`:342-343`), and validates them when reading
(`:454-467`, reporting "unsupported bytecode version" on a mismatch).

**Design decision (a direct consequence of option A)**:

- `Phi` is expanded into a Move sequence during the `translator.rs` stage and **produces no new
  opcode**.
- The 83 constants in `opcode.rs` have **zero changes**, `opcode_name()` (`120`) is unchanged, and
  the decoding match (`bytecode.rs:943-2350`) is unchanged.
- The `.42` format’s field layout has **zero changes**, and `VERSION` remains 4. Old `.42` files
  remain readable by the new binary, and new `.42` files remain readable by the old binary. **SSA
  transformation itself does not require a version bump.**

**The only scenario requiring a version bump is the correction of `2312 upvalue_count`** (decision
D17: fix it and handle it together with the version bump): add an `upvalue_count` field to
`BytecodeFunction` and have the encoder write it. This is **a new field in the format**, so bump
`VERSION` from 4 to 5. The reader’s existing version check rejects old files under its current
behavior—`.42` is a build artifact (`main.rs:671`), and cross-version read compatibility is not a
goal, so no additional migration mechanism is needed. The allocated but unused opcode values
(D32/D34 `Switch` / `TailCall`) will be reclaimed in the same batch.

## Implementation Notes

### Batch Strategy

Divide the work into four batches. **Every item within a batch gets an independent commit,
independent criterion, and independent revert.** In the table below, “change items” refer to numbers
in the preceding “Compiler Change Checklist.”

| Batch       | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Change items                 | Criterion                                                                                                                                                        | Rollback point                                                                                                                                                |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Eliminate multiple definitions: delete 3 statement-level reclamations → replace 6 save/restore sites with RAII guards → unify accounting                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 8-10 (first) → 11-16 → 17-18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariants). RAII and unified accounting are pure moves: **C1** zero-diff snapshots | One commit per item; `git revert` rolls it back                                                                                                               |
| **Batch b** | Switch to the SSA shape: `ir.rs` structural changes + temporary values produce `Operand::Value` (mechanical replacement) → `Phi` arm in `translator.rs` (option A) → linear-scan allocator. Within the batch, follow the three-level WBS sequence: 7b.1a (`Operand` variant convergence + introduction of the `Instruction::Phi` variant, **landed**) → 7b.1b (`FunctionIR` / `BasicBlock` / type-annotation fields, **landed**) → 7b.2 (`next_temp_reg` produces `Operand::Value`, **landed**) → 7b.3 (`Phi` arm in `translator.rs` + population of `BasicBlock.successors`, **landed**) → 7b.4 (linear-scan allocator + deletion of the three statement-level reclamations, in the same batch as 7a.1, **landed**) | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` fully green + `.42` round-trip tests                                                                               | The `ir.rs` change **must occupy a commit by itself** (the type change affects compilation throughout the repository and cannot be rolled back incrementally) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary + sentinel script → span consumption counting → make `method_def_ordinals` read-only                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + new script incorporated into CI; consumption count triggers zero new diagnostics (otherwise a real mismatch exists)             | Independent commit                                                                                                                                            |
| **Batch d** | Split `generate_call_expr_ir` + make the “fill in 0” fallback assertive                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 27-28                        | Pure-move portion: **C1** zero-diff snapshots. Assertion portion: **C4** corpus differential + first-layer verifier                                              | Independent commit                                                                                                                                            |

**Three dependencies whose order within the batches cannot be exchanged:**

1. Batch a’s “delete reclamation” comes after batch b’s `ir.rs` change (deletion of the three
   statement-level reclamations landed with 7b.4 and the linear-scan allocator, and 7a.1 was
   executed together)—first make `next_temp` increase monotonically, so that `ValueId` has the
   semantic basis for “unique definition” (that semantics only holds after `next_temp_reg` produces
   `Value` in 7b.2 and the `Phi` arm lands in 7b.3).
2. Batch b’s `Phi` arm must precede batch d’s decomposition—otherwise, decomposition would copy the
   old `arg_regs` ordering assumptions into the new file.
3. Batch d must be last—it depends on the preceding batches having converged the semantics of
   `arg_regs` to “one independent `ValueId` per argument.”

**Position relative to RFC-039**: all four batches fall within **P7** of RFC-039, and criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial**, but items
8-10, 11-16, and 17-18 within batch a have only ordering constraints and are not coupled to one
another.

### Hard Prerequisites

**Prerequisite One: criterion baseline.** The first-layer verifier’s `verify_loose` mode in
[07](07-equivalence-oracle.md) **must first pass on the existing (non-SSA) IR** before batch a may
begin.

Reason: the criterion for SSA transformation is C4 (behavioral equivalence + invariants). If the
invariant verifier itself cannot pass on the pre-change IR, every acceptance step in batches a–d
lacks an executable criterion and can rely only on corpus differentials—which are **precisely
insensitive** to defect classes 1/2/3 (as demonstrated by “Alternative A” in 07). **This is a
genuine ordering constraint.**

**Prerequisite Two: type-representation convergence.** [03](03-type-unification.md) must complete
the convergence of `pub use ast::Type` at `ir.rs:3` and remove the `From<MonoType> for IrType`
bridge at `bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` introduces a third set of
type-compatibility judgments.**

### Rollback Strategy

- Each change gets an independent commit, making the `git revert` granularity equal to the
  change-item granularity.
- The `ir.rs` structural change in batch b is **the only part that cannot be rolled back
  incrementally** (the type change affects compilation throughout the repository). It must therefore
  **occupy a commit by itself**, and batch a must already be green and have a revertible baseline
  before it.
- **Do not use a feature flag.** Reason: additions/removals of `Operand` variants cannot be isolated
  by a runtime switch; a feature flag would cause CI to stop testing the other path in the long
  term—a recurrence of the “test wiring depends on people remembering it, and corruption occurs
  silently” problem recorded in RFC-039.

### Expectation Management: Net Increase of 900–1,600 Lines

**This must be stated up front to prevent the question “why did the line count not decrease?” from
arising midway through implementation.**

| Item                                   | Line-count change                                                                                                                                                               |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                         | `ir_gen.rs` **8,448**                                                                                                                                                           |
| Delete 3 statement-level reclamations  | **−15**                                                                                                                                                                         |
| Replace 6 save/restore sites with RAII | Net ±0 (the RAII guard definition adds about 20 lines, while usage sites shrink from 22 lines to about 12)                                                                      |
| Change `next_temp_reg`                 | +5                                                                                                                                                                              |
| Split `generate_call_expr_ir`          | **+40–80** (signature and argument-passing overhead for 6 `emit_*` functions; this is the typical cost of a pure-move decomposition—**decomposition increases the line count**) |
| Span-keyed consumption counting        | +15                                                                                                                                                                             |
| Make `method_def_ordinals` read-only   | +20                                                                                                                                                                             |
| New `synth.rs`                         | +80–120                                                                                                                                                                         |
| New `verify.rs`                        | +400–600 (the first layer of 07 is within this article’s scope but counted separately)                                                                                          |
| New `regalloc.rs`                      | **+500–1,000** (271 lines measured in 7b.4)                                                                                                                                     |
| **Total**                              | **`ir_gen.rs` grows from about 8,448 to 8,600–8,800 lines; including new files, the net increase is 900–1,600 lines**                                                           |

**The early expectation of “8,448 → 5,000–6,000 lines” was wrong**; measured extrapolation produced
the opposite result. The reasons must be explained:

1. **The core benefit of this article is not fewer lines; it is eliminating an entire class of
   “wrong without reporting an error” defects.** The 6 manual save/restore sites (22 lines) become
   about 12 lines plus a guard type after SSA + RAII—**the line count is almost unchanged, but
   “forgot to restore” changes from something possible into a compile error**.
2. **Pure-move decomposition necessarily increases the line count.** Splitting the 746-line
   `generate_call_expr_ir` into 7 functions repeats signatures (the function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` bindings, and error propagation.
   **“Splitting the file reduces the line count” is a misconception**—it reduces cognitive load, not
   physical line count.
3. **The new verifier and allocator are net line increases.** These two additions (about 900–1,600
   lines) are precisely the **carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of line count, this article will most likely produce a **net increase**. The benefit lies
> in **verifiability**—turning “wrong without reporting an error” defects into invariants that fail
> when violated. If line count is used as an acceptance criterion, this article will fail; the
> correct acceptance criteria are those in “Batch Strategy.”

**This stage intentionally increases the amount of code in `ir_gen.rs`.** The decision of 2026-10-03
abolished all line-count and size gates (see prohibition two in [08](08-maintenance-mechanism.md));
therefore, **no line-count exemption list is needed**—the legitimacy of the increase is established
by the separation-of-responsibilities argument: SSA transformation separates “register allocation +
definition discipline” from lowering into an independent module, and the increased code volume buys
the elimination of the two classes of manual discipline in which “misalignment does not report an
error and only produces a wrong value”: 6 manual save/restore sites and the semantic reordering of
`arg_regs`.

## Key Decisions and Rationale

### Three Core Decisions

**Decision One: temporary-value strategy—delete the three statement-level rollbacks so that unique
definition becomes a mathematical consequence of construction.** The current rollbacks (`1954-1959`
/ `3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
increase monotonically, and every temporary slot naturally has exactly one definition point.
**Differences in criteria but not in guards**—for example, `3632` has a `result_reg.is_none()` guard
while the other two do not—disappear along with the hand-copied differences. The cost is increased
slot occupancy—the existing 255-slot upper-bound check (`temp_high_water` at `1992-2000`) provides a
backstop.

**Decision Two: do not perform mem2reg; perform explicit renaming only.** The reasons are given
under “Value Identification”: YaoXiang’s `&` / reference / ownership semantics make “which slots can
be promoted to registers” a type-system problem that requires the type representation converged by
[03](03-type-unification.md). **Performing mem2reg before 03 is guessing.** Explicit renaming
requires only changing the return-value semantics of `next_temp_reg` and deleting the three
rollbacks, without touching the control flow of any lowering algorithm.

**Decision Three: use option A for `Phi` (an IR-only pseudo-instruction expanded into a Move
sequence).** `executor/` and the `.42` format have **zero changes**, and the regression surface is
confined to L3. The decisive reason: changing the `.42` format triggers version-number issues and a
full compatibility discussion, while option A’s size cost is irrelevant in interpreted execution
(the executor runs Move instructions, not Phi). The cost is that `Phi` is not visible in
`dump_bytecode` (when entering the C1/C2 stage, note that the third layer in 07 must compare it).

### Rejected Alternatives

**A. Decompose everything at once (SSA + decomposition + `synth` boundary + allocator
simultaneously). Rejected.** This violates the core principle of 07: criteria must be tiered by
category. Mixing four batches into one PR mixes pure-move changes with C4 changes, so snapshot drift
cannot be attributed—when a snapshot changes, it is impossible to tell whether decomposition moved
something incorrectly or SSA changed it incorrectly. **Rollback granularity degrades from “change
item” to “everything.”**

**B. Split files first and optimize ordering later. Rejected.** Decomposition does not eliminate any
root cause: 6 manual save/restore sites remain scattered across 10 files, and the semantic
reordering of `arg_regs` still requires manual reasoning. Worse, **decomposing first loses the
opportunity to “build criteria before making changes”**—decomposition creates a large line-level
diff that mixes into the subsequent SSA diff and invalidates review. This has the same origin as the
8 orphan test trees (1,005 lines / 78 tests that never run) recorded in 06-cleanup-inventory.md:
**if the order is wrong, every subsequent step accelerates on an incorrect foundation.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as “a third path once considered.”** Its form is to leave `ir_gen.rs` unchanged and add a pass that
converts its non-SSA IR into SSA IR. **Its advantage** is that lowering does not change and rollback
is easy. **Reasons for rejection**: (1) it does not eliminate the root cause of defect class 1—the
three statement-level reclamations remain in `ir_gen.rs`, and the 6 save/restore sites remain
separated by 59–161 lines. A conversion pass can merge multiple definitions into `Phi`, but **it
cannot repair “an inner function’s name leaking into the outer function”**—that is a `cur_locals`
save/restore error occurring before the IR shape, and invisible to a conversion pass. (2) It makes
SSA a second representation—the repository already has the precedent of 3 parallel type
representations, and the core lesson of this project is that **“boundaries exist only in people’s
discipline.”** (3) `verify_loose` becomes a permanent burden.

**D. Keep the current implementation and only add a verifier. Rejected as the sole solution, but it
is a component of this article.** Alternative B of RFC-039 reaches the same conclusion: a gate can
prevent regression but cannot fix the current implementation. **What D can do must nevertheless be
made clear**: D can **catch instances** of defects 1/2/3 (provided the verifier covers dominance,
type consistency, and inner-scope isolation), but it cannot catch defect 4 (`closure_captures`
leakage)—that requires a cross-function liveness assertion beyond the boundary of
`verify(&ModuleIR)`. **D is therefore not an alternative to this article, but a prerequisite for
it.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1,600 lines** (new `verify.rs` / `regalloc.rs`). **This is not the risk
  point**—the decision of 2026-10-03 abolished line-count and size gates, and scale is addressed
  through separation of responsibilities (see prohibition two in [08](08-maintenance-mechanism.md)).
  The legitimacy of the increase lies in replacing the two classes of manual discipline in which
  “misalignment does not report an error and only produces a wrong value”: 6 manual save/restore
  sites and the semantic reordering of `arg_regs`.
- **The “unique definition” check in `verify.rs` is meaningless in the non-SSA shape**
  (`Operand::Local` allows multiple definitions)—07 records this limitation, and `verify_loose` is
  its degraded mode for this batch.
- **Deleting statement-level reclamation changes the temporary-slot allocation pattern** (executed
  in 7b.4, together with 7a.1): after reclamation is deleted, slots are reused by the linear-scan
  allocator according to live intervals, so **the allocation results change**—this is precisely why
  C4 must genuinely run the full corpus; the measured 403/403 differential was empty.
- **`u8` / 255 slot limit**: the two corpus cases that exceeded the limit after reclamation was
  deleted (`string_ops.yx` with 354 monotonic slots and `list_comp.yx` with 323 monotonic slots) are
  reduced by the allocator to **15 / 24 frame slots**; the budget gate now reports E3014 only when
  the post-allocation frame truly does not fit, with the function-body declaration span, rather than
  incorrectly rejecting based on the cumulative temporary count.
- **Benefits must be recorded as well**: turning “errors that cannot be tested” into “invariants
  that can be tested”; no backend changes (`executor/` and `.42` have zero changes); no changes to
  `Instruction` semantics (all 76 → 77 variants are retained, and all 50 `translate_*` functions are
  retained); incremental delivery (every item can be reverted independently).

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> item-by-item decisions. **This article leaves no pending items.**

## See Also

### Documentation

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching framework; four-layer model, criterion tiers, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; definitions of C4
  criteria, `verify_loose` / `verify_ssa`, three-layer criteria, and the limitation that snapshots
  are immune to `arg_regs` reordering
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; ownership of the
  `ReleasePlan` `PlanId` transformation
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; the counterexample of
  “create the directory first, then fail to wire it up”

### Code Locations

**IR definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` ([03](03-type-unification.md) scope)
- `src/middle/core/ir.rs:12-20` — 7 `Operand` variants
- `src/middle/core/ir.rs:47-533` — 76 `Instruction` variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for the `PlanId`
  proposal)
- `src/middle/core/ir.rs:689-719` — accessors such as `all_instructions`

**Defect class 1 (multiple definitions and save/restore)**

- `NestedBodyGuard` in `src/middle/lower/scope.rs` — save/restore for 7 nested-function-body entry
  points (`Deref`/`DerefMut` + `Drop`)
- `finalize_code_body` in `src/middle/lower/scope.rs` — sole table-extraction and E3014 budget point
  (`MAX_REGISTERS = 255`), combining block splitting / slot allocation / table extraction
- `take_cur_locals` in `src/middle/lower/scope.rs` — called only inside `finalize_code_body`
- `next_temp_reg` in `src/middle/lower/scope.rs` — maintains the named high-water mark
  (temporary-value slots belong to `regalloc.rs`)
- `src/middle/lower/regalloc.rs` — linear-scan allocator (sole producer of `value_slots` and
  `frame_slots`)
- `generate_function_ir` / `generate_block_ir` in `src/middle/lower/stmt.rs` and
  `generate_lambda_body_ir` in `src/middle/lower/expr/closure.rs` — statement-level reclamation
  removed in 7b.4 and replaced by allocator-based slot reuse
- `rebase_jump_targets` in `src/middle/lower/stmt.rs` — the “jump target exists” invariant from 07

**Defect class 2 (`arg_regs`)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reorderings and
  a clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent “fill in 0” fallback

**Defect class 3 (span keying)** — the following are **evidence pointers from the audit point**
(after the 4.5 key migration, `release_plan` / `overload_resolutions` have both become
`PlanId`-based; see the revised note in §2 of [02](02-stage-contract.md) for current pointers;
`method_def_ordinals` remains a pending item under mechanism three):

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — writing and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — initialization and sole write site of
  `method_def_ordinals` (key is the module-unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields in the transfer structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer transfer
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  production of `overload_resolutions`

**Defect class 4 (implicit ordering dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — AST synthesis and callback in
  `generate_spawn_for_ir`
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — equivalent handshake in `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — the two read sites of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — 3 other `ast::Type` construction sites

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — 66 `BytecodeInstr` variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1,408
  lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded discards (compare with
  `2344` / `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, `u8` limit 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file contains 83 constants
- `src/backends/interpreter/executor/ops/control.rs:90` — active `BytecodeInstr::Switch`
  implementation (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Current test state**

- `src/middle/core/ir_gen.rs` — 8,448 lines, 0 `#[cfg(test)]` blocks, 0 `mod tests` modules
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, with 29 tests
  in total; `def_assign` / `local_slots` positively pin DefId and slot naming, with no IR
  structural-invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop`
  (example of a per-opcode round-trip criterion; the last test in the file; the file contains 1,209
  lines / 23 `#[test]` functions)

**Engineering configuration**

- `Cargo.toml:37-39` — dev-dependencies contain only `criterion` / `proptest`; `[dependencies]`
  contains no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — the sidebar automatically scans the `/rfc/draft`
  directory; this article is not in that directory and is referenced by RFC-039 and the directory
  index
