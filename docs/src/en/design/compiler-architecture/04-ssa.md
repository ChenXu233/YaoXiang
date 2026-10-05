# Intermediate Representation SSA-ization

> **Subsidiary design document**. This article is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution phase sequence are in the RFC-039
> main text; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This article handles the IR construction discipline at the L3 layer, proposing a "each value defined
exactly once" construction approach for `src/middle/core/ir_gen.rs` (8448 lines). **The goal is not
to introduce a new instruction set, but to eliminate three classes of failure that produce no error
at all**:

1. 6 manual register save/restore pairs scattered tens of lines apart;
2. The `arg_regs` semantic reshuffling shared across 6 branches in `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. The silent failure of cross-layer contracts keyed by `Span`.

This article is responsible for: IR form (`Operand` / `Instruction`) changes, temporary value
allocation discipline, cross-layer contract explicitness, call expression dispatch splitting, and
the wiring of the IR static verifier.

This article is NOT responsible for: type representation convergence ([03](03-type-unification.md)),
stage contracts and obligations ledger ([02](02-stage-contract.md)), the definition and gating of
equivalence criteria ([07](07-equivalence-oracle.md)), frontend paradigms
([05](05-frontend-paradigm.md)), or dead code and unimplemented design cleanup
([06](06-cleanup-inventory.md)). This article references the conclusions of those documents in
multiple places without re-arguing them.

**Criteria category** is executed per **C4 (IR form change)** of
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, not full IR snapshot equality. SSA-ization necessarily changes IR form; forcing
snapshot equality would induce the team to relax criteria (07 documents precisely this trap).

**The position in the RFC-039 stage sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, acceptance is behavioral equivalence + `verify_ssa` green). The internal
implementation batch numbering in this article (batches a ~ d) is separate from the ordering within
P7: the former is this article's batching strategy, the latter is RFC-039's global phases.

## Feasibility Premises

**Verified facts.** The IR already has **all structural premises** needed for SSA construction:

| SSA Premise                  | Current State                                                                                                   | Location              |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------- | --------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                                | `ir.rs:624-628`       |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                  | `ir.rs:636-642`       |
| Scope nesting info           | `LocalSlot::scope_depth` ("0 = function parameter level, for disambiguating same-named slots in nested scopes") | `ir.rs:658`           |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                              | `ir.rs:47-533`        |
| Per-instruction span         | Each variant carries a `span: Span` field                                                                       | `ir.rs:47-533`        |
| Value type annotatable       | `LocalSlot::ty: MonoType`                                                                                       | `ir.rs:656`           |
| Slot count upper-bound check | E3014 check in `generate_function_ir` (`MAX_REGISTERS = 255`)                                                   | `ir_gen.rs:1987-2000` |

**Without these premises, the cost estimates in this article would be completely different.**

**Two decisive points:**

**First, SSA-ization is not adding capability—it is removing a capability dependency.** Currently,
temporary slots are **actively designed for multiple definitions**—`1954-1959` / `3632-3638` /
`4737-4742` explicitly roll back `next_temp`. The first thing SSA-ization does is **delete these
three rollback points**, letting `next_temp` monotonically increase. After deletion, **each
temporary slot naturally has exactly one definition point**—single definition shifts from "a rule
that needs checking" to "a mathematical consequence of the construction approach".

**Second, slot count upper bound is already a hard check.** `temp_high_water` at `ir_gen.rs:889` is
already tracking the true high-water mark, and `1992-2000` is already erroring at the 255 upper
bound. **Removing the rollback will not let slot counts run out of control**—`temp_high_water`
records the historical peak occupancy, independent of whether rollback occurs (the comment at
`887-888` says exactly this).

**The combined meaning of these two points**: `ir_gen.rs` is already maintaining all the bookkeeping
information needed for SSA; it just **recovers** that information. The bulk of SSA-ization work lies
in splitting `generate_call_expr_ir` and making cross-layer contracts explicit—not in building IR
expressive capability.

### Pre-dependency: Type Representation Unification

**Verified facts.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` also has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The serialization side has a third set, `ir::Type`, bridged by
`impl From<MonoType> for IrType` at `bytecode.rs:2353`.

That is: **Two sets of type representation coexist in the IR, and one of them directly `pub use`s
the AST's type.**

SSA-ization requires adding the `Phi` variant, recording a type for each SSA value (for the 07
verifier's "type-consistent" invariant), and judging whether two predecessors' types are compatible
at a CFG join. **If type representation has not converged, SSA will write a type-compatibility
judgment for each representation**—and a third set of representation will grow from that.

This is not a "which to do first is better" question—it is a question of **doing this first would
produce three sets of type-compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first complete the
`pub use ast::Type` convergence at `ir.rs:3` and the bridge removal at `bytecode.rs:2353`. Item 6 of
this article's change list explicitly records that `ir.rs:3` is **not touched in this article**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not a file that "is ugly but works". It is a file that **bets correctness on human
line-by-line review**:

- The file has **0 `#[cfg(test)]`, 0 `mod tests`** (verified).
- `src/middle/core/tests/mod.rs` declares `bytecode` / `def_assign` / `local_slots` modules, with 29
  tests in total—of which `def_assign.rs` and `local_slots.rs` **positively** cover ir_gen's DefId
  allocation and local slot naming (sentinel-level), and `bytecode.rs` mentions ir_gen in 4 places
  (`421` / `423` / `1028` / `1173`), **all reverse assertions**—"ir_gen does not produce", "ir_gen
  does not construct", "ir_gen frontend does not construct the corresponding IR".

That is: ir_gen has 29 sentinel-level tests pinning down local behavior, but **no IR structural
invariant checks** (dominance, single definition, jump targets, type consistency)—not one of the
three modules declared in `middle/core/tests/mod.rs` makes a positive assertion about overall IR
form.

And this file happens to contain several classes of defects that **compile successfully, pass all
checks, the program runs, just with wrong values**. Their common property: **failure produces no
error**.

This is where the highest risk lies. Not "function is too long", not "naming is chaotic", but **this
file's core invariants have no machine-executable enforcement**.

`ir_gen.rs:248-249` writes this as an explicit contract:

> Save before nested function body generation, restore after—**must be physically adjacent to
> `next_temp`'s save/restore points**, otherwise inner function names will leak to the outer scope.

This is a rule **expressed by a comment instead of an assertion**. It holds today only because those
6 save/restore pairs all happen to be written correctly.

**Why "just add a verifier" is not enough**: A verifier can **catch** these defects, but cannot
catch their **root cause**. The cause is not "missed in review" but:

> **The current IR form itself allows a slot to be defined multiple times, and `ir_gen`'s temporary
> register allocator actively depends on this.**

Concrete evidence: three statement-level register reclaims (`1954-1959` / `3632-3638` /
`4737-4742`)—they roll back `next_temp` at the end of each statement to the larger of "last named
slot ⋈ `temp_floor`" (`1959`: `self.next_temp = named_watermark.max(self.temp_floor);`). This means
the same `Operand::Local(n)` slot **is written repeatedly within a single function body**, and the
number of writes, the order of writes, and the live range are entirely determined by "rollback
timing"—a single manual decision.

Under this form, "single definition" is not a rule that can be incrementally imposed—it **directly
conflicts with the register allocation strategy**. So SSA-ization is not adding a check to existing
code—it is **replacing the allocation strategy that depends on multiple definitions**. The good news
is that the replacement cost of this strategy is very low—see "Feasibility Premises" above.

### Defect Class 1: 6 Manual save/restore, Failure Silent

**Verified facts.** 6 save/restore pairs; the maximum distance between save and restore is **161
lines**:

| Save Point            | Restore Point | Span      | Function                        | Entry Line |
| --------------------- | ------------- | --------- | ------------------------------- | ---------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | 63 lines  | `generate_method_ir`            | `1677`     |
| `1858-1866`           | `2026-2029`   | 161 lines | `generate_function_ir`          | `1830`     |
| `2116`                | `2180`        | 64 lines  | `generate_curry_innermost_func` | `2105`     |
| `2224-2229`           | `2284-2288`   | 59 lines  | `generate_curry_function_ir`    | `2209`     |
| `2768-2777`           | `2820-2851`   | 74 lines  | `generate_anon_binding_ir`      | `2759`     |
| `4686-4695`           | `4767-4774`   | 72 lines  | `generate_lambda_body_ir`       | `4678`     |

> **Attribution notes**: The two curry function correspondences are `2116`/`2180` belonging to
> `generate_curry_innermost_func` (entry `2105`), `2224-2229`/`2284-2288` belonging to
> `generate_curry_function_ir` (entry `2209`)—that is, "innermost" goes to the former, do not
> reverse-intuit by line number size.

Fields involved: `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span` /
`loop_stack`, uniformly driven by `next_temp_reg` (`ir_gen.rs:884-891`). This function maintains the
true high-water mark `temp_high_water` at `889` (comment self-statement: statement-level reclaim
will roll back `next_temp`; total slots are based on historical peak occupancy). A similar comment
convention exists for `cur_span` (`238-242` self-statement: "save before nested function body
generation, restore after (same treatment as next_temp)").

**Two places with inconsistent semantics** (this is a more insidious issue than "missed writes"):

| Location         | Semantic                                                                                                                 |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `ir_gen.rs:1798` | `take_cur_locals(param_types.len())` — truncate by **parameter count**                                                   |
| `ir_gen.rs:2001` | `take_cur_locals(total_locals)`, where `total_locals = self.temp_high_water` (`:1992`) — truncate by **high-water mark** |

`generate_method_ir` uses the former, **and lacks the E3014 overflow check at `1992-2000`**. The two
functions give two different answers to "how many slots should the function body have", and neither
reports an error.

**Three duplicate statement-level register reclaims with consistent criteria but different guards**:

| Location    | Function                           | Guard                     |
| ----------- | ---------------------------------- | ------------------------- |
| `1954-1959` | `generate_function_ir`             | None                      |
| `3632-3638` | `generate_block_ir` (entry `3602`) | `if result_reg.is_none()` |
| `4737-4742` | `generate_lambda_body_ir`          | None                      |

The guard at `3632` has a clear reason (comment at `3626-3631`: blocks at expression operand
positions may hold sibling argument temporaries that survive across blocks in the outer scope). The
other two have no equivalent guard. **The same criterion is hand-copied in three places, with a
guard in one and none in the other two**—this kind of difference is exactly what SSA is meant to
eliminate.

**The restore point of `generate_anon_binding_ir` is separated by IR construction** (`2768-2777`
save → `2820-2824` restore 5 fields → `2829-2848` construct `func_ir` → `2851` only then restore
`loop_stack`). The two halves of the same state are physically separated by 22 lines of construction
code, and the order dependency is **reversed** (restore slot table first, then construct IR, then
restore loop stack).

**Consequence**: Miss one restore → `next_temp` leaks, temporary value numbering jumps, `cur_locals`
names leak to outer scope. **The IR remains self-consistent, passes all checks, still compiles—just
with wrong values.**

### Defect Class 2: `arg_regs` Semantic Reshuffling in `generate_call_expr_ir`

**Verified facts.** `generate_call_expr_ir` is at `ir_gen.rs:7269-8014`, **746 lines**, single
function. The function signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (also carries `_expr` / `result_reg` /
`instructions` / `constants`); internally 6 branches share the same `Vec<Operand>` and perform
**semantic rewrites**:

| Line        | Semantic                                                                                                                                            |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten by `slots` then re-`collect` — named argument reshuffle                                                                     |
| `7724`      | Struct construction: per-slot `unwrap` by `final_args` then re-`collect` — field reshuffle                                                          |
| `7835-7848` | Function call: named argument reshuffle; `7838-7846` **pads missing slots with a fresh register holding 0** (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();` — clone then emit                                                                                |

Original form of the four reshuffles:

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

The comment at `7835-7848` self-statement: "Uncovered slots: remain absent, type check phase reports
E1010. Runtime pads 0 for safety (normal path will not reach here)."

**This design has two layers of risk:**

1. **Misalignment reports no error, only wrong value.** Each of the 6 branches independently
   determines the final order of `arg_regs`; any order assumption error will not panic.
2. **Canonicalized snapshots are immune to this.** 07 has documented this limitation: snapshot tools
   rename temporary values by "first occurrence order", so "the 3rd argument should have used the
   5th register" looks identical in the snapshot. **This class of defect does not even have a
   snapshot-level criterion in the C4 phase.**

**Additional verification: the struct construction path's reshuffle has no duplicate-slot guard.**
The `final_args` at `7724` is filled by "positional arguments inserted first, named arguments
overwriting by field name"—**no `slots[idx].is_some()` or similar duplicate-slot check before
overwrite** (contrast: the namespace path at `7381` has one)—when a named argument silently
overwrites a positional one, no panic, no diagnostic; and the argument **evaluation order** (source
order) diverges here from the final `arg_regs` order (field declaration order). The batch d
`fill_missing` assertion must also cover "duplicate slot" check, otherwise the struct construction
path's silent overwrite will survive intact.

### Defect Class 3: Span-Keyed Cross-Layer Contract Silent Failure

**Verified facts.** The repository has **two** span-keyed cross-layer contracts:

| Contract               | Declaration                                  | Production                                                                               | Transfer                                                               | Consumption              |
| ---------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------ |
| `release_plan`         | `ir_gen.rs:191` `HashMap<Span, Vec<String>>` | `ownership.rs:31` (`ReleasePlan::drops` field), `2574` `build_release_plan`, `2786` call | `typecheck/types.rs:31` ← `checker.rs:1282` / `1440` ← `ir_gen.rs:341` | `ir_gen.rs:1942`         |
| `overload_resolutions` | `ir_gen.rs:222` `HashMap<Span, String>`      | `inference/expressions.rs:4366`, `inference/statements.rs:2758` / `2857`                 | `typecheck/types.rs:56` ← `checker.rs:1447-1448` ← `ir_gen.rs:364-366` | `ir_gen.rs:7420`, `7493` |

Form at `1942`:

```rust
// NLL Release: insert Drop instructions at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some` — **a miss is a miss, no else branch, no assertion, no counter**. Any inconsistency in
span calculation on either side → **Drop instructions silently disappear, zero errors**. The Drop
sequence of refinement types is the core carrier of this project's ownership semantics
(`ownership.rs:26-32` self-statement: "NLL precise release plan", key is the Span of the last use
position), and its implementation is silent.

`7420` / `7493` are of the same form (`if let Some(mangled) = self.overload_resolutions.get(span)`),
with the consequence that overload resolution falls back to the default binding.

> **`method_def_ordinals` is NOT span-keyed.** It is `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with the sole write point at `:1692-1702` (assigning `#N` suffix by
> `base_name` definition order). Its risk is **another kind**: definition-order dependency—the `#N`
> suffix is determined by AST definition order; if the generation order is inconsistent with the
> typecheck registration order, the same function name will resolve to different targets. This is
> not span mismatch, but equally "wrong without error"; see "Additional Mechanisms for Span-Keyed"
> mechanism 3 for handling.

### Defect Class 4: spawn/lambda Implicit Handshake

**Verified facts.** `ir_gen.rs` has **two places** that construct `ast::Expr::Lambda` themselves and
call back to `generate_expr_ir`, relying on a set of mutable fields for handshake:

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
`generate_lambda_body_ir` (entry `4678`), `6600` clears. Two read points: `:6284` (inside closure
body `LoadUpvalue`) and **`:458`** (`closure_captures.contains_key(head)` on the `flatten_namespace`
path).

The read point at `:458` is the most dangerous: it is on the **namespace resolution path**, meaning
a live `HashMap<String, usize>` state can change variable name resolution results. Between `6589`
write and `6600` clear, a complete closure body generation (`6598`) happens; if any path inside
`generate_lambda_body_ir` returns early without clearing the table (it has multiple return points
when `?` operator propagates errors), **`closure_captures` will leak into unrelated code at
`:458`**.

Similar implicit state also includes `pending_env_names` (Handshake B written at `6699`, cleared at
`6714`).

### What SSA Will NOT Solve (Honestly Listed)

| Problem                                           | Location                                                                                                                     | Why SSA Doesn't Solve It                                                                                                                                                                                                                 |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                      | `ir_gen.rs:5390-5687`, **298 lines**, self-recursive 4 times (`5473` / `5504` / `5619` / `5640`), external call point `5270` | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **intrinsic to the algorithm**. SSA merely turns the `JmpIfNot` produced by guards into `Phi`, not reducing the number of guards |
| `eval_const_expr` complexity                      | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)      | Constant folding's expression coverage is unrelated to SSA                                                                                                                                                                               |
| Span-keyed failure                                | `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)                                                            | **SSA does not touch this layer at all.** Independent mechanism needed, see "Additional Mechanisms for Span-Keyed"                                                                                                                       |
| `Instruction` variant count                       | 76 (`ir.rs:47-533`)                                                                                                          | SSA adds `Phi`, **net change +1**. 76 variants correspond to 76 bytecode opcodes or degrade to NOP; SSA does not reduce this                                                                                                             |
| `method_def_ordinals` definition-order dependency | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                        | Same kind as span-keyed but different cause, see "Additional Mechanisms for Span-Keyed" mechanism 3                                                                                                                                      |

## Target Design

### SSA Form Definition

**Design judgment.** The target form is as follows.

#### How `Operand` Variants Change

Current `ir.rs:12-20` has 7 variants. **Repository-wide construction point verification**:

| Variant             | Production Construction Points                                                                                                                                                     | Disposition                                                                            |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 8 files                                                                                                                                                                            | **Keep**, unchanged                                                                    |
| `Local(usize)`      | 7 files                                                                                                                                                                            | **Keep but split semantics** (see below)                                               |
| `Arg(usize)`        | 4 files                                                                                                                                                                            | **Keep**, as implicit definitions of function entry (parameters = block 0 definitions) |
| `Global(usize)`     | 5 files                                                                                                                                                                            | **Keep**, unchanged                                                                    |
| `Temp(usize)`       | **0 construction points** (only handling arms in `codegen/operand.rs:39-45` and `:75`)                                                                                             | **Delete** (dead variant)                                                              |
| `Label(usize)`      | **0 construction points**                                                                                                                                                          | **Delete** (dead variant)                                                              |
| `Register(u8)`      | **0 construction points** (`ir.rs:19` comment says "Added for codegen", but codegen goes through `OperandResolver` in `codegen/operand.rs`, which does not construct this variant) | **Delete** (dead variant)                                                              |

`Local(usize)` is split into two variants because it currently **carries two mutually exclusive
semantics**—"named variable storage location" (can be written multiple times) and "compiler
temporary register" (should be written once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: dense numbering within function, single definition
    Value(ValueId),
    /// Named local variable storage slot: addressable, allows multiple definitions (non-SSA portion)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a `u32` newtype (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`), dense
numbering within function, **monotonically increasing, never rolled back**.

#### `Phi` as a New `Instruction` Variant

```rust
/// Join point: each predecessor basic block provides one definition of this value
/// incoming sorted by predecessor block label ascending (guaranteed at construction, for canonicalization)
Phi {
    dst: Operand,                                  // Must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (all enforced by `verify_ssa`, see [07](07-equivalence-oracle.md)
layer 1):

1. `incoming.len() == number of predecessors of this block`.
2. The label set of `incoming` == the label set of those blocks whose `successors` include this
   block's label in `blocks`.
3. All `incoming` values have the same `MonoType` (**depends on 03's type representation having
   converged**).

**Disposing of the guard issue at `3632-3638`**: Currently `generate_block_ir`'s reclaim has the
`if result_reg.is_none()` guard (comment at `3626-3631`: blocks at expression operand positions may
hold sibling argument temporaries that survive across blocks in the outer scope). After SSA-ization
this guard **is no longer needed**—because temporary values are no longer rolled back, sibling
arguments that survive across blocks hold their own `ValueId` and will not be affected by rollback.
**This is where SSA-ization truly simplifies code, not just renames things.**

#### Value Identification

**Design judgment**: No mem2reg, no virtual register numbering, **only explicit renaming**. Reasons:

- YaoXiang's `&` / reference / ownership semantics make "which slots can be mem2reg'd" a **type
  system problem**, not a CFG problem. Verified `ir.rs:656` `LocalSlot::ty: MonoType` carries
  complete type information, but determining "whether this `MonoType` can be mem2reg'd" requires the
  converged type representation from 03 and refinement type rules. **Doing mem2reg before 03 is
  guessing.**
- Explicit renaming only requires changing the return-value semantic of one function
  `next_temp_reg()` (`ir_gen.rs:884-891`) + deleting three rollback points, **not touching any
  lowering algorithm's control flow**.

Value table form: `FunctionBody::Code` adds `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot), None for pure temporaries
    pub debug_name: Option<String>,
    pub defining_block: usize,   // Label of the basic block where this value is defined
}
```

`values` and `locals` coexist: `locals` carries addressable named variables (may be defined multiple
times), `values` carries single-definition SSA values. **The existence of `locals` does not violate
SSA**—it is the "storage" portion outside SSA (LLVM's alloca model).

#### Change Magnitude Estimate

| Item                                              | Estimate               | Basis                                                                                                          |
| ------------------------------------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` refactor                          | ~15 lines              | `ir_gen.rs:884-891` single function                                                                            |
| Delete three statement-level reclaims             | −15 lines              | `1954-1959` / `3632-3638` / `4737-4742`                                                                        |
| 6 save/restore pairs collapse to RAII             | Net ±0 lines           | See "Per-Item Disposition Table"                                                                               |
| `ir.rs` structural changes                        | +120 ~ 180 lines       | `ValueId` / `Phi` / `ValueInfo` / value table                                                                  |
| `translator.rs` add `Phi` arm                     | +25 ~ 40 lines         | One of the existing 50 `translate_*`                                                                           |
| `executor/` add `PHI` instruction or runtime fill | +60 ~ 120 lines        | Depends on `Phi`'s bytecode form selection                                                                     |
| Rename `Operand::Local` → `Value` call sites      | Mechanical replacement | **Verified**: `Operand::Local(` appears **323 times** in `ir_gen.rs`, **355 times** across 6 files in the repo |

### Per-Item Disposition Table

| Defect Class                                      | SSA Eliminates?            | Disposition                                                                                                                                                                                                                                                                                                                                            | Criterion                                                            |
| ------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| **1. 6 manual save/restore**                      | **Yes (root cause level)** | `next_temp` / `temp_high_water` / `temp_floor` become `&mut` monotonic counters, no longer need cross-function-body save. `cur_locals` / `loop_stack` / `cur_span` use RAII guard (`struct NestedBodyGuard<'a>`, restores on `Drop`), **making "forgot to restore" a compile error**. The comment contract at `248-249` is enforced by the type system | 07 layer 1 "inner isolation" invariant + `verify_loose`/`verify_ssa` |
| **1b. Inconsistent semantics (`1798` vs `2001`)** | **Yes**                    | Unify to `take_cur_locals(self.temp_high_water)`, with E3014 check (`1992-2000`) extracted to shared function `check_register_budget()`, called by both                                                                                                                                                                                                | Corpus diff + snapshot (pure refactor phase can be zero-diff)        |
| **1c. Three statement-level reclaim criteria**    | **Yes**                    | All three deleted (`1954-1959` / `3632-3638` / `4737-4742`), including `3632`'s `result_reg.is_none()` guard                                                                                                                                                                                                                                           | C4: behavioral equivalence + invariants                              |
| **2. `arg_regs` semantic reshuffling**            | **Partial**                | SSA turns "wrong register" into "wrong value ID" and is catchable by dominance checks, but **the reshuffling logic itself remains**. Real elimination is via function splitting—6 branches each `push`, `arg_regs` ownership stays at function top                                                                                                     | Layer 1 dominance invariant (snapshots are immune)                   |
| **3. Span-keyed failure**                         | **No**                     | SSA does not touch it at all. See "Additional Mechanisms for Span-Keyed"                                                                                                                                                                                                                                                                               | 07's `test_release_plan_spans_consumed`                              |
| **4. Implicit order dependency**                  | **No**                     | See "`synth.rs` Boundary"                                                                                                                                                                                                                                                                                                                              | New explicit criterion needed                                        |
| `compile_pattern` 298 lines                       | No                         | See "What SSA Will NOT Solve"                                                                                                                                                                                                                                                                                                                          | —                                                                    |
| `eval_const_expr` 191 lines                       | No                         | See "What SSA Will NOT Solve"                                                                                                                                                                                                                                                                                                                          | —                                                                    |
| `Instruction` 76 variants                         | No (+1)                    | New `Phi` → 77                                                                                                                                                                                                                                                                                                                                         | —                                                                    |

### Register Allocator: Self-Developed or Use Existing Crate

**Verified facts (decisive)**:

1. `Operand::Register(u8)` (`ir.rs:19`) has **0 construction points repository-wide**.
2. The backend is not a register machine—`OperandResolver` in `src/middle/passes/codegen/operand.rs`
   resolves `Operand::Local/Temp/Arg` to `u8`, upper limit 255 (the `Temp` arm overflow at `39-45`
   is E3014).
3. `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1577 lines, 50 `translate_*`,
   function bodies distributed across `101-1558`) outputs
   `BytecodeInstruction::new(opcode, operands)`, with opcode from `src/backends/common/opcode.rs`
   (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter's opcode dispatch family.

**Conclusion**: The so-called "register" is an **operand stack slot index, not a physical
register**. The target machine **has no finite register file, no register pressure, no spill/split
needs**.

**Evaluate three options**:

| Option                                                     | Evaluation                                                                                                                                                                                                                                                                                                                                                                                        |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `regalloc` / `regalloc2` (rustc lineage)                   | **Not adopted.** Both take rustc's `Function` trait + MIR's `Place`/`Operand` abstractions as input. Writing an adapter layer for YaoXiang IR requires implementing the full trait (CFG, liveness, spill slot, scratch), **500+ lines of adapter code** for zero benefit—because there is no real register file to color. And rustc itself only does linear scan here (when `enable-llvm` is off) |
| `graph-alloc` series                                       | **Not adopted.** Same as above, plus API is even less stable                                                                                                                                                                                                                                                                                                                                      |
| **Self-developed linear scan (simplified Chaitin-Briggs)** | **Adopted.** What YaoXiang actually needs is: in `values` order, map each `ValueId` to a non-conflicting `u8` stack slot (`OperandResolver` already has the `u8` contract and E3014 upper limit). This is essentially **maximum clique coloring of non-overlapping live ranges**, which under no-register-file constraints degenerates to "first-available slot allocation in definition order"   |

**Design judgment**: Self-developed, target 500-1000 lines (including `values → Reg` mapping, live
range computation, and `OperandResolver` interface). **At the same time, must acknowledge**: if the
`u8`/255 slot upper limit is insufficient under real load, the self-developed allocator must handle
overflow, and existing code at `1992-2000` already treats it as an error. **This means this article
does NOT solve** the "large function insufficient registers" problem—that is a downstream
consequence of `compile_pattern` recursion depth, a separate topic.

`Cargo.toml:37-39` dev-dependencies only include `criterion` / `proptest`; no regalloc / petgraph /
graphlib in `[dependencies]`—introducing any of them is a new dependency surface.

### Splitting Strategy for `generate_call_expr_ir` 746 Lines

**Must be done last**, and **the SSA form and criteria must already exist**. Reason: the split
itself is C1 (pure refactor, criterion is snapshot zero-diff), but only after `arg_regs`'s semantic
is collapsed by SSA to "each argument has an independent `ValueId`" will the split not carry old
order assumptions into the new file.

**Core design constraint: `arg_regs` ownership stays at function top; 6 branches only `push`, never
reshuffle.**

```
generate_call_expr_ir (746 lines)
├── Argument evaluation: evaluate in source order, push to arg_regs — shared by 6 branches, the only place allowed to append
├── CallArgs (value object)
│   └── arg_regs: Vec<Operand>  — created only at top, 6 branches only push, never re-collect/reorder
├── Branch 1 → emit_namespace_call(args: CallArgs)         [7395 branch extracted]
├── Branch 2 → emit_bound_call(args: CallArgs)              [7406+ branch extracted]
├── Branch 3 → emit_struct_ctor(args: CallArgs)             [7724 branch extracted]
├── Branch 4 → emit_plain_call(args: CallArgs)              [7835-7848 branch extracted]
├── Branch 5 → emit_curried_call(args: CallArgs)
└── Branch 6 → emit_method_call(args: CallArgs)             [7420 / 7493 branch extracted]
```

The "pad 0" logic at `7835-7848` (`7838-7846`) descends to
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, **and the
self-statement comment "normal path will not reach here" is replaced with an assertion**: if any
slot is missing and typecheck did not report E1010, return diagnostic directly. **This turns a
silent fallback into a hard error—the most direct improvement this article makes to defect
class 2.**

**Criterion**: The pure refactor portion (SSA form unchanged) uses snapshot zero-diff; the
assertion-ization of `fill_missing` belongs to C4, using corpus diff + layer 1 verifier.

### `synth.rs` Boundary: Turning Implicit Callbacks into Explicit Synthetic AST Boundaries

**Verified facts**: In `ir_gen.rs`, `ast::Expr` is constructed at **exactly 2 locations**—`:4246`
and `:6701` (also `:6704` constructs `ast::Stmt`, `:510` / `:1266` / `:1399` construct `ast::Type`).

**Proposal**: Create new `src/middle/lower/synth.rs`, as the **only** module in the repo allowed to
construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not "tidiness" but:

> Constructing AST means **re-entering the frontend lowering entry point**. And the
> `pending_env_vars` / `closure_captures` state that `generate_expr_ir` expects is **precondition
> prepared by the caller**, and this precondition currently has no expression in any type or
> signature—it relies on the runtime fact of "whoever called me" to hold.

Change to explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST capturing env; capture set is explicitly given by parameters,
/// not dependent on the caller's prior write to self.pending_env_*
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Specific changes:

| Location                       | Current                                                                                    | After                                                                                                                                                                                               |
| ------------------------------ | ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → construct Lambda → callback                                     | Call `synth::synth_closure(...)` to get AST, explicitly pass `SynthEnv`; `generate_lambda_expr_ir` (`6543`) adds an internal entry carrying `SynthEnv`, **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → construct → callback → clear                                                       | Same as above; the `clear()` at `6713-6714` disappears (no shared state to clear)                                                                                                                   |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                    | Changed to read from parameter                                                                                                                                                                      |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                           | Same as above; `closure_captures` descends to a **local variable during closure body generation**, the `clear()` at `6600` disappears                                                               |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads shared field on namespace resolution path | **This is the most dangerous spot**: change to explicit parameter, otherwise any `?` early return inside `generate_lambda_body_ir` will leak the capture table to unrelated code                    |

**Criterion**: New sentinel test `test_no_ast_construction_outside_synth`, scans `src/middle/**`
except `synth.rs` for no `ast::Expr::` / `ast::Stmt {` construction syntax. Form same as RFC-039's
`check-module-boundary.py`.

### Disposition of Span-Keyed Issues (What SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism 1: Consumption coverage assertion (mandatory, lowest cost).** 07 already defines
`test_release_plan_spans_consumed` (Span set produced by ownership ⊆ Span set consumed by IR). This
article's supplement is to make it a **runtime counter** rather than just a test:

```rust
// ir_gen.rs near :341
// Record how many plans typecheck handed over
self.release_plan_total = type_result.release_plan.drops.len();
// Each time :1942 hits, self.release_plan_hit += 1;
// At end of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions are runtime-observable for non-`ref` locals (destructor
side effects), but **"the drop that should happen didn't happen" may not immediately crash under
YaoXiang's current value semantics + Arc/Rc model**—this is exactly why it is silent. The counter
turns "silent" into "detectable".

**Mechanism 2: Span key changed to `DefId` or explicit plan ID (mid-term).** The fundamental fix is
**not to use Span as the key**. `Span` is a source location that drifts due to macro expansion,
`include`, and multi-file merging. `FunctionIR` already has `def: Option<DefId>` field
(`ir.rs:677`), showing DefId is available at the IR layer. **Design judgment**: Introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, allocated by ownership / overload
resolution at production, matched by IR at consumption via `PlanId`. **This belongs to 02's
obligations ledger (`Obligations`) and should be implemented together with
[02](02-stage-contract.md), not placed in this article.**

**Mechanism 3: `method_def_ordinals` definition-order dependency (independent small item).** This
field is `HashMap<String, usize>` (`ir_gen.rs:224`), key is `"{type_name}.{method_name}"` (**no
module qualification**, written at `:1691-1700`)—if two modules define a same-named `Type.method`,
they share the same counter, and `#N` mangled names will drift. **Its failure mode is not span
mismatch but order/duplicate-name mismatch**—if typecheck's registration order is inconsistent with
ir_gen's definition order, the same-named method will resolve to different targets. Disposition:
change to typecheck side **producing once** `HashMap<DefId, String>` (bare name or `#N` mangled
name), ir_gen only reads, never writes. **This eliminates one mutable cross-function state inside
`ir_gen`.**

### Tracking the Three Hardcoded Drops

**Verified facts.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1408
lines) has three hardcoded drops:

| Location           | Field                | Value                                          | Consequence                                                                                             |
| ------------------ | -------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count zeroes after `.42` round-trip                                                     |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **`.42` product loses the entire exception table**                                                      |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` product loses global variable info** (note `2347`'s `global_names` **IS** filled, inconsistent) |

**Design judgment (revised 2026-10-05, decision D52): All three taken into this article's scope as
P7 batch e (7e), no longer as "independent issue".** Original judgment (last two as independent
issues) reasoning is recorded below, along with why it was overturned:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields), unrelated to
   SSA.~~ True, but not a reason for exclusion—P7 already touches `bytecode.rs`, and fixing all
   three in one go only needs one `VERSION` bump (4→5); splitting into two bumps is more expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → bump version → scope expands from "IR
   form" to "product format".~~ Bump mechanism already exists (`MAGIC` + `VERSION` header, reader
   rejects by version), and `.42` is a build product; cross-version read compatibility is not a
   goal—what expands is workload, not risk.
3. ~~The actual risk level of `2315` / `2341` is low; running `.42` directly is a minor path.~~
   **Overturned**: losing the exception table makes throw/try behave incorrectly when `.42` is run
   directly—throw/try is core language semantics; a minor path does not equal silent wrongness. This
   refactoring's principle is **leaving no unimplemented legacy of core functionality**
   (`Vec::new() // Not implemented yet` is exactly that kind of legacy).

**Also retained**: All three must still be registered in the `Obligations` ledger as "produced but
not serialized" until 7e completes, to prevent it from becoming a fourth "thing nobody knows about".
`2312` lands together with the version bump (D17); already-assigned but unused opcode values
(D32/D34) are reclaimed in the same batch.

## Detailed Design

### Cascading Effects of IR Structural Changes

#### `src/middle/core/ir.rs` (905 lines)

| Change                                                               | Location            | Nature                                            |
| -------------------------------------------------------------------- | ------------------- | ------------------------------------------------- |
| Add `ValueId(u32)` type                                              | Near file head `10` | New                                               |
| `Operand` delete `Temp` / `Label` / `Register`, add `Value(ValueId)` | `12-20`             | **Variant add/delete**                            |
| `Instruction` add `Phi` variant                                      | End of `47-533`     | **Variants +1 → 77**                              |
| `FunctionBody::Code` add `values: Vec<ValueInfo>`                    | `636-642`           | Field +1                                          |
| Add `ValueInfo` struct                                               | Near `660`          | New                                               |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`              | `689-719`           | Need to add `values()` / `values_mut()` accessors |

The `pub use ...ast::Type` at `ir.rs:3` is **NOT touched by this article**—it is
[03](03-type-unification.md)'s scope. But `ValueInfo::ty` must use the **single type after
convergence**.

#### `src/middle/core/bytecode.rs` (2422 lines)

| Change                                                            | Location                              | Nature                                                                                               |
| ----------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `BytecodeInstr` add `Phi` or "runtime expansion to Move" strategy | `127-554` (currently **66** variants) | Variant +1 or +0 (see below)                                                                         |
| `opcode()` add `Phi` arm                                          | `558`                                 | Required (`BytecodeInstr` exhaustive match)                                                          |
| `size()` add `Phi` arm                                            | `649`                                 | Required                                                                                             |
| `From<BytecodeFile>` decode match add `Phi` arm                   | `943-2350`                            | Required                                                                                             |
| `upvalue_count: 0` fix                                            | `2312`                                | Tangentially fixed in this article (in-memory format; on-disk requires bumping `VERSION`, see below) |

**The bytecode form of `Phi` needs a design decision.** Two options (**design judgment, recommend
A**, reasons in "Key Decisions and Reasons"):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded at `translator.rs` to a
  series of `Move { dst: vreg, src: incoming[i] }` at the start of the block, selected by
  predecessor block index. **No new opcode**, `.42` format zero change. Cost: bytecode volume
  increases (one per predecessor).
- B: New `PHI` opcode + predecessor index encoding, interpreter maintains register version on jump.
  **Requires changing `executor/` state model + `.42` format + version number**. Benefit is only
  volume.

Decisive reason for choosing A: changing `.42` format triggers version number issues and full
compatibility discussions, **and A's volume cost is irrelevant under interpretation** (what executes
is Move, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1577 lines, 50 `translate_*`, distributed `101-1558`)

| Change                                              | Location                                                                                                                         | Nature                                                                   |
| --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `Operand::Value(v)` dispatch                        | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                  |
| `Instruction::Phi` translation arm                  | Inside dispatch match, before `Free` (533)                                                                                       | New branch, expanded to Move series (option A)                           |
| Existing 9 NOP degradations **positions unchanged** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified 9**: `655-657` covers 2 variants, `658-660` covers 4 variants |

**SSA-ization will not reduce NOP count.** These 9 degradations reflect the "IR has, bytecode
doesn't" expressiveness gap; SSA-ization does not touch them.

#### `src/backends/interpreter/executor/`

If option A is adopted, `executor/` **needs no changes at all**—`Phi` disappears from
interpreter-visible form at the `translator.rs` stage. **This is the second decisive reason for
choosing A.**

`executor/ops/control.rs:90`'s `BytecodeInstr::Switch` has an active implementation, but **no**
`Switch` / `BrTable` / `JumpTable` variant exists in `ir.rs` (verified: 0 hits for these three names
in `ir.rs`)—**this is a reverse gap**: the bytecode layer has instructions the IR layer cannot
produce. It does not affect this article, but is recorded here because it shows "IR → bytecode" is
not surjective.

`executor/debug.rs:194`'s dispatch match also needs no changes (no new opcode under option A).

### Compiler Change List

**File by file, function by function.** All line numbers are the **starting point** of the change
(verified):

| #   | File                                      | Location                                         | Change                                                                                                           |
| --- | ----------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/core/ir.rs`                   | `12-20`                                          | `Operand` variant add/delete                                                                                     |
| 2   | `src/middle/core/ir.rs`                   | End of `47-533`                                  | Add `Instruction::Phi`                                                                                           |
| 3   | `src/middle/core/ir.rs`                   | `636-642`                                        | `FunctionBody::Code` add `values`                                                                                |
| 4   | `src/middle/core/ir.rs`                   | Near `660`                                       | Add `ValueInfo`                                                                                                  |
| 5   | `src/middle/core/ir.rs`                   | `689-719`                                        | Add `values()` / `values_mut()` accessors                                                                        |
| 6   | `src/middle/core/ir.rs`                   | `3`                                              | **`pub use ast::Type` not touched** ([03](03-type-unification.md) scope, recorded only)                          |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                        | Return `Operand::Value`, monotonically increasing, no rollback                                                   |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                      | **Delete** statement-level reclaim                                                                               |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                      | **Delete** statement-level reclaim (including `result_reg.is_none()` guard)                                      |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                      | **Delete** statement-level reclaim                                                                               |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                        | Replace with RAII guard (`generate_method_ir`, entry `1677`)                                                     |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                        | Replace with RAII guard (`generate_function_ir`, entry `1830`)                                                   |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                  | Replace with RAII guard (`generate_curry_innermost_func`, entry `2105`)                                          |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                        | Replace with RAII guard (`generate_curry_function_ir`, entry `2209`)                                             |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                        | Replace with RAII guard (`generate_anon_binding_ir`, entry `2759`)                                               |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                        | Replace with RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                      | E3014 check extracted to `check_register_budget()`, shared by `1798`/`2001` after semantic unification           |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                           | `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`, eliminate semantic inconsistency |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                        | Comment contract deleted (enforced by type system), replaced with explanation pointing to `verify`               |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                      | Change to call `synth::synth_closure`                                                                            |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                      | Change to call `synth::synth_closure`, delete `6713-6714`                                                        |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                      | Read `SynthEnv` from parameter, no longer `mem::take` shared field                                               |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                             | `closure_captures` descends to local, delete `clear()`                                                           |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                            | `closure_captures.contains_key` changed to explicit parameter (**highest priority implicit dependency**)         |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                   | Add span-keyed consumption counter                                                                               |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                      | `method_def_ordinals` changed to read-only                                                                       |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                      | **Last**: split into top-level `CallArgs` + 6 `emit_*`                                                           |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                      | "Pad 0" fallback changed to returning diagnostic                                                                 |
| 29  | `src/middle/lower/synth.rs`               | **New**                                          | Synthetic AST boundary                                                                                           |
| 30  | `src/middle/ir/verify.rs`                 | **New** ([07](07-equivalence-oracle.md) layer 1) | `verify_loose` / `verify_ssa`                                                                                    |
| 31  | `src/middle/passes/codegen/translator.rs` | dispatch match                                   | Add `Instruction::Phi` arm (option A: expand to Move series)                                                     |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                | `Operand::Value` resolution arm; delete `Temp` arm (`39-45`)                                                     |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                        | **Unchanged under option A** (`Phi` does not enter bytecode)                                                     |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                       | **Unchanged under option A**                                                                                     |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                           | `upvalue_count: 0` fix                                                                                           |
| 36  | `src/backends/interpreter/executor/**`    | —                                                | **Zero changes under option A**                                                                                  |
| 37  | `src/middle/passes/regalloc.rs`           | **New**                                          | Linear scan allocator                                                                                            |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New**                                          | `synth.rs` boundary sentinel check                                                                               |

### Backward Compatibility: Version Number of `.42` Format

**Verified facts**: The `.42` on-disk format **already has a version number
mechanism**—`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` ("YXBC") and
`VERSION: u32 = 4`, written to header (`:342-343`), verified on read (`:454-467`, mismatch reports
"unsupported bytecode version").

**Design judgment (direct corollary of option A)**:

- `Phi` expands to `Move` series at the `translator.rs` stage, **producing no new opcode**.
- The 83 constants in `opcode.rs` have **zero changes**, `opcode_name()` (`120`) unchanged, decode
  match (`bytecode.rs:943-2350`) unchanged.
- The `.42` format's field layout has **zero changes**, `VERSION` stays at 4. Old `.42` files can
  still be read by new binaries, new `.42` files can still be read by old binaries. **SSA-ization
  itself does not require a version bump.**

**The only scenario requiring a version bump is the `2312 upvalue_count` fix** (decision D17: fix,
and handle together with the version bump): add `upvalue_count` field to `BytecodeFunction` and have
the encoder write it—this is a **new field in the format**, bumping `VERSION` from 4 to 5. The
reader's existing version check rejects old files per existing behavior—`.42` is a build product
(`main.rs:671`), cross-version read compatibility is not a goal, the bump needs no additional
migration mechanism. Already-assigned but unused opcode values (`Switch` / `TailCall` for D32/D34)
are reclaimed together with the bump.

## Implementation Key Points

### Batching Strategy

Four batches. **Each item in a batch is an independent commit, independent criterion, independent
revert.** The "Change Item" column below refers to the number in the "Compiler Change List" above.

| Batch       | Content                                                                                                                                            | Change Items                 | Criterion                                                                                                                                                  | Rollback Point                                                                                                    |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Cut off multiple definitions: delete 3 statement-level reclaims → 6 save/restore replaced with RAII guard → semantic unification                   | 8-10 (first) → 11-16 → 17-18 | Delete reclaims: **C4** (behavioral equivalence + layer 1 dominance invariant). RAII and semantic unification are pure refactor: **C1** snapshot zero-diff | Per-item single commit, `git revert` rolls back                                                                   |
| **Batch b** | SSA form switch: `ir.rs` structural change + `Operand` mechanical replacement (323 places) → `translator.rs` add `Phi` arm → linear scan allocator | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` all green + `.42` round-trip test                                                                            | `ir.rs` change **occupies a single commit** (type change affects whole-repo compile, cannot incrementally revert) |
| **Batch c** | Implicit contract explicitness: `synth.rs` boundary + sentinel script → span consumption counter → `method_def_ordinals` read-only                 | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + new script added to CI; consumption counter zero-triggered on new diagnostic (otherwise indicates real mismatch)          | Independent commit                                                                                                |
| **Batch d** | `generate_call_expr_ir` split + "pad 0" fallback assertion                                                                                         | 27-28                        | Pure refactor portion: **C1** snapshot zero-diff. Assertion portion: **C4** corpus diff + layer 1 verifier                                                 | Independent commit                                                                                                |

**Three intra-batch order dependencies that cannot be swapped:**

1. Batch a's "delete reclaims" must precede batch b's `ir.rs` changes—let `next_temp` monotonically
   increase first, then `ValueId` has the semantic basis of "single definition".
2. Batch b's `Phi` arm must precede batch d's split—otherwise the split will carry old `arg_regs`
   order assumptions into the new file.
3. Batch d must be last—it depends on the first two batches having collapsed `arg_regs` semantic to
   "each argument has an independent `ValueId`".

**Position relative to RFC-039**: All four batches above fall within RFC-039's **P7**, criteria
grading (`verify_loose` / `verify_ssa`) is established by P2. Batches are **serial** to each other,
but within batch a, 8-10 / 11-16 / 17-18 have only order constraints and no coupling.

### Preconditions (Hard)

**Precondition 1: Criteria baseline.** The layer 1 verifier's `verify_loose` mode in
[07](07-equivalence-oracle.md) **must first run green on existing (non-SSA) IR** before batch a is
allowed to start.

Reason: SSA-ization's criterion is C4 (behavioral equivalence + invariants). If the invariant
verifier itself cannot run green on existing IR before the refactor, then each acceptance of batches
a ~ d lacks an executable criterion and can only rely on corpus diff—and corpus diff is **exactly
insensitive** to defect classes 1/2/3 (07's "Alternative A" has already demonstrated this). **This
is a real ordering constraint.**

**Precondition 2: Type representation convergence.** [03](03-type-unification.md) must complete the
`pub use ast::Type` convergence at `ir.rs:3` and the `From<MonoType> for IrType` bridge removal at
`bytecode.rs:2353`. **Otherwise `ValueInfo::ty` will introduce a third set of type-compatibility
judgments.**

### Rollback Strategy

- Each change is an independent commit, `git revert` granularity = change item granularity.
- Batch b's `ir.rs` structural change is the **only non-incrementally-revertable** part (type change
  affects whole-repo compile). Therefore it must **occupy a single commit**, and batch a must have
  run green with a revertable baseline before it.
- **No feature flag.** Reason: `Operand` variant add/delete cannot be isolated with runtime
  switches; feature flags leave CI untested on one path for extended periods—this is a recurrence of
  what RFC-039 documented as "test wiring depends on human memory, rot happens silently".

### Expectation Management: Net 900-1600 Line Increase

**Must be said upfront, or it will be challenged mid-implementation with "line count didn't go
down".**

| Item                              | Line Change                                                                                                                                             |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                    | `ir_gen.rs` **8448**                                                                                                                                    |
| Delete 3 statement-level reclaims | **−15**                                                                                                                                                 |
| 6 save/restore replaced with RAII | Net ±0 (RAII guard definition ~+20, use sites drop from 22 lines to ~12 lines)                                                                          |
| `next_temp_reg` refactor          | +5                                                                                                                                                      |
| `generate_call_expr_ir` split     | **+40 ~ 80** (6 `emit_*` signature and parameter passing overhead; this is the typical cost of pure refactor splits—**splitting increases line count**) |
| Span-keyed consumption counter    | +15                                                                                                                                                     |
| `method_def_ordinals` read-only   | +20                                                                                                                                                     |
| `synth.rs` new file               | +80 ~ 120                                                                                                                                               |
| `verify.rs` new file              | +400 ~ 600 (07 layer 1, counted independently though in this article's scope)                                                                           |
| `regalloc.rs` new file            | **+500 ~ 1000**                                                                                                                                         |
| **Total**                         | **`ir_gen.rs` roughly 8448 → 8600 ~ 8800; including new files net increase 900 ~ 1600 lines**                                                           |

**The earlier expectation direction of "8448 → 5000~6000" was wrong**; actual projection shows the
opposite. Need to explain why:

1. **This article's core benefit is not reducing line count, but eliminating an entire class of
   "wrong without error" defects.** The 6 manual save/restore pairs (22 lines) become ~12 lines +
   one guard type after SSA + RAII—**code line count barely changes, but "forgot to restore" goes
   from a possibility to a compile error**.
2. **Pure refactor splits necessarily increase line count.** Splitting `generate_call_expr_ir`'s 746
   lines into 7 functions requires duplicating signatures (this function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` binding, error propagation.
   **"Splitting files reduces line count" is a misunderstanding—it reduces cognitive load, not
   physical line count.**
3. **The new verifier and allocator are net line additions.** These two (~900-1600 lines) are
   precisely the **carriers of the benefit**.

**Honest statement of net benefit**:

> Line-count-wise, this article will most likely be **net increase**. The benefit lies in
> **verifiability**—turning "wrong without error" defects into "if it doesn't pass, it errors"
> invariants. If line count is used as the acceptance criterion, this article will fail; the correct
> acceptance criterion is the one in "Batching Strategy".

**This phase will net-increase `ir_gen.rs` code volume, intentionally.** The 2026-10-03 decision has
cancelled all line-count / volume gates (see [08](08-maintenance-mechanism.md) prohibition 2), so
**no line-count exemption list needed**—the increase's legitimacy comes from the
responsibility-separation argument: SSA-ization extracts "register allocation + definition
discipline" from lowering into independent modules, the code volume increase in exchange is
eliminating 6 manual save/restore pairs and `arg_regs` semantic reshuffling—two classes of
"misalignment silently wrong" manual discipline.

## Key Decisions and Reasons

### Three Core Decisions

**Decision 1: Temporary value strategy—delete three statement-level rollbacks, let single definition
become a mathematical consequence of construction.** The current rollbacks (`1954-1959` /
`3632-3638` / `4737-4742`) are for reusing temporary slots. Deleting them makes `next_temp`
monotonically increase, and each temporary slot naturally has exactly one definition point.
**Consistent criterion but different guards** (`3632` has `result_reg.is_none()` guard, the other
two don't)—this kind of hand-copied difference disappears. The cost is increased slot
occupancy—`temp_high_water` already has the 255 upper-limit check (`1992-2000`) as backstop.

**Decision 2: No mem2reg, only explicit renaming.** Reasons given in "Value Identification":
YaoXiang's `&` / reference / ownership semantics make "which slots can be mem2reg'd" a type system
problem, requiring the converged type representation from [03](03-type-unification.md) to determine.
**Doing mem2reg before 03 is guessing.** Explicit renaming only requires changing the return-value
semantic of one function `next_temp_reg` + deleting three rollback points, not touching any lowering
algorithm's control flow.

**Decision 3: `Phi` adopts option A (IR-only pseudo-instruction, expanded to Move series).**
`executor/` and `.42` format have **zero changes**, regression surface limited to L3. Decisive
reason: changing `.42` format triggers version number issues and full compatibility discussions, and
A's volume cost is irrelevant under interpretation (what executes is Move, not Phi). The cost is
that `Phi` is invisible in `dump_bytecode` (need to be careful when entering C1/C2 phase, layer 3 in
07 must compare it).

### Rejected Alternative Plans

**A. One-shot full split (SSA + split + `synth` boundary + allocator all at once). Rejected.**
Violates 07's core principle: criteria must be graded by category. Mixing four batches in one PR,
mixing pure refactor changes with C4 changes together, snapshot drift cannot be attributed—as soon
as the snapshot changes you don't know if the split refactored wrong or the SSA changed wrong.
**Rollback granularity degrades from "change item" to "everything".**

**B. Split files first, then optimize order. Rejected.** Splitting eliminates no root cause: 6
manual save/restore are still scattered across 10 files, `arg_regs` semantic reshuffling still
requires manual reasoning. Worse, **splitting first loses the opportunity to "build criteria before
changing"**—splitting produces massive line-level diffs that mix into subsequent SSA change diffs,
rendering review ineffective. This is the same source as the 8 orphan test trees (1005 lines / 78
tests never run) recorded in 06-cleanup-inventory.md: **wrong order, every subsequent step
accelerates on the wrong foundation.**

**C. Introduce an intermediate SSA layer without changing existing lowering. Rejected, but recorded
as "the third path once considered".** The form is to keep `ir_gen.rs` unchanged and add a pass that
converts its non-SSA IR output to SSA IR. **Pros**: lowering completely untouched, easy to rollback.
**Reasons for rejection**: (1) Does not eliminate the root cause of defect class 1—three
statement-level reclaims remain in `ir_gen.rs`, 6 save/restore pairs are still 59-161 lines apart;
the conversion pass can merge multiple definitions into `Phi`, but **cannot fix "inner function
names leaking to outer scope"**—that's a `cur_locals` save/restore error, occurring before IR form,
invisible to the conversion pass. (2) Makes SSA a second representation—the repository has 3
parallel type representations as a precedent, and the project's core lesson is "**boundaries only
exist in human awareness**". (3) `verify_loose` becomes a permanent burden.

**D. Keep current state, only add verifier. Rejected as the sole solution, but it is a component of
this article.** RFC-039 Alternative B has given the same judgment: gating can prevent regression,
cannot fix current state. **But must be clear what D can achieve**: D can **catch instances** of
defect classes 1/2/3 (provided the verifier covers dominance, type consistency, inner isolation),
but cannot catch defect class 4 (`closure_captures` leak)—that requires a cross-function liveness
assertion, beyond the function boundary of `verify(&ModuleIR)`. **Therefore D and this article are
not alternatives; D is a prerequisite.**

## Known Limitations and Risks

### Risks

- **Net 900–1600 line code increase** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point**—the 2026-10-03 decision has cancelled line-count / volume gates; scale issues are resolved
  by responsibility separation (see [08](08-maintenance-mechanism.md) prohibition 2). The increase's
  legitimacy lies in: it replaces 6 manual save/restore pairs and `arg_regs` semantic
  reshuffling—two classes of "misalignment silently wrong" manual discipline.
- **`verify.rs`'s "single definition" check is meaningless under non-SSA form** (`Operand::Local`
  allows multiple definitions)—07 has recorded this limitation, `verify_loose` is its degraded mode
  before this batch.
- **Deleting statement-level reclaims will change temporary slot allocation pattern** (batch a).
  Even though `temp_high_water` guarantees no overflow, **allocation results will change**—this is
  exactly why the C4 "behavioral equivalence" criterion must run the full corpus, not spot-check.
- **The `u8` / 255 slot upper limit** (`codegen/operand.rs:39-45`) after deleting
  reclaims—**unverified** whether still sufficient. The E3014 check in `generate_function_ir`
  (`1992-2000`) will catch it, but catching means compile failure—may expose a batch of large
  functions previously masked by "rollback".
- **Pros to record together**: turning "untestable errors" into "testable invariants"; not changing
  the backend (`executor/` and `.42` zero changes); not touching `Instruction` semantics (76 → 77
  variants all preserved, all 50 `translate_*` preserved); incremental (each item independently
  revertable).

> **The open questions originally listed in this section have all been adjudicated.** Per-item
> decisions are in [RFC-039 Decision Register](../rfc/draft/039-compiler-architecture.md) (D1–D50).
> **This article leaves no open items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../rfc/draft/039-compiler-architecture.md) — The
  overarching charter; four-layer model, criteria grading, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **Hard prerequisite**; C4 category criteria
  definition, `verify_loose` / `verify_ssa`, three-layer criteria, snapshots' immunity to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **Hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; `PlanId` refactoring of
  `ReleasePlan` belongs here
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; cautionary tale of
  "directory created first, wiring failed later"

### Code Locations

**IR Definition**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` ([03](03-type-unification.md) scope)
- `src/middle/core/ir.rs:12-20` — `Operand` 7 variants
- `src/middle/core/ir.rs:47-533` — `Instruction` 76 variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for `PlanId`
  plan)
- `src/middle/core/ir.rs:689-719` — `all_instructions` and other accessors

**Defect Class 1 (save/restore)**

- `src/middle/core/ir_gen.rs:248-249` — Explicit contract "must be physically adjacent to
  next_temp's save/restore points"
- `src/middle/core/ir_gen.rs:238-242` — Same comment convention for `cur_span`
- `src/middle/core/ir_gen.rs:884-891` — `next_temp_reg`, including true high-water `temp_high_water`
- `src/middle/core/ir_gen.rs:1954-1959` / `3632-3638` / `4737-4742` — Three statement-level register
  reclaims
- `src/middle/core/ir_gen.rs:1798` vs `2001` — `take_cur_locals` semantic inconsistency
- `src/middle/core/ir_gen.rs:1987-2000` — E3014 register overflow check (`MAX_REGISTERS = 255`)
- `src/middle/core/ir_gen.rs:1337` — `rebase_jump_targets` (07 "jump target exists" invariant
  target)
- Function entries for 6 save/restore pairs: `1677` / `1830` / `2105` / `2209` / `2759` / `4678`

**Defect Class 2 (arg_regs)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — Three semantic reshuffles and
  clones
- `src/middle/core/ir_gen.rs:7838-7846` — "Pad 0" silent fallback

**Defect Class 3 (span-keyed)**

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — Three cross-layer field declarations
- `src/middle/core/ir_gen.rs:341` / `1942` — `release_plan` write and consumption
- `src/middle/core/ir_gen.rs:7420` / `7493` — `overload_resolutions` consumption
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — `method_def_ordinals` initialization and sole
  write point (key is unmodule-qualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — Two fields in the transfer structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — Cross-layer transfer
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  `overload_resolutions` production

**Defect Class 4 (implicit order dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — `generate_spawn_for_ir`'s AST synthesis and
  callback
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — `generate_spawn_expr_ir`'s same handshake
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — Two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — 3 other `ast::Type` construction sites

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr` 66 variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1408 lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — Three hardcoded drops (contrast with `2344`
  / `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, `u8` upper limit 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; file has 83 constants total
- `src/backends/interpreter/executor/ops/control.rs:90` — Active implementation of
  `BytecodeInstr::Switch` (no corresponding IR variant)
- `src/backends/interpreter/executor/debug.rs:194` — Dispatch match

**Test Status**

- `src/middle/core/ir_gen.rs` — 8448 lines, 0 `#[cfg(test)]`, 0 `mod tests`
- `src/middle/core/tests/mod.rs` — Declares `bytecode` / `def_assign` / `local_slots`, 29 tests
  total; `def_assign` / `local_slots` positively pin down DefId and slot naming, no IR structural
  invariant check
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 mentions of ir_gen, all
  reverse assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop`
  (per-opcode round-trip criterion example, last test in file; file has 1209 lines / 23 `#[test]`
  total)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies only `criterion` / `proptest`; no regalloc / petgraph /
  graphlib in `[dependencies]`
- `docs/src/.vitepress/config.js:226-231` — Sidebar auto-scans `/design/rfc/draft` directory; this
  article is not under that directory, referenced by RFC-039 and this directory index
