---
title: Intermediate Representation SSA-ification
---

# Intermediate Representation SSA-ification

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance criteria classification, and execution phase ordering are in the body
> of RFC-039; the positioning of each subsidiary document is described in
> [this directory index](index.md).

## Positioning and Scope

This document addresses the IR construction discipline for the L3 layer, proposing a "single
definition per value" construction approach for `src/middle/core/ir_gen.rs` (8448 lines). **The goal
is not to introduce a new instruction set, but to eliminate three classes of failure-silent
discipline defects**:

1. 6 manual register save/restore pairs scattered tens of lines apart;
2. The `arg_regs` semantic rearrangement shared by 6 branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. The silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: IR form (`Operand` / `Instruction`) changes, temporary value
allocation discipline, cross-layer contract explicitation, call expression dispatch splitting, and
the wiring of the IR static validator.

This document is **not** responsible for: type representation unification
([03](03-type-unification.md)), stage contracts and the obligation ledger
([02](02-stage-contract.md)), definition of equivalence criteria and gating
([07](07-equivalence-oracle.md)), frontend paradigm ([05](05-frontend-paradigm.md)), dead code and
empty-design cleanup ([06](06-cleanup-inventory.md)). This document references the conclusions of
these documents in many places and does not repeat the arguments.

**The criterion category** is executed per the **C4 (IR form change)** category of
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, not full IR snapshot equality. SSA-ification inevitably changes IR form; forcing
snapshot equality would induce the team to relax criteria (07 records exactly this trap).

**The position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, with acceptance being behavioral equivalence + `verify_ssa` green). The
implementation batch numbering within this document (batches a through d) is distinct from the
internal ordering of P7: the former is this document's batching strategy, the latter is RFC-039's
global phases.

## Feasibility Prerequisites

**Verified facts.** The IR already possesses **all structural prerequisites** needed for SSA
construction:

| SSA prerequisite             | Current state in this repo                                                                            | Location              |
| ---------------------------- | ----------------------------------------------------------------------------------------------------- | --------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                      | `ir.rs:624-628`       |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                        | `ir.rs:636-642`       |
| Scope nesting information    | `LocalSlot::scope_depth` ("0 = function parameter layer, for disambiguating same-name nested scopes") | `ir.rs:658`           |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                    | `ir.rs:47-533`        |
| Span per instruction         | Every variant carries a `span: Span` field                                                            | `ir.rs:47-533`        |
| Annotatable value type       | `LocalSlot::ty: MonoType`                                                                             | `ir.rs:656`           |
| Slot count upper-bound check | E3014 check within `generate_function_ir` (`MAX_REGISTERS = 255`)                                     | `ir_gen.rs:1987-2000` |

**If these prerequisites were not in place, the cost estimates in this document would be entirely
different.**

**Two decisive points:**

**First, SSA-ification is not adding a capability, it is removing a capability dependency.** In the
current state, temporary slots are **actively designed for multiple definitions**—`1954-1959` /
`3632-3638` / `4737-4742` explicitly roll back `next_temp`. The first thing SSA-ification does is
**delete these three rollbacks**, letting `next_temp` monotonically increase. After deletion,
**every temporary slot naturally has only one definition point**—single definition transitions from
"a rule that needs to be checked" into "a mathematical consequence of the construction approach".

**Second, the slot count upper bound is already a hard check.** The `temp_high_water` at
`ir_gen.rs:889` is already tracking the true high-water mark, and `1992-2000` is already reporting
errors against the 255 upper bound. **Removing the rollbacks will not cause slot count to run out of
control**—`temp_high_water` records the historical peak occupancy, regardless of whether rollback
occurs (the comment at `887-888` is exactly about this).

**What these two points together imply**: `ir_gen.rs` is already maintaining all the bookkeeping
information required by SSA; it has just been **discarded**. The bulk of the SSA-ification work lies
in the splitting of `generate_call_expr_ir` and the explicitation of cross-layer contracts, not in
building IR expression capability.

### Prerequisite: Type Representation Unification

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` separately has `use crate::frontend::core::typecheck::MonoType;`; `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The serialization side has a third set, `ir::Type`, bridged by
`impl From<MonoType> for IrType` at `bytecode.rs:2353`.

That is: **two type representations coexist within the IR, and one of them directly `pub use`s the
AST type.**

SSA-ification needs to add a `Phi` variant, record a type for every SSA value (for the "consistent
types" invariant of the 07 validator), and determine whether the types of two predecessors at a CFG
merge point are compatible. **If the type representation has not yet been unified, SSA will write a
type-compatibility check for every representation**—and a third representation will grow right then
and there.

This is not a matter of "which is better to do first"; it is **a matter of "doing SSA first produces
three type-compatibility logics"**. [03 Type Representation Unification](03-type-unification.md)
must complete the convergence of `pub use ast::Type` at `ir.rs:3` and the removal of the bridge at
`bytecode.rs:2353` first. Item 6 in the change list of this document explicitly records that
**`ir.rs:3` is not modified by this document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not a "ugly but works" file. It is a **file that bets correctness on human
line-by-line review**:

- The file has **0 `#[cfg(test)]`, 0 `mod tests`** (verified by actual measurement).
- `src/middle/core/tests/mod.rs` declares three modules—`bytecode` / `def_assign` /
  `local_slots`—with 29 tests in total. Among them, `def_assign.rs` and `local_slots.rs`
  **positively** cover DefId allocation and local slot naming in ir_gen (sentinel level); the 4
  mentions of ir_gen in `bytecode.rs` (`421` / `423` / `1028` / `1173`) are **all reverse
  assertions**—"ir_gen does not produce", "ir_gen also does not construct", "ir_gen frontend does
  not construct the corresponding IR".

That is: ir_gen has 29 sentinel-level tests pinning down local behavior, but **no IR structural
invariant validation** (dominance, single definition, jump targets, type consistency)—none of the
three modules declared in `middle/core/tests/mod.rs` makes a positive assertion about the overall IR
form.

And this file happens to contain several classes of defects of the form **"compiles successfully,
all checks green, program runs, just produces the wrong value"**. Their common nature is: **failure
produces no error**.

This is the highest risk. Not "the function is too long", not "naming is chaotic", but **that the
core invariants of this file have no machine-enforced means of enforcement**.

`ir_gen.rs:248-249` writes this as an explicit contract:

> Save before nested function body generation, restore after—**must be physically adjacent to the
> `next_temp` save/restore points**, otherwise the names of the inner function will leak into the
> outer one.

This is a **rule expressed as a comment instead of an assertion**. It holds today only because the 6
save/restore pairs all happen to be written correctly.

**Why "just add a verifier" is not enough**: a verifier can **catch** these defects, but it cannot
catch the **causes of these defects**. The cause is not "the reviewer missed it", but:

> **The current IR form itself allows a slot to be defined multiple times, and `ir_gen`'s temporary
> register allocator actively depends on this.**

The concrete evidence is the three statement-level register reclamations (`1954-1959` / `3632-3638`
/ `4737-4742`)—they roll `next_temp` back at the end of every statement to the maximum of "last
named slot ⨿ `temp_floor`" (`1959`: `self.next_temp = named_watermark.max(self.temp_floor);`). This
means the same `Operand::Local(n)` slot is **written repeatedly within one function body**, and the
number of writes, the write order, and the live range are all determined by a single manual
decision: "the timing of the rollback".

In this form, "single definition" is not a rule that can be incrementally imposed; it is **in direct
conflict with the register allocation strategy**. Therefore, SSA-ification is not about adding a
check to existing code; it is about **replacing the allocation strategy that depends on multiple
definitions**. The good news is that the cost of replacing this strategy is low—see the section
"Feasibility Prerequisites" above.

### Defect Class 1: 6 manual save/restore pairs, silent on failure

**Verified fact.** 6 save/restore pairs, with the maximum distance between save and restore reaching
**161 lines**:

| Save point            | Restore point | Span      | Function                        | Entry line |
| --------------------- | ------------- | --------- | ------------------------------- | ---------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | 63 lines  | `generate_method_ir`            | `1677`     |
| `1858-1866`           | `2026-2029`   | 161 lines | `generate_function_ir`          | `1830`     |
| `2116`                | `2180`        | 64 lines  | `generate_curry_innermost_func` | `2105`     |
| `2224-2229`           | `2284-2288`   | 59 lines  | `generate_curry_function_ir`    | `2209`     |
| `2768-2777`           | `2820-2851`   | 74 lines  | `generate_anon_binding_ir`      | `2759`     |
| `4686-4695`           | `4767-4774`   | 72 lines  | `generate_lambda_body_ir`       | `4678`     |

> **Attribution clarification**: the correspondence between the two curry functions is:
> `2116`/`2180` belong to `generate_curry_innermost_func` (entry `2105`), and
> `2224-2229`/`2284-2288` belong to `generate_curry_function_ir` (entry `2209`)—i.e., the
> "innermost" goes to the former; do not invert it by line number intuition.

Fields involved: `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span` /
`loop_stack`, all driven by `next_temp_reg` (`ir_gen.rs:884-891`). This function maintains the true
high-water mark `temp_high_water` at `889` (per its own comment: statement-level reclamation rolls
back `next_temp`, and the total slot count is the historical peak occupancy). The same comment
convention applies to `cur_span` (self-described at `238-242` as "save before nested function body
generation, restore after—same handling as `next_temp`").

**Two inconsistencies in criterion** (this is a more insidious problem than "a missing write"):

| Location         | Criterion                                                                                                                |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `ir_gen.rs:1798` | `take_cur_locals(param_types.len())` — truncate by **parameter count**                                                   |
| `ir_gen.rs:2001` | `take_cur_locals(total_locals)`, where `total_locals = self.temp_high_water` (`:1992`) — truncate by **high-water mark** |

`generate_method_ir` uses the former, **and does not have the E3014 overflow check at `1992-2000`**.
The two functions give two different answers to "how many slots should the function body have", and
neither reports an error.

**Three duplicated statement-level register reclamations, same criterion but different guards**:

| Location    | Function                           | Guard                     |
| ----------- | ---------------------------------- | ------------------------- |
| `1954-1959` | `generate_function_ir`             | None                      |
| `3632-3638` | `generate_block_ir` (entry `3602`) | `if result_reg.is_none()` |
| `4737-4742` | `generate_lambda_body_ir`          | None                      |

The guard at `3632` has a clear reason (comment at `3626-3631`: for expression-operand-position
blocks, the outer layer may hold sibling argument temporaries that live across blocks). The other
two have no equivalent guard. **The same criterion is hand-copied in three places, with one guarded
and two unguarded**—this kind of difference is exactly what SSA is meant to eliminate.

**The restore point in `generate_anon_binding_ir` is separated by IR construction** (`2768-2777`
save → `2820-2824` restore 5 fields → `2829-2848` construct `func_ir` → `2851` finally restore
`loop_stack`). The two halves of the same state group are physically split by 22 lines of
construction code, and the order dependency is **inverted** (restore the slot table first, then
construct the IR, then restore the loop stack).

**Consequence**: missing one restore → `next_temp` leaks, temporary values skip numbers,
`cur_locals` names leak into the outer layer. **The IR remains self-consistent, still passes all
checks, still compiles—just produces the wrong value.**

### Defect Class 2: `arg_regs` semantic rearrangement in `generate_call_expr_ir`

**Verified fact.** `generate_call_expr_ir` is at `ir_gen.rs:7269-8014`, **746 lines**, a single
function. The function signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (with additional `_expr` / `result_reg` /
`instructions` / `constants`); its 6 internal branches share a single `Vec<Operand>` and perform
**semantic rewrites**:

| Line        | Semantics                                                                                                                                                   |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten by `slots` and then `collect` again — named argument rearrangement                                                                  |
| `7724`      | Struct construction: `unwrap` per slot from `final_args` and then `collect` again — field rearrangement                                                     |
| `7835-7848` | Function call: named argument rearrangement; `7838-7846` **supplies a new register with value 0 for missing slots** (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();` — clone before emitting instructions                                                                     |

The source form of the four rearrangements:

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

The self-description in the comment at `7835-7848` reads: "Uncovered slots: remain absent, with the
typecheck phase reporting E1010. For runtime safety, supply 0 (the normal path will not reach
here)."

**This design carries two layers of risk**:

1. **Misalignment reports no error, just wrong value.** Each of the 6 branches independently decides
   the final order of `arg_regs`; any order-assumption error in any place will not panic.
2. **Normalized snapshots are immune to this.** 07 records this limitation: the snapshot tool
   renames temporaries by "order of first appearance", so "the 3rd argument should have used the 5th
   register" looks completely identical in the snapshot. **For this class of defect, even the
   snapshot-level criterion does not exist in the C4 phase.**

**Supplementary verification: the struct-construction path's rearrangement has no duplicate-slot
guard.** `final_args` at `7724` is filled by "positional arguments first, then named arguments
overwriting by field name"—**no `slots[idx].is_some()` style duplicate-slot check is performed
before overwriting** (compare the namespace path which does have one at `7381`)—a named argument
silently overwrites a positional argument without panic, without diagnostic; and the **evaluation
order** of arguments (source order) diverges here from the final `arg_regs` order (field declaration
order). The assertion-ification of `fill_missing` in batch d must cover the "duplicate slot" check
as well, or the silent overwrite of the struct-construction path will survive unchanged.

### Defect Class 3: Span-keyed cross-layer contracts failing silently

**Verified fact.** There are exactly **two** span-keyed cross-layer contracts in the whole repo:

| Contract               | Declaration                                  | Production                                                                               | Transport                                                              | Consumption              |
| ---------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------ |
| `release_plan`         | `ir_gen.rs:191` `HashMap<Span, Vec<String>>` | `ownership.rs:31` (`ReleasePlan::drops` field), `2574` `build_release_plan`, `2786` call | `typecheck/types.rs:31` ← `checker.rs:1282` / `1440` ← `ir_gen.rs:341` | `ir_gen.rs:1942`         |
| `overload_resolutions` | `ir_gen.rs:222` `HashMap<Span, String>`      | `inference/expressions.rs:4366`, `inference/statements.rs:2758` / `2857`                 | `typecheck/types.rs:56` ← `checker.rs:1447-1448` ← `ir_gen.rs:364-366` | `ir_gen.rs:7420`, `7493` |

The form at `1942`:

```rust
// NLL Release: insert Drop instructions at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some` — **if not found, then not found; no `else` branch, no assertion, no count**. Any
inconsistency in how the two sides compute the span → **Drop instructions silently disappear, zero
errors**. The Drop sequence for refinement types is the core carrier of this project's ownership
semantics (self-described at `ownership.rs:26-32` as "NLL precise release plan", keyed by the span
of the last use), and its landing is silent.

`7420` / `7493` have the same form (`if let Some(mangled) = self.overload_resolutions.get(span)`);
the consequence is that overload resolution falls back to the default binding.

> **`method_def_ordinals` is not span-keyed.** It is a `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with the sole write point at `:1692-1702` (assigning `#N` suffixes in
> definition order of `base_name`). Its risk is of a **different kind**: definition-order
> dependence—the `#N` suffix is determined by the AST definition order; if the generation order and
> the typecheck registration order diverge, the same function name resolves to a different target.
> This is not a span mismatch, but it is likewise "wrong without error"; disposition is described in
> "Additional Mechanisms for Span Keying" mechanism three.

### Defect Class 4: spawn/lambda implicit handshake

**Verified fact.** `ir_gen.rs` contains **two** locations where it constructs `ast::Expr::Lambda`
itself and calls back into `generate_expr_ir`, relying on a set of mutable fields for the handshake:

**Handshake A — `generate_spawn_for_ir` (entry `4143`):**

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir` (entry `6543`) → consumes at `6571-6572`
`std::mem::take(&mut self.pending_env_vars)`.

**Handshake B — `generate_spawn_expr_ir` (entry `6641`):**

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**Handshake C — `closure_captures`**: written at `6589-6593`, **within the same function**, calls
`generate_lambda_body_ir` (entry `4678`) at `6598`, and clears at `6600`. There are two read points:
`:6284` (`LoadUpvalue` inside the closure body) and **`:458`**
(`closure_captures.contains_key(head)` on the `flatten_namespace` path).

The read point at `:458` is the most dangerous: it is on the **namespace resolution path**, meaning
that the live state of a `HashMap<String, usize>` can change the result of variable-name resolution.
Between the write at `6589` and the clear at `6600` lies a complete closure-body generation
(`6598`); if any path inside `generate_lambda_body_ir` returns early without clearing the table (it
has multiple return points when the `?` operator propagates errors), **`closure_captures` will leak
into unrelated subsequent code at `:458`**.

Other implicit state of the same kind includes `pending_env_names` (written by handshake B at
`6699`, cleared at `6714`).

### What SSA Will Not Solve (Honestly Listed)

| Issue                                             | Location                                                                                                                    | Why SSA does not solve it                                                                                                                                                                                               |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Complexity of `compile_pattern`                   | `ir_gen.rs:5390-5687`, **298 lines**, self-recursive 4 times (`5473` / `5504` / `5619` / `5640`), external call site `5270` | The dispatch complexity of pattern matching (union payload / tuple / struct / literal / nested) is **inherent to the algorithm**. SSA only turns guards' `JmpIfNot` into `Phi`; it does not reduce the number of guards |
| Complexity of `eval_const_expr`                   | `ir_gen.rs:2298-2488`, **191 lines** (`2297` carries `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion) | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                         |
| Span-keyed failure                                | `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)                                                           | **SSA does not touch this layer at all.** It requires an independent mechanism; see "Additional Mechanisms for Span Keying"                                                                                             |
| `Instruction` variant count                       | 76 (`ir.rs:47-533`)                                                                                                         | SSA adds `Phi`, **net change +1**. 76 variants correspond to 76 bytecode opcodes or are downgraded to NOP; they will not decrease due to SSA                                                                            |
| `method_def_ordinals` definition-order dependence | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                       | Same class as span keying but a different cause; see "Additional Mechanisms for Span Keying" mechanism three                                                                                                            |

## Target Design

### SSA Form Definition

**Design judgment.** The target form is as follows.

#### How `Operand` Variants Change

The current `ir.rs:12-20` has 7 variants. **A whole-repo construction-point survey**:

| Variant             | Construction sites                                                                                                                                                                           | Disposition                                                                                       |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 8 files                                                                                                                                                                                      | **Keep**, no change                                                                               |
| `Local(usize)`      | 7 files                                                                                                                                                                                      | **Keep but split semantics** (see below)                                                          |
| `Arg(usize)`        | 4 files                                                                                                                                                                                      | **Keep**, as the implicit definition of function entry (parameters are the definition of block 0) |
| `Global(usize)`     | 5 files                                                                                                                                                                                      | **Keep**, no change                                                                               |
| `Temp(usize)`       | **0 construction sites** (only handle arms at `codegen/operand.rs:39-45` and `:75`)                                                                                                          | **Delete** (dead variant)                                                                         |
| `Label(usize)`      | **0 construction sites**                                                                                                                                                                     | **Delete** (dead variant)                                                                         |
| `Register(u8)`      | **0 construction sites** (the comment at `ir.rs:19` says "Added for codegen", but codegen goes through the `OperandResolver` in `codegen/operand.rs`, which does not construct this variant) | **Delete** (dead variant)                                                                         |

`Local(usize)` is split into two variants, because it currently **carries two mutually exclusive
semantics simultaneously**—"storage location of a named variable" (writable multiple times) and
"compiler temporary register" (should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: dense numbering within a function, single definition
    Value(ValueId),
    /// Storage slot of a named local variable: addressable, multiple definitions allowed (non-SSA part)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a `u32` newtype (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing, never rolled back**.

#### `Phi` as a New `Instruction` Variant

```rust
/// Merge point: each predecessor basic block provides one definition of this value
/// incoming is sorted by predecessor block label in ascending order (guaranteed at construction time, for easy normalization)
Phi {
    dst: Operand,                                  // Must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (all enforced by `verify_ssa`; see the first layer of
[07](07-equivalence-oracle.md)):

1. `incoming.len() == number of predecessors of this block`.
2. The set of labels in `incoming` == the set of labels of all blocks in `blocks` whose `successors`
   include this block's label.
3. All `incoming` values share the same `MonoType` (**depends on 03's type representation being
   unified**).

**Disposing of the guard issue at `3632-3638`**: the current reclamation in `generate_block_ir`
carries the guard `if result_reg.is_none()` (comment at `3626-3631`: for expression-operand-position
blocks, the outer layer may hold sibling argument temporaries that live across blocks). After
SSA-ification, this guard is **no longer needed**—because temporaries are no longer rolled back, the
sibling argument temporaries living across blocks hold their own `ValueId` and are not affected by
the rollback. **This is where SSA-ification can genuinely simplify code, not merely rename things.**

#### Value Identification

**Design judgment**: do not mem2reg, do not use virtual register numbers, **only do explicit
renaming**. Reasons:

- YaoXiang's `&` / reference / ownership semantics make "which slots are mem2reg-able" a **type
  system problem**, not a CFG problem. It is verified that `ir.rs:656` `LocalSlot::ty: MonoType`
  carries the complete type information, but determining "whether this `MonoType` can be mem2reg-ed"
  requires the cooperation of the type representation after 03 convergence and the refinement type
  rules. **Doing mem2reg before 03 is guessing.**
- Explicit renaming only needs to change the return-value semantics of `next_temp_reg()`
  (`ir_gen.rs:884-891`) + delete the three rollbacks; **it does not touch any lowering algorithm's
  control flow**.

Value-table form: `FunctionBody::Code` gains a `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot), None for purely temporary values
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block where this value is defined
}
```

`values` and `locals` coexist: `locals` carry addressable named variables (which may be defined
multiple times), and `values` carry single-definition SSA values. **The existence of `locals` does
not violate SSA**—it is precisely the "storage" part outside SSA (LLVM's alloca model).

#### Change-Size Estimate

| Item                                               | Estimate               | Basis                                                                                                                                  |
| -------------------------------------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` refactor                           | ~15 lines              | `ir_gen.rs:884-891` single function                                                                                                    |
| Delete three statement-level reclamations          | −15 lines              | `1954-1959` / `3632-3638` / `4737-4742`                                                                                                |
| 6 save/restore pairs converged to RAII             | net ±0 lines           | See "Per-Item Disposition Table"                                                                                                       |
| `ir.rs` structural changes                         | +120 ~ 180 lines       | `ValueId` / `Phi` / `ValueInfo` / value table                                                                                          |
| `translator.rs` adds a `Phi` arm                   | +25 ~ 40 lines         | One of the existing 50 `translate_*`                                                                                                   |
| `executor/` adds `PHI` instruction or runtime fill | +60 ~ 120 lines        | Depends on the `Phi` bytecode form choice                                                                                              |
| Renaming `Operand::Local` → `Value` at call sites  | Mechanical replacement | **Verified by actual measurement**: `Operand::Local(` appears **323** times in `ir_gen.rs`; 355 times across 6 files in the whole repo |

### Per-Item Disposition Table

| Defect class                                       | Eliminated by SSA?                | Disposition                                                                                                                                                                                                                                                                                                                                                                             | Criterion                                                                      |
| -------------------------------------------------- | --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| **1. 6 manual save/restore pairs**                 | **Yes (at the root-cause level)** | `next_temp` / `temp_high_water` / `temp_floor` become `&mut` monotonic counters, no longer need to be saved across function bodies. `cur_locals` / `loop_stack` / `cur_span` are switched to RAII guards (`struct NestedBodyGuard<'a>`, restored in `Drop`), **turning "forgetting to restore" into a compile error**. The comment contract at `248-249` is enforced by the type system | 07 first-layer "inner-layer isolation" invariant + `verify_loose`/`verify_ssa` |
| **1b. Inconsistent criterion (`1798` vs `2001`)**  | **Yes**                           | Unified to `take_cur_locals(self.temp_high_water)`, and the E3014 check (`1992-2000`) is extracted into the shared function `check_register_budget()`, called from both places                                                                                                                                                                                                          | Corpus diff + snapshot (zero-diff in pure move phase)                          |
| **1c. Three statement-level reclamation criteria** | **Yes**                           | All three deleted (`1954-1959` / `3632-3638` / `4737-4742`), including the `result_reg.is_none()` guard at `3632`                                                                                                                                                                                                                                                                       | C4: behavioral equivalence + invariants                                        |
| **2. `arg_regs` semantic rearrangement**           | **Partial**                       | SSA turns "using the wrong register" into "using the wrong value ID" and is caught by dominance checks, but **the rearrangement logic itself remains**. Real elimination comes from function splitting—each of the 6 branches `push`es, and ownership of `arg_regs` stays at the top of the function                                                                                    | First-layer dominance invariant (snapshots are immune)                         |
| **3. Span-keyed failure**                          | **No**                            | SSA does not touch this at all. See "Additional Mechanisms for Span Keying"                                                                                                                                                                                                                                                                                                             | 07's `test_release_plan_spans_consumed`                                        |
| **4. Implicit order dependencies**                 | **No**                            | See "`synth.rs` boundary"                                                                                                                                                                                                                                                                                                                                                               | Requires new explicit criteria                                                 |
| `compile_pattern` 298 lines                        | No                                | See "What SSA Will Not Solve"                                                                                                                                                                                                                                                                                                                                                           | —                                                                              |
| `eval_const_expr` 191 lines                        | No                                | See "What SSA Will Not Solve"                                                                                                                                                                                                                                                                                                                                                           | —                                                                              |
| `Instruction` 76 variants                          | No (+1)                           | New `Phi` → 77                                                                                                                                                                                                                                                                                                                                                                          | —                                                                              |

### Register Allocator: Custom or Existing Crate

**Verified facts (decisive)**:

1. `Operand::Register(u8)` (`ir.rs:19`) has **0 construction sites across the whole repo**.
2. The backend is not a register machine—the `OperandResolver` in
   `src/middle/passes/codegen/operand.rs` resolves `Operand::Local/Temp/Arg` to `u8`, with an upper
   bound of 255 (the `Temp` arm at `39-45` overflows to E3014).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1577 lines, 50
   `translate_*` functions, bodies distributed across `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`, with opcodes from `src/backends/common/opcode.rs`
   (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter's opcode dispatch family.

**Conclusion**: the so-called "register" is an **operand stack slot index, not a physical
register**. The target machine has **no finite register file, no register pressure, no spill/split
requirement**.

**Three candidate approaches evaluated**:

| Approach                                           | Evaluation                                                                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (rustc lineage)           | **Not adopted.** Both take rustc's `Function` trait + MIR's `Place`/`Operand` abstraction as input. Writing an adapter for YaoXiang IR requires implementing the full trait (CFG, liveness, spill slot, scratch)—**500+ lines of adapter code for zero benefit**—because there is no real register file to color. Moreover, rustc itself only does linear scan here (when `enable-llvm` is off)                |
| `graph-alloc` family                               | **Not adopted.** Same reason, and the API is even less stable                                                                                                                                                                                                                                                                                                                                                  |
| **Custom linear scan (Chaitin-Briggs simplified)** | **Adopted.** What YaoXiang actually needs is: in `values` order, map each `ValueId` to a non-conflicting `u8` stack slot (the `OperandResolver` already has a `u8` contract and the E3014 upper bound). This is essentially **maximum-clique coloring on a non-overlapping-liveness graph**, which, under no-register-file constraints, degenerates to "allocate the first available slot in definition order" |

**Design judgment**: custom-built, target 500-1000 lines (including the `values → Reg` mapping,
liveness computation, and the interface with `OperandResolver`). **At the same time, we must
admit**: if the `u8`/255 slot upper bound proves insufficient under real load, the custom allocator
must handle overflow, while the existing code already treats it as an error at `1992-2000`. **This
means this document does not solve the "register shortage in large functions" problem**—**that is a
downstream consequence of `compile_pattern`'s recursion depth and belongs to a separate topic.**

`Cargo.toml:37-39`'s dev-dependencies include only `criterion` / `proptest`; `[dependencies]` has no
regalloc / petgraph / graphlib—introducing any crate is a new dependency surface.

### Splitting Strategy for `generate_call_expr_ir` (746 lines)

**Must be done last**, and **the SSA form and criteria must be in place first**. Reason: splitting
itself is C1 (pure move, criterion is snapshot zero-diff), but only after the semantics of
`arg_regs` have been reduced by SSA to "one independent `ValueId` per argument" will the splitting
not carry the old ordering assumptions into the new file as-is.

**Core design constraint**: ownership of `arg_regs` stays at the top of the function; the 6 branches
only `push`, never rearrange.

```
generate_call_expr_ir (746 lines)
├── Argument evaluation: evaluate in source order, push into arg_regs — shared by all 6 branches, the only place where append is allowed
├── CallArgs (value object)
│   └── arg_regs: Vec<Operand>  — created only at the top; the 6 branches only push, never collect/reorder again
├── Branch 1 → emit_namespace_call(args: CallArgs)         [extract from 7395]
├── Branch 2 → emit_bound_call(args: CallArgs)              [extract from 7406+]
├── Branch 3 → emit_struct_ctor(args: CallArgs)             [extract from 7724]
├── Branch 4 → emit_plain_call(args: CallArgs)              [extract from 7835-7848]
├── Branch 5 → emit_curried_call(args: CallArgs)
└── Branch 6 → emit_method_call(args: CallArgs)             [extract from 7420 / 7493]
```

The "supply 0" logic at `7835-7848` (`7838-7846`) is sunk to
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, **and the
self-described comment "the normal path will not reach here" is replaced with an assertion**: if any
slot is missing and typecheck has not reported E1010, return a diagnostic directly. **This converts
a silent fallback into a hard error, and is the most direct improvement this document makes to
defect class 2.**

**Criterion**: pure-move parts (SSA form unchanged) use snapshot zero-diff; the assertion-ification
of `fill_missing` falls under C4, using corpus diff + the first-layer validator.

### `synth.rs` Boundary: Turning Implicit Callbacks into an Explicit Synthetic AST Boundary

**Verified fact**: the locations in `ir_gen.rs` that construct `ast::Expr` are **exactly 2**—`:4246`
and `:6701` (and also `:6704` constructs `ast::Stmt`; `:510` / `:1266` / `:1399` construct
`ast::Type`).

**Proposal**: create `src/middle/lower/synth.rs`, as the **sole** module in the whole repo allowed
to construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not "tidiness"
but:

> Constructing AST means **re-entering the frontend lowering entry**. And the `pending_env_vars` /
> `closure_captures` state that `generate_expr_ir` expects is a **precondition that the caller is
> responsible for preparing**, and that precondition is not currently expressed by any type or
> signature—it holds by virtue of the runtime fact of "who calls me".

Switch to an explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST that captures env; the capture set is given explicitly by parameters,
/// not depending on the caller's prior write to self.pending_env_*
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Specific changes:

| Location                       | Current                                                                                          | New                                                                                                                                                                                                         |
| ------------------------------ | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → build Lambda → callback                                               | Call `synth::synth_closure(...)` to obtain AST, explicitly pass `SynthEnv`; `generate_lambda_expr_ir` (`6543`) gains an internal entry that carries `SynthEnv`, **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → build → callback → clear                                                                 | Same as above; the `clear()` at `6713-6714` disappears (no shared state to clear)                                                                                                                           |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                          | Read from parameter instead                                                                                                                                                                                 |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                                 | Same as above; `closure_captures` is demoted to a **local variable during closure body generation**; the `clear()` at `6600` disappears                                                                     |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads a shared field on the namespace-resolution path | **This is the most dangerous one**: change to an explicit parameter; otherwise any `?`-driven early return in `generate_lambda_body_ir` would leak the capture table into unrelated code                    |

**Criterion**: add a sentinel test `test_no_ast_construction_outside_synth`, scanning
`src/middle/**` outside of `synth.rs` for the absence of `ast::Expr::` / `ast::Stmt {` construction
syntax. Same form as RFC-039's `check-module-boundary.py`.

### Disposition of Span-Keyed Issues (the part SSA cannot solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism One: consumption-coverage assertion (required, lowest cost).** 07 already defines
`test_release_plan_spans_consumed` (the Span set produced by ownership ⊆ the Span set consumed by
IR). This document's addition is to make it a **runtime counter** rather than just a test:

```rust
// near ir_gen.rs:341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// Each time :1942 hits, self.release_plan_hit += 1;
// At the tail of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions on non-`ref` locals are runtime-observable (destructor
side effects), but **"a needed drop was not performed" does not necessarily crash immediately under
YaoXiang's current value semantics + Arc/Rc model**—this is exactly why it is silent. Counting turns
"silent" into "detectable".

**Mechanism Two: Span key changed to `DefId` or explicit plan ID (medium term).** The fundamental
fix is **not using Span as the key**. `Span` is a source location and will drift due to macro
expansion, `include`, and multi-file merging. `FunctionIR` already has the `def: Option<DefId>`
field (`ir.rs:677`), showing that DefId is usable at the IR layer. **Design judgment**: introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, allocated by ownership / overload
resolution at production time, matched by IR at consumption time by `PlanId`. **This belongs in 02's
obligation ledger (`Obligations`) and should be implemented together with
[02](02-stage-contract.md), not placed in this document.**

**Mechanism Three: `method_def_ordinals`' definition-order dependence (independent small item).**
This field is a `HashMap<String, usize>` (`ir_gen.rs:224`), keyed by `"{type_name}.{method_name}"`
(**without module qualification**, written at `:1691-1700`)—if two modules define a same-named
`Type.method`, they share the same counter, and the `#N` mangled names drift along with it. **Its
failure mode is not a span mismatch but an order/duplicate-name mismatch**—if typecheck's
registration order and ir_gen's definition order diverge, the same-named method resolves to a
different target. Disposition: have typecheck **produce once** a `HashMap<DefId, String>` (bare name
or `#N` mangled name), and ir_gen only reads, never writes. **This eliminates one mutable
cross-function state inside `ir_gen`.**

### Tracking the Three Hardcoded Drops

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1408 lines)
contains three hardcoded drops:

| Location           | Field                | Value                                          | Consequence                                                                                                                          |
| ------------------ | -------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count resets to zero after `.42` round-trip                                                                          |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **`.42` artifact loses the entire exception table**                                                                                  |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` artifact loses global variable information** (note that `global_names` at `2347` **is** populated; the two are inconsistent) |

**Design judgment: the latter two (`2315` / `2341`) should be filed as independent issues, not
placed in this document's scope.** Reasons:

1. They are **format-layer gaps** (`BytecodeFile` does not serialize these fields), unrelated to
   SSA.
2. Fixing them requires changing the `.42` disk format → requires bumping the `VERSION: u32` at
   `codegen/bytecode.rs:14-16` (currently 4) → triggers a reader-side compatibility discussion →
   scope balloons from "IR form" to "artifact format".
3. The **actual risk level** of `2315` / `2341` is lower than the three classes of defects this
   document handles: loss of the exception table causes throw/try to behave incorrectly on direct
   `.42` runs, but direct `.42` runs are a secondary path (the comment at `2344` shows that the main
   value of `.42` is debug rendering, `#327`).

**But they must be registered in 07's `Obligations` ledger as "produced but not serialized"**, lest
they become a fourth "thing nobody knows about". `2312` (`upvalue_count`) is recommended to be fixed
in passing within this document's change list—it lives on the `From<BytecodeFile>` of the in-memory
format; to make it into the `.42` disk artifact, you also need to add a serialization field to
`BytecodeFunction` and bump `VERSION` to 5 (see decision D17).

## Detailed Design

### Chain Effects of the IR Structure Change

#### `src/middle/core/ir.rs` (905 lines)

| Change                                                               | Location                    | Nature                                            |
| -------------------------------------------------------------------- | --------------------------- | ------------------------------------------------- |
| New `ValueId(u32)` type                                              | Near file head, around `10` | New                                               |
| `Operand` delete `Temp` / `Label` / `Register`, add `Value(ValueId)` | `12-20`                     | **Variant add/remove**                            |
| `Instruction` adds `Phi` variant                                     | End of `47-533`             | **Variants +1 → 77**                              |
| `FunctionBody::Code` adds `values: Vec<ValueInfo>`                   | `636-642`                   | Field +1                                          |
| New `ValueInfo` struct                                               | Around `660`                | New                                               |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`              | `689-719`                   | Need to add `values()` / `values_mut()` accessors |

The `pub use ...ast::Type` at `ir.rs:3` is **not modified by this document**—it belongs to
[03](03-type-unification.md). But `ValueInfo::ty` must use the **unified** single type.

#### `src/middle/core/bytecode.rs` (2422 lines)

| Change                                                          | Location                              | Nature                                                                                           |
| --------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `BytecodeInstr` adds `Phi` or "runtime expand to Move" strategy | `127-554` (currently **66** variants) | Variant +1 or +0 (see below)                                                                     |
| `opcode()` adds a `Phi` arm                                     | `558`                                 | Required (`BytecodeInstr` exhaustive match)                                                      |
| `size()` adds a `Phi` arm                                       | `649`                                 | Required                                                                                         |
| `From<BytecodeFile>` decode match adds a `Phi` arm              | `943-2350`                            | Required                                                                                         |
| `upvalue_count: 0` fix                                          | `2312`                                | Fixed in passing in this document (in-memory format; disk requires bumping `VERSION`, see below) |

**The bytecode form of `Phi` requires a design decision**. Two options (**design judgment, A is
recommended**, with reasons in "Key Decisions and Rationale"):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` at the head of the block, selected by the
  predecessor block index. **No new opcode**, `.42` format zero change. Cost: increased bytecode
  size (one copy per predecessor).
- B: New `PHI` opcode + predecessor-index encoding; the interpreter maintains register versions at
  jump time. **Requires changing the `executor/` state model + `.42` format + version number**. The
  only benefit is size.

The decisive reason for choosing A: changing the `.42` format triggers a version-number issue and a
full-compatibility discussion, **whereas A's size cost is irrelevant under interpreted execution**
(what executes is Move, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1577 lines, 50 `translate_*` functions, distributed across `101-1558`)

| Change                                             | Location                                                                                                                         | Nature                                                                      |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `Operand::Value(v)` dispatch                       | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                     |
| `Instruction::Phi` translate arm                   | Inside dispatch match, before `Free` (`533`)                                                                                     | New branch, expanded into a Move sequence (option A)                        |
| Existing 9 NOP downgrades' **positions unchanged** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9**: `655-657` covers 2 variants, `658-660` covers 4 variants |

**SSA-ification does not reduce the NOP count.** These 9 downgrades reflect the "IR has it, bytecode
doesn't" expressiveness gap; SSA-ification does not touch them.

#### `src/backends/interpreter/executor/`

If option A is adopted, `executor/` **requires no changes whatsoever**—`Phi` disappears into
`translator.rs` from the interpreter's perspective. **This is the second decisive reason for
choosing A.**

`BytecodeInstr::Switch` at `executor/ops/control.rs:90` has a live implementation, but **no**
`Switch` / `BrTable` / `JumpTable` variants exist in `ir.rs` (verified: zero hits for these three
names in `ir.rs`)—**this is a reverse gap**: the bytecode layer has an instruction that the IR layer
cannot produce. It does not affect this document, but is recorded here because it shows that "IR →
bytecode" is not surjective.

The dispatch match in `executor/debug.rs:194` likewise needs no change (under option A, no new
opcode).

### Compiler Change List

**Per file, per function.** All line numbers are the change **starting point** (verified by actual
measurement):

| No. | File                                      | Location                                             | Change                                                                                                            |
| --- | ----------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/core/ir.rs`                   | `12-20`                                              | `Operand` variant add/remove                                                                                      |
| 2   | `src/middle/core/ir.rs`                   | end of `47-533`                                      | New `Instruction::Phi`                                                                                            |
| 3   | `src/middle/core/ir.rs`                   | `636-642`                                            | `FunctionBody::Code` adds `values`                                                                                |
| 4   | `src/middle/core/ir.rs`                   | around `660`                                         | New `ValueInfo`                                                                                                   |
| 5   | `src/middle/core/ir.rs`                   | `689-719`                                            | Add `values()` / `values_mut()` accessors                                                                         |
| 6   | `src/middle/core/ir.rs`                   | `3`                                                  | **`pub use ast::Type` not touched** ([03](03-type-unification.md) scope, recorded only)                           |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                            | Return `Operand::Value`, monotonically increasing, no rollback                                                    |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                          | **Delete** statement-level reclamation                                                                            |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                          | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                               |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                          | **Delete** statement-level reclamation                                                                            |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                            | Switch to RAII guard (`generate_method_ir`, entry `1677`)                                                         |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                            | Switch to RAII guard (`generate_function_ir`, entry `1830`)                                                       |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                      | Switch to RAII guard (`generate_curry_innermost_func`, entry `2105`)                                              |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                            | Switch to RAII guard (`generate_curry_function_ir`, entry `2209`)                                                 |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                            | Switch to RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                   |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                            | Switch to RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                    |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                          | E3014 check extracted into `check_register_budget()`, shared by both `1798`/`2001` after criterion unification    |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                               | `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`, eliminate criterion inconsistency |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                            | Delete the comment contract (enforced by the type system), replace with a pointer to `verify`                     |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                          | Switch to call `synth::synth_closure`                                                                             |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                          | Switch to call `synth::synth_closure`, delete `6713-6714`                                                         |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                          | Read `SynthEnv` from parameter, no longer `mem::take` shared field                                                |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                                 | Demote `closure_captures` to local, delete `clear()`                                                              |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                                | `closure_captures.contains_key` switched to an explicit parameter (**highest-priority implicit dependency**)      |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                       | Add span-keyed consumption counter                                                                                |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                          | `method_def_ordinals` changed to read-only                                                                        |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                          | **Last to do**: split into top-level `CallArgs` + 6 `emit_*`                                                      |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                          | "Supply 0" fallback changed to return a diagnostic                                                                |
| 29  | `src/middle/lower/synth.rs`               | **New**                                              | Synthetic AST boundary                                                                                            |
| 30  | `src/middle/ir/verify.rs`                 | **New** ([07](07-equivalence-oracle.md) first layer) | `verify_loose` / `verify_ssa`                                                                                     |
| 31  | `src/middle/passes/codegen/translator.rs` | dispatch match                                       | Add `Instruction::Phi` arm (option A: expanded into a Move sequence)                                              |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                    | `Operand::Value` resolve arm; delete the `Temp` arm (`39-45`)                                                     |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                            | **Unchanged under option A** (`Phi` does not enter bytecode)                                                      |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                           | **Unchanged under option A**                                                                                      |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                               | `upvalue_count: 0` fix                                                                                            |
| 36  | `src/backends/interpreter/executor/**`    | —                                                    | **Zero changes under option A**                                                                                   |
| 37  | `src/middle/passes/regalloc.rs`           | **New**                                              | Linear scan allocator                                                                                             |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New**                                              | `synth.rs` boundary sentinel check                                                                                |

### Backward Compatibility: `.42` Format Version Number

**Verified fact**: the `.42` disk format **already has a version number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` ("YXBC") and
`VERSION: u32 = 4`, written to the header (`:342-343`), checked on read (`:454-467`, reporting
"unsupported bytecode version" on mismatch).

**Design judgment (direct corollary of option A)**:

- `Phi` is expanded in `translator.rs` into a `Move` sequence, **producing no new opcode**.
- `opcode.rs`'s 83 constants have **zero changes**; `opcode_name()` (`120`) is unchanged; the decode
  match (`bytecode.rs:943-2350`) is unchanged.
- The field layout of the `.42` format has **zero changes**; `VERSION` stays at 4. Old `.42` files
  can still be read by the new binary, and new `.42` files can still be read by the old binary.
  **SSA-ification itself does not require a version bump.**

**The only scenario that requires a version bump is the fix at `2312 upvalue_count`** (decision D17:
fix it, and handle the version bump together): add an `upvalue_count` field to `BytecodeFunction`
and have the encoder write it, which is a **new field in the format**, bumping `VERSION` from 4
to 5. The reader side's existing version check rejects old files per the established behavior—`.42`
is a build artifact (`main.rs:671`); cross-version read compatibility is not a goal, so the version
bump does not require an extra migration mechanism. Already-allocated but unused opcode values
(D32/D34's `Switch` / `TailCall`) are reclaimed along with the version bump.

## Implementation Notes

### Batching Strategy

Four batches. **Each item within a batch is an independent commit, an independent criterion, and an
independently revertible unit.** The "Change items" in the table below refer to the numbers in the
"Compiler Change List" above.

| Batch       | Content                                                                                                                                              | Change items                 | Criterion                                                                                                                                                         | Rollback point                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Cut off multiple definitions: delete the 3 statement-level reclamations → 6 save/restore pairs switched to RAII guards → criterion unification       | 8-10 (first) → 11-16 → 17-18 | Deleting reclamation: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and criterion unification are pure moves: **C1** snapshot zero-diff | Per-item single commit, `git revert` rolls back                                                                                               |
| **Batch b** | SSA form switch: `ir.rs` structural change + `Operand` mechanical replacement (323 sites) → `translator.rs` adds a `Phi` arm → linear scan allocator | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` all green + `.42` round-trip tests                                                                                  | The `ir.rs` change **occupies its own commit** (the type change affects the whole repo's compilation and cannot be incrementally rolled back) |
| **Batch c** | Explicate implicit contracts: `synth.rs` boundary + sentinel script → span consumption counter → `method_def_ordinals` read-only                     | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + new script included in CI; consumption counter zero triggers on new diagnostics (otherwise a real mismatch exists)               | Independent commit                                                                                                                            |
| **Batch d** | `generate_call_expr_ir` split + "supply 0" fallback assertion-ification                                                                              | 27-28                        | Pure-move parts: **C1** snapshot zero-diff. Assertion-ification parts: **C4** corpus diff + first-layer validator                                                 | Independent commit                                                                                                                            |

**Three dependencies where in-batch order is not exchangeable:**

1. Batch a's "delete reclamation" must precede batch b's `ir.rs` change—first let `next_temp`
   monotonically increase, so that `ValueId` has the semantic basis of "single definition".
2. Batch b's `Phi` arm must precede batch d's split—otherwise the split will carry the old
   `arg_regs` ordering assumptions into the new file as-is.
3. Batch d must be last—it depends on the first two batches having already reduced `arg_regs`
   semantics to "one independent `ValueId` per argument".

**Position relative to RFC-039**: all four batches above fall within RFC-039's **P7**, with the
criteria classification (`verify_loose` / `verify_ssa`) established by P2. Batches are **serial** to
each other, but within batch a the 8-10 / 11-16 / 17-18 segments have only ordering constraints and
no coupling.

### Prerequisites (Hard)

**Prerequisite One: criterion baseline.** The `verify_loose` mode of
[07](07-equivalence-oracle.md)'s first-layer validator **must first run green on the existing
(non-SSA) IR** before batch a is allowed to begin.

Reason: the criterion for SSA-ification is C4 (behavioral equivalence + invariants). If the
invariant validator itself cannot run green on the existing IR before the refactor, then every
acceptance in batches a through d lacks an executable criterion and can only rely on corpus diff—and
corpus diff is exactly insensitive to defect classes 1/2/3 (07's "Alternative A" has already argued
this). **This is a real ordering constraint.**

**Prerequisite Two: type representation unification.** [03](03-type-unification.md) must complete
the convergence of `pub use ast::Type` at `ir.rs:3` and the removal of the
`From<MonoType> for IrType` bridge at `bytecode.rs:2353`. **Otherwise `ValueInfo::ty` will introduce
a third type-compatibility check.**

### Rollback Strategy

- Each change is an independent commit; `git revert` granularity = change-item granularity.
- Batch b's `ir.rs` structural change is the **only part that cannot be incrementally rolled back**
  (the type change affects the whole repo's compilation). Therefore it must **occupy a single
  commit**, and before it, batch a must have already run green with a revertable baseline.
- **Do not use feature flags.** Reason: `Operand` variant add/remove cannot be isolated with a
  runtime switch; a feature flag would leave CI not testing the other path for a long time—which is
  exactly the recurrence of "test wiring relies on human memory, corruption happens silently"
  recorded in RFC-039.

### Expectation Management: Net Increase of 900-1600 Lines

**This must be made clear upfront, otherwise it will be challenged mid-implementation with "the line
count did not go down".**

| Item                                      | Line count change                                                                                                                                                     |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                            | `ir_gen.rs` **8448**                                                                                                                                                  |
| Delete 3 statement-level reclamations     | **−15**                                                                                                                                                               |
| 6 save/restore pairs switched to RAII     | net ±0 (RAII guard definition ~+20, usage sites go from 22 lines to ~12 lines)                                                                                        |
| `next_temp_reg` refactor                  | +5                                                                                                                                                                    |
| `generate_call_expr_ir` split             | **+40 ~ 80** (the overhead of signatures and parameter passing for 6 `emit_*`; this is the typical cost of a pure-move split, **splitting increases the line count**) |
| Span-keyed consumption counter            | +15                                                                                                                                                                   |
| `method_def_ordinals` read-only-ification | +20                                                                                                                                                                   |
| New `synth.rs` file                       | +80 ~ 120                                                                                                                                                             |
| New `verify.rs` file                      | +400 ~ 600 (07 first layer, though in this document's scope, counted independently)                                                                                   |
| New `regalloc.rs` file                    | **+500 ~ 1000**                                                                                                                                                       |
| **Total**                                 | **`ir_gen.rs` ~8448 → 8600 ~ 8800; with new files, net increase 900 ~ 1600 lines**                                                                                    |

**The early expectation of "8448 → 5000~6000" was the wrong direction**; the actual
measurement-derived result is the opposite. The reasons need to be explained:

1. **The core benefit of this document is not reducing line count, but eliminating an entire class
   of "fail silently" defects.** The 6 manual save/restore pairs (22 lines) become about 12 lines +
   one guard type after SSA + RAII—**the line count barely changes, but "forgetting to restore"
   changes from a possibility into a compile error**.
2. **Pure-move splits necessarily increase line count.** Splitting 746 lines of
   `generate_call_expr_ir` into 7 functions requires repeating the signature (this function already
   has `#[allow(clippy::too_many_arguments)]` at `7268`), `&mut self` binding, and error propagation
   in each. **"Splitting files to reduce line count" is a misconception—it reduces cognitive load,
   not physical line count.**
3. **The new validator and allocator are net additions.** These two items (~900-1600 lines) are
   precisely **the carriers of the benefit**.

**An honest statement of the net benefit**:

> In terms of line count, this document is most likely a **net increase**. The benefit lies in
> **verifiability**—turning "fail silently" defects into invariants that "if they don't pass, an
> error is reported". If line count is the acceptance standard, this document will fail; the correct
> acceptance standard is the criterion in the "Batching Strategy" section.

**This phase will cause a net increase in `ir_gen.rs` line count, and that is intentional.** The
2026-10-03 decision has removed all line-count / size gates (see prohibition two of
[08](08-maintenance-mechanism.md)), so **no line-count exemption list is needed**—the legitimacy of
the increase comes from the separation-of-concerns argument: SSA-ification extracts "register
allocation + definition discipline" from lowering into independent modules; the line count increases
in exchange for eliminating two classes of "misalignment-silent-on-failure" manual discipline: the 6
manual save/restore pairs and the `arg_regs` semantic rearrangement.

## Key Decisions and Rationale

### Three Core Decisions

**Decision One: Temporary value strategy—delete the three statement-level rollbacks, making single
definition a mathematical consequence of construction.** The current rollbacks (`1954-1959` /
`3632-3638` / `4737-4742`) exist to reuse temporary slots. Delete them, let `next_temp`
monotonically increase, and every temporary slot naturally has only one definition point. **The
criterion is the same but the guard is different** (the one at `3632` has the `result_reg.is_none()`
guard, the other two do not); this kind of hand-copied difference disappears along with them. The
cost is increased slot occupancy—the 255 upper bound check on `temp_high_water` (`1992-2000`) acts
as a backstop.

**Decision Two: do not mem2reg, only do explicit renaming.** The rationale is already given in
"Value Identification": YaoXiang's `&` / reference / ownership semantics make "which slots are
mem2reg-able" a type system problem, requiring the type representation after
[03](03-type-unification.md) convergence to determine. **Doing mem2reg before 03 is guessing.**
Explicit renaming only needs to change the return-value semantics of `next_temp_reg` at one place +
delete the three rollbacks; it does not touch any lowering algorithm's control flow.

**Decision Three: `Phi` adopts option A (IR-only pseudo-instruction, expanded into a Move
sequence).** `executor/` and the `.42` format have **zero changes**, and the regression surface is
confined to L3. The decisive reason: changing the `.42` format triggers a version-number issue and a
full-compatibility discussion, whereas A's size cost is irrelevant under interpreted execution (what
executes is Move, not Phi). The cost is that `Phi` is invisible in `dump_bytecode` (care is needed
when entering the C1/C2 phase; the third layer of 07 must compare it).

### Alternatives Rejected

**A. All-at-once split (do SSA + splitting + `synth` boundary + allocator simultaneously).
Rejected.** Violates 07's core principle: criteria must be classified by category. Mixing four
batches into one PR, mixing pure-move changes with C4 changes together, makes snapshot drift
impossible to attribute—as soon as the snapshot changes, you can't tell whether the split moved
something wrong or the SSA change went wrong. **Rollback granularity degrades from "change item" to
"everything".**

**B. Split files first, then optimize ordering. Rejected.** Splitting does not eliminate any root
cause: 6 manual save/restore pairs are still scattered across 10 files, and `arg_regs` semantic
rearrangement still requires manual reasoning. Worse, **splitting first loses the opportunity to
"build the criterion before changing"**—the split produces a huge amount of line-level diff that
gets mixed into the diffs of subsequent SSA changes, voiding review. This is the same source as the
8 orphaned test trees (1005 lines / 78 tests never run) recorded in 06-cleanup-inventory.md: **when
the order is wrong, every subsequent step accelerates on a wrong foundation.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as "the third path once considered".** The form is: keep `ir_gen.rs` unchanged, add a new pass that
converts its non-SSA IR output into SSA IR. **The advantage** is that lowering is completely
untouched and rollback is easy. **Rejection reasons**: (1) It does not eliminate the root cause of
defect class 1—the three statement-level reclamations remain in `ir_gen.rs`, and the 6 save/restore
pairs are still 59-161 lines apart; a conversion pass can merge multiple definitions into `Phi`, but
**it cannot fix "the inner function's name leaking into the outer"**—that is a `cur_locals`
save/restore error, occurring before the IR form, and a conversion pass cannot see it. (2) It turns
SSA into a second representation—the repo has the prior failure of 3 parallel type representations,
and this project's core lesson is that "**boundaries only exist in human awareness**". (3)
`verify_loose` becomes a permanent burden.

**D. Keep the status quo, just add a verifier. Rejected as the only solution, but it is part of this
document.** RFC-039's Alternative B has already given the same judgment: a gate can prevent
regression, but cannot fix the status quo. **But it must be made clear what D can achieve**: D can
**catch instances** of defect classes 1/2/3 (provided the verifier covers dominance, type
consistency, and inner-layer isolation), but it cannot catch defect class 4 (`closure_captures`
leak)—that requires a cross-function liveness assertion, beyond the function boundary of
`verify(&ModuleIR)`. **Therefore D and this document are not alternatives, but a prerequisite
relationship.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900–1600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a
  risk**—the 2026-10-03 decision has removed line-count / size gates, and scale issues are resolved
  by separation of concerns (see prohibition two of [08](08-maintenance-mechanism.md)). The
  legitimacy of the increase lies in: it replaces two classes of "misalignment-silent-on-failure"
  manual discipline, namely the 6 manual save/restore pairs and the `arg_regs` semantic
  rearrangement.
- **`verify.rs`'s "single definition" check is meaningless under the non-SSA form**
  (`Operand::Local` allows multiple definitions)—07 has already recorded this limitation, and
  `verify_loose` is its degraded mode before this batch.
- **Deleting the statement-level reclamations will change the temporary slot allocation pattern**
  (batch a). Even with `temp_high_water` guaranteeing no overflow, **the allocation result will
  change**—this is exactly why the "behavioral equivalence" criterion in the C4 phase must truly run
  the full corpus, not spot checks.
- **Whether the `u8` / 255 slot upper bound** (`codegen/operand.rs:39-45`) is still sufficient after
  deleting the rollbacks is **unverified**. The E3014 check in `generate_function_ir` (`1992-2000`)
  will catch it, but catching means compilation failure—which may expose a batch of oversized
  functions previously masked by "rollback".
- **The advantages must be recorded as well**: turning "errors that cannot be measured" into
  "invariants that can be measured"; not changing the backend (`executor/` and `.42` have zero
  changes); not changing `Instruction` semantics (all 76 → 77 variants preserved, all 50
  `translate_*` preserved); incremental (each item is independently revertible).

> **The open questions originally listed in this section have all been adjudicated.** See the
> [RFC-039 Decision Registry](../rfc/draft/039-compiler-architecture.md) (D1–D50) for item-by-item
> decisions. **This document leaves no open items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../rfc/draft/039-compiler-architecture.md) — The
  overarching outline; four-layer model, criterion classification, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **Hard prerequisite**; C4 category
  criterion definition, `verify_loose` / `verify_ssa`, three-layer criteria, the snapshot's
  limitation against `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **Hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; the `PlanId` refactor for
  `ReleasePlan`
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphaned test trees; the cautionary tale of
  "build the directory first, then fail to wire it up"

### Code Locations

**IR Definition**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` ([03](03-type-unification.md) scope)
- `src/middle/core/ir.rs:12-20` — `Operand`'s 7 variants
- `src/middle/core/ir.rs:47-533` — `Instruction`'s 76 variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for the `PlanId`
  approach)
- `src/middle/core/ir.rs:689-719` — `all_instructions` and other accessors

**Defect Class 1 (save/restore)**

- `src/middle/core/ir_gen.rs:248-249` — Explicit contract "must be physically adjacent to the
  `next_temp` save/restore points"
- `src/middle/core/ir_gen.rs:238-242` — Same comment convention for `cur_span`
- `src/middle/core/ir_gen.rs:884-891` — `next_temp_reg`, including the `temp_high_water` true
  high-water mark
- `src/middle/core/ir_gen.rs:1954-1959` / `3632-3638` / `4737-4742` — Three statement-level register
  reclamations
- `src/middle/core/ir_gen.rs:1798` vs `2001` — `take_cur_locals` criterion inconsistency
- `src/middle/core/ir_gen.rs:1987-2000` — E3014 register overflow check (`MAX_REGISTERS = 255`)
- `src/middle/core/ir_gen.rs:1337` — `rebase_jump_targets` (the "jump target exists" invariant
  of 07)
- The 6 save/restore function entries: `1677` / `1830` / `2105` / `2209` / `2759` / `4678`

**Defect Class 2 (arg_regs)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — Three semantic rearrangements
  and the clone
- `src/middle/core/ir_gen.rs:7838-7846` — Silent "supply 0" fallback

**Defect Class 3 (span keying)**

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — Declarations of the three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — Write and consumption of `release_plan`
- `src/middle/core/ir_gen.rs:7420` / `7493` — Consumption of `overload_resolutions`
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — Initialization and sole write point of
  `method_def_ordinals` (key is the module-unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — Two fields on the transport structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — Cross-layer transport
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  `overload_resolutions` production

**Defect Class 4 (implicit order dependencies)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — `generate_spawn_for_ir`'s AST synthesis and
  callback
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — Same handshake in `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — Two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — 3 other `ast::Type` constructions

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr`'s 66 variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1408 lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — Three hardcoded drops (compare with `2344`
  / `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (the bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  downgrades
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, `u8` upper bound 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; the file has 83 constants in total
- `src/backends/interpreter/executor/ops/control.rs:90` — Live implementation of
  `BytecodeInstr::Switch` (no corresponding variant on the IR side)
- `src/backends/interpreter/executor/debug.rs:194` — Dispatch match

**Test Status Quo**

- `src/middle/core/ir_gen.rs` — 8448 lines, 0 `#[cfg(test)]`, 0 `mod tests`
- `src/middle/core/tests/mod.rs` — Declares `bytecode` / `def_assign` / `local_slots`, 29 tests in
  total; `def_assign` / `local_slots` positively pin DefId and slot naming, no IR structural
  invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 mentions of ir_gen, all
  reverse assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop`
  (per-opcode round-trip criterion example, the last test in the file; the file has 1209 lines / 23
  `#[test]` in total)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies include only `criterion` / `proptest`; `[dependencies]` has
  no regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — The sidebar does an auto-scan of the `/design/rfc/draft`
  directory; this document is not under that directory, and is referenced by RFC-039 and this
  directory index
