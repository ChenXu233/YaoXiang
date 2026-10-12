# Intermediate Representation SSA-ization

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiered acceptance criteria, and execution phase order are in the body of
> RFC-039; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This document addresses the IR construction discipline of layer L3 and proposes a "single definition
per value" construction approach for `src/middle/core/ir_gen.rs` (8,448 lines). **The goal is not to
introduce a new instruction set, but to eliminate three classes of defects that fail without
producing any error**:

1. 6 manual register save/restore pairs scattered dozens of lines apart;
2. The `arg_regs` semantic reordering shared by 6 branches within `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document covers: IR form (`Operand` / `Instruction`) changes, temporary value allocation
discipline, cross-layer contract explicitation, call expression dispatch decomposition, and the
wiring of the IR static validator.

This document does not cover: type representation unification ([03](03-type-unification.md)), stage
contracts and obligation ledgers ([02](02-stage-contract.md)), definition and gating of equivalence
criteria ([07](07-equivalence-oracle.md)), front-end paradigm ([05](05-frontend-paradigm.md)), or
dead code and empty-design cleanup ([06](06-cleanup-inventory.md)). This document references
conclusions from these documents in many places and does not re-argue them.

**The criterion category** is executed as **C4 (IR form change)** per
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariant**, not IR snapshot full equality. SSA-ization necessarily changes IR form; forcing
snapshot equality would induce the team to relax criteria (this is exactly the trap recorded in 07).

**The position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, acceptance is behavioral equivalence + `verify_ssa` green). The internal
implementation batch numbering (batches a–d) within this document is separate from the ordering
within P7: the former is this document's batching strategy, the latter is RFC-039's global phasing.

## Feasibility Prerequisites

**Verified facts.** The IR already has **all the structural prerequisites** required for SSA
construction:

| SSA prerequisite             | Current state in this repo                                                                                | Location              |
| ---------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                          | `ir.rs:624-628`       |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                            | `ir.rs:636-642`       |
| Scope nesting information    | `LocalSlot::scope_depth` ("0 = function parameter layer, for disambiguating same names in nested scopes") | `ir.rs:658`           |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                        | `ir.rs:47-533`        |
| Per-instruction span         | Each variant carries a `span: Span` field                                                                 | `ir.rs:47-533`        |
| Value type annotatable       | `LocalSlot::ty: MonoType`                                                                                 | `ir.rs:656`           |
| Slot count upper-bound check | E3014 check inside `generate_function_ir` (`MAX_REGISTERS = 255`)                                         | `ir_gen.rs:1987-2000` |

**If these prerequisites were not in place, this document's cost estimate would be entirely
different.**

**The two decisive points:**

**First, SSA-ization is not adding a capability, it is deleting a capability dependency.** In the
current state, temporary slots are **actively designed to be multiply defined**—three explicit
rollbacks of `next_temp` at `1954-1959` / `3632-3638` / `4737-4742`. The first thing SSA-ization
does is **delete these three rollbacks**, letting `next_temp` monotonically increase. After
deletion, **each temporary slot naturally has only one definition point**—single definition shifts
from "a rule that needs to be checked" to "a mathematical consequence of the construction method".

**Second, the slot count upper bound is already a hard check.** `temp_high_water` at `ir_gen.rs:889`
is already tracking the true high-water mark, and `1992-2000` is already reporting errors against
the 255 upper bound. **Removing rollbacks will not let slot counts go out of
control**—`temp_high_water` records the historical peak usage, which is independent of whether
rollbacks happen (the comment at `887-888` says exactly this).

**The combined implication of these two points**: `ir_gen.rs` is already maintaining all the
bookkeeping information needed for SSA, it has merely **thrown this information away**. The workload
of SSA-ization is mainly in splitting `generate_call_expr_ir` and explicit-izing cross-layer
contracts, not in building IR expressive capability.

### Prerequisite: Type Representation Unification

**Verified facts.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` separately has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. A third set on the serialization side is `ir::Type`, bridged by
`impl From<MonoType> for IrType` at `bytecode.rs:2353`.

That is: **two type representations exist in the IR simultaneously, and one of them directly
`pub use`s the AST's type.**

SSA-ization needs to add a `Phi` variant, record a type for each SSA value (for the "type
consistency" invariant of the 07 validator), and judge whether types from two predecessors at a CFG
merge are compatible. **If the type representation has not yet been unified, SSA will write one
type-compatibility check per representation**—and the third representation will grow on top.

This is not a "which to do first" question, it is a question of **doing this first will produce
three sets of type-compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first complete the
`pub use ast::Type` unification at `ir.rs:3` and the bridge teardown at `bytecode.rs:2353`. Item 6
of this document's change list explicitly records that `ir.rs:3` **is not touched by this
document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not a "ugly but working" file. It is a file **that bets correctness on per-line human
review**:

- The file contains **0 `#[cfg(test)]` and 0 `mod tests`** (verified by measurement).
- `src/middle/core/tests/mod.rs` declares three modules `bytecode` / `def_assign` / `local_slots`,
  with 29 tests in total—among which `def_assign.rs` and `local_slots.rs` **positively** cover
  ir_gen's DefId allocation and local slot naming (sentinel-level), and `bytecode.rs` references
  ir_gen 4 times (`421` / `423` / `1028` / `1173`) **all as reverse assertions**—"ir_gen does not
  produce", "ir_gen also does not construct", "ir_gen front-end does not construct the corresponding
  IR".

That is: ir_gen has 29 sentinel-level tests nailing down local behavior, but **has no IR structural
invariant checks** (dominance, single definition, jump targets, type consistency)—not one of the
three modules declared in `middle/core/tests/mod.rs` makes positive assertions about the overall IR
form.

And this file happens to contain several classes of defects that **compile successfully, all checks
are green, the program runs, only the values are wrong**. Their common property: **failure produces
no error**.

This is the highest risk area. Not "the function is too long", not "naming is messy", but **the core
invariants of this file have no machine-enforceable mechanism**.

`ir_gen.rs:248-249` writes this as an explicit textual contract:

> Before generating a nested function body, save; afterward, restore—**must be physically adjacent
> to the `next_temp` save/restore points**, otherwise inner function names will leak into the outer
> scope.

This is a **rule expressed as a comment in place of an assertion**. It holds today only because
those 6 save/restore pairs currently all happen to be written correctly.

**Why it is not sufficient to "just add a verifier"**: a verifier can **catch** these defects, but
it cannot catch their **root cause**. The root cause is not "missed something during review";
rather:

> **The current IR form itself permits a slot to be multiply defined, and `ir_gen`'s temporary
> register allocator actively depends on this.**

Concrete evidence is three statement-level register reclamations (`1954-1959` / `3632-3638` /
`4737-4742`)—at the end of each statement they **roll back** `next_temp` to the maximum of "the last
named slot ⋈ `temp_floor`" (`1959`: `self.next_temp = named_watermark.max(self.temp_floor);`). This
means the same `Operand::Local(n)` slot is **written to repeatedly** within a single function body,
and the write count, write order, and live range are entirely decided by the single manual decision
of "when to roll back".

In this form, "single definition" is not a rule that can be incrementally imposed, but is **in
direct conflict with the register allocation policy**. So SSA-ization is not adding a check to
existing code, it is **replacing the policy that depends on multiple definitions**. The good news is
that the replacement cost of this policy is very low—see "Feasibility Prerequisites" above.

### Defect Class 1: 6 manual save/restore pairs, no error on failure

**Verified facts.** 6 save/restore pairs, with the longest distance between save and restore being
**161 lines**:

| Save point            | Restore point | Span      | Function                        | Entry line |
| --------------------- | ------------- | --------- | ------------------------------- | ---------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | 63 lines  | `generate_method_ir`            | `1677`     |
| `1858-1866`           | `2026-2029`   | 161 lines | `generate_function_ir`          | `1830`     |
| `2116`                | `2180`        | 64 lines  | `generate_curry_innermost_func` | `2105`     |
| `2224-2229`           | `2284-2288`   | 59 lines  | `generate_curry_function_ir`    | `2209`     |
| `2768-2777`           | `2820-2851`   | 74 lines  | `generate_anon_binding_ir`      | `2759`     |
| `4686-4695`           | `4767-4774`   | 72 lines  | `generate_lambda_body_ir`       | `4678`     |

> **Attribution note**: the two curry function correspondences are `2116`/`2180` belonging to
> `generate_curry_innermost_func` (entry `2105`), and `2224-2229`/`2284-2288` belonging to
> `generate_curry_function_ir` (entry `2209`)—i.e. the "innermost" goes with the former; do not
> reverse-infer by line-number intuition.

Fields involved: `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span` /
`loop_stack`, all driven by `next_temp_reg` (`ir_gen.rs:884-891`). This function maintains the true
`temp_high_water` at `889` (comment self-stated: statement-level reclamation rolls back `next_temp`,
total slot count is based on historical peak usage). A similar comment convention exists for
`cur_span` (`238-242` self-stated: "before generating nested function body save, after restore (same
handling as next_temp)").

**Two consistency discrepancies** (a more subtle problem than "missed one"):

| Location         | Criterion                                                                                                             |
| ---------------- | --------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:1798` | `take_cur_locals(param_types.len())` — slice by **parameter count**                                                   |
| `ir_gen.rs:2001` | `take_cur_locals(total_locals)`, where `total_locals = self.temp_high_water` (`:1992`) — slice by **high-water mark** |

`generate_method_ir` uses the former, and **lacks the E3014 overflow check from `1992-2000`**. The
two functions give two different answers to "how many slots should the function body have", and
neither errors.

**Three duplicated statement-level register reclamations, same criterion but different guards**:

| Location    | Function                           | Guard                     |
| ----------- | ---------------------------------- | ------------------------- |
| `1954-1959` | `generate_function_ir`             | None                      |
| `3632-3638` | `generate_block_ir` (entry `3602`) | `if result_reg.is_none()` |
| `4737-4742` | `generate_lambda_body_ir`          | None                      |

The guard at `3632` has an explicit reason (comment at `3626-3631`: blocks at the expression-operand
position; the outer layer may hold sibling-argument temporaries that survive across blocks). The
other two have no equivalent guard. **The same criterion is hand-copied in three places, one has a
guard and two do not**—this kind of difference is exactly what SSA is meant to eliminate.

**The restore point of `generate_anon_binding_ir` is split by IR construction** (`2768-2777` save →
`2820-2824` restore 5 fields → `2829-2848` construct `func_ir` → `2851` finally restore
`loop_stack`). The two halves of the same state group are physically separated by 22 lines of
construction code, and the order dependency is **reversed** (first restore slot table, then
construct IR, then restore loop stack).

**Consequence**: miss one restore → `next_temp` leaks, temporary values get skipped, `cur_locals`
names leak into the outer scope. **The IR is still self-consistent, still passes all checks, still
compiles, only the values are wrong.**

### Defect Class 2: `arg_regs` semantic reordering in `generate_call_expr_ir`

**Verified facts.** `generate_call_expr_ir` is at `ir_gen.rs:7269-8014`, **746 lines**, a single
function. The function signature (`7269-7279`) takes `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (plus `_expr` / `result_reg` / `instructions` /
`constants`), and inside 6 branches share the same `Vec<Operand>` and perform **semantic
rewriting**:

| Line        | Semantics                                                                                                                                            |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten by `slots` and re-`collect` — named argument reordering                                                                      |
| `7724`      | Struct construction: `unwrap` each slot by `final_args` and re-`collect` — field reordering                                                          |
| `7835-7848` | Function call: named argument reordering; `7838-7846` **pads a new register with value 0** for missing slots (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();` — clone before emitting instructions                                                              |

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

The comment at `7835-7848` self-states: "Uncovered slots: remain absent, and the type check phase
reports E1010. At runtime, pad with 0 as a safety net (normal path will not reach here)."

**This design carries two layers of risk**:

1. **Misalignment does not error, only misvalues.** Each of the 6 branches decides the final order
   of `arg_regs`; any order assumption error will not panic.
2. **Normalized snapshots are immune to this.** 07 has already recorded this limitation: snapshot
   tools rename temporary values by "first-appearance order", so "the 3rd argument should use the
   5th register" looks identical in the snapshot. **This class of defect does not even have
   snapshot-level criteria at the C4 stage.**

**Supplementary verification: the struct construction path's reordering has no duplicate-slot
guard.** The `final_args` at `7724` is filled by "positional arguments placed first, named arguments
overwrite by field name", with **no `slots[idx].is_some()`-style duplicate-slot check before
overwrite** (in contrast, the namespace path at `7381` has one)—a named argument silently overwrites
a positional argument without panicking or diagnosing; and the **argument evaluation order** (source
order) **diverges here from the final `arg_regs` order** (field declaration order). Batch d's
`fill_missing` assertion-ization must also cover the "duplicate slot" check, otherwise the struct
construction path's silent overwriting will survive intact.

### Defect Class 3: Span-keyed cross-layer contracts silently fail

**Verified facts.** There are **two** span-keyed cross-layer contracts in the entire repo:

| Contract               | Declaration                                  | Production                                                                               | Transit                                                                | Consumption              |
| ---------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------ |
| `release_plan`         | `ir_gen.rs:191` `HashMap<Span, Vec<String>>` | `ownership.rs:31` (`ReleasePlan::drops` field), `2574` `build_release_plan`, `2786` call | `typecheck/types.rs:31` ← `checker.rs:1282` / `1440` ← `ir_gen.rs:341` | `ir_gen.rs:1942`         |
| `overload_resolutions` | `ir_gen.rs:222` `HashMap<Span, String>`      | `inference/expressions.rs:4366`, `inference/statements.rs:2758` / `2857`                 | `typecheck/types.rs:56` ← `checker.rs:1447-1448` ← `ir_gen.rs:364-366` | `ir_gen.rs:7420`, `7493` |

The form at `1942`:

```rust
// NLL Release: insert Drop instruction at statement boundary
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some` — **a miss is just a miss, no else branch, no assertion, no counter**. Any
inconsistency in how the two sides compute span → **Drop instructions silently disappear, zero
errors**. The Drop sequence of refinement types is the core carrier of ownership semantics in this
project (`ownership.rs:26-32` self-stated: "NLL precise release plan", keyed by Span of last-use
position), and its landing is silent.

`7420` / `7493` share the same form (`if let Some(mangled) = self.overload_resolutions.get(span)`),
with the consequence that overload resolution falls back to the default binding.

> **`method_def_ordinals` is not span-keyed.** It is `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with the sole write point at `:1692-1702` (allocating `#N` suffix by
> `base_name`'s definition order). Its risk is of a **different kind**: definition-order
> dependency—the `#N` suffix is decided by the AST definition order; if the generation order is
> inconsistent with the typecheck registration order, the same function name resolves to different
> targets. This is not a span mismatch, but it equally "errors without reporting"; disposition in
> "Additional mechanisms for span-keyed", mechanism three.

### Defect Class 4: spawn/lambda implicit handshake

**Verified facts.** `ir_gen.rs` has **two places** where it itself constructs `ast::Expr::Lambda`
and calls back into `generate_expr_ir`, relying on a group of mutable fields to handshake:

**Handshake A — `generate_spawn_for_ir` (entry `4143`):**

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir` (entry `6543`) → `6571-6572`
`std::mem::take(&mut self.pending_env_vars)` consumes.

**Handshake B — `generate_spawn_expr_ir` (entry `6641`):**

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**Handshake C — `closure_captures`**: written at `6589-6593`, **in the same function** `6598` calls
`generate_lambda_body_ir` (entry `4678`), `6600` clears. There are two read points: `:6284`
(`LoadUpvalue` in closure body) and **`:458`** (`closure_captures.contains_key(head)` on the
`flatten_namespace` path).

The read point at `:458` is the most dangerous: it sits on the **namespace resolution** path,
meaning the alive state of a `HashMap<String, usize>` can change the variable name resolution
result. Between `6589`'s write and `6600`'s clear lies a complete closure body generation (`6598`);
if any path inside `generate_lambda_body_ir` returns early without clearing the table (it has
multiple return points when `?` propagates errors), **`closure_captures` will leak into unrelated
code at `:458`**.

The same kind of implicit state also includes `pending_env_names` (B-handshake writes at `6699`,
clears at `6714`).

### What SSA Will Not Solve (Honestly Listed)

| Problem                                           | Location                                                                                                                   | Why SSA does not solve it                                                                                                                                                                                                 |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                      | `ir_gen.rs:5390-5687`, **298 lines**, self-recurses 4 times (`5473` / `5504` / `5619` / `5640`), external call site `5270` | The dispatch complexity of pattern matching (union payload / tuple / struct / literal / nested) is **intrinsic to the algorithm**. SSA only turns guard-produced `JmpIfNot` into `Phi`, not reducing the number of guards |
| `eval_const_expr` complexity                      | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)    | Constant folding's expression coverage is independent of SSA                                                                                                                                                              |
| Span-keyed failure                                | `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)                                                          | **SSA does not touch this layer at all.** Requires independent mechanism, see "Additional mechanisms for span-keyed"                                                                                                      |
| `Instruction` variant count                       | 76 (`ir.rs:47-533`)                                                                                                        | SSA adds `Phi`, **net change +1**. The 76 variants correspond to 76 bytecode opcodes or downgrade to NOP, not reduced by SSA                                                                                              |
| `method_def_ordinals` definition-order dependency | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                      | Same kind as span-keyed but different cause, see "Additional mechanisms for span-keyed" mechanism three                                                                                                                   |

## Target Design

### SSA Form Definition

**Design judgment.** The target form is as follows.

#### How to change the `Operand` variants

Current state `ir.rs:12-20` has 7 variants in total. **Whole-repo construction-point survey**:

| Variant             | Production construction points                                                                                                                                              | Disposition                                                                                |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `Const(ConstValue)` | 8 files                                                                                                                                                                     | **Keep**, unchanged                                                                        |
| `Local(usize)`      | 7 files                                                                                                                                                                     | **Keep but split semantics** (see below)                                                   |
| `Arg(usize)`        | 4 files                                                                                                                                                                     | **Keep**, as implicit definitions of function entry (parameters are block 0's definitions) |
| `Global(usize)`     | 5 files                                                                                                                                                                     | **Keep**, unchanged                                                                        |
| `Temp(usize)`       | **0 construction points** (only `codegen/operand.rs:39-45` and `:75` have handling arms)                                                                                    | **Remove** (dead variant)                                                                  |
| `Label(usize)`      | **0 construction points**                                                                                                                                                   | **Remove** (dead variant)                                                                  |
| `Register(u8)`      | **0 construction points** (`ir.rs:19` comment claims "Added for codegen", but codegen goes through `codegen/operand.rs`'s `OperandResolver`, not constructing this variant) | **Remove** (dead variant)                                                                  |

`Local(usize)` is split into two variants, because currently it **simultaneously carries two
mutually exclusive semantics**—"storage location of a named variable" (writable multiple times) and
"compiler temporary register" (should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: dense ID within function, single definition
    Value(ValueId),
    /// Storage slot of a named local variable: addressable, allows multiple definitions (non-SSA part)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a `u32` newtype (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within a function, **monotonically increasing, never rolled back**.

#### `Phi` as a new `Instruction` variant

```rust
/// Merge point: each predecessor basic block provides one definition for this value
/// incoming sorted by predecessor block label ascending (guaranteed at construction, for canonicalization)
Phi {
    dst: Operand,                                  // Must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (all enforced by `verify_ssa`, see [07](07-equivalence-oracle.md)
layer 1):

1. `incoming.len() == the number of predecessors of this block`.
2. The label set of `incoming` == the set of labels of all blocks in `blocks` whose `successors`
   contain this block's label.
3. The `MonoType` of all `incoming` values is the same (**depends on type representation being
   unified by 03**).

**Disposition of the guard problem at `3632-3638`**: the current `generate_block_ir` reclamation has
the `if result_reg.is_none()` guard (comment at `3626-3631`: blocks at the expression-operand
position; the outer layer may hold sibling-argument temporaries that survive across blocks). After
SSA-ization, this guard **is no longer needed**—because temporary values are no longer rolled back,
sibling-argument temporaries surviving across blocks hold their own `ValueId`, not affected by
rollback. **This is a place where SSA-ization can actually simplify code, not just rename things.**

#### Value identification approach

**Design judgment**: do not do mem2reg, do not use virtual register numbers, **only do explicit
renaming**. Reasons:

- YaoXiang's `&` / reference / ownership semantics make "which slots can be mem2reg'd" a **type
  system problem**, not a CFG problem. It is verified that `ir.rs:656` `LocalSlot::ty: MonoType`
  carries complete type information, but determining "whether this `MonoType` can be mem2reg'd"
  requires the cooperation of the unified type representation from 03 and refinement type rules.
  **Doing mem2reg before 03 is guessing.**
- Explicit renaming only needs to change the return value semantics of one place, `next_temp_reg()`
  (`ir_gen.rs:884-891`), plus delete three rollbacks, **without touching any lowering algorithm's
  control flow**.

Value table form: `FunctionBody::Code` adds `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot), None means pure temporary
    pub debug_name: Option<String>,
    pub defining_block: usize,   // Label of the basic block where the definition resides
}
```

`values` and `locals` coexist: `locals` carries addressable named variables (possibly multiply
defined), `values` carries single-definition SSA values. **The existence of `locals` does not
violate SSA**—it is exactly the "storage" part outside SSA (LLVM's alloca model).

#### Change-size estimate

| Item                                                 | Estimate               | Basis                                                                                                                    |
| ---------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `next_temp_reg` refactor                             | ~15 lines              | `ir_gen.rs:884-891` single function                                                                                      |
| Delete three statement-level reclamations            | −15 lines              | `1954-1959` / `3632-3638` / `4737-4742`                                                                                  |
| 6 save/restore pairs consolidated to RAII            | net ±0 lines           | See "Item-by-item disposition table"                                                                                     |
| `ir.rs` structural change                            | +120 ~ 180 lines       | `ValueId` / `Phi` / `ValueInfo` / value table                                                                            |
| Add `Phi` arm to `translator.rs`                     | +25 ~ 40 lines         | One of the existing 50 `translate_*`                                                                                     |
| Add `PHI` instruction or runtime fill to `executor/` | +60 ~ 120 lines        | Depends on the bytecode-form selection of `Phi`                                                                          |
| Rename `Operand::Local` → `Value` call sites         | mechanical replacement | **Verified**: `Operand::Local(` appears **323** times within `ir_gen.rs`, **355** times across 6 files in the whole repo |

### Item-by-item Disposition Table

| Defect class                                       | Whether SSA eliminates     | Disposition                                                                                                                                                                                                                                                                                                                                                  | Criterion                                                            |
| -------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| **1. 6 manual save/restore pairs**                 | **Yes (root-cause level)** | `next_temp` / `temp_high_water` / `temp_floor` become `&mut` monotonic counters, no need to save across function bodies. `cur_locals` / `loop_stack` / `cur_span` switched to RAII guard (`struct NestedBodyGuard<'a>`, restore on `Drop`), **making "forgot to restore" a compile error**. The comment contract at `248-249` is enforced by the type system | 07 layer 1 "inner isolation" invariant + `verify_loose`/`verify_ssa` |
| **1b. Criterion inconsistency (`1798` vs `2001`)** | **Yes**                    | Unified to `take_cur_locals(self.temp_high_water)`, with the E3014 check (`1992-2000`) extracted to a shared function `check_register_budget()`, called from both places                                                                                                                                                                                     | Corpus diff + snapshot (pure relocation phase can be zero-diff)      |
| **1c. Three statement-level reclamation criteria** | **Yes**                    | All three deleted (`1954-1959` / `3632-3638` / `4737-4742`), including the `result_reg.is_none()` guard at `3632`                                                                                                                                                                                                                                            | C4: behavioral equivalence + invariants                              |
| **2. `arg_regs` semantic reordering**              | **Partial**                | SSA turns "wrong register" into "wrong value ID" and can be caught by dominance checks, but **the reordering logic itself remains**. Real elimination relies on function decomposition—6 branches each `push`, `arg_regs` ownership stays at the top of the function                                                                                         | Layer 1 dominance invariant (snapshots are immune)                   |
| **3. Span-keyed failure**                          | **No**                     | SSA does not touch it at all. See "Additional mechanisms for span-keyed"                                                                                                                                                                                                                                                                                     | 07's `test_release_plan_spans_consumed`                              |
| **4. Implicit order dependency**                   | **No**                     | See "`synth.rs` boundary"                                                                                                                                                                                                                                                                                                                                    | New explicit criteria required                                       |
| `compile_pattern` 298 lines                        | No                         | See "What SSA will not solve"                                                                                                                                                                                                                                                                                                                                | —                                                                    |
| `eval_const_expr` 191 lines                        | No                         | See "What SSA will not solve"                                                                                                                                                                                                                                                                                                                                | —                                                                    |
| `Instruction` 76 variants                          | No (+1)                    | Add `Phi` → 77                                                                                                                                                                                                                                                                                                                                               | —                                                                    |

### Register Allocator: In-house or Existing Crate

**Verified facts (decisive)**:

1. `Operand::Register(u8)` (`ir.rs:19`) has **0 construction points in the whole repo**.
2. The backend is not a register machine—`src/middle/passes/codegen/operand.rs`'s `OperandResolver`
   resolves `Operand::Local/Temp/Arg` to `u8`, upper bound 255 (the `Temp` arm overflow at `39-45`
   is E3014).
3. `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1577 lines, 50 `translate_*`,
   function bodies at `101-1558`) outputs `BytecodeInstruction::new(opcode, operands)`, where
   opcodes come from `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter's opcode dispatch family.

**Conclusion**: the so-called "register" is **an operand stack slot index, not a physical
register**. The target machine **has no finite register file, no register pressure, no spill/split
requirements**.

**Evaluating three options**:

| Option                                               | Evaluation                                                                                                                                                                                                                                                                                                                                                                                  |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (rustc lineage)             | **Not adopted.** Both take rustc's `Function` trait + MIR's `Place`/`Operand` abstractions as input. Writing an adapter for YaoXiang IR requires implementing the full trait (CFG, liveness, spill slot, scratch), **500+ lines of adapter code** for zero benefit—because there is no real register file to color. And rustc itself only does linear scan here (when `enable-llvm` is off) |
| `graph-alloc` series                                 | **Not adopted.** Same as above, with even less stable API                                                                                                                                                                                                                                                                                                                                   |
| **In-house linear scan (Chaitin-Briggs simplified)** | **Adopted.** What YaoXiang actually needs is: mapping each `ValueId` to a non-conflicting `u8` stack slot in `values` order (`OperandResolver` already has the `u8` contract and E3014 upper bound). This is essentially **max-clique coloring of non-overlapping live ranges**, which under no-register-file constraints degenerates to "first-available slot in definition order"         |

**Design judgment**: in-house, target 500-1000 lines (including `values → Reg` mapping, live-range
computation, and the interface with `OperandResolver`). **At the same time, must acknowledge**: if
the `u8`/255 slot upper bound is insufficient under real load, the in-house allocator must handle
overflow, and the existing code already treats this as an error in `1992-2000`. **This means this
document does NOT solve** the "large function with not enough registers" problem—**that is a
downstream consequence of `compile_pattern`'s recursion depth, a separate topic.**

`Cargo.toml:37-39`'s dev-dependencies only have `criterion` / `proptest`; no regalloc / petgraph /
graphlib in `[dependencies]`—introducing any crate adds a new dependency surface.

### Decomposition strategy for the 746-line `generate_call_expr_ir`

**Must be done last**, and **must have the SSA form and criteria in place first**. Reason: the
decomposition itself is C1 (pure relocation, criterion is snapshot zero-diff), but only after
`arg_regs`' semantics are reduced by SSA to "one independent `ValueId` per argument" will the
decomposition not transplant the old order assumptions into the new file as-is.

**Core design constraint: `arg_regs` ownership stays at the top of the function; the 6 branches only
`push`, never reorder.**

```
generate_call_expr_ir (746 lines)
├── Argument evaluation: evaluate in source order, push to arg_regs — shared by 6 branches, the only place appending is allowed
├── CallArgs (value object)
│   └── arg_regs: Vec<Operand>  — created only at the top, 6 branches only push, never re-collect/reorder
├── Branch 1 → emit_namespace_call(args: CallArgs)         [7395 branch extracted]
├── Branch 2 → emit_bound_call(args: CallArgs)              [7406+ branch extracted]
├── Branch 3 → emit_struct_ctor(args: CallArgs)             [7724 branch extracted]
├── Branch 4 → emit_plain_call(args: CallArgs)              [7835-7848 branch extracted]
├── Branch 5 → emit_curried_call(args: CallArgs)
└── Branch 6 → emit_method_call(args: CallArgs)             [7420 / 7493 branch extracted]
```

The "pad 0" logic at `7835-7848` (`7838-7846`) is lowered to
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, **and the self-stated
comment "normal path will not reach here" is replaced with an assertion**: if any slot is absent and
typecheck has not reported E1010, directly return a diagnostic. **This turns a silent fallback into
a hard error, and is this document's most direct improvement on defect class 2.**

**Criterion**: pure-relocation parts (SSA form unchanged) use snapshot zero-diff; `fill_missing`'s
assertion-ization belongs to C4, using corpus diff + layer 1 validator.

### The `synth.rs` Boundary: Turning Implicit Callbacks into Explicit Synthesized AST Boundaries

**Verified facts**: there are **exactly 2** locations in `ir_gen.rs` that construct
`ast::Expr`—`:4246` and `:6701` (additionally `:6704` constructs `ast::Stmt`, and `:510` / `:1266` /
`:1399` construct `ast::Type`).

**Proposal**: create `src/middle/lower/synth.rs` as the **only module in the whole repo** allowed to
construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not "tidiness" but:

> Constructing AST means **re-entering the front-end lowering entry point**. And the
> `pending_env_vars` / `closure_captures` state that `generate_expr_ir` expects is a **prerequisite
> that the caller is responsible for preparing**, and this prerequisite is currently not expressed
> by any type or signature—it holds only because of the runtime fact of "who called me".

Change to explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST capturing env; capture set given explicitly by parameter,
/// does not depend on caller having written to self.pending_env_* beforehand
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Specific changes:

| Location                       | Current state                                                                              | After change                                                                                                                                                                                             |
| ------------------------------ | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → construct Lambda → callback                                     | Call `synth::synth_closure(...)` to get AST, pass `SynthEnv` explicitly; `generate_lambda_expr_ir` (`6543`) gains an internal entry that carries `SynthEnv`, **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → construct → callback → clear                                                       | Same as above; the `clear()` at `6713-6714` disappears (no shared state to clear)                                                                                                                        |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                    | Read from parameter instead                                                                                                                                                                              |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                           | Same as above; `closure_captures` is downgraded to a **local variable during closure body generation**, the `clear()` at `6600` disappears                                                               |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads shared field on namespace resolution path | **This is the most dangerous spot**: change to an explicit parameter, otherwise any `?` early return in `generate_lambda_body_ir` would leak the capture table to unrelated code                         |

**Criterion**: add sentinel test `test_no_ast_construction_outside_synth`, scan `src/middle/**` and
disallow `ast::Expr::` / `ast::Stmt {` construction syntax outside `synth.rs`. Form same as
RFC-039's `check-module-boundary.py`.

### Disposition of Span-keyed Issues (Parts SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism 1: consumption-coverage assertion (required, lowest cost).** 07 has already defined
`test_release_plan_spans_consumed` (Span set produced by ownership ⊆ Span set consumed by IR). This
document's addition is making it a **runtime counter** rather than just a test:

```rust
// ir_gen.rs near :341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// Each time :1942 is hit, self.release_plan_hit += 1;
// At the tail of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions for non-`ref` locals are runtime-observable (destructor
side effects), but **"a drop that should happen didn't" is not necessarily an immediate crash under
YaoXiang's current value semantics + Arc/Rc model**—which is exactly why it is silent. Counting
turns "silent" into "detectable".

**Mechanism 2: Span key changed to `DefId` or explicit plan ID (medium term).** The fundamental fix
is **not using Span as a key**. `Span` is a source-code location, subject to drift due to macro
expansion, `include`, and multi-file merging. `FunctionIR` already has a `def: Option<DefId>` field
(`ir.rs:677`), indicating DefId is available at the IR layer. **Design judgment**: introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, allocated by ownership / overload
resolution at production time, matched by `PlanId` when IR consumes. **This falls under 02's
obligation ledger (`Obligations`), should be implemented together with [02](02-stage-contract.md),
not placed in this document.**

**Mechanism 3: `method_def_ordinals` definition-order dependency (independent small item).** This
field is `HashMap<String, usize>` (`ir_gen.rs:224`), key is `"{type_name}.{method_name}"` (**no
module qualification**, written at `:1691-1700`)—if two modules define same-named `Type.method`,
they share the same counter, and the `#N` mangled name drifts. **Its failure mode is not span
mismatch but order/duplicate-name mismatch**—if typecheck's registration order is inconsistent with
ir_gen's definition order, same-named methods resolve to different targets. Disposition: have
typecheck **produce once** a `HashMap<DefId, String>` (raw name or `#N` mangled name), ir_gen only
reads, does not write. **This eliminates a mutable cross-function state inside `ir_gen`.**

### Three Hardcoded Discard Tracking

**Verified facts.** Inside `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`,
1408 lines) there are three hardcoded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                     |
| ------------------ | -------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count zeroes out after `.42` round-trip                                                         |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **`.42` output loses the entire exception table**                                                               |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` output loses global variable information** (note: `2347`'s `global_names` **is** filled, inconsistency) |

**Design judgment (revised 2026-10-05, decision D52): all three absorbed into this document's scope
as P7 batch e (7e), no longer a "separate issue"**. The original judgment (the latter two as
separate issues) is recorded below, along with why it was overturned:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields), unrelated to
   SSA.~~ True, but not grounds for exclusion—P7 already touches `bytecode.rs`, and fixing all three
   at once needs only one `VERSION` bump (4→5), splitting into two bumps is more expensive.
2. ~~Fixing them requires changing the `.42` disk format → version bump → scope balloons from "IR
   form" to "artifact format".~~ The version-bump mechanism already exists (`MAGIC` + `VERSION`
   header, reader side rejects by version), and `.42` is a build artifact, cross-version reading
   compatibility is not a goal—the work inflates, not the risk.
3. ~~`2315` / `2341` are actually low risk; running `.42` directly is a secondary path.~~
   **Overturned**: losing the exception table makes throw/try behave incorrectly when running `.42`
   directly—throw/try are core language semantics; secondary path does not mean silently wrong. This
   refactoring's principle is **no core-feature "to-be-implemented" residue**
   (`Vec::new() // Not implemented yet` is exactly that kind of residue).

**Preserved in parallel**: all three must still be registered in the `Obligations` ledger as
"produced but not serialized" until 7e is complete, to prevent it from becoming a fourth "no one
knows" item. `2312` is landed together with the version bump (D17); already-allocated but unused
opcode values (D32/D34) are reclaimed in the same batch.

## Detailed Design

### Cascading Effects of IR Structural Changes

#### `src/middle/core/ir.rs` (905 lines)

| Change                                                                | Location                | Nature                                            |
| --------------------------------------------------------------------- | ----------------------- | ------------------------------------------------- |
| Add `ValueId(u32)` type                                               | File header around `10` | New                                               |
| `Operand`: remove `Temp` / `Label` / `Register`, add `Value(ValueId)` | `12-20`                 | **Variants add/remove**                           |
| `Instruction`: add `Phi` variant                                      | End of `47-533`         | **Variants +1 → 77**                              |
| `FunctionBody::Code`: add `values: Vec<ValueInfo>`                    | `636-642`               | Field +1                                          |
| Add `ValueInfo` struct                                                | around `660`            | New                                               |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`               | `689-719`               | Need to add `values()` / `values_mut()` accessors |

`ir.rs:3`'s `pub use ...ast::Type` **is not touched by this document**—it is
[03](03-type-unification.md)'s scope. But `ValueInfo::ty` must use the **unified** single type.

#### `src/middle/core/bytecode.rs` (2422 lines)

| Change                                                             | Location                              | Nature                                                                                        |
| ------------------------------------------------------------------ | ------------------------------------- | --------------------------------------------------------------------------------------------- |
| `BytecodeInstr`: add `Phi` or "runtime expansion to Move" strategy | `127-554` (currently **66** variants) | Variants +1 or +0 (see below)                                                                 |
| `opcode()`: add `Phi` arm                                          | `558`                                 | Required (`BytecodeInstr` exhaustive match)                                                   |
| `size()`: add `Phi` arm                                            | `649`                                 | Required                                                                                      |
| `From<BytecodeFile>` decode match: add `Phi` arm                   | `943-2350`                            | Required                                                                                      |
| `upvalue_count: 0` fix                                             | `2312`                                | Fixed in passing by this document (in-memory format; disk requires `VERSION` bump, see below) |

**The bytecode form of `Phi` needs a design decision**. Two options (**design judgment, recommend
A**, reasons in "Key Decisions and Rationale"):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  sequence of `Move { dst: vreg, src: incoming[i] }` at the start of the block, chosen by
  predecessor block index. **No new opcode**, `.42` format zero-change. Cost: bytecode volume
  increases (one per predecessor).
- B: Add `PHI` opcode + predecessor index encoding, interpreter maintains register versions on
  jumps. **Requires changing `executor/` state model + `.42` format + version number**. Benefit is
  only volume.

Decisive reason for choosing A: `.42` format change would trigger version-number issues and
full-compatibility discussions, **and A's volume cost is irrelevant under interpreted execution**
(what executes is Move, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1577 lines, 50 `translate_*`, distributed across `101-1558`)

| Change                                             | Location                                                                                                                         | Nature                                                                      |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `Operand::Value(v)` dispatch                       | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                     |
| `Instruction::Phi` translation arm                 | Inside dispatch match, before `Free` (`533`)                                                                                     | New branch, expanded to a Move sequence (option A)                          |
| Existing 9 NOP downgrades' **positions unchanged** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9**: `655-657` covers 2 variants, `658-660` covers 4 variants |

**SSA-ization will not reduce the NOP count.** These 9 downgrades reflect the "IR has, bytecode does
not" expressiveness gap; SSA-ization does not touch them.

#### `src/backends/interpreter/executor/`

If option A is adopted, `executor/` **needs no changes**—`Phi` disappears from interpreter view at
the `translator.rs` stage. **This is the second decisive reason for choosing A.**

The `BytecodeInstr::Switch` at `executor/ops/control.rs:90` has an active implementation, but in
`ir.rs` **no** `Switch` / `BrTable` / `JumpTable` variant exists (verified: zero hits for these
three names in `ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions the IR layer
cannot produce. It does not affect this document, but is recorded here because it shows that "IR →
bytecode" is not surjective.

The dispatch match at `executor/debug.rs:194` likewise needs no change (no new opcode under option
A).

### Compiler Change List

**Per file, per function.** All line numbers are **starting points** of the change (verified by
measurement):

| #   | File                                      | Location                                         | Change                                                                                                              |
| --- | ----------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/core/ir.rs`                   | `12-20`                                          | `Operand` variant add/remove                                                                                        |
| 2   | `src/middle/core/ir.rs`                   | end of `47-533`                                  | Add `Instruction::Phi`                                                                                              |
| 3   | `src/middle/core/ir.rs`                   | `636-642`                                        | `FunctionBody::Code` add `values`                                                                                   |
| 4   | `src/middle/core/ir.rs`                   | around `660`                                     | Add `ValueInfo`                                                                                                     |
| 5   | `src/middle/core/ir.rs`                   | `689-719`                                        | Add `values()` / `values_mut()` accessors                                                                           |
| 6   | `src/middle/core/ir.rs`                   | `3`                                              | **`pub use ast::Type` not touched** ([03](03-type-unification.md) scope, only recorded)                             |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                        | Return `Operand::Value`, monotonically increasing, no rollback                                                      |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                      | **Delete** statement-level reclamation                                                                              |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                      | **Delete** statement-level reclamation (including `result_reg.is_none()` guard)                                     |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                      | **Delete** statement-level reclamation                                                                              |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                        | Switch to RAII guard (`generate_method_ir`, entry `1677`)                                                           |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                        | Switch to RAII guard (`generate_function_ir`, entry `1830`)                                                         |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                  | Switch to RAII guard (`generate_curry_innermost_func`, entry `2105`)                                                |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                        | Switch to RAII guard (`generate_curry_function_ir`, entry `2209`)                                                   |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                        | Switch to RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                     |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                        | Switch to RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                      |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                      | E3014 check extracted as `check_register_budget()`, shared after unifying `1798`/`2001` criterion                   |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                           | `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`, eliminating criterion inconsistency |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                        | Comment contract deleted (enforced by type system), replaced with explanation pointing to `verify`                  |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                      | Switch to call `synth::synth_closure`                                                                               |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                      | Switch to call `synth::synth_closure`, delete `6713-6714`                                                           |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                      | Read `SynthEnv` from parameter, no longer `mem::take` shared field                                                  |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                             | `closure_captures` downgraded to local, delete `clear()`                                                            |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                            | `closure_captures.contains_key` switched to explicit parameter (**highest-priority implicit dependency**)           |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                   | Add span-keyed consumption counter                                                                                  |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                      | `method_def_ordinals` switched to read-only                                                                         |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                      | **Done last**: split into top `CallArgs` + 6 `emit_*`                                                               |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                      | "Pad 0" fallback changed to return diagnostic                                                                       |
| 29  | `src/middle/lower/synth.rs`               | **New**                                          | Synthesized AST boundary                                                                                            |
| 30  | `src/middle/ir/verify.rs`                 | **New** ([07](07-equivalence-oracle.md) layer 1) | `verify_loose` / `verify_ssa`                                                                                       |
| 31  | `src/middle/passes/codegen/translator.rs` | dispatch match                                   | Add `Instruction::Phi` arm (option A: expand to Move sequence)                                                      |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                | `Operand::Value` resolution arm; remove `Temp` arm (`39-45`)                                                        |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                        | **Unchanged under option A** (`Phi` does not enter bytecode)                                                        |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                       | **Unchanged under option A**                                                                                        |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                           | `upvalue_count: 0` fix                                                                                              |
| 36  | `src/backends/interpreter/executor/**`    | —                                                | **Zero changes under option A**                                                                                     |
| 37  | `src/middle/passes/regalloc.rs`           | **New**                                          | Linear scan allocator                                                                                               |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New**                                          | `synth.rs` boundary sentinel check                                                                                  |

### Backward Compatibility: `.42` Format Version Number

**Verified facts**: the `.42` disk format **already has a version-number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` ("YXBC") and
`VERSION: u32 = 4`, written into the header (`:342-343`), and the reader side validates it
(`:454-467`, mismatch reports "unsupported bytecode version").

**Design judgment (direct corollary of option A)**:

- `Phi` is expanded in `translator.rs` into a `Move` sequence, **producing no new opcode**.
- The 83 constants in `opcode.rs` have **zero changes**, `opcode_name()` (`120`) unchanged, decode
  match (`bytecode.rs:943-2350`) unchanged.
- `.42` format field layout **zero changes**, `VERSION` stays at 4. Old `.42` files can still be
  read by the new binary, and new `.42` files can still be read by the old binary. **SSA-ization
  itself does not need a version bump.**

**The only scenario that needs a version bump is the `2312 upvalue_count` fix** (decision D17: fix,
and handle with the version bump): add an `upvalue_count` field to `BytecodeFunction` and have the
encoder write it, which is a **format-adding-field** change, bumping `VERSION` from 4 to 5. The
existing reader-side version check rejects old files according to existing behavior—`.42` is a build
artifact (`main.rs:671`), cross-version reading compatibility is not a goal, the bump needs no extra
migration mechanism. Already-allocated but unused opcode values (D32/D34's `Switch` / `TailCall`)
are reclaimed together with the version bump.

## Implementation Notes

### Batching Strategy

Four batches. **Each item in a batch is an independent commit, independent criterion, independent
revert.** The "Change item" column in the table below points to the serial number in the "Compiler
Change List" above.

| Batch       | Content                                                                                                                                                 | Change items                 | Criterion                                                                                                                                                         | Revert point                                                                                                           |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Cut off multiple definitions: delete 3 statement-level reclamations → switch 6 save/restore to RAII guard → unify criteria                              | 8-10 (first) → 11-16 → 17-18 | Delete reclamations: **C4** (behavioral equivalence + layer 1 dominance invariant). RAII and criterion unification are pure relocation: **C1** snapshot zero-diff | Per-item single commit, `git revert` reverts                                                                           |
| **Batch b** | SSA form switch: `ir.rs` structural change + mechanical replacement of `Operand` (323 sites) → add `Phi` arm to `translator.rs` → linear scan allocator | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` all green + `.42` round-trip test                                                                                   | `ir.rs` change **occupies a single commit** (type change affects whole-repo compilation, not incrementally revertible) |
| **Batch c** | Implicit contract explicit-ization: `synth.rs` boundary + sentinel script → span consumption counter → `method_def_ordinals` made read-only             | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + new script integrated into CI; consumption counter zero-triggers on new diagnostic (otherwise real mismatch exists)              | Independent commit                                                                                                     |
| **Batch d** | `generate_call_expr_ir` decomposition + "pad 0" fallback assertion-ization                                                                              | 27-28                        | Pure-relocation part: **C1** snapshot zero-diff. Assertion-ization part: **C4** corpus diff + layer 1 validator                                                   | Independent commit                                                                                                     |

**Three non-interchangeable in-batch ordering dependencies:**

1. Batch a's "delete reclamations" must come before batch b's `ir.rs` change—first make `next_temp`
   monotonically increase, so that `ValueId` has the semantic basis of "single definition".
2. Batch b's `Phi` arm must come before batch d's decomposition—otherwise the decomposition will
   transplant the old `arg_regs` order assumptions into the new file as-is.
3. Batch d must be last—it depends on the first two batches having reduced `arg_regs` semantics to
   "one independent `ValueId` per argument".

**Position relative to RFC-039**: all four batches above fall within RFC-039's **P7**; the criterion
tiers (`verify_loose` / `verify_ssa`) are established by P2. The batches are **serial** with respect
to each other, but within batch a, items 8-10 / 11-16 / 17-18 have only ordering constraints, no
coupling.

### Prerequisites (Hard)

**Prerequisite 1: criterion baseline.** The layer 1 validator's `verify_loose` mode from
[07](07-equivalence-oracle.md) **must first run green on the existing (non-SSA) IR** before batch a
is allowed.

Reason: the criterion for SSA-ization is C4 (behavioral equivalence + invariants). If the invariant
validator itself cannot run green on the existing IR before the refactor, then every acceptance of
batches a–d lacks an executable criterion, and can only rely on corpus diffs—and corpus diffs are
exactly **insensitive** to defect classes 1/2/3 (07's "Alternative Plan A" has already argued this).
**This is a real ordering constraint.**

**Prerequisite 2: type representation unification.** [03](03-type-unification.md) must complete the
`pub use ast::Type` unification at `ir.rs:3` and the `From<MonoType> for IrType` bridge teardown at
`bytecode.rs:2353`. **Otherwise, `ValueInfo::ty` will introduce a third set of type-compatibility
checks.**

### Rollback Strategy

- Each change is an independent commit; `git revert` granularity = change-item granularity.
- Batch b's `ir.rs` structural change is the **only non-incrementally-revertible** part (type
  changes affect whole-repo compilation). Therefore it must **occupy a single commit**, and before
  it, batch a must have run green and have a revertible baseline.
- **No feature flag.** Reason: `Operand` variant add/remove cannot be isolated by a runtime switch;
  feature flag would leave CI not testing the other path for a long time—this is exactly the
  recurrence of "test wiring relies on human memory, rot happens silently" recorded in RFC-039.

### Expectation Management: Net Increase of 900-1600 Lines

**Must be said upfront, otherwise it will be questioned by "line count didn't decrease"
mid-implementation.**

| Item                                  | Line change                                                                                                                                                                |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                        | `ir_gen.rs` **8448**                                                                                                                                                       |
| Delete 3 statement-level reclamations | **−15**                                                                                                                                                                    |
| 6 save/restore pairs switched to RAII | net ±0 (RAII guard definition ~+20, use sites go from 22 lines to ~12 lines)                                                                                               |
| `next_temp_reg` refactor              | +5                                                                                                                                                                         |
| `generate_call_expr_ir` decomposition | **+40 ~ 80** (signature and parameter-passing overhead of 6 `emit_*`; this is the typical cost of pure-relocation decomposition, **decomposition makes line count go up**) |
| Span-keyed consumption counter        | +15                                                                                                                                                                        |
| `method_def_ordinals` made read-only  | +20                                                                                                                                                                        |
| `synth.rs` new file                   | +80 ~ 120                                                                                                                                                                  |
| `verify.rs` new file                  | +400 ~ 600 (07 layer 1, within this document's scope but counted separately)                                                                                               |
| `regalloc.rs` new file                | **+500 ~ 1000**                                                                                                                                                            |
| **Total**                             | **`ir_gen.rs` from ~8448 → 8600 ~ 8800; including new files, net increase 900 ~ 1600 lines**                                                                               |

**The early expectation "8448 → 5000~6000" is in the wrong direction**; the result of measured
extrapolation is the opposite. Need to explain why:

1. **This document's core benefit is not reducing line count, it is eliminating a whole class of
   defects that "error without reporting".** The 6 manual save/restore pairs (22 lines) become about
   12 lines + one guard type after SSA + RAII—**the line count is almost unchanged, but "forgot to
   restore" shifts from being a possibility to being a compile error**.
2. **Pure-relocation decomposition inevitably makes the line count go up.** Splitting
   `generate_call_expr_ir`'s 746 lines into 7 functions, each must repeat the signature (the
   function already has `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` binding, and
   error propagation. **"Splitting files to reduce line count" is a misconception—it reduces
   cognitive load, not physical lines.**
3. **The new verifier and allocator are net line additions.** These two items (~900-1600 lines) are
   **exactly the carriers of the benefit**.

**Honest statement of net benefit**:

> In line count, this document is most likely a **net increase**. The benefit is in
> **verifiability**—turning defects that "error without reporting" into invariants that "fail to
> pass means error". If line count is taken as the acceptance criterion, this document will fail;
> the correct acceptance criterion is the criteria in the "Batching Strategy".

**This phase will make `ir_gen.rs`'s code volume net increase, and intentionally so.** The
2026-10-03 decision has canceled all line-count / volume gates (see
[08](08-maintenance-mechanism.md) prohibition two), therefore **no line-count waiver list is
needed**—the legitimacy of the increase is argued by separation of responsibilities: SSA-ization
separates "register allocation + definition discipline" from lowering into an independent module;
the code increase is in exchange for eliminating the 6 manual save/restore pairs and the `arg_regs`
semantic reordering, two classes of "misalignment doesn't error, only misvalues" manual discipline.

## Key Decisions and Rationale

### Three Core Decisions

**Decision 1: temporary value strategy—delete the three statement-level rollbacks, making single
definition a mathematical consequence of construction.** The current rollbacks (`1954-1959` /
`3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
monotonically increase, and each temporary slot naturally has only one definition point. **Same
criterion but different guards** (`3632` has `result_reg.is_none()` guard, the other two do not) and
this kind of hand-copied difference disappears. The cost is that slot usage goes
up—`temp_high_water` already has the 255 upper-bound check (`1992-2000`) as a backstop.

**Decision 2: do not do mem2reg, only do explicit renaming.** Reasons already given in "Value
identification approach": YaoXiang's `&` / reference / ownership semantics make "which slots can be
mem2reg'd" a type system problem, requiring the unified type representation from
[03](03-type-unification.md) to determine. **Doing mem2reg before 03 is guessing.** Explicit
renaming only needs to change the return-value semantics of one place, `next_temp_reg`, and delete
three rollbacks, without touching any lowering algorithm's control flow.

**Decision 3: `Phi` adopts option A (IR-only pseudo-instruction, expanded to Move sequence).**
`executor/` and `.42` format have **zero changes**, the regression surface is limited to L3.
Decisive reason: `.42` format change would trigger version-number issues and full-compatibility
discussions, while A's volume cost is irrelevant under interpreted execution (what executes is Move,
not Phi). Cost is that `Phi` is invisible in `dump_bytecode` (needs attention when entering C1/C2
phases, 07 layer 3 must compare against it).

### Rejected Alternatives

**A. All-at-once split (SSA + decomposition + `synth` boundary + allocator all at once). Rejected.**
Violates 07's core principle: criteria must be tiered by category. Mixing four batches into one PR
mixes pure-relocation changes with C4 changes, making snapshot drift impossible to attribute—as soon
as the snapshot changes, one cannot tell whether the split is wrong or the SSA is wrong. **Rollback
granularity degrades from "change item" to "everything".**

**B. Split files first, then optimize order. Rejected.** Splitting does not eliminate any root
cause: 6 manual save/restore pairs are still scattered across 10 files, `arg_regs` semantic
reordering still requires manual reasoning. Worse, **splitting first loses the "build criteria
before changing" opportunity**—splitting produces a lot of line-level diff that gets mixed into
subsequent SSA diffs, invalidating review. This is of the same origin as the 8 orphaned test trees
(1005 lines / 78 tests never run) recorded in 06-cleanup-inventory.md: **wrong order means every
subsequent step is accelerating on the wrong foundation.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as "the third path once considered".** The form is: keep `ir_gen.rs` unchanged, add a pass that
converts its non-SSA IR output into SSA IR. **Advantage**: lowering is completely untouched, easy to
revert. **Reason for rejection**: (1) Does not eliminate defect class 1's root cause—the three
statement-level reclamations are still in `ir_gen.rs`, the 6 save/restore pairs are still separated
by 59-161 lines; a conversion pass can merge multiple definitions into `Phi`, but **cannot fix
"inner function names leak into outer scope"**—that is a `cur_locals` save/restore error, occurring
before IR form, invisible to the conversion pass. (2) Turns SSA into a second representation—there
is a prior failure of 3 parallel type representations in the repo, and the core lesson of this
project is that "**boundaries exist only in human self-awareness**". (3) `verify_loose` becomes a
permanent burden.

**D. Keep the status quo, only add a verifier. Rejected as the only solution, but it is a component
of this document.** RFC-039 alternative plan B has already given the same judgment: gates prevent
regression, they do not fix the status quo. **But must make clear what D can achieve**: D can
**catch** the **instances** of defect classes 1/2/3 (provided the verifier covers dominance, type
consistency, inner isolation), but cannot catch defect class 4 (`closure_captures` leak)—that
requires a cross-function liveness assertion, beyond `verify(&ModuleIR)`'s function boundary.
**Therefore D and this document are not alternatives, but a prerequisite.**

## Known Limitations and Risks

### Risks

- **Net code increase of 900-1600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point**—the 2026-10-03 decision has canceled line-count / volume gates, scale issues are resolved
  by separation of responsibilities (see [08](08-maintenance-mechanism.md) prohibition two). The
  legitimacy of the increase is: it replaces the 6 manual save/restore pairs and the `arg_regs`
  semantic reordering, two classes of "misalignment doesn't error, only misvalues" manual
  discipline.
- **The "single definition" check in `verify.rs` is meaningless under non-SSA form**
  (`Operand::Local` allows multiple definitions)—07 has already recorded this limitation;
  `verify_loose` is its degraded mode before this batch.
- **Deleting statement-level reclamations will change the temporary slot allocation pattern** (batch
  a). Even though `temp_high_water` guarantees no overflow, **the allocation result will
  change**—this is exactly why the C4 stage's "behavioral equivalence" criterion must actually run
  the full corpus, not sample.
- **Whether the `u8` / 255 slot upper bound** (`codegen/operand.rs:39-45`) is still sufficient after
  deleting rollbacks is **unverified**. The E3014 check in `generate_function_ir` (`1992-2000`) will
  catch it, but catching means compile failure—which may expose a batch of
  previously-rolled-back-masked super-large functions.
- **The upside must be recorded in parallel**: turning "untestable errors" into "testable
  invariants"; not changing the backend (`executor/` and `.42` zero changes); not moving
  `Instruction`'s semantics (all 76 → 77 variants preserved, all 50 `translate_*` preserved);
  incremental (each item independently revertible).

> **All originally-listed open questions in this section have been decided.** Item-by-item decisions
> are in the [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no open items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Overarching charter; four-layer model, criterion tiers, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **Hard prerequisite**; C4 category
  criterion definition, `verify_loose` / `verify_ssa`, three-layer criteria, snapshot immunity to
  `arg_regs` limitation
- [03-type-unification.md](03-type-unification.md) — **Hard prerequisite**; unification of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; `ReleasePlan`'s `PlanId`
  refactor attribution
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphaned test trees; "build directory
  first, then wiring fails" cautionary tale

### Code Locations

**IR Definitions**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` ([03](03-type-unification.md) scope)
- `src/middle/core/ir.rs:12-20` — `Operand` 7 variants
- `src/middle/core/ir.rs:47-533` — `Instruction` 76 variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for `PlanId`
  scheme)
- `src/middle/core/ir.rs:689-719` — `all_instructions` and other accessors

**Defect Class 1 (save/restore)**

- `src/middle/core/ir_gen.rs:248-249` — Explicit textual contract "must be physically adjacent to
  the next_temp save/restore points"
- `src/middle/core/ir_gen.rs:238-242` — Same comment convention for `cur_span`
- `src/middle/core/ir_gen.rs:884-891` — `next_temp_reg`, with `temp_high_water` true high-water mark
- `src/middle/core/ir_gen.rs:1954-1959` / `3632-3638` / `4737-4742` — Three statement-level register
  reclamations
- `src/middle/core/ir_gen.rs:1798` vs `2001` — `take_cur_locals` criterion inconsistency
- `src/middle/core/ir_gen.rs:1987-2000` — E3014 register overflow check (`MAX_REGISTERS = 255`)
- `src/middle/core/ir_gen.rs:1337` — `rebase_jump_targets` (07's "jump target exists" invariant
  target)
- 6 save/restore function entries: `1677` / `1830` / `2105` / `2209` / `2759` / `4678`

**Defect Class 2 (arg_regs)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — Three semantic reorderings and
  clone
- `src/middle/core/ir_gen.rs:7838-7846` — "Pad 0" silent fallback

**Defect Class 3 (span-keyed)**

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — Three cross-layer field declarations
- `src/middle/core/ir_gen.rs:341` / `1942` — `release_plan` write and consumption
- `src/middle/core/ir_gen.rs:7420` / `7493` — `overload_resolutions` consumption
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — `method_def_ordinals` initialization and sole
  write point (key is module-unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — Two fields in the transit structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — Cross-layer transit
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  `overload_resolutions` production

**Defect Class 4 (implicit order dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — `generate_spawn_for_ir`'s AST synthesis and
  callback
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — Same kind of handshake in
  `generate_spawn_expr_ir`
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — Two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — Other 3 `ast::Type` constructions

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr` 66 variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1408 lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — Three hardcoded discards (contrast `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge to be torn down by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  downgrades
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, `u8` upper bound 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; 83 constants in the file
- `src/backends/interpreter/executor/ops/control.rs:90` — Active implementation of
  `BytecodeInstr::Switch` (no corresponding variant in IR)
- `src/backends/interpreter/executor/debug.rs:194` — Dispatch match

**Test Current State**

- `src/middle/core/ir_gen.rs` — 8448 lines, 0 `#[cfg(test)]`, 0 `mod tests`
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, 29 tests in
  total; `def_assign` / `local_slots` positively nail DefId and slot naming, no IR structural
  invariant check
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 mentions of ir_gen, all
  reverse assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop`
  (per-opcode round-trip criterion example, last test in the file; 1209 lines / 23 `#[test]` in
  total)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies only `criterion` / `proptest`; no regalloc / petgraph /
  graphlib in `[dependencies]`
- `docs/src/.vitepress/config.js:226-231` — Sidebar auto-scans the `/rfc/draft` directory; this
  document is not in that directory, referenced by RFC-039 and this directory index
